mod harness;

use harness::{Run, Tree, feed};

const A_CONFIG: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}]
}"#;

const A_GUARDED_EDIT: &str = r#"{
  "hook_event_name": "PreToolUse",
  "session_id": "s1",
  "tool_name": "Write",
  "tool_input": {"file_path": "klin.json"}
}"#;

const AN_ORDINARY_EDIT: &str = r#"{
  "hook_event_name": "PreToolUse",
  "session_id": "s1",
  "tool_name": "Write",
  "tool_input": {"file_path": "src/main.rs"}
}"#;

const AN_AMBIGUOUS_COMMAND: &str = r#"{
  "hook_event_name": "PreToolUse",
  "session_id": "s1",
  "tool_name": "Bash",
  "tool_input": {"command": "rm klin.json"}
}"#;

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;
const A_CODEX_STOP: &str = r#"{"hook_event_name":"Stop","session_id":"s1","turn_id":"t1","permission_mode":"default","stop_hook_active":false}"#;
const A_CODEX_SECOND_STOP: &str = r#"{"hook_event_name":"Stop","session_id":"s1","turn_id":"t1","permission_mode":"default","stop_hook_active":true}"#;

fn codex(name: &str, command: &str) -> String {
    format!(
        r#"{{"hook_event_name":"PreToolUse","session_id":"s1","turn_id":"t1","permission_mode":"default","tool_name":{name:?},"tool_input":{{"command":{command:?}}}}}"#
    )
}

fn failing() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", A_CONFIG);
    tree.words("README.md", 5);
    tree.base();
    tree.words("README.md", 30);
    tree
}

fn guard(args: &[&str], event: &str) -> Run {
    feed(Tree::new().root(), &[&["guard"], args].concat(), event)
}

fn stop(tree: &Tree, event: &str, args: &[&str]) -> Run {
    feed(tree.root(), &[&["gate", "--hook"], args].concat(), event)
}

#[test]
fn a_pre_tool_event_comes_back_as_a_decision() {
    let denied = guard(&[], A_GUARDED_EDIT);
    assert_eq!(denied.code, 2, "{}", denied.out);
    assert!(denied.says("refused"), "{}", denied.out);

    let allowed = guard(&[], AN_ORDINARY_EDIT);
    assert_eq!(allowed.code, 0, "{}", allowed.out);
    assert_eq!(allowed.out, "", "{}", allowed.out);
}

/// Claude Code reads an ask from stdout on exit 0. A host klin cannot place is not credited
/// with `ask`, so the ambiguous class reaches it as the deny of ADR 0011.
#[test]
fn an_ambiguous_call_asks_claude_and_denies_a_host_without_ask() {
    let asked = guard(&["--host", "claude"], AN_AMBIGUOUS_COMMAND);
    assert_eq!(asked.code, 0, "{}", asked.out);
    assert!(asked.says(r#""permissionDecision":"ask""#), "{}", asked.out);
    assert!(asked.says("klin.json"), "{}", asked.out);

    let denied = guard(&["--host", "borg"], AN_AMBIGUOUS_COMMAND);
    assert_eq!(denied.code, 2, "{}", denied.out);
    assert!(!denied.says("permissionDecision"), "{}", denied.out);
    assert!(denied.says("klin.json"), "{}", denied.out);
}

#[test]
fn a_stop_event_comes_back_as_a_block_or_a_pass() {
    let blocked = stop(&failing(), A_STOP, &[]);
    assert_eq!(blocked.code, 2, "{}", blocked.out);

    let tree = failing();
    tree.words("README.md", 5);
    let passed = stop(&tree, A_STOP, &[]);
    assert_eq!(passed.code, 0, "{}", passed.out);
}

#[test]
fn a_stop_that_says_it_already_blocked_this_turn_is_read_that_way() {
    let run = stop(&failing(), A_SECOND_STOP, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("still, after one round of fixes"), "{}", run.out);
    assert!(run.says("not blocking a second time"), "{}", run.out);
}

#[test]
fn a_codex_stop_honors_its_blocked_before_flag() {
    let first = stop(&failing(), A_CODEX_STOP, &[]);
    assert_eq!(first.code, 2, "{}", first.out);

    let run = stop(&failing(), A_CODEX_SECOND_STOP, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("not blocking a second time"), "{}", run.out);
}

#[test]
fn an_event_it_cannot_place_is_read_as_claude_and_says_so() {
    for event in [r#"{"event": "somethingElse"}"#, r#"{"conversation": 1}"#] {
        let run = guard(&[], event);
        assert_eq!(run.code, 0, "{event}: {}", run.out);
        assert!(run.says("reading it as claude"), "{event}: {}", run.out);
    }
}

#[test]
fn a_named_host_it_does_not_know_is_read_as_claude_and_says_so() {
    let run = guard(&["--host", "borg"], A_GUARDED_EDIT);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("reading it as claude"), "{}", run.out);
}

#[test]
fn a_codex_event_uses_codex_decisions_for_shell_and_mcp_calls() {
    for name in ["Bash", "mcp__server__tool"] {
        let run = guard(&[], &codex(name, "rm klin.json"));
        assert_eq!(run.code, 2, "{name}: {}", run.out);
        assert!(run.says("no question to ask"), "{name}: {}", run.out);
    }
}

#[test]
fn a_codex_apply_patch_judges_every_file_path_and_ignores_patch_text() {
    let allowed = codex(
        "apply_patch",
        "*** Begin Patch\n*** Update File: src/main.rs\n@@\n+mention klin.json here\n*** End Patch",
    );
    let run = guard(&[], &allowed);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("permissionDecision"), "{}", run.out);

    let denied = codex(
        "apply_patch",
        "*** Begin Patch\n*** Update File: src/main.rs\n*** Update File: klin.json\n*** End Patch",
    );
    let run = guard(&[], &denied);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("refused"), "{}", run.out);
}

/// A malformed event says nothing about the turn, so the guard allows and the stop is not blocked.
#[test]
fn a_malformed_event_under_a_named_host_allows_and_never_blocks() {
    for event in ["", "not json", "{"] {
        let run = guard(&["--host", "claude"], event);
        assert_eq!(run.code, 0, "{event}: {}", run.out);
        assert_eq!(run.out, "", "{event}: {}", run.out);

        let stopped = stop(&failing(), event, &["--host", "claude"]);
        assert_ne!(stopped.code, 2, "{event}: {}", stopped.out);
    }
}
