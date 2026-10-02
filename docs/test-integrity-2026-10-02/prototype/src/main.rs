use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tree_sitter::{Node, Parser};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    Rust,
    Ts,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Level {
    Tautology,
    Exists,
    Partial,
    Exact,
}

impl Level {
    fn name(self) -> &'static str {
        match self {
            Level::Tautology => "tautology",
            Level::Exists => "existence",
            Level::Partial => "partial",
            Level::Exact => "exact",
        }
    }
}

#[derive(Clone, Debug)]
struct Check {
    family: &'static str,
    level: Option<Level>,
    error: bool,
    actual: String,
    expected: String,
    args: Vec<String>,
    text: String,
}

impl Check {
    fn key(&self) -> String {
        format!("{}|{}|{}", self.family, self.actual, self.expected)
    }
}

#[derive(Clone, Debug, Default)]
struct Test {
    id: String,
    line: usize,
    checks: Vec<Check>,
    implicit: usize,
    smoke: bool,
    statements: usize,
    disabled: Vec<(&'static str, bool)>,
    body: String,
    title: String,
}

#[derive(Default)]
struct FileFacts {
    tests: Vec<Test>,
    mocks: Vec<(String, String, usize)>,
    asserted_names: HashSet<String>,
}

struct Site {
    candidate: &'static str,
    shape: String,
    file: String,
    line: usize,
    tags: Vec<&'static str>,
    test: String,
    text: String,
}

const SKIP_DIRS: &[&str] = &[
    ".git", "node_modules", "vendor", "build", ".build", "dist", "target", "__pycache__", ".venv",
    "venv", "coverage", ".next", "out", "fixtures",
];

const TEST_SEGMENTS: &[&str] = &["test", "tests", "__tests__", "spec", "specs", "testing", "e2e", "__mocks__", "mocks"];

const HELPER_PREFIXES: &[&str] = &["assert", "expect", "check", "verify", "ensure", "should"];

const SMOKE_WORDS: &[&str] = &[
    "not throw", "not crash", "not fail", "not panic", "not error", "without throwing", "without crashing",
    "without error", "without errors", "without failing", "smoke", "does not reject", "doesn't throw",
    "doesn't crash", "should render", "renders",
];

const TS_EXACT: &[&str] = &[
    "toBe", "toEqual", "toStrictEqual", "toMatchSnapshot", "toMatchInlineSnapshot", "toMatchFileSnapshot",
    "toHaveBeenCalledWith", "toBeCalledWith", "toHaveBeenLastCalledWith", "lastCalledWith",
    "toHaveBeenNthCalledWith", "nthCalledWith", "toHaveReturnedWith", "toBeNull", "toBeUndefined", "toBeNaN",
    "toHaveLength", "toBeCloseTo", "toHaveBeenCalledTimes", "toBeCalledTimes", "toHaveBeenCalledOnce",
    "toHaveBeenCalledExactlyOnceWith", "toBeTrue", "toBeFalse", "toHaveValue", "toHaveText", "toHaveURL",
    "toHaveTitle", "toHaveTextContent", "toHaveCount", "equal", "eql", "equals", "strictEqual", "deepEqual",
    "deepStrictEqual", "is",
];

const TS_EXISTS: &[&str] = &[
    "toBeDefined", "toBeTruthy", "toBeFalsy", "toBeInTheDocument", "toBeVisible", "toHaveBeenCalled",
    "toBeCalled", "toExist", "toBeAttached", "ok", "exist",
];

const TS_ERROR: &[&str] = &[
    "toThrow", "toThrowError", "toThrowErrorMatchingSnapshot", "toThrowErrorMatchingInlineSnapshot", "throws",
    "rejects", "toReject",
];

fn lang_of(path: &str) -> Option<(Lang, tree_sitter::Language)> {
    let ext = path.rsplit('.').next()?;
    Some(match ext {
        "rs" => (Lang::Rust, tree_sitter_rust::LANGUAGE.into()),
        "ts" | "mts" | "cts" => (Lang::Ts, tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => (Lang::Ts, tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => return None,
    })
}

fn path_is_test(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let (dirs, name) = segments.split_at(segments.len() - 1);
    let name = name[0];
    dirs.iter().any(|dir| TEST_SEGMENTS.contains(dir))
        || name.ends_with("_test.rs")
        || name == "tests.rs"
        || name.contains(".test.")
        || name.contains(".spec.")
}

fn text<'a>(node: Node, src: &'a [u8]) -> &'a str {
    node.utf8_text(src).unwrap_or_default()
}

fn named_children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn squash(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_whitespace() {
            continue;
        }
        out.push(if c == '\'' || c == '`' { '"' } else { c });
    }
    out.trim_end_matches(',').to_string()
}

fn split_top(inner: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    let mut chars = inner.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            current.push(c);
            if c == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '`' => {
                quote = Some(c);
                current.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn strip_strings(text: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == '\\' {
                chars.next();
            } else if c == q {
                quote = None;
                out.push('S');
            }
            continue;
        }
        if c == '"' || c == '`' || (c == '\'' && !out.ends_with(|p: char| p.is_alphanumeric())) {
            quote = Some(c);
            continue;
        }
        out.push(c);
    }
    out
}

fn identifiers(text: &str) -> Vec<String> {
    let stripped = strip_strings(text);
    let mut out = Vec::new();
    let mut current = String::new();
    let mut prev_digit = false;
    for c in stripped.chars() {
        if c.is_alphanumeric() || c == '_' {
            if current.is_empty() && c.is_ascii_digit() {
                prev_digit = true;
            }
            if current.is_empty() && prev_digit {
                if !c.is_alphanumeric() && c != '_' {
                    prev_digit = false;
                }
                continue;
            }
            current.push(c);
        } else {
            prev_digit = false;
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn is_literal(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let stripped = strip_strings(t);
    let allowed = ["true", "false", "None", "Some", "Ok", "null", "undefined", "vec", "String", "from", "to_string", "to_owned", "into"];
    let keyed: Vec<String> = stripped
        .split(|c: char| c == ',' || c == '{')
        .filter_map(|part| part.split_once(':').map(|(key, _)| key.trim().to_string()))
        .collect();
    identifiers(t).iter().all(|id| allowed.contains(&id.as_str()) || keyed.contains(id))
}

fn literal_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut quote: Option<char> = None;
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    let mut number = String::new();
    let mut prev_ident = false;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == '\\' {
                current.push(c);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else if c == q {
                out.push(format!("\"{current}\""));
                current.clear();
                quote = None;
            } else {
                current.push(c);
            }
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            quote = Some(c);
            continue;
        }
        if c.is_ascii_digit() && !prev_ident || (!number.is_empty() && (c.is_ascii_digit() || c == '.' || c == '_')) {
            number.push(c);
            continue;
        }
        if !number.is_empty() {
            out.push(number.trim_end_matches('.').to_string());
            number.clear();
        }
        prev_ident = c.is_alphanumeric() || c == '_';
    }
    if !number.is_empty() {
        out.push(number);
    }
    out
}

fn tautology_text(actual: &str, expected: &str) -> bool {
    let a = squash(actual);
    let e = squash(expected);
    if !a.is_empty() && a == e {
        return true;
    }
    if is_literal(actual) && (expected.is_empty() || is_literal(expected)) {
        return true;
    }
    false
}

fn condition_tautology(cond: &str) -> bool {
    let c = squash(cond);
    if c == "true" || c == "!false" || c.ends_with("||true") || c.starts_with("true||") {
        return true;
    }
    if (c.ends_with(".len()>=0") || c.ends_with(".length>=0")) && !c.contains("&&") {
        return true;
    }
    for op in ["==", "==="] {
        if let Some((left, right)) = split_operator(cond, op) {
            return tautology_text(&left, &right);
        }
    }
    is_literal(cond)
}

fn split_operator(cond: &str, op: &str) -> Option<(String, String)> {
    let bytes = cond.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = quote {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' | b'`' => quote = Some(c),
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 && cond[i..].starts_with(op) {
            let before = if i > 0 { bytes[i - 1] } else { b' ' };
            let after = bytes.get(i + op.len()).copied().unwrap_or(b' ');
            let clean = match op {
                "==" | "!=" => before != b'=' && before != b'!' && before != b'<' && before != b'>' && after != b'=',
                "===" | "!==" => after != b'=',
                ">=" | "<=" => before != b'=' && before != b'-' && after != b'=',
                ">" | "<" => before == b' ' && after == b' ',
                _ => true,
            };
            if clean {
                return Some((cond[..i].trim().to_string(), cond[i + op.len()..].trim().to_string()));
            }
        }
        i += 1;
    }
    None
}

fn mentions_error(text: &str) -> bool {
    let t = squash(text);
    ["unwrap_err", "expect_err", "is_err()", "Err(", "should_panic", "toThrow", ".rejects", "catch_unwind"]
        .iter()
        .any(|needle| t.contains(needle))
}

fn classify_condition(cond: &str, family: &'static str, text: &str) -> Check {
    let mut check = Check {
        family,
        level: Some(Level::Exact),
        error: mentions_error(cond),
        actual: squash(cond),
        expected: String::new(),
        args: vec![squash(cond)],
        text: squash(text),
    };
    if condition_tautology(cond) {
        check.level = Some(Level::Tautology);
        return check;
    }
    for op in ["===", "=="] {
        if let Some((left, right)) = split_operator(cond, op) {
            let (actual, expected) = if is_literal(&left) && !is_literal(&right) { (right, left) } else { (left, right) };
            check.family = "eq";
            check.error = mentions_error(&actual) || mentions_error(&expected);
            check.actual = squash(&actual);
            check.expected = squash(&expected);
            return check;
        }
    }
    let c = squash(cond);
    if ["is_some()", "is_ok()", "!is_empty()", ".is_empty()==false", ".length>0", ".len()>0", "!=null", "!==null", "!==undefined"]
        .iter()
        .any(|needle| c.contains(needle))
        && !check.error
        || c.starts_with('!') && c.ends_with(".is_empty()")
    {
        check.level = Some(Level::Exists);
        return check;
    }
    for op in ["!==", "!=", ">=", "<=", ">", "<"] {
        if let Some((left, right)) = split_operator(cond, op) {
            check.level = Some(Level::Partial);
            check.actual = squash(&left);
            check.expected = squash(&right);
            return check;
        }
    }
    if ["contains(", "starts_with(", "ends_with(", "includes(", "startsWith(", "endsWith(", "matches!(", ".test("]
        .iter()
        .any(|needle| c.contains(needle))
        && !check.error
    {
        check.level = Some(Level::Partial);
    }
    check
}

struct Ctx<'a> {
    lang: Lang,
    src: &'a [u8],
    helpers: HashMap<String, Vec<Check>>,
}

fn line_of(node: Node) -> usize {
    node.start_position().row + 1
}

fn rust_macro_name(node: Node, src: &[u8]) -> String {
    node.child_by_field_name("macro")
        .map(|m| text(m, src).rsplit("::").next().unwrap_or_default().to_string())
        .unwrap_or_default()
}

fn token_args(node: Node, src: &[u8]) -> Vec<String> {
    let Some(tree) = named_children(node).into_iter().find(|c| c.kind() == "token_tree") else { return Vec::new() };
    let t = text(tree, src);
    let inner = &t[1..t.len().saturating_sub(1)];
    split_top(inner)
}

fn rust_macro_check(node: Node, ctx: &Ctx) -> Option<Check> {
    let name = rust_macro_name(node, ctx.src);
    let args = token_args(node, ctx.src);
    let whole = text(node, ctx.src);
    let base = name.trim_start_matches("prop_").trim_start_matches("debug_");
    match base {
        "assert_eq" | "assert_str_eq" => {
            let left = args.first().cloned().unwrap_or_default();
            let right = args.get(1).cloned().unwrap_or_default();
            let (actual, expected) = if is_literal(&left) && !is_literal(&right) { (right, left) } else { (left, right) };
            let tautology = tautology_text(&actual, &expected);
            Some(Check {
                family: "eq",
                level: Some(if tautology { Level::Tautology } else { Level::Exact }),
                error: mentions_error(&actual) || mentions_error(&expected),
                actual: squash(&actual),
                expected: squash(&expected),
                args: args.iter().map(|a| squash(a)).collect(),
                text: squash(whole),
            })
        }
        "assert_ne" => {
            let left = args.first().cloned().unwrap_or_default();
            let right = args.get(1).cloned().unwrap_or_default();
            Some(Check {
                family: "ne",
                level: Some(if tautology_text(&left, &right) { Level::Tautology } else { Level::Partial }),
                error: mentions_error(&left),
                actual: squash(&left),
                expected: squash(&right),
                args: args.iter().map(|a| squash(a)).collect(),
                text: squash(whole),
            })
        }
        "assert" => {
            let cond = args.first().cloned().unwrap_or_default();
            let mut check = classify_condition(&cond, "cond", whole);
            check.args = args.iter().map(|a| squash(a)).collect();
            Some(check)
        }
        "assert_matches" => {
            let actual = args.first().cloned().unwrap_or_default();
            let pattern = args.get(1).cloned().unwrap_or_default();
            Some(Check {
                family: "matches",
                level: Some(Level::Partial),
                error: pattern.trim_start().starts_with("Err") || mentions_error(&actual),
                actual: squash(&actual),
                expected: squash(&pattern),
                args: args.iter().map(|a| squash(a)).collect(),
                text: squash(whole),
            })
        }
        _ if name.starts_with("assert_") && name.ends_with("snapshot") => Some(Check {
            family: "snapshot",
            level: Some(Level::Exact),
            error: false,
            actual: squash(args.iter().find(|a| !a.trim_start().starts_with('"')).map(String::as_str).unwrap_or_default()),
            expected: squash(args.last().map(String::as_str).unwrap_or_default()),
            args: args.iter().map(|a| squash(a)).collect(),
            text: squash(whole),
        }),
        _ if helper_name(&name) => Some(helper_check(&name, args, whole, ctx)),
        _ => None,
    }
}

fn helper_name(name: &str) -> bool {
    HELPER_PREFIXES.iter().any(|p| {
        name.strip_prefix(p).is_some_and(|rest| rest.is_empty() || rest.starts_with('_') || rest.starts_with(|c: char| c.is_ascii_uppercase()))
    })
}

fn helper_check(name: &str, args: Vec<String>, whole: &str, ctx: &Ctx) -> Check {
    let resolved = ctx.helpers.get(name);
    let level = resolved.and_then(|checks| checks.iter().filter_map(|c| c.level).max());
    Check {
        family: "helper",
        level,
        error: resolved.is_some_and(|checks| checks.iter().any(|c| c.error)) || name.to_lowercase().contains("err") || name.to_lowercase().contains("throw"),
        actual: name.to_string(),
        expected: squash(&args.join(",")),
        args: args.iter().map(|a| squash(a)).collect(),
        text: squash(whole),
    }
}

fn ts_callee_name(function: Node, src: &[u8]) -> String {
    match function.kind() {
        "identifier" => text(function, src).to_string(),
        "member_expression" => function.child_by_field_name("property").map(|p| text(p, src).to_string()).unwrap_or_default(),
        _ => String::new(),
    }
}

fn ts_expect_chain<'t>(call: Node<'t>, src: &[u8]) -> Option<(Node<'t>, Vec<String>, String)> {
    let function = call.child_by_field_name("function")?;
    if function.kind() != "member_expression" {
        return None;
    }
    let matcher = text(function.child_by_field_name("property")?, src).to_string();
    let mut modifiers = Vec::new();
    let mut object = function.child_by_field_name("object")?;
    loop {
        match object.kind() {
            "member_expression" => {
                modifiers.push(text(object.child_by_field_name("property")?, src).to_string());
                object = object.child_by_field_name("object")?;
            }
            "call_expression" => {
                let inner = object.child_by_field_name("function")?;
                let name = text(inner, src);
                if name == "expect" || name == "expect.soft" || name == "expect.poll" {
                    return Some((object, modifiers, matcher));
                }
                return None;
            }
            "await_expression" | "parenthesized_expression" => {
                object = named_children(object).into_iter().next()?;
            }
            _ => return None,
        }
    }
}

fn call_args(call: Node, src: &[u8]) -> Vec<String> {
    call.child_by_field_name("arguments")
        .map(|args| named_children(args).into_iter().filter(|a| !a.kind().contains("comment")).map(|a| text(a, src).to_string()).collect())
        .unwrap_or_default()
}

fn ts_check(call: Node, ctx: &Ctx) -> Option<Check> {
    let src = ctx.src;
    let whole = text(call, src);
    if let Some((expect_call, modifiers, matcher)) = ts_expect_chain(call, src) {
        let actual = call_args(expect_call, src).into_iter().next().unwrap_or_default();
        let args = call_args(call, src);
        let expected = args.first().cloned().unwrap_or_default();
        let negated = modifiers.iter().any(|m| m == "not");
        let rejects = modifiers.iter().any(|m| m == "rejects");
        let error = rejects || TS_ERROR.contains(&matcher.as_str()) && !negated;
        let mut level = if error {
            Level::Exact
        } else if TS_EXACT.contains(&matcher.as_str()) {
            if negated {
                if matches!(matcher.as_str(), "toBeNull" | "toBeUndefined") { Level::Exists } else { Level::Partial }
            } else {
                Level::Exact
            }
        } else if TS_EXISTS.contains(&matcher.as_str()) || negated && TS_ERROR.contains(&matcher.as_str()) {
            Level::Exists
        } else {
            Level::Partial
        };
        if matcher == "toHaveProperty" && args.len() < 2 || matcher == "toHaveLength" && negated {
            level = Level::Exists;
        }
        if matcher == "toBeGreaterThanOrEqual" && squash(&expected) == "0" && (actual.ends_with(".length") || actual.ends_with(".size")) {
            level = Level::Tautology;
        }
        if !error && tautology_text(&actual, if TS_EXISTS.contains(&matcher.as_str()) { "" } else { &expected }) {
            level = Level::Tautology;
        }
        let family = if error { "error" } else if matcher.contains("Snapshot") { "snapshot" } else if level == Level::Exact { "eq" } else { "rel" };
        return Some(Check {
            family,
            level: Some(level),
            error,
            actual: squash(&actual),
            expected: squash(&format!("{}{}", if negated { "not." } else { "" }, if expected.is_empty() { matcher.clone() } else { expected })),
            args: args.iter().map(|a| squash(a)).collect(),
            text: squash(whole),
        });
    }
    let function = call.child_by_field_name("function")?;
    let ftext = text(function, src);
    let args = call_args(call, src);
    if ftext == "assert" || ftext == "assert.ok" || ftext == "assert.strict" {
        let cond = args.first().cloned().unwrap_or_default();
        let mut check = classify_condition(&cond, "cond", whole);
        check.args = args.iter().map(|a| squash(a)).collect();
        return Some(check);
    }
    if let Some(method) = ftext.strip_prefix("assert.").or_else(|| ftext.strip_prefix("t.")).or_else(|| ftext.strip_prefix("strict.")) {
        let left = args.first().cloned().unwrap_or_default();
        let right = args.get(1).cloned().unwrap_or_default();
        let error = matches!(method, "throws" | "rejects" | "throwsAsync");
        let level = if error {
            Level::Exact
        } else if matches!(method, "equal" | "strictEqual" | "deepEqual" | "deepStrictEqual" | "is" | "same") {
            if tautology_text(&left, &right) { Level::Tautology } else { Level::Exact }
        } else if matches!(method, "notEqual" | "notStrictEqual" | "notDeepEqual" | "notDeepStrictEqual" | "match" | "not" | "notSame") {
            Level::Partial
        } else if matches!(method, "doesNotThrow" | "doesNotReject" | "truthy" | "ok" | "pass") {
            Level::Exists
        } else if matches!(method, "fail" | "plan" | "end") {
            return None;
        } else {
            Level::Partial
        };
        return Some(Check {
            family: if error { "error" } else if level == Level::Exact { "eq" } else { "rel" },
            level: Some(level),
            error,
            actual: squash(&left),
            expected: squash(&right),
            args: args.iter().map(|a| squash(a)).collect(),
            text: squash(whole),
        });
    }
    if function.kind() == "identifier" && ftext != "expect" && (helper_name(ftext) || ctx.helpers.contains_key(ftext)) {
        return Some(helper_check(ftext, args, whole, ctx));
    }
    None
}

fn is_test_attribute(path: &str) -> bool {
    let last = path.rsplit("::").next().unwrap_or_default();
    last == "test" || last == "rstest" || last == "test_case" || last == "quickcheck"
}

fn cfg_value(expr: &str) -> Option<bool> {
    let e = expr.trim();
    if e == "true" || e == "test" {
        return Some(true);
    }
    if e == "false" {
        return Some(false);
    }
    let (head, rest) = e.split_once('(')?;
    let inner = rest.strip_suffix(')')?;
    let parts: Vec<String> = split_top(inner);
    match head.trim() {
        "any" => {
            let values: Vec<Option<bool>> = parts.iter().map(|p| cfg_value(p)).collect();
            if values.iter().any(|v| *v == Some(true)) {
                Some(true)
            } else if values.iter().all(|v| *v == Some(false)) {
                Some(false)
            } else {
                None
            }
        }
        "all" => {
            let values: Vec<Option<bool>> = parts.iter().map(|p| cfg_value(p)).collect();
            if values.iter().any(|v| *v == Some(false)) {
                Some(false)
            } else if values.iter().all(|v| *v == Some(true)) {
                Some(true)
            } else {
                None
            }
        }
        "not" => cfg_value(parts.first()?).map(|v| !v),
        _ => None,
    }
}

fn rust_attributes<'t>(node: Node<'t>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut prev = node.prev_named_sibling();
    while let Some(p) = prev {
        match p.kind() {
            "attribute_item" => out.push(squash(text(p, src))),
            "line_comment" | "block_comment" => {}
            _ => break,
        }
        prev = p.prev_named_sibling();
    }
    out
}

fn attribute_path(attr: &str) -> String {
    let inner = attr.trim_start_matches("#[").trim_end_matches(']');
    inner.split(|c: char| c == '(' || c == '=').next().unwrap_or_default().to_string()
}

fn collect_checks(node: Node, ctx: &Ctx, test: &mut Test) {
    match (ctx.lang, node.kind()) {
        (Lang::Rust, "macro_invocation") => {
            if let Some(check) = rust_macro_check(node, ctx) {
                test.checks.push(check);
                return;
            }
            let name = rust_macro_name(node, ctx.src);
            if name == "panic" || name == "unreachable" {
                test.implicit += 1;
            }
            let body = text(node, ctx.src);
            if body.contains(".unwrap()") || body.contains(".expect(") || body.contains('?') {
                test.smoke = true;
            }
            return;
        }
        (Lang::Rust, "call_expression") => {
            if let Some(function) = node.child_by_field_name("function") {
                if function.kind() == "field_expression" {
                    let field = function.child_by_field_name("field").map(|f| text(f, ctx.src)).unwrap_or_default();
                    match field {
                        "unwrap_err" | "expect_err" => test.checks.push(Check {
                            family: "error",
                            level: Some(Level::Exact),
                            error: true,
                            actual: squash(text(function.child_by_field_name("value").unwrap_or(function), ctx.src)),
                            expected: field.to_string(),
                            args: Vec::new(),
                            text: squash(text(node, ctx.src)),
                        }),
                        "unwrap" | "expect" => test.smoke = true,
                        _ => {}
                    }
                } else {
                    let name = text(function, ctx.src).rsplit("::").next().unwrap_or_default().to_string();
                    if helper_name(&name) || ctx.helpers.contains_key(&name) {
                        test.checks.push(helper_check(&name, call_args(node, ctx.src), text(node, ctx.src), ctx));
                    }
                }
            }
        }
        (Lang::Rust, "try_expression") => test.smoke = true,
        (Lang::Ts, "call_expression") => {
            if let Some(check) = ts_check(node, ctx) {
                let helper = check.family == "helper";
                test.checks.push(check);
                if !helper {
                    return;
                }
            }
            if let Some(function) = node.child_by_field_name("function") {
                let name = ts_callee_name(function, ctx.src);
                if ["getBy", "getAllBy", "findBy", "findAllBy"].iter().any(|p| name.starts_with(p)) {
                    test.implicit += 1;
                }
                if is_ts_test_call(node, ctx.src).is_some() {
                    return;
                }
            }
        }
        (Lang::Ts, "throw_statement") => test.implicit += 1,
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_checks(child, ctx, test);
    }
}

fn block_statements(block: Node) -> Vec<Node> {
    named_children(block).into_iter().filter(|c| !c.kind().contains("comment")).collect()
}

fn early_return(statements: &[Node], ctx: &Ctx) -> bool {
    let Some(first) = statements.first() else { return false };
    let t = squash(text(*first, ctx.src));
    t.starts_with("return;") || t == "return" || t.starts_with("returnOk(())") || t.starts_with("if(true)return") || t.starts_with("if(true){return")
}

fn rust_test(node: Node, ctx: &Ctx, prefix: &str, attrs: &[String], module_disabled: bool) -> Option<Test> {
    if !attrs.iter().any(|a| is_test_attribute(&attribute_path(a))) {
        return None;
    }
    let name = text(node.child_by_field_name("name")?, ctx.src);
    let body = node.child_by_field_name("body")?;
    let mut test = Test {
        id: format!("{prefix}{name}"),
        line: line_of(node),
        title: name.to_string(),
        body: text(body, ctx.src).to_string(),
        ..Default::default()
    };
    let statements = block_statements(body);
    test.statements = statements.len();
    if node.child_by_field_name("return_type").is_some() {
        test.smoke = true;
    }
    collect_checks(body, ctx, &mut test);
    for attr in attrs {
        let path = attribute_path(attr);
        if path == "should_panic" {
            test.checks.push(Check {
                family: "error",
                level: Some(Level::Exact),
                error: true,
                actual: "should_panic".into(),
                expected: attr.clone(),
                args: Vec::new(),
                text: attr.clone(),
            });
        }
        if path == "ignore" {
            test.disabled.push(("ignore", true));
        }
        if path == "cfg" {
            let inner = attr.trim_start_matches("#[cfg(").trim_end_matches(")]");
            if cfg_value(inner) == Some(false) {
                test.disabled.push(("cfg-never", false));
            }
        }
    }
    if module_disabled {
        test.disabled.push(("cfg-never", false));
    }
    if early_return(&statements, ctx) {
        test.disabled.push(("early-return", false));
    }
    Some(test)
}

fn rust_walk(node: Node, ctx: &Ctx, prefix: &str, disabled: bool, out: &mut FileFacts) {
    for child in named_children(node) {
        match child.kind() {
            "function_item" => {
                let attrs = rust_attributes(child, ctx.src);
                if let Some(test) = rust_test(child, ctx, prefix, &attrs, disabled) {
                    out.tests.push(test);
                }
            }
            "mod_item" => {
                let attrs = rust_attributes(child, ctx.src);
                let never = attrs.iter().any(|a| attribute_path(a) == "cfg" && cfg_value(a.trim_start_matches("#[cfg(").trim_end_matches(")]")) == Some(false));
                let name = child.child_by_field_name("name").map(|n| text(n, ctx.src)).unwrap_or_default();
                if let Some(body) = child.child_by_field_name("body") {
                    rust_walk(body, ctx, &format!("{prefix}{name}::"), disabled || never, out);
                }
            }
            _ => {}
        }
    }
}

fn rust_helpers<'t>(node: Node<'t>, src: &[u8], out: &mut Vec<(String, Node<'t>)>) {
    for child in named_children(node) {
        match child.kind() {
            "function_item" => {
                let attrs = rust_attributes(child, src);
                if !attrs.iter().any(|a| is_test_attribute(&attribute_path(a))) {
                    if let (Some(name), Some(body)) = (child.child_by_field_name("name"), child.child_by_field_name("body")) {
                        out.push((text(name, src).to_string(), body));
                    }
                }
            }
            "mod_item" => {
                if let Some(body) = child.child_by_field_name("body") {
                    rust_helpers(body, src, out);
                }
            }
            _ => {}
        }
    }
}

fn is_ts_test_call<'t>(call: Node<'t>, src: &[u8]) -> Option<(&'static str, Vec<(String, String)>)> {
    let mut function = call.child_by_field_name("function")?;
    let mut modifiers = Vec::new();
    loop {
        match function.kind() {
            "identifier" => {
                return match text(function, src) {
                    "it" | "test" => Some(("test", modifiers)),
                    "describe" | "suite" => Some(("describe", modifiers)),
                    "xit" | "xtest" => {
                        modifiers.push(("x".into(), String::new()));
                        Some(("test", modifiers))
                    }
                    "fit" => {
                        modifiers.push(("f".into(), String::new()));
                        Some(("test", modifiers))
                    }
                    "xdescribe" | "fdescribe" => {
                        modifiers.push(("x".into(), String::new()));
                        Some(("describe", modifiers))
                    }
                    _ => None,
                };
            }
            "member_expression" => {
                let property = text(function.child_by_field_name("property")?, src).to_string();
                modifiers.push((property, String::new()));
                function = function.child_by_field_name("object")?;
            }
            "call_expression" => {
                let inner = function.child_by_field_name("function")?;
                let args = call_args(function, src).join(",");
                if inner.kind() == "member_expression" {
                    let property = text(inner.child_by_field_name("property")?, src).to_string();
                    modifiers.push((property, args));
                    function = inner.child_by_field_name("object")?;
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string()
}

fn ts_disabled(modifiers: &[(String, String)], ctx: &Ctx) -> Vec<(&'static str, bool)> {
    let _ = ctx;
    let mut out = Vec::new();
    for (name, args) in modifiers {
        match name.as_str() {
            "skip" | "only" | "x" | "f" => {
                let direct = modifiers.iter().filter(|(n, _)| !matches!(n.as_str(), "each" | "concurrent" | "sequential")).count() == 1;
                out.push((if name == "only" || name == "f" { "focus" } else { "skip" }, direct || name == "x" || name == "f"));
            }
            "todo" => out.push(("todo", false)),
            "fixme" => out.push(("fixme", false)),
            "skipIf" => {
                if is_literal(args) && !matches!(squash(args).as_str(), "false" | "0" | "\"\"" | "null" | "undefined") {
                    out.push(("skip-if-constant", false));
                }
            }
            "runIf" => {
                if matches!(squash(args).as_str(), "false" | "0" | "\"\"" | "null" | "undefined") {
                    out.push(("run-if-constant", false));
                }
            }
            _ => {}
        }
    }
    out
}

fn ts_walk(node: Node, ctx: &Ctx, path: &str, inherited: &[(&'static str, bool)], out: &mut FileFacts) {
    if node.kind() == "call_expression" {
        if let Some((kind, modifiers)) = is_ts_test_call(node, ctx.src) {
            let args: Vec<Node> = node
                .child_by_field_name("arguments")
                .map(|a| named_children(a).into_iter().filter(|c| !c.kind().contains("comment")).collect())
                .unwrap_or_default();
            let title = args.first().map(|a| unquote(text(*a, ctx.src))).unwrap_or_default();
            let callback = args.iter().rev().find(|a| matches!(a.kind(), "arrow_function" | "function_expression" | "function")).copied();
            let mut disabled = inherited.to_vec();
            disabled.extend(ts_disabled(&modifiers, ctx));
            if kind == "describe" {
                if let Some(cb) = callback {
                    ts_walk(cb, ctx, &format!("{path}{title} > "), &disabled, out);
                }
                return;
            }
            let mut test = Test {
                id: format!("{path}{title}"),
                line: line_of(node),
                title: title.clone(),
                disabled,
                ..Default::default()
            };
            if let Some(cb) = callback {
                if let Some(body) = cb.child_by_field_name("body") {
                    test.body = text(body, ctx.src).to_string();
                    if body.kind() == "statement_block" {
                        let statements = block_statements(body);
                        test.statements = statements.len();
                        if early_return(&statements, ctx) {
                            test.disabled.push(("early-return", false));
                        }
                    } else {
                        test.statements = 1;
                    }
                    collect_checks(body, ctx, &mut test);
                }
            }
            if modifiers.iter().any(|(n, _)| n == "each") {
                test.id = format!("{path}{title} [each]");
            }
            out.tests.push(test);
            return;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        ts_walk(child, ctx, path, inherited, out);
    }
}

fn ts_helpers<'t>(node: Node<'t>, src: &[u8], out: &mut Vec<(String, Node<'t>)>) {
    match node.kind() {
        "function_declaration" => {
            if let (Some(name), Some(body)) = (node.child_by_field_name("name"), node.child_by_field_name("body")) {
                out.push((text(name, src).to_string(), body));
            }
        }
        "variable_declarator" => {
            if let (Some(name), Some(value)) = (node.child_by_field_name("name"), node.child_by_field_name("value")) {
                if matches!(value.kind(), "arrow_function" | "function_expression" | "function") {
                    if let Some(body) = value.child_by_field_name("body") {
                        out.push((text(name, src).to_string(), body));
                    }
                }
            }
        }
        "call_expression" if is_ts_test_call(node, src).is_some_and(|(k, _)| k == "test") => return,
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        ts_helpers(child, src, out);
    }
}

fn ts_mocks(node: Node, src: &[u8], out: &mut Vec<(String, String, usize)>) {
    if node.kind() == "call_expression" {
        if let Some(function) = node.child_by_field_name("function") {
            let f = text(function, src);
            let args = call_args(node, src);
            match f {
                "vi.mock" | "jest.mock" | "vi.doMock" | "jest.doMock" | "jest.unstable_mockModule" => {
                    if let Some(p) = args.first() {
                        out.push(("module".into(), unquote(p), line_of(node)));
                    }
                }
                "vi.spyOn" | "jest.spyOn" => {
                    if let Some(p) = args.get(1) {
                        let chained = node.parent().is_some_and(|p| p.kind() == "member_expression");
                        if chained {
                            out.push(("spy".into(), unquote(p), line_of(node)));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        ts_mocks(child, src, out);
    }
}

fn file_facts(path: &str, src: &[u8], timing: &mut Timing) -> Option<FileFacts> {
    let (lang, grammar) = lang_of(path)?;
    if lang == Lang::Ts && !path_is_test(path) {
        return None;
    }
    let started = Instant::now();
    let mut parser = Parser::new();
    parser.set_language(&grammar).expect("grammar");
    let tree = parser.parse(src, None)?;
    timing.parse += started.elapsed();
    let started = Instant::now();
    let root = tree.root_node();
    let mut ctx = Ctx { lang, src, helpers: HashMap::new() };
    let mut bodies = Vec::new();
    match lang {
        Lang::Rust => rust_helpers(root, src, &mut bodies),
        Lang::Ts => ts_helpers(root, src, &mut bodies),
    }
    for _ in 0..2 {
        let mut resolved = HashMap::new();
        for (name, body) in &bodies {
            let mut scratch = Test::default();
            collect_checks(*body, &ctx, &mut scratch);
            if !scratch.checks.is_empty() {
                resolved.insert(name.clone(), scratch.checks);
            }
        }
        ctx.helpers = resolved;
    }
    let mut facts = FileFacts::default();
    match lang {
        Lang::Rust => rust_walk(root, &ctx, "", false, &mut facts),
        Lang::Ts => {
            ts_walk(root, &ctx, "", &[], &mut facts);
            ts_mocks(root, src, &mut facts.mocks);
        }
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    for test in &mut facts.tests {
        let n = seen.entry(test.id.clone()).or_insert(0);
        *n += 1;
        if *n > 1 {
            test.id = format!("{} #{}", test.id, n);
        }
        for check in &test.checks {
            for id in identifiers(&check.actual) {
                facts.asserted_names.insert(id);
            }
        }
    }
    timing.walk += started.elapsed();
    if facts.tests.is_empty() && facts.mocks.is_empty() {
        return None;
    }
    Some(facts)
}

struct Timing {
    parse: Duration,
    walk: Duration,
}

fn walk_tree(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            if !SKIP_DIRS.contains(&name) {
                walk_tree(root, &path, out);
            }
        } else if lang_of(name).is_some() {
            out.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
        }
    }
}

fn level_name(level: Option<Level>) -> &'static str {
    level.map(Level::name).unwrap_or("unresolved")
}

fn observable(test: &Test) -> bool {
    test.implicit > 0 || test.checks.iter().any(|c| c.level != Some(Level::Tautology))
}

// ponytail: drops everything from the first #[cfg(test)] line on, which holds where the test module ends the file
fn without_test_module(src: &str) -> String {
    match src.find("#[cfg(test)]") {
        Some(at) => src[..at].to_string(),
        None => src.to_string(),
    }
}

fn production_swaps(before: &Path, after: &Path, files: &[String]) -> Vec<(String, String)> {
    let mut removed: BTreeSet<String> = BTreeSet::new();
    let mut added: BTreeSet<String> = BTreeSet::new();
    for file in files {
        if path_is_test(file) || lang_of(file).is_none() {
            continue;
        }
        let old = without_test_module(&std::fs::read_to_string(before.join(file)).unwrap_or_default());
        let new = without_test_module(&std::fs::read_to_string(after.join(file)).unwrap_or_default());
        let old_lines: HashMap<&str, usize> = old.lines().fold(HashMap::new(), |mut m, l| {
            *m.entry(l).or_insert(0) += 1;
            m
        });
        let new_lines: HashMap<&str, usize> = new.lines().fold(HashMap::new(), |mut m, l| {
            *m.entry(l).or_insert(0) += 1;
            m
        });
        for (line, n) in &old_lines {
            if new_lines.get(line).copied().unwrap_or(0) < *n {
                removed.extend(literal_tokens(line));
            }
        }
        for (line, n) in &new_lines {
            if old_lines.get(line).copied().unwrap_or(0) < *n {
                added.extend(literal_tokens(line));
            }
        }
    }
    let mut out = Vec::new();
    for r in &removed {
        for a in &added {
            if r != a {
                out.push((r.clone(), a.clone()));
            }
        }
    }
    out
}

fn compare(file: &str, before: Option<&FileFacts>, after: &FileFacts, swaps: &[(String, String)], moved: &HashSet<String>, out: &mut Vec<Site>) {
    let old: HashMap<&str, &Test> = before.map(|b| b.tests.iter().map(|t| (t.id.as_str(), t)).collect()).unwrap_or_default();
    for test in &after.tests {
        let site = |candidate: &'static str, shape: String, tags: Vec<&'static str>, text: String| Site {
            candidate,
            shape,
            file: file.to_string(),
            line: test.line,
            tags,
            test: test.id.clone(),
            text,
        };
        let Some(prior) = old.get(test.id.as_str()) else {
            if !observable(test) {
                let mut tags = Vec::new();
                if test.statements == 0 {
                    tags.push("stubs");
                }
                if test.checks.is_empty() && (test.smoke || SMOKE_WORDS.iter().any(|w| test.title.to_lowercase().contains(w))) {
                    tags.push("smoke");
                }
                if test.checks.iter().any(|c| c.level.is_none()) {
                    tags.push("unresolved");
                }
                if !test.disabled.is_empty() {
                    tags.push("disabled");
                }
                let shape = if test.checks.is_empty() { "none" } else { "tautology-only" };
                out.push(site("new-test-unchecked", shape.into(), tags, test.title.clone()));
            }
            for (shape, escaped) in &test.disabled {
                out.push(site("disabled", (*shape).into(), if *escaped { vec!["escapes"] } else { vec![] }, test.title.clone()));
            }
            continue;
        };
        for (shape, escaped) in &test.disabled {
            if !prior.disabled.iter().any(|(s, _)| s == shape) {
                out.push(site("disabled", (*shape).into(), if *escaped { vec!["escapes"] } else { vec![] }, test.title.clone()));
            }
        }
        let before_checks: Vec<&Check> = prior.checks.iter().filter(|c| c.level != Some(Level::Tautology)).collect();
        if !before_checks.is_empty() && !observable(test) {
            let mut tags = Vec::new();
            if test.statements == 0 {
                tags.push("stubs");
            }
            let shape = if test.checks.is_empty() { "none-left" } else { "tautology-only" };
            out.push(site("all-checks-removed", shape.into(), tags, test.checks.iter().map(|c| c.text.clone()).collect::<Vec<_>>().join(" ")));
            continue;
        }
        let mut remaining: Vec<&Check> = test.checks.iter().collect();
        let mut gone: Vec<&Check> = Vec::new();
        for check in &prior.checks {
            if let Some(pos) = remaining.iter().position(|c| c.key() == check.key()) {
                remaining.remove(pos);
            } else {
                gone.push(check);
            }
        }
        let added = remaining;
        let error_after = test.checks.iter().any(|c| c.error);
        for g in gone {
            if g.level == Some(Level::Tautology) {
                continue;
            }
            let g_level = g.level.unwrap_or(Level::Exact);
            let same_actual: Vec<&&Check> = added.iter().filter(|a| !g.actual.is_empty() && a.actual == g.actual && a.error == g.error).collect();
            if let Some(weaker) = same_actual.iter().filter(|a| a.level.is_some_and(|l| l < g_level)).min_by_key(|a| a.level) {
                if !same_actual.iter().any(|a| a.level.is_some_and(|l| l >= g_level)) {
                    out.push(site("weakened", format!("{}->{}", g_level.name(), level_name(weaker.level)), vec![], format!("{} => {}", g.text, weaker.text)));
                    continue;
                }
            }
            if let Some(changed) = same_actual.iter().find(|a| a.level.is_some_and(|l| l >= g_level) && a.expected != g.expected) {
                let old_tokens = literal_tokens(&g.expected);
                let new_tokens = literal_tokens(&changed.expected);
                let mirrored = old_tokens.iter().any(|o| !new_tokens.contains(o) && new_tokens.iter().any(|n| !old_tokens.contains(n) && swaps.contains(&(o.clone(), n.clone()))));
                let shape = if mirrored { "mirrors-production" } else { "changed" };
                out.push(site("expected-changed", shape.into(), if mirrored { vec![] } else { vec!["no-mirror"] }, format!("{} => {}", g.text, changed.text)));
                continue;
            }
            if same_actual.iter().any(|a| a.level.is_some_and(|l| l >= g_level)) {
                continue;
            }
            let expected_kept = !g.expected.is_empty()
                && !is_bool(&g.expected)
                && added.iter().any(|a| a.error == g.error && a.level.map_or(true, |l| l >= g_level) && (a.expected == g.expected || a.args.iter().any(|x| x == &g.expected)));
            if expected_kept {
                if added.iter().any(|a| a.level.is_none() && a.args.iter().any(|x| x == &g.expected)) {
                    out.push(site("assertion-removed", "into-unresolved-helper".into(), vec!["unresolved"], g.text.clone()));
                }
                continue;
            }
            let tokens = literal_tokens(&g.expected);
            let squashed_body = squash(&test.body);
            let table = !tokens.is_empty()
                && tokens.iter().all(|t| squashed_body.contains(&squash(t)))
                && added.iter().copied().chain(test.checks.iter()).any(|a| a.error == g.error && a.level.map_or(false, |l| l >= g_level) && is_identifier(&a.expected));
            if table {
                continue;
            }
            if g.error && added.iter().any(|a| a.error) || moved.contains(&g.key()) {
                continue;
            }
            let merged = !tokens.is_empty()
                && added.iter().any(|a| a.error == g.error && a.level.is_some_and(|l| l >= g_level) && {
                    let into = literal_tokens(&a.expected);
                    tokens.iter().all(|t| into.contains(t))
                });
            if merged {
                continue;
            }
            if g.error && !error_after {
                out.push(site("error-expectation-removed", if added.is_empty() { "removed" } else { "replaced" }.into(), vec![], g.text.clone()));
                continue;
            }
            let mut tags = Vec::new();
            if added.iter().any(|a| a.level.is_none()) {
                tags.push("unresolved");
            }
            let shape = if added.iter().any(|a| a.level == Some(Level::Tautology)) {
                "replaced-by-tautology"
            } else if added.iter().any(|a| a.level.is_some_and(|l| l < g_level)) {
                "replaced-by-weaker"
            } else if added.is_empty() {
                "removed"
            } else {
                "replaced"
            };
            out.push(site("assertion-removed", shape.into(), tags, g.text.clone()));
        }
    }
}

fn is_bool(text: &str) -> bool {
    matches!(text, "true" | "false" | "")
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '[' || c == ']') && !text.chars().next().unwrap().is_ascii_digit() && !is_literal(text)
}

fn test_counts(before: &HashMap<String, FileFacts>, after: &HashMap<String, FileFacts>) -> (usize, usize, usize) {
    let keys = |test: &Test| {
        let mut keys: Vec<String> = test.checks.iter().map(Check::key).collect();
        keys.sort();
        keys
    };
    let held = before.values().map(|f| f.tests.len()).sum();
    let made = after.values().map(|f| f.tests.len()).sum();
    let mut touched = 0;
    for (file, facts) in after {
        let old: HashMap<&str, &Test> = before.get(file).map(|b| b.tests.iter().map(|t| (t.id.as_str(), t)).collect()).unwrap_or_default();
        touched += facts.tests.iter().filter(|t| old.get(t.id.as_str()).is_none_or(|o| keys(o) != keys(t))).count();
    }
    for (file, facts) in before {
        let new: HashSet<&str> = after.get(file).map(|a| a.tests.iter().map(|t| t.id.as_str()).collect()).unwrap_or_default();
        touched += facts.tests.iter().filter(|t| !new.contains(t.id.as_str())).count();
    }
    (held, made, touched)
}

fn added_keys(before: &HashMap<String, FileFacts>, after: &HashMap<String, FileFacts>) -> HashSet<String> {
    let mut out = HashSet::new();
    for (file, facts) in after {
        let mut old: HashMap<String, usize> = HashMap::new();
        for test in before.get(file).map(|b| b.tests.as_slice()).unwrap_or_default() {
            for check in &test.checks {
                *old.entry(check.key()).or_insert(0) += 1;
            }
        }
        for test in &facts.tests {
            for check in &test.checks {
                let key = check.key();
                match old.get_mut(&key) {
                    Some(n) if *n > 0 => *n -= 1,
                    _ => {
                        out.insert(key);
                    }
                }
            }
        }
    }
    out
}

fn mock_sites(file: &str, before: Option<&FileFacts>, after: &FileFacts, out: &mut Vec<Site>) {
    let old: Vec<(String, String)> = before.map(|b| b.mocks.iter().map(|(k, t, _)| (k.clone(), t.clone())).collect()).unwrap_or_default();
    let stem = file.rsplit('/').next().unwrap_or_default().split('.').next().unwrap_or_default().to_string();
    let mut remaining = old.clone();
    for (kind, target, line) in &after.mocks {
        if let Some(pos) = remaining.iter().position(|(k, t)| k == kind && t == target) {
            remaining.remove(pos);
            continue;
        }
        let module_stem = target.rsplit('/').next().unwrap_or_default().split('.').next().unwrap_or_default();
        let subject = if kind == "module" { module_stem == stem && target.starts_with('.') } else { after.asserted_names.contains(target) };
        out.push(Site {
            candidate: "mocked-subject",
            shape: if kind == "module" { "module".into() } else { "spy".into() },
            file: file.to_string(),
            line: *line,
            tags: if subject { vec![] } else { vec!["collaborator"] },
            test: "-".into(),
            text: target.clone(),
        });
    }
}

fn analyze(root: &Path, files: &[String], timing: &mut Timing) -> HashMap<String, FileFacts> {
    let mut out = HashMap::new();
    for file in files {
        let Ok(src) = std::fs::read(root.join(file)) else { continue };
        if let Some(facts) = file_facts(file, &src, timing) {
            out.insert(file.clone(), facts);
        }
    }
    out
}

fn print(sites: &[Site]) {
    for site in sites {
        let tags = if site.tags.is_empty() { "-".to_string() } else { site.tags.join(",") };
        println!("{}\t{}\t{}\t{}\t{}\t{}\t{}", site.candidate, site.shape, site.file, site.line, tags, site.test, site.text);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut timing = Timing { parse: Duration::ZERO, walk: Duration::ZERO };
    match args.first().map(String::as_str) {
        Some("new") => {
            let before_root = Path::new(&args[1]);
            let after_root = Path::new(&args[2]);
            let files: Vec<String> = args[3..].to_vec();
            let before = analyze(before_root, &files, &mut timing);
            let after = analyze(after_root, &files, &mut timing);
            let swaps = production_swaps(before_root, after_root, &files);
            let moved = added_keys(&before, &after);
            let mut sites = Vec::new();
            let mut names: Vec<&String> = after.keys().collect();
            names.sort();
            for file in names {
                compare(file, before.get(file), &after[file], &swaps, &moved, &mut sites);
                mock_sites(file, before.get(file), &after[file], &mut sites);
            }
            print(&sites);
            let (held, made, touched) = test_counts(&before, &after);
            eprintln!("tests\t{held}\t{made}\t{touched}");
        }
        Some("dump") => {
            let root = Path::new(&args[1]);
            let mut files = Vec::new();
            walk_tree(root, root, &mut files);
            let facts = analyze(root, &files, &mut timing);
            let mut names: Vec<&String> = facts.keys().collect();
            names.sort();
            for file in names {
                for test in &facts[file].tests {
                    let disabled: Vec<&str> = test.disabled.iter().map(|(s, _)| *s).collect();
                    println!("{file}\t{}\t{}\tstatements={} implicit={} smoke={} disabled={:?}", test.line, test.id, test.statements, test.implicit, test.smoke, disabled);
                    for check in &test.checks {
                        println!("\t{}\t{}\terror={}\t{}", check.family, level_name(check.level), check.error, check.text);
                    }
                }
                for (kind, target, line) in &facts[file].mocks {
                    println!("{file}\t{line}\tmock {kind} {target}");
                }
            }
        }
        Some("time") => {
            let root = Path::new(&args[1]);
            let mut files = Vec::new();
            walk_tree(root, root, &mut files);
            let mut walks = Vec::new();
            let mut parse = Duration::ZERO;
            let mut count = 0;
            for file in &files {
                let Ok(src) = std::fs::read(root.join(file)) else { continue };
                let mut one = Timing { parse: Duration::ZERO, walk: Duration::ZERO };
                if file_facts(file, &src, &mut one).is_some() {
                    count += 1;
                    parse += one.parse;
                    walks.push(one.walk);
                }
            }
            walks.sort();
            let walk: Duration = walks.iter().sum();
            let pick = |q: f64| walks.get(((walks.len() as f64 - 1.0) * q) as usize).copied().unwrap_or_default();
            println!(
                "files\t{}\tparse_ms\t{:.1}\twalk_ms\t{:.1}\twalk_p50_us\t{}\twalk_p99_us\t{}\twalk_max_us\t{}",
                count,
                parse.as_secs_f64() * 1e3,
                walk.as_secs_f64() * 1e3,
                pick(0.5).as_micros(),
                pick(0.99).as_micros(),
                walks.last().copied().unwrap_or_default().as_micros()
            );
        }
        _ => {
            eprintln!("usage: asserts new BEFORE AFTER FILE... | asserts dump ROOT | asserts time ROOT");
            std::process::exit(2);
        }
    }
}
