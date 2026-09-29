//! What TypeScript syntax says. One adapter serves both grammars, because a `.tsx` file is
//! TypeScript and TSX is a grammar variant rather than a language. A module specifier is kept
//! as it was written, because resolving it to a file is #50's work. An `export` at the top of a
//! file is what exposes a name past the module, and only an explicit type annotation is a
//! declared contract: a type the compiler would infer is spelled `?`.

use tree_sitter::Node;

use crate::syntax::structural::{
    Adapter, ExportLeaf, Exported, Imported, Spelling, Visibility, above, spelled, text_of,
};
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
    visibility,
    exported_as,
    owner,
    contract,
    exported,
};

const PATTERNS: &str = r#"
(function_declaration) @function
(generator_function_declaration) @function
(function_signature) @function
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
(export_statement) @export
"#;

/// The declarations an `export` statement says everything about for itself, so the statement
/// adds no export fact of its own.
const DECLARED: &[&str] = &[
    "function_declaration",
    "generator_function_declaration",
    "function_signature",
    "class_declaration",
    "abstract_class_declaration",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
    "lexical_declaration",
    "variable_declaration",
];

/// The nodes that only wrap a declaration on its way up to the `export` that exposes it.
const WRAPPERS: &[&str] = &[
    "lexical_declaration",
    "variable_declaration",
    "ambient_declaration",
];

/// The declarations spelled like a function: a header, parameters and a return type.
const FUNCTION_LIKE: &[&str] = &[
    "function_declaration",
    "generator_function_declaration",
    "function_signature",
    "method_definition",
    "method_signature",
    "abstract_method_signature",
    "call_signature",
    "construct_signature",
];

/// The members a class or an interface body lists in its contract.
const MEMBERS: &[&str] = &[
    "method_definition",
    "method_signature",
    "abstract_method_signature",
    "public_field_definition",
    "property_signature",
    "call_signature",
    "construct_signature",
    "index_signature",
];

const INFERRED: &str = ": ?";

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

/// The `export` statement at the top of the file that exposes this declaration directly, through
/// the `const`, `let`, `var` or `declare` that wraps it, and `None` for a declaration exposed
/// no such way: a member of a class, or a name inside a namespace or an ambient module.
fn exporting(node: Node) -> Option<Node> {
    let mut at = node;
    while let Some(parent) = at.parent().filter(|held| WRAPPERS.contains(&held.kind())) {
        at = parent;
    }
    at.parent()
        .filter(|held| held.kind() == "export_statement")
        .filter(|held| held.parent().is_some_and(|top| top.kind() == "program"))
}

/// What exposes a declaration: `export` at the top of the file, or nothing. TypeScript writes
/// no restricted form.
fn visibility(node: Node, _: &[u8]) -> Visibility {
    match exporting(node) {
        Some(_) => Visibility::Public,
        None => Visibility::Private,
    }
}

/// `default` where the export that exposes the declaration says so, so a consumer addresses it
/// by that name and never by the declaration's own.
fn exported_as(node: Node, _: &[u8]) -> Option<String> {
    exporting(node)
        .filter(|held| has_token(*held, "default"))
        .map(|_| "default".to_string())
}

/// A member belongs to its class, so no method here has an owner of its own.
fn owner(_: Node, _: &[u8]) -> Option<String> {
    None
}

fn has_token(node: Node, token: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| !child.is_named() && child.kind() == token)
}

/// The declared contract of one declaration, canonical, and `None` for a class member, whose
/// contract is part of its class. An implementation that follows its overload signatures adds
/// nothing to the set and is empty.
fn contract(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        _ if implementation(node, source) => Some(String::new()),
        kind if FUNCTION_LIKE.contains(&kind) && above(node, &["class_body"]).is_none() => {
            Some(function_like(node, source))
        }
        "variable_declarator" => Some(declarator(node, source)),
        "class_declaration"
        | "abstract_class_declaration"
        | "interface_declaration"
        | "type_alias_declaration"
        | "enum_declaration" => Some(canonical(node, source)),
        _ => None,
    }
}

/// Whether a function declaration directly follows an overload signature of its own name, so a
/// consumer never calls it.
fn implementation(node: Node, source: &[u8]) -> bool {
    if !matches!(
        node.kind(),
        "function_declaration" | "generator_function_declaration"
    ) {
        return false;
    }
    let name = |held: Node| {
        held.child_by_field_name("name")
            .map(|name| text_of(name, source))
    };
    let at = node
        .parent()
        .filter(|held| held.kind() == "export_statement")
        .unwrap_or(node);
    let mut before = at.prev_named_sibling();
    while let Some(held) = before.filter(|held| held.kind() == "comment") {
        before = held.prev_named_sibling();
    }
    let signature = match before {
        Some(held) if held.kind() == "export_statement" => held.child_by_field_name("declaration"),
        other => other,
    };
    signature.is_some_and(|held| held.kind() == "function_signature" && name(held) == name(node))
}

fn canonical(node: Node, source: &[u8]) -> String {
    let text = spelled(node, source, &|held| spelling(held, source));
    text.trim_end_matches(';').trim_end().to_string()
}

/// A function's header, parameters and return type, with `?` for a return type the compiler
/// would infer.
fn function_like(node: Node, source: &[u8]) -> String {
    let mut text = canonical(node, source);
    let constructor = node
        .child_by_field_name("name")
        .is_some_and(|name| text_of(name, source) == "constructor");
    if node.child_by_field_name("return_type").is_none()
        && !constructor
        && !matches!(node.kind(), "construct_signature")
    {
        text.push_str(INFERRED);
    }
    if node.kind() == "construct_signature" && node.child_by_field_name("type").is_none() {
        text.push_str(INFERRED);
    }
    text
}

/// `const name: Type`, with the keyword its declaration wrote and `?` for a type the compiler
/// would infer. The initializer is implementation and never spelled.
fn declarator(node: Node, source: &[u8]) -> String {
    let keyword = node
        .parent()
        .and_then(|held| held.child_by_field_name("kind"))
        .map_or_else(|| "var".to_string(), |kind| text_of(kind, source));
    let name = node
        .child_by_field_name("name")
        .map(|name| text_of(name, source))
        .unwrap_or_default();
    format!("{keyword} {name}{}", annotated(node, source))
}

/// The type annotation a node carries, spelled, and `?` where it carries none.
fn annotated(node: Node, source: &[u8]) -> String {
    match node.child_by_field_name("type") {
        Some(of) => spelled(of, source, &|held| spelling(held, source)),
        None => INFERRED.to_string(),
    }
}

/// How one node is spelled in a canonical contract: bodies but a namespace's, initializers,
/// comments and decorators leave, a private member leaves, a parameter keeps its type and loses
/// its binding name, and a class or interface body lists its members in one order.
fn spelling(node: Node, source: &[u8]) -> Spelling {
    match node.kind() {
        "comment" | "decorator" => Spelling::Skip,
        "statement_block" if is_field(node, "body") && !namespace_body(node) => Spelling::Skip,
        "required_parameter" | "optional_parameter" => Spelling::Replace(parameter(node, source)),
        "class_body" | "interface_body" => Spelling::Replace(members(node, source)),
        "string" => Spelling::Replace(text_of(node, source)),
        _ => Spelling::Keep,
    }
}

/// One parameter: its accessibility where it declares a property, `_` for a binding name that
/// is no contract, `...` for a rest parameter, `?` where a caller may omit it, and its type or
/// `?`. A default no required parameter follows makes it omittable. A default a required
/// parameter follows shows as `= ..`, because a caller passes `undefined` to reach it. Its value
/// never shows.
fn parameter(node: Node, source: &[u8]) -> String {
    let modifiers = modifiers(node, source);
    let pattern = node.child_by_field_name("pattern");
    let rest = pattern.is_some_and(|held| held.kind() == "rest_pattern");
    let name = match (modifiers.is_empty(), pattern) {
        (false, Some(held)) => text_of(held, source),
        (true, Some(held)) if held.kind() == "this" => "this".to_string(),
        _ if rest => "..._".to_string(),
        _ => "_".to_string(),
    };
    let (optional, reached) = omission(node, !modifiers.is_empty());
    let mut out = modifiers.join(" ");
    if !out.is_empty() {
        out.push(' ');
    }
    format!("{out}{name}{optional}{}{reached}", annotated(node, source))
}

/// The modifiers that make a constructor parameter declare a property of its class.
fn modifiers(node: Node, source: &[u8]) -> Vec<String> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| {
            matches!(
                child.kind(),
                "accessibility_modifier" | "override_modifier" | "readonly"
            )
        })
        .map(|child| text_of(child, source))
        .collect()
}

/// The public and protected properties a constructor declares through its parameters, each
/// spelled as a member of its class: its modifiers, its name, `?` where optional, and its type.
fn properties(constructor: Node, source: &[u8]) -> Vec<String> {
    let Some(list) = constructor.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut cursor = list.walk();
    list.named_children(&mut cursor)
        .filter_map(|held| {
            let modifiers = modifiers(held, source);
            let name = held.child_by_field_name("pattern")?;
            let optional = if held.kind() == "optional_parameter" {
                "?"
            } else {
                ""
            };
            (!modifiers.is_empty() && !modifiers.iter().any(|modifier| modifier == "private")).then(
                || {
                    format!(
                        "{} {}{optional}{}",
                        modifiers.join(" "),
                        text_of(name, source),
                        annotated(held, source)
                    )
                },
            )
        })
        .collect()
}

/// How a caller may leave a parameter out: `?` where it is optional or its default no required
/// parameter follows, and ` = ..` where a required parameter follows its default. A default on
/// a parameter that declares a property is always ` = ..`, because the property it declares is
/// never `undefined` the way an optional one may be.
fn omission(node: Node, property: bool) -> (&'static str, &'static str) {
    let defaulted = node.child_by_field_name("value").is_some();
    match (node.kind() == "optional_parameter", defaulted) {
        (true, _) => ("?", ""),
        (false, true) if property || required_after(node) => ("", " = .."),
        (false, true) => ("?", ""),
        (false, false) => ("", ""),
    }
}

/// Whether a required parameter follows this one: one with no `?`, no default and no rest.
fn required_after(node: Node) -> bool {
    let mut after = node.next_named_sibling();
    while let Some(held) = after {
        let rest = held
            .child_by_field_name("pattern")
            .is_some_and(|pattern| pattern.kind() == "rest_pattern");
        if held.kind() == "required_parameter"
            && held.child_by_field_name("value").is_none()
            && !rest
        {
            return true;
        }
        after = held.next_named_sibling();
    }
    false
}

/// The members of a class or interface body, each spelled, private ones left out, in one order
/// whatever order the source wrote them in.
fn members(body: Node, source: &[u8]) -> String {
    let mut listed = overload_sets(body, source);
    listed.sort();
    listed.dedup();
    format!("{{ {} }}", listed.join("; "))
}

/// The public members of a body, one entry each, except that the overloads of one method, call
/// or construct signature share an entry in source order, and an implementation that directly
/// follows them leaves, keeping only the properties a constructor declares through its
/// parameters.
fn overload_sets(body: Node, source: &[u8]) -> Vec<String> {
    let mut cursor = body.walk();
    let mut sets: Vec<(Option<String>, Vec<String>)> = Vec::new();
    let mut signed: Option<String> = None;
    for child in body
        .named_children(&mut cursor)
        .filter(|child| listed(*child, source))
    {
        let key = overloadable(child, source);
        let implementation = child.kind() == "method_definition";
        let follows = key.is_some() && signed == key;
        signed = key.clone().filter(|_| !implementation);
        if implementation && follows {
            sets.extend(
                properties(child, source)
                    .into_iter()
                    .map(|held| (None, vec![held])),
            );
            continue;
        }
        joined(&mut sets, key, member(child, source));
    }
    sets.into_iter()
        .map(|(_, texts)| texts.join("; "))
        .collect()
}

/// Whether a body lists this member in its contract: a member that is not private, and a
/// private constructor, because it stops a consumer constructing the class.
fn listed(node: Node, source: &[u8]) -> bool {
    let constructor = node
        .child_by_field_name("name")
        .is_some_and(|name| text_of(name, source) == "constructor");
    MEMBERS.contains(&node.kind()) && (constructor || !private(node, source))
}

/// One member's text added to the overload set its key names, or to a set of its own.
fn joined(sets: &mut Vec<(Option<String>, Vec<String>)>, key: Option<String>, text: String) {
    let set = key
        .as_ref()
        .and_then(|key| sets.iter_mut().find(|(held, _)| held.as_ref() == Some(key)));
    match set {
        Some((_, texts)) => texts.push(text),
        None => sets.push((key, vec![text])),
    }
}

/// What one overload set of a body is known by: the kind of signature, `static`, `get` or `set`
/// where written, and the name without the quotes or brackets a string name is written in. Accessibility
/// and `async` are no part of it, and a member that has no overloads has none.
fn overloadable(node: Node, source: &[u8]) -> Option<String> {
    let kind = match node.kind() {
        "method_definition" | "method_signature" | "abstract_method_signature" => "method",
        "call_signature" => "call",
        "construct_signature" => "construct",
        _ => return None,
    };
    let mut key = kind.to_string();
    for mark in ["static", "get", "set"] {
        if has_token(node, mark) {
            key.push(' ');
            key.push_str(mark);
        }
    }
    if let Some(name) = node.child_by_field_name("name") {
        key.push(' ');
        let written = text_of(name, source);
        let computed = written
            .strip_prefix('[')
            .and_then(|inner| inner.strip_suffix(']'))
            .filter(|inner| inner.starts_with(['"', '\'']));
        key.push_str(computed.unwrap_or(&written).trim_matches(['"', '\'']));
    }
    Some(key)
}

fn member(node: Node, source: &[u8]) -> String {
    if FUNCTION_LIKE.contains(&node.kind()) {
        return function_like(node, source);
    }
    let text = spelled(node, source, &|held| {
        let own = held.parent().is_some_and(|parent| parent.id() == node.id());
        match own && (held.kind() == "=" || is_field(held, "value")) {
            true => Spelling::Skip,
            false => spelling(held, source),
        }
    });
    let text = text.trim_end_matches(';').trim_end().to_string();
    match node.kind() {
        "public_field_definition" | "property_signature"
            if node.child_by_field_name("type").is_none() =>
        {
            format!("{text}{INFERRED}")
        }
        _ => text,
    }
}

/// Whether a member is no part of the class's contract: `private`, or a `#name`.
fn private(node: Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    let declared_private = node.children(&mut cursor).any(|child| {
        child.kind() == "accessibility_modifier" && text_of(child, source) == "private"
    });
    declared_private
        || node
            .child_by_field_name("name")
            .is_some_and(|name| name.kind() == "private_property_identifier")
}

fn is_field(node: Node, field: &str) -> bool {
    node.parent()
        .and_then(|held| held.child_by_field_name(field))
        .is_some_and(|found| found.id() == node.id())
}

fn namespace_body(node: Node) -> bool {
    node.parent()
        .is_some_and(|held| held.kind() == "internal_module")
}

/// What one `export` statement at the top of the file exposes beyond its own declaration: a
/// default value, a clause of local names, a re-export clause, a star, a namespace star, a
/// namespace, or a form V1 recognizes and cannot list.
fn exported(node: Node, source: &[u8]) -> Option<Exported> {
    if node.parent().is_none_or(|top| top.kind() != "program") {
        return None;
    }
    if let Some(declaration) = node.child_by_field_name("declaration") {
        return match namespace(declaration) {
            Some(held) => namespace_export(held, source),
            None => (!declared(declaration)).then(unsupported),
        };
    }
    if has_token(node, "default") {
        return Some(default_export(node, source));
    }
    if has_token(node, "=") {
        return Some(unsupported());
    }
    let Some(leaves) = clause_leaves(node, source) else {
        return Some(unsupported());
    };
    Some(Exported {
        source: node
            .child_by_field_name("source")
            .map(|from| specifier(from, source)),
        type_only: has_token(node, "type"),
        supported: true,
        leaves,
        namespace: None,
    })
}

/// `export namespace N`, through a `declare` too: the name it exposes and its whole
/// declaration, canonical.
fn namespace_export(node: Node, source: &[u8]) -> Option<Exported> {
    let name = text_of(node.child_by_field_name("name")?, source);
    Some(Exported {
        source: None,
        type_only: false,
        supported: true,
        leaves: vec![ExportLeaf {
            path: name.clone(),
            name: Some(name),
        }],
        namespace: Some(canonical(node, source)),
    })
}

/// `export default x`: the local name where one is written, and an empty path for an
/// anonymous value.
fn default_export(node: Node, source: &[u8]) -> Exported {
    let path = node
        .child_by_field_name("value")
        .filter(|held| held.kind() == "identifier")
        .map(|held| text_of(held, source))
        .unwrap_or_default();
    Exported {
        source: None,
        type_only: false,
        supported: true,
        leaves: vec![ExportLeaf {
            path,
            name: Some("default".to_string()),
        }],
        namespace: None,
    }
}

/// The leaves of an export clause, a namespace star or a bare star, and `None` for any other
/// form.
fn clause_leaves(node: Node, source: &[u8]) -> Option<Vec<ExportLeaf>> {
    let mut cursor = node.walk();
    let clause = node
        .named_children(&mut cursor)
        .find(|child| matches!(child.kind(), "export_clause" | "namespace_export"));
    Some(match clause {
        Some(held) if held.kind() == "namespace_export" => vec![ExportLeaf {
            path: "*".to_string(),
            name: held.named_child(0).map(|name| text_of(name, source)),
        }],
        Some(held) => specifiers(held, source),
        None if has_token(node, "*") => vec![ExportLeaf {
            path: "*".to_string(),
            name: None,
        }],
        None => return None,
    })
}

/// Whether an exported declaration says everything about itself, through a `declare` too.
fn declared(node: Node) -> bool {
    match node.kind() {
        "ambient_declaration" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .any(|child| DECLARED.contains(&child.kind()))
        }
        kind => DECLARED.contains(&kind),
    }
}

/// The namespace an exported declaration is, through a `declare` too.
fn namespace(node: Node) -> Option<Node> {
    match node.kind() {
        "internal_module" => Some(node),
        "ambient_declaration" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .find(|child| child.kind() == "internal_module")
        }
        _ => None,
    }
}

fn unsupported() -> Exported {
    Exported {
        source: None,
        type_only: false,
        supported: false,
        leaves: Vec::new(),
        namespace: None,
    }
}

/// Every specifier of an export clause: the name it exposes and, where `as` renames it, the
/// alias it is exposed under.
fn specifiers(clause: Node, source: &[u8]) -> Vec<ExportLeaf> {
    let mut cursor = clause.walk();
    clause
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "export_specifier")
        .filter_map(|child| {
            let name = child.child_by_field_name("name")?;
            Some(ExportLeaf {
                path: text_of(name, source),
                name: Some(text_of(alias_or_name(child)?, source)),
            })
        })
        .collect()
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
