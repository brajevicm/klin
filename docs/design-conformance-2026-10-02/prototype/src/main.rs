use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tree_sitter::{Node, Parser};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Rust,
    Ts,
}

const SKIP_DIRS: &[&str] = &[
    ".git", "node_modules", "vendor", "build", ".build", "dist", "target", ".venv", "venv", "coverage", ".next",
    "out", "fixtures",
];

const TEST_SEGMENTS: &[&str] = &[
    "test", "tests", "__tests__", "spec", "specs", "testing", "e2e", "__mocks__", "mocks", "testdata", "test_utils",
    "testutils", "benches", "bench", "examples", "example",
];

const MODULE_ROOTS: &[&str] = &["mod.rs", "lib.rs", "main.rs", "index.ts", "index.tsx", "index.mts"];

const STOP_WORDS: &[&str] = &[
    "self", "this", "new", "get", "set", "let", "const", "return", "value", "result", "data", "err", "error", "none",
    "some", "fn", "function", "the", "and", "for", "with", "from", "into", "unwrap", "clone", "default", "string",
    "str", "len", "true", "false", "null", "undefined", "await", "async", "mut", "ref", "vec", "map", "iter", "ok",
];

const TS_GLOBALS: &[&str] = &[
    "JSON", "Math", "Object", "Array", "Promise", "console", "String", "Number", "Boolean", "Date", "Reflect",
    "fetch", "window", "document", "process", "globalThis", "Intl", "Buffer", "URL", "crypto",
];

const RUST_STD_TRAITS: &[&str] = &[
    "Clone", "Copy", "Debug", "Default", "Display", "PartialEq", "Eq", "PartialOrd", "Ord", "Hash", "Drop", "From",
    "Into", "TryFrom", "AsRef", "AsMut", "Deref", "DerefMut", "Borrow", "Iterator", "IntoIterator", "Error",
    "FromStr", "Send", "Sync", "Serialize", "Deserialize", "Future", "Fn", "FnMut", "FnOnce", "Add", "Sub", "Mul",
    "Div", "Neg", "Not", "Index", "IndexMut", "Extend", "FromIterator", "Write", "Read",
];

const DYNAMIC_MACROS: &[&str] = &["submit", "inventory::submit", "register", "distributed_slice", "ctor"];

struct FileInfo {
    path: String,
    dir: String,
    stem: String,
    lang: Lang,
    test: bool,
    generated: bool,
    names: Vec<String>,
    macros: Vec<(usize, String)>,
}

#[derive(Clone)]
struct Func {
    file: String,
    owner: Option<String>,
    name: String,
    line: usize,
    exported: bool,
    trait_impl: bool,
    test: bool,
    delegate: Option<String>,
    external_callee: bool,
    lines: usize,
    size: usize,
    shingles: Vec<u64>,
    canonical: u64,
    words: Vec<String>,
}

impl Func {
    fn key(&self) -> String {
        format!("{}#{}::{}", self.file, self.owner.as_deref().unwrap_or(""), self.name)
    }

    fn label(&self) -> String {
        match &self.owner {
            Some(owner) => format!("{owner}.{}", self.name),
            None => self.name.clone(),
        }
    }
}

struct TypeDecl {
    file: String,
    name: String,
    line: usize,
    methods: Vec<String>,
    decorated: bool,
    mentions: HashSet<String>,
}

struct Impl {
    derived: bool,
    file: String,
    family: String,
    ty: String,
    methods: Vec<String>,
}

struct Call {
    file: String,
    line: usize,
    callee: String,
    func: Option<String>,
    func_name: Option<String>,
    test: bool,
}

struct Registration {
    file: String,
    line: usize,
    end: usize,
    names: BTreeSet<String>,
    binding: Option<String>,
}

struct Branch {
    file: String,
    line: usize,
    func: String,
    func_label: String,
    literal: String,
    idents: HashSet<String>,
}

struct Dynamic {
    file: String,
    line: usize,
    what: String,
}

struct Use {
    file: String,
    line: usize,
    target: Vec<String>,
}

struct Import {
    file: String,
    line: usize,
    spec: String,
}

#[derive(Default)]
struct Tree {
    files: Vec<FileInfo>,
    manifests: Vec<String>,
    funcs: Vec<Func>,
    types: Vec<TypeDecl>,
    impls: Vec<Impl>,
    calls: Vec<Call>,
    regs: Vec<Registration>,
    branches: Vec<Branch>,
    dynamics: Vec<Dynamic>,
    uses: Vec<Use>,
    imports: Vec<Import>,
    policy: bool,
    parse: Duration,
    extract: Duration,
}

fn lang_of(path: &str) -> Option<(Lang, tree_sitter::Language)> {
    if path.ends_with(".d.ts") {
        return None;
    }
    let ext = path.rsplit('.').next()?;
    Some(match ext {
        "rs" => (Lang::Rust, tree_sitter_rust::LANGUAGE.into()),
        "ts" | "mts" | "cts" => (Lang::Ts, tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => (Lang::Ts, tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => return None,
    })
}

fn text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or_default()
}

fn named(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).filter(|child| !child.kind().contains("comment")).collect()
}

fn row(node: Node) -> usize {
    node.start_position().row + 1
}

fn path_is_test(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let (dirs, name) = segments.split_at(segments.len() - 1);
    let name = name[0];
    dirs.iter().any(|dir| TEST_SEGMENTS.contains(dir))
        || name.starts_with("test_")
        || name.ends_with("_test.rs")
        || name == "tests.rs"
        || name.contains(".test.")
        || name.contains(".spec.")
        || name.contains(".stories.")
}

fn is_generated(src: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&src[..src.len().min(2048)]).to_lowercase();
    ["@generated", "do not edit", "auto-generated", "autogenerated", "code generated"]
        .iter()
        .any(|marker| head.contains(marker))
}

fn norm(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn words_of(name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (index, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            continue;
        }
        let boundary = c.is_uppercase()
            && !current.is_empty()
            && (chars[index - 1].is_lowercase() || chars.get(index + 1).is_some_and(|next| next.is_lowercase()));
        if boundary {
            out.push(std::mem::take(&mut current));
        }
        current.extend(c.to_lowercase());
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn last_word(name: &str) -> Option<String> {
    let words = words_of(name);
    (words.len() >= 2).then(|| words.last().cloned()).flatten()
}

fn strip_generics(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 && !c.is_whitespace() => out.push(c),
            _ => {}
        }
    }
    out.replace("?.", ".").replace("::.", ".").trim_end_matches("::").to_string()
}

fn type_name(node: Node, src: &[u8]) -> String {
    let full = strip_generics(text(node, src));
    let full = full.trim_start_matches("dyn").trim_start_matches('&');
    full.rsplit(|c| c == ':' || c == '.').next().unwrap_or_default().to_string()
}

fn fnv(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(0x100000001b3)
}

fn str_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |hash, byte| fnv(hash, byte as u64))
}

const IDENTIFIER_KINDS: &[&str] = &[
    "identifier", "field_identifier", "property_identifier", "type_identifier", "shorthand_property_identifier",
    "private_property_identifier", "shorthand_property_identifier_pattern",
];

const LITERAL_KINDS: &[&str] = &[
    "string_literal", "string", "string_fragment", "raw_string_literal", "integer_literal", "float_literal",
    "number", "char_literal", "template_string", "string_content", "escape_sequence",
];

fn body_facts(body: Node, src: &[u8]) -> (usize, Vec<u64>, u64, Vec<String>, HashSet<String>) {
    let mut kinds = Vec::new();
    let mut canonical = 0xcbf29ce484222325u64;
    let mut words = Vec::new();
    let mut idents = HashSet::new();
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if node.kind().contains("comment") {
            continue;
        }
        if node.is_named() {
            kinds.push(node.kind_id() as u64);
        }
        if node.child_count() == 0 || LITERAL_KINDS.contains(&node.kind()) {
            let token = if IDENTIFIER_KINDS.contains(&node.kind()) {
                let name = text(node, src);
                idents.insert(name.to_string());
                for word in words_of(name) {
                    if word.len() >= 3 && !STOP_WORDS.contains(&word.as_str()) {
                        words.push(word);
                    }
                }
                "I"
            } else if LITERAL_KINDS.contains(&node.kind()) {
                "L"
            } else {
                node.kind()
            };
            canonical = fnv(canonical, str_hash(token));
            if LITERAL_KINDS.contains(&node.kind()) {
                continue;
            }
        }
        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }
    let mut shingles: Vec<u64> = kinds.windows(5).map(|w| w.iter().fold(0xcbf29ce484222325, |h, k| fnv(h, *k))).collect();
    shingles.sort_unstable();
    shingles.dedup();
    (kinds.len(), shingles, canonical, words, idents)
}

fn unwrap_expr(mut node: Node) -> Node {
    loop {
        match node.kind() {
            "try_expression" | "await_expression" | "parenthesized_expression" | "non_null_expression" | "as_expression"
            | "satisfies_expression" | "expression_statement" | "return_expression" | "return_statement" => {
                match named(node).into_iter().find(|child| !child.kind().contains("type")) {
                    Some(child) => node = child,
                    None => return node,
                }
            }
            _ => return node,
        }
    }
}

fn arg_name(node: Node, src: &[u8]) -> Option<String> {
    let node = unwrap_expr(node);
    match node.kind() {
        "identifier" => Some(text(node, src).to_string()),
        "reference_expression" | "unary_expression" => named(node).last().and_then(|inner| arg_name(*inner, src)),
        _ => None,
    }
}

fn params(node: Node, src: &[u8], lang: Lang) -> Option<Vec<String>> {
    let list = node.child_by_field_name("parameters").or_else(|| node.child_by_field_name("parameter"))?;
    if list.kind() == "identifier" {
        return Some(vec![text(list, src).to_string()]);
    }
    let mut out = Vec::new();
    for param in named(list) {
        match (lang, param.kind()) {
            (Lang::Rust, "self_parameter") => {}
            (Lang::Rust, "parameter") => {
                let pattern = param.child_by_field_name("pattern")?;
                let name = text(pattern, src).trim_start_matches("mut ").to_string();
                out.push(name);
            }
            (Lang::Ts, "required_parameter" | "optional_parameter") => {
                let pattern = param.child_by_field_name("pattern")?;
                if pattern.kind() == "this" {
                    continue;
                }
                out.push(text(pattern, src).to_string());
            }
            _ => out.push(format!("<{}>", param.kind())),
        }
    }
    Some(out)
}

fn single_statement(body: Node) -> Option<Node> {
    match body.kind() {
        "block" | "statement_block" => {
            let statements = named(body);
            (statements.len() == 1).then(|| statements[0])
        }
        _ => Some(body),
    }
}

struct Extract<'a> {
    lang: Lang,
    path: &'a str,
    src: &'a [u8],
    test_file: bool,
    test_ranges: Vec<(usize, usize)>,
    external: HashSet<String>,
    crate_modules: HashSet<String>,
    tree: &'a mut Tree,
    info: FileInfo,
}

#[derive(Clone, Default)]
struct Scope {
    owner: Option<String>,
    trait_impl: bool,
    exported: bool,
    func: Option<(String, String)>,
    idents: Option<std::rc::Rc<HashSet<String>>>,
}

impl Extract<'_> {
    fn in_test(&self, line: usize) -> bool {
        self.test_file || self.test_ranges.iter().any(|(from, to)| (*from..=*to).contains(&line))
    }

    fn callee(&self, node: Node) -> String {
        strip_generics(text(node, self.src))
    }

    fn function(&mut self, node: Node, name: String, body: Node, scope: &Scope, exported: bool) -> Scope {
        let line = row(node);
        let (size, shingles, canonical, words, idents) = body_facts(body, self.src);
        let mut delegate = None;
        let mut external_callee = false;
        if let (Some(list), Some(statement)) = (params(node, self.src, self.lang), single_statement(body)) {
            let call = unwrap_expr(statement);
            if call.kind() == "call_expression" {
                if let (Some(function), Some(arguments)) =
                    (call.child_by_field_name("function"), call.child_by_field_name("arguments"))
                {
                    let args: Vec<Option<String>> = named(arguments).into_iter().map(|a| arg_name(a, self.src)).collect();
                    let callee = self.callee(function);
                    let forwards = args.len() == list.len() && args.iter().zip(&list).all(|(a, p)| a.as_deref() == Some(p.as_str()));
                    let recursive = [name.clone(), format!("this.{name}"), format!("self.{name}"), format!("Self::{name}")].contains(&callee);
                    if forwards && !recursive {
                        let root = callee.split(|c| c == '.' || c == ':').next().unwrap_or_default().to_string();
                        external_callee = self.external.contains(&root)
                            || (self.lang == Lang::Ts && TS_GLOBALS.contains(&root.as_str()))
                            || (self.lang == Lang::Rust
                                && callee.contains("::")
                                && root.chars().next().is_some_and(char::is_lowercase)
                                && !["crate", "self", "super"].contains(&root.as_str())
                                && !self.crate_modules.contains(&root));
                        delegate = Some(callee);
                    }
                }
            }
        }
        let func = Func {
            file: self.path.to_string(),
            owner: scope.owner.clone(),
            name: name.clone(),
            line,
            exported,
            trait_impl: scope.trait_impl,
            test: self.in_test(line),
            delegate,
            external_callee,
            lines: node.end_position().row + 1 - line + 1,
            size,
            shingles,
            canonical,
            words,
        };
        let key = func.key();
        let label = func.label();
        self.tree.funcs.push(func);
        Scope { func: Some((key, label)), idents: Some(std::rc::Rc::new(idents)), ..scope.clone() }
    }

    fn registration(&mut self, node: Node, binding: Option<String>) {
        let mut names = BTreeSet::new();
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            if current.child_count() == 0 && (IDENTIFIER_KINDS.contains(&current.kind()) || current.kind() == "identifier") {
                let name = text(current, self.src);
                if name.chars().next().is_some_and(char::is_uppercase) && name.chars().any(char::is_lowercase) {
                    names.insert(name.to_string());
                }
            }
            let mut cursor = current.walk();
            stack.extend(current.children(&mut cursor));
        }
        if names.len() >= 2 {
            self.tree.regs.push(Registration {
                file: self.path.to_string(),
                line: row(node),
                end: node.end_position().row + 1,
                names,
                binding,
            });
        }
    }

    fn binding_of(&self, node: Node, scope: &Scope) -> Option<String> {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "variable_declarator" | "let_declaration" | "const_item" | "static_item" => {
                    let field = if parent.kind() == "let_declaration" { "pattern" } else { "name" };
                    return parent.child_by_field_name(field).map(|n| text(n, self.src).to_string());
                }
                "function_item" | "function_declaration" | "method_definition" => break,
                _ => current = parent.parent(),
            }
        }
        scope.func.as_ref().map(|(_, label)| label.rsplit('.').next().unwrap_or(label).to_string())
    }

    fn branch(&mut self, node: Node, literal: Node, scope: &Scope) {
        if let (Some((key, label)), Some(idents)) = (&scope.func, &scope.idents) {
            self.tree.branches.push(Branch {
                file: self.path.to_string(),
                line: row(node),
                func: key.clone(),
                func_label: label.clone(),
                literal: text(literal, self.src).to_string(),
                idents: (**idents).clone(),
            });
        }
    }

    fn visit(&mut self, node: Node, scope: &Scope) {
        let src = self.src;
        let kind = node.kind();
        let top = node.parent().is_some_and(|p| {
            matches!(p.kind(), "source_file" | "program" | "declaration_list" | "export_statement")
                || (p.kind() == "expression_statement" && p.parent().is_some_and(|g| g.kind() == "source_file"))
        });
        let mut inner = scope.clone();
        match (self.lang, kind) {
            (Lang::Rust, "impl_item") => {
                let ty = node.child_by_field_name("type").map(|t| type_name(t, src)).unwrap_or_default();
                let family = node.child_by_field_name("trait").map(|t| type_name(t, src));
                let methods: Vec<String> = node
                    .child_by_field_name("body")
                    .map(|body| {
                        named(body)
                            .into_iter()
                            .filter(|item| item.kind() == "function_item")
                            .filter_map(|item| item.child_by_field_name("name").map(|n| text(n, src).to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                if let Some(family) = &family {
                    self.tree.impls.push(Impl { derived: false, file: self.path.to_string(), family: family.clone(), ty: ty.clone(), methods: methods.clone() });
                }
                if let Some(decl) = self.tree.types.iter_mut().rev().find(|t| t.name == ty && t.file == self.path) {
                    decl.methods.extend(methods);
                }
                inner.owner = Some(ty);
                inner.trait_impl = family.is_some();
            }
            (Lang::Rust, "struct_item" | "enum_item" | "union_item") => {
                if let Some(name) = node.child_by_field_name("name") {
                    let name = text(name, src).to_string();
                    let mut previous = node.prev_named_sibling();
                    while let Some(attribute) = previous.filter(|p| p.kind() == "attribute_item") {
                        let body = text(attribute, src).replace(' ', "");
                        if let Some(start) = body.find("derive(") {
                            let list = &body[start + 7..body.rfind(')').unwrap_or(body.len())];
                            for derived in list.split(',') {
                                let derived = derived.rsplit("::").next().unwrap_or_default().trim_end_matches(')');
                                if !derived.is_empty() {
                                    self.tree.impls.push(Impl { derived: true, file: self.path.to_string(), family: derived.to_string(), ty: name.clone(), methods: Vec::new() });
                                }
                            }
                        }
                        previous = attribute.prev_named_sibling();
                    }
                    if top {
                        self.info.names.push(name.clone());
                    }
                    let mentions = type_mentions(node, src);
                    self.tree.types.push(TypeDecl { file: self.path.to_string(), name, line: row(node), methods: Vec::new(), decorated: false, mentions });
                }
            }
            (Lang::Rust, "trait_item") => {
                if let Some(name) = node.child_by_field_name("name") {
                    inner.owner = Some(text(name, src).to_string());
                    inner.trait_impl = true;
                    if top {
                        self.info.names.push(text(name, src).to_string());
                    }
                }
            }
            (Lang::Rust, "const_item" | "static_item" | "type_item") => {
                if let (true, Some(name)) = (top, node.child_by_field_name("name")) {
                    self.info.names.push(text(name, src).to_string());
                }
            }
            (Lang::Rust, "function_item") => {
                if let (Some(name), Some(body)) = (node.child_by_field_name("name"), node.child_by_field_name("body")) {
                    let name = text(name, src).to_string();
                    if top && scope.owner.is_none() {
                        self.info.names.push(name.clone());
                    }
                    let exported = named(node).iter().any(|c| c.kind() == "visibility_modifier");
                    inner = self.function(node, name, body, scope, exported);
                }
            }
            (Lang::Rust, "attribute_item") => {
                let body = text(node, src);
                if body.contains("distributed_slice") || body.contains("ctor") {
                    self.tree.dynamics.push(Dynamic { file: self.path.to_string(), line: row(node), what: strip_generics(body) });
                }
            }
            (Lang::Rust, "macro_invocation") => {
                let name = node.child_by_field_name("macro").map(|m| text(m, src).to_string()).unwrap_or_default();
                if top && name != "macro_rules" {
                    self.info.macros.push((row(node), name.clone()));
                }
                if DYNAMIC_MACROS.contains(&name.as_str()) {
                    self.tree.dynamics.push(Dynamic { file: self.path.to_string(), line: row(node), what: format!("{name}!") });
                }
                let binding = self.binding_of(node, scope);
                self.registration(node, binding);
            }
            (Lang::Rust, "array_expression" | "match_expression" | "tuple_expression") => {
                let binding = self.binding_of(node, scope);
                self.registration(node, binding);
            }
            (Lang::Rust, "if_expression") => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    if text(condition, src).contains("==") {
                        if let Some(literal) = string_in(condition) {
                            self.branch(node, literal, scope);
                        }
                    }
                }
            }
            (Lang::Rust, "match_arm") => {
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    if let Some(literal) = named(pattern).into_iter().find(|p| p.kind() == "string_literal") {
                        self.branch(node, literal, scope);
                    }
                }
            }
            (Lang::Rust, "use_declaration") => {
                if let Some(argument) = node.child_by_field_name("argument") {
                    for path in expand_use(&text(argument, src).replace(char::is_whitespace, "")) {
                        self.rust_use(row(node), &path);
                    }
                }
                return;
            }
            (Lang::Rust, "scoped_identifier" | "scoped_type_identifier") => {
                let parent_scoped = node.parent().is_some_and(|p| matches!(p.kind(), "scoped_identifier" | "scoped_type_identifier"));
                let path = strip_generics(text(node, src));
                if !parent_scoped && (path.starts_with("crate::") || path.starts_with("super::")) {
                    self.rust_use(row(node), &path);
                }
            }
            (Lang::Rust, "call_expression") | (Lang::Ts, "call_expression") => {
                if let Some(function) = node.child_by_field_name("function") {
                    let callee = self.callee(function);
                    let args = node.child_by_field_name("arguments");
                    let literal_arg = args.and_then(|a| named(a).into_iter().next()).is_none_or(|a| a.kind() == "string");
                    if self.lang == Lang::Ts && (callee == "import" || callee == "require") && !literal_arg {
                        self.tree.dynamics.push(Dynamic { file: self.path.to_string(), line: row(node), what: format!("{callee}(<computed>)") });
                    }
                    if callee.contains("import.meta.glob") || callee.ends_with("require.context") {
                        self.tree.dynamics.push(Dynamic { file: self.path.to_string(), line: row(node), what: callee.clone() });
                    }
                    if self.lang == Lang::Ts && callee == "require" && literal_arg {
                        if let Some(spec) = args.and_then(|a| named(a).into_iter().next()) {
                            self.tree.imports.push(Import { file: self.path.to_string(), line: row(node), spec: text(spec, src).trim_matches(|c| c == '"' || c == '\'').to_string() });
                        }
                    }
                    let line = row(node);
                    self.tree.calls.push(Call {
                        file: self.path.to_string(),
                        line,
                        callee,
                        func: scope.func.as_ref().map(|(k, _)| k.clone()),
                        func_name: scope.func.as_ref().map(|(_, l)| l.rsplit('.').next().unwrap_or(l).to_string()),
                        test: self.in_test(line),
                    });
                }
            }
            (Lang::Ts, "new_expression") => {
                if let Some(constructor) = node.child_by_field_name("constructor") {
                    let line = row(node);
                    self.tree.calls.push(Call {
                        file: self.path.to_string(),
                        line,
                        callee: format!("new {}", self.callee(constructor)),
                        func: scope.func.as_ref().map(|(k, _)| k.clone()),
                        func_name: scope.func.as_ref().map(|(_, l)| l.rsplit('.').next().unwrap_or(l).to_string()),
                        test: self.in_test(line),
                    });
                }
            }
            (Lang::Ts, "class_declaration" | "abstract_class_declaration" | "class") => {
                let name = node.child_by_field_name("name").map(|n| text(n, src).to_string());
                if let Some(name) = name {
                    let decorated = named(node).iter().any(|c| c.kind() == "decorator")
                        || node.parent().is_some_and(|p| p.kind() == "export_statement" && named(p).iter().any(|c| c.kind() == "decorator"));
                    let body = node.child_by_field_name("body");
                    let methods: Vec<String> = body
                        .map(|b| {
                            named(b)
                                .into_iter()
                                .filter(|m| matches!(m.kind(), "method_definition" | "public_field_definition"))
                                .filter_map(|m| m.child_by_field_name("name").map(|n| text(n, src).to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    let mut heritage = false;
                    for clause in named(node).into_iter().filter(|c| c.kind() == "class_heritage").flat_map(named) {
                        heritage = true;
                        for parent in named(clause) {
                            if parent.kind() == "type_arguments" {
                                continue;
                            }
                            self.tree.impls.push(Impl { derived: false, file: self.path.to_string(), family: type_name(parent, src), ty: name.clone(), methods: methods.clone() });
                        }
                    }
                    if top || node.parent().is_some_and(|p| p.kind() == "export_statement") {
                        self.info.names.push(name.clone());
                    }
                    let mentions = node.child_by_field_name("body").map(|b| type_mentions(b, src)).unwrap_or_default();
                    self.tree.types.push(TypeDecl { file: self.path.to_string(), name: name.clone(), line: row(node), methods, decorated, mentions });
                    inner.owner = Some(name);
                    inner.trait_impl = heritage;
                    inner.exported = ts_exported(node);
                }
            }
            (Lang::Ts, "interface_declaration" | "type_alias_declaration" | "enum_declaration") => {
                if let Some(name) = node.child_by_field_name("name") {
                    if ts_top(node) {
                        self.info.names.push(text(name, src).to_string());
                    }
                }
            }
            (Lang::Ts, "function_declaration" | "generator_function_declaration") => {
                if let (Some(name), Some(body)) = (node.child_by_field_name("name"), node.child_by_field_name("body")) {
                    let name = text(name, src).to_string();
                    if ts_top(node) {
                        self.info.names.push(name.clone());
                    }
                    inner = self.function(node, name, body, scope, ts_exported(node));
                }
            }
            (Lang::Ts, "method_definition") => {
                if let (Some(name), Some(body)) = (node.child_by_field_name("name"), node.child_by_field_name("body")) {
                    let name = text(name, src).to_string();
                    inner = self.function(node, name, body, scope, scope.exported);
                }
            }
            (Lang::Ts, "variable_declarator") => {
                if let (Some(name), Some(value)) = (node.child_by_field_name("name"), node.child_by_field_name("value")) {
                    let name = text(name, src).to_string();
                    if ts_top(node) {
                        self.info.names.push(name.clone());
                    }
                    if matches!(value.kind(), "arrow_function" | "function_expression" | "function") {
                        if let Some(body) = value.child_by_field_name("body") {
                            let exported = ts_exported(node);
                            let fscope = self.function(value, name, body, scope, exported);
                            self.visit(body, &fscope);
                            return;
                        }
                    }
                }
            }
            (Lang::Ts, "array" | "object" | "switch_statement") => {
                let binding = self.binding_of(node, scope);
                self.registration(node, binding);
                if kind == "switch_statement" {
                    if let Some(body) = node.child_by_field_name("body") {
                        for case in named(body).into_iter().filter(|c| c.kind() == "switch_case") {
                            if let Some(value) = case.child_by_field_name("value").filter(|v| v.kind() == "string") {
                                self.branch(case, value, scope);
                            }
                        }
                    }
                }
            }
            (Lang::Ts, "if_statement" | "ternary_expression") => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    if text(condition, src).contains("==") {
                        if let Some(literal) = string_in(condition) {
                            self.branch(node, literal, scope);
                        }
                    }
                }
            }
            (Lang::Ts, "import_statement") | (Lang::Ts, "export_statement") => {
                if let Some(source) = node.child_by_field_name("source") {
                    let spec = text(source, src).trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string();
                    self.tree.imports.push(Import { file: self.path.to_string(), line: row(node), spec });
                }
            }
            (Lang::Ts, "decorator") => {
                inner.exported = scope.exported;
            }
            _ => {}
        }
        for child in named(node) {
            self.visit(child, &inner);
        }
    }

    fn rust_use(&mut self, line: usize, path: &str) {
        let segments: Vec<String> = path.split("::").map(|s| s.to_string()).collect();
        let module = rust_module_path(self.path);
        let target = match segments.first().map(String::as_str) {
            Some("crate") => segments[1..].to_vec(),
            Some("super") => {
                let mut base = module.clone();
                let mut rest = &segments[..];
                while rest.first().map(String::as_str) == Some("super") {
                    base.pop();
                    rest = &rest[1..];
                }
                base.extend(rest.iter().cloned());
                base
            }
            Some("self") => {
                let mut base = module.clone();
                base.extend(segments[1..].iter().cloned());
                base
            }
            _ => return,
        };
        self.tree.uses.push(Use { file: self.path.to_string(), line, target });
    }
}

fn string_in(node: Node) -> Option<Node> {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if matches!(current.kind(), "string_literal" | "string") {
            return Some(current);
        }
        stack.extend(named(current));
    }
    None
}

fn type_mentions(node: Node, src: &[u8]) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == "type_identifier" {
            out.insert(text(current, src).to_string());
        }
        stack.extend(named(current));
    }
    out
}

fn ts_top(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "program" => return true,
            "export_statement" | "lexical_declaration" | "variable_declaration" => current = parent.parent(),
            _ => return false,
        }
    }
    false
}

fn ts_exported(node: Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind() {
            "export_statement" => return true,
            "lexical_declaration" | "variable_declaration" | "variable_declarator" => current = parent.parent(),
            _ => return false,
        }
    }
    false
}

fn expand_use(text: &str) -> Vec<String> {
    let text = text.trim_end_matches(';');
    let Some(open) = text.find('{') else {
        let path = text.split(" as ").next().unwrap_or(text).split("as").next().unwrap_or(text);
        return vec![path.trim_end_matches("::*").to_string()];
    };
    let prefix = &text[..open];
    let inner = &text[open + 1..text.rfind('}').unwrap_or(text.len())];
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in inner.chars() {
        match c {
            '{' => {
                depth += 1;
                current.push(c);
            }
            '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    parts.push(current);
    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .flat_map(|part| expand_use(&format!("{prefix}{part}")))
        .map(|p| p.trim_end_matches("::self").to_string())
        .collect()
}

fn rust_crate_root(path: &str) -> Option<(String, String)> {
    let index = path.rfind("src/").filter(|i| *i == 0 || path.as_bytes()[i - 1] == b'/')?;
    Some((path[..index].to_string(), path[index + 4..].to_string()))
}

fn rust_module_path(path: &str) -> Vec<String> {
    let Some((_, rest)) = rust_crate_root(path) else { return Vec::new() };
    let mut segments: Vec<String> = rest.trim_end_matches(".rs").split('/').map(|s| s.to_string()).collect();
    if matches!(segments.last().map(String::as_str), Some("mod" | "lib" | "main")) {
        segments.pop();
    }
    if segments.first().map(String::as_str) == Some("bin") {
        return vec!["bin".to_string()];
    }
    segments
}

fn external_bindings(lang: Lang, root: Node, src: &[u8]) -> HashSet<String> {
    let mut out = HashSet::new();
    for item in named(root) {
        match (lang, item.kind()) {
            (Lang::Rust, "use_declaration") => {
                if let Some(argument) = item.child_by_field_name("argument") {
                    for path in expand_use(&text(argument, src).replace(char::is_whitespace, "")) {
                        let first = path.split("::").next().unwrap_or_default();
                        if !["crate", "self", "super"].contains(&first) {
                            out.insert(path.rsplit("::").next().unwrap_or_default().to_string());
                            out.insert(first.to_string());
                        }
                    }
                }
            }
            (Lang::Ts, "import_statement") => {
                let spec = item.child_by_field_name("source").map(|s| text(s, src).trim_matches(|c| c == '"' || c == '\'').to_string()).unwrap_or_default();
                if !(spec.starts_with('.') || spec.starts_with('/') || spec.starts_with("@/") || spec.starts_with("~/") || spec.starts_with('#')) {
                    let mut stack = vec![item];
                    while let Some(node) = stack.pop() {
                        if node.kind() == "identifier" {
                            out.insert(text(node, src).to_string());
                        }
                        stack.extend(named(node));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn cfg_test_ranges(root: Node, src: &[u8], out: &mut Vec<(usize, usize)>) {
    let children = named(root);
    for (index, child) in children.iter().enumerate() {
        if child.kind() == "attribute_item" && text(*child, src).replace(' ', "").contains("cfg(test)") {
            if let Some(item) = children[index + 1..].iter().find(|next| next.kind() != "attribute_item") {
                out.push((row(*item), item.end_position().row + 1));
            }
        }
        if child.kind() == "mod_item" {
            if let Some(body) = child.child_by_field_name("body") {
                cfg_test_ranges(body, src, out);
            }
        }
    }
}

fn walk(root: &Path, dir: &Path, sources: &mut Vec<String>, manifests: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if path.is_symlink() {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) {
                walk(root, &path, sources, manifests);
            }
        } else if lang_of(&name).is_some() {
            sources.push(relative);
        } else if name == "package.json" || name == "Cargo.toml" {
            manifests.push(relative);
        }
    }
}

fn read_tree(root: &Path) -> Tree {
    let mut tree = Tree::default();
    let mut sources = Vec::new();
    walk(root, root, &mut sources, &mut tree.manifests);
    tree.policy = std::fs::read_to_string(root.join("klin.json")).is_ok_and(|s| s.contains("\"layers\""));
    let mut crate_modules: HashMap<String, HashSet<String>> = HashMap::new();
    for path in &sources {
        if let Some((krate, rest)) = rust_crate_root(path) {
            let first = rest.split('/').next().unwrap_or_default().trim_end_matches(".rs").to_string();
            crate_modules.entry(krate).or_default().insert(first);
        }
    }
    let mut parser = Parser::new();
    for path in &sources {
        let Some((lang, language)) = lang_of(path) else { continue };
        let Ok(src) = std::fs::read(root.join(path)) else { continue };
        parser.set_language(&language).unwrap();
        let started = Instant::now();
        let Some(parsed) = parser.parse(&src, None) else { continue };
        tree.parse += started.elapsed();
        let started = Instant::now();
        let root_node = parsed.root_node();
        let dir = path.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
        let file_name = path.rsplit('/').next().unwrap_or(path);
        let stem = file_name.split('.').next().unwrap_or(file_name).to_string();
        let mut test_ranges = Vec::new();
        if lang == Lang::Rust {
            cfg_test_ranges(root_node, &src, &mut test_ranges);
        }
        let info = FileInfo {
            path: path.clone(),
            dir,
            stem,
            lang,
            test: path_is_test(path),
            generated: is_generated(&src),
            names: Vec::new(),
            macros: Vec::new(),
        };
        let external = external_bindings(lang, root_node, &src);
        let modules = rust_crate_root(path).and_then(|(k, _)| crate_modules.get(&k).cloned()).unwrap_or_default();
        let mut extract = Extract {
            lang,
            path,
            src: &src,
            test_file: info.test,
            test_ranges,
            external,
            crate_modules: modules,
            tree: &mut tree,
            info,
        };
        extract.visit(root_node, &Scope::default());
        let info = extract.info;
        tree.files.push(info);
        tree.extract += started.elapsed();
    }
    tree
}

struct Finding {
    candidate: &'static str,
    status: &'static str,
    file: String,
    line: usize,
    message: String,
}

fn excluded_file(tree: &Tree, file: &str) -> bool {
    tree.files.iter().find(|f| f.path == file).is_none_or(|f| f.test || f.generated)
}

struct Family<'a> {
    dir: &'a str,
    name: String,
    files: BTreeSet<String>,
    members: BTreeSet<String>,
    suffix: Option<String>,
    methods: BTreeSet<String>,
}

fn sibling_files<'a>(tree: &'a Tree, dir: &str) -> Vec<&'a FileInfo> {
    tree.files
        .iter()
        .filter(|f| f.dir == dir && !f.test && !f.generated && !MODULE_ROOTS.contains(&f.path.rsplit('/').next().unwrap_or_default()))
        .collect()
}

fn shapes(file: &FileInfo) -> BTreeSet<String> {
    let stem = norm(&file.stem);
    let mut out = BTreeSet::new();
    if stem.len() < 2 {
        return out;
    }
    for name in &file.names {
        let name = norm(name);
        if let Some(at) = name.find(&stem) {
            out.insert(format!("{}*{}", &name[..at], &name[at + stem.len()..]));
        }
    }
    out
}

fn families<'a>(tree: &'a Tree) -> Vec<Family<'a>> {
    let mut by_dir: BTreeMap<(&str, &str), (BTreeSet<String>, BTreeSet<String>, Vec<&Impl>)> = BTreeMap::new();
    for imp in &tree.impls {
        let Some(file) = tree.files.iter().find(|f| f.path == imp.file) else { continue };
        if imp.derived || RUST_STD_TRAITS.contains(&imp.family.as_str()) || file.test || file.generated || MODULE_ROOTS.contains(&file.path.rsplit('/').next().unwrap_or_default()) {
            continue;
        }
        let entry = by_dir.entry((file.dir.as_str(), imp.family.as_str())).or_default();
        entry.0.insert(imp.file.clone());
        entry.1.insert(imp.ty.clone());
        entry.2.push(imp);
    }
    let mut out = Vec::new();
    for ((dir, name), (files, members, impls)) in by_dir {
        if files.len() < 2 {
            continue;
        }
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for member in &members {
            if let Some(word) = last_word(member) {
                *counts.entry(word).or_default() += 1;
            }
        }
        let suffix = counts.into_iter().max_by_key(|(_, n)| *n).filter(|(_, n)| *n >= 2 && *n * 2 > members.len()).map(|(w, _)| w);
        let mut methods: Option<BTreeSet<String>> = None;
        for imp in impls.iter().filter(|i| !i.methods.is_empty()) {
            let set: BTreeSet<String> = imp.methods.iter().cloned().collect();
            methods = Some(match methods {
                Some(held) => held.intersection(&set).cloned().collect(),
                None => set,
            });
        }
        out.push(Family { dir, name: name.to_string(), files, members, suffix, methods: methods.unwrap_or_default() });
    }
    out
}

fn family_bypass(before: &Tree, after: &Tree, out: &mut Vec<Finding>) {
    let base_families = families(before);
    let before_types: HashSet<(String, String)> = before
        .types
        .iter()
        .map(|t| (t.file.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default(), t.name.clone()))
        .collect();
    let before_files: HashSet<&str> = before.files.iter().map(|f| f.path.as_str()).collect();
    for ty in &after.types {
        if excluded_file(after, &ty.file) {
            continue;
        }
        let file = after.files.iter().find(|f| f.path == ty.file).unwrap();
        if MODULE_ROOTS.contains(&file.path.rsplit('/').next().unwrap_or_default()) {
            continue;
        }
        if before_types.contains(&(file.dir.clone(), ty.name.clone())) {
            continue;
        }
        let new_file = !before_files.contains(file.path.as_str());
        let file_shapes = shapes(file);
        let mut own_methods: BTreeSet<String> = ty.methods.iter().cloned().collect();
        for imp in after.impls.iter().filter(|i| i.ty == ty.name) {
            own_methods.extend(imp.methods.iter().cloned());
        }
        let implemented: BTreeSet<&str> = after.impls.iter().filter(|i| i.ty == ty.name).map(|i| i.family.as_str()).collect();
        let file_families: BTreeSet<&str> = after.impls.iter().filter(|i| i.file == ty.file).map(|i| i.family.as_str()).collect();
        let siblings = sibling_files(before, &file.dir);
        for family in base_families.iter().filter(|f| f.dir == file.dir) {
            let mut reasons = Vec::new();
            if new_file {
                for shape in &file_shapes {
                    let holders: Vec<&&FileInfo> = siblings.iter().filter(|s| shapes(s).contains(shape)).collect();
                    let members = holders.iter().filter(|h| family.files.contains(&h.path)).count();
                    if holders.len() >= 2 && members * 2 > holders.len() {
                        reasons.push(format!("declares `{shape}` like {} sibling files, {members} of which implement {}", holders.len(), family.name));
                    }
                }
            }
            if let (Some(suffix), Some(word)) = (&family.suffix, last_word(&ty.name)) {
                if *suffix == word {
                    reasons.push(format!("is named `*{}` like {} members", capitalize(suffix), family.members.len()));
                }
            }
            if !family.methods.is_empty() && family.methods.is_subset(&own_methods) {
                reasons.push(format!("defines {} like every member", family.methods.iter().cloned().collect::<Vec<_>>().join(", ")));
            }
            if reasons.is_empty() || ty.mentions.contains(&family.name) {
                continue;
            }
            let conforms = implemented.contains(family.name.as_str()) || (new_file && file_families.contains(family.name.as_str()));
            if conforms {
                continue;
            }
            let instead: Vec<String> = base_families
                .iter()
                .filter(|other| other.dir == file.dir && other.name != family.name && implemented.contains(other.name.as_str()))
                .map(|other| format!("{} ({} siblings)", other.name, other.files.len()))
                .collect();
            let unknown = if ty.decorated {
                Some("the class is decorated, so a framework may register it".to_string())
            } else if file.lang == Lang::Rust && !file.macros.is_empty() {
                Some(format!("the file invokes {} at the top level, which may generate the impl", file.macros.iter().map(|(_, m)| format!("{m}!")).collect::<Vec<_>>().join(", ")))
            } else {
                None
            };
            let mut message = format!(
                "{} in {}/ {}; it does not implement {}, which {} of {} sibling files implement (e.g. {})",
                ty.name,
                file.dir,
                reasons.join("; "),
                family.name,
                family.files.len(),
                siblings.len(),
                family.files.iter().take(2).map(|f| f.rsplit('/').next().unwrap_or(f).to_string()).collect::<Vec<_>>().join(", ")
            );
            if !instead.is_empty() {
                message.push_str(&format!("; it implements {} instead", instead.join(", ")));
            }
            let status = match unknown {
                Some(why) => {
                    message.push_str(&format!("; UNKNOWN: {why}"));
                    "unknown"
                }
                None => "finding",
            };
            out.push(Finding { candidate: "family-bypass", status, file: ty.file.clone(), line: ty.line, message });
        }
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

fn global_families(tree: &Tree) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for imp in &tree.impls {
        if !imp.derived && !RUST_STD_TRAITS.contains(&imp.family.as_str()) && !excluded_file(tree, &imp.file) {
            out.entry(imp.family.clone()).or_default().insert(imp.ty.clone());
        }
    }
    out.retain(|_, members| members.len() >= 2);
    out
}

fn registries<'a>(tree: &'a Tree, members: &BTreeSet<String>) -> Vec<&'a Registration> {
    tree.regs
        .iter()
        .filter(|r| !excluded_file(tree, &r.file) && r.names.intersection(members).count() >= 2)
        .collect()
}

fn dynamic_near<'a>(tree: &'a Tree, files: &[&str]) -> Option<&'a Dynamic> {
    let dirs: HashSet<&str> = files.iter().map(|f| f.rsplit_once('/').map(|(d, _)| d).unwrap_or("")).collect();
    tree.dynamics.iter().find(|d| !excluded_file(tree, &d.file) && dirs.contains(d.file.rsplit_once('/').map(|(d, _)| d).unwrap_or("")))
}

fn registration_bypass(before: &Tree, after: &Tree, out: &mut Vec<Finding>) {
    let base = global_families(before);
    let now = global_families(after);
    for (family, members) in &now {
        let Some(held) = base.get(family) else { continue };
        let base_regs = registries(before, held);
        if base_regs.is_empty() {
            continue;
        }
        let after_regs = registries(after, members);
        for member in members.difference(held) {
            let Some(imp) = after.impls.iter().find(|i| &i.family == family && &i.ty == member) else { continue };
            if excluded_file(after, &imp.file) || after_regs.iter().any(|r| r.names.contains(member)) {
                continue;
            }
            let direct: Vec<&Call> = after
                .calls
                .iter()
                .filter(|c| !c.test && c.file != imp.file && c.callee.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).any(|s| s == member))
                .collect();
            let Some(first) = direct.first() else { continue };
            let reg = base_regs[0];
            let mut files: Vec<&str> = vec![imp.file.as_str()];
            files.extend(base_regs.iter().map(|r| r.file.as_str()));
            let mut message = format!(
                "{member} implements {family}, but no registration lists it; {}:{} lists {} other members ({}); {}:{} uses {member} directly",
                reg.file,
                reg.line,
                reg.names.intersection(held).count(),
                reg.names.intersection(held).take(3).cloned().collect::<Vec<_>>().join(", "),
                first.file,
                first.line
            );
            let status = match dynamic_near(after, &files) {
                Some(d) => {
                    message.push_str(&format!("; UNKNOWN: {}:{} discovers members at run time ({})", d.file, d.line, d.what));
                    "unknown"
                }
                None => "finding",
            };
            out.push(Finding { candidate: "registration-bypass", status, file: first.file.clone(), line: first.line, message });
        }
        let bindings: BTreeSet<&str> = base_regs.iter().filter_map(|r| r.binding.as_deref()).collect();
        let held_branches: HashSet<(String, String)> = before.branches.iter().map(|b| (b.func.clone(), b.literal.clone())).collect();
        for branch in &after.branches {
            if excluded_file(after, &branch.file) || held_branches.contains(&(branch.func.clone(), branch.literal.clone())) {
                continue;
            }
            if after_regs.iter().any(|r| r.file == branch.file && (r.line..=r.end).contains(&branch.line)) {
                continue;
            }
            let Some(binding) = bindings.iter().find(|b| branch.idents.contains(**b)) else { continue };
            let reg = base_regs.iter().find(|r| r.binding.as_deref() == Some(*binding)).unwrap();
            let mut files: Vec<&str> = vec![branch.file.as_str()];
            files.push(reg.file.as_str());
            let mut message = format!(
                "{} dispatches {family} through `{binding}` ({}:{}, {} members); the new branch on {} handles one case outside it",
                branch.func_label,
                reg.file,
                reg.line,
                reg.names.intersection(held).count(),
                branch.literal
            );
            let status = match dynamic_near(after, &files) {
                Some(d) => {
                    message.push_str(&format!("; UNKNOWN: {}:{} discovers members at run time ({})", d.file, d.line, d.what));
                    "unknown"
                }
                None => "finding",
            };
            out.push(Finding { candidate: "registration-bypass", status, file: branch.file.clone(), line: branch.line, message });
        }
    }
}

fn specific(callee: &str) -> bool {
    if callee.contains('(') || callee.contains('[') || callee.starts_with("new ") {
        return false;
    }
    let root = callee.split(|c| c == '.' || c == ':').next().unwrap_or_default();
    if ["this", "self", "super", "Self"].contains(&root) {
        return false;
    }
    callee.split(|c| c == '.' || c == ':').filter(|s| !s.is_empty()).count() >= 3
}

fn wrapper_bypass(before: &Tree, after: &Tree, out: &mut Vec<Finding>) {
    let mut sites: BTreeMap<&str, Vec<&Call>> = BTreeMap::new();
    for call in before.calls.iter().filter(|c| !c.test && specific(&c.callee) && !excluded_file(before, &c.file)) {
        sites.entry(call.callee.as_str()).or_default().push(call);
    }
    let funcs: HashMap<String, &Func> = before.funcs.iter().map(|f| (f.key(), f)).collect();
    let held: HashSet<(String, String)> = before.calls.iter().map(|c| (c.func.clone().unwrap_or_default(), c.callee.clone())).collect();
    for call in after.calls.iter().filter(|c| !c.test && specific(&c.callee) && !excluded_file(after, &c.file)) {
        let key = (call.func.clone().unwrap_or_default(), call.callee.clone());
        if held.contains(&key) {
            continue;
        }
        let Some(base_sites) = sites.get(call.callee.as_str()) else { continue };
        let owners: BTreeSet<&str> = base_sites.iter().filter_map(|c| c.func.as_deref()).collect();
        if owners.len() != 1 || base_sites.iter().any(|c| c.func.is_none()) {
            continue;
        }
        let owner = owners.into_iter().next().unwrap();
        let Some(wrapper) = funcs.get(owner) else { continue };
        if !wrapper.exported || wrapper.lines > 15 || call.func.as_deref() == Some(owner) {
            continue;
        }
        let callers = before
            .calls
            .iter()
            .filter(|c| c.func.as_deref() != Some(owner) && c.callee.rsplit(|ch| ch == '.' || ch == ':').next() == Some(wrapper.name.as_str()))
            .count();
        if callers == 0 {
            continue;
        }
        out.push(Finding {
            candidate: "wrapper-bypass",
            status: "finding",
            file: call.file.clone(),
            line: call.line,
            message: format!(
                "{} calls {} directly; the base calls it only inside {} ({}:{}), which {} call site(s) use",
                call.func_name.as_deref().unwrap_or("top-level code"),
                call.callee,
                wrapper.label(),
                wrapper.file,
                wrapper.line,
                callers
            ),
        });
    }
}

fn jaccard(a: &[u64], b: &[u64]) -> f64 {
    let (mut i, mut j, mut both) = (0, 0, 0usize);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                both += 1;
                i += 1;
                j += 1;
            }
        }
    }
    let union = a.len() + b.len() - both;
    if union == 0 { 0.0 } else { both as f64 / union as f64 }
}

const MIN_SIZE: usize = 50;
const STRUCTURAL: f64 = 0.7;
const LEXICAL: f64 = 0.7;

fn near_clone(before: &Tree, after: &Tree, out: &mut Vec<Finding>) {
    let held: HashSet<String> = before.funcs.iter().map(Func::key).collect();
    let existing: Vec<&Func> = after.funcs.iter().filter(|f| !f.test && held.contains(&f.key()) && f.size >= MIN_SIZE && !excluded_file(after, &f.file)).collect();
    let fresh: Vec<&Func> = after.funcs.iter().filter(|f| !f.test && !held.contains(&f.key()) && f.size >= MIN_SIZE && !excluded_file(after, &f.file)).collect();
    if fresh.is_empty() {
        return;
    }
    let mut df: HashMap<&str, usize> = HashMap::new();
    let all: Vec<&Func> = after.funcs.iter().filter(|f| !f.test).collect();
    for func in &all {
        let unique: HashSet<&str> = func.words.iter().map(String::as_str).collect();
        for word in unique {
            *df.entry(word).or_default() += 1;
        }
    }
    let total = all.len().max(1) as f64;
    let vector = |func: &Func| -> HashMap<String, f64> {
        let mut tf: HashMap<String, f64> = HashMap::new();
        for word in &func.words {
            *tf.entry(word.clone()).or_default() += 1.0;
        }
        for (word, weight) in tf.iter_mut() {
            *weight *= (total / *df.get(word.as_str()).unwrap_or(&1) as f64).ln();
        }
        tf
    };
    let cosine = |a: &HashMap<String, f64>, b: &HashMap<String, f64>| -> f64 {
        let dot: f64 = a.iter().filter_map(|(w, x)| b.get(w).map(|y| x * y)).sum();
        let na: f64 = a.values().map(|x| x * x).sum::<f64>().sqrt();
        let nb: f64 = b.values().map(|x| x * x).sum::<f64>().sqrt();
        if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
    };
    let vectors: Vec<HashMap<String, f64>> = existing.iter().map(|f| vector(f)).collect();
    for func in fresh {
        let mine = vector(func);
        let mut best_structural: Option<(f64, &Func)> = None;
        let mut best_lexical: Option<(f64, &Func)> = None;
        let mut canonical: Option<&Func> = None;
        for (index, other) in existing.iter().enumerate() {
            if other.canonical == func.canonical {
                canonical = Some(other);
                continue;
            }
            let s = jaccard(&func.shingles, &other.shingles);
            if best_structural.is_none_or(|(b, _)| s > b) {
                best_structural = Some((s, other));
            }
            let l = cosine(&mine, &vectors[index]);
            if best_lexical.is_none_or(|(b, _)| l > b) {
                best_lexical = Some((l, other));
            }
        }
        if std::env::var_os("RELATIONS_DEBUG").is_some() {
            eprintln!(
                "near-clone {} structural {:.2} {} lexical {:.2} {}",
                func.label(),
                best_structural.map_or(0.0, |(s, _)| s),
                best_structural.map(|(_, o)| o.label()).unwrap_or_default(),
                best_lexical.map_or(0.0, |(s, _)| s),
                best_lexical.map(|(_, o)| o.label()).unwrap_or_default()
            );
        }
        if let Some(other) = canonical {
            out.push(Finding {
                candidate: "near-clone",
                status: "canonical",
                file: func.file.clone(),
                line: func.line,
                message: format!("{} has the canonical body of {} ({}:{}); #48 owns this", func.label(), other.label(), other.file, other.line),
            });
            continue;
        }
        if let Some((score, other)) = best_structural.filter(|(s, _)| *s >= STRUCTURAL) {
            out.push(Finding {
                candidate: "near-clone",
                status: "structural",
                file: func.file.clone(),
                line: func.line,
                message: format!("{} has {:.2} of its syntax shape in common with {} ({}:{})", func.label(), score, other.label(), other.file, other.line),
            });
        }
        if let Some((score, other)) = best_lexical.filter(|(s, _)| *s >= LEXICAL) {
            out.push(Finding {
                candidate: "near-clone",
                status: "lexical",
                file: func.file.clone(),
                line: func.line,
                message: format!("{} has a name-token similarity of {:.2} to {} ({}:{})", func.label(), score, other.label(), other.file, other.line),
            });
        }
    }
}

fn delegation(before: &Tree, after: &Tree, out: &mut Vec<Finding>) {
    let held: HashSet<String> = before.funcs.iter().map(Func::key).collect();
    let mut layers: BTreeMap<(String, String), (usize, usize, usize)> = BTreeMap::new();
    for func in after.funcs.iter().filter(|f| !held.contains(&f.key())) {
        if let Some(owner) = &func.owner {
            let entry = layers.entry((func.file.clone(), owner.clone())).or_insert((0, 0, func.line));
            entry.0 += 1;
            if func.delegate.is_some() && !func.trait_impl && !func.external_callee {
                entry.1 += 1;
            }
        }
    }
    for func in after.funcs.iter().filter(|f| !held.contains(&f.key())) {
        let Some(callee) = &func.delegate else { continue };
        if func.test || excluded_file(after, &func.file) {
            continue;
        }
        let file_name = func.file.rsplit('/').next().unwrap_or_default();
        let mut tags = Vec::new();
        if func.trait_impl {
            tags.push("trait-impl");
        }
        if func.external_callee {
            tags.push("external");
        }
        if MODULE_ROOTS.contains(&file_name) && func.exported {
            tags.push("facade");
        }
        let status = if tags.is_empty() { "finding" } else { "excluded" };
        let layer = func
            .owner
            .as_ref()
            .and_then(|o| layers.get(&(func.file.clone(), o.clone())))
            .filter(|(all, forwarding, _)| *all >= 2 && all == forwarding)
            .map(|(all, _, _)| format!("; every one of the {all} new methods of {} only forwards", func.owner.as_deref().unwrap_or_default()))
            .unwrap_or_default();
        out.push(Finding {
            candidate: "delegation-only",
            status,
            file: func.file.clone(),
            line: func.line,
            message: format!(
                "{} only passes its arguments to {}; it adds no check, default or conversion{}{}",
                func.label(),
                callee,
                layer,
                if tags.is_empty() { String::new() } else { format!(" [{}]", tags.join(",")) }
            ),
        });
    }
}

struct Graph {
    edges: BTreeMap<(String, String, String), Vec<(String, usize)>>,
    unresolved: usize,
}

fn ts_root(tree: &Tree, path: &str) -> Option<String> {
    let package = tree
        .manifests
        .iter()
        .filter(|m| m.ends_with("package.json"))
        .map(|m| m.trim_end_matches("package.json").to_string())
        .filter(|dir| path.starts_with(dir.as_str()))
        .max_by_key(String::len)?;
    let src = format!("{package}src/");
    Some(if tree.files.iter().any(|f| f.path.starts_with(&src)) { src } else { package })
}

fn component_of(root: &str, path: &str) -> Option<String> {
    let rest = path.strip_prefix(root)?;
    let mut segments = rest.split('/');
    let first = segments.next()?;
    Some(if segments.next().is_some() { first.to_string() } else { "(root)".to_string() })
}

fn normalize(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            _ => out.push(segment),
        }
    }
    out.join("/")
}

fn graph(tree: &Tree) -> Graph {
    let mut edges: BTreeMap<(String, String, String), Vec<(String, usize)>> = BTreeMap::new();
    let mut unresolved = 0;
    let modules: HashMap<String, HashSet<String>> = tree.files.iter().filter_map(|f| rust_crate_root(&f.path)).fold(HashMap::new(), |mut acc, (krate, rest)| {
        acc.entry(krate).or_default().insert(rest.split('/').next().unwrap_or_default().trim_end_matches(".rs").to_string());
        acc
    });
    for item in &tree.uses {
        if excluded_file(tree, &item.file) {
            continue;
        }
        let Some((krate, _)) = rust_crate_root(&item.file) else { continue };
        let from = rust_module_path(&item.file).first().cloned().unwrap_or_else(|| "(root)".to_string());
        let to = match item.target.first() {
            Some(first) if modules.get(&krate).is_some_and(|m| m.contains(first)) => first.clone(),
            Some(_) => "(root)".to_string(),
            None => continue,
        };
        if from != to {
            edges.entry((krate, from, to)).or_default().push((item.file.clone(), item.line));
        }
    }
    for item in &tree.imports {
        if excluded_file(tree, &item.file) {
            continue;
        }
        let Some(root) = ts_root(tree, &item.file) else { continue };
        let target = if item.spec.starts_with('.') {
            let dir = item.file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
            normalize(&format!("{dir}/{}", item.spec))
        } else if item.spec.starts_with("@/") || item.spec.starts_with("~/") || item.spec.starts_with('#') || item.spec.starts_with("src/") {
            unresolved += 1;
            continue;
        } else {
            continue;
        };
        let Some(from) = component_of(&root, &item.file) else { continue };
        let target_path = if tree.files.iter().any(|f| f.path.starts_with(&format!("{target}/"))) { format!("{target}/index.ts") } else { target };
        let Some(to) = component_of(&root, &target_path) else { continue };
        if from != to {
            edges.entry((root, from, to)).or_default().push((item.file.clone(), item.line));
        }
    }
    Graph { edges, unresolved }
}

fn reaches(edges: &BTreeSet<(String, String)>, from: &str, to: &str) -> bool {
    let mut seen = HashSet::new();
    let mut stack = vec![from.to_string()];
    while let Some(node) = stack.pop() {
        if node == to {
            return true;
        }
        if seen.insert(node.clone()) {
            stack.extend(edges.iter().filter(|(a, _)| *a == node).map(|(_, b)| b.clone()));
        }
    }
    false
}

fn path_of(edges: &BTreeSet<(String, String)>, from: &str, to: &str) -> Vec<String> {
    let mut previous: HashMap<String, String> = HashMap::new();
    let mut queue = std::collections::VecDeque::from([from.to_string()]);
    let mut seen = HashSet::from([from.to_string()]);
    while let Some(node) = queue.pop_front() {
        if node == to {
            let mut path = vec![node.clone()];
            let mut current = node;
            while let Some(p) = previous.get(&current) {
                path.push(p.clone());
                current = p.clone();
            }
            path.reverse();
            return path;
        }
        for (_, next) in edges.iter().filter(|(a, _)| *a == node) {
            if seen.insert(next.clone()) {
                previous.insert(next.clone(), node.clone());
                queue.push_back(next.clone());
            }
        }
    }
    Vec::new()
}

fn component_cycle(before: &Tree, after: &Tree, out: &mut Vec<Finding>) {
    let old = graph(before);
    let new = graph(after);
    let pairs = |g: &Graph, root: &str| -> BTreeSet<(String, String)> {
        g.edges.keys().filter(|(r, _, _)| r == root).map(|(_, a, b)| (a.clone(), b.clone())).collect()
    };
    let roots: BTreeSet<&String> = new.edges.keys().map(|(r, _, _)| r).collect();
    for root in roots {
        let base = pairs(&old, root);
        let now = pairs(&new, root);
        for (a, b) in now.difference(&base) {
            if !reaches(&now, b, a) || (reaches(&base, a, b) && reaches(&base, b, a)) {
                continue;
            }
            let sites = &new.edges[&(root.clone(), a.clone(), b.clone())];
            let back = path_of(&now, b, a);
            let back_count: usize = back.windows(2).map(|w| new.edges.get(&(root.clone(), w[0].clone(), w[1].clone())).map_or(0, Vec::len)).sum();
            let example = back
                .windows(2)
                .next()
                .and_then(|w| new.edges.get(&(root.clone(), w[0].clone(), w[1].clone())))
                .and_then(|s| s.first())
                .map(|(f, l)| format!(", e.g. {f}:{l}"))
                .unwrap_or_default();
            let status = if after.policy { "policy" } else { "finding" };
            out.push(Finding {
                candidate: "component-cycle",
                status,
                file: sites[0].0.clone(),
                line: sites[0].1,
                message: format!(
                    "new dependency {a} -> {b} in {} closes the cycle {} ({} imports{}){}",
                    if root.is_empty() { "." } else { root.trim_end_matches('/') },
                    back.join(" -> "),
                    back_count,
                    example,
                    if after.policy { "; klin.json declares layers, so layering owns this edge" } else { "" }
                ),
            });
        }
    }
    if new.unresolved > old.unresolved {
        out.push(Finding {
            candidate: "component-cycle",
            status: "unknown",
            file: String::new(),
            line: 0,
            message: format!("{} imports use a path alias that the prototype does not resolve ({} in the base)", new.unresolved, old.unresolved),
        });
    }
}

fn print(findings: &mut [Finding]) {
    findings.sort_by(|a, b| (a.candidate, &a.file, a.line, a.status, &a.message).cmp(&(b.candidate, &b.file, b.line, b.status, &b.message)));
    for f in findings.iter() {
        println!("{}\t{}\t{}:{}\t{}", f.candidate, f.status, f.file, f.line, f.message);
    }
}

fn main() {
    let worker = std::thread::Builder::new().stack_size(256 << 20).spawn(run).unwrap();
    std::process::exit(worker.join().unwrap_or(101));
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("new") if args.len() == 3 => {
            let before = read_tree(Path::new(&args[1]));
            let after = read_tree(Path::new(&args[2]));
            let mut findings = Vec::new();
            family_bypass(&before, &after, &mut findings);
            registration_bypass(&before, &after, &mut findings);
            wrapper_bypass(&before, &after, &mut findings);
            near_clone(&before, &after, &mut findings);
            delegation(&before, &after, &mut findings);
            component_cycle(&before, &after, &mut findings);
            print(&mut findings);
        }
        Some("time") if args.len() == 2 => {
            let started = Instant::now();
            let tree = read_tree(Path::new(&args[1]));
            let read = started.elapsed();
            let started = Instant::now();
            let _ = families(&tree);
            let _ = global_families(&tree);
            let g = graph(&tree);
            let index = started.elapsed();
            let impl_bytes: usize = tree.impls.iter().map(|i| i.file.len() + i.family.len() + i.ty.len() + i.methods.iter().map(String::len).sum::<usize>()).sum();
            let call_bytes: usize = tree.calls.iter().filter(|c| specific(&c.callee)).map(|c| c.callee.len() + c.func.as_ref().map_or(0, String::len) + 8).sum();
            let reg_bytes: usize = tree.regs.iter().map(|r| r.names.iter().map(String::len).sum::<usize>() + r.file.len() + 16).sum();
            let shingle_bytes: usize = tree.funcs.iter().filter(|f| f.size >= MIN_SIZE).map(|f| f.shingles.len() * 8).sum();
            println!(
                "files\t{}\tparse_ms\t{:.1}\textract_ms\t{:.1}\tread_ms\t{:.1}\tindex_ms\t{:.1}\timpls\t{}\timpl_bytes\t{}\tspecific_calls\t{}\tcall_bytes\t{}\tregistrations\t{}\treg_bytes\t{}\tfunctions\t{}\tshingle_bytes\t{}\tcomponent_edges\t{}",
                tree.files.len(),
                tree.parse.as_secs_f64() * 1e3,
                tree.extract.as_secs_f64() * 1e3,
                read.as_secs_f64() * 1e3,
                index.as_secs_f64() * 1e3,
                tree.impls.len(),
                impl_bytes,
                tree.calls.iter().filter(|c| specific(&c.callee)).count(),
                call_bytes,
                tree.regs.len(),
                reg_bytes,
                tree.funcs.len(),
                shingle_bytes,
                g.edges.len()
            );
        }
        _ => {
            eprintln!("usage: relations new BEFORE AFTER | relations time ROOT");
            return 2;
        }
    }
    0
}
