mod harness;

use harness::Tree;
use serde_json::{Value, json};

const DAY: u64 = 86_400;

fn tree(lines: &[Value]) -> Tree {
    let tree = Tree::new();
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
    tree
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

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
const GATES: [&str; 4] = ["escapes", "stubs", "inventory", "a-gate-from-the-future"];

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
        "exit": if blocked { 1 } else { 0 },
    })
}

const UNWRAP: &str = "Handle the error, or accept it in klin.json, before you push.";

#[test]
fn a_block_and_a_green_stop_after_it_read_as_one_shortcut_the_agent_fixed() {
    let tree = tree(&[
        stop(
            200,
            true,
            vec![finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP)],
            vec![],
        ),
        stop(100, false, vec![], vec![]),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("klin, this week in this repository"),
        "{}",
        run.out
    );
    assert!(run.says("klin caught 1 shortcut."), "{}", run.out);
    assert!(
        run.says("The agent fixed 1 of them before you saw them."),
        "{}",
        run.out
    );
    assert!(!run.says("Still there"), "{}", run.out);
    assert!(run.says("Fixed after klin asked"), "{}", run.out);
    assert!(run.says("unwrap() in src/io.rs:12"), "{}", run.out);
    assert!(run.says("klin ran 2 times"), "{}", run.out);
}

#[test]
fn a_block_and_a_red_pass_through_leave_the_finding_open_with_its_remedy() {
    let site = finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[
        stop(2 * DAY, true, vec![site.clone()], vec![]),
        stop(2 * DAY - 60, false, vec![site], vec![]),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("One is still there."), "{}", run.out);
    assert!(run.says("Still there"), "{}", run.out);
    assert!(run.says("unwrap() in src/io.rs:12, left on"), "{}", run.out);
    assert!(run.says(UNWRAP), "{}", run.out);
}

#[test]
fn a_deleted_test_klin_let_through_reads_as_an_ask_and_never_as_a_fix() {
    let tree = tree(&[
        stop(
            200,
            true,
            vec![finding(
                "inventory",
                "tests/pay.rs",
                20,
                "refund_twice",
                "Restore the test.",
            )],
            vec![],
        ),
        stop(
            100,
            false,
            vec![],
            vec![json!({
                "gate": "inventory",
                "outcome": "deleted",
                "file": "tests/pay.rs",
                "line": 20,
                "text": "the test site refund_twice in tests/pay.rs went in this window",
            })],
        ),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("You were asked"), "{}", run.out);
    assert!(run.says("klin asked you about 1 shortcut."), "{}", run.out);
    assert!(!run.says("Fixed after klin asked"), "{}", run.out);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["asked-once"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 0, "{json}");
}

#[test]
fn a_gate_klin_has_no_check_for_is_read_and_printed_like_any_other() {
    let tree = tree(&[
        stop(
            200,
            true,
            vec![finding(
                "a-gate-from-the-future",
                "src/new.rs",
                7,
                "a site",
                "Fix it before you push.",
            )],
            vec![],
        ),
        stop(100, false, vec![], vec![]),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("Fixed after klin asked"), "{}", run.out);
    assert!(run.says("a site in src/new.rs:7"), "{}", run.out);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(
        json["episodes"][0]["gate"], "a-gate-from-the-future",
        "{json}"
    );
}

#[test]
fn json_prints_one_episode_per_intervention_with_its_gate_and_its_outcome() {
    let open = finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[
        stop(
            3 * DAY,
            true,
            vec![
                open.clone(),
                finding("stubs", "src/pay.rs", 41, "todo!()", "Do the work."),
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
    assert_eq!(episodes.len(), 3, "{json}");
    assert_eq!(json["counts"]["caught"], 3, "{json}");
    assert_eq!(json["counts"]["open"], 2, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 1, "{json}");
    assert_eq!(json["stops"], 3, "{json}");
    let stubs = episodes
        .iter()
        .find(|episode| episode["gate"] == "stubs")
        .unwrap_or_else(|| panic!("{json}"));
    assert_eq!(stubs["outcome"], "fixed-next", "{json}");
    assert_eq!(stubs["file"], "src/pay.rs", "{json}");
}

#[test]
fn a_window_with_no_interventions_says_what_klin_did_and_an_empty_journal_says_it_started() {
    let quiet = tree(&[
        stop(200, false, vec![], vec![]),
        stop(100, false, vec![], vec![]),
    ]);
    let run = quiet.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("klin caught no shortcuts. klin ran 2 times and asked nothing."),
        "{}",
        run.out
    );

    let fresh = Tree::new();
    let first = fresh.run(&["stats"]);
    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        first.says("klin started watching today. Come back after a few turns."),
        "{}",
        first.out
    );
}

#[test]
fn a_line_from_a_newer_schema_is_skipped_counted_and_fails_nothing() {
    let mut newer = stop(100, true, vec![], vec![]);
    newer["schema"] = json!(2);
    let tree = tree(&[stop(200, false, vec![], vec![]), newer]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("Measurement"), "{}", run.out);
    assert!(
        run.says("klin skipped 1 journal line(s) it does not understand."),
        "{}",
        run.out
    );

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["skipped"], 1, "{json}");
    assert_eq!(json["stops"], 1, "{json}");
}

#[test]
fn since_widens_the_window_and_the_title_says_which_one_it_is() {
    let tree = tree(&[
        stop(
            20 * DAY,
            true,
            vec![finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP)],
            vec![],
        ),
        stop(19 * DAY, false, vec![], vec![]),
    ]);

    let week = tree.run(&["stats"]);
    assert_eq!(week.code, 0, "{}", week.out);
    assert!(week.says("klin caught no shortcuts"), "{}", week.out);

    let month = tree.run(&["stats", "--since", "30d"]);
    assert_eq!(month.code, 0, "{}", month.out);
    assert!(
        month.says("klin, this month in this repository"),
        "{}",
        month.out
    );
    assert!(month.says("klin caught 1 shortcut."), "{}", month.out);
}

#[test]
fn all_lifts_the_cap_of_five_items_in_a_group() {
    let mut lines = Vec::new();
    for index in 0..7 {
        lines.push(stop(
            (700 - index * 10) as u64,
            true,
            vec![finding(
                "escapes",
                &format!("src/f{index}.rs"),
                1,
                "unwrap()",
                UNWRAP,
            )],
            vec![],
        ));
        lines.push(stop((695 - index * 10) as u64, false, vec![], vec![]));
    }
    let tree = tree(&lines);

    let capped = tree.run(&["stats"]);
    assert!(
        capped.says("and 2 more. klin stats --all"),
        "{}",
        capped.out
    );
    let all = tree.run(&["stats", "--all"]);
    assert!(!all.says("klin stats --all"), "{}", all.out);
    assert!(all.says("src/f6.rs"), "{}", all.out);
}

#[test]
fn a_since_that_is_not_a_number_of_days_is_a_usage_error() {
    let tree = tree(&[stop(100, false, vec![], vec![])]);
    let run = tree.run(&["stats", "--since", "a-week"]);
    assert_eq!(run.code, 2, "{}", run.out);
}

/// A stop on a tree that does not build: no gate ran, so no gate row says anything.
fn a_build_failure(ago: u64) -> Value {
    let mut line = stop(ago, true, vec![], vec![]);
    line["gates"] = json!([]);
    line
}

#[test]
fn a_gate_that_did_not_run_is_no_answer_and_never_reads_as_a_fix() {
    let site = finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let open = tree(&[
        stop(300, true, vec![site.clone()], vec![]),
        a_build_failure(200),
    ]);
    let json = open.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["open"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 0, "{json}");

    let later = tree(&[
        stop(300, true, vec![site], vec![]),
        a_build_failure(200),
        stop(100, false, vec![], vec![]),
    ]);
    let json = later.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["fixed-later"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 0, "{json}");
}

#[test]
fn one_file_no_stop_could_read_is_counted_once_however_many_stops_saw_it() {
    let mut lines = Vec::new();
    for index in 0..4 {
        lines.push(stop(
            400 - index * 10,
            false,
            vec![],
            vec![json!({
                "gate": "complexity",
                "outcome": "unparsed",
                "file": "src/odd.rs",
                "text": "no grammar reads it",
            })],
        ));
    }
    let tree = tree(&lines);

    let run = tree.run(&["stats"]);
    assert!(
        run.says("klin could not read 1 file(s) in this window."),
        "{}",
        run.out
    );
    assert_eq!(tree.run(&["stats", "--json"]).json()["unreadable"], 1);
}
