mod harness;

use harness::Tree;
use serde_json::{Value, json};

const DAY: u64 = 86_400;

fn tree(lines: &[Value]) -> Tree {
    let tree = Tree::new();
    journal(&tree, lines);
    tree
}

/// Writes these lines as the tree's journal, in place of whatever it held.
fn journal(tree: &Tree, lines: &[Value]) {
    let at = tree.state("journal.jsonl");
    if let Some(parent) = at.parent() {
        assert!(
            std::fs::create_dir_all(parent).is_ok(),
            "the state directory"
        );
    }
    let text: String = lines
        .iter()
        .map(|line| line.to_string() + "\n")
        .collect::<Vec<_>>()
        .concat();
    assert!(std::fs::write(&at, text).is_ok(), "the journal");
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

/// A finding with the `id` of spec 11.2, which is what a real ratchet gate records and what
/// keys one regression across stops.
fn found(id: &str, gate: &str, file: &str, line: u64, text: &str, remedy: &str) -> Value {
    let mut site = finding(gate, file, line, text, remedy);
    site["id"] = json!(id);
    site
}

/// A finding with no `id`, which is the shape a gate such as `doc-size` records.
fn finding(gate: &str, file: &str, line: u64, text: &str, remedy: &str) -> Value {
    json!({
        "gate": gate,
        "outcome": "new",
        "file": file,
        "line": line,
        "text": text,
        "fix_advice": remedy,
    })
}

/// Every gate these fixtures use. A stop runs them all, and a gate a finding names is the one
/// that failed, which is the shape spec 11.2 gives a real stop's row.
const GATES: [&str; 5] = [
    "escapes",
    "stubs",
    "inventory",
    "doc-size",
    "a-gate-from-the-future",
];

fn gates(findings: &[Value]) -> Vec<Value> {
    GATES
        .iter()
        .map(|name| {
            let failed = findings.iter().any(|site| site["gate"] == *name);
            json!({"name": name, "status": if failed { "FAIL" } else { "ok" }, "ms": 1})
        })
        .collect()
}

fn stop(ago: u64, blocked: bool, findings: Vec<Value>, notes: Vec<Value>) -> Value {
    json!({
        "schema": 1,
        "version": "0.0.0",
        "kind": "stop",
        "time": now() - ago,
        "host": "claude",
        "session": "s-1",
        "prompt": 1,
        "hook": {"blocked": blocked, "delivery": "none", "gate_spent": true,
                 "build_blocks": 0, "blocked_before": false},
        "verdict": if blocked { "red" } else { "green" },
        "findings": findings,
        "notes": notes,
        "gates": gates(&findings),
        "timing": {"total_ms": 20, "build_ms": 0, "lock_ms": 0, "klin_ms": 20},
        "asked": [],
        "flags": [],
        "config_hash": "c-1",
        "exit": if blocked { 2 } else { 0 },
    })
}

/// The same stop under a configuration a person changed, which spec 11.4 records so a reader can
/// tell a code fix from a policy change.
fn reconfigured(mut line: Value) -> Value {
    line["config_hash"] = json!("c-2");
    line
}

const UNWRAP: &str = "Handle the error, or accept it in klin.json, before you push.";

/// The prompt line the stops of `stop` ran under, with the excerpt spec 11.4 records.
fn prompt_line(ago: u64, text: &str) -> Value {
    json!({"schema": 1, "version": "0.0.0", "kind": "prompt", "time": now() - ago,
           "session": "s-1", "prompt": 1, "text": text})
}

// The counted unit, and what keys it.

#[test]
fn one_id_over_four_blocked_stops_is_one_regression_with_its_latest_outcome() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut lines: Vec<Value> = (0..4)
        .map(|step| stop(400 - step * 10, true, vec![site.clone()], vec![]))
        .collect();
    lines.push(stop(300, false, vec![], vec![]));
    let tree = tree(&lines);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 1, "{json}");
    assert_eq!(json["episodes"].as_array().map(Vec::len), Some(1), "{json}");
    assert_eq!(json["episodes"][0]["outcome"], "fixed-later", "{json}");

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("Nothing needs your attention."), "{}", run.out);
    assert!(
        run.says("klin caught 1 regression this week. It was fixed after klin flagged it."),
        "{}",
        run.out
    );
}

#[test]
fn two_ids_at_one_line_stay_two_regressions_and_a_renamed_id_is_never_merged_with_the_old_one() {
    let tree = tree(&[
        stop(
            400,
            true,
            vec![
                found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP),
                found("id-b", "stubs", "src/io.rs", 12, "todo!()", "Do the work."),
            ],
            vec![],
        ),
        stop(
            300,
            true,
            vec![
                found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP),
                found("id-b", "stubs", "src/io.rs", 12, "todo!()", "Do the work."),
                found("id-c", "escapes", "src/moved.rs", 12, "unwrap()", UNWRAP),
            ],
            vec![],
        ),
    ]);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 3, "{json}");
    assert_eq!(json["counts"]["open"], 3, "{json}");
}

#[test]
fn a_finding_with_no_id_is_counted_by_its_recorded_fields_and_never_dropped() {
    let long = finding("doc-size", "README.md", 0, "", "Cut the document.");
    let other = finding("doc-size", "CONTEXT.md", 0, "", "Cut the document.");
    let tree = tree(&[
        stop(400, true, vec![long.clone(), other.clone()], vec![]),
        stop(300, true, vec![long, other], vec![]),
    ]);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 2, "{json}");
    let files: Vec<&str> = json["episodes"]
        .as_array()
        .unwrap_or_else(|| panic!("{json}"))
        .iter()
        .filter_map(|one| one["key"]["file"].as_str())
        .collect();
    assert!(files.contains(&"README.md"), "{json}");
    assert!(files.contains(&"CONTEXT.md"), "{json}");
    assert_eq!(json["episodes"][0]["id"], Value::Null, "{json}");
}

#[test]
fn two_sites_under_one_failing_gate_resolve_independently() {
    let first = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let second = found("id-b", "escapes", "src/api.rs", 83, "expect()", UNWRAP);
    let tree = tree(&[
        stop(400, true, vec![first.clone(), second.clone()], vec![]),
        stop(300, true, vec![second], vec![]),
    ]);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 2, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 1, "{json}");
    assert_eq!(json["counts"]["open"], 1, "{json}");
}

#[test]
fn a_gate_that_measured_nothing_never_makes_an_earlier_regression_read_as_fixed() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut errored = stop(300, false, vec![], vec![]);
    errored["gates"] = json!([{"name": "escapes", "status": "ERR", "ms": 1}]);
    let mut nothing_ran = stop(250, true, vec![], vec![]);
    nothing_ran["gates"] = json!([]);

    let open = tree(&[stop(400, true, vec![site.clone()], vec![]), errored]);
    let json = open.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["open"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 0, "{json}");

    let build = tree(&[stop(400, true, vec![site.clone()], vec![]), nothing_ran]);
    let json = build.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["open"], 1, "{json}");

    let later = tree(&[
        stop(400, true, vec![site], vec![]),
        {
            let mut line = stop(300, true, vec![], vec![]);
            line["gates"] = json!([]);
            line
        },
        stop(200, false, vec![], vec![]),
    ]);
    let json = later.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["fixed-next"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-later"], 0, "{json}");
}

#[test]
fn a_regression_that_goes_after_the_config_changed_is_not_reported_as_a_code_fix() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[
        stop(400, true, vec![site], vec![]),
        reconfigured(stop(300, false, vec![], vec![])),
    ]);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["config-changed"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 0, "{json}");
    assert_eq!(json["episodes"][0]["config_changed"], true, "{json}");

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("klin caught 1 regression this week. 1 resolved after the config changed."),
        "{}",
        run.out
    );

    let all = tree.run(&["stats", "--all"]);
    assert_eq!(all.code, 0, "{}", all.out);
    assert!(
        all.says("Resolved after the config changed."),
        "{}",
        all.out
    );
    assert!(!all.says("was fixed after klin flagged"), "{}", all.out);
}

// The opening state, in priority order.

#[test]
fn every_regression_resolved_says_nothing_needs_your_attention() {
    let mut lines = Vec::new();
    for step in 0..3u64 {
        lines.push(stop(
            400 - step * 20,
            true,
            vec![found(
                &format!("id-{step}"),
                "escapes",
                &format!("src/f{step}.rs"),
                1,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ));
        lines.push(stop(390 - step * 20, false, vec![], vec![]));
    }
    let tree = tree(&lines);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("Nothing needs your attention."), "{}", run.out);
    assert!(
        run.says("klin caught 3 regressions this week. All 3 were fixed after klin flagged them."),
        "{}",
        run.out
    );
    assert!(!run.says("klin ran"), "{}", run.out);
    assert!(!run.says("shortcut"), "{}", run.out);
}

#[test]
fn one_open_regression_opens_the_report_and_names_its_site() {
    let tree = tree(&[
        prompt_line(500, "Fix the refund flow"),
        stop(
            400,
            true,
            vec![
                found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP),
                found("id-b", "stubs", "src/pay.rs", 41, "todo!()", "Do the work."),
            ],
            vec![],
        ),
        stop(
            300,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/io.rs",
                12,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 regression needs your attention."),
        "{}",
        run.out
    );
    assert!(run.says("klin caught 2 this week."), "{}", run.out);
    assert!(
        run.says("1 was fixed after klin flagged it."),
        "{}",
        run.out
    );
    assert!(run.says("src/io.rs:12  unwrap()"), "{}", run.out);
    assert!(!run.says("while you asked for"), "{}", run.out);
}

#[test]
fn the_default_report_names_three_open_sites_and_points_at_all_for_the_rest() {
    let sites: Vec<Value> = (0..4u64)
        .map(|step| {
            found(
                &format!("id-{step}"),
                "escapes",
                &format!("src/f{step}.rs"),
                step + 1,
                "unwrap()",
                UNWRAP,
            )
        })
        .collect();
    let tree = tree(&[stop(400, true, sites, vec![])]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("4 regressions need your attention."),
        "{}",
        run.out
    );
    assert!(run.says("and 1 more · klin stats --all"), "{}", run.out);
    assert_eq!(
        run.out.matches("unwrap()").count(),
        3,
        "three sites and no more: {}",
        run.out
    );

    let all = tree.run(&["stats", "--all"]);
    assert!(!all.says("and 1 more · klin stats --all"), "{}", all.out);
    assert!(all.says("src/f3.rs:4"), "{}", all.out);
}

#[test]
fn a_reset_sets_regressions_aside_and_never_calls_them_fixed_or_still_in_the_tree() {
    let tree = tree(&[
        stop(
            400,
            true,
            vec![
                found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP),
                found("id-b", "stubs", "src/pay.rs", 41, "todo!()", "Do the work."),
            ],
            vec![],
        ),
        reset(300),
        stop(200, false, vec![], vec![]),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("2 regressions were set aside when you restarted."),
        "{}",
        run.out
    );
    assert!(!run.says("needs your attention"), "{}", run.out);
    assert!(!run.says("still there"), "{}", run.out);
    assert!(!run.says("were fixed"), "{}", run.out);

    let all = tree.run(&["stats", "--all"]);
    assert!(
        all.says("You restarted, and 2 regressions were set aside."),
        "{}",
        all.out
    );

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["set-aside"], 2, "{json}");
    assert_eq!(json["counts"]["caught"], 2, "{json}");
}

#[test]
fn open_attention_comes_before_the_uncertainty_a_reset_left() {
    let tree = tree(&[
        stop(
            500,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/old.rs",
                3,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ),
        reset(400),
        stop(
            300,
            true,
            vec![found(
                "id-b",
                "stubs",
                "src/pay.rs",
                41,
                "todo!()",
                "Do it.",
            )],
            vec![],
        ),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let said = run.out.replace("\r\n", "\n");
    let open = said
        .find("1 regression needs your attention.")
        .unwrap_or_else(|| panic!("{said}"));
    let aside = said
        .find("1 more was set aside when you restarted.")
        .unwrap_or_else(|| panic!("{said}"));
    assert!(open < aside, "{said}");
}

/// The whole opening order, top pair first: a window klin did not measure whole says so above
/// the regressions it knows are open, and the set-aside uncertainty follows both.
#[test]
fn measurement_doubt_opens_the_report_above_the_open_regressions_it_knows_of() {
    let unparsed = json!({"gate": "complexity", "outcome": "unparsed", "file": "src/odd.rs",
                          "text": "no grammar reads it"});
    let tree = tree(&[
        stop(
            500,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/old.rs",
                3,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ),
        reset(400),
        stop(
            300,
            true,
            vec![found(
                "id-b",
                "stubs",
                "src/pay.rs",
                41,
                "todo!()",
                "Do it.",
            )],
            vec![unparsed],
        ),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let said = run.out.replace("\r\n", "\n");
    let at = |text: &str| {
        said.find(text)
            .unwrap_or_else(|| panic!("{text} missing from: {said}"))
    };
    let gap = at("Stats may be incomplete: 1 source file couldn't be parsed.");
    let open = at("1 regression needs your attention.");
    let aside = at("1 more was set aside when you restarted.");
    assert!(gap < open && open < aside, "{said}");
    assert!(!run.says("Nothing needs your attention."), "{said}");
}

#[test]
fn measurement_doubt_outranks_the_value_story_and_forbids_nothing_needs_your_attention() {
    let unparsed = json!({"gate": "complexity", "outcome": "unparsed", "file": "src/odd.rs",
                          "text": "no grammar reads it"});
    let other = json!({"gate": "complexity", "outcome": "unparsed", "file": "src/odder.rs",
                       "text": "no grammar reads it"});
    let tree = tree(&[
        stop(
            400,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/io.rs",
                12,
                "unwrap()",
                UNWRAP,
            )],
            vec![unparsed.clone(), other.clone()],
        ),
        stop(300, false, vec![], vec![unparsed, other]),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("Stats may be incomplete: 2 source files couldn't be parsed."),
        "{}",
        run.out
    );
    assert!(!run.says("Nothing needs your attention."), "{}", run.out);
    assert!(
        run.says("klin caught 1 regression this week. The one known regression was fixed."),
        "{}",
        run.out
    );
}

#[test]
fn a_quiet_window_klin_did_not_measure_whole_never_says_everything_is_clear() {
    let lost = json!({"gate": "dead-symbols", "outcome": "lost", "file": "src/gone.rs",
                      "text": "the base measured it and this tree did not"});
    let tree = tree(&[stop(300, false, vec![], vec![lost])]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("Stats may be incomplete: 1 file wasn't measured."),
        "{}",
        run.out
    );
    assert!(
        run.says("No regressions were found this week."),
        "{}",
        run.out
    );
    assert!(!run.says("Nothing needs your attention."), "{}", run.out);

    let all = tree.run(&["stats", "--all"]);
    assert!(all.says("Measurement"), "{}", all.out);
    assert!(
        all.says("1 file(s) the base measured and this tree did not: src/gone.rs"),
        "{}",
        all.out
    );
}

#[test]
fn a_journal_line_the_reader_cannot_take_lowers_confidence_and_fails_nothing() {
    let mut newer = stop(100, true, vec![], vec![]);
    newer["schema"] = json!(2);
    let tree = tree(&[stop(200, false, vec![], vec![]), newer]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("Stats may be incomplete: 1 journal record couldn't be read."),
        "{}",
        run.out
    );

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["skipped"], 1, "{json}");
    assert_eq!(json["confidence"]["whole"], false, "{json}");
}

#[test]
fn a_measured_window_with_no_regression_says_none_were_found_and_nothing_else() {
    let quiet = tree(&[
        stop(200, false, vec![], vec![]),
        stop(100, false, vec![], vec![]),
    ]);
    let run = quiet.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("Nothing needs your attention."), "{}", run.out);
    assert!(
        run.says("No regressions were found this week."),
        "{}",
        run.out
    );
    assert!(!run.says("klin ran"), "{}", run.out);
    assert!(!run.says("took"), "{}", run.out);
}

#[test]
fn an_empty_journal_says_klin_is_on() {
    let fresh = Tree::new();
    let first = fresh.run(&["stats"]);
    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        first.says("klin is on. Your first recap appears after the agent finishes a task."),
        "{}",
        first.out
    );
}

// What the default report leaves out.

#[test]
fn the_default_report_prints_no_activity_dashboard() {
    let tree = tree(&[
        prompt_line(500, "Fix the refund flow"),
        guard(450, "deny", "config-write"),
        stop(
            400,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/io.rs",
                12,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ),
        stop(300, false, vec![], vec![]),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    for absent in [
        "klin ran",
        "seconds in total",
        "Last week",
        "This week is",
        "klin refused",
        "Fix the refund flow",
        "Still there",
        "escapes",
        "shortcut",
    ] {
        assert!(!run.says(absent), "{absent} in: {}", run.out);
    }
}

// Questions, which are not regressions.

#[test]
fn a_deleted_test_klin_let_through_is_a_question_and_stays_out_of_the_count() {
    let tree = tree(&[
        stop(
            400,
            true,
            vec![
                found(
                    "id-a",
                    "inventory",
                    "tests/pay.rs",
                    20,
                    "fn refund_twice() {",
                    "Restore the test.",
                ),
                found(
                    "id-b",
                    "inventory",
                    "tests/pay.rs",
                    40,
                    "fn refund_once() {",
                    "Restore the test.",
                ),
            ],
            vec![],
        ),
        stop(
            300,
            false,
            vec![],
            vec![json!({
                "gate": "inventory",
                "outcome": "deleted",
                "file": "tests/pay.rs",
                "line": 20,
                "text": "the test site refund_twice went in this window",
            })],
        ),
    ]);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["asked-once"], 1, "{json}");
    assert_eq!(json["counts"]["caught"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 1, "{json}");

    let run = tree.run(&["stats"]);
    assert!(
        run.says("klin caught 1 regression this week. It was fixed after klin flagged it."),
        "{}",
        run.out
    );

    let all = tree.run(&["stats", "--all"]);
    assert!(
        all.says("a test deleted from tests/pay.rs:20, fn refund_twice(). The agent said why."),
        "{}",
        all.out
    );
}

#[test]
fn a_reset_and_a_guard_deny_ask_the_person_nothing_and_a_guard_ask_does() {
    let tree = tree(&[
        guard(500, "deny", "config-write"),
        guard(450, "ask", "state-mention"),
        stop(
            400,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/io.rs",
                12,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ),
        reset(300),
    ]);

    let all = tree.run(&["stats", "--all"]);
    assert_eq!(all.code, 0, "{}", all.out);
    assert!(all.says("klin refused an edit to klin.json"), "{}", all.out);
    assert!(
        all.says("klin asked you before a command that named klin's own state"),
        "{}",
        all.out
    );
    assert!(
        all.says("You restarted, and 1 regression was set aside."),
        "{}",
        all.out
    );
    assert!(
        !all.says("klin asked you before an edit to klin.json"),
        "{}",
        all.out
    );

    let json = tree.run(&["stats", "--json"]).json();
    let kinds: Vec<&str> = json["audit"]
        .as_array()
        .unwrap_or_else(|| panic!("{json}"))
        .iter()
        .filter_map(|one| one["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["reset", "guard", "guard"], "{json}");
}

// The catalogue owns the words, and a gate klin no longer has stays readable.

#[test]
fn every_catalogue_gate_gives_the_report_a_human_label_of_its_own() {
    for gate in catalogue() {
        let tree = tree(&[stop(
            300,
            true,
            vec![found("id-a", &gate, "src/one.rs", 1, "a site", "Fix it.")],
            vec![],
        )]);
        let all = tree.run(&["stats", "--all"]);
        assert_eq!(all.code, 0, "{}", all.out);
        assert!(
            !all.says(&format!("  1 {gate}\n")),
            "{gate} printed its own name for want of a label: {}",
            all.out
        );
        assert!(all.says("  1 "), "{gate}: {}", all.out);
    }
}

/// Every check the catalogue holds, read off the error the runner prints when a written
/// configuration names no gate over a tree the survey finds nothing in.
fn catalogue() -> Vec<String> {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let Some((_, listed)) = run.out.split_once("one of: ") else {
        panic!("no check list in: {}", run.out);
    };
    listed
        .lines()
        .next()
        .unwrap_or_default()
        .split(", ")
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

#[test]
fn a_gate_klin_has_no_check_for_is_read_and_printed_under_its_recorded_name() {
    let tree = tree(&[stop(
        300,
        true,
        vec![found(
            "id-a",
            "a-gate-from-the-future",
            "src/new.rs",
            7,
            "a site",
            "Fix it before you push.",
        )],
        vec![],
    )]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("src/new.rs:7  a site"), "{}", run.out);

    let all = tree.run(&["stats", "--all"]);
    assert!(all.says("1 a-gate-from-the-future"), "{}", all.out);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(
        json["episodes"][0]["gate"], "a-gate-from-the-future",
        "{json}"
    );
    assert_eq!(
        json["episodes"][0]["label"], "a-gate-from-the-future",
        "{json}"
    );
}

// The evidence surfaces.

#[test]
fn all_carries_the_story_the_default_report_hides() {
    let tree = tree(&[
        prompt_line(500, "Fix the refund flow"),
        stop(
            400,
            true,
            vec![
                found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP),
                found("id-b", "stubs", "src/pay.rs", 41, "todo!()", "Do the work."),
            ],
            vec![],
        ),
        stop(
            300,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/io.rs",
                12,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ),
    ]);

    let all = tree.run(&["stats", "--all"]);
    assert_eq!(all.code, 0, "{}", all.out);
    assert!(
        all.says("klin, this week in this repository"),
        "{}",
        all.out
    );
    assert!(
        all.says("You asked: \"Fix the refund flow\""),
        "{}",
        all.out
    );
    assert!(all.says("1 escape hatch"), "{}", all.out);
    assert!(all.says("1 stub"), "{}", all.out);
    assert!(all.says("Still open."), "{}", all.out);
    assert!(
        all.says("Fixed after klin flagged it on the next measured try."),
        "{}",
        all.out
    );
    assert!(all.says(UNWRAP), "{}", all.out);
    assert!(!all.says("fixed by the agent"), "{}", all.out);
}

#[test]
fn json_prints_one_episode_per_regression_identity_and_no_grouped_more() {
    let open = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[
        stop(
            3 * DAY,
            true,
            vec![
                open.clone(),
                found("id-b", "stubs", "src/pay.rs", 41, "todo!()", "Do the work."),
            ],
            vec![],
        ),
        stop(2 * DAY, true, vec![open.clone()], vec![]),
        stop(DAY, false, vec![open], vec![]),
    ]);

    let json = tree.run(&["stats", "--json"]).json();
    let episodes = json["episodes"]
        .as_array()
        .unwrap_or_else(|| panic!("{json}"));
    assert_eq!(episodes.len(), 2, "{json}");
    assert!(
        episodes.iter().all(|one| one.get("more").is_none()),
        "{json}"
    );
    assert_eq!(json["counts"]["caught"], 2, "{json}");
    assert_eq!(json["counts"]["open"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 1, "{json}");
    assert_eq!(json["activity"]["stops"], 3, "{json}");
    let escapes = episodes
        .iter()
        .find(|one| one["id"] == "id-a")
        .unwrap_or_else(|| panic!("{json}"));
    assert_eq!(escapes["outcome"], "open", "{json}");
    assert_eq!(escapes["tries"], 2, "{json}");
    assert_eq!(escapes["label"], "escape hatch", "{json}");
}

#[test]
fn a_since_that_is_not_a_number_of_days_is_a_usage_error() {
    let tree = tree(&[stop(100, false, vec![], vec![])]);
    let run = tree.run(&["stats", "--since", "a-week"]);
    assert_eq!(run.code, 2, "{}", run.out);
}

#[test]
fn since_widens_the_window_and_the_sentence_says_which_one_it_is() {
    let tree = tree(&[
        stop(
            20 * DAY,
            true,
            vec![found(
                "id-a",
                "escapes",
                "src/io.rs",
                12,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ),
        stop(19 * DAY, false, vec![], vec![]),
    ]);

    let week = tree.run(&["stats"]);
    assert_eq!(week.code, 0, "{}", week.out);
    assert!(
        week.says("No regressions were found this week."),
        "{}",
        week.out
    );

    let month = tree.run(&["stats", "--since", "30d"]);
    assert_eq!(month.code, 0, "{}", month.out);
    assert!(
        month.says("klin caught 1 regression this month."),
        "{}",
        month.out
    );
}

// The turn and the session.

fn in_session(mut line: Value, session: &str) -> Value {
    line["session"] = json!(session);
    line
}

fn guard(ago: u64, decision: &str, reason: &str) -> Value {
    json!({"schema": 1, "version": "0.0.0", "kind": "guard", "time": now() - ago,
           "session": "s-1", "decision": decision, "reason": reason})
}

fn reset(ago: u64) -> Value {
    json!({"schema": 1, "version": "0.0.0", "kind": "reset", "time": now() - ago,
           "session": null, "prompt": 1})
}

#[test]
fn session_reports_only_the_lines_carrying_the_newest_session_id() {
    let earlier = found("id-a", "escapes", "src/old.rs", 3, "unwrap()", UNWRAP);
    let newer = found("id-b", "stubs", "src/pay.rs", 41, "todo!()", "Do the work.");
    let tree = tree(&[
        in_session(stop(400, true, vec![earlier], vec![]), "s-1"),
        in_session(stop(300, false, vec![], vec![]), "s-1"),
        in_session(stop(200, true, vec![newer.clone()], vec![]), "s-2"),
        in_session(stop(100, false, vec![], vec![]), "s-2"),
    ]);

    let run = tree.run(&["stats", "--session"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("Nothing needs your attention."), "{}", run.out);
    assert!(
        run.says("klin caught 1 regression this session. It was fixed after klin flagged it."),
        "{}",
        run.out
    );
    assert!(!run.says("src/old.rs"), "{}", run.out);
}

const HOOKED: &str = r#"{
  "escapes": { "in": "src" }
}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false,
                         "session_id": "s-1"}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true,
                                "session_id": "s-1"}"#;
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-1",
                           "prompt": "Fix the refund flow"}"#;
const CLEAN: &str = "pub fn f(v: Option<i32>) -> i32 {\n    v.unwrap_or(0)\n}\n";

fn hooked() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", HOOKED);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

fn an_escape() -> String {
    format!(
        "pub fn f(v: Option<i32>) -> i32 {{\n    v.{}()\n}}\n",
        "unwrap"
    )
}

fn hook(tree: &Tree, event: &str) -> harness::Run {
    harness::feed(tree.root(), &["gate", "--hook"], event)
}

fn prompt(tree: &Tree) {
    let run = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
}

/// A prompt, an escape, and the stop that blocks on it.
fn blocked(tree: &Tree) {
    prompt(tree);
    tree.write("src/lib.rs", &an_escape());
    let run = hook(tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
}

/// What a stop hands the person through the host's `systemMessage`, or nothing.
fn told(run: &harness::Run) -> String {
    run.printed
        .lines()
        .find_map(|line| {
            let held: Value = serde_json::from_str(line).ok()?;
            held.get("systemMessage")?.as_str().map(str::to_string)
        })
        .unwrap_or_default()
}

#[test]
fn turn_reports_the_stops_since_the_stamp_and_a_reset_sets_the_rest_aside() {
    let tree = hooked();
    blocked(&tree);

    let before = tree.run(&["stats", "--turn"]);
    assert_eq!(before.code, 0, "{}", before.out);
    assert!(
        before.says("1 regression needs your attention."),
        "{}",
        before.out
    );
    assert!(before.says("klin caught 1 this turn."), "{}", before.out);

    let reset = tree.run(&["turn", "reset"]);
    assert_eq!(reset.code, 0, "{}", reset.out);
    let after = hook(&tree, A_SECOND_STOP);
    assert_eq!(after.code, 0, "{}", after.out);

    let turn = tree.run(&["stats", "--turn"]);
    assert!(
        turn.says("No regressions were found this turn."),
        "{}",
        turn.out
    );

    let week = tree.run(&["stats"]);
    assert!(
        week.says("1 regression was set aside when you restarted."),
        "{}",
        week.out
    );
    assert!(!week.says("still in your code"), "{}", week.out);
}

#[test]
fn a_green_stop_after_a_block_tells_the_turn_in_regressions_and_claims_no_author() {
    let tree = hooked();
    blocked(&tree);
    tree.write("src/lib.rs", CLEAN);

    let green = hook(&tree, A_SECOND_STOP);
    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(
        told(&green),
        "klin caught 1 regression this turn. It was fixed after klin flagged it.",
        "{}",
        green.out
    );

    let quiet = hooked();
    prompt(&quiet);
    let run = hook(&quiet, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(told(&run), "", "{}", run.out);
}

#[test]
fn a_red_pass_through_tells_the_person_one_regression_still_needs_them() {
    let tree = hooked();
    blocked(&tree);

    let through = hook(&tree, A_SECOND_STOP);
    assert_eq!(through.code, 0, "{}", through.out);
    assert_eq!(
        told(&through),
        "1 regression still needs your attention. `klin stats --turn` shows it.",
        "{}",
        through.out
    );
}

#[test]
fn a_fix_in_a_later_prompt_of_the_same_turn_still_tells_the_count() {
    let tree = hooked();
    blocked(&tree);
    let through = hook(&tree, A_SECOND_STOP);
    assert_eq!(through.code, 0, "{}", through.out);

    prompt(&tree);
    tree.write("src/lib.rs", CLEAN);
    let green = hook(&tree, A_STOP);
    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(
        told(&green),
        "klin caught 1 regression this turn. It was fixed after klin flagged it.",
        "{}",
        green.out
    );
}

/// A journal whose last weekly line went out eight days ago, from a stop under a stamp long gone.
fn weekly_eight_days_ago(tree: &Tree) {
    let mut old = stop(8 * DAY, false, vec![], vec![]);
    old["window"] = json!({"kind": "turn", "before": "an-old-stamp"});
    old["told"] = json!(["turn", "weekly"]);
    journal(tree, &[old]);
}

#[test]
fn the_weekly_line_rides_the_first_turn_end_seven_days_after_the_last_and_not_the_next() {
    let tree = hooked();
    weekly_eight_days_ago(&tree);

    blocked(&tree);
    tree.write("src/lib.rs", CLEAN);
    let first = hook(&tree, A_SECOND_STOP);
    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        told(&first).starts_with("klin caught 1 regression this turn"),
        "{}",
        first.out
    );
    assert!(
        told(&first).ends_with(
            "\nklin caught 1 regression in the last seven days. It was fixed after klin \
             flagged it. `klin stats` shows them."
        ),
        "{}",
        first.out
    );

    blocked(&tree);
    tree.write("src/lib.rs", CLEAN);
    let next = hook(&tree, A_SECOND_STOP);
    assert_eq!(next.code, 0, "{}", next.out);
    assert!(told(&next).starts_with("klin caught"), "{}", next.out);
    assert!(
        !told(&next).contains("in the last seven days"),
        "{}",
        next.out
    );
}

/// A stop's telling reads the history its turn and its week need and stops there. The prefix here
/// is a month of blocked stops, each with its own regression, so the richer per-site aggregation
/// of #172 is what the bounded reader of #184 keeps out of the stop path.
#[test]
fn a_long_history_of_regressions_does_not_change_what_the_turn_end_tells() {
    let tree = hooked();
    let old: Vec<Value> = (0..2_000u64)
        .map(|step| {
            stop(
                30 * DAY + step,
                true,
                vec![found(
                    &format!("old-{step}"),
                    "escapes",
                    &format!("src/old{step}.rs"),
                    1,
                    "unwrap()",
                    UNWRAP,
                )],
                vec![],
            )
        })
        .collect();
    journal(&tree, &old);

    blocked(&tree);
    tree.write("src/lib.rs", CLEAN);
    let run = hook(&tree, A_SECOND_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        told(&run).starts_with("klin caught 1 regression this turn. It was fixed after klin"),
        "{}",
        run.out
    );
    assert!(
        told(&run).ends_with(
            "\nklin caught 1 regression in the last seven days. It was fixed after klin \
             flagged it. `klin stats` shows them."
        ),
        "{}",
        run.out
    );
}

/// A whole test file the base held that the working tree no longer has. The site names no
/// declaration, so the audit line names the file alone.
#[test]
fn a_deleted_test_file_reads_as_the_file_deleted() {
    let tree = tree(&[
        stop(
            300,
            true,
            vec![finding("inventory", "tests/test_two.py", 0, "", "")],
            vec![],
        ),
        stop(
            200,
            false,
            vec![],
            vec![
                json!({"gate": "inventory", "outcome": "deleted", "file": "tests/test_two.py",
                        "line": 0, "text": "the test file went in this window"}),
            ],
        ),
    ]);

    let all = tree.run(&["stats", "--all"]);
    assert_eq!(all.code, 0, "{}", all.out);
    assert!(
        all.says("tests/test_two.py deleted. The agent said why."),
        "{}",
        all.out
    );
    assert_eq!(
        tree.run(&["stats", "--json"]).json()["counts"]["asked-once"],
        1
    );
}

/// The facts the default report stopped printing are still facts. `--json` keeps the previous
/// window and klin's own time, which is the `klin_ms` of spec 11.4 and never the project's build.
#[test]
fn json_keeps_the_previous_window_and_klins_own_time_the_default_no_longer_prints() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let other = found("id-b", "stubs", "src/pay.rs", 41, "todo!()", "Do the work.");
    let this_week = [
        timed(
            stop(2 * DAY, true, vec![site.clone()], vec![]),
            9_000,
            7_000,
        ),
        timed(stop(2 * DAY - 60, false, vec![], vec![]), 9_000, 8_000),
    ];
    let mut two = vec![
        stop(15 * DAY, false, vec![], vec![]),
        stop(10 * DAY, true, vec![site, other.clone()], vec![]),
        stop(10 * DAY - 60, false, vec![other], vec![]),
    ];
    two.extend(this_week.iter().cloned());

    let both = tree(&two);
    let json = both.run(&["stats", "--json"]).json();
    assert_eq!(json["earlier"], json!({"caught": 2, "open": 1}), "{json}");
    assert_eq!(json["activity"]["klin_ms"], 3_000, "{json}");

    let run = both.run(&["stats"]);
    assert!(!run.says("Last week"), "{}", run.out);
    assert!(!run.says("seconds in total"), "{}", run.out);

    let alone = tree(&this_week);
    assert_eq!(
        alone.run(&["stats", "--json"]).json()["earlier"],
        Value::Null
    );
}

fn timed(mut line: Value, total_ms: u64, build_ms: u64) -> Value {
    line["timing"] = json!({"total_ms": total_ms, "build_ms": build_ms, "lock_ms": 0,
                            "klin_ms": total_ms - build_ms});
    line
}

fn when(mut line: Value, time: u64) -> Value {
    line["time"] = json!(time);
    line
}

#[test]
fn turn_reads_exactly_the_lines_at_or_after_the_time_the_stamp_was_taken() {
    let tree = hooked();
    prompt(&tree);
    let taken: u64 = tree
        .field("time")
        .parse()
        .unwrap_or_else(|_| panic!("the stamp holds its time"));
    let old = found("id-a", "escapes", "src/old.rs", 1, "unwrap()", UNWRAP);
    let new = found("id-b", "stubs", "src/new.rs", 1, "todo!()", "Do the work.");
    journal(
        &tree,
        &[
            when(stop(0, true, vec![old], vec![]), taken - 1),
            when(stop(0, true, vec![new], vec![]), taken),
        ],
    );

    let run = tree.run(&["stats", "--turn"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("src/new.rs:1  todo!()"), "{}", run.out);
    assert!(!run.says("src/old.rs"), "{}", run.out);
}

/// A window the reader could not place. `--turn` over a worktree holding no readable stamp reads
/// no line at all, so the report says so instead of reporting a quiet turn.
#[test]
fn a_window_klin_cannot_place_says_so_and_never_reads_as_a_quiet_one() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[stop(300, true, vec![site], vec![])]);

    let turn = tree.run(&["stats", "--turn"]);
    assert_eq!(turn.code, 0, "{}", turn.out);
    assert!(
        turn.says("Stats may be incomplete: klin could not tell where this turn began."),
        "{}",
        turn.out
    );
    assert!(!turn.says("Nothing needs your attention."), "{}", turn.out);
    assert!(!turn.says("No regressions were found"), "{}", turn.out);
    assert_eq!(
        tree.run(&["stats", "--turn", "--json"]).json()["confidence"]["whole"],
        false
    );

    let week = tree.run(&["stats"]);
    assert!(!week.says("could not tell where"), "{}", week.out);
    assert!(
        week.says("1 regression needs your attention."),
        "{}",
        week.out
    );
}

/// The same hole on the sibling scope: a journal carrying no session id cannot place --session.
#[test]
fn a_session_scope_over_a_journal_with_no_session_id_says_it_could_not_place_it() {
    let mut line = stop(300, false, vec![], vec![]);
    line["session"] = json!(null);
    let tree = tree(&[line]);

    let run = tree.run(&["stats", "--session"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("Stats may be incomplete: klin could not tell where this session began."),
        "{}",
        run.out
    );
    assert!(!run.says("No regressions were found"), "{}", run.out);
}
