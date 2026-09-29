mod harness;

use std::fmt::Write;

use harness::Tree;

const RUST: &str = r#"{"dead_symbols":{"in":"src"}}"#;
const TYPESCRIPT: &str = RUST;

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
fn a_reference_in_another_language_does_not_keep_a_rust_declaration_alive() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "fn helper() {}\n");
    tree.write("web/caller.ts", "export function caller() { helper(); }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:1"), "{}", run.out);
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
fn references_lost_from_two_files_name_the_first_file_in_path_order() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/zed.rs", "fn helper() {}\nfn spare() {}\n");
    tree.write("src/lib.rs", "fn helper() {}\n");
    tree.write("src/b_caller.rs", "fn main() { helper(); spare(); }\n");
    tree.write("src/a_caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.write("src/b_caller.rs", "fn main() { spare(); }\n");
    tree.write("src/a_caller.rs", "pub fn call() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 dead symbol(s) got worse"), "{}", run.out);
    assert_eq!(
        run.out.matches("lost reference in src/a_caller.rs").count(),
        2,
        "{}",
        run.out
    );
    assert!(
        !run.says("lost reference in src/b_caller.rs"),
        "{}",
        run.out
    );
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
        r#"{"dead_symbols":{"in":"src","ignore":["generated_*"]}}"#,
    );
    tree.write("src/lib.rs", "fn generated_helper() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn a_private_function_only_a_serde_default_names_passes() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "#[derive(Deserialize)]\npub struct Settings {\n    #[serde(default = \"default_cell_zoom\")]\n    pub zoom: u8,\n}\n\nfn default_cell_zoom() -> u8 {\n    3\n}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn a_private_function_only_a_serde_skip_serializing_if_names_passes() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "#[derive(Deserialize, Serialize)]\npub struct Terrain {\n    #[serde(default = \"real_height\", skip_serializing_if = \"is_real_height\")]\n    pub height: u8,\n}\n\nfn real_height() -> u8 {\n    3\n}\n\nfn is_real_height(height: &u8) -> bool {\n    *height == 3\n}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn a_private_function_no_serde_key_names_still_fails() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "#[derive(Deserialize)]\npub struct Settings {\n    #[serde(rename = \"zoom_level\", default)]\n    pub zoom: u8,\n}\n\nfn zoom_level() -> u8 {\n    3\n}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new dead symbol(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:7"), "{}", run.out);
}

#[test]
fn a_serde_attribute_inside_cfg_attr_names_its_function_too() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "pub struct Settings {\n    #[cfg_attr(feature = \"serde\", serde(default = \"default_cell_zoom\"))]\n    pub zoom: u8,\n}\n\nfn default_cell_zoom() -> u8 {\n    3\n}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn a_serde_path_references_only_its_last_segment() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "#[derive(Deserialize)]\npub struct Settings {\n    #[serde(default = \"presets::default_cell_zoom\")]\n    pub zoom: u8,\n}\n\nfn default_cell_zoom() -> u8 {\n    3\n}\n\nfn presets() {}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new dead symbol(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:11"), "{}", run.out);
}

#[test]
fn a_serde_with_module_names_no_function() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "#[derive(Deserialize)]\npub struct Settings {\n    #[serde(with = \"zoom_format\")]\n    pub zoom: u8,\n}\n\nfn zoom_format() {}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:7"), "{}", run.out);
}

#[test]
fn serde_tokens_inside_a_macro_call_name_no_function() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "pub fn settings() {\n    some_macro!(serde(default = \"default_cell_zoom\"));\n}\n\nfn default_cell_zoom() -> u8 {\n    3\n}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:5"), "{}", run.out);
}

#[test]
fn a_raw_string_serde_path_names_its_function() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/lib.rs",
        "#[derive(Deserialize)]\npub struct Settings {\n    #[serde(default = r#\"default_cell_zoom\"#)]\n    pub zoom: u8,\n}\n\nfn default_cell_zoom() -> u8 {\n    3\n}\n",
    );

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 dead symbol(s)"), "{}", run.out);
}

#[test]
fn a_new_private_typescript_function_fails() {
    let tree = Tree::new();
    tree.write("klin.json", TYPESCRIPT);
    tree.write("src/App.tsx", "function Component() { return null; }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new dead symbol(s)"), "{}", run.out);
    assert!(run.says("src/App.tsx:1"), "{}", run.out);
}

#[test]
fn a_typescript_function_that_loses_its_last_reference_fails_as_worsened() {
    let tree = Tree::new();
    tree.write("klin.json", TYPESCRIPT);
    tree.write("src/lib.ts", "function helper() {}\n");
    tree.write("src/caller.ts", "export function caller() { helper(); }\n");
    tree.base();
    tree.write("src/caller.ts", "export function caller() {}\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("lost reference in src/caller.ts"), "{}", run.out);
}

#[test]
fn a_dead_typescript_symbol_already_in_the_base_is_held() {
    let tree = Tree::new();
    tree.write("klin.json", TYPESCRIPT);
    tree.write("src/lib.ts", "function old_debt() {}\n");
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
fn a_retired_language_selector_is_rejected() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"dead_symbols":{"languages":["tsx"]}}"#);
    tree.write("src/App.tsx", "function Component() { return null; }\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"languages\""), "{}", run.out);
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
fn an_explicit_scope_with_only_unsupported_files_is_a_configuration_error() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"dead_symbols":{"in":"src"}}"#);
    tree.write("src/module.py", "def unused():\n    return 1\n");

    let run = tree.run(&["dead-symbols"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("has an \"in\" scope with no applicable file"),
        "{}",
        run.out
    );
}

#[test]
fn unsupported_structural_files_are_outside_dead_symbol_coverage() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{}"#);
    tree.write("src/module.py", "def unused():\n    return 1\n");

    let run = tree.run(&["gate", "--json"]);

    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let gate = report["gates"]
        .as_array()
        .and_then(|gates| gates.iter().find(|gate| gate["name"] == "dead-symbols"))
        .unwrap_or_else(|| panic!("dead-symbols gate: {report}"));
    assert_eq!(gate["coverage"]["not_measured"].as_u64(), Some(0));
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
        assert!(writeln!(&mut source, "fn dead_{number}() {{}}").is_ok());
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
        r#"{"dead_symbols":{"in":"src"},"accepted":[{"gate":"dead-symbols","file":"src/lib.rs","text":"fn unused() {}","dead":1}]}"#,
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

/// A changed run's judgement state, and the whole run's, for a base of `unchanged` files that
/// each declare `declarations` referenced functions, with one more file the working tree edits.
fn states(unchanged: usize, declarations: usize) -> (u64, u64) {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    for file in 0..unchanged {
        let mut source = String::new();
        for at in 0..declarations {
            assert!(writeln!(&mut source, "fn held_{file}_{at}() {{}}").is_ok());
            assert!(
                writeln!(
                    &mut source,
                    "fn call_{file}_{at}() {{ held_{file}_{at}(); }}"
                )
                .is_ok()
            );
        }
        tree.write(&format!("src/held_{file}.rs"), &source);
    }
    tree.write(
        "src/edited.rs",
        "fn edited() {}\nfn call_edited() { edited(); }\n",
    );
    tree.base();
    tree.write(
        "src/edited.rs",
        "fn edited() {}\nfn call_edited() { edited(); }\nfn also() { edited(); }\n",
    );

    let built = |flags: &[&str]| {
        let mut args = vec!["gate", "--json", "--gate", "dead-symbols"];
        args.extend_from_slice(flags);
        let report = tree.run(&args).json();
        report["gates"][0]["facts"]["states"]
            .as_u64()
            .unwrap_or_else(|| panic!("no dead-symbols states in {report}"))
    };
    (built(&["--changed"]), built(&[]))
}

#[test]
fn a_changed_run_builds_no_state_for_the_declarations_it_does_not_judge() {
    let (few, few_whole) = states(4, 1);
    let (many, many_whole) = states(4, 5);

    assert_eq!(few, many, "a changed run built state outside its scope");
    assert_eq!(few, 5, "the changed file's own declarations are judged");
    assert!(many_whole > few_whole, "{many_whole} then {few_whole}");
}

/// A changed run of `dead-symbols` alone, which is what the Stop hook scopes.
fn changed(tree: &Tree) -> harness::Run {
    tree.run(&["gate", "--changed", "--gate", "dead-symbols"])
}

#[test]
fn removing_the_last_reference_in_a_changed_caller_worsens_an_unchanged_declaration() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.write("src/caller.rs", "pub fn call() {}\n");

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("src/service.rs:1"), "{}", run.out);
    assert!(run.says("lost reference in src/caller.rs"), "{}", run.out);
}

#[test]
fn removing_a_serde_attribute_in_a_changed_file_worsens_an_unchanged_helper() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write(
        "src/service.rs",
        "fn default_cell_zoom() -> u8 {\n    3\n}\n",
    );
    tree.write(
        "src/settings.rs",
        "pub struct Settings {\n    #[serde(default = \"default_cell_zoom\")]\n    pub zoom: u8,\n}\n",
    );
    tree.base();
    tree.write(
        "src/settings.rs",
        "pub struct Settings {\n    pub zoom: u8,\n}\n",
    );

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("src/service.rs:1"), "{}", run.out);
    assert!(run.says("lost reference in src/settings.rs"), "{}", run.out);
}

#[test]
fn a_changed_typescript_caller_worsens_an_unchanged_declaration_the_same_way() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.ts", "function helper() {}\n");
    tree.write("src/caller.ts", "export function call() { helper(); }\n");
    tree.base();
    tree.write("src/caller.ts", "export function call() {}\n");

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/service.ts:1"), "{}", run.out);
}

#[test]
fn a_reference_another_unchanged_caller_still_holds_is_no_regression() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.write("src/other.rs", "pub fn other() { helper(); }\n");
    tree.base();
    tree.write("src/caller.rs", "pub fn call() {}\n");

    let run = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_changed_caller_that_references_a_dead_declaration_is_an_improvement() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() {}\n");
    tree.base();
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");

    let run = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn deleting_the_only_caller_worsens_the_unchanged_declaration() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.remove("src/caller.rs");

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/service.rs:1"), "{}", run.out);
}

#[test]
fn every_declaration_of_an_affected_name_stays_conservatively_in_scope() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/twin.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.write("src/caller.rs", "pub fn call() {}\n");

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/service.rs:1"), "{}", run.out);
    assert!(run.says("src/twin.rs:1"), "{}", run.out);
}

/// Under the conservative name rule a declaration is dead only while no reference names it
/// outside itself, so the one reachable shape of base-held debt under an affected name is the
/// turn that adds the reference back. It must read as an improvement, never as a new finding.
#[test]
fn base_dead_debt_an_affected_name_reaches_is_never_new() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/debt.rs", "fn helper() {}\nfn stale() {}\n");
    tree.write("src/caller.rs", "pub fn call() {}\n");
    tree.base();
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");

    let run = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("got worse"), "{}", run.out);
}

#[test]
fn unrelated_historical_debt_outside_the_changed_scope_stays_silent() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    let mut debt = String::new();
    for number in 0..40 {
        assert!(writeln!(&mut debt, "fn stale_{number}() {{}}").is_ok());
    }
    tree.write("src/debt.rs", &debt);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.write(
        "src/caller.rs",
        "pub fn call() { helper(); }\npub fn more() {}\n",
    );

    let run = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/debt.rs"), "{}", run.out);
}

#[test]
fn a_changed_caller_the_grammar_cannot_read_guesses_no_deadness() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.write("src/caller.rs", "pub fn call( { helper(\n");

    let run = changed(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("the grammar could not parse"), "{}", run.out);
    assert!(!run.says("src/service.rs:1"), "{}", run.out);
}

#[test]
fn a_renamed_caller_that_keeps_its_reference_is_no_regression() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.remove("src/caller.rs");
    tree.write("src/renamed.rs", "pub fn call() { helper(); }\n");

    let run = changed(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_renamed_caller_that_drops_its_reference_worsens_the_declaration() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.remove("src/caller.rs");
    tree.write("src/renamed.ts", "export function call() {}\n");

    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/service.rs:1"), "{}", run.out);
}

#[test]
fn unrelated_declarations_do_not_enter_a_changed_runs_judgement_state() {
    let (few, _) = states(4, 1);
    let (many, _) = states(4, 5);

    assert_eq!(few, many, "an unaffected name grew the changed run's state");
}

#[test]
fn a_changed_file_that_keeps_its_reference_names_widens_nothing() {
    let tree = Tree::new();
    tree.write("klin.json", RUST);
    tree.write("src/service.rs", "fn helper() {}\nfn spare() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); spare(); }\n");
    tree.base();
    tree.write(
        "src/caller.rs",
        "pub fn call() { helper(); spare(); }\n// a comment\n",
    );

    let report = tree
        .run(&["gate", "--json", "--changed", "--gate", "dead-symbols"])
        .json();

    assert_eq!(
        report["gates"][0]["facts"]["states"], 0,
        "the unchanged declarations were judged: {report}"
    );
}
