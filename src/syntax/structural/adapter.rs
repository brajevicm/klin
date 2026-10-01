use tree_sitter::Node;

use super::facts::{ExportLeaf, Visibility};

/// What one language adapter states. A query names the node kinds that declare something, and
/// the capture name says what kind of thing; everything else is the handful of judgments a
/// query cannot make.
pub(crate) struct Adapter {
    pub patterns: &'static str,
    /// The node kinds that are a use of a name.
    pub identifiers: &'static [&'static str],
    /// The node kinds that turn a function into a method when one holds it.
    pub methods_in: &'static [&'static str],
    /// Names invoked by the runtime without a source reference.
    pub entry_points: &'static [&'static str],
    pub visible: fn(Node) -> bool,
    pub imported: fn(Node, &[u8]) -> Imported,
    /// The file a module declaration was remapped to, for a language that writes such a thing.
    pub remapped: fn(Node, &[u8]) -> Option<String>,
    /// The inline modules that hold a node, outermost first.
    pub nesting: fn(Node, &[u8]) -> Vec<String>,
    /// The segments of the whole path a node writes, and `None` for any other node, including a
    /// path inside a longer one.
    pub qualified: fn(Node, &[u8]) -> Option<Vec<String>>,
    /// The first segments a qualified path resolves from inside the crate. A path that starts
    /// with any other name is kept only where it names a module the file declares beside it.
    pub rooted: &'static [&'static str],
    /// The node kinds of a block, whose items are local to it.
    pub blocks: &'static [&'static str],
    /// The names a node writes inside a string that the language calls by that text, such as a
    /// function a Rust `serde` attribute names or a name a format string captures.
    pub quoted: fn(Node, &[u8]) -> Vec<String>,
    /// What a declaration's or a module declaration's own modifier says.
    pub visibility: fn(Node, &[u8]) -> Visibility,
    /// The external name a declaration is exported under where it differs from its own name.
    pub exported_as: fn(Node, &[u8]) -> Option<String>,
    /// The type an inherent implementation adds a method to.
    pub owner: fn(Node, &[u8]) -> Option<String>,
    /// The nodes that write each name a declaration's name binds where it is a destructuring
    /// pattern.
    pub destructured: fn(Node) -> Vec<Node>,
    /// The canonical declared contract of a declaration, and `None` for a form V1 does not
    /// canonicalize.
    pub contract: fn(Node, &[u8]) -> Option<String>,
    /// What an `@export` capture in the file at this path exposes, and `None` where the node
    /// exports nothing a declaration does not already say for itself.
    pub exported: fn(Node, &[u8], &str) -> Option<Exported>,
}

/// What one import states, before the shared reader puts it at a line. The specifier is kept
/// as it was written, and the module graph resolves it.
pub(crate) struct Imported {
    pub module: Option<String>,
    pub names: Vec<String>,
    pub paths: Vec<String>,
}

/// What one export statement states, before the shared reader puts it at a line.
pub(crate) struct Exported {
    pub source: Option<String>,
    pub type_only: bool,
    pub supported: bool,
    pub leaves: Vec<ExportLeaf>,
    pub contract: Option<String>,
}

/// How the canonical spelling treats one node: leave the subtree out, write this text for it
/// and go no deeper, or spell it token by token.
pub(crate) enum Spelling {
    Skip,
    Replace(String),
    Keep,
}

/// The canonical text of one node: every token the rule keeps, one space apart, with the
/// spacing a reader expects around punctuation. Comments never reach it, because a rule skips
/// them, and the rule decides what a body, an attribute or a binding name becomes.
pub(crate) fn spelled(node: Node, source: &[u8], rule: &dyn Fn(Node) -> Spelling) -> String {
    let mut tokens = Vec::new();
    collect_tokens(node, source, rule, &mut tokens);
    tidy(&tokens)
}

fn collect_tokens(
    node: Node,
    source: &[u8],
    rule: &dyn Fn(Node) -> Spelling,
    out: &mut Vec<String>,
) {
    match rule(node) {
        Spelling::Skip => {}
        Spelling::Replace(text) => out.push(text),
        Spelling::Keep if node.child_count() == 0 => out.push(text_of(node, source)),
        Spelling::Keep => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_tokens(child, source, rule, out);
            }
        }
    }
}

/// Tokens joined by one space, less the space a reader would not write: before a closing
/// bracket or a separator, after an opening bracket, around a path separator or a member dot,
/// and between a name and the bracket that opens its arguments. A separator left dangling
/// before a closing bracket, where a skipped token stood after it, is dropped.
fn tidy(tokens: &[String]) -> String {
    let tokens: Vec<&str> = tokens
        .iter()
        .map(String::as_str)
        .filter(|token| !token.is_empty())
        .collect();
    let mut out = String::new();
    let mut last: Option<&str> = None;
    for (at, token) in tokens.iter().enumerate() {
        if dangling(&tokens, at) {
            continue;
        }
        if !last.is_none_or(|last| glued(last, token)) {
            out.push(' ');
        }
        out.push_str(token);
        last = Some(token);
    }
    out
}

/// Whether the token at `at` is a separator nothing follows but a closing bracket or another
/// separator.
fn dangling(tokens: &[&str], at: usize) -> bool {
    const CLOSES: &[&str] = &[")", "]", "}", ">"];
    tokens[at] == ","
        && tokens
            .get(at + 1)
            .is_none_or(|next| CLOSES.contains(next) || *next == ",")
}

/// Whether no space stands between these two tokens.
fn glued(last: &str, token: &str) -> bool {
    const NO_SPACE_BEFORE: &[&str] = &[",", ";", ")", "]", ">", ":", "?", ".", "::", "!"];
    const NO_SPACE_AFTER: &[&str] = &["(", "[", "<", "&", "::", ".", "#", "*", "..."];
    const OPENS: &[&str] = &["(", "[", "<"];
    NO_SPACE_AFTER.contains(&last)
        || NO_SPACE_BEFORE.contains(&token)
        || (OPENS.contains(&token) && ends_a_name(last))
}

/// Whether a token is one an argument bracket attaches to directly: a name, a closing bracket
/// or a closing angle.
fn ends_a_name(token: &str) -> bool {
    token == ">"
        || token
            .chars()
            .last()
            .is_some_and(|last| last.is_alphanumeric() || matches!(last, '_' | ')' | ']'))
}

/// The nearest node above this one whose kind is one of these.
pub(crate) fn above<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut holder = node.parent();
    while let Some(found) = holder {
        if kinds.contains(&found.kind()) {
            return Some(found);
        }
        holder = found.parent();
    }
    None
}

/// The text one node covers, and the empty string when it is not valid UTF-8.
pub(crate) fn text_of(node: Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or_default().to_string()
}
