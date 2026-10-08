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

    let run = tree.run(&["check", "eslint"]);
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

    let run = tree.run(&["check", "eslint"]);
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

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 0 result(s) on lines this window changed, 1 held"),
        "{}",
        run.out
    );
}

#[test]
fn a_result_in_an_untracked_file_fails() {
    let tree = tree(ONE);
    tree.write("src/b.ts", "one\ntwo\n");
    tree.write(
        "eslint.sarif",
        &report(&[result("src/b.ts", 2, "no-any", "Unexpected any")]),
    );

    let run = tree.run(&["check", "eslint"]);
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

    let run = tree.run(&["check", "eslint"]);
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

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_accepted_entry_that_matches_nothing_is_a_review_item_and_no_failure() {
    let tree = tree(
        r#"{
        "sarif": [{"name": "eslint", "report": "eslint.sarif"}],
        "accepted": [
            {"gate": "eslint", "file": "src/a.ts", "text": "no-any: gone", "count": 1}
        ]
    }"#,
    );
    tree.write("eslint.sarif", &report(&[]));

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("REVIEW: 1 accepted entry matched nothing this run"),
        "{}",
        run.out
    );
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

    let run = tree.run(&["check", "eslint"]);
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

    let run = tree.run(&["check", "eslint"]);
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

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("could not place"), "{}", run.out);
    assert!(run.says("../outside/x.ts"), "{}", run.out);
}

#[test]
fn a_differential_entry_says_on_its_ok_line_that_it_judged_every_result() {
    let tree = tree(DIFFERENTIAL);
    tree.write("eslint.sarif", &report(&[]));

    let run = tree.run(&["check", "eslint"]);
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

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("is a list of entries"), "{}", run.out);
}

#[test]
fn an_entry_with_an_unknown_field_is_a_config_error() {
    let tree = tree(r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "extra": true}]}"#);

    let run = tree.run(&["check"]);
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

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 3, "{}", run.out);
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

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.ts:2"), "{}", run.out);
}

#[test]
fn a_report_that_is_not_sarif_is_a_tool_error_hole() {
    let tree = tree(ONE);
    tree.write("eslint.sarif", "not a report\n");

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("INCOMPLETE  eslint"), "{}", run.out);
    assert!(run.says("HOLE: tool-error"), "{}", run.out);
    assert!(run.says("is not SARIF"), "{}", run.out);
}

#[test]
fn a_missing_report_is_a_tool_error_hole_the_json_lists() {
    let tree = tree(ONE);

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("eslint.sarif"), "{}", run.out);

    let report = tree.run(&["check", "eslint", "--json"]).json();
    assert_eq!(report["exit"], 3, "{report}");
    assert_eq!(report["measurement"], "incomplete", "{report}");
    assert_eq!(hole(&report, "eslint")["reason"], "tool-error", "{report}");
}

/// The hole one gate's measurement record lists.
fn hole(report: &serde_json::Value, gate: &str) -> serde_json::Value {
    report["measurements"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|record| record["check"] == gate)
        .map(|record| record["holes"][0].clone())
        .unwrap_or_default()
}

#[test]
fn a_command_the_shell_cannot_find_is_a_tool_error_hole_with_its_detail() {
    let tree = tree(
        r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "run": "klin-no-such-scanner"}]}"#,
    );

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(
        run.says("HOLE: tool-error (command-not-found)"),
        "{}",
        run.out
    );

    let report = tree.run(&["check", "eslint", "--json"]).json();
    assert_eq!(
        hole(&report, "eslint")["detail"],
        "command-not-found",
        "{report}"
    );
}

#[test]
fn an_empty_fresh_report_is_complete_and_claims_no_coverage() {
    let tree = tree(ONE);
    tree.write("eslint.sarif", &report(&[]));

    let report = tree.run(&["check", "eslint", "--json"]).json();
    assert_eq!(report["exit"], 0, "{report}");
    assert_eq!(report["measurement"], "complete", "{report}");
    assert_eq!(report["capabilities"][0]["kind"], "integration", "{report}");
    assert_eq!(
        report["capabilities"][0]["coverage_claim"], "unverified",
        "{report}"
    );
}

#[test]
fn a_failing_run_with_a_hole_exits_1_and_lists_both() {
    let tree = tree(
        r#"{"sarif": [
            {"name": "eslint", "report": "eslint.sarif"},
            {"name": "semgrep", "report": "semgrep.sarif"}
        ]}"#,
    );
    tree.write(
        "eslint.sarif",
        &report(&[result("src/a.ts", 2, "no-any", "Unexpected any")]),
    );

    let report = tree.run(&["check", "eslint", "semgrep", "--json"]).json();
    assert_eq!(report["exit"], 1, "{report}");
    assert_eq!(report["judgement"], "fail", "{report}");
    assert_eq!(report["measurement"], "incomplete", "{report}");
    assert_eq!(report["findings"][0]["check"], "eslint", "{report}");
    assert_eq!(hole(&report, "semgrep")["reason"], "tool-error", "{report}");
}

#[test]
fn an_error_and_a_hole_together_exit_2() {
    let tree = tree(
        r#"{"doc_size": {"gone.md": 10}, "sarif": [{"name": "eslint", "report": "eslint.sarif"}]}"#,
    );

    let report = tree.run(&["check", "doc-size", "eslint", "--json"]).json();
    assert_eq!(report["exit"], 2, "{report}");
    assert_eq!(report["execution"], "error", "{report}");
    assert_eq!(report["measurement"], "incomplete", "{report}");
    assert_eq!(hole(&report, "eslint")["reason"], "tool-error", "{report}");
}

#[test]
fn an_integration_never_runs_at_the_stop() {
    let tree =
        tree(r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "run": "touch ran"}]}"#);

    let run = harness::feed(
        tree.root(),
        harness::AGENT,
        r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        !tree.path("ran").exists(),
        "the Stop ran the integration: {}",
        run.out
    );
    assert!(!run.says("eslint"), "{}", run.out);
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

    let run = tree.run(&["check", "eslint"]);
    assert_eq!(run.code, 3, "{}", run.out);
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

    let listed = tree.run(&["policy"]);
    assert!(listed.says("eslint — runs"), "{}", listed.out);
    assert!(listed.says("semgrep — runs"), "{}", listed.out);

    let run = tree.run(&["check", "semgrep"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    semgrep"), "{}", run.out);
}

#[test]
fn an_entry_with_no_name_names_the_key_it_is_missing() {
    let tree = tree(r#"{"sarif": [{"report": "eslint.sarif"}]}"#);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("has no \"name\""), "{}", run.out);
}

#[test]
fn a_run_that_never_exits_is_stopped_at_the_limit_and_named() {
    let tree = tree(
        r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif", "run": "sleep 60; echo never"}]}"#,
    );

    let started = std::time::Instant::now();
    let run = tree.run_with(&[("KLIN_COMMAND_LIMIT", "1")], &["check", "eslint"]);
    assert!(started.elapsed().as_secs() < 30, "{}", run.out);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("sleep 60; echo never"), "{}", run.out);
    assert!(run.says("the 1 second limit"), "{}", run.out);
}

#[test]
fn the_commands_of_one_run_share_twice_the_limit() {
    let tree = tree(
        r#"{"sarif": [
            {"name": "first", "report": "first.sarif", "run": "sleep 60"},
            {"name": "second", "report": "second.sarif", "run": "sleep 60"},
            {"name": "third", "report": "third.sarif", "run": "sleep 60"}
        ]}"#,
    );

    let started = std::time::Instant::now();
    let run = tree.run_with(
        &[("KLIN_COMMAND_LIMIT", "1")],
        &["check", "first", "second", "third"],
    );
    assert!(started.elapsed().as_secs() < 30, "{}", run.out);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("the 1 second limit"), "{}", run.out);
    assert!(
        run.says("klin did not start it, because the 2 second deadline from klin's start passed"),
        "{}",
        run.out
    );
}
