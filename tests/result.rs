//! The result model of `klin check`: the three axes, the exit codes and the check document.
//! Spec 7, 11.3, 11.7.

mod harness;

use harness::Tree;
use serde_json::{Value, json};

const CLEAN: &str = "pub fn one(a: i32) -> i32 {\n    a + 1\n}\n";

/// A tree whose base holds a clean source file and a short document, under `config`.
fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

/// A tree with a document and no source, so no check that reads code applies.
fn without_source(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.base();
    tree
}

fn checked(tree: &Tree, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["check", "--json"];
    all.extend_from_slice(args);
    let run = tree.run(&all);
    (run.code, run.json())
}

fn row<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no row {name} in {report}"))
}

fn run_holes(report: &Value) -> Vec<String> {
    report["measurements"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|record| record["check"].is_null())
        .flat_map(|record| record["holes"].as_array().cloned().unwrap_or_default())
        .filter_map(|hole| hole["reason"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_clean_run_exits_0_with_pass_and_complete() {
    let tree = tree("{}");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert_eq!(report["judgement"], "pass", "{report}");
    assert_eq!(report["measurement"], "complete", "{report}");
    assert_eq!(report["execution"], "ok", "{report}");
    assert_eq!(report["exit"], 0, "{report}");
}

#[test]
fn the_text_ends_with_the_three_axes_and_the_exit_code() {
    let tree = tree("{}");

    let run = tree.run(&["check"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn the_document_carries_its_schema_version_and_command() {
    let tree = tree("{}");

    let (_, report) = checked(&tree, &[]);

    assert_eq!(report["schema_version"], 1, "{report}");
    assert_eq!(report["command"], "check", "{report}");
    assert_eq!(
        report["klin"]["version"],
        env!("CARGO_PKG_VERSION"),
        "{report}"
    );
    assert_eq!(report["config"]["present"], true, "{report}");
}

#[test]
fn a_failing_run_exits_1() {
    let tree = tree(r#"{"doc_size": {"README.md": 10}}"#);
    tree.words("README.md", 30);

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(report["judgement"], "fail", "{report}");
    assert_eq!(row(&report, "doc-size")["judgement"], "fail", "{report}");
    let finding = &report["findings"][0];
    for key in [
        "id", "check", "kind", "outcome", "file", "line", "text", "values", "ceiling", "matched",
        "remedy",
    ] {
        assert!(
            finding
                .as_object()
                .is_some_and(|fields| fields.contains_key(key)),
            "{key} missing: {report}"
        );
    }
    assert_eq!(finding["check"], "doc-size", "{report}");
    assert_eq!(finding["kind"], "metric", "{report}");
    assert_eq!(finding["outcome"], "new", "{report}");
    assert_eq!(finding["file"], "README.md", "{report}");
    assert!(finding["line"].is_null(), "{report}");
    assert!(finding["text"].is_null(), "{report}");
    assert_eq!(
        finding["values"],
        json!({"words": 30, "ceiling": 10}),
        "{report}"
    );
    assert_eq!(finding["ceiling"], json!({"words": 10}), "{report}");
    assert!(finding["matched"].is_null(), "{report}");
    assert!(
        finding["remedy"]
            .as_str()
            .is_some_and(|remedy| !remedy.is_empty()),
        "{report}"
    );
    let id = finding["id"].as_str().unwrap_or_default();
    assert!(
        id.len() == 16 && id.chars().all(|c| c.is_ascii_hexdigit()),
        "{report}"
    );
    let (_, again) = checked(&tree, &[]);
    assert_eq!(again["findings"][0]["id"], finding["id"], "{report}");
}

#[test]
fn an_invalid_configuration_exits_2_with_no_judgement_and_no_measurement() {
    let tree = tree(r#"{"not_a_section": 1}"#);

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 2, "{report}");
    assert_eq!(report["execution"], "error", "{report}");
    assert!(report["judgement"].is_null(), "{report}");
    assert!(report["measurement"].is_null(), "{report}");
    assert_eq!(report["errors"][0]["kind"], "configuration", "{report}");
    assert!(report["errors"][0]["check"].is_null(), "{report}");
    assert_eq!(report["capabilities"], Value::Array(Vec::new()), "{report}");
    assert_eq!(report["config"]["present"], true, "{report}");
    assert!(
        report["config"]["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("klin.json")),
        "{report}"
    );
}

#[test]
fn a_config_path_that_names_no_file_is_an_invocation_error_and_not_present() {
    let tree = tree("{}");

    let (code, report) = checked(&tree, &["--config", "absent.json"]);

    assert_eq!(code, 2, "{report}");
    assert_eq!(report["errors"][0]["kind"], "invocation", "{report}");
    assert_eq!(report["config"]["present"], false, "{report}");
}

#[test]
fn an_invalid_configuration_says_err_and_the_axes_in_the_text() {
    let tree = tree(r#"{"not_a_section": 1}"#);

    let run = tree.run(&["check"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("ERR: "), "{}", run.out);
    assert!(
        run.says("judgement: none, measurement: none, execution: error, exit 2"),
        "{}",
        run.out
    );
}

#[test]
fn a_capability_configuration_error_exits_2_and_the_other_capabilities_still_report() {
    let tree = tree(r#"{"doc_size": {"gone.md": 10}}"#);

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 2, "{report}");
    assert_eq!(report["execution"], "error", "{report}");
    assert_eq!(report["judgement"], "pass", "{report}");
    assert_eq!(row(&report, "doc-size")["execution"], "error", "{report}");
    assert_eq!(row(&report, "escapes")["execution"], "ok", "{report}");
    assert_eq!(row(&report, "escapes")["judgement"], "pass", "{report}");
    assert_eq!(report["errors"][0]["kind"], "configuration", "{report}");
    assert_eq!(report["errors"][0]["check"], "doc-size", "{report}");
}

#[test]
fn a_capability_error_prints_an_err_row_and_line() {
    let tree = tree(r#"{"doc_size": {"gone.md": 10}}"#);

    let run = tree.run(&["check"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("ERR   doc-size"), "{}", run.out);
    assert!(run.says("ERR: "), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_selector_that_names_an_unknown_gate_exits_2_as_an_invocation_error() {
    let tree = tree("{}");

    let (code, report) = checked(&tree, &["no-such-check"]);

    assert_eq!(code, 2, "{report}");
    assert_eq!(report["errors"][0]["kind"], "invocation", "{report}");
    assert!(report["judgement"].is_null(), "{report}");
}

#[test]
fn a_selector_that_names_an_excluded_gate_exits_2() {
    let tree = tree(r#"{"escapes": false}"#);

    let run = tree.run(&["check", "escapes"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("is excluded"), "{}", run.out);
}

#[test]
fn a_selector_for_a_capability_that_does_not_apply_is_an_unsupported_hole() {
    let tree = without_source("{}");

    let (code, report) = checked(&tree, &["escapes"]);

    assert_eq!(code, 3, "{report}");
    assert_eq!(report["measurement"], "incomplete", "{report}");
    assert_eq!(
        row(&report, "escapes")["state"],
        "not-applicable",
        "{report}"
    );
    let gate = report["measurements"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|record| record["check"] == "escapes")
        .cloned()
        .unwrap_or_default();
    assert_eq!(gate["holes"][0]["reason"], "unsupported", "{report}");
}

#[test]
fn a_documentation_only_tree_whose_documents_were_measured_is_complete() {
    let tree = without_source("{}");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert_eq!(report["measurement"], "complete", "{report}");
    assert!(run_holes(&report).is_empty(), "{report}");
    assert_eq!(
        row(&report, "complexity")["state"],
        "not-applicable",
        "{report}"
    );
    assert_eq!(row(&report, "doc-citations")["state"], "active", "{report}");
}

#[test]
fn source_the_base_held_and_the_tree_lost_does_not_pass_on_the_documents_alone() {
    let tree = tree("{}");
    tree.remove("src/lib.rs");
    assert!(std::fs::remove_dir(tree.path("src")).is_ok());

    let (code, report) = checked(&tree, &[]);

    assert_ne!(code, 0, "{report}");
    assert_eq!(report["measurement"], "incomplete", "{report}");
    assert_eq!(run_holes(&report), ["nothing-measured"], "{report}");
}

#[test]
fn a_file_that_is_not_source_left_where_the_source_went_does_not_let_the_run_pass() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("README.md", 5);
    tree.write("tools/run.py", "def main():\n    return 1\n");
    tree.base();
    tree.remove("tools/run.py");
    tree.write("tools/notes.md", "kept\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 3, "{report}");
    assert_eq!(run_holes(&report), ["nothing-measured"], "{report}");
}

#[test]
fn a_whole_run_where_no_capability_applies_is_a_nothing_measured_hole() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("notes.txt", "nothing klin measures");
    tree.base();

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 3, "{report}");
    assert_eq!(run_holes(&report), ["nothing-measured"], "{report}");
}

#[test]
fn a_changed_run_is_never_a_nothing_measured_hole() {
    let tree = without_source("{}");
    tree.words("README.md", 6);

    let (code, report) = checked(&tree, &["--changed"]);

    assert_eq!(code, 0, "{report}");
    assert!(run_holes(&report).is_empty(), "{report}");
}

/// #507: a check whose section pins its own `in` measures what the pin names, so its own
/// configuration error shows rather than hiding behind the hole.
#[test]
fn a_check_whose_section_pins_in_does_not_count_toward_nothing_measured() {
    let tree = without_source(r#"{"escapes": {"in": "lib"}}"#);

    let (code, report) = checked(&tree, &["escapes"]);

    assert!(run_holes(&report).is_empty(), "{report}");
    assert_eq!(row(&report, "escapes")["state"], "active", "{report}");
    assert_ne!(code, 3, "{report}");
}

#[test]
fn a_changed_run_over_a_change_with_no_measurable_file_exits_0() {
    let tree = tree("{}");
    tree.write("notes.txt", "a note\n");

    let run = tree.run(&["check", "--changed"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn the_timing_and_cost_fields_sit_under_diagnostics() {
    let tree = tree("{}");

    let (_, report) = checked(&tree, &[]);

    let gates = report["diagnostics"]["gates"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        gates
            .iter()
            .any(|gate| gate["name"] == "complexity" && gate["ms"].is_u64()),
        "{report}"
    );
    assert!(row(&report, "complexity").get("ms").is_none(), "{report}");
}

#[test]
fn each_row_names_its_kind_and_placement_and_each_record_its_semantics_version() {
    let tree = tree("{}");

    let (_, report) = checked(&tree, &[]);

    let escapes = row(&report, "escapes");
    assert_eq!(escapes["kind"], "check", "{report}");
    assert_eq!(
        escapes["placement"],
        serde_json::json!(["stop", "check"]),
        "{report}"
    );
    assert_eq!(escapes["coverage_claim"], "verified", "{report}");
    let record = report["measurements"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|record| record["check"] == "escapes")
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        record["basis"]["producer"]["semantics_version"], 1,
        "{report}"
    );
}

#[test]
fn policy_prints_each_capability_s_placement() {
    let tree = tree(r#"{"sarif": [{"name": "scan", "report": "scan.sarif"}]}"#);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("escapes — runs\n      placement: stop, check"),
        "{}",
        run.out
    );
    assert!(
        run.says("scan — runs\n      placement: check"),
        "{}",
        run.out
    );

    let json = tree.run(&["policy", "--json"]).json();
    let scan = json["capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["name"] == "scan")
        .cloned()
        .unwrap_or_default();
    assert_eq!(scan["placement"], serde_json::json!(["check"]), "{json}");
}

/// A base tree klin cannot lay out fails only the gates that read it. A temporary directory that
/// does not exist is the failure here. Spec 7.3.
#[test]
fn a_base_tree_klin_cannot_lay_out_is_a_git_error_of_its_gates_and_the_others_still_report() {
    let tree = tree(r#"{"doc_size": {"README.md": 10}}"#);
    tree.words("README.md", 30);

    let run = tree.run_with(
        &[("TMPDIR", "/nonexistent/klin-tmp")],
        &["check", "--json", "doc-size", "escapes"],
    );
    let report = run.json();

    assert_eq!(run.code, 2, "{report}");
    assert_eq!(report["judgement"], "fail", "{report}");
    assert_eq!(row(&report, "doc-size")["judgement"], "fail", "{report}");
    assert_eq!(row(&report, "escapes")["execution"], "error", "{report}");
    assert_eq!(report["errors"][0]["kind"], "git", "{report}");
    assert_eq!(report["errors"][0]["check"], "escapes", "{report}");
}

#[test]
fn source_the_base_held_in_a_directory_the_tree_still_has_does_not_pass_either() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("README.md", 5);
    tree.write("tools/run.py", "def main():\n    return 1\n");
    tree.base();
    tree.remove("tools/run.py");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 3, "{report}");
    assert_eq!(run_holes(&report), ["nothing-measured"], "{report}");
}
