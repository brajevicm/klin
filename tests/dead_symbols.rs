mod harness;

use std::fmt::Write;

use harness::Tree;

const RUST: &str = r#"{"dead_symbols":{"roots":["src"],"languages":["rust"]}}"#;
const TYPESCRIPT: &str = r#"{"dead_symbols":{"roots":["src"],"languages":["typescript"]}}"#;

#[test]
fn a_new_private_unreferenced_rust_function_fails_as_new() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/lib.rs", "fn unused() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new dead symbol(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:1"), "{}", run.out);
}

#[test]
fn a_private_function_referenced_from_another_file_passes() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/lib.rs", "fn used() {}\n");
    tree.write("src/main.rs", "fn main() { used(); }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn losing_the_last_reference_is_worsened_and_names_the_old_reference_file() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/lib.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "fn main() { helper(); }\n");
    tree.base();
    tree.write("src/caller.rs", "fn main() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("lost reference in src/caller.rs"), "{}", run.out);
}

#[test]
fn a_dead_symbol_already_in_the_base_is_a_note_and_does_not_fail() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/lib.rs", "fn old_debt() {}\n");
    tree.base();

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: 1 dead symbol(s) the base already held:"),
        "{}",
        run.out
    );
}

#[test]
fn externally_visible_and_test_functions_are_not_judged() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "pub fn api() {}\n\n#[test]\nfn test_api() {}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn one_reference_keeps_all_duplicate_names_alive() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/one.rs", "fn same() {}\n");
    tree.write("src/two.rs", "fn same() {}\n");
    tree.write("src/main.rs", "fn main() { same(); }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn configured_name_globs_are_ignored() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"dead_symbols":{"roots":["src"],"languages":["rust"],"ignore":["generated_*"]}}"#,
    );
    tree.write("src/lib.rs", "fn generated_helper() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn a_new_private_typescript_function_fails() {
    let tree = Tree::new();
    tree.write("klin.json", TYPESCRIPT);
    tree.write("src/index.ts", "function unused() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/index.ts:1"), "{}", run.out);
}

#[test]
fn a_private_typescript_main_is_judged() {
    let tree = Tree::new();
    tree.write("klin.json", TYPESCRIPT);
    tree.write("src/index.ts", "function main() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/index.ts:1"), "{}", run.out);
}

#[test]
fn one_typescript_reference_keeps_duplicate_names_alive() {
    let tree = Tree::new();
    tree.write("klin.json", TYPESCRIPT);
    tree.write("src/one.ts", "function same() {}\n");
    tree.write("src/two.ts", "function same() {}\n");
    tree.write("src/entry.ts", "export function entry() { same(); }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn tsx_is_selected_through_typescript_not_as_a_structural_language() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"dead_symbols":{"roots":["src"],"languages":["tsx"]}}"#,
    );
    tree.write("src/App.tsx", "function Component() { return null; }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no language called \"tsx\""), "{}", run.out);
}

#[test]
fn exported_typescript_and_tsx_declarations_use_the_shared_logical_language() {
    let tree = Tree::new();
    tree.write("klin.json", TYPESCRIPT);
    tree.write("src/index.ts", "export function api() {}\n");
    tree.write("src/App.tsx", "function Component() { return null; }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("src/index.ts:1"), "{}", run.out);
    assert!(run.says("src/App.tsx:1"), "{}", run.out);
}

#[test]
fn an_unsupported_parser_language_is_reported_as_not_measured() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"dead_symbols":{"roots":["src"],"languages":["python"]}}"#,
    );
    tree.write("src/module.py", "def unused():\n    return 1\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("unsupported structural languages were not measured"),
        "{}",
        run.out
    );
    assert!(run.says("1 not measured"), "{}", run.out);
}

#[test]
fn unsupported_structural_files_are_counted_in_gate_json_coverage() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"dead_symbols":{"roots":["src"],"languages":["python"]}}"#,
    );
    tree.write("src/module.py", "def unused():\n    return 1\n");

    let run = tree.run(&["gate", "--json"]);

    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    let gate = report["gates"]
        .as_array()
        .and_then(|gates| gates.iter().find(|gate| gate["name"] == "dead-symbols"))
        .expect("dead-symbols gate");
    assert_eq!(gate["coverage"]["not_measured"].as_u64(), Some(1));
}

#[test]
fn a_grammar_rejection_keeps_the_existing_unparsed_error() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/broken.rs", "fn broken( {\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("grammar could not parse"), "{}", run.out);
}

#[test]
fn report_lists_every_current_dead_symbol_without_the_note_cap() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    let mut source = String::new();
    for number in 0..21 {
        writeln!(&mut source, "fn dead_{number}() {{}}").expect("write fixture");
    }
    tree.write("src/lib.rs", &source);

    let run = tree.run(&["dead-symbols", "--report"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("REPORT: 21 dead symbol(s):"), "{}", run.out);
    assert!(run.says("src/lib.rs:21  dead_20"), "{}", run.out);
}

#[test]
fn an_accepted_dead_symbol_is_held() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"dead_symbols":{"roots":["src"],"languages":["rust"]},"accepted":[{"gate":"dead-symbols","file":"src/lib.rs","text":"fn unused() {}","dead":1}]}"#,
    );
    tree.write("src/lib.rs", "fn unused() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 declaration(s) judged, 1 dead symbol(s)"),
        "{}",
        run.out
    );
}
