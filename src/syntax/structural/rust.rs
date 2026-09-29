//! What Rust syntax says, with no name resolved and no macro expanded. A `use` keeps the path
//! as it was written beside every path its tree names, and a path from `crate`, `self` or
//! `super` keeps its segments, because the module graph resolves them.

use tree_sitter::Node;

use crate::syntax::tolerant;

use crate::syntax::structural::{
    Adapter, ExportLeaf, Exported, Imported, Spelling, Visibility, above, spelled, text_of,
};

pub(crate) const ADAPTER: Adapter = Adapter {
    patterns: PATTERNS,
    identifiers: &["identifier", "type_identifier", "field_identifier"],
    methods_in: &["impl_item", "trait_item"],
    entry_points: &["main"],
    visible,
    imported,
    remapped,
    nesting,
    qualified,
    quoted,
    visibility,
    exported_as,
    owner,
    contract,
    exported,
};

const PATTERNS: &str = r"
(function_item) @function
(function_signature_item) @function
(struct_item) @type
(enum_item) @type
(union_item) @type
(trait_item) @type
(type_item) @type
(const_item) @constant
(static_item) @constant
(mod_item) @module
(use_declaration) @import
(use_declaration) @export
(extern_crate_declaration) @export
(extern_crate_declaration) @crate
";

/// The declarations whose contract V1 canonicalizes.
const CONTRACTED: &[&str] = &[
    "function_item",
    "function_signature_item",
    "struct_item",
    "enum_item",
    "union_item",
    "trait_item",
    "type_item",
    "const_item",
    "static_item",
];

/// What never reaches a canonical contract: comments, attributes and the item's own modifier.
/// `noise` lets a directly written `#[non_exhaustive]` through.
const NOISE: &[&str] = &[
    "line_comment",
    "block_comment",
    "attribute_item",
    "inner_attribute_item",
    "visibility_modifier",
];

/// The node kinds a path of several segments is written as.
const SCOPED: &[&str] = &["scoped_identifier", "scoped_type_identifier"];

/// The first segments a path resolves from inside this crate.
const RELATIVE: &[&str] = &["crate", "self", "super"];

/// The `serde` keys whose string is the path of a function the derive calls. `with` names a
/// module, which no one name stands for, so it is not here.
const SERDE_CALLABLES: &[&str] = &[
    "default",
    "skip_serializing_if",
    "serialize_with",
    "deserialize_with",
    "getter",
];

/// Whether this declaration is reachable past the file that holds it: it says so itself, a
/// trait states it, or a trait implementation carries it and the trait exposes it.
fn visible(node: Node) -> bool {
    marked(node) || above(node, &["trait_item"]).is_some() || implements_a_trait(node)
}

fn marked(node: Node) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == "visibility_modifier")
}

fn implements_a_trait(node: Node) -> bool {
    above(node, &["impl_item"]).is_some_and(|holder| holder.child_by_field_name("trait").is_some())
}

/// The path a `use` names, kept verbatim, the names it binds into this file, and every path its
/// tree names.
fn imported(node: Node, source: &[u8]) -> Imported {
    let Some(argument) = node.child_by_field_name("argument") else {
        return Imported {
            module: None,
            names: Vec::new(),
            paths: Vec::new(),
        };
    };
    let mut names = Vec::new();
    bound(argument, source, &mut names);
    Imported {
        module: Some(text_of(argument, source)),
        names,
        paths: leaves(argument, source)
            .into_iter()
            .map(|leaf| leaf.path)
            .collect(),
    }
}

/// Every leaf one use tree names, with the name each binds.
fn leaves(argument: Node, source: &[u8]) -> Vec<ExportLeaf> {
    let mut out = Vec::new();
    expanded(argument, source, &[], &mut out);
    out
}

/// What a declaration's own modifier says. `pub` alone is public, and any `pub(...)` is
/// restricted to some part of the crate.
fn visibility(node: Node, source: &[u8]) -> Visibility {
    let mut cursor = node.walk();
    let modifier = node
        .children(&mut cursor)
        .find(|child| child.kind() == "visibility_modifier");
    match modifier.map(|found| text_of(found, source)) {
        None => Visibility::Private,
        Some(text) if text == "pub" => Visibility::Public,
        Some(_) => Visibility::Restricted,
    }
}

/// Rust addresses every item by its own name, so nothing is exported under another one here;
/// a `pub use ... as` alias is an export leaf.
fn exported_as(_: Node, _: &[u8]) -> Option<String> {
    None
}

/// The type an inherent `impl` adds this method to, by its bare name, and `None` for a method
/// of a trait or of a trait implementation, whose contract belongs to the trait.
fn owner(node: Node, source: &[u8]) -> Option<String> {
    let list = node
        .parent()
        .filter(|held| held.kind() == "declaration_list")?;
    let holder = list
        .parent()
        .filter(|held| held.kind() == "impl_item" && held.child_by_field_name("trait").is_none())?;
    type_name(holder.child_by_field_name("type")?, source)
}

/// The bare name of a type, without its generic arguments or the path before it.
fn type_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "type_identifier" => Some(text_of(node, source)),
        "generic_type" => type_name(node.child_by_field_name("type")?, source),
        "scoped_type_identifier" => Some(text_of(node.child_by_field_name("name")?, source)),
        _ => None,
    }
}

/// The declared contract of one item, canonical, and `None` for a form V1 does not cover. A
/// `#[non_exhaustive]` written above the item leads it.
fn contract(node: Node, source: &[u8]) -> Option<String> {
    if !CONTRACTED.contains(&node.kind()) {
        return None;
    }
    let spelled = spelled(node, source, &|held| spelling(held, source));
    Some(match outer_non_exhaustive(node, source) {
        true => format!("#[non_exhaustive] {spelled}"),
        false => spelled,
    })
}

/// Whether a `#[non_exhaustive]` stands among the attributes and comments directly above a node.
fn outer_non_exhaustive(node: Node, source: &[u8]) -> bool {
    let mut above = node.prev_named_sibling();
    while let Some(held) = above.filter(|held| NOISE.contains(&held.kind())) {
        if non_exhaustive(held, source) {
            return true;
        }
        above = held.prev_named_sibling();
    }
    false
}

/// Whether an attribute item is `#[non_exhaustive]` written directly, and not through
/// `#[cfg_attr(...)]`.
fn non_exhaustive(node: Node, source: &[u8]) -> bool {
    node.kind() == "attribute_item"
        && node
            .named_child(0)
            .filter(|held| held.kind() == "attribute" && held.named_child_count() == 1)
            .and_then(|held| held.named_child(0))
            .is_some_and(|name| text_of(name, source) == "non_exhaustive")
}

/// How one node is spelled in a canonical contract: bodies, initializers, comments, attributes,
/// modifiers and binding names leave, a trait method's default body is `{ .. }`, a private
/// named field leaves and puts `..` in its struct's field list, a private tuple position
/// becomes `_`, a `#[non_exhaustive]` stays, and everything else is kept as written.
fn spelling(node: Node, source: &[u8]) -> Spelling {
    let parent_kind = node.parent().map_or("", |held| held.kind());
    if noise(node, parent_kind, source) {
        return Spelling::Skip;
    }
    match node.kind() {
        "block" if parent_kind == "function_item" => body(node),
        _ if initializer(node, parent_kind) && node.kind() != "=" => {
            Spelling::Replace("..".to_string())
        }
        "field_declaration_list" => fields(node, parent_kind, source),
        "parameter" => parameter(node, source),
        "string_literal" => Spelling::Replace(text_of(node, source)),
        _ => field(node, parent_kind, source),
    }
}

/// Whether a node never reaches a canonical contract: a comment, an attribute other than
/// `#[non_exhaustive]`, the item's own modifier, or the `=` and initializer of a `const` or
/// `static` outside a trait. A trait's default `const` keeps `= ..`, because an implementor may
/// rely on it.
fn noise(node: Node, parent_kind: &str, source: &[u8]) -> bool {
    let in_trait = above(node, &["trait_item"]).is_some();
    (NOISE.contains(&node.kind()) && !non_exhaustive(node, source))
        || (!in_trait && initializer(node, parent_kind))
}

/// A function's body: `{ .. }` for a trait method's default body, which an implementor may
/// rely on, and `;` everywhere else.
fn body(node: Node) -> Spelling {
    let in_trait = node
        .parent()
        .is_some_and(|held| above(held, &["trait_item"]).is_some());
    Spelling::Replace(if in_trait { "{ .. }" } else { ";" }.to_string())
}

/// A struct's named fields as the public ones and a closing `..` where any field is private, so
/// adding the first private field changes the contract and adding another does not. A variant's
/// fields stay as written.
fn fields(list: Node, parent_kind: &str, source: &[u8]) -> Spelling {
    if parent_kind != "struct_item" {
        return Spelling::Keep;
    }
    let mut cursor = list.walk();
    let (public, private): (Vec<Node>, Vec<Node>) = list
        .named_children(&mut cursor)
        .filter(|held| held.kind() == "field_declaration")
        .partition(|held| visibility(*held, source) == Visibility::Public);
    if private.is_empty() {
        return Spelling::Keep;
    }
    let mut shown: Vec<String> = public
        .into_iter()
        .map(|held| spelled(held, source, &|inner| spelling(inner, source)))
        .collect();
    shown.push("..".to_string());
    Spelling::Replace(format!("{{ {} }}", shown.join(", ")))
}

/// Whether this node is the `=` or the value of a `const` or `static`.
fn initializer(node: Node, parent_kind: &str) -> bool {
    matches!(parent_kind, "const_item" | "static_item")
        && (node.kind() == "=" || is_field(node, "value"))
}

/// A parameter as its type alone, with `self` kept where it is the receiver.
fn parameter(node: Node, source: &[u8]) -> Spelling {
    let receiver = node
        .child_by_field_name("pattern")
        .is_some_and(|pattern| pattern.kind() == "self");
    match node.child_by_field_name("type") {
        Some(of) => Spelling::Replace(format!(
            "{}: {}",
            if receiver { "self" } else { "_" },
            spelled(of, source, &|held| spelling(held, source))
        )),
        None => Spelling::Keep,
    }
}

/// A private named field leaves, a private tuple position becomes `_`, and every field of an
/// enum variant stays.
fn field(node: Node, parent_kind: &str, source: &[u8]) -> Spelling {
    if above(node, &["enum_variant"]).is_some() {
        return Spelling::Keep;
    }
    if node.kind() == "field_declaration" && visibility(node, source) != Visibility::Public {
        return Spelling::Skip;
    }
    if parent_kind == "ordered_field_declaration_list"
        && node.is_named()
        && !pub_before(node, source)
    {
        return Spelling::Replace("_".to_string());
    }
    Spelling::Keep
}

/// Whether this node is the child its parent holds under this field name.
fn is_field(node: Node, field: &str) -> bool {
    node.parent()
        .and_then(|held| held.child_by_field_name(field))
        .is_some_and(|found| found.id() == node.id())
}

/// Whether a plain `pub` sits directly before this tuple field's type.
fn pub_before(node: Node, source: &[u8]) -> bool {
    node.prev_sibling()
        .filter(|held| held.kind() == "visibility_modifier")
        .is_some_and(|held| text_of(held, source) == "pub")
}

/// What a plain `pub use` or `pub extern crate` exposes: every leaf of a use tree under the name
/// it binds, or the crate under its alias. A restricted or private one exposes nothing past the
/// module.
fn exported(node: Node, source: &[u8]) -> Option<Exported> {
    if visibility(node, source) != Visibility::Public {
        return None;
    }
    let leaves = match node.kind() {
        "extern_crate_declaration" => vec![extern_crate(node, source)?],
        _ => leaves(node.child_by_field_name("argument")?, source),
    };
    Some(Exported {
        source: None,
        type_only: false,
        supported: true,
        leaves,
    })
}

/// The crate an `extern crate` names, bound under its alias where one is written.
fn extern_crate(node: Node, source: &[u8]) -> Option<ExportLeaf> {
    let path = text_of(node.child_by_field_name("name")?, source);
    let name = node
        .child_by_field_name("alias")
        .map_or_else(|| path.clone(), |alias| text_of(alias, source));
    Some(ExportLeaf {
        path,
        name: Some(name),
    })
}

/// Every name one use tree binds, which is the last segment of a path, the alias where one is
/// written, and each of these again inside a list. A glob binds no name this pass can see.
fn bound(node: Node, source: &[u8], out: &mut Vec<String>) {
    match node.kind() {
        "identifier" => out.push(text_of(node, source)),
        "scoped_identifier" => follow(node, "name", source, out),
        "use_as_clause" => follow(node, "alias", source, out),
        "scoped_use_list" => follow(node, "list", source, out),
        "use_list" => listed(node, source, out),
        _ => {}
    }
}

/// Every path one use tree names under a prefix, with the name it binds: the path an alias
/// renames under the alias, each entry of a list under the list's own path, the path above a
/// `self` in a list under its last segment, and a glob as `*` under no name.
fn expanded(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<ExportLeaf>) {
    match node.kind() {
        "use_as_clause" => {
            let alias = node
                .child_by_field_name("alias")
                .map(|alias| text_of(alias, source));
            if let Some(path) = node.child_by_field_name("path") {
                let from = out.len();
                expanded(path, source, prefix, out);
                for held in &mut out[from..] {
                    held.name = alias.clone().or(held.name.take());
                }
            }
        }
        "scoped_use_list" => scoped_list(node, source, prefix, out),
        "use_list" => each_path(node, source, prefix, out),
        "use_wildcard" => leaf(prefix, node.named_child(0), Some("*"), source, out),
        "self" if !prefix.is_empty() => out.push(ExportLeaf {
            path: prefix.join("::"),
            name: prefix.last().cloned(),
        }),
        _ => leaf(prefix, Some(node), None, source, out),
    }
}

fn scoped_list(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<ExportLeaf>) {
    let at = extended(prefix, node.child_by_field_name("path"), source);
    if let (Some(at), Some(list)) = (at, node.child_by_field_name("list")) {
        expanded(list, source, &at, out);
    }
}

fn each_path(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<ExportLeaf>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        expanded(child, source, prefix, out);
    }
}

/// One path a use tree ends at, with a last segment such as a glob's `*` where one is written,
/// bound under its last segment, and under no name for a glob.
fn leaf(
    prefix: &[String],
    path: Option<Node>,
    last: Option<&str>,
    source: &[u8],
    out: &mut Vec<ExportLeaf>,
) {
    if let Some(mut at) = extended(prefix, path, source) {
        let name = match last {
            Some(_) => None,
            None => at.last().cloned(),
        };
        at.extend(last.map(str::to_string));
        out.push(ExportLeaf {
            path: at.join("::"),
            name,
        });
    }
}

/// The prefix with one more path's segments, the prefix alone where no path is written, and
/// `None` where the path is not one a segment list can say.
fn extended(prefix: &[String], path: Option<Node>, source: &[u8]) -> Option<Vec<String>> {
    let mut at = prefix.to_vec();
    if let Some(path) = path {
        at.extend(segments(path, source)?);
    }
    Some(at)
}

/// The segments of one path, with an empty first segment where the path starts at `::`, and
/// `None` where a segment is not a name, such as a qualified `<T as Trait>`.
fn segments(node: Node, source: &[u8]) -> Option<Vec<String>> {
    match node.kind() {
        "scoped_identifier" | "scoped_type_identifier" => scoped(node, source),
        "generic_type" => segments(node.child_by_field_name("type")?, source),
        "identifier" | "type_identifier" | "crate" | "self" | "super" => {
            Some(vec![text_of(node, source)])
        }
        _ => None,
    }
}

fn scoped(node: Node, source: &[u8]) -> Option<Vec<String>> {
    let mut out = match node.child_by_field_name("path") {
        Some(path) => segments(path, source)?,
        None => vec![String::new()],
    };
    out.push(text_of(node.child_by_field_name("name")?, source));
    Some(out)
}

/// The file an attribute above a module declaration sends it to, which Rust writes
/// `#[path = "other.rs"]`. The name is kept as it was written, and the module graph resolves it.
fn remapped(node: Node, source: &[u8]) -> Option<String> {
    let mut above = node.prev_named_sibling();
    while let Some(held) = above.filter(|held| held.kind() == "attribute_item") {
        if let Some(file) = sends_to(held, source) {
            return Some(file);
        }
        above = held.prev_named_sibling();
    }
    None
}

fn sends_to(item: Node, source: &[u8]) -> Option<String> {
    let attribute = item
        .named_child(0)
        .filter(|held| held.kind() == "attribute")?;
    if text_of(attribute.named_child(0)?, source) != "path" {
        return None;
    }
    let named = attribute.child_by_field_name("value")?;
    Some(text_of(named, source).trim_matches('"').to_string())
}

/// The names a string calls by its value: the terminal segment of the path the value of a
/// `serde` key the derive calls spells, which is the one name the call resolves by, and every
/// name a format string that a macro call or a `macro_rules!` body holds captures.
fn quoted(node: Node, source: &[u8]) -> Vec<String> {
    if !matches!(node.kind(), "string_literal" | "raw_string_literal") {
        return Vec::new();
    }
    let read: fn(&str) -> Vec<String> = if serde_callable(node, source) {
        |value| terminal(value).into_iter().collect()
    } else if above(node, &["macro_invocation", "macro_definition"]).is_some() {
        captures
    } else {
        return Vec::new();
    };
    string_value(node, source)
        .map(|value| read(&value))
        .unwrap_or_default()
}

/// The names the captures of a format string write: the argument of `{name}` or `{name:spec}`,
/// and a width or a precision such as `WIDTH$` in `{:>WIDTH$.PREC$}`. `{{` is a brace, and a
/// position such as `{0}` names nothing.
fn captures(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find('{') {
        rest = &rest[at + 1..];
        if let Some(after) = rest.strip_prefix('{') {
            rest = after;
            continue;
        }
        let inside = &rest[..rest.find('}').unwrap_or(rest.len())];
        let (argument, spec) = inside.split_once(':').unwrap_or((inside, ""));
        out.extend(named(argument.trim()));
        for (at, _) in spec.match_indices('$') {
            let before = &spec[..at];
            let start = before
                .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                .map_or(0, |found| found + 1);
            out.extend(named(&before[start..]));
        }
    }
    out
}

/// This text as a name a capture writes, and `None` for a position or anything else.
fn named(text: &str) -> Option<String> {
    let identifier = text.chars().all(|c| c.is_alphanumeric() || c == '_');
    (identifier && text.starts_with(|c: char| !c.is_ascii_digit())).then(|| text.to_string())
}

/// A string literal's value, its content with every escape sequence decoded, and `None` where
/// an escape is one klin cannot decode. A line continuation drops its newline and the
/// whitespace that follows it, as the compiler does.
fn string_value(node: Node, source: &[u8]) -> Option<String> {
    let mut value = String::new();
    let mut continued = false;
    let mut cursor = node.walk();
    for part in node.named_children(&mut cursor) {
        let text = text_of(part, source);
        match part.kind() {
            "string_content" if continued => {
                value.push_str(text.trim_start_matches(CONTINUED_SPACE))
            }
            "string_content" => value.push_str(&text),
            "escape_sequence" => value.push_str(&unescaped(&text)?),
            _ => {}
        }
        continued = part.kind() == "escape_sequence" && continues(&text);
    }
    Some(value)
}

/// The whitespace a line continuation skips at the start of the next line.
const CONTINUED_SPACE: &[char] = &[' ', '\t', '\n', '\r'];

/// Whether an escape sequence is a line continuation, a backslash before the end of a line.
fn continues(escape: &str) -> bool {
    escape
        .strip_prefix('\\')
        .is_some_and(|body| body.starts_with(['\n', '\r']))
}

/// The escapes that stand for one fixed character.
const SIMPLE_ESCAPES: &[(&str, char)] = &[
    ("n", '\n'),
    ("r", '\r'),
    ("t", '\t'),
    ("0", '\0'),
    ("\\", '\\'),
    ("'", '\''),
    ("\"", '"'),
];

/// What one Rust escape sequence stands for: a Unicode or byte code, a fixed character, or
/// nothing for a line continuation.
fn unescaped(escape: &str) -> Option<String> {
    let body = escape.strip_prefix('\\')?;
    let code = body
        .strip_prefix("u{")
        .and_then(|rest| rest.strip_suffix('}'))
        .or_else(|| body.strip_prefix('x'));
    if let Some(code) = code {
        let code = u32::from_str_radix(&code.replace('_', ""), 16).ok()?;
        return char::from_u32(code).map(String::from);
    }
    if continues(escape) {
        return Some(String::new());
    }
    SIMPLE_ESCAPES
        .iter()
        .find(|(written, _)| *written == body)
        .map(|(_, value)| value.to_string())
}

/// The function a decoded path names, as the Rust grammar reads the path as an expression, so
/// generic arguments, a qualified-self prefix such as `<T as Trait>` and a const-generic block
/// with any literal or comment inside it never change which name the path ends in. A value the
/// grammar does not read as exactly one path names nothing.
fn terminal(path: &str) -> Option<String> {
    let snippet = format!("fn f() {{ {path}; }}");
    let file = tolerant("path.rs", &snippet)?;
    let root = file.root();
    let body = root
        .named_child(0)
        .filter(|_| !root.has_error() && root.named_child_count() == 1)?
        .child_by_field_name("body")
        .filter(|body| body.named_child_count() == 1)?;
    let statement = body
        .named_child(0)
        .filter(|held| held.kind() == "expression_statement")?;
    callable(statement.named_child(0)?, file.bytes())
}

/// The name a path expression ends in.
fn callable(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(text_of(node, source)),
        "scoped_identifier" => callable(node.child_by_field_name("name")?, source),
        "generic_function" => callable(node.child_by_field_name("function")?, source),
        _ => None,
    }
}

/// Whether this string follows one of `SERDE_CALLABLES` and `=` in the tokens of a `serde(...)`
/// that a Rust attribute holds.
fn serde_callable(node: Node, source: &[u8]) -> bool {
    let key = node
        .prev_sibling()
        .filter(|held| held.kind() == "=")
        .and_then(|held| held.prev_sibling());
    let tokens = node.parent().filter(|held| held.kind() == "token_tree");
    key.is_some_and(|held| SERDE_CALLABLES.contains(&text_of(held, source).as_str()))
        && tokens.is_some_and(|tokens| serde_attribute(tokens, source))
}

/// Whether these tokens are the arguments of `serde`, as an attribute of its own or directly
/// inside a `cfg_attr` attribute, so the same tokens inside a macro call never count.
fn serde_attribute(tokens: Node, source: &[u8]) -> bool {
    let named =
        |held: Node, name: &str| held.kind() == "identifier" && text_of(held, source) == name;
    if !tokens
        .prev_sibling()
        .is_some_and(|held| named(held, "serde"))
    {
        return false;
    }
    let attribute = match tokens.parent() {
        Some(held) if held.kind() == "attribute" => held,
        Some(held) if held.kind() == "token_tree" => match held.parent() {
            Some(outer)
                if outer.kind() == "attribute"
                    && outer
                        .named_child(0)
                        .is_some_and(|name| named(name, "cfg_attr")) =>
            {
                outer
            }
            _ => return false,
        },
        _ => return false,
    };
    attribute
        .parent()
        .is_some_and(|item| matches!(item.kind(), "attribute_item" | "inner_attribute_item"))
}

/// The names of the inline modules above a node, outermost first.
fn nesting(node: Node, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut holder = node.parent();
    while let Some(found) = holder {
        let inline = found.kind() == "mod_item" && found.child_by_field_name("body").is_some();
        if let Some(name) = found.child_by_field_name("name").filter(|_| inline) {
            names.push(text_of(name, source));
        }
        holder = found.parent();
    }
    names.reverse();
    names
}

/// The path this node writes from `crate`, `self` or `super`, where the node is the whole path
/// and not a visibility such as `pub(in crate::a)`.
fn qualified(node: Node, source: &[u8]) -> Option<String> {
    if !SCOPED.contains(&node.kind())
        || inside_a_longer_path(node)
        || above(node, &["visibility_modifier"]).is_some()
    {
        return None;
    }
    let segments = segments(node, source)?;
    RELATIVE
        .contains(&segments.first()?.as_str())
        .then(|| segments.join("::"))
}

/// Whether this path is the leading part of a longer one, directly or through the generic
/// arguments of one of its segments.
fn inside_a_longer_path(node: Node) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let (holder, part) = match (parent.kind(), parent.parent()) {
        ("generic_type", Some(up)) => (up, parent),
        _ => (parent, node),
    };
    SCOPED.contains(&holder.kind())
        && holder
            .child_by_field_name("path")
            .is_some_and(|path| path.id() == part.id())
}

fn follow(node: Node, field: &str, source: &[u8], out: &mut Vec<String>) {
    if let Some(child) = node.child_by_field_name(field) {
        bound(child, source, out);
    }
}

fn listed(node: Node, source: &[u8], out: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        bound(child, source, out);
    }
}
