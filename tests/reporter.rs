mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::Tree;
use serde_json::Value;

const CONFIG: &str = r#"{
  "escapes": { "in": "src", "except": "src/skipped.rs" },
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

const DOCUMENT: &str = r#"{
  "doc_size": {"README.md": 10}
}"#;

const TANGLED: &str = r##"fn tangled(a: i32) -> i32 {
    if a > 0 && a < 10 {
        for x in 0..a {
            if x == 3 { return 1; }
        }
    } else if a == 0 || a == -1 {
        return 2;
    }
    match a {
        1 => 1,
        9 => 0,
    }
}
"##;

fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree
}

/// A tree whose escapes gate finds two files and measures one, because the config excludes the
/// other, and whose base holds both.
fn two_files() -> Tree {
    let tree = tree(CONFIG);
    tree.write("src/a.rs", "fn f() {}\n");
    tree.write("src/skipped.rs", text::WRAPPED);
    tree.base();
    tree
}

fn json(run: &harness::Run) -> Value {
    match serde_json::from_str(&run.out) {
        Ok(report) => report,
        Err(why) => panic!("{why} — the run printed:\n{}", run.out),
    }
}

fn list<'a>(report: &'a Value, key: &str) -> &'a [Value] {
    match report.get(key).and_then(Value::as_array) {
        Some(records) => records.as_slice(),
        None => panic!("no {key} array in {report}"),
    }
}

fn named<'a>(report: &'a Value, gate: &str) -> &'a Value {
    harness::gate_rows(report)
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row.get("name").and_then(Value::as_str) == Some(gate))
        .unwrap_or_else(|| panic!("no {gate} row in {report}"))
}

/// The id of a run's first finding, which every finding must carry.
fn id(run: &harness::Run) -> String {
    let held = list(&json(run), "findings")[0]
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    assert!(!held.is_empty(), "no finding id in:\n{}", run.out);
    held
}

fn count(row: &Value, key: &str) -> u64 {
    row.get("coverage")
        .and_then(|coverage| coverage.get(key))
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("no coverage {key} in {row}"))
}

#[test]
fn the_ok_line_carries_the_four_coverage_counts() {
    let tree = two_files();

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("(2 file(s) found, 1 measured, 1 excluded, 0 unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn the_runner_prints_the_ok_line_of_every_gate_it_passed() {
    let tree = two_files();

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(
        run.says("OK: 0 escape site(s) in the tree ("),
        "{}",
        run.out
    );
    assert!(run.says("1 measured, 1 excluded"), "{}", run.out);
}

/// The runner prints the run's own context once, however many gates say it for themselves
/// under their own command. Spec 4.3, 11.1.
#[test]
fn the_runner_says_the_window_and_the_derived_values_once() {
    let tree = Tree::new();
    tree.write("src/lib.rs", "fn f() -> i32 {\n    1\n}\n");
    tree.base();

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out.matches("window: branch").count(), 1, "{}", run.out);
    assert_eq!(
        run.out.matches("derived: complexity cc").count(),
        1,
        "{}",
        run.out
    );
}

#[test]
fn the_json_carries_the_coverage_of_every_gate() {
    let tree = two_files();

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = json(&run);
    let row = named(&report, "escapes");
    assert_eq!(row.get("status").and_then(Value::as_str), Some("ok"));
    assert_eq!(count(row, "found"), 2, "{}", run.out);
    assert_eq!(count(row, "measured"), 1, "{}", run.out);
    assert_eq!(count(row, "not_measured"), 0, "{}", run.out);
    assert_eq!(count(row, "excluded"), 1, "{}", run.out);
    assert_eq!(count(row, "unreadable"), 0, "{}", run.out);
    assert_eq!(row.get("findings").and_then(Value::as_u64), Some(0));
    assert_eq!(row.get("notes").and_then(Value::as_u64), Some(0));
}

#[test]
fn a_file_the_grammar_rejected_is_one_the_coverage_calls_unreadable() {
    let tree = tree(CONFIG);
    tree.write("src/a.rs", "fn f() {}\n");
    tree.base();
    tree.write("src/broken.rs", "fn ( { ) unbalanced");

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = json(&run);
    let row = named(&report, "complexity");
    assert_eq!(count(row, "found"), 2, "{}", run.out);
    assert_eq!(count(row, "measured"), 1, "{}", run.out);
    assert_eq!(count(row, "unreadable"), 1, "{}", run.out);
    assert_eq!(row.get("status").and_then(Value::as_str), Some("ok"));
    assert_eq!(row.get("findings").and_then(Value::as_u64), Some(0));
}

#[test]
fn a_new_finding_says_that_nothing_matched() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("nothing matched"), "{}", run.out);
}

#[test]
fn a_worsened_finding_names_the_base_site_it_matched_and_the_ceiling() {
    let tree = tree(CONFIG);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write(
        "src/knot.rs",
        &TANGLED.replace("a == 0 ||", "a == 0 || a == -2 ||"),
    );

    let run = tree.run(&["check", "complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("cc 10, 13 lines, was cc 9, 13 lines"),
        "{}",
        run.out
    );
    assert!(
        run.says("matched the base site at src/knot.rs:1"),
        "{}",
        run.out
    );
    assert!(run.says("ceiling cc 8, lines 60"), "{}", run.out);
}

#[test]
fn a_worsened_finding_names_the_accepted_entry_it_matched() {
    let tree = tree(&format!(
        r#"{{
  "accepted": [{{"gate": "escapes", "file": "src/lib.rs", "text": "{}", "count": 1}}],
  "escapes": {{ "in": "src" }}
}}"#,
        text::ONE_SITE
    ));
    tree.write("src/lib.rs", text::TWO_ON_TWO_LINES);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("matched the accepted entry for src/lib.rs"),
        "{}",
        run.out
    );
}

#[test]
fn a_json_finding_carries_the_matched_site_the_ceiling_and_an_id() {
    let tree = tree(CONFIG);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write(
        "src/knot.rs",
        &TANGLED.replace("a == 0 ||", "a == 0 || a == -2 ||"),
    );

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = json(&run);
    let finding = &list(&report, "findings")[0];
    let matched = finding
        .get("matched")
        .unwrap_or_else(|| panic!("no matched in {finding}"));
    assert_eq!(
        matched.get("file").and_then(Value::as_str),
        Some("src/knot.rs"),
        "{}",
        run.out
    );
    assert_eq!(
        matched.get("values").and_then(|values| values.get("cc")),
        Some(&Value::from(9)),
        "{}",
        run.out
    );
    assert_eq!(
        matched.get("accepted").and_then(Value::as_bool),
        Some(false),
        "{}",
        run.out
    );
    assert_eq!(
        finding.get("ceiling").and_then(Value::as_str),
        Some("cc 8, lines 60"),
        "{}",
        run.out
    );
    assert!(
        !finding
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .is_empty(),
        "{}",
        run.out
    );
}

#[test]
fn a_new_json_finding_carries_a_null_match() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = json(&run);
    let finding = &list(&report, "findings")[0];
    assert_eq!(finding.get("matched"), Some(&Value::Null), "{}", run.out);
    assert_eq!(finding.get("ceiling"), Some(&Value::Null), "{}", run.out);
}

#[test]
fn one_finding_keeps_its_id_across_stops_and_commits() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", text::WRAPPED);

    let first = tree.run(&["check", "--json"]);
    tree.commit("the work so far");
    let second = tree.run(&["check", "--json"]);
    assert_eq!(first.code, 1, "{}", first.out);
    assert_eq!(second.code, 1, "{}", second.out);
    assert_eq!(id(&first), id(&second), "{}", second.out);
}

#[test]
fn the_id_follows_the_path_so_a_rename_changes_it() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", text::WRAPPED);
    let first = tree.run(&["check", "--json"]);
    tree.remove("src/lib.rs");
    tree.write("src/other.rs", text::WRAPPED);
    let second = tree.run(&["check", "--json"]);

    assert_ne!(id(&first), id(&second), "{}", second.out);
}

#[test]
fn a_held_document_names_the_size_the_base_holds_it_at() {
    let tree = tree(DOCUMENT);
    tree.words("README.md", 30);
    tree.base();
    tree.words("README.md", 20);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("over its ceiling of 10, held at the base at 30 words"),
        "{}",
        run.out
    );
}
