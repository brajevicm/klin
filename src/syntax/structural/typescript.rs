//! What TypeScript syntax says. One adapter serves both grammars, because a `.tsx` file is
//! TypeScript and TSX is a grammar variant rather than a language. A module specifier is kept
//! as it was written, because resolving it to a file is #50's work.

use tree_sitter::Node;

use crate::syntax::structural::{Adapter, Imported, above, text_of};
use crate::syntax::walk;

pub(crate) const ADAPTER: Adapter = Adapter {
    patterns: PATTERNS,
    identifiers: &[
        "identifier",
        "type_identifier",
        "property_identifier",
        "shorthand_property_identifier",
    ],
    methods_in: &[],
    entry_points: &[],
    visible,
    imported,
    remapped,
    nesting,
    qualified,
};

const PATTERNS: &str = r#"
(function_declaration) @function
(generator_function_declaration) @function
(method_definition) @method
(method_signature) @method
(abstract_method_signature) @method
(class_declaration) @type
(abstract_class_declaration) @type
(interface_declaration) @type
(type_alias_declaration) @type
(enum_declaration) @type
(lexical_declaration "const" (variable_declarator) @constant)
(lexical_declaration "let" (variable_declarator) @variable)
(variable_declaration (variable_declarator) @variable)
(import_statement) @import
(export_statement source: (string)) @import
"#;

/// Whether this declaration leaves the file: an `export` above it says so, and a member of an
/// exported class leaves with the class.
fn visible(node: Node) -> bool {
    above(node, &["export_statement"]).is_some()
}

/// The module specifier an import or a re-export names, without its quotes, and the names it
/// binds into this file.
fn imported(node: Node, source: &[u8]) -> Imported {
    Imported {
        module: node
            .child_by_field_name("source")
            .map(|from| specifier(from, source)),
        names: bindings(node, source),
        paths: Vec::new(),
    }
}

/// TypeScript writes no module declaration, so none of them is remapped.
fn remapped(_: Node, _: &[u8]) -> Option<String> {
    None
}

/// A TypeScript file is one module, so nothing inside it is held by another.
fn nesting(_: Node, _: &[u8]) -> Vec<String> {
    Vec::new()
}

/// A TypeScript file reaches another module only through an import specifier.
fn qualified(_: Node, _: &[u8]) -> Option<String> {
    None
}

fn specifier(node: Node, source: &[u8]) -> String {
    text_of(node, source)
        .trim_matches(['"', '\'', '`'])
        .to_string()
}

fn bindings(node: Node, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    walk(node, &mut |found| {
        if let Some(name) = binding(found, source) {
            out.push(name);
        }
    });
    out
}

/// The name one clause binds: the alias where a specifier renames what it takes, the name
/// otherwise, and the single identifier a default or namespace import writes.
fn binding(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "import_specifier" | "export_specifier" => Some(text_of(alias_or_name(node)?, source)),
        "namespace_import" | "import_clause" => Some(text_of(sole_identifier(node)?, source)),
        _ => None,
    }
}

fn alias_or_name(node: Node) -> Option<Node> {
    node.child_by_field_name("alias")
        .or_else(|| node.child_by_field_name("name"))
}

fn sole_identifier(node: Node) -> Option<Node> {
    node.named_child(0)
        .filter(|child| child.kind() == "identifier")
}
