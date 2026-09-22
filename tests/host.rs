mod harness;

use harness::{Run, Tree, feed};

const A_CONFIG: &str = r#"{
  "doc_size": {"README.md": 10}
}"#;

/// Claude Code's event shape, from its hooks reference. The ledger in
/// `docs/HOST_COMPATIBILITY.md` names the host version and date each claim was last verified
/// against, and the weekly canary re-runs that check on the current stable release.
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
/// Claude Code sends `permission_mode` on most events, and so does Codex. Only `turn_id` is Codex's.
const A_CLAUDE_AMBIGUOUS_COMMAND_WITH_PERMISSION_MODE: &str = r#"{"hook_event_name":"PreToolUse","session_id":"s1","cwd":"/x","permission_mode":"default","tool_name":"Bash","tool_input":{"command":"rm klin.json"}}"#;
/// Codex CLI's event shape, from its hooks reference. `docs/HOST_COMPATIBILITY.md` records
/// which Codex version last carried it.
const A_CODEX_STOP: &str = r#"{"hook_event_name":"Stop","session_id":"s1","turn_id":"t1","permission_mode":"default","stop_hook_active":false}"#;
const A_CODEX_SECOND_STOP: &str = r#"{"hook_event_name":"Stop","session_id":"s1","turn_id":"t1","permission_mode":"default","stop_hook_active":true}"#;
/// Cursor's event shape, measured on Cursor 3.20.21 on 2026-09-16; the payloads carry that
/// version. `docs/cursor-compatibility.md` holds the measurements, `docs/HOST_COMPATIBILITY.md`
/// the support row.
const A_CURSOR_SHELL_COMMAND: &str = r#"{"hook_event_name":"preToolUse","cursor_version":"3.20.21","conversation_id":"s1","session_id":"s1","tool_name":"Shell","tool_input":{"command":"rm klin.json"}}"#;
const A_CURSOR_SHELL_EVENT: &str = r#"{"hook_event_name":"beforeShellExecution","cursor_version":"3.20.21","conversation_id":"s1","command":"rm klin.json"}"#;
const A_CURSOR_MCP_CALL: &str = r#"{"hook_event_name":"beforeMCPExecution","cursor_version":"3.20.21","conversation_id":"s1","tool_name":"mcp__server__tool","tool_input":{},"command":"rm klin.json"}"#;
const A_CURSOR_STOP: &str = r#"{"hook_event_name":"stop","cursor_version":"3.20.21","conversation_id":"s1","session_id":"s1","loop_count":5}"#;

fn codex(name: &str, command: &str) -> String {
    format!(
        r#"{{"hook_event_name":"PreToolUse","session_id":"s1","turn_id":"t1","permission_mode":"default","tool_name":{name:?},"tool_input":{{"command":{command:?}}}}}"#
    )
}

fn cursor(name: &str, input: &str) -> String {
    format!(
        r#"{{"hook_event_name":"preToolUse","cursor_version":"3.20.21","conversation_id":"s1","tool_name":{name:?},"tool_input":{input}}}"#
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
    assert!(run.says("still, after a round of fixes"), "{}", run.out);
    assert!(run.says("not blocking again"), "{}", run.out);
}

#[test]
fn a_claude_stop_after_a_gate_block_over_a_changed_tree_spends_the_second() {
    let tree = failing();
    let first = stop(&tree, A_STOP, &[]);
    assert_eq!(first.code, 2, "{}", first.out);

    tree.words("README.md", 31);
    let second = stop(&tree, A_SECOND_STOP, &[]);
    assert_eq!(second.code, 2, "{}", second.out);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
}

#[test]
fn a_claude_event_that_carries_permission_mode_is_still_asked() {
    let run = guard(&[], A_CLAUDE_AMBIGUOUS_COMMAND_WITH_PERMISSION_MODE);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(r#""permissionDecision":"ask""#), "{}", run.out);
}

/// Cursor reads `permission` from stdout on a refusal. Its documented `ask` was not enforced by
/// the verified host, so klin fails the question closed like Codex does. An allow is exit 0 with
/// no stdout, matching Claude Code and Codex, so a user-scope plugin does not pre-approve every
/// call in a tree that never wrote `klin.json`. Issue #67 AC 6, Spec 9.1.
#[test]
fn cursor_events_use_native_permission_decisions() {
    let refused = guard(&[], A_CURSOR_SHELL_COMMAND);
    assert_eq!(refused.code, 2, "{}", refused.out);
    assert!(refused.says(r#""permission":"deny""#), "{}", refused.out);
    assert!(
        refused.says("did not enforce a question"),
        "{}",
        refused.out
    );

    let denied = guard(&[], &cursor("Write", r#"{"file_path":"klin.json"}"#));
    assert_eq!(denied.code, 2, "{}", denied.out);
    assert!(denied.says(r#""permission":"deny""#), "{}", denied.out);

    let allowed = guard(&[], &cursor("Write", r#"{"file_path":"src/main.rs"}"#));
    assert_eq!(allowed.code, 0, "{}", allowed.out);
    assert!(!allowed.says("permission"), "{}", allowed.out);
}

/// An MCP event carries the server's own launch command at the top level. The agent did not run
/// it, so the guard reads no command there.
#[test]
fn cursor_mcp_events_ignore_the_server_launch_command() {
    let run = guard(&[], A_CURSOR_MCP_CALL);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("permission"), "{}", run.out);
    assert!(!run.says("klin.json"), "{}", run.out);
}

/// A Cursor guard in a tree that never wrote `klin.json` prints nothing on an allow, on every
/// event the plugin hooks. Spec 9.1, issue #67 AC 6.
#[test]
fn cursor_guard_is_silent_in_a_tree_without_klin_json() {
    for event in [
        cursor("Write", r#"{"file_path":"src/main.rs"}"#),
        A_CURSOR_SHELL_EVENT
            .to_string()
            .replace("rm klin.json", "ls"),
        A_CURSOR_MCP_CALL.to_string(),
    ] {
        let run = guard(&[], &event);
        assert_eq!(run.code, 0, "{event}: {}", run.out);
        assert!(run.out.is_empty(), "{event}: {}", run.out);
    }
}

/// Cursor runs a user-scope hook from `~/.cursor`, so the tree comes from the event: `cwd` on a
/// tool event, and `workspace_roots` on a stop.
#[test]
fn cursor_guard_measures_the_tree_the_event_names() {
    let tree = failing();
    let elsewhere = Tree::bare();
    let event = serde_json::json!({
        "hook_event_name": "preToolUse",
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "workspace_roots": [tree.root()],
        "cwd": tree.root(),
        "tool_name": "Write",
        "tool_input": {"file_path": "klin.json"}
    });

    let run = feed(elsewhere.root(), &["guard"], &event.to_string());

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(r#""permission":"deny""#), "{}", run.out);
}

#[test]
fn cursor_stop_measures_the_workspace_root_the_event_names() {
    let tree = failing();
    let elsewhere = Tree::bare();
    let event = serde_json::json!({
        "hook_event_name": "stop",
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "workspace_roots": [tree.root()],
        "status": "completed",
        "loop_count": 0
    });

    let run = feed(
        elsewhere.root(),
        &["gate", "--hook", "--changed"],
        &event.to_string(),
    );

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(r#""followup_message":"#), "{}", run.out);
    assert!(run.says("README.md"), "{}", run.out);
}

#[test]
fn cursor_reads_a_shell_command_off_the_event_itself() {
    let run = guard(&[], A_CURSOR_SHELL_EVENT);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(r#""permission":"deny""#), "{}", run.out);
    assert!(run.says("klin.json"), "{}", run.out);
}

#[test]
fn cursor_stop_blocks_and_ignores_loop_count() {
    let tree = Tree::new();
    tree.write("klin.json", A_CONFIG);
    tree.words("README.md", 5);
    tree.base();
    let session = serde_json::json!({
        "hook_event_name": "sessionStart",
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "workspace_roots": [tree.root()]
    });
    let opened = feed(tree.root(), &["radius"], &session.to_string());
    assert_eq!(opened.code, 0, "{}", opened.out);
    tree.words("README.md", 30);

    let blocked = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(blocked.code, 2, "{}", blocked.out);
    assert!(blocked.says(r#""followup_message":"#), "{}", blocked.out);
    assert!(blocked.says("fix what each names"), "{}", blocked.out);
    assert!(!blocked.says("not blocking again"), "{}", blocked.out);

    let answer: serde_json::Value = match serde_json::from_str(blocked.printed.trim()) {
        Ok(answer) => answer,
        Err(why) => panic!("{why} — Cursor stop printed:\n{}", blocked.printed),
    };
    let followup = answer["followup_message"].as_str().unwrap_or_default();
    assert!(!followup.is_empty(), "{answer}");
    let prompts = tree.field("prompts");
    let echoed = serde_json::json!({
        "hook_event_name": "beforeSubmitPrompt",
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "workspace_roots": [tree.root()],
        "prompt": followup
    });
    let radius = feed(tree.root(), &["radius"], &echoed.to_string());
    assert_eq!(radius.code, 0, "{}", radius.out);
    assert_eq!(radius.out, "", "the expected follow-up opened a turn");
    assert_eq!(
        tree.field("prompts"),
        prompts,
        "the follow-up raised the counter"
    );

    let passed = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(passed.code, 0, "{}", passed.out);
    assert!(
        passed.says("the tree did not change since the last gate block"),
        "{}",
        passed.out
    );

    let silent = stop(&Tree::new(), A_CURSOR_STOP, &[]);
    assert_eq!(silent.code, 0, "{}", silent.out);
    assert_eq!(silent.out, "", "{}", silent.out);
}

/// A note the stop tells the person uses Cursor's follow-up field, not Claude Code's notice.
#[test]
fn cursor_tells_a_note_as_a_followup() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{
  "complexity": {"cc": 1, "lines": 1}
}"#,
    );
    tree.write("src/flow.rs", "fn f() {}\n");
    tree.base();
    tree.write("src/flow.rs", "%%% not rust %%%\n");

    let run = stop(&tree, A_CURSOR_STOP, &[]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(r#""followup_message":"#), "{}", run.out);
    assert!(run.says("NOTE:"), "{}", run.out);
    assert!(!run.says("systemMessage"), "{}", run.out);
}

#[test]
fn a_codex_stop_honors_its_blocked_before_flag() {
    let first = stop(&failing(), A_CODEX_STOP, &[]);
    assert_eq!(first.code, 2, "{}", first.out);

    let run = stop(&failing(), A_CODEX_SECOND_STOP, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("not blocking again"), "{}", run.out);
}

#[test]
fn a_codex_continuation_over_a_changed_tree_spends_the_second_gate_block() {
    let tree = failing();
    let first = stop(&tree, A_CODEX_STOP, &[]);
    assert_eq!(first.code, 2, "{}", first.out);

    tree.words("README.md", 31);
    let second = stop(&tree, A_CODEX_SECOND_STOP, &[]);
    assert_eq!(second.code, 2, "{}", second.out);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);

    tree.words("README.md", 32);
    let third = stop(&tree, A_CODEX_SECOND_STOP, &[]);
    assert_eq!(third.code, 0, "{}", third.out);
    assert!(third.says("has blocked 2 stops"), "{}", third.out);
}

/// Cursor submits each block report as the next prompt. klin consumes that prompt without a
/// fresh gate budget, so the stop after it spends the prompt's second block and no more.
#[test]
fn a_cursor_followup_gains_no_fresh_gate_budget() {
    let tree = Tree::new();
    tree.write("klin.json", A_CONFIG);
    tree.words("README.md", 5);
    tree.base();
    let session = serde_json::json!({
        "hook_event_name": "sessionStart",
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "workspace_roots": [tree.root()]
    });
    let opened = feed(tree.root(), &["radius"], &session.to_string());
    assert_eq!(opened.code, 0, "{}", opened.out);
    tree.words("README.md", 30);
    let first = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(first.code, 2, "{}", first.out);
    echo_followup(&tree, &first);

    tree.words("README.md", 31);
    let second = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(second.code, 2, "{}", second.out);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
    echo_followup(&tree, &second);

    tree.words("README.md", 32);
    let third = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(third.code, 0, "{}", third.out);
    assert!(third.says("has blocked 2 stops"), "{}", third.out);
}

fn echo_followup(tree: &Tree, blocked: &Run) {
    let answer: serde_json::Value = match serde_json::from_str(blocked.printed.trim()) {
        Ok(answer) => answer,
        Err(why) => panic!("{why} — Cursor stop printed:\n{}", blocked.printed),
    };
    let echoed = serde_json::json!({
        "hook_event_name": "beforeSubmitPrompt",
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "workspace_roots": [tree.root()],
        "prompt": answer["followup_message"]
    });
    let radius = feed(tree.root(), &["radius"], &echoed.to_string());
    assert_eq!(radius.code, 0, "{}", radius.out);
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

    for headers in [
        "*** Update File: src/main.rs\n*** Update File: klin.json",
        "*** Update File: a.json\n*** Move to: klin.json",
    ] {
        let denied = codex(
            "apply_patch",
            &format!("*** Begin Patch\n{headers}\n*** End Patch"),
        );
        let run = guard(&[], &denied);
        assert_eq!(run.code, 2, "{headers}: {}", run.out);
        assert!(run.says("refused"), "{headers}: {}", run.out);
    }
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
