//! What Rust syntax says, with no name resolved and no macro expanded. A `use` keeps the path
//! as it was written, because resolving it to a file is #50's work.

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
(mod_item !body) @module
(use_declaration) @import
";

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

/// The path a `use` names, kept verbatim, and the names it binds into this file.
fn imported(node: Node, source: &[u8]) -> Imported {
    let Some(argument) = node.child_by_field_name("argument") else {
        return Imported {
            module: None,
            names: Vec::new(),
        };
    };
    let mut names = Vec::new();
    bound(argument, source, &mut names);
    Imported {
        module: Some(text_of(argument, source)),
        names,
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

/// The file an attribute above a module declaration sends it to, which Rust writes
/// `#[path = "other.rs"]`. Resolving it is #50's work, so the name is kept as it was written.
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
