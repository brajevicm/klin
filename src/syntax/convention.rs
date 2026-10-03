//! What a language's own conventions say about a function a grammar read: whether it declares
//! a test, and whether its body is a placeholder. Both are read off the one function walk of
//! `syntax`, and both are shared — `inventory` ratchets the tests, `stubs` reports the shapes —
//! so neither lives inside a check that would then own syntax for the other. Spec 8.2.

use tree_sitter::Node;

use crate::error::Error;
use crate::record;
use crate::syntax::{
    Language, Parsed, ParsedFile, Unparsed, language_of, line_at, read, tolerant, walk,
};

/// The declaration a language's test convention names a test function by, anywhere on the
/// declaration line, so a modifier before it is allowed. Fixed in the binary, the way the
/// escapes table is. Spec 8.2.
const TEST_NAMES: &[&str] = &["fn test_", "def test_", "func test_", "func Test"];

/// The calls a convention declares a test by, at the start of the declaration line, so a call
/// to one of these names inside a body is not a declaration.
const TEST_CALLS: &[&str] = &["it(", "test("];

/// The markers it writes as an annotation, on the declaration line or on the run of marker lines
/// above it. A Rust attribute marks a test by its path, which `test_attribute` reads.
const TEST_ANNOTATIONS: &[&str] = &["@Test"];

/// One test function a tree holds: the site of ADR 0008, and the body hash the cross-file pass
/// of spec 4.4 matches on. What `inventory` ratchets the existence of. Spec 8.2.
#[derive(Clone)]
pub struct Test {
    pub file: String,
    pub line: u64,
    pub text: String,
    pub body: u64,
}

/// Every function in one source text that the language's own test convention marks as a test.
/// The walk is the one every parsing gate does, and only the marker table is new. Nothing for a
/// path no grammar here reads, and nothing for a text the grammar rejects. Spec 8.2.
pub fn tests(path: &str, source: &str, unparsed: &mut Vec<Unparsed>) -> Result<Vec<Test>, Error> {
    let Some(language) = language_of(path) else {
        return Ok(Vec::new());
    };
    match read(path, source, language)? {
        Parsed::Read(file) => Ok(tests_in(&file)),
        Parsed::Rejected(refused) => {
            unparsed.push(refused);
            Ok(Vec::new())
        }
    }
}

/// The same, for a caller that already holds the parse, so no check reads one file twice.
pub fn tests_in(file: &ParsedFile) -> Vec<Test> {
    let lines = file.lines();
    file.functions()
        .into_iter()
        .filter_map(|node| declared(file, node, &lines))
        .collect()
}

fn declared(file: &ParsedFile, node: Node, lines: &[&str]) -> Option<Test> {
    let path = file.path;
    let from = node.start_position().row;
    let end = node.end_position().row.min(lines.len().saturating_sub(1));
    let row = declaration_row(lines, from, end);
    marks_a_test(node, file.bytes(), lines, row).then(|| Test {
        file: path.to_string(),
        line: row as u64 + 1,
        text: line_at(lines, row),
        body: record::body_hash(&lines[row..=end.max(row)].join("\n")),
    })
}

/// The row the declaration of a test sits on. A language that writes the test marker as an
/// annotation may put it inside the function's own node, so the node's first line is not the
/// declaration. Neither the site of ADR 0008 nor the body hash of 4.4 may keep that line: an
/// annotation above the name would hold the name in the body, and a rename would not match.
fn declaration_row(lines: &[&str], from: usize, to: usize) -> usize {
    (from..=to)
        .find(|row| {
            let line = line_at(lines, *row);
            !line.is_empty() && !only_a_marker(&line)
        })
        .unwrap_or(from)
}

/// Whether this line carries nothing but an attribute or an annotation, so the declaration it
/// marks sits on a line below it.
fn only_a_marker(line: &str) -> bool {
    (line.starts_with('#') || line.starts_with('@'))
        && (line.ends_with(']') || line.ends_with(')') || !line.contains(' '))
}

/// Whether the convention marks the function that starts on this row: a marker on the
/// declaration line, an annotation on the run of marker lines directly above it, or a Rust
/// test attribute on the function.
fn marks_a_test(node: Node, source: &[u8], lines: &[&str], row: usize) -> bool {
    let declaration = line_at(lines, row);
    TEST_NAMES
        .iter()
        .any(|marker| mentions(&declaration, marker))
        || TEST_CALLS
            .iter()
            .any(|marker| declaration.starts_with(marker))
        || annotated(&declaration)
        || attributed(lines, row)
        || test_attribute(node, source)
}

fn attributed(lines: &[&str], row: usize) -> bool {
    lines[..row.min(lines.len())]
        .iter()
        .rev()
        .map(|line| line.trim())
        .take_while(|line| only_a_marker(line))
        .any(annotated)
}

fn annotated(line: &str) -> bool {
    TEST_ANNOTATIONS.iter().any(|marker| line.contains(marker))
}

/// Whether a Rust attribute among the attributes and comments directly above this function has
/// a path that ends in the segment `test`, with or without arguments, as `#[test]` and
/// `#[tokio::test(...)]` do. Read from the grammar, so the lines the attribute spans and the
/// whitespace inside it change nothing.
fn test_attribute(node: Node, source: &[u8]) -> bool {
    let mut above = node.prev_named_sibling();
    while let Some(held) = above.filter(|held| PRELUDE.contains(&held.kind())) {
        let path = held
            .named_child(0)
            .filter(|attribute| attribute.kind() == "attribute")
            .and_then(|attribute| attribute.named_child(0));
        if path.is_some_and(|path| ends_in_test(path, source)) {
            return true;
        }
        above = held.prev_named_sibling();
    }
    false
}

fn ends_in_test(path: Node, source: &[u8]) -> bool {
    let name = match path.kind() {
        "scoped_identifier" => path.child_by_field_name("name"),
        _ => Some(path),
    };
    name.is_some_and(|name| name.kind() == "identifier" && name.utf8_text(source) == Ok("test"))
}

/// Whether the text names this marker where no identifier runs into it, so `myfunc Test` is
/// not a `func Test` declaration.
fn mentions(text: &str, marker: &str) -> bool {
    text.match_indices(marker).any(|(at, _)| {
        text[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric() && before != '_' && before != '.')
    })
}

/// The placeholder body shapes of spec 8.2, and the remedy each one carries. A shape is what a
/// line pattern cannot see, so `stubs` reads it from this walk. #114.
pub struct Stub {
    pub line: u64,
    pub text: String,
    pub name: &'static str,
    pub remedy: &'static str,
}

const PASS_BODY: (&str, &str) = ("pass body", "implement the body");
const ELIDED_BODY: (&str, &str) = ("elided body", "implement the body");
const EMPTY_TEST: (&str, &str) = ("empty test", "write the assertion the test name promises");

/// A comment that stands in for the body it replaces, once the comment markers and the
/// whitespace are off it. Fixed in the binary, the way the marker table is.
const ELISIONS: &[&str] = &["...", "rest of the"];

/// The body a shape can be read off: a run of statements, and no expression a function returns.
/// A concise arrow body such as `() => value` is one expression and does the work of one.
const BODY_BLOCKS: &[&str] = &[
    "block",
    "statement_block",
    "body_statement",
    "function_body",
];

/// A decorator that declares a body is meant to be empty, so no shape of it is a stub.
const ABSTRACT_DECORATORS: &[&str] = &["abstractmethod", "abstractproperty", "overload"];

/// A base class whose methods declare a shape and no body.
const ABSTRACT_BASES: &[&str] = &["Protocol", "ABC"];

/// Every placeholder body shape one source text holds, at the declaration line of the function
/// that holds it. Nothing for a path no grammar here reads, and nothing for a text the grammar
/// rejects. Spec 8.2.
pub fn stubs(path: &str, source: &str) -> Vec<Stub> {
    let Some(language) = language_of(path) else {
        return Vec::new();
    };
    let Ok(Parsed::Read(file)) = read(path, source, language) else {
        return Vec::new();
    };
    let lines = file.lines();
    let mut out: Vec<Stub> = file
        .functions()
        .into_iter()
        .filter_map(|node| placeholder(&file, node, &lines))
        .collect();
    out.sort_by_key(|stub| stub.line);
    out
}

fn placeholder(file: &ParsedFile, node: Node, lines: &[&str]) -> Option<Stub> {
    let from = node.start_position().row;
    let to = node
        .end_position()
        .row
        .min(lines.len().saturating_sub(1))
        .max(from);
    let row = declaration_row(lines, from, to);
    let (name, remedy) = shape(node, file.language, file.bytes(), lines, row)?;
    Some(Stub {
        line: row as u64 + 1,
        text: line_at(lines, row),
        name,
        remedy,
    })
}

/// The shape of one function's body, and `None` when the body does work or when the
/// declaration is abstract. A declaration that carries no body at all, such as a trait method
/// without a default or an interface method, has no shape to judge.
fn shape(
    node: Node,
    language: &Language,
    source: &[u8],
    lines: &[&str],
    row: usize,
) -> Option<(&'static str, &'static str)> {
    let body = node.child_by_field_name("body")?;
    if !BODY_BLOCKS.contains(&body.kind())
        || !owns_the_declaration(node, language)
        || declared_abstract(node, source)
    {
        return None;
    }
    let mut cursor = body.walk();
    let children: Vec<Node> = body.named_children(&mut cursor).collect();
    let (comments, statements): (Vec<&Node>, Vec<&Node>) = children
        .iter()
        .partition(|child| child.kind().contains("comment"));
    match statements.as_slice() {
        [only] if only.kind() == "pass_statement" => Some(PASS_BODY),
        [] if comments.iter().any(|child| elides(child, source)) => Some(ELIDED_BODY),
        [] if marks_a_test(node, source, lines, row) => Some(EMPTY_TEST),
        _ => None,
    }
}

/// Whether the declaration line this function is judged at is its own. A callback written
/// inside the call that declares a test shares that line, and the shape of its body is not the
/// shape of the test's.
fn owns_the_declaration(node: Node, language: &Language) -> bool {
    let row = node.start_position().row;
    let mut above = node.parent();
    while let Some(holder) = above {
        if holder.start_position().row == row && language.functions.contains(&holder.kind()) {
            return false;
        }
        above = holder.parent();
    }
    true
}

fn elides(comment: &Node, source: &[u8]) -> bool {
    let text = comment
        .utf8_text(source)
        .unwrap_or_default()
        .trim_start_matches(['/', '#', '*', '!', '-'])
        .trim_end_matches(['/', '*'])
        .trim()
        .to_lowercase();
    ELISIONS.iter().any(|elision| text.starts_with(elision))
}

/// Whether a declaration above this function says its body is meant to be empty: a decorator
/// that names it abstract, or a class that states a shape and no body.
fn declared_abstract(node: Node, source: &[u8]) -> bool {
    let mut above = node.parent();
    while let Some(holder) = above {
        match holder.kind() {
            "decorated_definition" if decorated_with(holder, source, ABSTRACT_DECORATORS) => {
                return true;
            }
            "class_definition" => {
                return holder
                    .child_by_field_name("superclasses")
                    .is_some_and(|bases| based_on(bases, source, ABSTRACT_BASES));
            }
            _ => {}
        }
        above = holder.parent();
    }
    false
}

/// Whether one of this definition's decorators names one of these. The name a decorator calls
/// is read on its own, so an argument that spells `overload` inside a route is not one.
fn decorated_with(node: Node, source: &[u8], wanted: &[&str]) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == "decorator")
        .filter_map(|child| child.named_child(0))
        .any(|called| named(callee(called), source, wanted))
}

/// Whether one of the bases in this argument list names one of these. Whole names only, so
/// `StoreABC` is not `ABC`.
fn based_on(bases: Node, source: &[u8], wanted: &[&str]) -> bool {
    let mut cursor = bases.walk();
    bases
        .named_children(&mut cursor)
        .any(|base| named(callee(base), source, wanted))
}

/// The name a call or a subscript is written on, and the node itself when it is a name already.
fn callee(node: Node) -> Node {
    match node.kind() {
        "call" => node.child_by_field_name("function").unwrap_or(node),
        "subscript" => node.child_by_field_name("value").unwrap_or(node),
        _ => node,
    }
}

/// Whether this name, or the last segment of this dotted name, is one of these.
fn named(node: Node, source: &[u8], wanted: &[&str]) -> bool {
    let text = node.utf8_text(source).unwrap_or_default();
    let tail = text.rsplit('.').next().unwrap_or(text);
    wanted.contains(&tail)
}

/// The declarations Rust may write between an attribute and the item it marks, which the walk
/// steps over to reach that item.
const PRELUDE: &[&str] = &[
    "attribute_item",
    "line_comment",
    "block_comment",
    "doc_comment",
];

/// The line ranges an inline Rust test module covers, which `escapes` skips by the rule of
/// spec 8.2. Read off whatever the grammar could make of the text, so a file it only partly
/// read still has its tests skipped. Nothing for a path no grammar here reads.
pub fn test_module_ranges(path: &str, source: &str) -> Vec<(u64, u64)> {
    let Some(file) = tolerant(path, source) else {
        return Vec::new();
    };
    cfg_test_ranges(file.root(), file.bytes())
}

pub fn cfg_test_ranges(root: Node, source: &[u8]) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    marked_ranges(root, source, &mut out);
    out
}

fn marked_ranges(node: Node, source: &[u8], out: &mut Vec<(u64, u64)>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "attribute_item" && is_cfg_test(child, source) {
            out.push((
                child.start_position().row as u64 + 1,
                item_after(child).end_position().row as u64 + 1,
            ));
            continue;
        }
        marked_ranges(child, source, out);
    }
}

fn item_after(attribute: Node) -> Node {
    let mut node = attribute;
    while let Some(next) = node.next_named_sibling() {
        node = next;
        if !PRELUDE.contains(&node.kind()) {
            break;
        }
    }
    node
}

fn is_cfg_test(node: Node, source: &[u8]) -> bool {
    node.utf8_text(source)
        .is_ok_and(|text| text.split_whitespace().collect::<String>() == "#[cfg(test)]")
}

/// The byte each Rust `cfg_attr` in this text starts at where it skips its test on every
/// target, which a pattern cannot decide, so the Rust grammar reads each one. One parse per
/// file, and none for a file that writes no `cfg_attr`. Spec 8.2.
pub fn skipped_everywhere(path: &str, source: &str) -> Vec<usize> {
    let Some(file) = tolerant(path, source).filter(|_| source.contains("cfg_attr")) else {
        return Vec::new();
    };
    let bytes = file.bytes();
    let mut out = Vec::new();
    walk(file.root(), &mut |node| {
        let attribute = node
            .named_child(0)
            .filter(|_| node.kind() == "attribute_item")
            .filter(|attribute| attribute.kind() == "attribute");
        let arguments = attribute
            .filter(|attribute| {
                attribute
                    .named_child(0)
                    .is_some_and(|path| word_of(path, bytes) == "cfg_attr")
            })
            .and_then(|attribute| attribute.child_by_field_name("arguments"));
        if arguments.is_some_and(|arguments| ignores(arguments, bytes)) {
            out.push(node.start_byte());
        }
    });
    out
}

/// Whether the arguments of a `cfg_attr` apply `ignore` on every target: the predicate always
/// holds, and one attribute it applies is `ignore` or another `cfg_attr` that does the same.
fn ignores(arguments: Node, source: &[u8]) -> bool {
    let groups = groups(arguments);
    let Some((predicate, applied)) = groups.split_first() else {
        return false;
    };
    holds(predicate, source) == Some(true)
        && applied.iter().any(|attribute| match attribute.as_slice() {
            [name, ..] if word_of(*name, source) == "ignore" => true,
            [name, inner] if word_of(*name, source) == "cfg_attr" => ignores(*inner, source),
            _ => false,
        })
}

/// The value of a cfg predicate on every target, and `None` where it depends on a
/// configuration option. An option such as `windows` may hold or not, so `any(windows,
/// not(any()))` always holds and `all(windows, not(any()))` may not.
fn holds(predicate: &[Node], source: &[u8]) -> Option<bool> {
    let (name, arguments) = match predicate {
        [only] => return option(*only, source),
        [name, arguments] if arguments.kind() == "token_tree" => (name, arguments),
        _ => return None,
    };
    let values: Vec<Option<bool>> = groups(*arguments)
        .iter()
        .filter(|inner| !inner.is_empty())
        .map(|inner| holds(inner, source))
        .collect();
    match (word_of(*name, source), values.as_slice()) {
        ("all", _) if values.contains(&Some(false)) => Some(false),
        ("all", _) => values.iter().all(Option::is_some).then_some(true),
        ("any", _) if values.contains(&Some(true)) => Some(true),
        ("any", _) => values.iter().all(Option::is_some).then_some(false),
        ("not", [only]) => only.map(|held| !held),
        _ => None,
    }
}

/// The value of one cfg option on every target: `test` holds wherever a test runs, `true` and
/// `false` are what they say, and any other option may hold or not.
fn option(token: Node, source: &[u8]) -> Option<bool> {
    match token.utf8_text(source).unwrap_or_default() {
        "test" | "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// The tokens between the delimiters of a token tree, split at each comma, with the comments
/// left out.
fn groups(tree: Node) -> Vec<Vec<Node>> {
    let mut cursor = tree.walk();
    let tokens: Vec<Node> = tree
        .children(&mut cursor)
        .filter(|token| !token.is_extra())
        .collect();
    let inside = tokens.get(1..tokens.len().saturating_sub(1)).unwrap_or(&[]);
    inside
        .split(|token| token.kind() == ",")
        .map(<[Node]>::to_vec)
        .collect()
}

fn word_of<'a>(token: Node, source: &'a [u8]) -> &'a str {
    match token.kind() {
        "identifier" => token.utf8_text(source).unwrap_or_default(),
        _ => "",
    }
}
