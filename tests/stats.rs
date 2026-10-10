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

/// How many episodes of the report ended this way. Spec 13.3 keeps the episodes beside the
/// document.
fn outcomes(json: &Value, outcome: &str) -> usize {
    json["episodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|episode| episode["outcome"] == outcome)
        .count()
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

/// A finding with the `id` of spec 11.7, which is what a real ratchet check records and what
/// keys one regression across stops.
fn found(id: &str, gate: &str, file: &str, line: u64, text: &str, remedy: &str) -> Value {
    let mut site = finding(gate, file, line, text, remedy);
    site["id"] = json!(id);
    site
}

/// A finding with no `id`.
fn finding(gate: &str, file: &str, line: u64, text: &str, remedy: &str) -> Value {
    json!({
        "check": gate,
        "kind": "metric",
        "outcome": "new",
        "file": file,
        "line": line,
        "text": text,
        "remedy": remedy,
    })
}

/// A note of the check document, as a Stop's `result` holds it.
fn note(gate: &str, kind: &str, file: &str, line: u64, message: &str) -> Value {
    json!({"check": gate, "kind": kind, "coverage": false, "file": file, "line": line,
           "message": message})
}

/// Every check these fixtures use. A stop runs them all, each under semantics version 1, which
/// is the shape spec 11.7 gives a real stop's document.
const GATES: [&str; 5] = [
    "escapes",
    "stubs",
    "inventory",
    "doc-size",
    "a-gate-from-the-future",
];

fn capabilities(findings: &[Value]) -> Vec<Value> {
    GATES
        .iter()
        .map(|name| {
            let failed = findings.iter().any(|site| site["check"] == *name);
            json!({"name": name, "state": "active", "execution": "ok",
                   "judgement": if failed { "fail" } else { "pass" }})
        })
        .collect()
}

fn measurements() -> Vec<Value> {
    GATES
        .iter()
        .map(|name| {
            json!({"check": name, "state": "complete", "holes": [],
                   "basis": {"producer": {"capability": name, "semantics_version": 1}}})
        })
        .collect()
}

fn stop(ago: u64, blocked: bool, findings: Vec<Value>, notes: Vec<Value>) -> Value {
    json!({
        "schema": 2,
        "version": "0.0.0",
        "kind": "stop",
        "time": now() - ago,
        "host": "claude",
        "session": "s-1",
        "prompt": 1,
        "hook": {"blocked": blocked, "delivery": "none", "gate_spent": true,
                 "build_blocks": 0, "blocked_before": false},
        "verdict": if blocked { "red" } else { "green" },
        "result": {
            "schema_version": 1,
            "command": "stop",
            "judgement": if blocked { "fail" } else { "pass" },
            "measurement": "complete",
            "execution": "ok",
            "exit": null,
            "capabilities": capabilities(&findings),
            "findings": findings,
            "reviews": [],
            "notes": notes,
            "measurements": measurements(),
        },
        "timing": {"total_ms": 20, "build_ms": 0, "lock_ms": 0, "klin_ms": 20},
        "asked": [],
        "flags": [],
        "told": [],
        "config_hash": "c-1",
        "notice": null,
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
    json!({"schema": 2, "version": "0.0.0", "kind": "prompt", "time": now() - ago,
           "session": "s-1", "prompt": 1, "text": text})
}

// The counted unit, and what keys it.

#[test]
fn a_blocked_stop_that_spent_no_gate_block_opens_no_regression() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut build = stop(400, true, vec![site.clone()], vec![]);
    build["hook"]["gate_block"] = Value::Null;
    let mut gate = stop(300, true, vec![site], vec![]);
    gate["hook"]["gate_block"] = json!(1);

    let without = tree(std::slice::from_ref(&build));
    let json = without.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 0, "{json}");

    let with = tree(&[build, gate]);
    let json = with.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 1, "{json}");
}

#[test]
fn one_id_over_four_blocked_stops_is_one_regression_with_its_latest_outcome() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut lines: Vec<Value> = (0..4)
        .map(|step| stop(400 - step * 10, true, vec![site.clone()], vec![]))
        .collect();
    lines.push(stop(300, false, vec![], vec![]));
    let tree = tree(&lines);

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 1, "{json}");
    assert_eq!(json["episodes"].as_array().map(Vec::len), Some(1), "{json}");
    assert_eq!(json["episodes"][0]["outcome"], "fixed-later", "{json}");

    let run = tree.run(&["report", "--since", "7d"]);
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

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
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

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
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

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 2, "{json}");
    assert_eq!(outcomes(&json, "fixed-next"), 1, "{json}");
    assert_eq!(json["counts"]["open"], 1, "{json}");
}

#[test]
fn a_gate_that_measured_nothing_never_makes_an_earlier_regression_read_as_fixed() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut errored = stop(300, false, vec![], vec![]);
    errored["result"]["capabilities"] =
        json!([{"name": "escapes", "state": "active", "execution": "error"}]);
    let mut nothing_ran = stop(250, true, vec![], vec![]);
    nothing_ran["result"]["capabilities"] = json!([]);

    let open = tree(&[stop(400, true, vec![site.clone()], vec![]), errored]);
    let json = open.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["open"], 1, "{json}");
    assert_eq!(outcomes(&json, "fixed-next"), 0, "{json}");

    let build = tree(&[stop(400, true, vec![site.clone()], vec![]), nothing_ran]);
    let json = build.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["open"], 1, "{json}");

    let later = tree(&[
        stop(400, true, vec![site], vec![]),
        {
            let mut line = stop(300, true, vec![], vec![]);
            line["result"]["capabilities"] = json!([]);
            line
        },
        stop(200, false, vec![], vec![]),
    ]);
    let json = later.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(outcomes(&json, "fixed-next"), 1, "{json}");
    assert_eq!(outcomes(&json, "fixed-later"), 0, "{json}");
}

/// Two measurements compare only under one semantics version, so a regression gone from a
/// measurement under another is not compared, and never a fix. Spec 8.3, 13.2.
#[test]
fn a_regression_gone_under_another_semantics_version_is_not_compared_and_not_fixed() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut later = stop(300, false, vec![], vec![]);
    for record in later["result"]["measurements"]
        .as_array_mut()
        .into_iter()
        .flatten()
    {
        record["basis"]["producer"]["semantics_version"] = json!(2);
    }
    let tree = tree(&[stop(400, true, vec![site], vec![]), later]);

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["not_compared"], 1, "{json}");
    assert_eq!(json["counts"]["fixed"], 0, "{json}");
    assert_eq!(json["regressions"][0]["state"], "not-compared", "{json}");

    let run = tree.run(&["report", "--since", "7d"]);
    assert!(
        run.says("1 regression went under a changed measurement, so klin did not compare it."),
        "{}",
        run.out
    );
    assert!(!run.says("fixed"), "{}", run.out);
}

/// A later measurement that still holds the site under the new semantics version makes that
/// version the one a fix compares with. Spec 8.3.
#[test]
fn a_regression_held_under_a_new_semantics_version_and_gone_under_it_is_fixed() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut held = stop(300, false, vec![site.clone()], vec![]);
    let mut gone = stop(200, false, vec![], vec![]);
    for line in [&mut held, &mut gone] {
        for record in line["result"]["measurements"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            record["basis"]["producer"]["semantics_version"] = json!(2);
        }
    }
    let tree = tree(&[stop(400, true, vec![site], vec![]), held, gone]);

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["fixed"], 1, "{json}");
    assert_eq!(json["counts"]["not_compared"], 0, "{json}");
}

#[test]
fn a_regression_that_goes_after_the_config_changed_is_not_reported_as_a_code_fix() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[
        stop(400, true, vec![site], vec![]),
        reconfigured(stop(300, false, vec![], vec![])),
    ]);

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(outcomes(&json, "config-changed"), 1, "{json}");
    assert_eq!(outcomes(&json, "fixed-next"), 0, "{json}");
    assert_eq!(json["episodes"][0]["config_changed"], true, "{json}");

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("klin caught 1 regression this week. 1 resolved after the config changed."),
        "{}",
        run.out
    );

    let all = tree.run(&["report", "--since", "7d", "--details"]);
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

    let run = tree.run(&["report", "--since", "7d"]);
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

    let run = tree.run(&["report", "--since", "7d"]);
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

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("4 regressions need your attention."),
        "{}",
        run.out
    );
    assert!(
        run.says("and 1 more · klin report --details"),
        "{}",
        run.out
    );
    assert_eq!(
        run.out.matches("unwrap()").count(),
        3,
        "three sites and no more: {}",
        run.out
    );

    let all = tree.run(&["report", "--since", "7d", "--details"]);
    assert!(
        !all.says("and 1 more · klin report --details"),
        "{}",
        all.out
    );
    assert!(all.says("src/f3.rs:4"), "{}", all.out);
}

#[test]
fn an_advisory_stop_sets_regressions_aside_and_never_calls_them_fixed_or_still_in_the_tree() {
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
        advisory(300),
        stop(200, false, vec![], vec![]),
    ]);

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(
            "2 regressions were set aside, because the window moved before klin judged it again."
        ),
        "{}",
        run.out
    );
    assert!(!run.says("needs your attention"), "{}", run.out);
    assert!(!run.says("still there"), "{}", run.out);
    assert!(!run.says("were fixed"), "{}", run.out);

    let all = tree.run(&["report", "--since", "7d", "--details"]);
    assert!(
        all.says("the history moved (incoming-commits), so klin blocked nothing"),
        "{}",
        all.out
    );

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["set_aside"], 2, "{json}");
    assert_eq!(json["counts"]["fixed"], 0, "{json}");
    assert_eq!(json["counts"]["caught"], 2, "{json}");
    assert_eq!(json["counts"]["advisory"], 1, "{json}");
    assert_eq!(json["regressions"][0]["state"], "set-aside", "{json}");
    assert_eq!(json["advisory"][0]["reason"], "incoming-commits", "{json}");
}

/// A Stop that wrote `unjudged` lets the next prompt move the stamp, so a regression still open
/// then is set aside and never fixed. Spec 6.6, 13.2.
#[test]
fn a_prompt_that_moves_an_unjudged_stamp_sets_open_regressions_aside() {
    let site = found("id-a", "escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let mut unjudged = stop(300, false, vec![], vec![]);
    unjudged["verdict"] = json!("unjudged");
    unjudged["result"]["capabilities"] = json!([]);
    let tree = tree(&[
        stop(400, true, vec![site], vec![]),
        unjudged,
        prompt_line(200, "go on"),
        stop(100, false, vec![], vec![]),
    ]);

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["counts"]["set_aside"], 1, "{json}");
    assert_eq!(json["counts"]["fixed"], 0, "{json}");
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
        advisory(400),
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

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let said = run.out.replace("\r\n", "\n");
    let open = said
        .find("1 regression needs your attention.")
        .unwrap_or_else(|| panic!("{said}"));
    let aside = said
        .find("1 more was set aside, because the window moved before klin judged it again.")
        .unwrap_or_else(|| panic!("{said}"));
    assert!(open < aside, "{said}");
}

/// The whole opening order, top pair first: a window klin did not measure whole says so above
/// the regressions it knows are open, and the set-aside uncertainty follows both.
#[test]
fn measurement_doubt_opens_the_report_above_the_open_regressions_it_knows_of() {
    let unparsed = coverage("src/odd.rs");
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
        advisory(400),
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

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let said = run.out.replace("\r\n", "\n");
    let at = |text: &str| {
        said.find(text)
            .unwrap_or_else(|| panic!("{text} missing from: {said}"))
    };
    let gap = at("Stats may be incomplete: 1 file wasn't measured.");
    let open = at("1 regression needs your attention.");
    let aside = at("1 more was set aside, because the window moved before klin judged it again.");
    assert!(gap < open && open < aside, "{said}");
    assert!(!run.says("Nothing needs your attention."), "{said}");
}

#[test]
fn measurement_doubt_outranks_the_value_story_and_forbids_nothing_needs_your_attention() {
    let unparsed = coverage("src/odd.rs");
    let other = coverage("src/odder.rs");
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

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("Stats may be incomplete: 2 files weren't measured."),
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
    let mut quiet = stop(300, false, vec![], vec![]);
    quiet["result"]["findings"] = json!([{
        "id": "lost-1", "check": null, "kind": "measurement-lost", "outcome": "held",
        "file": "src/gone.rs", "line": null, "text": "src/gone.rs",
        "values": {"reason": "parse"},
    }]);
    let tree = tree(&[quiet]);

    let run = tree.run(&["report", "--since", "7d"]);
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

    let all = tree.run(&["report", "--since", "7d", "--details"]);
    assert!(all.says("Measurement"), "{}", all.out);
    assert!(
        all.says("1 file(s) no capability measured: src/gone.rs"),
        "{}",
        all.out
    );
}

#[test]
fn a_journal_line_the_reader_cannot_take_lowers_confidence_and_fails_nothing() {
    let mut newer = stop(100, true, vec![], vec![]);
    newer["schema"] = json!(99);
    let tree = tree(&[stop(200, false, vec![], vec![]), newer]);

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("Stats may be incomplete: 1 journal record couldn't be read."),
        "{}",
        run.out
    );

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["skipped_lines"], 1, "{json}");
}

#[test]
fn a_measured_window_with_no_regression_says_none_were_found_and_nothing_else() {
    let quiet = tree(&[
        stop(200, false, vec![], vec![]),
        stop(100, false, vec![], vec![]),
    ]);
    let run = quiet.run(&["report", "--since", "7d"]);
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
    let first = fresh.run(&["report", "--since", "7d"]);
    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        first.says("klin is on. Your first recap appears after the agent finishes a task."),
        "{}",
        first.out
    );
}

#[test]
fn the_session_report_over_an_empty_journal_points_at_the_week() {
    let fresh = Tree::new();
    let run = fresh.run(&["report"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("The journal holds no session yet"), "{}", run.out);
    assert!(run.says("`klin report --since 7d`"), "{}", run.out);
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

    let run = tree.run(&["report", "--since", "7d"]);
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
            vec![note(
                "inventory",
                "deleted",
                "tests/pay.rs",
                20,
                "the test site refund_twice went in this window",
            )],
        ),
    ]);

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(outcomes(&json, "asked-once"), 1, "{json}");
    assert_eq!(json["counts"]["caught"], 1, "{json}");
    assert_eq!(outcomes(&json, "fixed-next"), 1, "{json}");

    let run = tree.run(&["report", "--since", "7d"]);
    assert!(
        run.says("klin caught 1 regression this week. It was fixed after klin flagged it."),
        "{}",
        run.out
    );

    let all = tree.run(&["report", "--since", "7d", "--details"]);
    assert!(
        all.says("a test deleted from tests/pay.rs:20, fn refund_twice(). The agent said why."),
        "{}",
        all.out
    );
}

#[test]
fn a_guard_deny_asks_the_person_nothing_and_a_guard_ask_does() {
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
    ]);

    let all = tree.run(&["report", "--since", "7d", "--details"]);
    assert_eq!(all.code, 0, "{}", all.out);
    assert!(all.says("klin refused an edit to klin.json"), "{}", all.out);
    assert!(
        all.says("klin asked you before a command that named klin's own state"),
        "{}",
        all.out
    );
    assert!(
        !all.says("klin asked you before an edit to klin.json"),
        "{}",
        all.out
    );

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
    let kinds: Vec<&str> = json["audit"]
        .as_array()
        .unwrap_or_else(|| panic!("{json}"))
        .iter()
        .filter_map(|one| one["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["guard", "guard"], "{json}");
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
        let all = tree.run(&["report", "--since", "7d", "--details"]);
        assert_eq!(all.code, 0, "{}", all.out);
        assert!(
            !all.says(&format!("  1 {gate}\n")),
            "{gate} printed its own name for want of a label: {}",
            all.out
        );
        assert!(all.says("  1 "), "{gate}: {}", all.out);
    }
}

/// Every check the catalogue holds, read off the hole the runner prints when a written
/// configuration names no gate over a tree the survey finds nothing in.
fn catalogue() -> Vec<String> {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let run = tree.run(&["check"]);
    assert_eq!(run.code, 3, "{}", run.out);
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

    let run = tree.run(&["report", "--since", "7d"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("src/new.rs:7  a site"), "{}", run.out);

    let all = tree.run(&["report", "--since", "7d", "--details"]);
    assert!(all.says("1 a-gate-from-the-future"), "{}", all.out);

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
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

    let all = tree.run(&["report", "--since", "7d", "--details"]);
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

    let json = tree.run(&["report", "--since", "7d", "--json"]).json();
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
    assert_eq!(outcomes(&json, "fixed-next"), 1, "{json}");
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
    let run = tree.run(&["report", "--since", "a-week"]);
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

    let week = tree.run(&["report", "--since", "7d"]);
    assert_eq!(week.code, 0, "{}", week.out);
    assert!(
        week.says("No regressions were found this week."),
        "{}",
        week.out
    );

    let month = tree.run(&["report", "--since", "30d"]);
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
    json!({"schema": 2, "version": "0.0.0", "kind": "guard", "time": now() - ago,
           "session": "s-1", "decision": decision, "reason": reason})
}

/// An advisory Stop as one that took a fresh stamp after the history moved records it: it
/// measured every check and found nothing, which proves no fix, because other people's commits
/// came into the window it measured. Spec 6.6, 13.1, 13.2.
fn advisory(ago: u64) -> Value {
    let mut line = stop(ago, false, vec![], vec![]);
    line["verdict"] = json!("advisory");
    line["advisory"] = json!("incoming-commits");
    line
}

/// The coverage note of a file a capability did not measure. Spec 7.2.
fn coverage(file: &str) -> Value {
    json!({"check": "complexity", "kind": "parse", "coverage": true, "file": file,
           "message": "no grammar reads it"})
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

    let run = tree.run(&["report"]);
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
    harness::feed(tree.root(), harness::AGENT, event)
}

fn prompt(tree: &Tree) {
    let run = harness::feed(tree.root(), harness::AGENT, A_PROMPT);
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
fn a_site_that_spends_both_gate_blocks_is_one_regression() {
    let tree = hooked();
    blocked(&tree);
    tree.write("src/other.rs", "pub fn g() -> i32 {\n    1\n}\n");
    let again = hook(&tree, A_SECOND_STOP);
    assert_eq!(again.code, 2, "{}", again.out);
    assert!(again.says("gate block 2 of 2"), "{}", again.out);

    let turn = tree.run(&["report"]);
    assert_eq!(turn.code, 0, "{}", turn.out);
    assert!(
        turn.says("1 regression needs your attention."),
        "{}",
        turn.out
    );
    assert!(turn.says("klin caught 1 this session."), "{}", turn.out);
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
        "1 regression still needs your attention. `klin report` shows it.",
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

const SUITE: &str = "#[test]\nfn alpha() {\n    assert!(true);\n}\n\n#[test]\nfn beta() {\n    assert!(1 == 1);\n}\n";
const ONE_TEST: &str = "#[test]\nfn alpha() {\n    assert!(true);\n}\n";

/// A base that holds a suite of two tests, a prompt, one test deleted, and the stop that asks
/// about it.
fn asked_about_a_deleted_test() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", HOOKED);
    tree.write("src/lib.rs", CLEAN);
    tree.write("tests/suite.rs", SUITE);
    tree.base();
    prompt(&tree);
    tree.write("tests/suite.rs", ONE_TEST);
    let asked = hook(&tree, A_STOP);
    assert_eq!(asked.code, 2, "{}", asked.out);
    tree
}

#[test]
fn a_deleted_test_klin_let_through_after_asking_counts_only_as_asked_once() {
    let tree = asked_about_a_deleted_test();
    let through = hook(&tree, A_SECOND_STOP);
    assert_eq!(through.code, 0, "{}", through.out);

    let json = tree.run(&["report", "--json"]).json();
    assert_eq!(outcomes(&json, "asked-once"), 1, "{json}");
    assert_eq!(json["counts"]["caught"], 0, "{json}");
    assert_eq!(outcomes(&json, "fixed-next"), 0, "{json}");
    assert_eq!(outcomes(&json, "fixed-later"), 0, "{json}");
}

#[test]
fn the_stop_that_lets_a_deleted_test_through_says_no_regression_was_fixed() {
    let tree = asked_about_a_deleted_test();
    let through = hook(&tree, A_SECOND_STOP);
    assert_eq!(through.code, 0, "{}", through.out);

    let said = told(&through);
    assert!(said.contains("tests/suite.rs:7  fn beta() {"), "{said}");
    assert!(!said.contains("regression"), "{said}");
    assert!(!said.contains("fixed"), "{said}");
}

#[test]
fn a_deleted_test_restored_after_the_block_counts_as_caught_and_fixed_next() {
    let tree = asked_about_a_deleted_test();
    tree.write("tests/suite.rs", SUITE);
    let restored = hook(&tree, A_SECOND_STOP);
    assert_eq!(restored.code, 0, "{}", restored.out);
    assert_eq!(
        told(&restored),
        "klin caught 1 regression this turn. It was fixed after klin flagged it.",
        "{}",
        restored.out
    );

    let json = tree.run(&["report", "--json"]).json();
    assert_eq!(json["counts"]["caught"], 1, "{json}");
    assert_eq!(outcomes(&json, "fixed-next"), 1, "{json}");
    assert_eq!(outcomes(&json, "asked-once"), 0, "{json}");
}

#[test]
fn a_deleted_test_whose_file_went_after_klin_asked_is_not_a_fixed_regression() {
    let tree = asked_about_a_deleted_test();
    tree.remove("tests/suite.rs");
    let file = hook(&tree, A_SECOND_STOP);
    assert_eq!(file.code, 2, "{}", file.out);
    let through = hook(&tree, A_SECOND_STOP);
    assert_eq!(through.code, 0, "{}", through.out);
    assert!(!told(&through).contains("regression"), "{}", through.out);

    let json = tree.run(&["report", "--json"]).json();
    assert_eq!(outcomes(&json, "asked-once"), 2, "{json}");
    assert_eq!(json["counts"]["caught"], 0, "{json}");
    assert_eq!(outcomes(&json, "fixed-next"), 0, "{json}");
}

fn journal_lines(tree: &Tree) -> usize {
    std::fs::read_to_string(tree.state("journal.jsonl"))
        .unwrap_or_default()
        .lines()
        .count()
}

/// A turn that changed nothing tells the person nothing, even where the window still holds
/// earlier work that leaves a note, and it still writes its verdict and its journal line.
/// Spec 10.7.
#[test]
fn a_stop_over_the_tree_its_prompt_saw_tells_nothing_and_still_writes_its_verdict() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"escapes": {"in": "README.md"}}"#);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    prompt(&tree);
    tree.write("src/work.rs", CLEAN);
    let edited = hook(&tree, A_STOP);
    assert_eq!(edited.code, 0, "{}", edited.out);
    assert!(told(&edited).contains("left a note"), "{}", edited.out);

    prompt(&tree);
    let before = journal_lines(&tree);
    let quiet = hook(&tree, A_STOP);

    assert_eq!(quiet.code, 0, "{}", quiet.out);
    assert_eq!(told(&quiet), "", "{}", quiet.out);
    assert_ne!(tree.field("verdict"), "aborted", "{}", quiet.out);
    assert_eq!(journal_lines(&tree), before + 1, "{}", quiet.out);

    tree.write("src/more.rs", CLEAN);
    let changed = hook(&tree, A_STOP);
    assert!(told(&changed).contains("left a note"), "{}", changed.out);
}

/// After a gate block, a turn that changed nothing repeats no turn-end line: the verdict stays
/// red, so the next turn that changes a file tells the person again. Spec 10.7.
#[test]
fn a_turn_that_changed_nothing_after_a_gate_block_sends_no_turn_end_line() {
    let tree = hooked();
    blocked(&tree);
    let through = hook(&tree, A_SECOND_STOP);
    assert!(told(&through).contains("still need"), "{}", through.out);

    prompt(&tree);
    assert_eq!(hook(&tree, A_STOP).code, 2);
    let quiet = hook(&tree, A_SECOND_STOP);

    assert_eq!(quiet.code, 0, "{}", quiet.out);
    assert_eq!(told(&quiet), "", "{}", quiet.out);
    assert!(quiet.says("changed: 1 file(s)"), "{}", quiet.out);
    assert_eq!(tree.field("verdict"), "red", "{}", quiet.out);
}

/// A turn that changed a file, took a block, and changed it back still tells, though its last
/// block saw the tree its prompt saw. Spec 10.7.
#[test]
fn a_turn_that_changed_a_file_back_after_a_block_still_tells() {
    let tree = hooked();
    blocked(&tree);
    assert_eq!(hook(&tree, A_SECOND_STOP).code, 0);

    prompt(&tree);
    tree.write("src/other.rs", &an_escape());
    assert_eq!(hook(&tree, A_STOP).code, 2);
    tree.remove("src/other.rs");
    assert_eq!(hook(&tree, A_SECOND_STOP).code, 2);
    let through = hook(&tree, A_SECOND_STOP);

    assert_eq!(through.code, 0, "{}", through.out);
    assert_ne!(told(&through), "", "{}", through.out);
}

/// A library crate with `lib` as its root, hooked and committed as the base.
fn library(lib: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"build": [], "escapes": {"in": "src"}}"#);
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    tree.write("src/lib.rs", lib);
    tree.base();
    tree
}

#[test]
fn a_red_pass_through_whose_open_regressions_are_all_public_api_names_the_breaks_not_a_count() {
    let tree = library("pub fn parse() {}\npub fn render() {}\n");
    prompt(&tree);
    tree.write("src/lib.rs", "pub fn kept() {}\n");
    assert_eq!(hook(&tree, A_STOP).code, 2);

    let through = hook(&tree, A_SECOND_STOP);
    let counted = tree.run(&["report"]);

    assert_eq!(through.code, 0, "{}", through.out);
    assert_eq!(
        told(&through),
        "Public API compatibility breaks still need your attention. `klin report` shows them.",
        "{}",
        through.out
    );
    assert!(
        counted.says("2 regressions need your attention."),
        "{}",
        counted.out
    );

    let mixed = library("pub fn parse() {}\n");
    prompt(&mixed);
    mixed.write("src/lib.rs", &an_escape());
    assert_eq!(hook(&mixed, A_STOP).code, 2);
    let through = hook(&mixed, A_SECOND_STOP);
    assert_eq!(
        told(&through),
        "2 regressions still need your attention. `klin report` shows them.",
        "{}",
        through.out
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
             flagged it. `klin report --since 7d` shows them."
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
             flagged it. `klin report --since 7d` shows them."
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
            vec![note(
                "inventory",
                "deleted",
                "tests/test_two.py",
                0,
                "the test file went in this window",
            )],
        ),
    ]);

    let all = tree.run(&["report", "--since", "7d", "--details"]);
    assert_eq!(all.code, 0, "{}", all.out);
    assert!(
        all.says("tests/test_two.py deleted. The agent said why."),
        "{}",
        all.out
    );
    assert_eq!(
        outcomes(
            &tree.run(&["report", "--since", "7d", "--json"]).json(),
            "asked-once"
        ),
        1
    );
}

/// The facts the default report stopped printing are still facts. `--json` keeps klin's own
/// time, which is the `klin_ms` of spec 13.1 and never the project's build.
#[test]
fn json_keeps_klins_own_time_the_default_no_longer_prints() {
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
    let json = both.run(&["report", "--since", "7d", "--json"]).json();
    assert_eq!(json["activity"]["klin_ms"], 3_000, "{json}");

    let run = both.run(&["report", "--since", "7d"]);
    assert!(!run.says("Last week"), "{}", run.out);
    assert!(!run.says("seconds in total"), "{}", run.out);
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

    let run = tree.run(&["report"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("src/new.rs:1  todo!()"), "{}", run.out);
    assert!(!run.says("src/old.rs"), "{}", run.out);
}

/// A journal carrying no session id cannot place the default session scope, so the report says
/// so instead of reporting a quiet session.
#[test]
fn a_session_scope_over_a_journal_with_no_session_id_says_it_could_not_place_it() {
    let mut line = stop(300, false, vec![], vec![]);
    line["session"] = json!(null);
    let tree = tree(&[line]);

    let run = tree.run(&["report"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("holds no session"), "{}", run.out);
    assert!(!run.says("No regressions were found"), "{}", run.out);
}
