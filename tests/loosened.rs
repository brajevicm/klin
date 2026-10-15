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
