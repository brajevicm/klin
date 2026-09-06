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
