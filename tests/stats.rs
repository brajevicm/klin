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
        "exit": if blocked { 2 } else { 0 },
    })
}

const UNWRAP: &str = "Handle the error, or accept it in klin.json, before you push.";

/// The prompt line the stops of `stop` ran under, with the excerpt spec 11.4 records.
fn prompt_line(ago: u64, text: &str) -> Value {
    json!({"schema": 1, "version": "0.0.0", "kind": "prompt", "time": now() - ago,
           "session": "s-1", "prompt": 1, "text": text})
}

#[test]
fn a_block_and_a_green_stop_after_it_read_as_one_shortcut_the_agent_fixed() {
    let tree = tree(&[
        prompt_line(300, "Fix the refund flow"),
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
    assert!(run.says("The agent fixed it on its own."), "{}", run.out);
    assert!(!run.says("Still there"), "{}", run.out);
    assert!(run.says("Fixed after klin asked"), "{}", run.out);
    assert!(
        run.says(r#"unwrap() in src/io.rs:12, while you asked for "Fix the refund flow""#),
        "{}",
        run.out
    );
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
    assert!(run.says("The agent asked you once."), "{}", run.out);
    assert!(
        run.says("a test deleted from tests/pay.rs:20, refund_twice. The agent said why."),
        "{}",
        run.out
    );
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
    assert_eq!(json["counts"]["fixed-next"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-later"], 0, "{json}");
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

// #156: the turn and the session, what the person was asked, last week, and the turn end.

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
    let earlier = finding("escapes", "src/old.rs", 3, "unwrap()", UNWRAP);
    let newer = finding("stubs", "src/pay.rs", 41, "todo!()", "Do the work.");
    let tree = tree(&[
        in_session(stop(400, true, vec![earlier], vec![]), "s-1"),
        in_session(stop(300, false, vec![], vec![]), "s-1"),
        in_session(stop(200, true, vec![newer.clone()], vec![]), "s-2"),
        reset(150),
        in_session(stop(100, false, vec![newer], vec![]), "s-2"),
    ]);

    let run = tree.run(&["stats", "--session"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("klin, this session in this repository"),
        "{}",
        run.out
    );
    assert!(run.says("klin caught 1 shortcut."), "{}", run.out);
    assert!(run.says("todo!() in src/pay.rs:41"), "{}", run.out);
    assert!(!run.says("src/old.rs"), "{}", run.out);
    assert!(run.says("klin ran 2 times"), "{}", run.out);

    let json = tree.run(&["stats", "--session", "--json"]).json();
    assert_eq!(json["counts"]["reset"], 1, "{json}");
}

#[test]
fn a_guard_deny_a_reset_and_a_deleted_test_each_read_as_a_sentence_under_you_were_asked() {
    let tree = tree(&[
        guard(500, "deny", "config-write"),
        reset(400),
        stop(
            200,
            true,
            vec![finding(
                "inventory",
                "tests/pay.rs",
                20,
                "fn refund_twice() {",
                "",
            )],
            vec![],
        ),
        stop(
            100,
            false,
            vec![],
            vec![
                json!({"gate": "inventory", "outcome": "deleted", "file": "tests/pay.rs",
                        "line": 20, "text": "the test site went in this window"}),
            ],
        ),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("You were asked"), "{}", run.out);
    assert!(run.says("klin refused an edit to klin.json"), "{}", run.out);
    assert!(run.says("You told klin to start over."), "{}", run.out);
    assert!(
        run.says("a test deleted from tests/pay.rs:20, fn refund_twice(). The agent said why."),
        "{}",
        run.out
    );
    assert!(run.says("The agent asked you once."), "{}", run.out);
    assert!(!run.says("You started the judgment over"), "{}", run.out);

    let json = tree.run(&["stats", "--json"]).json();
    let kinds: Vec<&str> = json["asked"]
        .as_array()
        .unwrap_or_else(|| panic!("{json}"))
        .iter()
        .filter_map(|asked| asked["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["asked-once", "reset", "guard"], "{json}");
    assert_eq!(json["asked"][2]["decision"], "deny", "{json}");
    assert_eq!(json["asked"][2]["reason"], "config-write", "{json}");
}

#[test]
fn a_deleted_test_file_reads_as_the_file_deleted() {
    let tree = tree(&[
        stop(
            200,
            true,
            vec![finding("inventory", "tests/test_two.py", 0, "", "")],
            vec![],
        ),
        stop(
            100,
            false,
            vec![],
            vec![
                json!({"gate": "inventory", "outcome": "deleted", "file": "tests/test_two.py",
                        "line": 0, "text": "the test file went in this window"}),
            ],
        ),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("tests/test_two.py deleted. The agent said why."),
        "{}",
        run.out
    );
}

#[test]
fn a_journal_holding_two_full_weeks_compares_them_and_one_holding_one_does_not() {
    let site = finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let other = finding("stubs", "src/pay.rs", 41, "todo!()", "Do the work.");
    let this_week = [
        stop(2 * DAY, true, vec![site.clone()], vec![]),
        stop(2 * DAY - 60, false, vec![], vec![]),
    ];
    let mut two = vec![
        stop(15 * DAY, false, vec![], vec![]),
        stop(10 * DAY, true, vec![site.clone(), other.clone()], vec![]),
        stop(10 * DAY - 60, false, vec![other], vec![]),
    ];
    two.extend(this_week.iter().cloned());

    let both = tree(&two);
    let run = both.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("Last week: 2 shortcuts, 1 left open. This week is better."),
        "{}",
        run.out
    );
    let json = both.run(&["stats", "--json"]).json();
    assert_eq!(json["earlier"], json!({"caught": 2, "open": 1}), "{json}");

    let alone = tree(&this_week);
    let one = alone.run(&["stats"]);
    assert_eq!(one.code, 0, "{}", one.out);
    assert!(!one.says("Last week"), "{}", one.out);
    assert_eq!(
        alone.run(&["stats", "--json"]).json()["earlier"],
        Value::Null
    );
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
fn turn_reports_the_stops_since_the_stamp_and_after_a_reset_only_the_stops_after_it() {
    let tree = hooked();
    blocked(&tree);

    let before = tree.run(&["stats", "--turn"]);
    assert_eq!(before.code, 0, "{}", before.out);
    assert!(
        before.says("klin, this turn in this repository"),
        "{}",
        before.out
    );
    assert!(before.says("klin caught 1 shortcut."), "{}", before.out);

    let reset = tree.run(&["turn", "reset"]);
    assert_eq!(reset.code, 0, "{}", reset.out);
    let after = hook(&tree, A_SECOND_STOP);
    assert_eq!(after.code, 0, "{}", after.out);

    let turn = tree.run(&["stats", "--turn"]);
    assert!(
        turn.says("klin caught no shortcuts. klin ran once and asked nothing."),
        "{}",
        turn.out
    );
    let week = tree.run(&["stats", "--json"]).json();
    assert_eq!(week["counts"]["reset"], 1, "{week}");
    let report = tree.run(&["stats"]);
    assert!(report.says("You set aside 1 shortcut."), "{}", report.out);
    assert!(
        report.says(r#"src/lib.rs:2, while you asked for "Fix the refund flow""#),
        "{}",
        report.out
    );
    assert!(
        report.says("If it's still there, fix it, or accept it in `klin.json`, before you push."),
        "{}",
        report.out
    );
}

#[test]
fn a_fix_in_a_later_prompt_of_the_same_turn_still_tells_the_count_fixed() {
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
        "klin: the agent took 1 shortcut this turn and fixed it after klin asked.",
        "{}",
        green.out
    );
}

#[test]
fn a_green_stop_after_a_block_tells_the_count_fixed_and_one_with_no_block_tells_nothing() {
    let tree = hooked();
    blocked(&tree);
    tree.write("src/lib.rs", CLEAN);

    let green = hook(&tree, A_SECOND_STOP);
    assert_eq!(green.code, 0, "{}", green.out);
    assert_eq!(
        told(&green),
        "klin: the agent took 1 shortcut this turn and fixed it after klin asked.",
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
fn a_red_pass_through_tells_the_person_one_shortcut_is_still_there() {
    let tree = hooked();
    blocked(&tree);

    let through = hook(&tree, A_SECOND_STOP);
    assert_eq!(through.code, 0, "{}", through.out);
    assert_eq!(
        told(&through),
        "klin: one shortcut is still there. `klin stats --turn` names it.",
        "{}",
        through.out
    );
}

// #164: the report reads the record the way spec 11.5 says.

fn timed(mut line: Value, total_ms: u64, build_ms: u64) -> Value {
    line["timing"] = json!({"total_ms": total_ms, "build_ms": build_ms, "lock_ms": 0,
                            "klin_ms": total_ms - build_ms});
    line
}

#[test]
fn the_cost_line_counts_klins_own_time_and_not_the_build() {
    let site = finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[
        timed(stop(200, true, vec![site], vec![]), 9_000, 7_000),
        timed(stop(100, false, vec![], vec![]), 9_000, 8_000),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("klin ran 2 times and took 3 seconds in total."),
        "{}",
        run.out
    );
}

#[test]
fn a_reset_and_a_guard_refusal_ask_the_person_nothing_and_still_print_under_you_were_asked() {
    let site = finding("escapes", "src/io.rs", 12, "unwrap()", UNWRAP);
    let tree = tree(&[
        guard(400, "deny", "config-write"),
        stop(300, true, vec![site], vec![]),
        reset(200),
        stop(100, false, vec![], vec![]),
    ]);

    let run = tree.run(&["stats"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("klin caught 1 shortcut."), "{}", run.out);
    assert!(!run.says("asked you"), "{}", run.out);
    assert!(run.says("You were asked"), "{}", run.out);
    assert!(run.says("klin refused an edit to klin.json"), "{}", run.out);
    assert!(run.says("You set aside 1 shortcut."), "{}", run.out);
}

#[test]
fn a_deleted_test_is_matched_by_its_site_so_the_one_restored_beside_it_reads_as_fixed() {
    let tree = tree(&[
        stop(
            200,
            true,
            vec![
                finding("inventory", "tests/pay.rs", 20, "fn refund_twice() {", ""),
                finding("inventory", "tests/pay.rs", 40, "fn refund_once() {", ""),
            ],
            vec![],
        ),
        stop(
            100,
            false,
            vec![],
            vec![
                json!({"gate": "inventory", "outcome": "deleted", "file": "tests/pay.rs",
                        "line": 20, "text": "the test site went in this window"}),
            ],
        ),
    ]);

    let json = tree.run(&["stats", "--json"]).json();
    assert_eq!(json["counts"]["asked-once"], 1, "{json}");
    assert_eq!(json["counts"]["fixed-next"], 1, "{json}");
    assert_eq!(json["asked"][0]["line"], 20, "{json}");
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
    let old = finding("escapes", "src/old.rs", 1, "unwrap()", UNWRAP);
    let new = finding("stubs", "src/new.rs", 1, "todo!()", "Do the work.");
    journal(
        &tree,
        &[
            when(stop(0, true, vec![old], vec![]), taken - 1),
            when(stop(0, true, vec![new], vec![]), taken),
        ],
    );

    let run = tree.run(&["stats", "--turn"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("todo!() in src/new.rs:1"), "{}", run.out);
    assert!(!run.says("src/old.rs"), "{}", run.out);
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
        told(&first).starts_with("klin: the agent took 1 shortcut this turn"),
        "{}",
        first.out
    );
    assert!(
        told(&first).ends_with(
            "\nIn the last seven days, klin caught 1 shortcut and the agent fixed it on its own. \
             `klin stats` lists them."
        ),
        "{}",
        first.out
    );

    blocked(&tree);
    tree.write("src/lib.rs", CLEAN);
    let next = hook(&tree, A_SECOND_STOP);
    assert_eq!(next.code, 0, "{}", next.out);
    assert!(
        told(&next).starts_with("klin: the agent took"),
        "{}",
        next.out
    );
    assert!(
        !told(&next).contains("In the last seven days"),
        "{}",
        next.out
    );
}
