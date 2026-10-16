//! A change that loosens `klin.json`: `klin check` also judges each changed gate under the
//! base's `klin.json` and names, as a review item, each finding the loosening hides. Spec 16.2.

mod harness;

use harness::Tree;
use serde_json::Value;

const BASE: &str = r#"{"escapes": {"in": "src"}}"#;
const CLEAN: &str = "export const a = 1;\n";
const ESCAPED: &str = "export const b = 1;\nexport const n = (y as any).n;\n";

fn based() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", BASE);
    tree.write("src/a.ts", CLEAN);
    tree.base();
    tree
}

fn loosened(report: &Value) -> Vec<&Value> {
    report["reviews"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|review| review["kind"] == "config-loosened")
        .collect()
}

#[test]
fn turning_escapes_off_names_the_escape_it_hides_and_exits_0() {
    let tree = based();
    tree.write("klin.json", r#"{"escapes": false}"#);
    tree.write("src/b.ts", ESCAPED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["judgement"], "review", "{report}");
    let hidden = loosened(&report);
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["check"], "escapes", "{report}");
    assert_eq!(hidden[0]["file"], "src/b.ts", "{report}");
    assert_eq!(hidden[0]["line"], 2, "{report}");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("REVIEW: klin.json loosens the policy:"),
        "{}",
        run.out
    );
    assert!(
        run.says("escapes: false hides 1 new finding: src/b.ts:2"),
        "{}",
        run.out
    );
    assert!(run.says("as any"), "{}", run.out);
}

#[test]
fn a_scope_that_leaves_the_file_out_names_the_escape_it_hides() {
    let tree = based();
    tree.write(
        "klin.json",
        r#"{"escapes": {"in": "src", "except": "src/b.ts"}}"#,
    );
    tree.write("src/b.ts", ESCAPED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let hidden = loosened(&report);
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "src/b.ts", "{report}");
}

#[test]
fn a_change_to_klin_json_that_hides_nothing_adds_no_review_item() {
    let tree = based();
    tree.write("klin.json", r#"{"escapes": false}"#);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert!(loosened(&report).is_empty(), "{report}");
}

#[test]
fn a_finding_the_loosened_run_still_fails_is_not_named_as_hidden() {
    let tree = based();
    tree.write(
        "klin.json",
        r#"{"escapes": {"in": "src", "except": "src/c.ts"}}"#,
    );
    tree.write("src/b.ts", ESCAPED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    assert!(loosened(&report).is_empty(), "{report}");
}

#[test]
fn an_accepted_entry_the_change_adds_names_the_escape_it_holds() {
    let tree = based();
    tree.write(
        "klin.json",
        r#"{"escapes": {"in": "src"},
            "accepted": [{"gate": "escapes", "file": "src/b.ts",
                          "text": "export const n = (y as any).n;", "count": 1,
                          "reason": "kept for the migration"}]}"#,
    );
    tree.write("src/b.ts", ESCAPED);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the accepted list of escapes hides 1 new finding: src/b.ts:2"),
        "{}",
        run.out
    );
}

#[test]
fn a_first_klin_json_that_turns_escapes_off_names_the_escape_it_hides() {
    let tree = Tree::new();
    tree.write("src/a.ts", CLEAN);
    tree.base();
    tree.write("klin.json", r#"{"escapes": false}"#);
    tree.write("src/b.ts", ESCAPED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let hidden = loosened(&report);
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "src/b.ts", "{report}");
}

#[test]
fn a_run_that_selects_another_gate_or_names_a_config_judges_once() {
    let tree = based();
    tree.write("klin.json", r#"{"escapes": false}"#);
    tree.write("src/b.ts", ESCAPED);

    let selected = tree.run(&["check", "complexity", "--json"]);
    assert_eq!(selected.code, 0, "{}", selected.out);
    let report = selected.json();
    assert!(loosened(&report).is_empty(), "{report}");

    let named = tree.run(&["check", "--config", "klin.json", "--json"]);
    assert_eq!(named.code, 0, "{}", named.out);
    let report = named.json();
    assert!(loosened(&report).is_empty(), "{report}");
}

#[test]
fn a_base_policy_that_cannot_be_judged_says_so_in_a_note() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"escapes": {"in": "src"},
            "sarif": [{"name": "escapes", "report": "out/lint.sarif"}]}"#,
    );
    tree.write("src/a.ts", CLEAN);
    tree.base();
    tree.write("klin.json", r#"{"escapes": false}"#);
    tree.write("src/b.ts", ESCAPED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert!(loosened(&report).is_empty(), "{report}");
    let unjudged: Vec<&Value> = report["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|note| note["kind"] == "config-loosened")
        .collect();
    assert_eq!(unjudged.len(), 1, "{report}");
    let said = unjudged[0]["message"].as_str().unwrap_or_default();
    assert!(said.contains("two gates are named escapes"), "{said}");

    let run = tree.run(&["check"]);
    assert!(
        run.says("NOTE: klin.json changed, and the base's policy could not be judged:"),
        "{}",
        run.out
    );
}

#[test]
fn an_accepted_entry_for_one_of_two_same_text_sites_names_the_site_it_holds() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity": {"in": "src", "cc": 1}}"#);
    tree.write("src/lib.rs", "pub fn a() {}\n");
    tree.base();
    let knot = "    fn knot(a: bool) -> u8 {\n        if a { 1 } else { 0 }\n    }\n";
    tree.write(
        "src/b.rs",
        &format!("pub struct A;\nimpl A {{\n{knot}}}\npub struct B;\nimpl B {{\n{knot}}}\n"),
    );
    tree.write(
        "klin.json",
        r#"{"complexity": {"in": "src", "cc": 1},
            "accepted": [{"gate": "complexity", "file": "src/b.rs",
                          "text": "fn knot(a: bool) -> u8 {", "cc": 2, "lines": 3,
                          "reason": "kept for the migration"}]}"#,
    );

    let run = tree.run(&["check", "complexity", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    let failing: Vec<&Value> = report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|finding| finding["outcome"] != "held")
        .collect();
    assert_eq!(failing.len(), 1, "{report}");
    let hidden = loosened(&report);
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "src/b.rs", "{report}");
    assert_ne!(hidden[0]["line"], failing[0]["line"], "{report}");
}

#[test]
fn a_gate_that_errs_under_the_base_policy_adds_a_note_beside_the_items_of_the_others() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"escapes": {"in": "src"}, "complexity": {"in": "src"},
            "accepted": [{"gate": "complexity", "file": "src/a.ts",
                          "text": "export const a = 1;", "reason": "no values"}]}"#,
    );
    tree.write("src/a.ts", CLEAN);
    tree.base();
    tree.write("klin.json", r#"{"escapes": false}"#);
    tree.write("src/b.ts", ESCAPED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let hidden = loosened(&report);
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "src/b.ts", "{report}");
    let unjudged: Vec<&Value> = report["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|note| note["kind"] == "config-loosened")
        .collect();
    assert_eq!(unjudged.len(), 1, "{report}");
    let said = unjudged[0]["message"].as_str().unwrap_or_default();
    assert!(said.contains("does not give a number"), "{said}");
}

const ONLY_COMPLEXITY: &str = r#""escapes": false, "stubs": false, "secrets": false,
    "dead_symbols": false, "reachability": false, "public_api": false"#;
const VALID: &str = "pub fn a() -> i32 {\n    1\n}\n";
const BROKEN: &str = "pub fn a( -> i32 {{{\n";

fn loosened_of<'a>(report: &'a Value, check: &str) -> Vec<&'a Value> {
    loosened(report)
        .into_iter()
        .filter(|review| review["check"] == check)
        .collect()
}

fn unjudged(report: &Value) -> Vec<&Value> {
    report["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|note| note["kind"] == "config-loosened")
        .collect()
}

#[test]
fn an_accepted_lost_file_the_change_adds_is_named() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity": {"in": "src"}}"#);
    tree.write("src/lib.rs", VALID);
    tree.base();
    tree.write("src/lib.rs", BROKEN);
    tree.write(
        "klin.json",
        r#"{"complexity": {"in": "src"},
            "accepted": [{"gate": "measurement-lost", "file": "src/lib.rs", "reason": "x"}]}"#,
    );

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let hidden = loosened_of(&report, "measurement-lost");
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "src/lib.rs", "{report}");
    let said = hidden[0]["text"].as_str().unwrap_or_default();
    assert!(
        said.contains("the accepted list of measurement-lost"),
        "{said}"
    );
}

#[test]
fn a_scope_that_leaves_out_a_file_the_change_breaks_names_the_lost_file() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        &format!(r#"{{"complexity": {{"in": "src"}}, {ONLY_COMPLEXITY}}}"#),
    );
    tree.write("src/lib.rs", VALID);
    tree.write("src/b.rs", VALID);
    tree.base();
    tree.write("src/b.rs", BROKEN);
    tree.write(
        "klin.json",
        &format!(r#"{{"complexity": {{"in": "src", "except": "src/b.rs"}}, {ONLY_COMPLEXITY}}}"#),
    );

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let hidden = loosened_of(&report, "measurement-lost");
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "src/b.rs", "{report}");
}

#[test]
fn an_integration_that_cannot_measure_under_the_base_policy_adds_a_note() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"sarif": [{"name": "eslint", "report": "missing.sarif"}]}"#,
    );
    tree.write("src/a.ts", CLEAN);
    tree.base();
    tree.write("klin.json", "{}");

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let notes = unjudged(&report);
    assert_eq!(notes.len(), 1, "{report}");
    let said = notes[0]["message"].as_str().unwrap_or_default();
    assert!(said.contains("eslint"), "{said}");
}

#[test]
fn a_local_check_runs_the_command_of_an_integration_the_change_removed() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "run": "touch ran"}]}"#,
    );
    tree.write(".gitignore", "ran\neslint.sarif\n");
    tree.write("src/a.ts", CLEAN);
    tree.base();
    tree.write("klin.json", "{}");

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(tree.path("ran").exists(), "{}", run.out);
}

#[test]
fn the_setup_baseline_never_runs_an_integration_the_change_removed() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "run": "touch ran"}]}"#,
    );
    tree.write(".gitignore", "ran\neslint.sarif\n");
    tree.write("src/a.ts", CLEAN);
    tree.base();
    tree.write("klin.json", "{}");

    let run = tree.run(&["setup"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path("ran").exists(), "{}", run.out);
}

#[test]
fn an_accepted_entry_for_one_of_two_findings_on_one_line_names_the_other() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity": {"in": "src", "cc": 1}}"#);
    tree.write("src/lib.rs", "pub fn a() {}\n");
    tree.base();
    let line = "impl A { fn knot(a: bool) -> u8 { if a { 1 } else { 0 } } \
                fn knot(a: bool) -> u8 { if a { 1 } else { 0 } } }";
    tree.write("src/b.rs", &format!("pub struct A;\n{line}\n"));
    tree.write(
        "klin.json",
        &serde_json::json!({
            "complexity": {"in": "src", "cc": 1},
            "accepted": [{"gate": "complexity", "file": "src/b.rs", "text": line,
                          "cc": 2, "lines": 1, "reason": "kept for the migration"}],
        })
        .to_string(),
    );

    let run = tree.run(&["check", "complexity", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    let hidden = loosened(&report);
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["line"], 2, "{report}");
}

const ESLINT: &str = r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif"}]}"#;
const ONE_RESULT: &str = r#"{"version": "2.1.0", "runs": [{
    "tool": {"driver": {"name": "eslint"}},
    "results": [{"ruleId": "no-any", "message": {"text": "Unexpected any"}, "locations": [
        {"physicalLocation": {"artifactLocation": {"uri": "src/a.ts"},
                              "region": {"startLine": 2}}}]}]
}]}"#;

fn linted() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", ESLINT);
    tree.write("src/a.ts", "one\ntwo\nthree\n");
    tree.base();
    tree.write("src/a.ts", "one\nchanged\nthree\n");
    tree
}

/// The report, written after every change so it is newer than each changed file.
fn reported(tree: &Tree) {
    std::thread::sleep(std::time::Duration::from_millis(20));
    tree.write("eslint.sarif", ONE_RESULT);
}

#[test]
fn renaming_an_integration_that_still_fails_its_result_hides_nothing() {
    let tree = linted();
    tree.write(
        "klin.json",
        r#"{"sarif": [{"name": "lint", "report": "eslint.sarif"}]}"#,
    );
    reported(&tree);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    assert!(loosened(&report).is_empty(), "{report}");
}

#[test]
fn a_run_that_names_the_new_name_of_a_renamed_integration_judges_its_old_entry() {
    let tree = linted();
    tree.write(
        "klin.json",
        r#"{"sarif": [{"name": "lint", "report": "eslint.sarif"}],
            "accepted": [{"gate": "lint", "file": "src/a.ts",
                          "text": "no-any: Unexpected any", "count": 1,
                          "reason": "kept for the migration"}]}"#,
    );
    reported(&tree);

    let run = tree.run(&["check", "lint", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let hidden = loosened(&report);
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "src/a.ts", "{report}");
}
