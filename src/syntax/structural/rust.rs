//! What Rust syntax says, with no name resolved and no macro expanded. A `use` keeps the path
//! as it was written beside every path its tree names, and a path from `crate`, `self` or
//! `super` keeps its segments, because the module graph resolves them.

use tree_sitter::Node;

use crate::syntax::structural::{Adapter, Imported, above, text_of};

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
";

/// The node kinds a path of several segments is written as.
const SCOPED: &[&str] = &["scoped_identifier", "scoped_type_identifier"];

/// The first segments a path resolves from inside this crate.
const RELATIVE: &[&str] = &["crate", "self", "super"];

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
    let mut paths = Vec::new();
    expanded(argument, source, &[], &mut paths);
    Imported {
        module: Some(text_of(argument, source)),
        names,
        paths,
    }
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

/// Every path one use tree names under a prefix: the path an alias renames, each entry of a list
/// under the list's own path, the path above a `self` in a list, and a glob as `*`.
fn expanded(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<String>) {
    match node.kind() {
        "use_as_clause" => {
            if let Some(path) = node.child_by_field_name("path") {
                expanded(path, source, prefix, out);
            }
        }
        "scoped_use_list" => scoped_list(node, source, prefix, out),
        "use_list" => each_path(node, source, prefix, out),
        "use_wildcard" => leaf(prefix, node.named_child(0), Some("*"), source, out),
        "self" if !prefix.is_empty() => out.push(prefix.join("::")),
        _ => leaf(prefix, Some(node), None, source, out),
    }
}

fn scoped_list(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<String>) {
    let at = extended(prefix, node.child_by_field_name("path"), source);
    if let (Some(at), Some(list)) = (at, node.child_by_field_name("list")) {
        expanded(list, source, &at, out);
    }
}

fn each_path(node: Node, source: &[u8], prefix: &[String], out: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        expanded(child, source, prefix, out);
    }
}

/// One path a use tree ends at, with a last segment such as a glob's `*` where one is written.
fn leaf(
    prefix: &[String],
    path: Option<Node>,
    last: Option<&str>,
    source: &[u8],
    out: &mut Vec<String>,
) {
    if let Some(mut at) = extended(prefix, path, source) {
        at.extend(last.map(str::to_string));
        out.push(at.join("::"));
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
