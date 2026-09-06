mod harness;

use harness::Tree;

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const EVERY_GATE: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "escapes": { "roots": ["src"], "languages": ["rust"], "baseline": "detent/escapes-baseline.json" },
  "complexity": { "sources": ["src"], "ceilings": {"cc": 8, "lines": 60},
                  "baseline": "detent/complexity-baseline.json" }
}"#;

fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("quality.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
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
        "detent/complexity-baseline.json",
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
        "detent/escapes-baseline.json",
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
        "detent/escapes-baseline.json",
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
fn the_config_flag_names_the_quality_json_every_gate_runs_under() {
    let tree = tree(EVERY_GATE);
    tree.write("elsewhere/quality.json", EVERY_GATE);
    tree.words("elsewhere/README.md", 30);
    tree.write("elsewhere/src/lib.rs", CLEAN);

    let run = tree.run(&["gate", "--config", &tree.at("elsewhere/quality.json")]);
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

fn committed(config: &str) -> Tree {
    let tree = tree(config);
    tree.repository();
    tree.commit("base");
    tree
}

#[test]
fn changed_scopes_the_scoped_gates_to_the_working_tree_and_untracked_files() {
    let tree = committed(EVERY_GATE);
    tree.write("src/old.rs", AN_ESCAPE);
    tree.commit("old debt");
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
    let tree = committed(EVERY_GATE);
    tree.words("README.md", 30);
    tree.commit("a long README");
    tree.write("src/new.rs", CLEAN);

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("README.md is 30 words"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_baseline_entry_for_a_file_outside_the_changed_set_is_not_stale() {
    let tree = committed(EVERY_GATE);
    tree.write(
        "detent/escapes-baseline.json",
        r#"{ "entries": [{"file": "src/gone.rs", "text": "the line that held it", "line": 1,
             "escape": "unwrap", "count": 1}] }"#,
    );
    tree.commit("an accepted escape");
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
    let tree = committed(EVERY_GATE);
    tree.git(&["update-ref", "refs/remotes/origin/release", "HEAD"]);
    tree.write("src/new.rs", AN_ESCAPE);
    tree.commit("work on the branch");

    let in_ci = tree.run_with(&[("GITHUB_BASE_REF", "release")], &["gate", "--changed"]);
    assert_eq!(in_ci.code, 1, "{}", in_ci.out);
    assert!(in_ci.says("src/new.rs:2"), "{}", in_ci.out);

    let locally = tree.run(&["gate", "--changed"]);
    assert_eq!(locally.code, 0, "{}", locally.out);
    assert!(locally.says("changed: 0 file(s)"), "{}", locally.out);
}

#[test]
fn changed_outside_a_repository_is_a_tool_error_rather_than_an_empty_pass() {
    let tree = tree(EVERY_GATE);
    tree.write("src/new.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("--changed needs a git repository"), "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn changed_restricts_complexity_as_well_as_escapes() {
    let tree = committed(EVERY_GATE);
    tree.write("src/old.rs", &tangled("was_here"));
    tree.commit("old debt");
    tree.write("src/new.rs", &tangled("is_new"));

    let scoped = tree.run(&["gate", "--changed"]);
    assert_eq!(scoped.code, 1, "{}", scoped.out);
    assert!(scoped.says("FAIL  complexity"), "{}", scoped.out);
    assert!(scoped.says("src/new.rs:1"), "{}", scoped.out);
    assert!(!scoped.says("src/old.rs"), "{}", scoped.out);
}
