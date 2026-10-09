mod harness;

use std::io::Write;
use std::process::{Command, Stdio};

use harness::{AGENT, Run, Tree, binary, empty_home, feed};

const SESSION: &str =
    r#"{"hook_event_name": "SessionStart", "session_id": "s1", "source": "startup"}"#;
const PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s1", "prompt_id": "p1", "prompt": "go"}"#;
const STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const AN_ORDINARY_EDIT: &str = r#"{"hook_event_name": "PreToolUse", "tool_name": "Write", "tool_input": {"file_path": "src/main.rs"}}"#;
const A_CONFIG_EDIT: &str = r#"{"hook_event_name": "PreToolUse", "tool_name": "Write", "tool_input": {"file_path": "klin.json"}}"#;

fn event(tree: &Tree, event: &str) -> Run {
    feed(tree.root(), AGENT, event)
}

/// A configuration that does not parse breaks only what reads it: the guard reads no
/// configuration, and a session or prompt still opens the turn. Spec 10.2.
#[test]
fn an_invalid_config_still_answers_pre_tool_session_and_prompt() {
    let tree = Tree::new();
    tree.write("klin.json", "{not json at all");

    let allowed = event(&tree, AN_ORDINARY_EDIT);
    assert_eq!(allowed.code, 0, "{}", allowed.out);
    assert_eq!(allowed.out, "", "{}", allowed.out);
    let denied = event(&tree, A_CONFIG_EDIT);
    assert_eq!(denied.code, 2, "{}", denied.out);
    assert!(denied.says("refused"), "{}", denied.out);

    let session = event(&tree, SESSION);
    assert_eq!(session.code, 0, "{}", session.out);
    assert!(tree.state("turn").is_file(), "the session wrote no stamp");
    let prompted = event(&tree, PROMPT);
    assert_eq!(prompted.code, 0, "{}", prompted.out);
    assert_eq!(tree.field("prompts"), "2", "the prompt raised no counter");
}

/// A tree with no klin.json at its worktree root is not opted in: every event exits 0, prints
/// nothing and writes no state. Spec 5.1.
#[test]
fn without_a_config_every_event_answers_nothing() {
    let tree = Tree::new();
    tree.write("src/main.rs", "fn main() {}\n");

    for payload in [SESSION, PROMPT, A_CONFIG_EDIT, STOP] {
        let run = event(&tree, payload);
        assert_eq!(run.code, 0, "{payload}: {}", run.out);
        assert_eq!(run.out, "", "{payload}: {}", run.out);
    }
    assert!(!tree.path(".git/klin").exists(), "an event wrote state");
}

/// Exit 2 blocks a Stop or denies a tool call on every host, so the ingress keeps it for those
/// alone. Spec 10.10.
#[test]
fn a_usage_error_an_unknown_argument_or_an_unreadable_event_never_exits_two() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": {"README.md": 3}}"#);
    tree.words("README.md", 30);

    for args in [
        &["__agent"][..],
        &["__agent", "nonsense"],
        &["__agent", "--host"],
    ] {
        let run = feed(tree.root(), args, A_CONFIG_EDIT);
        assert_eq!(run.code, 0, "{args:?}: {}", run.out);
    }
    for payload in [
        "",
        "not json",
        "{",
        "[1, 2]",
        r#"{"hook_event_name": "Elsewhere"}"#,
    ] {
        let run = event(&tree, payload);
        assert_eq!(run.code, 0, "{payload}: {}", run.out);
    }

    let unknown = feed(
        tree.root(),
        &[
            "__agent",
            "event",
            "--frobnicate",
            "--host",
            "claude",
            "extra",
        ],
        A_CONFIG_EDIT,
    );
    assert_eq!(unknown.code, 2, "{}", unknown.out);
    assert!(unknown.says("refused"), "{}", unknown.out);
}

/// An ingress that fails without a decision leaves a journal note in a tree that opted in, and
/// still exits 0, so no host reads it as a block or a deny. A tree that did not opt in keeps no
/// state. Spec 10.10.
#[test]
fn an_ingress_failure_in_an_opted_in_tree_writes_a_journal_note() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": {"README.md": 3}}"#);
    tree.words("README.md", 2);

    let usage = feed(tree.root(), &["__agent"], STOP);
    assert_eq!(usage.code, 0, "{}", usage.out);
    let unreadable = event(&tree, "not json");
    assert_eq!(unreadable.code, 0, "{}", unreadable.out);

    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let notes: Vec<serde_json::Value> = journal
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line["kind"] == "note")
        .collect();
    assert_eq!(notes.len(), 2, "{journal}");
    assert_eq!(notes[0]["schema"], 2, "{journal}");
    assert!(
        notes[0]["message"]
            .as_str()
            .is_some_and(|said| said.contains("takes `event`")),
        "{journal}"
    );
    assert!(
        notes[1]["message"]
            .as_str()
            .is_some_and(|said| said.contains("could not read")),
        "{journal}"
    );

    let off = Tree::new();
    let run = event(&off, "not json");
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        !off.path(".git/klin").exists(),
        "a tree that did not opt in kept state"
    );
}

/// A Stop that panics, here because the host closed stdout before klin told the person, answers
/// nothing: it exits 1, never 2, and leaves a journal note in a tree that opted in. Spec 10.10.
#[test]
fn a_stop_that_panics_exits_1_and_writes_a_journal_note() {
    let tree = Tree::new();
    tree.write("klin.json", "{not json at all");

    let mut child = Command::new(binary())
        .arg("__agent")
        .arg("event")
        .env("HOME", empty_home())
        .current_dir(tree.root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run klin");
    drop(child.stdout.take());
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(STOP.as_bytes())
        .expect("write stdin");
    let done = child.wait_with_output().expect("wait for klin");
    let said = String::from_utf8_lossy(&done.stderr);
    assert_eq!(done.status.code(), Some(1), "{said}");

    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let note = journal
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|line| line["kind"] == "note")
        .unwrap_or_else(|| panic!("no journal note: {journal}"));
    assert_eq!(note["event"], "stop", "{journal}");
    assert!(
        note["message"]
            .as_str()
            .is_some_and(|message| message.contains("failed while it answered")),
        "{journal}"
    );
}

/// The opt-in walk ends at the first directory that holds a `.git` entry, so a repository nested
/// inside an opted-in one is not opted in by the outer file. A `.git` file whose `gitdir` names
/// nothing ends no walk. Spec 5.1.
#[test]
fn the_opt_in_walk_stops_at_the_first_git_entry() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("inner/src/main.rs", "fn main() {}\n");
    tree.git(&["-C", "inner", "init", "-q"]);

    let inner = feed(&tree.path("inner"), AGENT, A_CONFIG_EDIT);
    assert_eq!(inner.code, 0, "{}", inner.out);
    assert_eq!(inner.out, "", "{}", inner.out);

    tree.write("linked/src/main.rs", "fn main() {}\n");
    tree.write("linked/.git", &format!("gitdir: {}\n", tree.at(".git")));
    let linked = feed(&tree.path("linked"), AGENT, A_CONFIG_EDIT);
    assert_eq!(linked.code, 0, "{}", linked.out);
    assert_eq!(linked.out, "", "{}", linked.out);

    tree.write("stale/src/main.rs", "fn main() {}\n");
    tree.write("stale/.git", "gitdir: /nowhere/at/all\n");
    let outer = feed(
        &tree.path("stale"),
        AGENT,
        &format!(
            r#"{{"hook_event_name": "PreToolUse", "tool_name": "Write", "tool_input": {{"file_path": {:?}}}}}"#,
            tree.at("klin.json")
        ),
    );
    assert_eq!(outer.code, 2, "{}", outer.out);
}
