use std::io::BufRead;
use tree_sitter::{Node, Parser};

const FUNCTIONS: &[&str] = &[
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "generator_function_declaration",
    "function_item",
    "closure_expression",
];
const DECLARATIVE: &[&str] = &[
    "object",
    "array",
    "type_annotation",
    "interface_declaration",
    "type_alias_declaration",
    "enum_declaration",
    "import_statement",
    "export_clause",
    "type_arguments",
    "object_type",
];
const JSX: &[&str] = &["jsx_element", "jsx_self_closing_element", "jsx_fragment"];
const CALLS: &[&str] = &["call_expression", "new_expression", "macro_invocation"];
const CONTROL_OR_AWAIT: &[&str] = &[
    "if_statement",
    "switch_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "try_statement",
    "ternary_expression",
    "await_expression",
    "if_expression",
    "match_expression",
    "for_expression",
    "while_expression",
    "loop_expression",
    "try_expression",
];
const STATEMENTS: &[&str] = &[
    "expression_statement",
    "lexical_declaration",
    "if_statement",
    "switch_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "do_statement",
    "try_statement",
    "return_statement",
    "throw_statement",
    "break_statement",
    "continue_statement",
    "labeled_statement",
    "with_statement",
    "debugger_statement",
    "empty_statement",
    "let_declaration",
];

#[derive(Default)]
struct Count {
    total: usize,
    jsx: usize,
    declarative: usize,
    in_units: usize,
    calls: usize,
    control_or_await: usize,
    statements: usize,
}

fn kind_of(node: Node) -> Option<&'static str> {
    let mut cur = node.parent();
    while let Some(n) = cur {
        let k = n.kind();
        if k == "jsx_expression" || FUNCTIONS.contains(&k) || k == "statement_block" {
            return Some("logic");
        }
        if JSX.contains(&k) {
            return Some("jsx");
        }
        if DECLARATIVE.contains(&k) {
            return Some("declarative");
        }
        cur = n.parent();
    }
    None
}

fn leaves<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.child_count() == 0 {
        if !node.kind().contains("comment") && !node.is_missing() {
            out.push(node);
        }
        return;
    }
    let mut c = node.walk();
    for ch in node.children(&mut c) {
        leaves(ch, out);
    }
}

fn units<'a>(node: Node<'a>, lo: usize, hi: usize, out: &mut Vec<(usize, usize)>) {
    let (s, e) = (node.start_position().row, node.end_position().row);
    if e < lo || s > hi {
        return;
    }
    if FUNCTIONS.contains(&node.kind()) && s >= lo && e <= hi {
        out.push((node.start_byte(), node.end_byte()));
        return;
    }
    let mut c = node.walk();
    for ch in node.children(&mut c) {
        units(ch, lo, hi, out);
    }
}

fn count_g4(node: Node, lo: usize, hi: usize, count: &mut Count) {
    let (start, end) = (node.start_position().row, node.end_position().row);
    if end < lo || start > hi {
        return;
    }
    if start >= lo && end <= hi && !node.is_missing() && !node.has_error() {
        let kind = node.kind();
        if CALLS.contains(&kind) {
            count.calls += 1;
        }
        if CONTROL_OR_AWAIT.contains(&kind) {
            count.control_or_await += 1;
        }
        if STATEMENTS.contains(&kind) {
            count.statements += 1;
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count_g4(child, lo, hi, count);
    }
}

fn main() {
    let ts = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let tsx = tree_sitter_typescript::LANGUAGE_TSX.into();
    let rust = tree_sitter_rust::LANGUAGE.into();
    for line in std::io::stdin().lock().lines() {
        let q: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let path = q["file"].as_str().unwrap();
        let (lo, hi) = (
            q["start"].as_u64().unwrap() as usize - 1,
            q["end"].as_u64().unwrap() as usize - 1,
        );
        let src = std::fs::read(path).unwrap();
        let mut p = Parser::new();
        p.set_language(if path.ends_with(".tsx") {
            &tsx
        } else if path.ends_with(".rs") {
            &rust
        } else {
            &ts
        })
        .unwrap();
        let tree = p.parse(&src, None).unwrap();
        let mut all = Vec::new();
        leaves(tree.root_node(), &mut all);
        let mut whole = Vec::new();
        units(tree.root_node(), lo, hi, &mut whole);
        let mut c = Count::default();
        count_g4(tree.root_node(), lo, hi, &mut c);
        for leaf in all
            .iter()
            .filter(|l| (lo..=hi).contains(&l.start_position().row))
        {
            c.total += 1;
            match kind_of(*leaf) {
                Some("jsx") => c.jsx += 1,
                Some("declarative") => c.declarative += 1,
                _ => {}
            }
            if whole
                .iter()
                .any(|&(s, e)| leaf.start_byte() >= s && leaf.end_byte() <= e)
            {
                c.in_units += 1;
            }
        }
        println!(
            "{}",
            serde_json::json!({"total": c.total, "jsx": c.jsx, "declarative": c.declarative, "in_units": c.in_units, "calls": c.calls, "control_or_await": c.control_or_await, "statements": c.statements})
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragment_counts_calls_control_flow_and_complete_statements() {
        let source = "async function route(ctx) {\n  const item = await getItem(ctx);\n  if (!item) return notFound();\n  await save(item);\n  return item;\n}\n";
        let mut parser = Parser::new();
        let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        parser.set_language(&language).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut count = Count::default();
        count_g4(tree.root_node(), 1, 4, &mut count);

        assert_eq!(count.calls, 3);
        assert_eq!(count.control_or_await, 3);
        assert_eq!(count.statements, 5);
    }

    #[test]
    fn rust_fragment_counts_calls_control_flow_and_complete_statements() {
        let source = "fn route(ctx: &Ctx) -> Result<Item> {\n    let item = load(ctx)?;\n    if item.empty() {\n        bail!(\"empty\");\n    }\n    match item.kind {\n        Kind::A => save(&item),\n        _ => {}\n    }\n    Ok(item)\n}\n";
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE.into();
        parser.set_language(&language).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let mut count = Count::default();
        count_g4(tree.root_node(), 1, 9, &mut count);

        assert_eq!(count.calls, 5);
        assert_eq!(count.control_or_await, 3);
        assert_eq!(count.statements, 4);
    }
}
