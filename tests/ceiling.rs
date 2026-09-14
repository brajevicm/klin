mod harness;

use harness::{Run, Tree};

const WIDE: &str = "fn wide() -> i32 {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    \
                    let d = 4;\n    a + b + c + d\n}\n";
const SMALL: &str = "fn small() -> i32 {\n    1\n}\n";

fn tree(ceilings: &str) -> Tree {
    let ceilings: serde_json::Value = serde_json::from_str(ceilings)
        .unwrap_or_else(|why| panic!("the ceilings are not JSON: {why}"));
    let tree = Tree::new();
    tree.write(
        "klin.json",
        &format!(
            r#"{{ "project": "t", "complexity": {{ "in": "src", "cc": {}, "lines": {} }} }}"#,
            ceilings["cc"], ceilings["lines"]
        ),
    );
    tree
}

fn on(tree: &Tree, today: &str, args: &[&str]) -> Run {
    tree.run_with(&[("KLIN_TODAY", today)], args)
}

#[test]
fn a_numeric_ceiling_works_as_before() {
    let tree = tree(r#"{"cc": 8, "lines": 5}"#);
    tree.write("src/wide.rs", WIDE);

    let run = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/wide.rs:1"), "{}", run.out);
    assert!(!run.says("step"), "{}", run.out);
}

#[test]
fn the_lowest_due_step_wins_whatever_the_order_in_the_object() {
    for ceilings in [
        r#"{"cc": 8, "lines": {"2026-01-01": 60, "2027-01-01": 5}}"#,
        r#"{"cc": 8, "lines": {"2027-01-01": 5, "2026-01-01": 60}}"#,
    ] {
        let tree = tree(ceilings);
        tree.write("src/wide.rs", WIDE);

        let run = on(&tree, "2027-06-01", &["complexity"]);
        assert_eq!(run.code, 1, "{}", run.out);
        assert!(run.says("src/wide.rs:1"), "{}", run.out);
    }
}

#[test]
fn a_step_higher_than_an_earlier_one_never_wins() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": 5, "2027-01-01": 60}}"#);
    tree.write("src/wide.rs", WIDE);

    let run = on(&tree, "2027-06-01", &["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/wide.rs:1"), "{}", run.out);
}

#[test]
fn a_step_that_is_not_due_yet_is_not_used() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": 60, "2027-01-01": 5}}"#);
    tree.write("src/wide.rs", WIDE);

    let run = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn the_ok_line_names_the_step_in_force() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": 60, "2027-01-01": 5}}"#);
    tree.write("src/small.rs", SMALL);

    let run = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the 2026-01-01 step"), "{}", run.out);
}

#[test]
fn a_failure_names_the_step_in_force() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": 60, "2027-01-01": 5}}"#);
    tree.write("src/wide.rs", WIDE);

    let run = on(&tree, "2027-06-01", &["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("the 2027-01-01 step"), "{}", run.out);
}

#[test]
fn a_schedule_with_no_step_due_is_a_config_error_naming_the_key() {
    let tree = tree(r#"{"cc": 8, "lines": {"2027-01-01": 5}}"#);
    tree.write("src/small.rs", SMALL);

    let run = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"complexity\" \"lines\""), "{}", run.out);
    assert!(run.says("2026-06-01"), "{}", run.out);
}

#[test]
fn an_empty_schedule_is_a_config_error_naming_the_key() {
    let tree = tree(r#"{"cc": 8, "lines": {}}"#);
    tree.write("src/small.rs", SMALL);

    let run = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"complexity\" \"lines\""), "{}", run.out);
}

#[test]
fn a_malformed_date_is_a_config_error_naming_the_key_and_the_value() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-9-8": 60}}"#);
    tree.write("src/small.rs", SMALL);

    let run = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"complexity\" \"lines\""), "{}", run.out);
    assert!(run.says("2026-9-8"), "{}", run.out);
    assert!(run.says("YYYY-MM-DD"), "{}", run.out);
}

#[test]
fn a_step_that_is_not_a_number_is_a_config_error_naming_the_key() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": "60"}}"#);
    tree.write("src/small.rs", SMALL);

    let run = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"complexity\" \"lines\""), "{}", run.out);
    assert!(run.says("whole number"), "{}", run.out);
}

#[test]
fn a_malformed_klin_today_is_an_error_naming_the_variable() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": 60}}"#);
    tree.write("src/small.rs", SMALL);

    let run = on(&tree, "the-ninth", &["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("KLIN_TODAY"), "{}", run.out);
}

#[test]
fn a_lower_step_holds_the_complexity_the_base_holds_and_fails_new_code() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": 5}}"#);
    tree.write("src/wide.rs", WIDE);
    tree.base();

    let held = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(held.code, 0, "{}", held.out);

    tree.write("src/added.rs", WIDE);
    let added = on(&tree, "2026-06-01", &["complexity"]);
    assert_eq!(added.code, 1, "{}", added.out);
    assert!(added.says("src/added.rs:1"), "{}", added.out);
    assert!(!added.says("src/wide.rs"), "{}", added.out);
}

#[test]
fn a_dated_ceiling_on_a_document_names_the_step_and_fails_a_new_document() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "README.md", "ceiling": {"2026-01-01": 10}}]}"#,
    );
    tree.words("README.md", 30);

    let run = on(&tree, "2026-06-01", &["doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("over its ceiling of 10"), "{}", run.out);
    assert!(run.says("the 2026-01-01 step"), "{}", run.out);
}

#[test]
fn a_lower_step_holds_the_document_the_base_holds_and_fails_one_that_grew() {
    let tree = Tree::bare();
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "README.md", "ceiling": {"2026-01-01": 10}}]}"#,
    );
    tree.words("README.md", 30);
    tree.base();

    let held = on(&tree, "2026-06-01", &["doc-size"]);
    assert_eq!(held.code, 0, "{}", held.out);
    assert!(held.says("held at the base"), "{}", held.out);

    tree.words("README.md", 31);
    let grew = on(&tree, "2026-06-01", &["doc-size"]);
    assert_eq!(grew.code, 1, "{}", grew.out);
}

#[test]
fn a_document_the_base_lacks_is_new_debt_under_a_dated_ceiling() {
    let tree = Tree::bare();
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "README.md", "ceiling": {"2026-01-01": 10}}]}"#,
    );
    tree.words("README.md", 5);
    tree.base();

    tree.words("README.md", 30);
    let run = on(&tree, "2026-06-01", &["doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
}

#[test]
fn a_document_over_its_ceiling_outside_a_repository_still_fails() {
    let tree = Tree::bare();
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "README.md", "ceiling": {"2026-01-01": 10}}]}"#,
    );
    tree.words("README.md", 30);

    let run = on(&tree, "2026-06-01", &["doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("over its ceiling of 10"), "{}", run.out);
}

#[test]
fn the_gate_runner_reads_a_dated_ceiling_too() {
    let tree = tree(r#"{"cc": 8, "lines": {"2026-01-01": 5}}"#);
    tree.write("src/wide.rs", WIDE);

    let run = on(&tree, "2026-06-01", &["gate"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  complexity"), "{}", run.out);
}
