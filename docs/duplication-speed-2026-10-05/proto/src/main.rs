mod fixture;
mod postings;

use postings::{Postings, partition};

use std::collections::HashMap;

use std::io::Write;
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
    fmin: usize,
    fcut: usize,
    t: usize,
}

const FUNCTIONS: &[&str] = &[
    "function_item",
    "function_declaration",
    "generator_function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
];

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
    rows: Vec<u32>,
    functions: Vec<(u32, u32)>,
}

impl<'a> Stream<'a> {
    fn text(&self, node: Node) -> &'a [u8] {
        &self.source[node.byte_range()]
    }

    fn emit(&mut self, text: &[u8], row: usize) {
        let hash = fnv(self.lang.seed(), &[text]);
        self.out.push(hash);
        self.rows.push(row as u32);
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
                self.rows.push(node.start_position().row as u32);
            }
            None => self.emit(text, node.start_position().row),
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
        let function = FUNCTIONS.contains(&node.kind());
        let mut start = self.out.len();
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
                let name = function && cursor.field_name() == Some("name");
                self.walk(cursor);
                trailing = (kind == ";").then_some(before);
                if name {
                    start = self.out.len();
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
        if function {
            self.functions.push((start as u32, self.out.len() as u32));
        }
        if terminated {
            if let Some(before) = trailing {
                self.out.truncate(before);
                self.rows.truncate(before);
            }
            self.emit(b";", node.end_position().row);
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
    if params.t > 0 {
        return mod_minimizers(tokens, params);
    }
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

fn mod_minimizers(tokens: &[u64], params: Params) -> Vec<(u32, u32)> {
    let grams = kgrams(tokens, params.k);
    let small = kgrams(tokens, params.t);
    if grams.is_empty() {
        return Vec::new();
    }
    let span = params.w + params.k - params.t;
    let windows = grams.len().saturating_sub(params.w) + 1;
    let mut out = Vec::new();
    let mut deque: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut next = 0;
    let mut last = usize::MAX;
    for start in 0..windows {
        let end = (start + span).min(small.len());
        while next < end {
            while deque.back().is_some_and(|back| small[*back] > small[next]) {
                deque.pop_back();
            }
            deque.push_back(next);
            next += 1;
        }
        while deque[0] < start {
            deque.pop_front();
        }
        let chosen = start + (deque[0] - start) % params.w.min(grams.len());
        if chosen != last {
            last = chosen;
            out.push(((grams[chosen] >> 32) as u32, chosen as u32));
        }
    }
    out
}

struct Parsed {
    tokens: Vec<u64>,
    rows: Vec<u32>,
    functions: Vec<(u32, u32)>,
    error: bool,
}

fn function_hash(tokens: &[u64]) -> u64 {
    tokens
        .iter()
        .fold(mix(tokens.len() as u64), |hash, token| mix(hash.rotate_left(17) ^ token))
}

fn indexed_functions(parsed: &Parsed, params: Params) -> Vec<(u64, u32, u32)> {
    parsed
        .functions
        .iter()
        .filter(|(start, end)| (params.fmin..params.fcut).contains(&((end - start) as usize)))
        .map(|(start, end)| (function_hash(&parsed.tokens[*start as usize..*end as usize]), *start, end - start))
        .collect()
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
        rows: Vec::new(),
        functions: Vec::new(),
    };
    stream.walk(&mut root.walk());
    Parsed {
        functions: stream.functions,
        rows: stream.rows,
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
    assert!(params.w <= params.k, "a chain proves contiguity only when w <= k");
    let started = Instant::now();
    let files = source_files(root);
    let mut shared = Shared::new();
    let mut parse_ms = 0.0;
    let mut entries: Vec<(u32, u32)> = Vec::new();
    let mut functions: Vec<(u64, u32)> = Vec::new();
    let mut chain: Vec<u8> = Vec::new();
    let mut counts = Vec::with_capacity(files.len());
    let (mut total_tokens, mut errors, mut lines, mut all_functions) = (0usize, 0usize, 0usize, 0usize);
    for path in &files {
        let lang = Lang::of(path).unwrap();
        let before = Instant::now();
        let source = std::fs::read(root.join(path)).unwrap();
        let syntax = shared.parse(lang, &source);
        parse_ms += ms(before);
        lines += source.iter().filter(|byte| **byte == b'\n').count();
        let parsed = stream(lang, path, &source, &syntax);
        errors += usize::from(parsed.error);
        let base = total_tokens as u32;
        for token in &parsed.tokens {
            put(&mut chain, (token >> 32) as u32);
        }
        for (key, position) in winnow(&parsed.tokens, params) {
            entries.push((key, base + position));
        }
        all_functions += parsed.functions.len();
        for (hash, start, _) in indexed_functions(&parsed, params) {
            functions.push((hash, base + start));
        }
        total_tokens += parsed.tokens.len();
        counts.push(parsed.tokens.len() as u32);
    }
    let fingerprints = entries.len();
    entries.sort_unstable();
    functions.sort_unstable();
    let mut kept = Vec::with_capacity(entries.len());
    let mut capped = Vec::new();
    let (mut longest, mut longest_kept) = (0usize, 0usize);
    for run in entries.chunk_by(|a, b| a.0 == b.0) {
        longest = longest.max(run.len());
        if run.len() > params.cap {
            capped.push(run[0].0);
        } else {
            longest_kept = longest_kept.max(run.len());
            kept.extend_from_slice(run);
        }
    }
    let mut out = Vec::new();
    for value in [params.k, params.w, params.cap, params.fmin, params.fcut, files.len(), kept.len(), capped.len(), functions.len(), params.t] {
        put(&mut out, value as u32);
    }
    for (path, count) in files.iter().zip(&counts) {
        put(&mut out, path.len() as u32);
        out.extend_from_slice(path.as_bytes());
        put(&mut out, *count);
    }
    let width = (usize::BITS - total_tokens.max(1).leading_zeros()).max(1);
    let region_at = out.len();
    let regions: Vec<(u64, u32)> = entries.iter().map(|(key, at)| (u64::from(*key), *at)).collect();
    out.extend(postings::encode(&regions, width));
    capped.iter().for_each(|key| put(&mut out, *key));
    let function_at = out.len();
    let functions: Vec<(u64, u32)> = functions.iter().map(|(hash, at)| (hash >> 16, *at)).collect();
    out.extend(postings::encode(&functions, width));
    let function_bytes = out.len() - function_at;
    std::fs::create_dir_all(index).unwrap();
    std::fs::write(index.join("index.bin"), &out).unwrap();
    std::fs::write(index.join("chain.bin"), &chain).unwrap();
    let total_ms = ms(started);
    println!(
        "{{\"files\":{},\"lines\":{lines},\"tokens\":{total_tokens},\"error_files\":{errors},\"fingerprints\":{fingerprints},\"capped_keys\":{},\"kept_entries\":{},\"longest_posting\":{longest},\"longest_kept_posting\":{longest_kept},\"functions\":{all_functions},\"indexed_functions\":{},\"index_bytes\":{},\"region_bytes\":{},\"function_bytes\":{function_bytes},\"chain_bytes\":{},\"total_ms\":{total_ms:.1},\"read_parse_ms\":{parse_ms:.1},\"extra_ms\":{:.1}}}",
        files.len(),
        capped.len(),
        kept.len(),
        functions.len(),
        out.len(),
        function_at - region_at,
        chain.len(),
        total_ms - parse_ms
    );
}

fn ms(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

struct Index {
    bytes: &'static [u8],
    params: Params,
    paths: Vec<(usize, usize)>,
    starts: Vec<u32>,
    regions: Postings<'static>,
    capped_at: usize,
    capped: usize,
    functions: Postings<'static>,
}

impl Index {
    fn load(path: &Path) -> Index {
        let bytes: &'static [u8] = Box::leak(std::fs::read(path).unwrap().into_boxed_slice());
        let header: Vec<usize> = (0..10).map(|i| word(bytes, i * 4) as usize).collect();
        let params = Params {
            k: header[0],
            w: header[1],
            cap: header[2],
            fmin: header[3],
            fcut: header[4],
            t: header[9],
        };
        let mut at = 40;
        let mut paths = Vec::with_capacity(header[5]);
        let mut starts = Vec::with_capacity(header[5] + 1);
        let mut start = 0u32;
        for _ in 0..header[5] {
            let length = word(bytes, at) as usize;
            paths.push((at + 4, length));
            at += 4 + length;
            starts.push(start);
            start += word(bytes, at);
            at += 4;
        }
        starts.push(start);
        let (regions, used) = Postings::read(&bytes[at..]);
        let capped_at = at + used;
        let (functions, _) = Postings::read(&bytes[capped_at + header[7] * 4..]);
        Index {
            bytes,
            params,
            paths,
            starts,
            regions,
            capped_at,
            capped: header[7],
            functions,
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

    fn locate(&self, global: u32) -> (u32, u32) {
        let file = partition(self.starts.len(), |i| self.starts[i] <= global) - 1;
        (file as u32, global - self.starts[file])
    }

    fn postings(&self, key: u32, out: &mut Vec<u32>) {
        out.clear();
        self.regions.find(u64::from(key), out);
    }

    fn has(&self, key: u32, file: u32, position: i64) -> bool {
        let global = i64::from(self.starts[file as usize]) + position;
        position >= 0
            && global < i64::from(self.starts[file as usize + 1])
            && self.regions.contains(u64::from(key), global as u32)
    }

    fn is_capped(&self, key: u32) -> bool {
        let found = partition(self.capped, |i| word(self.bytes, self.capped_at + i * 4) < key);
        found < self.capped && word(self.bytes, self.capped_at + found * 4) == key
    }

    fn function_postings(&self, hash: u64) -> Vec<(u32, u32)> {
        let mut found = Vec::new();
        self.functions.find(hash >> 16, &mut found);
        found.into_iter().map(|global| self.locate(global)).collect()
    }
}

#[derive(Default)]
struct Outcome {
    hits: usize,
    capped_hits: usize,
    bridged: usize,
    anchors: usize,
    unanchored: usize,
    true_spans: usize,
    missed_spans: usize,
    check_missed_spans: usize,
    regions: usize,
    proven: usize,
    deferred: usize,
    function_hits: usize,
    false_regions: usize,
    false_functions: usize,
    true_regions: usize,
    missed_regions: usize,
    check_files: usize,
    check_blocked: usize,
    check_false: usize,
    check_missed: usize,
}

struct Changed {
    tokens: Vec<u64>,
    prints: Vec<(u32, u32)>,
    capped: Vec<(u32, u32)>,
    functions: Vec<(u64, u32, u32)>,
}

type Hit = (u32, u32, i64, u32);
type Chain = (u32, u32, i64, u32, u32);

fn chains(hits: &[Hit], k: usize) -> Vec<Chain> {
    hits.chunk_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2 && b.3 - a.3 <= k as u32)
        .map(|run| (run[0].0, run[0].1, run[0].2, run[0].3, run[run.len() - 1].3))
        .collect()
}

fn bridge(index: &Index, streams: &[Changed], hits: &mut Vec<Hit>, base_files: u32) -> usize {
    let k = index.params.k as u32;
    let mut bridged = 0;
    loop {
        let mut added = Vec::new();
        for (self_id, other, diagonal, first, last) in chains(hits, k as usize) {
            if other >= base_files {
                continue;
            }
            let capped = &streams[self_id as usize].capped;
            let near = partition(capped.len(), |i| capped[i].0 + k < first);
            for (position, key) in capped[near..].iter().take_while(|(position, _)| *position <= last + k) {
                let outside = *position < first || *position > last;
                if outside && index.has(*key, other, i64::from(*position) + diagonal) {
                    added.push((self_id, other, diagonal, *position));
                }
            }
        }
        added.retain(|hit| hits.binary_search(hit).is_err());
        if added.is_empty() {
            return bridged;
        }
        bridged += added.len();
        hits.extend(added);
        hits.sort_unstable();
        hits.dedup();
    }
}

fn anchor(index: &Index, streams: &[Changed], hits: &mut Vec<Hit>, excluded: &[bool]) -> usize {
    let k = index.params.k as u32;
    let anchored: std::collections::HashSet<(u32, u32)> = hits.iter().map(|hit| (hit.0, hit.3)).collect();
    let mut found = Vec::new();
    let mut added = 0;
    for (self_id, changed) in streams.iter().enumerate() {
        let loose: Vec<&(u32, u32)> = changed.capped.iter().filter(|(position, _)| !anchored.contains(&(self_id as u32, *position))).collect();
        for run in loose.chunk_by(|a, b| b.0 - a.0 <= k) {
            let (position, key) = *run[0];
            index.postings(key, &mut found);
            for global in found.iter().take(index.params.cap) {
                let (other, other_position) = index.locate(*global);
                if !excluded[other as usize] {
                    hits.push((self_id as u32, other, i64::from(other_position) - i64::from(position), position));
                    added += 1;
                }
            }
        }
    }
    hits.sort_unstable();
    hits.dedup();
    added
}

fn spans_missed(truth: &HashMap<(u32, u32, i64, i64), usize>, blocked: &[(u32, i64, i64)]) -> (usize, usize) {
    let mut spans: Vec<(u32, i64, i64)> = truth.iter().map(|((s, _, _, start), length)| (*s, *start, start + *length as i64)).collect();
    spans.sort_unstable();
    spans.dedup();
    let missed = spans
        .iter()
        .filter(|(s, start, end)| !blocked.iter().any(|(bs, bstart, bend)| bs == s && bstart <= start && bend >= end))
        .count();
    (spans.len(), missed)
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
    let (mut parse_ms, mut stream_ms, mut changed_tokens) = (0.0, 0.0, 0usize);
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
        let prints = winnow(&parsed.tokens, params);
        let functions = indexed_functions(&parsed, params);
        stream_ms += ms(before);
        changed_tokens += parsed.tokens.len();
        if let Some(id) = index.id(path) {
            excluded[id] = true;
        }
        streams.push(Changed { tokens: parsed.tokens, prints, capped: Vec::new(), functions });
    }

    let before = Instant::now();
    let mut outcome = Outcome::default();
    let mut hits: Vec<Hit> = Vec::new();
    let mut found = Vec::new();
    let mut local: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for (self_id, changed) in streams.iter().enumerate() {
        for (key, position) in &changed.prints {
            local.entry(*key).or_default().push((self_id as u32, *position));
        }
    }
    for self_id in 0..streams.len() {
        let mut capped = Vec::new();
        for (key, position) in &streams[self_id].prints {
            for (other_self, other_position) in &local[key] {
                let diagonal = i64::from(*other_position) - i64::from(*position);
                if (*other_self == self_id as u32 && diagonal > 0) || *other_self > self_id as u32 {
                    hits.push((self_id as u32, base_files + other_self, diagonal, *position));
                }
            }
            if index.is_capped(*key) {
                capped.push((*position, *key));
                continue;
            }
            index.postings(*key, &mut found);
            for global in &found {
                let (other, other_position) = index.locate(*global);
                if !excluded[other as usize] {
                    hits.push((self_id as u32, other, i64::from(other_position) - i64::from(*position), *position));
                }
            }
        }
        outcome.capped_hits += capped.len();
        streams[self_id].capped = capped;
    }
    hits.sort_unstable();
    hits.dedup();
    outcome.anchors = anchor(&index, &streams, &mut hits, &excluded);
    outcome.bridged = bridge(&index, &streams, &mut hits, base_files);
    outcome.hits = hits.len();
    let mut proven = Vec::new();
    let mut deferred = Vec::new();
    for chain in chains(&hits, params.k) {
        let length = (chain.4 - chain.3) as usize + params.k;
        outcome.regions += 1;
        if length >= t {
            proven.push(chain);
        } else if length + 2 * (params.w - 1) >= t {
            deferred.push(chain);
        }
    }
    outcome.proven = proven.len();
    outcome.deferred = deferred.len();
    let anchored: std::collections::HashSet<(u32, u32)> = hits.iter().map(|hit| (hit.0, hit.3)).collect();
    outcome.unanchored = streams
        .iter()
        .enumerate()
        .map(|(self_id, s)| s.capped.iter().filter(|(position, _)| !anchored.contains(&(self_id as u32, *position))).count())
        .sum();
    let lookup_ms = ms(before);

    let before = Instant::now();
    let mut function_hits = Vec::new();
    let mut local_functions: HashMap<u64, Vec<(u32, u32)>> = HashMap::new();
    for (self_id, changed) in streams.iter().enumerate() {
        for (hash, start, length) in &changed.functions {
            for (other, other_start) in index.function_postings(*hash) {
                if !excluded[other as usize] {
                    function_hits.push((self_id as u32, other, *start, other_start, *length));
                }
            }
            for (other_self, other_start) in local_functions.get(hash).into_iter().flatten() {
                function_hits.push((self_id as u32, base_files + other_self, *start, *other_start, *length));
            }
            local_functions.entry(*hash).or_default().push((self_id as u32, *start));
        }
    }
    outcome.function_hits = function_hits.len();
    let function_ms = ms(before);
    let design_ms = ms(started) - parse_ms;

    let before = Instant::now();
    let mut sources: HashMap<u32, Vec<u64>> = HashMap::new();
    let mut check_blocked = Vec::new();
    for (self_id, other, diagonal, first, last) in &deferred {
        let theirs: &Vec<u64> = if *other >= base_files {
            &streams[(*other - base_files) as usize].tokens
        } else {
            sources.entry(*other).or_insert_with(|| {
                let path = String::from_utf8_lossy(index.path(*other as usize)).into_owned();
                let lang = Lang::of(&path).unwrap();
                let source = std::fs::read(root.join(&path)).unwrap();
                let syntax = shared.parse(lang, &source);
                stream(lang, &path, &source, &syntax).tokens
            })
        };
        let (length, start) = exact(&streams[*self_id as usize].tokens, theirs, i64::from(*first), *diagonal, *last as usize + params.k);
        if length >= t {
            check_blocked.push((*self_id, *other, *diagonal, start, length));
        }
    }
    outcome.check_files = sources.len();
    outcome.check_blocked = check_blocked.len();
    let check_ms = ms(before);

    let mut all_hits = hits.clone();
    for (self_id, changed) in streams.iter().enumerate() {
        for (position, key) in &changed.capped {
            index.postings(*key, &mut found);
            for global in &found {
                let (other, other_position) = index.locate(*global);
                if !excluded[other as usize] {
                    all_hits.push((self_id as u32, other, i64::from(other_position) - i64::from(*position), *position));
                }
            }
        }
    }
    all_hits.sort_unstable();
    all_hits.dedup();
    let narrow: Vec<Vec<u32>> = streams.iter().map(|s| s.tokens.iter().map(|t| (t >> 32) as u32).collect()).collect();
    let chain: Vec<u8> = std::fs::read(index_dir.join("chain.bin")).unwrap();
    let base_tokens = |file: u32| -> Vec<u32> {
        let (start, end) = (index.starts[file as usize] as usize, index.starts[file as usize + 1] as usize);
        chain[start * 4..end * 4].chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect()
    };
    let tokens_of = |file: u32| -> Vec<u32> {
        if file >= base_files { narrow[(file - base_files) as usize].clone() } else { base_tokens(file) }
    };
    let mut blocked = std::collections::HashSet::new();
    let mut stop_spans = Vec::new();
    for (self_id, other, diagonal, first, last) in &proven {
        let theirs = tokens_of(*other);
        let (exact_length, exact_start) = exact(&narrow[*self_id as usize], &theirs, i64::from(*first), *diagonal, *last as usize + params.k);
        if exact_length < t {
            outcome.false_regions += 1;
        }
        blocked.insert((*self_id, *other, *diagonal, exact_start));
        stop_spans.push((*self_id, exact_start, exact_start + exact_length as i64));
    }
    let stop_blocked = blocked.clone();
    let mut all_spans = stop_spans.clone();
    for (self_id, other, diagonal, start, _) in &check_blocked {
        let theirs = tokens_of(*other);
        let (length, _) = exact(&narrow[*self_id as usize], &theirs, *start, *diagonal, *start as usize + t);
        if length < t {
            outcome.check_false += 1;
        }
        blocked.insert((*self_id, *other, *diagonal, *start));
        all_spans.push((*self_id, *start, *start + length as i64));
    }
    for (self_id, other, start, other_start, length) in &function_hits {
        let theirs = tokens_of(*other);
        let (a, b) = (*start as usize, *other_start as usize);
        if narrow[*self_id as usize][a..a + *length as usize] != theirs[b..b + *length as usize] {
            outcome.false_functions += 1;
        }
    }
    let mut truth = HashMap::new();
    for run in all_hits.chunk_by(|a, b| a.0 == b.0 && a.1 == b.1 && a.2 == b.2) {
        let theirs = tokens_of(run[0].1);
        for hit in run {
            let (length, start) = exact(&narrow[hit.0 as usize], &theirs, i64::from(hit.3), hit.2, hit.3 as usize + params.k);
            if length >= t {
                truth.insert((hit.0, hit.1, hit.2, start), length);
            }
        }
    }
    outcome.true_regions = truth.len();
    (outcome.true_spans, outcome.missed_spans) = spans_missed(&truth, &stop_spans);
    outcome.check_missed_spans = spans_missed(&truth, &all_spans).1;
    let missed: Vec<usize> = truth.iter().filter(|(key, _)| !stop_blocked.contains(*key)).map(|(_, length)| *length).collect();
    outcome.missed_regions = missed.len();
    let longest_missed = missed.iter().copied().max().unwrap_or(0);
    let check_missed: Vec<usize> = truth.iter().filter(|(key, _)| !blocked.contains(*key)).map(|(_, length)| *length).collect();
    outcome.check_missed = check_missed.len();
    let longest_check_missed = check_missed.iter().copied().max().unwrap_or(0);
    let path_bytes: usize = index.paths.iter().map(|(_, length)| length + 8).sum();

    let mut stdout = std::io::stdout();
    writeln!(
        stdout,
        "{{\"changed\":{},\"t\":{t},\"load_ms\":{load_ms:.2},\"read_parse_ms\":{parse_ms:.2},\"stream_ms\":{stream_ms:.2},\"changed_tokens\":{changed_tokens},\"lookup_ms\":{lookup_ms:.2},\"function_ms\":{function_ms:.2},\"design_c_ms\":{design_ms:.2},\"check_ms\":{check_ms:.2},\"hits\":{},\"capped_hits\":{},\"bridged\":{},\"anchors\":{},\"unanchored\":{},\"true_spans\":{},\"missed_spans\":{},\"check_missed_spans\":{},\"incomplete\":{},\"regions\":{},\"proven\":{},\"deferred\":{},\"function_hits\":{},\"false_regions\":{},\"false_functions\":{},\"true_regions\":{},\"missed_regions\":{},\"longest_missed\":{longest_missed},\"check_files\":{},\"check_blocked\":{},\"check_false\":{},\"check_missed\":{},\"longest_check_missed\":{longest_check_missed},\"path_bytes\":{path_bytes}}}",
        changed.len(),
        outcome.hits,
        outcome.capped_hits,
        outcome.bridged,
        outcome.anchors,
        outcome.unanchored,
        outcome.true_spans,
        outcome.missed_spans,
        outcome.check_missed_spans,
        outcome.unanchored > 0,
        outcome.regions,
        outcome.proven,
        outcome.deferred,
        outcome.function_hits,
        outcome.false_regions,
        outcome.false_functions,
        outcome.true_regions,
        outcome.missed_regions,
        outcome.check_files,
        outcome.check_blocked,
        outcome.check_false,
        outcome.check_missed,
    )
    .unwrap();
}

fn exact<T: PartialEq + Copy>(mine: &[T], theirs: &[T], first: i64, diagonal: i64, end: usize) -> (usize, i64) {
    let at = |position: i64| -> Option<(T, T)> {
        let other = position + diagonal;
        if position < 0 || other < 0 || position as usize >= mine.len() || other as usize >= theirs.len() {
            return None;
        }
        Some((mine[position as usize], theirs[other as usize]))
    };
    if (first..end as i64).any(|p| at(p).is_none_or(|(a, b)| a != b)) {
        return (0, first);
    }
    let mut start = first;
    while at(start - 1).is_some_and(|(a, b)| a == b) {
        start -= 1;
    }
    let mut stop = end as i64;
    while at(stop).is_some_and(|(a, b)| a == b) {
        stop += 1;
    }
    ((stop - start) as usize, start)
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
                k: number(4, 41),
                w: number(5, 20),
                cap: number(6, 64),
                fmin: number(7, 10),
                fcut: number(8, 98),
                t: number(9, 5),
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
        Some("lines") => {
            let root = Path::new(&args[2]);
            let t = number(3, 60);
            let fmin = number(4, 10);
            let mut shared = Shared::new();
            let (mut spans, mut functions, mut one_line) = (Vec::new(), Vec::new(), 0usize);
            for path in source_files(root) {
                let lang = Lang::of(&path).unwrap();
                let source = std::fs::read(root.join(&path)).unwrap();
                let syntax = shared.parse(lang, &source);
                let parsed = stream(lang, &path, &source, &syntax);
                let rows = &parsed.rows;
                for start in 0..rows.len().saturating_sub(t - 1) {
                    spans.push(rows[start + t - 1] - rows[start] + 1);
                }
                for (start, end) in &parsed.functions {
                    if (*end - *start) as usize >= fmin {
                        let lines = rows[*end as usize - 1] - rows[*start as usize] + 1;
                        one_line += usize::from(lines == 1);
                        functions.push(lines);
                    }
                }
            }
            let summary = |values: &mut Vec<u32>| {
                values.sort_unstable();
                let at = |q: f64| values.get(((values.len() as f64 - 1.0) * q) as usize).copied().unwrap_or(0);
                format!("n={} min={} p10={} median={} p90={} max={}", values.len(), at(0.0), at(0.1), at(0.5), at(0.9), at(1.0))
            };
            println!("{t} tokens span lines: {}", summary(&mut spans));
            println!("functions of {fmin}+ tokens span lines (name excluded): {} one_line={one_line}", summary(&mut functions));
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
        let params = Params { k: 20, w: 41, cap: 64, fmin: 10, fcut: 118, t: 0 };
        let shared: Vec<u64> = (1000..1060).collect();
        let left: Vec<u64> = (0..37).chain(shared.iter().copied()).collect();
        let right: Vec<u64> = (500..511).chain(shared.iter().copied()).chain(600..650).collect();
        let keys = |t: &[u64]| winnow(t, params).into_iter().map(|p| p.0).collect::<std::collections::HashSet<_>>();
        assert!(keys(&left).intersection(&keys(&right)).next().is_some());
    }

    #[test]
    fn mod_minimizers_keep_one_selection_in_every_window() {
        let params = Params { k: 41, w: 20, cap: 64, fmin: 10, fcut: 98, t: 21 };
        let mut state = 7u64;
        let tokens: Vec<u64> = (0..5000)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                state >> 60
            })
            .collect();
        let chosen: Vec<u32> = winnow(&tokens, params).into_iter().map(|p| p.1).collect();
        assert!(chosen.windows(2).all(|pair| pair[1] - pair[0] <= params.w as u32));
        assert!(chosen[0] < params.w as u32);
        assert!(chosen[chosen.len() - 1] as usize >= tokens.len() - params.k - params.w + 1);
        let shared: Vec<u64> = tokens[1000..1060].to_vec();
        let left: Vec<u64> = (100..137).chain(shared.iter().copied()).collect();
        let right: Vec<u64> = (200..211).chain(shared.iter().copied()).chain(300..350).collect();
        let keys = |t: &[u64]| winnow(t, params).into_iter().map(|p| p.0).collect::<std::collections::HashSet<_>>();
        assert!(keys(&left).intersection(&keys(&right)).next().is_some());
    }
}
