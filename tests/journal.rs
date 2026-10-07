mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const EVERY_GATE: &str = r#"{
  "doc_size": {"README.md": 10},
  "escapes": { "in": "src" },
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false,
                         "session_id": "s-1"}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true,
                                "session_id": "s-1"}"#;
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-1"}"#;
const A_SECOND_STOP_WITHOUT_SESSION: &str =
    r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;

fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

fn stop(tree: &Tree, event: &str) -> harness::Run {
    harness::feed(tree.root(), &["gate", "--hook"], event)
}

fn prompt(tree: &Tree) {
    let run = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
}

/// Every line of the journal, spec 11.4's record.
fn journal(tree: &Tree) -> Vec<Value> {
    let text = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|why| panic!("{why}: {line}")))
        .collect()
}

/// Only the stop lines, for a test that does not care about the prompt line `prompt(&tree)`
/// also appends.
fn stops(tree: &Tree) -> Vec<Value> {
    journal(tree)
        .into_iter()
        .filter(|line| field(line, &["kind"]) == "stop")
        .collect()
}

fn field<'a>(line: &'a Value, path: &[&str]) -> &'a Value {
    let mut held = line;
    for key in path {
        held = held
            .get(key)
            .unwrap_or_else(|| panic!("no {key} in {line}"));
    }
    held
}

fn has_flag(line: &Value, wanted: &str) -> bool {
    line.get("flags")
        .and_then(Value::as_array)
        .is_some_and(|flags| flags.iter().any(|flag| flag == wanted))
}

#[test]
fn a_stop_appends_one_line_holding_the_record_and_what_the_hook_knew() {
    let tree = tree(EVERY_GATE);
    prompt(&tree);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    let lines = stops(&tree);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(field(line, &["schema"]), 1, "{line}");
    assert_eq!(field(line, &["kind"]), "stop", "{line}");
    assert_eq!(field(line, &["host"]), "claude", "{line}");
    assert_eq!(field(line, &["session"]), "s-1", "{line}");
    assert_eq!(field(line, &["prompt"]), 1, "{line}");
    assert_eq!(field(line, &["verdict"]), "red", "{line}");
    assert_eq!(field(line, &["hook", "blocked"]), true, "{line}");
    assert_eq!(field(line, &["hook", "delivery"]), "block", "{line}");
    assert_eq!(field(line, &["hook", "blocked_before"]), false, "{line}");
    assert_eq!(field(line, &["hook", "build_blocks"]), 0, "{line}");
    assert_eq!(field(line, &["window", "kind"]), "turn", "{line}");
    assert_eq!(field(line, &["exit"]), 2, "{line}");
    assert_eq!(field(line, &["flags"]), &Value::Array(Vec::new()), "{line}");
    for key in [
        "version",
        "time",
        "derived",
        "gates",
        "findings",
        "notes",
        "asked",
        "timing",
        "config_hash",
    ] {
        assert!(line.get(key).is_some(), "no {key} in {line}");
    }
    assert!(field(line, &["timing", "klin_ms"]).is_u64(), "{line}");
    assert!(
        field(line, &["timing", "base_remove_ms"]).is_u64(),
        "{line}"
    );
    assert!(field(line, &["timing", "base_prune_ms"]).is_u64(), "{line}");
}

#[test]
fn a_blocking_stop_and_the_stop_after_it_record_the_spent_block() {
    let tree = tree(EVERY_GATE);
    prompt(&tree);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let second = stop(&tree, A_SECOND_STOP);
    assert_eq!(second.code, 0, "{}", second.out);

    let lines = stops(&tree);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(field(&lines[0], &["hook", "blocked"]), true, "{}", lines[0]);
    assert_eq!(field(&lines[0], &["exit"]), 2, "{}", lines[0]);
    assert_eq!(field(&lines[1], &["exit"]), 0, "{}", lines[1]);
    assert_eq!(
        field(&lines[1], &["hook", "blocked"]),
        false,
        "{}",
        lines[1]
    );
    assert_eq!(
        field(&lines[1], &["hook", "gate_spent"]),
        true,
        "{}",
        lines[1]
    );
    assert_eq!(
        field(&lines[1], &["hook", "delivery"]),
        "none",
        "{}",
        lines[1]
    );
    assert!(!second.says("klin radius"), "{}", second.out);
    assert!(!has_flag(&lines[1], "no-prompt-event"), "{}", lines[1]);
}

fn hook_facts(line: &Value) -> (bool, &Value, &Value, &Value) {
    (
        field(line, &["hook", "blocked"])
            .as_bool()
            .unwrap_or_default(),
        field(line, &["hook", "gate_block"]),
        field(line, &["hook", "gate_blocks"]),
        field(line, &["hook", "build_blocks"]),
    )
}

#[test]
fn each_stop_line_says_which_gate_block_it_spent_if_any() {
    let tree = tree(&EVERY_GATE.replacen('{', r#"{ "build": "test ! -f fails","#, 1));
    prompt(&tree);
    tree.words("README.md", 30);

    assert_eq!(stop(&tree, A_STOP).code, 2);
    assert_eq!(stop(&tree, A_SECOND_STOP).code, 0);
    tree.words("README.md", 31);
    assert_eq!(stop(&tree, A_SECOND_STOP).code, 2);
    tree.write("fails", "");
    assert_eq!(stop(&tree, A_SECOND_STOP).code, 2);
    tree.remove("fails");
    tree.words("README.md", 32);
    assert_eq!(stop(&tree, A_SECOND_STOP).code, 0);

    let lines = stops(&tree);
    let facts: Vec<_> = lines.iter().map(hook_facts).collect();
    let null = &Value::Null;
    let wanted = [
        (true, &Value::from(1), &Value::from(1), &Value::from(0)),
        (false, null, &Value::from(1), &Value::from(0)),
        (true, &Value::from(2), &Value::from(2), &Value::from(0)),
        (true, null, &Value::from(2), &Value::from(1)),
        (false, null, &Value::from(2), &Value::from(1)),
    ];
    assert_eq!(facts, wanted, "{lines:?}");
    for line in &lines {
        assert_eq!(field(line, &["hook", "gate_spent"]), true, "{line}");
    }
}

#[test]
fn a_spent_gate_block_tells_the_person_when_no_prompt_event_reached_klin() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let second = stop(&tree, A_SECOND_STOP);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(second.says("klin radius"), "{}", second.out);

    let lines = stops(&tree);
    assert!(has_flag(&lines[1], "no-prompt-event"), "{}", lines[1]);
}

#[test]
fn a_spent_gate_block_with_no_session_does_not_tell_the_person_to_run_radius() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let second = stop(&tree, A_SECOND_STOP_WITHOUT_SESSION);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(!second.says("klin radius"), "{}", second.out);

    let lines = stops(&tree);
    assert!(!has_flag(&lines[1], "no-prompt-event"), "{}", lines[1]);
}

#[test]
fn a_new_prompt_does_not_get_the_no_prompt_note_from_an_earlier_intervention() {
    let tree = gate_radius_tree();
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let session_start = r#"{"hook_event_name": "SessionStart", "session_id": "s-1"}"#;
    let started = harness::feed(tree.root(), &["radius"], session_start);
    assert_eq!(started.code, 0, "{}", started.out);

    let second = stop(&tree, A_SECOND_STOP);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(!second.says("klin radius"), "{}", second.out);

    let lines = stops(&tree);
    assert!(!has_flag(&lines[1], "no-prompt-event"), "{}", lines[1]);
}

#[test]
fn every_gate_row_carries_ms_and_the_held_count_its_ok_line_prints() {
    let tree = Tree::new();
    tree.write("klin.json", EVERY_GATE);
    tree.words("README.md", 5);
    let an_escape = format!("fn f(v: Option<i32>) -> i32 {{\n    v.{}()\n}}\n", "unwrap");
    tree.write("src/lib.rs", &an_escape);
    tree.base();
    prompt(&tree);

    let by_hand = tree.run(&["check", "escapes"]);
    assert_eq!(by_hand.code, 0, "{}", by_hand.out);
    assert!(
        by_hand.says("OK: 1 escape site(s) in the tree, all held at the base"),
        "{}",
        by_hand.out
    );

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    let lines = stops(&tree);
    let gates = field(&lines[0], &["gates"])
        .as_array()
        .unwrap_or_else(|| panic!("no gates list in {}", lines[0]));
    assert!(!gates.is_empty(), "{}", lines[0]);
    for gate in gates {
        assert!(field(gate, &["ms"]).is_u64(), "{gate}");
        assert!(field(gate, &["held"]).is_u64(), "{gate}");
    }
    let escapes = gates
        .iter()
        .find(|gate| field(gate, &["name"]) == "escapes")
        .unwrap_or_else(|| panic!("no escapes row in {}", lines[0]));
    assert_eq!(field(escapes, &["held"]), 1, "{escapes}");
}

#[test]
fn a_stop_that_wrote_no_verdict_still_writes_a_line_that_says_why() {
    let tree = tree(EVERY_GATE);
    prompt(&tree);
    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 0, "{}", first.out);

    let opened = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(tree.state("lock"));
    let Ok(lock) = opened else {
        panic!("the lock file could not be opened");
    };
    assert!(lock.lock().is_ok(), "another holder has the lock");

    let held = stop(&tree, A_STOP);
    assert_eq!(held.code, 0, "{}", held.out);
    let lines = journal(&tree);
    let line = lines.last().unwrap_or_else(|| panic!("an empty journal"));
    assert_eq!(field(line, &["verdict"]), "none", "{line}");
    assert!(
        field(line, &["why"])
            .as_str()
            .is_some_and(|why| why.contains("held the state directory")),
        "{line}"
    );
}

#[test]
fn an_unwritable_state_directory_still_reports_the_failure_and_blocks_nothing() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    let file = tree.at("a-file");
    tree.write("a-file", "");

    let run = harness::feed_with(
        tree.root(),
        &[("KLIN_STATE_DIR", file.as_str())],
        &["gate", "--hook"],
        A_STOP,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("blocks nothing"), "{}", run.out);
}

#[test]
fn cache_clean_leaves_the_journal_in_place() {
    let tree = tree(EVERY_GATE);
    prompt(&tree);
    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(tree.state("journal.jsonl").is_file());

    let clean = tree.run(&["cache", "clean"]);
    assert_eq!(clean.code, 0, "{}", clean.out);
    assert!(tree.state("journal.jsonl").is_file(), "{}", clean.out);
}

const A_SARIF_GATE: &str = r#"{
  "sarif": [{"name": "eslint", "report": "eslint.sarif"}],
  "accepted": [
    {"gate": "eslint", "file": "src/a.ts", "text": "no-any: on the changed line", "count": 1}
  ]
}"#;

fn a_sarif_result(line: u64, message: &str) -> String {
    format!(
        r#"{{"ruleId": "no-any", "message": {{"text": "{message}"}}, "locations": [
            {{"physicalLocation": {{
                "artifactLocation": {{"uri": "src/a.ts"}},
                "region": {{"startLine": {line}}}
            }}}}
        ]}}"#
    )
}

/// Spec 11.2 defines `held` as the findings a base site or an accepted entry carried, which is
/// one quantity and not two. A sarif gate also drops the results the window did not touch, and
/// those are its coverage and never its hold.
#[test]
fn a_gate_row_holds_what_the_ratchet_let_through_and_not_what_the_window_dropped() {
    let tree = Tree::new();
    tree.write("klin.json", A_SARIF_GATE);
    tree.write("src/a.ts", "one\ntwo\nthree\n");
    tree.base();
    prompt(&tree);
    tree.write("src/a.ts", "one\nchanged\nthree\n");
    tree.write(
        "eslint.sarif",
        &format!(
            r#"{{"version": "2.1.0", "runs": [{{
                "tool": {{"driver": {{"name": "eslint"}}}},
                "results": [{}, {}]
            }}]}}"#,
            a_sarif_result(2, "on the changed line"),
            a_sarif_result(1, "on a line the window did not change"),
        ),
    );

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    let lines = stops(&tree);
    let gates = field(&lines[0], &["gates"])
        .as_array()
        .unwrap_or_else(|| panic!("no gates list in {}", lines[0]));
    let eslint = gates
        .iter()
        .find(|gate| field(gate, &["name"]) == "eslint")
        .unwrap_or_else(|| panic!("no eslint row in {}", lines[0]));
    assert_eq!(field(eslint, &["held"]), 1, "{eslint}");
}

/// The `why` beside `verdict: "none"` names the reason this stop had, and no other. A stamp
/// klin read and could not write back is not a state directory klin found no stamp in.
#[test]
fn a_stamp_that_could_not_be_written_says_so_and_not_that_there_was_none() {
    let tree = tree(EVERY_GATE);
    prompt(&tree);
    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 0, "{}", first.out);

    let writing = tree.state("turn.writing");
    assert!(std::fs::create_dir(&writing).is_ok(), "the writing path");

    let held = stop(&tree, A_STOP);
    assert_eq!(held.code, 0, "{}", held.out);
    let line = journal(&tree)
        .pop()
        .unwrap_or_else(|| panic!("an empty journal"));
    assert_eq!(field(&line, &["verdict"]), "none", "{line}");
    assert_eq!(
        field(&line, &["why"]),
        "the turn stamp could not be written, so this stop wrote no verdict",
        "{line}"
    );
}

// #154: the journal knows the session, the prompt, the guard's refusals and resets.

const RADIUS_PINNED: &str = r#"{
  "radius": { "lines": 50, "directories": 2 }
}"#;

const GATE_WITH_RADIUS: &str = r#"{
  "doc_size": {"README.md": 10},
  "radius": { "lines": 50, "directories": 2 }
}"#;

fn radius_tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", RADIUS_PINNED);
    tree.write("src/a.rs", "// held\n");
    tree.base();
    tree
}

fn gate_radius_tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", GATE_WITH_RADIUS);
    tree.words("README.md", 5);
    tree.base();
    tree
}

#[test]
fn a_prompt_event_appends_a_line_with_the_counter_the_session_and_the_excerpt() {
    let tree = radius_tree();
    let event = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-9",
                    "prompt": "Fix the thing\nmore context on a second line"}"#;
    let run = harness::feed(tree.root(), &["radius"], event);
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(field(line, &["schema"]), 1, "{line}");
    assert_eq!(field(line, &["kind"]), "prompt", "{line}");
    assert_eq!(field(line, &["prompt"]), 1, "{line}");
    assert_eq!(field(line, &["session"]), "s-9", "{line}");
    assert_eq!(field(line, &["text"]), "Fix the thing", "{line}");
}

/// A session start moves the mark and raises the counter, but it is not a prompt event and
/// appends no prompt line. The prompt after it measures against that mark. Spec 9.6, 11.4.
#[test]
fn a_prompt_after_a_measurable_change_carries_the_radius_facts() {
    let tree = radius_tree();
    let session = r#"{"hook_event_name": "SessionStart", "session_id": "s-9"}"#;
    assert_eq!(harness::feed(tree.root(), &["radius"], session).code, 0);
    tree.write("src/a.rs", "// held\n// more\n");

    let event = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-9", "prompt": "go"}"#;
    let run = harness::feed(tree.root(), &["radius"], event);
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "prompt", "{line}");
    assert_eq!(field(line, &["prompt"]), 2, "{line}");
    assert!(field(line, &["radius", "lines"]).is_u64(), "{line}");
    assert!(field(line, &["radius", "formatting"]).is_u64(), "{line}");
    assert!(field(line, &["radius", "moved"]).is_u64(), "{line}");
    assert_eq!(field(line, &["radius", "directories"]), 1, "{line}");
    assert_eq!(field(line, &["radius", "wide"]), false, "{line}");
}

#[test]
fn journal_prompt_false_turns_off_the_excerpt_and_keeps_the_rest() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"journal": {"prompt": false}}"#);
    tree.base();

    let event = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-1",
                    "prompt": "a secret plan"}"#;
    let run = harness::feed(tree.root(), &["radius"], event);
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "prompt", "{line}");
    assert_eq!(field(line, &["session"]), "s-1", "{line}");
    assert!(line.get("text").is_none(), "{line}");
}

/// The privacy switch fails closed. A configuration klin cannot parse is the one case where it
/// cannot see `journal.prompt`, so the excerpt stays out of the record. Spec 5.2, 11.4.
#[test]
fn a_configuration_that_will_not_load_turns_off_the_excerpt() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"journal": {"prompt": false},}"#);
    tree.base();

    let event = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-1",
                    "prompt": "a secret plan"}"#;
    let run = harness::feed(tree.root(), &["radius"], event);
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "prompt", "{line}");
    assert!(line.get("text").is_none(), "{line}");
}

/// A configuration klin refuses at load for a schedule with no step due is one klin cannot read,
/// so the prompt line carries no excerpt, as it does for a file that will not parse. Spec 11.4, 14.
#[test]
fn a_schedule_with_no_step_due_turns_off_the_excerpt() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": {"README.md": {"2099-01-01": 100}}}"#,
    );
    tree.words("README.md", 5);
    tree.base();

    let event = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-1",
                    "prompt": "a secret plan"}"#;
    let run = harness::feed(tree.root(), &["radius"], event);
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "prompt", "{line}");
    assert!(line.get("text").is_none(), "{line}");
}

fn guard(tree: &Tree, event: &str) -> harness::Run {
    harness::feed(tree.root(), &["guard"], event)
}

#[test]
fn a_guard_deny_for_the_configuration_appends_a_line_with_config_write() {
    let tree = tree(EVERY_GATE);
    let event = format!(
        r#"{{"tool_name": "Write", "tool_input": {{"file_path": {:?}}}, "session_id": "s-7"}}"#,
        tree.at("klin.json")
    );
    let run = guard(&tree, &event);
    assert_eq!(run.code, 2, "{}", run.out);

    let lines = journal(&tree);
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert_eq!(field(line, &["schema"]), 1, "{line}");
    assert_eq!(field(line, &["kind"]), "guard", "{line}");
    assert_eq!(field(line, &["session"]), "s-7", "{line}");
    assert_eq!(field(line, &["decision"]), "deny", "{line}");
    assert_eq!(field(line, &["reason"]), "config-write", "{line}");
}

#[test]
fn a_guard_deny_for_the_state_directory_appends_a_line_with_state_write() {
    let tree = tree(EVERY_GATE);
    let run = guard(
        &tree,
        r#"{"tool_name": "Bash", "tool_input": {"command": "echo x > .git/klin/turn"}}"#,
    );
    assert_eq!(run.code, 2, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "guard", "{line}");
    assert_eq!(field(line, &["decision"]), "deny", "{line}");
    assert_eq!(field(line, &["reason"]), "state-write", "{line}");
}

#[test]
fn a_guard_ask_naming_the_configuration_appends_a_line_with_config_mention() {
    let tree = tree(EVERY_GATE);
    let run = guard(
        &tree,
        r#"{"tool_name": "Bash", "tool_input": {"command": "rm klin.json"}}"#,
    );
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "guard", "{line}");
    assert_eq!(field(line, &["decision"]), "ask", "{line}");
    assert_eq!(field(line, &["reason"]), "config-mention", "{line}");
}

/// The ask side of `state-write`: a shell command only names the state directory as a write
/// argument, which klin cannot prove the way a direct edit or a redirect does (ADR 0033), so it
/// asks rather than denies. The reason mirrors `config-mention`'s.
#[test]
fn a_guard_ask_naming_the_state_directory_appends_a_line_with_state_mention() {
    let tree = tree(EVERY_GATE);
    let run = guard(
        &tree,
        r#"{"tool_name": "Bash", "tool_input": {"command": "rm .git/klin/turn"}}"#,
    );
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "guard", "{line}");
    assert_eq!(field(line, &["decision"]), "ask", "{line}");
    assert_eq!(field(line, &["reason"]), "state-mention", "{line}");
}

#[test]
fn a_guard_deny_for_klin_setup_appends_a_line_with_setup() {
    let tree = tree(EVERY_GATE);
    let run = guard(
        &tree,
        r#"{"tool_name": "Bash", "tool_input": {"command": "klin setup"}}"#,
    );
    assert_eq!(run.code, 2, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "guard", "{line}");
    assert_eq!(field(line, &["decision"]), "deny", "{line}");
    assert_eq!(field(line, &["reason"]), "setup", "{line}");
}

/// Codex CLI has no question to ask on this event, so the ask reaches the agent as a refusal.
/// The journal records the answer the host delivered, not the one the guard reached, so a
/// reader of 11.4 counts a real refusal as a deny. Spec 9.1, 11.4.
#[test]
fn an_ask_a_host_delivers_as_a_refusal_is_journaled_as_a_deny() {
    let tree = tree(EVERY_GATE);
    let run = guard(
        &tree,
        r#"{"turn_id": "t-1", "tool_name": "Bash", "session_id": "s-8",
            "tool_input": {"command": "rm klin.json"}}"#,
    );
    assert_eq!(run.code, 2, "{}", run.out);

    let lines = journal(&tree);
    let line = &lines[0];
    assert_eq!(field(line, &["kind"]), "guard", "{line}");
    assert_eq!(field(line, &["session"]), "s-8", "{line}");
    assert_eq!(field(line, &["decision"]), "deny", "{line}");
    assert_eq!(field(line, &["reason"]), "config-mention", "{line}");
}

#[test]
fn a_guard_allow_appends_no_line() {
    let tree = tree(EVERY_GATE);
    let run = guard(
        &tree,
        r#"{"tool_name": "Read", "tool_input": {"file_path": "klin.json"}}"#,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(journal(&tree).is_empty(), "{:?}", journal(&tree));
}

#[test]
fn turn_reset_appends_a_line_with_the_prompt_counter() {
    let tree = tree(EVERY_GATE);
    prompt(&tree);
    prompt(&tree);
    tree.words("README.md", 30);
    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);

    let run = tree.run(&["turn", "reset"]);
    assert_eq!(run.code, 0, "{}", run.out);

    let lines = journal(&tree);
    let line = lines.last().unwrap_or_else(|| panic!("an empty journal"));
    assert_eq!(field(line, &["kind"]), "reset", "{line}");
    assert_eq!(field(line, &["session"]), &Value::Null, "{line}");
    assert_eq!(field(line, &["prompt"]), 2, "{line}");
}

/// Unrelated history in front of the lines a stop reads, dated a month back, so the bounded
/// reader of 11.4 must stop before any of it to answer for the session in front of it.
fn old_history(tree: &Tree, rows: usize) {
    let at = tree.state("journal.jsonl");
    assert!(
        at.parent()
            .is_some_and(|dir| std::fs::create_dir_all(dir).is_ok()),
        "the state directory"
    );
    let old = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
        .saturating_sub(30 * 86_400);
    let text: String = (0..rows)
        .map(|_| {
            format!(
                "{{\"schema\":1,\"version\":\"0.0.0\",\"kind\":\"stop\",\"time\":{old},\
                 \"session\":\"s-0\",\"prompt\":1,\"flags\":[],\"told\":[]}}\n"
            )
        })
        .collect();
    assert!(std::fs::write(&at, text).is_ok(), "the journal");
}

#[test]
fn a_spent_gate_block_finds_this_session_prompt_behind_an_old_journal() {
    let tree = tree(EVERY_GATE);
    old_history(&tree, 10_000);
    prompt(&tree);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let second = stop(&tree, A_SECOND_STOP);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(!second.says("klin radius"), "{}", second.out);

    let lines = stops(&tree);
    let last = lines.last().unwrap_or_else(|| panic!("a stop line"));
    assert!(!has_flag(last, "no-prompt-event"), "{last}");
}

#[test]
fn a_spent_gate_block_behind_an_old_journal_still_reports_a_missing_prompt() {
    let tree = tree(EVERY_GATE);
    old_history(&tree, 10_000);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let second = stop(&tree, A_SECOND_STOP);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(second.says("klin radius"), "{}", second.out);

    let lines = stops(&tree);
    let last = lines.last().unwrap_or_else(|| panic!("a stop line"));
    assert!(has_flag(last, "no-prompt-event"), "{last}");
}
