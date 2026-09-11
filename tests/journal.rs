mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const EVERY_GATE: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "doc_citations": [{"file": "README.md", "roots": ["."]}],
  "escapes": { "roots": ["src"], "languages": ["rust"] },
  "complexity": { "roots": ["src"], "ceilings": {"cc": 8, "lines": 60} }
}"#;

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false,
                         "session_id": "s-1"}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true,
                                "session_id": "s-1"}"#;
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;

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

fn field<'a>(line: &'a Value, path: &[&str]) -> &'a Value {
    let mut held = line;
    for key in path {
        held = held
            .get(key)
            .unwrap_or_else(|| panic!("no {key} in {line}"));
    }
    held
}

#[test]
fn a_stop_appends_one_line_holding_the_record_and_what_the_hook_knew() {
    let tree = tree(EVERY_GATE);
    prompt(&tree);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    let lines = journal(&tree);
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
    assert_eq!(field(line, &["exit"]), 1, "{line}");
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

    let lines = journal(&tree);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(field(&lines[0], &["hook", "blocked"]), true, "{}", lines[0]);
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

    let by_hand = tree.run(&["escapes"]);
    assert_eq!(by_hand.code, 0, "{}", by_hand.out);
    assert!(
        by_hand.says("OK: 1 escape site(s) in the tree, all held at the base"),
        "{}",
        by_hand.out
    );

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    let lines = journal(&tree);
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
fn an_unwritable_state_directory_leaves_the_exit_code_and_the_text_unchanged() {
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
    assert_eq!(run.code, 2, "{}", run.out);
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
  "project": "t",
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
    let lines = journal(&tree);
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
