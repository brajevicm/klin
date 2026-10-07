//! The green line's qualifier: why the ratchet passed the findings a run judged. The check owns
//! the state it measured, and the ratchet owns the reason, so a run that judged nothing claims
//! nothing. Spec 8.6.

mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::Tree;

const CONFIG: &str = r#"{
  "escapes": { "in": "src" }
}"#;

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree
}

fn accepted(entries: &str) -> String {
    format!(
        r#"{{ "accepted": [{entries}],
             "escapes": {{ "in": "src" }} }}"#
    )
}

fn entry(file: &str, count: u64) -> String {
    format!(
        r#"{{"gate": "escapes", "file": "{file}", "text": {:?}, "count": {count}}}"#,
        text::ONE_SITE
    )
}

#[test]
fn a_run_that_judged_no_finding_claims_nothing_about_the_base() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 0 escape site(s) in the tree ("),
        "{}",
        run.out
    );
    assert!(!run.says("held at the base"), "{}", run.out);
    assert!(!run.says("accepted list"), "{}", run.out);
}

#[test]
fn a_run_that_measured_nothing_claims_nothing_about_the_base() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree.write("README.md", "A word.\n");

    let run = tree.run(&["check", "--changed", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 file(s) found, 0 measured"), "{}", run.out);
    assert!(!run.says("held at the base"), "{}", run.out);
}

#[test]
fn a_run_the_base_holds_every_finding_of_says_the_base_held_them() {
    let tree = tree();
    tree.write("src/lib.rs", text::TWO_ON_TWO_LINES);
    tree.base();

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 1 escape site(s) in the tree, all held at the base"),
        "{}",
        run.out
    );
}

#[test]
fn a_finding_only_an_accepted_entry_holds_is_named_as_accepted_not_as_base_held() {
    let tree = tree();
    tree.write("klin.json", &accepted(&entry("src/lib.rs", 1)));
    tree.write("src/lib.rs", text::ONE);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 1 escape site(s) in the tree, all on the accepted list"),
        "{}",
        run.out
    );
    assert!(!run.says("held at the base"), "{}", run.out);
}

#[test]
fn a_mixed_run_names_the_base_and_the_accepted_list_with_a_count_each() {
    let tree = tree();
    tree.write("klin.json", &accepted(&entry("src/new.rs", 1)));
    tree.write("src/held.rs", text::ONE);
    tree.base();
    tree.write("src/new.rs", text::ONE);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 2 escape site(s) in the tree, 1 held at the base and 1 on the accepted list"),
        "{}",
        run.out
    );
}

#[test]
fn an_accepted_entry_that_matches_nothing_keeps_its_note_and_fails() {
    let tree = tree();
    tree.write("klin.json", &accepted(&entry("src/gone.rs", 1)));
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("matched nothing"), "{}", run.out);
}

#[test]
fn the_gate_row_counts_the_accepted_findings_beside_the_held_ones() {
    let tree = tree();
    tree.write("klin.json", &accepted(&entry("src/new.rs", 1)));
    tree.write("src/held.rs", text::ONE);
    tree.base();
    tree.write("src/new.rs", text::ONE);

    let run = tree.run(&["check", "--json", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let row = &harness::gate_rows(&report)[0];
    assert_eq!(row["held"], serde_json::json!(2), "{report}");
    assert_eq!(row["accepted"], serde_json::json!(1), "{report}");
}
