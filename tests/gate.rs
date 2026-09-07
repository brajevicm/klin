mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const EVERY_GATE: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "escapes": { "roots": ["src"], "languages": ["rust"], "baseline": "klin/escapes-baseline.json" },
  "complexity": { "sources": ["src"], "ceilings": {"cc": 8, "lines": 60},
                  "baseline": "klin/complexity-baseline.json" }
}"#;

fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree.write("src/work.rs", CLEAN);
    tree
}

fn at(run: &harness::Run, text: &str) -> usize {
    run.out
        .find(text)
        .unwrap_or_else(|| panic!("{text:?} is absent from:\n{}", run.out))
}

#[test]
fn every_configured_gate_runs_in_ladder_order() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(at(&run, "doc-size") < at(&run, "escapes"), "{}", run.out);
    assert!(at(&run, "escapes") < at(&run, "complexity"), "{}", run.out);
}

#[test]
fn a_gate_the_config_does_not_name_does_not_run() {
    let tree = tree(r#"{ "project": "t", "doc_size": [{"file": "README.md", "ceiling": 10}] }"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("doc-size"), "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(!run.says("complexity"), "{}", run.out);
    assert!(run.says("1 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_status_row_per_gate_and_a_summary_line() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(run.says("3 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_passing_gate_prints_a_row_and_nothing_else() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("OK:"), "{}", run.out);
}

#[test]
fn a_failing_gate_prints_its_full_output_under_its_row() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(
        run.says("README.md is 30 words, over its ceiling of 10"),
        "{}",
        run.out
    );
    assert!(run.says("3 gate(s), 1 failed."), "{}", run.out);
}

#[test]
fn every_gate_runs_even_when_an_earlier_one_failed() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write("src/lib.rs", "fn f() {\n    x.unwrap();\n}\n");

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("FAIL  escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(run.says("3 gate(s), 2 failed."), "{}", run.out);
}

#[test]
fn a_tool_error_is_distinguishable_from_a_gate_failure() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    std::fs::remove_file(tree.path("src/lib.rs")).expect("remove");
    tree.write(
        "klin/complexity-baseline.json",
        r#"{ "entries": "not a list" }"#,
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("ERR   complexity"), "{}", run.out);
    assert!(
        run.says("3 gate(s), 1 failed, 1 tool error."),
        "{}",
        run.out
    );
}

#[test]
fn a_tool_error_alone_exits_two() {
    let tree = tree(EVERY_GATE);
    tree.write(
        "klin/escapes-baseline.json",
        r#"{ "entries": "not a list" }"#,
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(run.says("3 gate(s), 1 tool error."), "{}", run.out);
}

#[test]
fn list_prints_the_configured_gates_and_runs_none_of_them() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "doc-size\nescapes\ncomplexity\n", "{:?}", run.out);
}

#[test]
fn gate_by_name_runs_only_that_gate() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate", "--gate", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
    assert!(run.says("1 gate(s), all passed."), "{}", run.out);
}

#[test]
fn gate_by_name_is_repeatable_and_keeps_ladder_order() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--gate", "complexity", "--gate", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(at(&run, "doc-size") < at(&run, "complexity"), "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(run.says("2 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_gate_name_the_config_does_not_configure_is_a_tool_error() {
    let tree = tree(r#"{ "project": "t", "doc_size": [{"file": "README.md", "ceiling": 10}] }"#);

    let run = tree.run(&["gate", "--gate", "escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no gate named escapes"), "{}", run.out);
    assert!(run.says("doc-size"), "{}", run.out);
}

#[test]
fn a_config_that_configures_no_gate_is_a_tool_error() {
    let tree = tree(r#"{ "project": "t" }"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("configures no gate"), "{}", run.out);
}

#[test]
fn a_section_named_after_the_command_is_a_tool_error() {
    let tree = tree(r#"{ "project": "t", "doc-size": [{"file": "README.md", "ceiling": 1}] }"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"doc-size\" is what the command"), "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
}

#[test]
fn list_says_no_gate_is_configured_rather_than_printing_nothing() {
    let tree = tree(r#"{ "project": "t" }"#);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("configures no gate"), "{}", run.out);
}

#[test]
fn strict_reaches_the_gates_that_take_it() {
    let tree = tree(EVERY_GATE);
    tree.write(
        "klin/escapes-baseline.json",
        r#"{ "entries": [{"file": "src/gone.rs", "text": "x.unwrap();", "line": 1,
             "escape": "unwrap", "count": 1}] }"#,
    );

    let loose = tree.run(&["gate"]);
    assert_eq!(loose.code, 0, "{}", loose.out);
    assert!(loose.says("ok    escapes"), "{}", loose.out);
    assert!(loose.says("matched nothing this run"), "{}", loose.out);

    let strict = tree.run(&["gate", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
    assert!(strict.says("FAIL  escapes"), "{}", strict.out);
}

#[test]
fn the_config_flag_names_the_klin_json_every_gate_runs_under() {
    let tree = tree(EVERY_GATE);
    tree.write("elsewhere/klin.json", EVERY_GATE);
    tree.words("elsewhere/README.md", 30);
    tree.write("elsewhere/src/lib.rs", CLEAN);

    let run = tree.run(&["gate", "--config", &tree.at("elsewhere/klin.json")]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
}

const AN_ESCAPE: &str = include_str!("fixtures/an_escape.rs");

fn tangled(name: &str) -> String {
    let arms: String = (0..12)
        .map(|step| format!("        {step} => n + {step},\n"))
        .collect();
    format!("fn {name}(n: i32) -> i32 {{\n    match n {{\n{arms}        _ => n,\n    }}\n}}\n")
}

fn based(config: &str, files: &[(&str, &str)]) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    for (name, text) in files {
        tree.write(name, text);
    }
    tree.base();
    tree
}

#[test]
fn changed_scopes_the_scoped_gates_to_the_working_tree_and_untracked_files() {
    let tree = based(EVERY_GATE, &[("src/old.rs", AN_ESCAPE)]);
    tree.write("src/new.rs", AN_ESCAPE);

    let scoped = tree.run(&["gate", "--changed"]);
    assert_eq!(scoped.code, 1, "{}", scoped.out);
    assert!(scoped.says("changed: 1 file(s)"), "{}", scoped.out);
    assert!(scoped.says("src/new.rs:2"), "{}", scoped.out);
    assert!(!scoped.says("src/old.rs"), "{}", scoped.out);

    let whole = tree.run(&["gate"]);
    assert_eq!(whole.code, 1, "{}", whole.out);
    assert!(whole.says("src/old.rs:2"), "{}", whole.out);
    assert!(whole.says("src/new.rs:2"), "{}", whole.out);
}

#[test]
fn a_gate_that_is_not_scoped_still_runs_over_everything() {
    let tree = based(EVERY_GATE, &[]);
    tree.words("README.md", 30);
    tree.write("src/new.rs", CLEAN);

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("README.md is 30 words"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_baseline_entry_for_a_file_outside_the_changed_set_is_not_stale() {
    let tree = based(
        EVERY_GATE,
        &[(
            "klin/escapes-baseline.json",
            r#"{ "entries": [{"file": "src/gone.rs", "text": "the line that held it", "line": 1,
             "escape": "unwrap", "count": 1}] }"#,
        )],
    );
    tree.write("src/new.rs", CLEAN);

    let scoped = tree.run(&["gate", "--changed", "--strict"]);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
    assert!(!scoped.says("matched nothing this run"), "{}", scoped.out);

    let whole = tree.run(&["gate"]);
    assert_eq!(whole.code, 0, "{}", whole.out);
    assert!(whole.says("matched nothing this run"), "{}", whole.out);
}

#[test]
fn changed_diffs_against_the_pull_request_base_when_ci_names_one() {
    let tree = based(EVERY_GATE, &[]);
    tree.write("src/new.rs", AN_ESCAPE);
    tree.commit("work on the branch");
    tree.git(&["update-ref", "refs/remotes/origin/release", "HEAD"]);
    tree.write("src/newer.rs", AN_ESCAPE);

    let in_ci = tree.run_with(&[("GITHUB_BASE_REF", "release")], &["gate", "--changed"]);
    assert_eq!(in_ci.code, 1, "{}", in_ci.out);
    assert!(in_ci.says("src/newer.rs:2"), "{}", in_ci.out);
    assert!(!in_ci.says("src/new.rs:2"), "{}", in_ci.out);

    let locally = tree.run(&["gate", "--changed"]);
    assert_eq!(locally.code, 1, "{}", locally.out);
    assert!(locally.says("src/new.rs:2"), "{}", locally.out);
}

#[test]
fn changed_outside_a_repository_is_a_tool_error_rather_than_an_empty_pass() {
    let tree = Tree::new();
    tree.write("klin.json", EVERY_GATE);
    tree.words("README.md", 5);
    tree.write("src/new.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no base commit"), "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn changed_restricts_complexity_as_well_as_escapes() {
    let tree = based(EVERY_GATE, &[("src/old.rs", &tangled("was_here"))]);
    tree.write("src/new.rs", &tangled("is_new"));

    let scoped = tree.run(&["gate", "--changed"]);
    assert_eq!(scoped.code, 1, "{}", scoped.out);
    assert!(scoped.says("FAIL  complexity"), "{}", scoped.out);
    assert!(scoped.says("src/new.rs:1"), "{}", scoped.out);
    assert!(!scoped.says("src/old.rs"), "{}", scoped.out);
}

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;

fn stop(tree: &Tree, event: &str) -> harness::Run {
    harness::feed(tree.root(), &["gate", "--hook"], event)
}

#[test]
fn hook_blocks_the_first_stop_and_hands_the_failures_back() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("a quality gate failed"), "{}", run.out);
    assert!(
        run.says("fix what each names, then stop again"),
        "{}",
        run.out
    );
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(
        run.says("README.md is 30 words, over its ceiling of 10"),
        "{}",
        run.out
    );
}

#[test]
fn hook_does_not_block_the_stop_after_that() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_SECOND_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("still, after one round of fixes"), "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("not blocking a second time"), "{}", run.out);
    assert!(!run.says("then stop again"), "{}", run.out);
}

#[test]
fn hook_says_nothing_when_every_gate_passes() {
    let tree = tree(EVERY_GATE);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn hook_without_an_event_on_stdin_reports_but_does_not_block() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    for event in ["", "not json"] {
        let run = stop(&tree, event);
        assert_eq!(run.code, 1, "{event:?}: {}", run.out);
        assert!(run.says("FAIL  doc-size"), "{event:?}: {}", run.out);
        assert!(!run.says("stop again"), "{event:?}: {}", run.out);
    }
}

#[test]
fn hook_blocks_on_a_tool_error_too() {
    let tree = tree(EVERY_GATE);
    tree.write(
        "klin/escapes-baseline.json",
        r#"{ "entries": "not a list" }"#,
    );

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(run.says("could not run a quality gate"), "{}", run.out);
    assert!(!run.says("a quality gate failed"), "{}", run.out);
    assert!(
        run.says("fix what each names, then stop again"),
        "{}",
        run.out
    );
}

#[test]
fn hook_names_both_when_a_gate_failed_and_another_could_not_run() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write(
        "klin/escapes-baseline.json",
        r#"{ "entries": "not a list" }"#,
    );

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("a quality gate failed, and another could not run"),
        "{}",
        run.out
    );
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("3 gate(s), 1 failed, 1 tool error."),
        "{}",
        run.out
    );
}

#[test]
fn hook_says_a_gate_could_not_run_after_a_second_stop_too() {
    let tree = tree(EVERY_GATE);
    tree.write(
        "klin/escapes-baseline.json",
        r#"{ "entries": "not a list" }"#,
    );

    let run = stop(&tree, A_SECOND_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("could not run a quality gate"), "{}", run.out);
    assert!(run.says("still, after one round of fixes"), "{}", run.out);
}

#[test]
fn hook_without_an_event_reports_a_tool_error_without_blocking_the_stop() {
    let tree = tree(r#"{ "project": "t" }"#);

    let run = stop(&tree, "");
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("configures no gate"), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

const BUILD_BLOCKED: &str = ".klin-build-blocked";

fn wrapper() -> String {
    let at = concat!(env!("CARGO_MANIFEST_DIR"), "/.claude/settings.json");
    std::fs::read_to_string(at).unwrap_or_default()
}

#[test]
fn the_wrapper_writes_the_stamp_the_binary_reads() {
    let settings = wrapper();
    assert!(settings.contains(BUILD_BLOCKED), "{settings}");
}

#[test]
fn the_wrapper_does_not_read_stop_hook_active() {
    let settings = wrapper();
    assert!(!settings.is_empty());
    assert!(!settings.contains("stop_hook_active"), "{settings}");
}

#[test]
fn hook_blocks_the_stop_after_a_build_failure_spent_the_turns_block() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write(BUILD_BLOCKED, "");

    let blocked = stop(&tree, A_SECOND_STOP);
    assert_eq!(blocked.code, 2, "{}", blocked.out);
    assert!(blocked.says("FAIL  doc-size"), "{}", blocked.out);
    assert!(
        blocked.says("fix what each names, then stop again"),
        "{}",
        blocked.out
    );
    assert!(
        !blocked.says("not blocking a second time"),
        "{}",
        blocked.out
    );

    let after = stop(&tree, A_SECOND_STOP);
    assert_eq!(after.code, 0, "{}", after.out);
    assert!(after.says("not blocking a second time"), "{}", after.out);
}

#[test]
fn a_passing_stop_spends_the_stamp_too() {
    let tree = tree(EVERY_GATE);
    tree.write(BUILD_BLOCKED, "");

    let passed = stop(&tree, A_SECOND_STOP);
    assert_eq!(passed.code, 0, "{}", passed.out);
    assert!(!tree.path(BUILD_BLOCKED).exists());

    tree.words("README.md", 30);
    let failed = stop(&tree, A_SECOND_STOP);
    assert_eq!(failed.code, 0, "{}", failed.out);
    assert!(failed.says("not blocking a second time"), "{}", failed.out);
}

#[test]
fn the_stamp_sits_beside_the_config_rather_than_the_working_directory() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write(BUILD_BLOCKED, "");

    let run = harness::feed(&tree.path("src"), &["gate", "--hook"], A_SECOND_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(!tree.path(BUILD_BLOCKED).exists());
}

#[test]
fn a_stamp_an_abandoned_turn_left_changes_nothing_at_the_next_first_stop() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write(BUILD_BLOCKED, "");

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("fix what each names, then stop again"),
        "{}",
        run.out
    );
    assert!(!tree.path(BUILD_BLOCKED).exists());
}

const A_LOOSE_ENTRY: &str = include_str!("fixtures/a_loose_entry.json");

#[test]
fn the_ladder_writes_no_baseline_where_the_config_names_one() {
    let tree = tree(EVERY_GATE);
    tree.write("src/lib.rs", AN_ESCAPE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        !tree.path("klin/escapes-baseline.json").exists(),
        "{}",
        run.out
    );
    assert!(
        !tree.path("klin/complexity-baseline.json").exists(),
        "{}",
        run.out
    );
}

#[test]
fn the_ladder_leaves_a_baseline_looser_than_the_code_byte_identical() {
    let tree = tree(EVERY_GATE);
    tree.write("src/lib.rs", AN_ESCAPE);
    let stored = tree.write("klin/escapes-baseline.json", A_LOOSE_ENTRY);
    tree.words("README.md", 30);

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("looser than the code"), "{}", run.out);
    assert_eq!(
        std::fs::read(&stored).ok(),
        Some(A_LOOSE_ENTRY.as_bytes().to_vec()),
        "{}",
        run.out
    );
}

const A_LOOSE_BASELINE: &str = include_str!("fixtures/a_loose_baseline.json");

fn json(run: &harness::Run) -> Value {
    match serde_json::from_str(&run.out) {
        Ok(report) => report,
        Err(why) => panic!("{why} — the run printed:\n{}", run.out),
    }
}

fn field<'a>(record: &'a Value, key: &str) -> &'a str {
    record.get(key).and_then(Value::as_str).unwrap_or("")
}

fn list<'a>(report: &'a Value, key: &str) -> &'a [Value] {
    match report.get(key).and_then(Value::as_array) {
        Some(records) => records.as_slice(),
        None => panic!("no {key} array in {report}"),
    }
}

fn outcomes(records: &[Value]) -> Vec<(&str, &str)> {
    records
        .iter()
        .map(|record| (field(record, "gate"), field(record, "outcome")))
        .collect()
}

#[test]
fn json_prints_one_object_holding_every_failing_finding() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write("src/lib.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = json(&run);
    assert_eq!(field(&report, "status"), "FAIL", "{}", run.out);
    assert!(
        field(&report, "summary").contains("2 failed"),
        "{}",
        run.out
    );
    let findings = list(&report, "findings");
    assert_eq!(
        outcomes(findings),
        [("doc-size", "new"), ("escapes", "new")],
        "{}",
        run.out
    );
}

#[test]
fn a_json_finding_carries_the_site_the_values_and_the_advice() {
    let tree = tree(EVERY_GATE);
    tree.write("src/lib.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = json(&run);
    let finding = &list(&report, "findings")[0];
    assert_eq!(field(finding, "file"), "src/lib.rs", "{}", run.out);
    assert_eq!(finding.get("line"), Some(&Value::from(2)), "{}", run.out);
    assert!(field(finding, "text").contains("unwrap"), "{}", run.out);
    assert_eq!(
        finding
            .get("values")
            .and_then(|values| values.get("escape")),
        Some(&Value::from("unwrap")),
        "{}",
        run.out
    );
    assert!(
        field(finding, "condition").contains("opts out"),
        "{}",
        run.out
    );
    assert!(
        field(finding, "fix_advice").contains("escape"),
        "{}",
        run.out
    );
}

#[test]
fn a_json_record_names_no_column_and_no_violation() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write("src/lib.rs", AN_ESCAPE);
    tree.write("klin/escapes-baseline.json", A_LOOSE_BASELINE);

    let run = tree.run(&["gate", "--json", "--strict"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = json(&run);
    let banned = ["column", "violation", "invariant"];
    assert!(!list(&report, "findings").is_empty(), "{}", run.out);
    assert!(!list(&report, "notes").is_empty(), "{}", run.out);
    for record in list(&report, "findings")
        .iter()
        .chain(list(&report, "notes"))
    {
        for key in banned {
            assert_eq!(record.get(key), None, "{key} is in {record}");
        }
    }
}

#[test]
fn json_notes_say_why_a_strict_run_failed_with_nothing_over_the_gate() {
    let tree = tree(EVERY_GATE);
    tree.write("src/lib.rs", AN_ESCAPE);
    tree.write("klin/escapes-baseline.json", A_LOOSE_BASELINE);

    let run = tree.run(&["gate", "--json", "--strict"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = json(&run);
    assert_eq!(field(&report, "status"), "FAIL", "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
    assert_eq!(
        outcomes(list(&report, "notes")),
        [("escapes", "improved"), ("escapes", "unmatched")],
        "{}",
        run.out
    );
}

#[test]
fn a_gate_that_could_not_run_is_a_json_finding_too() {
    let tree = tree(EVERY_GATE);
    tree.write(
        "klin/escapes-baseline.json",
        r#"{ "entries": "not a list" }"#,
    );

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = json(&run);
    assert_eq!(field(&report, "status"), "ERROR", "{}", run.out);
    let finding = &list(&report, "findings")[0];
    assert_eq!(field(finding, "gate"), "escapes", "{}", run.out);
    assert_eq!(field(finding, "outcome"), "error", "{}", run.out);
    assert!(
        field(finding, "text").contains("another shape"),
        "{}",
        run.out
    );
}

#[test]
fn a_passing_json_run_holds_no_findings() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = json(&run);
    assert_eq!(field(&report, "status"), "PASS", "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
    assert!(list(&report, "notes").is_empty(), "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_config_the_run_cannot_read_is_a_json_object_too() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--json", "--config", "absent.json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = json(&run);
    assert_eq!(field(&report, "status"), "ERROR", "{}", run.out);
    let finding = &list(&report, "findings")[0];
    assert_eq!(field(finding, "outcome"), "error", "{}", run.out);
    assert!(
        field(finding, "text").contains("could not be read"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_the_grammar_rejected_is_a_json_finding_at_its_own_file() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.rs", "fn ( { ) unbalanced");

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = json(&run);
    assert_eq!(
        outcomes(list(&report, "findings")),
        [("complexity", "unparsed")],
        "{}",
        run.out
    );
    assert_eq!(
        field(&list(&report, "findings")[0], "file"),
        "src/broken.rs",
        "{}",
        run.out
    );
}

const WORKFLOW: &str = include_str!("../.github/workflows/quality.yml");

fn ci_arguments() -> Vec<&'static str> {
    WORKFLOW
        .lines()
        .find(|line| line.contains("klin gate"))
        .unwrap_or_else(|| panic!("no klin gate line in:\n{WORKFLOW}"))
        .split_whitespace()
        .skip_while(|word| *word != "gate")
        .collect()
}

#[test]
fn deleting_a_section_makes_the_ci_invocation_exit_two() {
    let tree = tree(EVERY_GATE);

    let whole = tree.run(&ci_arguments());
    assert_eq!(whole.code, 0, "{}", whole.out);

    tree.write(
        "klin.json",
        r#"{ "project": "t",
             "doc_size": [{"file": "README.md", "ceiling": 10}],
             "complexity": { "sources": ["src"], "ceilings": {"cc": 8, "lines": 60},
                             "baseline": "klin/complexity-baseline.json" } }"#,
    );
    let deleted = tree.run(&ci_arguments());
    assert_eq!(deleted.code, 2, "{}", deleted.out);
    assert!(deleted.says("no gate named escapes"), "{}", deleted.out);
}

const TWO_COMPLEXITY_GATES: &str = r#"{
  "project": "t",
  "gates": [
    {"name": "complexity-src", "check": "complexity",
     "with": {"sources": ["src"], "ceilings": {"cc": 8, "lines": 60},
              "baseline": "klin/src-baseline.json"}},
    {"name": "complexity-tests", "check": "complexity",
     "with": {"sources": ["tests"], "ceilings": {"cc": 8, "lines": 60},
              "baseline": "klin/tests-baseline.json"}}
  ]
}"#;

const AN_EXCLUDED_GATE: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "escapes": false,
  "complexity": { "sources": ["src"], "ceilings": {"cc": 8, "lines": 60},
                  "baseline": "klin/complexity-baseline.json" }
}"#;

const NOTHING_SAID_ABOUT_ESCAPES: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "complexity": false
}"#;

#[test]
fn one_check_backs_two_gates_over_different_sources() {
    let tree = tree(TWO_COMPLEXITY_GATES);
    tree.write("tests/big.rs", &tangled("big"));

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("ok    complexity-src"), "{}", run.out);
    assert!(run.says("FAIL  complexity-tests"), "{}", run.out);
    assert!(run.says("2 gate(s), 1 failed."), "{}", run.out);
}

#[test]
fn a_named_gate_runs_alone_when_the_command_line_names_it() {
    let tree = tree(TWO_COMPLEXITY_GATES);
    tree.write("tests/big.rs", &tangled("big"));

    let run = tree.run(&["gate", "--gate", "complexity-src"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    complexity-src"), "{}", run.out);
    assert!(!run.says("complexity-tests"), "{}", run.out);
    assert!(run.says("1 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_gates_entry_naming_no_check_is_a_tool_error() {
    let tree = tree(
        r#"{ "project": "t",
              "gates": [{"name": "n", "check": "spelling", "with": {}}] }"#,
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no check called \"spelling\""), "{}", run.out);
    assert!(run.says("complexity"), "{}", run.out);
}

#[test]
fn two_gates_of_one_name_are_a_tool_error() {
    let tree = tree(
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "gates": [{"name": "doc-size", "check": "doc-size", "with": []}] }"#,
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("two gates are named doc-size"), "{}", run.out);
}

#[test]
fn a_section_set_to_false_excludes_its_gate_and_the_summary_counts_it() {
    let tree = tree(AN_EXCLUDED_GATE);
    tree.write("src/risky.rs", AN_ESCAPE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(
        run.says("2 gate(s), 1 excluded, all passed."),
        "{}",
        run.out
    );
}

#[test]
fn list_names_the_excluded_gates() {
    let tree = tree(AN_EXCLUDED_GATE);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        run.out, "doc-size\ncomplexity\nescapes — excluded\n",
        "{:?}",
        run.out
    );
}

#[test]
fn list_names_an_available_gate_the_config_does_not_mention() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("escapes — available, not configured"),
        "{}",
        run.out
    );
}

#[test]
fn a_gate_entry_set_off_is_excluded_too() {
    let tree = tree(
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "gates": [{"name": "complexity-tests", "check": "complexity", "off": true}] }"#,
    );

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("complexity-tests — excluded"), "{}", run.out);
}

#[test]
fn naming_an_excluded_gate_is_a_tool_error() {
    let tree = tree(AN_EXCLUDED_GATE);

    let run = tree.run(&["gate", "--gate", "escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("named escapes is excluded"), "{}", run.out);
}

#[test]
fn strict_refuses_a_gate_the_config_neither_configures_nor_excludes() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let loose = tree.run(&["gate"]);
    assert_eq!(loose.code, 0, "{}", loose.out);

    let strict = tree.run(&["gate", "--strict"]);
    assert_eq!(strict.code, 2, "{}", strict.out);
    assert!(
        strict.says("leaves these gates unaccounted for: escapes"),
        "{}",
        strict.out
    );
    assert!(strict.says("configure each one"), "{}", strict.out);
    assert!(strict.says("set its section to false"), "{}", strict.out);
}

#[test]
fn strict_passes_once_the_unaccounted_gate_is_set_to_false() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);
    tree.write(
        "klin.json",
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "complexity": false,
              "escapes": false }"#,
    );

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 gate(s), 2 excluded, all passed."),
        "{}",
        run.out
    );
}

#[test]
fn list_names_the_exclusions_when_every_gate_is_excluded() {
    let tree =
        tree(r#"{ "project": "t", "doc_size": false, "escapes": false, "complexity": false }"#);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        run.out, "doc-size — excluded\nescapes — excluded\ncomplexity — excluded\n",
        "{:?}",
        run.out
    );

    let judged = tree.run(&["gate"]);
    assert_eq!(judged.code, 2, "{}", judged.out);
    assert!(
        judged.says("excludes every gate it names: doc-size, escapes, complexity"),
        "{}",
        judged.out
    );
}
