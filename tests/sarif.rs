mod harness;

use std::fs;
use std::time::SystemTime;

use harness::Tree;

const ONE: &str = r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif"}]}"#;
const DIFFERENTIAL: &str = r#"{"sarif": [
    {"name": "eslint", "report": "eslint.sarif", "differential": true}
]}"#;

fn result(uri: &str, line: u64, rule: &str, message: &str) -> String {
    located(uri, "", line, rule, message)
}

fn based(uri: &str, base: &str, line: u64, rule: &str) -> String {
    located(uri, base, line, rule, "m")
}

fn located(uri: &str, base: &str, line: u64, rule: &str, message: &str) -> String {
    let named = match base.is_empty() {
        true => String::new(),
        false => format!(r#", "uriBaseId": "{base}""#),
    };
    format!(
        r#"{{"ruleId": "{rule}", "message": {{"text": "{message}"}}, "locations": [
            {{"physicalLocation": {{
                "artifactLocation": {{"uri": "{uri}"{named}}},
                "region": {{"startLine": {line}}}
            }}}}
        ]}}"#
    )
}

fn report(results: &[String]) -> String {
    reported(results, "")
}

fn reported(results: &[String], bases: &str) -> String {
    format!(
        r#"{{"version": "2.1.0", "runs": [{{
            "tool": {{"driver": {{"name": "eslint"}}}},
            {bases}
            "results": [{}]
        }}]}}"#,
        results.join(", ")
    )
}

/// A tree whose base holds three lines of `src/a.ts`, with the second line changed since.
fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.write("src/a.ts", "one\ntwo\nthree\n");
    tree.base();
    tree.write("src/a.ts", "one\nchanged\nthree\n");
    tree
}

#[test]
fn a_result_on_a_changed_line_fails_with_its_file_line_rule_and_message() {
    let tree = tree(ONE);
    tree.write(
        "eslint.sarif",
        &report(&[result("src/a.ts", 2, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new result(s)"), "{}", run.out);
    assert!(
        run.says("src/a.ts:2  count 1  no-any: Unexpected any"),
        "{}",
        run.out
    );
}

#[test]
fn the_ok_line_counts_the_files_it_placed_and_the_places_it_could_not() {
    let tree = tree(ONE);
    tree.write(
        "eslint.sarif",
        &report(&[
            result(
                "src/a.ts",
                1,
                "no-any",
                "on a line the window did not change",
            ),
            result("src/b.ts", 1, "no-any", "in a second file of the tree"),
            result("/elsewhere/x.ts", 2, "no-any", "outside the tree"),
        ]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("(3 file(s) found, 2 measured, 0 excluded, 1 unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn a_result_on_a_line_the_window_did_not_change_is_held_and_counted() {
    let tree = tree(ONE);
    tree.write(
        "eslint.sarif",
        &report(&[result("src/a.ts", 3, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["sarif"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 0 result(s) on lines this window changed, 1 held"),
        "{}",
        run.out
    );

    let gated = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(gated.code, 0, "{}", gated.out);
}

#[test]
fn a_result_in_an_untracked_file_fails() {
    let tree = tree(ONE);
    tree.write("src/b.ts", "one\ntwo\n");
    tree.write(
        "eslint.sarif",
        &report(&[result("src/b.ts", 2, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/b.ts:2"), "{}", run.out);
}

#[test]
fn differential_judges_every_result_wherever_it_sits() {
    let tree = tree(DIFFERENTIAL);
    tree.write(
        "eslint.sarif",
        &report(&[result("src/a.ts", 3, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.ts:3"), "{}", run.out);
}

#[test]
fn an_accepted_entry_holds_a_result() {
    let tree = tree(
        r#"{
        "sarif": [{"name": "eslint", "report": "eslint.sarif"}],
        "accepted": [
            {"gate": "eslint", "file": "src/a.ts", "text": "no-any: Unexpected any", "count": 1}
        ]
    }"#,
    );
    tree.write(
        "eslint.sarif",
        &report(&[result("src/a.ts", 2, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_accepted_entry_that_matches_nothing_is_a_note_and_a_strict_failure() {
    let tree = tree(
        r#"{
        "sarif": [{"name": "eslint", "report": "eslint.sarif"}],
        "accepted": [
            {"gate": "eslint", "file": "src/a.ts", "text": "no-any: gone", "count": 1}
        ]
    }"#,
    );
    tree.write("eslint.sarif", &report(&[]));

    let loose = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(loose.code, 0, "{}", loose.out);
    assert!(loose.says("matched nothing this run"), "{}", loose.out);

    let strict = tree.run(&["gate", "--strict", "--gate", "eslint"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
    assert!(strict.says("Delete the line"), "{}", strict.out);
}

#[test]
fn every_location_form_resolves_to_a_path_in_the_tree() {
    let tree = tree(ONE);
    let root = tree.root().display().to_string();
    tree.write(
        "eslint.sarif",
        &reported(
            &[
                result("src/a.ts", 2, "relative", "m"),
                result(&format!("{root}/src/a.ts"), 2, "absolute", "m"),
                result(&format!("file://{root}/src/a.ts"), 2, "uri", "m"),
                based("src/a.ts", "SRCROOT", 2, "based"),
            ],
            &format!(r#""originalUriBaseIds": {{"SRCROOT": {{"uri": "file://{root}/"}}}},"#),
        ),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("4 new result(s)"), "{}", run.out);
    for rule in ["relative", "absolute", "uri", "based"] {
        assert!(
            run.says(&format!("src/a.ts:2  count 1  {rule}")),
            "{}",
            run.out
        );
    }
}

#[test]
fn a_location_klin_cannot_place_is_a_note_and_is_not_judged() {
    let tree = tree(ONE);
    tree.write(
        "eslint.sarif",
        &report(&[
            result("/elsewhere/x.ts", 2, "outside", "m"),
            r#"{"ruleId": "nowhere", "message": {"text": "m"}}"#.to_string(),
        ]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("/elsewhere/x.ts"), "{}", run.out);
    assert!(run.says("nowhere"), "{}", run.out);
}

#[test]
fn a_relative_location_that_climbs_out_of_the_tree_is_a_note() {
    let tree = tree(DIFFERENTIAL);
    tree.write(
        "eslint.sarif",
        &report(&[result("../outside/x.ts", 2, "outside", "m")]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("could not place"), "{}", run.out);
    assert!(run.says("../outside/x.ts"), "{}", run.out);
}

#[test]
fn a_differential_entry_says_on_its_ok_line_that_it_judged_every_result() {
    let tree = tree(DIFFERENTIAL);
    tree.write("eslint.sarif", &report(&[]));

    let run = tree.run(&["sarif"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 0 result(s) judged, which is every result the scanner reported"),
        "{}",
        run.out
    );
}

#[test]
fn a_section_that_is_not_a_list_of_entries_is_a_config_error() {
    let tree = tree(r#"{"sarif": {"report": "eslint.sarif"}}"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("is a list of entries"), "{}", run.out);
}

#[test]
fn an_entry_with_an_unknown_field_is_a_config_error() {
    let tree = tree(r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "extra": true}]}"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("has unknown field \"extra\""), "{}", run.out);
}

#[test]
fn run_deletes_the_report_it_finds_before_it_reads_the_one_the_tool_wrote() {
    let tree = tree(r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "run": "true"}]}"#);
    tree.write(
        "eslint.sarif",
        &report(&[result("src/a.ts", 2, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("wrote no report"), "{}", run.out);
}

#[test]
fn run_judges_the_report_a_nonzero_exit_wrote() {
    let tree = tree(
        r#"{"sarif": [
            {"name": "eslint", "report": "eslint.sarif", "run": "cp fixture.json eslint.sarif; exit 1"}
        ]}"#,
    );
    tree.write(
        "fixture.json",
        &report(&[result("src/a.ts", 2, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.ts:2"), "{}", run.out);
}

#[test]
fn a_report_that_is_not_sarif_is_a_tool_error() {
    let tree = tree(ONE);
    tree.write("eslint.sarif", "not a report\n");

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("is not SARIF"), "{}", run.out);
}

#[test]
fn a_missing_report_is_a_tool_error() {
    let tree = tree(ONE);

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("eslint.sarif"), "{}", run.out);
}

#[test]
fn a_report_older_than_a_file_the_window_changed_is_a_tool_error() {
    let tree = tree(ONE);
    tree.write(
        "eslint.sarif",
        &report(&[result("src/a.ts", 2, "no-any", "Unexpected any")]),
    );
    let held = fs::File::options()
        .write(true)
        .open(tree.path("eslint.sarif"));
    assert!(
        held.is_ok_and(|file| file.set_modified(SystemTime::UNIX_EPOCH).is_ok()),
        "the report's modification time could not be set back"
    );

    let run = tree.run(&["gate", "--gate", "eslint"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("older than"), "{}", run.out);
}

#[test]
fn each_entry_is_its_own_gate() {
    let tree = tree(
        r#"{"sarif": [
            {"name": "eslint", "report": "eslint.sarif"},
            {"name": "semgrep", "report": "semgrep.sarif"}
        ]}"#,
    );
    tree.write("semgrep.sarif", &report(&[]));

    let listed = tree.run(&["gate", "--list"]);
    assert!(listed.says("eslint — runs"), "{}", listed.out);
    assert!(listed.says("semgrep — runs"), "{}", listed.out);

    let run = tree.run(&["gate", "--gate", "semgrep"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    semgrep"), "{}", run.out);
}

#[test]
fn an_entry_with_no_name_names_the_key_it_is_missing() {
    let tree = tree(r#"{"sarif": [{"report": "eslint.sarif"}]}"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("has no \"name\""), "{}", run.out);
}
