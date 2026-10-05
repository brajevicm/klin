mod fixture;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tree_sitter::{Language, Node, Parser, Tree as Syntax, TreeCursor};

const TERMINATED: &[&str] = &[
    "ambient_declaration",
    "break_statement",
    "continue_statement",
    "debugger_statement",
    "do_statement",
    "export_statement",
    "expression_statement",
    "function_signature",
    "import_alias",
    "import_statement",
    "lexical_declaration",
    "return_statement",
    "throw_statement",
    "type_alias_declaration",
    "variable_declaration",
];

#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Rust,
    Ts,
    Tsx,
}

impl Lang {
    fn of(path: &str) -> Option<Lang> {
        if is_test_path(path) {
            return None;
        }
        match Path::new(path).extension()?.to_str()? {
            "rs" => Some(Lang::Rust),
            "ts" if !path.ends_with(".d.ts") => Some(Lang::Ts),
            "tsx" => Some(Lang::Tsx),
            _ => None,
        }
    }

    fn grammar(self) -> Language {
        match self {
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::Ts => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }

    fn seed(self) -> u64 {
        match self {
            Lang::Rust => 0x52,
            Lang::Ts | Lang::Tsx => 0x54,
        }
    }
}

fn is_test_path(path: &str) -> bool {
    path.split('/').any(|part| {
        matches!(part, "tests" | "test" | "__tests__" | "benches" | "fixtures")
    }) || path.contains(".test.")
        || path.contains(".spec.")
        || path.ends_with("_test.rs")
        || path.ends_with("/tests.rs")
}

#[derive(Clone, Copy)]
struct Params {
    k: usize,
    w: usize,
    cap: usize,
}

fn fnv(seed: u64, parts: &[&[u8]]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325 ^ seed;
    for part in parts {
        for byte in *part {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
        hash = (hash ^ 0xff).wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn mix(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

struct Stream<'a> {
    lang: Lang,
    source: &'a [u8],
    imports: HashMap<&'a [u8], String>,
    out: Vec<u64>,
}

impl<'a> Stream<'a> {
    fn text(&self, node: Node) -> &'a [u8] {
        &self.source[node.byte_range()]
    }

    fn emit(&mut self, text: &[u8]) {
        let hash = fnv(self.lang.seed(), &[text]);
        self.out.push(hash);
    }

    fn leaf(&mut self, node: Node) {
        let text = self.text(node);
        if text.is_empty() {
            return;
        }
        let named = matches!(node.kind(), "identifier" | "type_identifier");
        match self.imports.get(text).filter(|_| named) {
            Some(provenance) => {
                let hash = fnv(self.lang.seed(), &[text, b"@", provenance.as_bytes()]);
                self.out.push(hash);
            }
            None => self.emit(text),
        }
    }

    fn walk(&mut self, cursor: &mut TreeCursor<'a>) {
        let node = cursor.node();
        if node.kind().ends_with("comment") {
            return;
        }
        if !cursor.goto_first_child() {
            self.leaf(node);
            return;
        }
        let terminated = self.lang != Lang::Rust && TERMINATED.contains(&node.kind());
        let mut trailing = None;
        let mut skip_item = false;
        loop {
            let child = cursor.node();
            let kind = child.kind();
            if self.lang == Lang::Rust && kind == "attribute_item" {
                let text = self.text(child);
                skip_item |= text.ends_with(b"test]") || contains(text, b"cfg(test)");
                if !skip_item {
                    self.walk(cursor);
                }
            } else if skip_item && child.is_named() {
                skip_item = false;
            } else if !skip_item {
                let before = self.out.len();
                self.walk(cursor);
                trailing = (kind == ";").then_some(before);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
        if terminated {
            if let Some(before) = trailing {
                self.out.truncate(before);
            }
            self.emit(b";");
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

fn imports<'a>(lang: Lang, path: &str, source: &'a [u8], root: Node) -> HashMap<&'a [u8], String> {
    let mut map = HashMap::new();
    let mut cursor = root.walk();
    for item in root.children(&mut cursor) {
        match (lang, item.kind()) {
            (Lang::Rust, "use_declaration") => {
                if let Some(argument) = item.child_by_field_name("argument") {
                    use_tree(source, argument, "", &mut map);
                }
            }
            (Lang::Ts | Lang::Tsx, "import_statement") => ts_import(path, source, item, &mut map),
            _ => {}
        }
    }
    map
}

fn text<'a>(source: &'a [u8], node: Node) -> &'a [u8] {
    &source[node.byte_range()]
}

fn joined(prefix: &str, rest: &[u8]) -> String {
    let rest = String::from_utf8_lossy(rest);
    if prefix.is_empty() {
        rest.into_owned()
    } else {
        format!("{prefix}::{rest}")
    }
}

fn use_tree<'a>(source: &'a [u8], node: Node, prefix: &str, map: &mut HashMap<&'a [u8], String>) {
    match node.kind() {
        "identifier" => {
            map.insert(text(source, node), joined(prefix, text(source, node)));
        }
        "scoped_identifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                map.insert(text(source, name), joined(prefix, text(source, node)));
            }
        }
        "use_as_clause" => {
            if let (Some(path), Some(alias)) = (
                node.child_by_field_name("path"),
                node.child_by_field_name("alias"),
            ) {
                map.insert(text(source, alias), joined(prefix, text(source, path)));
            }
        }
        "scoped_use_list" => {
            let nested = match node.child_by_field_name("path") {
                Some(path) => joined(prefix, text(source, path)),
                None => prefix.to_string(),
            };
            if let Some(list) = node.child_by_field_name("list") {
                use_tree(source, list, &nested, map);
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                use_tree(source, child, prefix, map);
            }
        }
        _ => {}
    }
}

fn ts_import<'a>(path: &str, source: &'a [u8], item: Node, map: &mut HashMap<&'a [u8], String>) {
    let Some(specifier) = item.child_by_field_name("source") else {
        return;
    };
    let raw = String::from_utf8_lossy(text(source, specifier));
    let raw = raw.trim_matches(|c| c == '"' || c == '\'');
    let specifier = if raw.starts_with("./") || raw.starts_with("../") {
        resolve(path, raw)
    } else {
        raw.to_string()
    };
    let mut cursor = item.walk();
    for clause in item.named_children(&mut cursor) {
        if clause.kind() != "import_clause" {
            continue;
        }
        let mut inner = clause.walk();
        for part in clause.named_children(&mut inner) {
            match part.kind() {
                "identifier" => {
                    map.insert(text(source, part), format!("\"{specifier}\"#default"));
                }
                "namespace_import" => {
                    if let Some(name) = part.named_child(0) {
                        map.insert(text(source, name), format!("\"{specifier}\"#*"));
                    }
                }
                "named_imports" => {
                    let mut names = part.walk();
                    for spec in part.named_children(&mut names) {
                        let Some(name) = spec.child_by_field_name("name") else {
                            continue;
                        };
                        let local = spec.child_by_field_name("alias").unwrap_or(name);
                        let imported = String::from_utf8_lossy(text(source, name));
                        map.insert(text(source, local), format!("\"{specifier}\"#{imported}"));
                    }
                }
                _ => {}
            }
        }
    }
}

fn resolve(path: &str, relative: &str) -> String {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    for segment in relative.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn kgrams(tokens: &[u64], k: usize) -> Vec<u64> {
    if tokens.len() < k {
        return Vec::new();
    }
    let base: u64 = 0x100_0000_01b3;
    let top = (0..k - 1).fold(1u64, |power, _| power.wrapping_mul(base));
    let mut hash = 0u64;
    let mut out = Vec::with_capacity(tokens.len() - k + 1);
    for (index, token) in tokens.iter().enumerate() {
        if index >= k {
            hash = hash.wrapping_sub(tokens[index - k].wrapping_mul(top));
        }
        hash = hash.wrapping_mul(base).wrapping_add(*token);
        if index + 1 >= k {
            out.push(mix(hash));
        }
    }
    out
}

fn winnow(tokens: &[u64], params: Params) -> Vec<(u32, u32)> {
    let grams = kgrams(tokens, params.k);
    let mut out = Vec::new();
    let mut window: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut last = usize::MAX;
    for (index, hash) in grams.iter().enumerate() {
        while window.back().is_some_and(|back| grams[*back] >= *hash) {
            window.pop_back();
        }
        window.push_back(index);
        if window[0] + params.w <= index {
            window.pop_front();
        }
        if (index + 1 >= params.w || index + 1 == grams.len()) && window[0] != last {
            last = window[0];
            out.push(((grams[last] >> 32) as u32, last as u32));
        }
    }
    out
}

struct Parsed {
    tokens: Vec<u64>,
    error: bool,
}

struct Shared {
    parsers: HashMap<u8, Parser>,
}

impl Shared {
    fn new() -> Shared {
        Shared {
            parsers: HashMap::new(),
        }
    }

    fn parse(&mut self, lang: Lang, source: &[u8]) -> Syntax {
        let parser = self.parsers.entry(lang as u8).or_insert_with(|| {
            let mut parser = Parser::new();
            parser.set_language(&lang.grammar()).unwrap();
            parser
        });
        parser.parse(source, None).unwrap()
    }
}

fn stream(lang: Lang, path: &str, source: &[u8], syntax: &Syntax) -> Parsed {
    let root = syntax.root_node();
    let mut stream = Stream {
        lang,
        source,
        imports: imports(lang, path, source, root),
        out: Vec::new(),
    };
    stream.walk(&mut root.walk());
    Parsed {
        tokens: stream.out,
        error: root.has_error(),
    }
}

fn source_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(root.join(&relative)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || matches!(name.as_str(), "node_modules" | "target" | "dist" | "build") {
                continue;
            }
            let path = relative.join(&name);
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pending.push(path);
            } else {
                let path = path.to_string_lossy().into_owned();
                if Lang::of(&path).is_some() {
                    out.push(path);
                }
            }
        }
    }
    out.sort();
    out
}

fn put(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn build(root: &Path, index: &Path, params: Params) {
    let started = Instant::now();
    let files = source_files(root);
    let mut shared = Shared::new();
    let mut parse_ms = 0.0;
    let mut entries: Vec<(u32, u32, u32)> = Vec::new();
    let mut chain: Vec<u8> = Vec::new();
    let mut counts = Vec::with_capacity(files.len());
    let (mut total_tokens, mut errors, mut lines) = (0usize, 0usize, 0usize);
    for (id, path) in files.iter().enumerate() {
        let lang = Lang::of(path).unwrap();
        let before = Instant::now();
        let source = std::fs::read(root.join(path)).unwrap();
        let syntax = shared.parse(lang, &source);
        parse_ms += ms(before);
        lines += source.iter().filter(|byte| **byte == b'\n').count();
        let parsed = stream(lang, path, &source, &syntax);
        errors += usize::from(parsed.error);
        total_tokens += parsed.tokens.len();
        counts.push(parsed.tokens.len() as u32);
        for token in &parsed.tokens {
            put(&mut chain, (token >> 32) as u32);
        }
        for (key, position) in winnow(&parsed.tokens, params) {
            entries.push((key, id as u32, position));
        }
    }
    let fingerprints = entries.len();
    entries.sort_unstable();
    let mut kept = Vec::with_capacity(entries.len());
    let mut capped = Vec::new();
    let (mut longest, mut longest_kept, mut distinct) = (0usize, 0usize, 0usize);
    for run in entries.chunk_by(|a, b| a.0 == b.0) {
        distinct += 1;
        longest = longest.max(run.len());
        if run.len() > params.cap {
            capped.push(run[0].0);
        } else {
            longest_kept = longest_kept.max(run.len());
            kept.extend_from_slice(run);
        }
    }
    let mut out = Vec::new();
    for value in [params.k, params.w, params.cap, files.len(), kept.len(), capped.len()] {
        put(&mut out, value as u32);
    }
    for (path, count) in files.iter().zip(&counts) {
        put(&mut out, path.len() as u32);
        out.extend_from_slice(path.as_bytes());
        put(&mut out, *count);
    }
    for entry in &kept {
        put(&mut out, entry.0);
    }
    for entry in &kept {
        put(&mut out, entry.1);
    }
    for entry in &kept {
        put(&mut out, entry.2);
    }
    for key in &capped {
        put(&mut out, *key);
    }
    std::fs::create_dir_all(index).unwrap();
    std::fs::write(index.join("index.bin"), &out).unwrap();
    std::fs::write(index.join("chain.bin"), &chain).unwrap();
    let total_ms = ms(started);
    println!(
        "{{\"files\":{},\"lines\":{lines},\"tokens\":{total_tokens},\"error_files\":{errors},\"fingerprints\":{fingerprints},\"distinct_keys\":{distinct},\"capped_keys\":{},\"kept_entries\":{},\"longest_posting\":{longest},\"longest_kept_posting\":{longest_kept},\"index_bytes\":{},\"chain_bytes\":{},\"total_ms\":{total_ms:.1},\"read_parse_ms\":{parse_ms:.1},\"extra_ms\":{:.1}}}",
        files.len(),
        capped.len(),
        kept.len(),
        out.len(),
        chain.len(),
        total_ms - parse_ms
    );
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

struct Index {
    bytes: Vec<u8>,
    params: Params,
    paths: Vec<(usize, usize)>,
    offsets: Vec<u64>,
    entries: usize,
    keys_at: usize,
    capped_at: usize,
    capped: usize,
}

impl Index {
    fn load(path: &Path) -> Index {
        let bytes = std::fs::read(path).unwrap();
        let header: Vec<usize> = (0..6).map(|i| word(&bytes, i * 4) as usize).collect();
        let params = Params {
            k: header[0],
            w: header[1],
            cap: header[2],
        };
        let mut at = 24;
        let mut paths = Vec::with_capacity(header[3]);
        let mut offsets = Vec::with_capacity(header[3] + 1);
        let mut offset = 0u64;
        for _ in 0..header[3] {
            let length = word(&bytes, at) as usize;
            paths.push((at + 4, length));
            at += 4 + length;
            offsets.push(offset);
            offset += u64::from(word(&bytes, at)) * 4;
            at += 4;
        }
        offsets.push(offset);
        Index {
            params,
            paths,
            offsets,
            entries: header[4],
            keys_at: at,
            capped_at: at + header[4] * 12,
            capped: header[5],
            bytes,
        }
    }

    fn path(&self, id: usize) -> &[u8] {
        let (at, length) = self.paths[id];
        &self.bytes[at..at + length]
    }

    fn id(&self, path: &str) -> Option<usize> {
        self.paths
            .binary_search_by(|(at, length)| self.bytes[*at..*at + *length].cmp(path.as_bytes()))
            .ok()
    }

    fn key(&self, index: usize) -> u32 {
        word(&self.bytes, self.keys_at + index * 4)
    }

    fn postings(&self, key: u32) -> std::ops::Range<usize> {
        let start = partition(self.entries, |i| self.key(i) < key);
        let end = partition(self.entries, |i| self.key(i) <= key);
        start..end
    }

    fn posting(&self, index: usize) -> (u32, u32) {
        let files = self.keys_at + self.entries * 4;
        let positions = files + self.entries * 4;
        (word(&self.bytes, files + index * 4), word(&self.bytes, positions + index * 4))
    }

    fn is_capped(&self, key: u32) -> bool {
        let found = partition(self.capped, |i| word(&self.bytes, self.capped_at + i * 4) < key);
        found < self.capped && word(&self.bytes, self.capped_at + found * 4) == key
    }
}

fn partition(len: usize, below: impl Fn(usize) -> bool) -> usize {
    let (mut low, mut high) = (0, len);
    while low < high {
        let middle = (low + high) / 2;
        if below(middle) {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low
}

#[derive(Default)]
struct Outcome {
    hits: usize,
    capped_hits: usize,
    regions: usize,
    approx_block: usize,
    approx_unclear: usize,
    exact_block: usize,
    chain_bytes_read: usize,
    files_read_for_chain: usize,
}

fn query(root: &Path, index_dir: &Path, changed_count: usize, t: usize) {
    let started = Instant::now();
    let index = Index::load(&index_dir.join("index.bin"));
    let load_ms = ms(started);
    let params = index.params;
    let all: Vec<String> = (0..index.paths.len())
        .map(|id| String::from_utf8_lossy(index.path(id)).into_owned())
        .collect();
    let step = (all.len() / changed_count.max(1)).max(1);
    let changed: Vec<&String> = all.iter().step_by(step).take(changed_count).collect();
    let base_files = index.paths.len() as u32;
    let mut shared = Shared::new();
    let mut parse_ms = 0.0;
    let mut stream_ms = 0.0;
    let mut walk_ms = 0.0;
    let mut changed_tokens = 0usize;
    let mut excluded = vec![false; index.paths.len()];
    let mut streams = Vec::with_capacity(changed.len());
    for path in &changed {
        let lang = Lang::of(path).unwrap();
        let before = Instant::now();
        let source = std::fs::read(root.join(path)).unwrap();
        let syntax = shared.parse(lang, &source);
        parse_ms += ms(before);
        let before = Instant::now();
        let parsed = stream(lang, path, &source, &syntax);
        walk_ms += ms(before);
        let prints = winnow(&parsed.tokens, params);
        stream_ms += ms(before);
        changed_tokens += parsed.tokens.len();
        if let Some(id) = index.id(path) {
            excluded[id] = true;
        }
        streams.push((parsed.tokens, prints));
    }

    let before = Instant::now();
    let mut outcome = Outcome::default();
    let mut hits: Vec<(u32, u32, i64, u32)> = Vec::new();
    let mut local: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for (self_id, (_, prints)) in streams.iter().enumerate() {
        for (key, position) in prints {
            local.entry(*key).or_default().push((self_id as u32, *position));
        }
    }
    for (self_id, (_, prints)) in streams.iter().enumerate() {
        for (key, position) in prints {
            if index.is_capped(*key) {
                outcome.capped_hits += 1;
                continue;
            }
            for entry in index.postings(*key) {
                let (other, other_position) = index.posting(entry);
                if excluded[other as usize] {
                    continue;
                }
                hits.push((self_id as u32, other, i64::from(other_position) - i64::from(*position), *position));
            }
            for (other_self, other_position) in &local[key] {
                let same = *other_self == self_id as u32;
                let diagonal = i64::from(*other_position) - i64::from(*position);
                if (same && diagonal > 0) || *other_self > self_id as u32 {
                    hits.push((self_id as u32, base_files + other_self, diagonal, *position));
                }
            }
        }
    }
    outcome.hits = hits.len();
    hits.sort_unstable();
    let lookup_ms = ms(before);

    let before = Instant::now();
    let mut regions = Vec::new();
    for run in hits.chunk_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2 && b.3 - a.3 <= params.w as u32) {
        let (self_id, other, diagonal, first) = run[0];
        let last = run[run.len() - 1].3;
        let lower = (last - first) as usize + params.k;
        outcome.regions += 1;
        if lower >= t {
            outcome.approx_block += 1;
        } else if lower + 2 * (params.w - 1) >= t {
            outcome.approx_unclear += 1;
        } else {
            continue;
        }
        regions.push((self_id, other, diagonal, first, last));
    }
    let approx_ms = ms(before);

    let before = Instant::now();
    let mut chains: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut file = File::open(index_dir.join("chain.bin")).unwrap();
    let narrow: Vec<Vec<u32>> = streams.iter().map(|s| s.0.iter().map(|t| (t >> 32) as u32).collect()).collect();
    for (self_id, other, diagonal, first, last) in regions {
        let mine = &narrow[self_id as usize];
        let theirs: &Vec<u32> = if other >= base_files {
            &narrow[(other - base_files) as usize]
        } else {
            chains.entry(other).or_insert_with(|| {
                let start = index.offsets[other as usize];
                let end = index.offsets[other as usize + 1];
                let mut buffer = vec![0u8; (end - start) as usize];
                file.seek(SeekFrom::Start(start)).unwrap();
                file.read_exact(&mut buffer).unwrap();
                outcome.chain_bytes_read += buffer.len();
                outcome.files_read_for_chain += 1;
                buffer.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
            })
        };
        let length = exact(mine, theirs, first as i64, diagonal, last as usize + params.k);
        if length >= t {
            outcome.exact_block += 1;
        }
    }
    let exact_ms = ms(before);
    let total_ms = ms(started);
    let mut stdout = std::io::stdout();
    writeln!(
        stdout,
        "{{\"changed\":{},\"t\":{t},\"load_ms\":{load_ms:.2},\"read_parse_ms\":{parse_ms:.2},\"stream_ms\":{stream_ms:.2},\"walk_ms\":{walk_ms:.2},\"changed_tokens\":{changed_tokens},\"lookup_ms\":{lookup_ms:.2},\"approx_ms\":{approx_ms:.2},\"exact_ms\":{exact_ms:.2},\"total_ms\":{total_ms:.2},\"design_b_ms\":{:.2},\"design_a_ms\":{:.2},\"hits\":{},\"capped_hits\":{},\"incomplete\":{},\"regions\":{},\"approx_block\":{},\"approx_unclear\":{},\"exact_block\":{},\"chain_files\":{},\"chain_bytes\":{}}}",
        changed.len(),
        load_ms + stream_ms + lookup_ms + approx_ms,
        load_ms + stream_ms + lookup_ms + approx_ms + exact_ms,
        outcome.hits,
        outcome.capped_hits,
        outcome.capped_hits > 0,
        outcome.regions,
        outcome.approx_block,
        outcome.approx_unclear,
        outcome.exact_block,
        outcome.files_read_for_chain,
        outcome.chain_bytes_read,
    )
    .unwrap();
}

fn exact(mine: &[u32], theirs: &[u32], first: i64, diagonal: i64, end: usize) -> usize {
    let at = |position: i64| -> Option<(u32, u32)> {
        let other = position + diagonal;
        if position < 0 || other < 0 || position as usize >= mine.len() || other as usize >= theirs.len() {
            return None;
        }
        Some((mine[position as usize], theirs[other as usize]))
    };
    let mut start = first;
    while at(start - 1).is_some_and(|(a, b)| a == b) {
        start -= 1;
    }
    let mut stop = end as i64;
    while at(stop).is_some_and(|(a, b)| a == b) {
        stop += 1;
    }
    if (first..end as i64).any(|p| at(p).is_none_or(|(a, b)| a != b)) {
        return 0;
    }
    (stop - start) as usize
}

fn generate(dir: &Path, name: &str) {
    let (files, profile) = match name {
        "10k" => (5_000, fixture::BASE),
        "300k" => (5_000, fixture::DENSE_300K),
        "1m" => (5_000, fixture::DENSE_1M),
        other => panic!("unknown fixture {other}"),
    };
    let generated = fixture::write_sources(&fixture::Tree(dir.to_path_buf()), files, files / 100, profile);
    if let Some(expected) = profile.expected {
        assert_eq!(generated, expected, "generator copy drifted");
    }
    println!("{} loc={} digest={}", profile.name, generated.loc, generated.digest);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let number = |i: usize, default: usize| args.get(i).map_or(default, |v| v.parse().unwrap());
    match args.get(1).map(String::as_str) {
        Some("gen") => generate(Path::new(&args[2]), &args[3]),
        Some("build") => build(
            Path::new(&args[2]),
            Path::new(&args[3]),
            Params {
                k: number(4, 20),
                w: number(5, 41),
                cap: number(6, 64),
            },
        ),
        Some("walk") => {
            let root = Path::new(&args[2]);
            let mut shared = Shared::new();
            let files: Vec<(String, Vec<u8>, Syntax)> = source_files(root)
                .into_iter()
                .map(|path| {
                    let source = std::fs::read(root.join(&path)).unwrap();
                    let syntax = shared.parse(Lang::of(&path).unwrap(), &source);
                    (path, source, syntax)
                })
                .collect();
            let started = Instant::now();
            let mut tokens = 0;
            for _ in 0..number(3, 20) {
                for (path, source, syntax) in &files {
                    tokens += stream(Lang::of(path).unwrap(), path, source, syntax).tokens.len();
                }
            }
            println!("{tokens} tokens, {:.1} ns/token", ms(started) * 1e6 / tokens as f64);
        }
        Some("query") => query(Path::new(&args[2]), Path::new(&args[3]), number(4, 20), number(5, 60)),
        _ => eprintln!("usage: gen DIR 10k|300k|1m | build ROOT INDEX [K W CAP] | query ROOT INDEX [N T]"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(path: &str, source: &str) -> Vec<u64> {
        let lang = Lang::of(path).unwrap();
        let syntax = Shared::new().parse(lang, source.as_bytes());
        stream(lang, path, source.as_bytes(), &syntax).tokens
    }

    #[test]
    fn automatic_semicolons_keep_return_apart() {
        let split = tokens("a.ts", "function f() { return\nvalue }");
        let joined = tokens("a.ts", "function f() { return value }");
        let explicit = tokens("a.ts", "function f() { return value; }");
        assert_ne!(split, joined);
        assert_eq!(joined, explicit);
    }

    #[test]
    fn comments_layout_and_test_items_leave_no_tokens() {
        let plain = tokens("a.rs", "fn f() -> u8 { 1 }");
        let noisy = tokens("a.rs", "// c\nfn f()\n  -> u8 { /* x */ 1 }\n#[cfg(test)]\nmod t { fn g() {} }");
        assert_eq!(plain, noisy);
    }

    #[test]
    fn import_aliases_carry_their_source() {
        let a = tokens("a.ts", "import { parse as run } from \"parser-a\";\nrun();");
        let b = tokens("a.ts", "import { execute as run } from \"parser-b\";\nrun();");
        assert_ne!(a[a.len() - 4..], b[b.len() - 4..]);
    }

    #[test]
    fn winnowing_finds_every_shared_run_of_k_plus_w_minus_one() {
        let params = Params { k: 20, w: 41, cap: 64 };
        let shared: Vec<u64> = (1000..1060).collect();
        let left: Vec<u64> = (0..37).chain(shared.iter().copied()).collect();
        let right: Vec<u64> = (500..511).chain(shared.iter().copied()).chain(600..650).collect();
        let keys = |t: &[u64]| winnow(t, params).into_iter().map(|p| p.0).collect::<std::collections::HashSet<_>>();
        assert!(keys(&left).intersection(&keys(&right)).next().is_some());
    }
}
