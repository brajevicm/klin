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
/// A later stop of the same turn ends on another message, which Codex sends with it. Spec 9.8.
const A_CODEX_THIRD_STOP: &str = r#"{"hook_event_name":"Stop","session_id":"s1","turn_id":"t1","permission_mode":"default","stop_hook_active":true,"last_assistant_message":"fixed"}"#;
/// Cursor's event shape, measured on Cursor 3.20.21 on 2026-09-16; the payloads carry that
/// version. `docs/cursor-compatibility.md` holds the measurements, `docs/HOST_COMPATIBILITY.md`
/// the support row.
const A_CURSOR_SHELL_COMMAND: &str = r#"{"hook_event_name":"preToolUse","cursor_version":"3.20.21","conversation_id":"s1","session_id":"s1","tool_name":"Shell","tool_input":{"command":"rm klin.json"}}"#;
const A_CURSOR_SHELL_EVENT: &str = r#"{"hook_event_name":"beforeShellExecution","cursor_version":"3.20.21","conversation_id":"s1","command":"rm klin.json"}"#;
const A_CURSOR_MCP_CALL: &str = r#"{"hook_event_name":"beforeMCPExecution","cursor_version":"3.20.21","conversation_id":"s1","tool_name":"mcp__server__tool","tool_input":{},"command":"rm klin.json"}"#;
const A_CURSOR_STOP: &str = r#"{"hook_event_name":"stop","cursor_version":"3.20.21","conversation_id":"s1","session_id":"s1","loop_count":0}"#;

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
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    feed(tree.root(), &[harness::AGENT, args].concat(), event)
}

fn stop(tree: &Tree, event: &str, args: &[&str]) -> Run {
    feed(tree.root(), &[harness::AGENT, args].concat(), event)
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
    assert!(run.says("this stop is not blocked"), "{}", run.out);
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

    let run = feed(elsewhere.root(), harness::AGENT, &event.to_string());

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

    let run = feed(elsewhere.root(), harness::AGENT, &event.to_string());

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(r#""followup_message":"#), "{}", run.out);
    assert!(run.says("then stop again"), "{}", run.out);
    assert!(run.says("README.md"), "{}", run.out);
}

#[test]
fn cursor_reads_a_shell_command_off_the_event_itself() {
    let run = guard(&[], A_CURSOR_SHELL_EVENT);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(r#""permission":"deny""#), "{}", run.out);
    assert!(run.says("klin.json"), "{}", run.out);
}

/// Cursor 3.21.18 did not submit the `followup_message` of a stop hook that exited 2, and did
/// submit one from a hook that exited 0, so a Cursor block exits 0. docs/cursor-compatibility.md.
#[test]
fn a_cursor_block_exits_0_and_is_journaled_as_a_block() {
    let tree = cursor_failing();
    let blocked = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(blocked.code, 0, "{}", blocked.out);
    assert!(blocked.says(r#""followup_message":"#), "{}", blocked.out);
    assert!(blocked.says("gate block 1 of 2"), "{}", blocked.out);

    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let line: serde_json::Value = journal
        .lines()
        .rfind(|line| line.contains(r#""kind":"stop""#))
        .and_then(|line| serde_json::from_str(line).ok())
        .unwrap_or_default();
    assert_eq!(line["hook"]["blocked"], true, "{line}");
    assert_eq!(line["hook"]["gate_block"], 1, "{line}");
    assert_eq!(line["result"]["exit"], serde_json::Value::Null, "{line}");
    assert_eq!(line["notice"], serde_json::Value::Null, "{line}");
}

/// `loop_count` counts the automatic follow-ups before this stop. It says nothing about a
/// block, so a first stop blocks whatever count it carries.
#[test]
fn a_first_cursor_stop_blocks_whatever_its_loop_count() {
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
    let opened = feed(tree.root(), harness::AGENT, &session.to_string());
    assert_eq!(opened.code, 0, "{}", opened.out);
    tree.words("README.md", 30);

    let blocked = stop(&tree, &cursor_stop_at("s1", 5), &[]);
    assert_eq!(blocked.code, 0, "{}", blocked.out);
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
    let radius = feed(tree.root(), harness::AGENT, &echoed.to_string());
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

/// Cursor submits a `followup_message` as the next agent prompt, so a note for the person never
/// uses it, and Claude Code's notice is not Cursor's either. Spec 10.7.
#[test]
fn cursor_never_tells_a_note_as_a_followup() {
    let tree = cursor_noted();

    let run = stop(&tree, A_CURSOR_STOP, &[]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("followup_message"), "{}", run.out);
    assert!(run.says("NOTE:"), "{}", run.out);
    assert!(!run.says("systemMessage"), "{}", run.out);
}

/// `AskUserQuestion` may keep the turn open, so the Stop after the person answers judges the tree
/// with no prompt between. klin intercepts no ask tool: same-tree pass-through and the two-block
/// cap bound the flow. Spec 10.5.
#[test]
fn an_ask_tool_flow_keeps_same_tree_pass_through_and_the_two_block_cap() {
    let tree = failing();
    let first = stop(&tree, A_STOP, &[]);
    assert_eq!(first.code, 2, "{}", first.out);
    assert!(first.says("gate block 1 of 2"), "{}", first.out);

    let ask = r#"{"hook_event_name":"PreToolUse","session_id":"s1","tool_name":"AskUserQuestion","tool_input":{"questions":[]}}"#;
    let asked = feed(tree.root(), harness::AGENT, ask);
    assert_eq!(asked.code, 0, "{}", asked.out);
    assert_eq!(asked.printed, "", "{}", asked.out);

    let answered = stop(&tree, A_SECOND_STOP, &[]);
    assert_eq!(answered.code, 0, "{}", answered.out);
    assert!(answered.says("not blocking again"), "{}", answered.out);

    tree.words("README.md", 31);
    let second = stop(&tree, A_SECOND_STOP, &[]);
    assert_eq!(second.code, 2, "{}", second.out);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);

    tree.words("README.md", 32);
    let capped = stop(&tree, A_SECOND_STOP, &[]);
    assert_eq!(capped.code, 0, "{}", capped.out);
    assert!(capped.says("has blocked 2 stops"), "{}", capped.out);
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
    let third = stop(&tree, A_CODEX_THIRD_STOP, &[]);
    assert_eq!(third.code, 0, "{}", third.out);
    assert!(third.says("has blocked 2 stops"), "{}", third.out);
}

/// Cursor submits each block report as the next prompt. klin consumes that prompt without a fresh
/// gate budget, so the stops after it spend the prompt's second block and no more. A stop that
/// blocks nothing hands Cursor no follow-up at all. Spec 10.4, 10.7.
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
    let opened = feed(tree.root(), harness::AGENT, &session.to_string());
    assert_eq!(opened.code, 0, "{}", opened.out);
    tree.words("README.md", 30);
    let first = stop(&tree, A_CURSOR_STOP, &[]);
    assert!(first.says("gate block 1 of 2"), "{}", first.out);
    echo_followup(&tree, &first);

    let again = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(again.code, 0, "{}", again.out);
    assert!(!again.says("followup_message"), "{}", again.out);

    tree.words("README.md", 31);
    let second = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
    echo_followup(&tree, &second);

    tree.words("README.md", 32);
    let third = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(third.code, 0, "{}", third.out);
    assert!(third.says("has blocked 2 stops"), "{}", third.out);
}

/// Cursor sends no flag that a stop already blocked, so klin's own record is the only bound. A
/// gate block klin cannot record would repeat at every stop, so it blocks nothing.
#[test]
fn a_cursor_gate_block_klin_cannot_record_blocks_nothing() {
    let tree = failing();
    let record = tree.path(".git/klin/build-blocked");
    assert!(
        std::fs::create_dir_all(&record).is_ok(),
        "{}",
        record.display()
    );

    for at in 1..=3 {
        let run = stop(&tree, A_CURSOR_STOP, &[]);
        assert_eq!(run.code, 0, "stop {at}: {}", run.out);
        assert!(run.says("FAIL  doc-size"), "stop {at}: {}", run.out);
        assert!(
            run.says("klin could not record a gate block"),
            "stop {at}: {}",
            run.out
        );
    }
}

fn echo_followup(tree: &Tree, blocked: &Run) {
    echo_from(tree, blocked, "s1");
}

fn echo_from(tree: &Tree, blocked: &Run, session: &str) {
    let answer: serde_json::Value = match serde_json::from_str(blocked.printed.trim()) {
        Ok(answer) => answer,
        Err(why) => panic!("{why} — Cursor stop printed:\n{}", blocked.printed),
    };
    submit(tree, session, &answer["followup_message"]);
}

fn submit(tree: &Tree, session: &str, prompt: &serde_json::Value) {
    let submitted = serde_json::json!({
        "hook_event_name": "beforeSubmitPrompt",
        "cursor_version": "3.20.21",
        "conversation_id": session,
        "workspace_roots": [tree.root()],
        "prompt": prompt
    });
    let radius = feed(tree.root(), harness::AGENT, &submitted.to_string());
    assert_eq!(radius.code, 0, "{}", radius.out);
}

fn cursor_stop(session: &str) -> String {
    cursor_stop_at(session, 0)
}

/// A Cursor stop whose `loop_count` says how many automatic follow-ups came before it.
fn cursor_stop_at(session: &str, loop_count: u64) -> String {
    serde_json::json!({
        "hook_event_name": "stop",
        "cursor_version": "3.21.18",
        "conversation_id": session,
        "session_id": session,
        "loop_count": loop_count
    })
    .to_string()
}

/// Cursor submits the follow-up that wins its merge of every stop hook's answer, which may be
/// another hook's text klin cannot recognize, and that text raises the prompt counter. A stop
/// whose `loop_count` is above 0 follows an automatic message, so it keeps the budget of the
/// prompt the chain continues. A person's message brings `loop_count` back to 0 and a fresh
/// budget. docs/cursor-compatibility.md.
#[test]
fn a_cursor_stop_after_an_automatic_message_keeps_its_prompts_budget() {
    let tree = cursor_failing();
    let first = stop(&tree, &cursor_stop_at("s1", 0), &[]);
    assert!(first.says("gate block 1 of 2"), "{}", first.out);

    let prompts = tree.field("prompts");
    submit(&tree, "s1", &"ANOTHER-HOOK-WON-THE-MERGE".into());
    assert_ne!(
        tree.field("prompts"),
        prompts,
        "the merged text read as a person's prompt"
    );
    let unchanged = stop(&tree, &cursor_stop_at("s1", 1), &[]);
    assert_eq!(unchanged.code, 0, "{}", unchanged.out);
    assert!(
        unchanged.says("the tree did not change since the last gate block"),
        "{}",
        unchanged.out
    );
    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let line: serde_json::Value = journal
        .lines()
        .rfind(|line| line.contains(r#""kind":"stop""#))
        .and_then(|line| serde_json::from_str(line).ok())
        .unwrap_or_default();
    assert_eq!(line["hook"]["continued"], true, "{line}");

    tree.words("README.md", 31);
    let second = stop(&tree, &cursor_stop_at("s1", 2), &[]);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
    tree.words("README.md", 32);
    let capped = stop(&tree, &cursor_stop_at("s1", 3), &[]);
    assert!(capped.says("has blocked 2 stops"), "{}", capped.out);

    submit(&tree, "s1", &"keep going".into());
    let fresh = stop(&tree, &cursor_stop_at("s1", 0), &[]);
    assert!(fresh.says("gate block 1 of 2"), "{}", fresh.out);
}

/// A person's prompt after a prompt that spent both gate blocks gets the whole budget, even
/// where its first stop spends none and another hook's message then continues the chain.
#[test]
fn a_cursor_chain_after_a_clean_stop_inherits_no_earlier_prompts_budget() {
    let tree = exhausted("s1");
    submit(&tree, "s1", &"fix the docs".into());
    tree.words("README.md", 5);
    let clean = stop(&tree, &cursor_stop_at("s1", 0), &[]);
    assert_eq!(clean.code, 0, "{}", clean.out);
    assert!(!clean.says("gate block"), "{}", clean.out);

    submit(&tree, "s1", &"ANOTHER-HOOK-WON-THE-MERGE".into());
    tree.words("README.md", 30);
    let continued = stop(&tree, &cursor_stop_at("s1", 1), &[]);
    assert!(continued.says("gate block 1 of 2"), "{}", continued.out);
}

/// The block record belongs to the session that took it, so a chain in another session starts
/// its own budget and never carries the first session's cap.
#[test]
fn a_cursor_chain_inherits_no_other_sessions_budget() {
    let tree = exhausted("s1");
    submit(&tree, "s2", &"ANOTHER-HOOK-WON-THE-MERGE".into());
    tree.words("README.md", 33);
    let other = stop(&tree, &cursor_stop_at("s2", 1), &[]);
    assert!(other.says("gate block 1 of 2"), "{}", other.out);
}

/// A failing Cursor tree whose session already spent both gate blocks under its prompt.
fn exhausted(session: &str) -> Tree {
    let tree = cursor_failing();
    let first = stop(&tree, &cursor_stop_at(session, 0), &[]);
    assert!(first.says("gate block 1 of 2"), "{}", first.out);
    echo_from(&tree, &first, session);
    tree.words("README.md", 31);
    let second = stop(&tree, &cursor_stop_at(session, 1), &[]);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
    echo_from(&tree, &second, session);
    tree.words("README.md", 32);
    let capped = stop(&tree, &cursor_stop_at(session, 2), &[]);
    assert!(capped.says("has blocked 2 stops"), "{}", capped.out);
    tree
}

/// A Cursor session opened over a tree whose README is within its ceiling, then pushed over it.
fn cursor_failing() -> Tree {
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
    let opened = feed(tree.root(), harness::AGENT, &session.to_string());
    assert_eq!(opened.code, 0, "{}", opened.out);
    tree.words("README.md", 30);
    tree
}

/// A tree whose only news is a file no grammar reads, which every stop tells as a note.
fn cursor_noted() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity": {"cc": 1, "lines": 1}}"#);
    tree.write("src/flow.rs", "fn f() {}\n");
    tree.base();
    let session = serde_json::json!({
        "hook_event_name": "sessionStart",
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "workspace_roots": [tree.root()]
    });
    let opened = feed(tree.root(), harness::AGENT, &session.to_string());
    assert_eq!(opened.code, 0, "{}", opened.out);
    tree.write("src/new.rs", "%%% not rust %%%\n");
    tree
}

/// A Cursor notice that blocks nothing emits no `followup_message`. The journal holds it, `klin
/// status` and `klin report` show it while its window is open, and it expires when the window
/// closes. Spec 10.7, 11.4, 13.2.
#[test]
fn a_cursor_notice_lands_in_the_journal_status_and_report_until_its_window_closes() {
    let tree = cursor_noted();
    let run = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("followup_message"), "{}", run.out);

    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let line: serde_json::Value = journal
        .lines()
        .rfind(|line| line.contains(r#""kind":"stop""#))
        .and_then(|line| serde_json::from_str(line).ok())
        .unwrap_or_default();
    assert_eq!(line["notice"]["delivered"], false, "{line}");
    let message = line["notice"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("NOTE:"), "{line}");

    let status = tree.run(&["status", "--json"]).json();
    assert_eq!(
        status["window"]["notices"][0]["message"], message,
        "{status}"
    );
    let report = tree.run(&["report", "--json"]).json();
    assert_eq!(report["counts"]["notices"], 1, "{report}");
    assert_eq!(report["notices"][0]["message"], message, "{report}");
    assert_eq!(report["notices"][0]["delivered"], false, "{report}");
    let text = tree.run(&["report"]);
    assert!(
        text.says("klin left you a notice in this window:"),
        "{}",
        text.out
    );

    let stamped = tree.field("commit");
    submit(&tree, "s1", &"the next task".into());
    assert_ne!(tree.field("commit"), stamped, "the prompt kept the stamp");
    let held = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    let mut held: serde_json::Value = serde_json::from_str(&held).unwrap_or_default();
    held["time"] = (held["time"].as_u64().unwrap_or_default() + 5).into();
    tree.write(".git/klin/turn", &held.to_string());

    let status = tree.run(&["status", "--json"]).json();
    assert_eq!(
        status["window"]["notices"],
        serde_json::json!([]),
        "{status}"
    );
    let report = tree.run(&["report", "--json"]).json();
    assert_eq!(report["counts"]["notices"], 0, "{report}");
}

/// Two Cursor sessions share one worktree. What one session was handed does not replace what
/// the other was, so the first session's echoed block report still opens no turn.
#[test]
fn a_second_cursor_session_does_not_refresh_the_first_sessions_budget() {
    let tree = cursor_failing();
    let blocked = stop(&tree, &cursor_stop("s1"), &[]);
    assert!(blocked.says("gate block 1 of 2"), "{}", blocked.out);

    let other = stop(&tree, &cursor_stop("s2"), &[]);
    assert_eq!(other.code, 0, "{}", other.out);
    assert!(!other.says("followup_message"), "{}", other.out);

    let prompts = tree.field("prompts");
    echo_from(&tree, &blocked, "s1");
    assert_eq!(tree.field("prompts"), prompts, "an echo opened a turn");

    tree.words("README.md", 31);
    let second = stop(&tree, &cursor_stop("s1"), &[]);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
}

/// Cursor submits a block report as its next prompt, so a report klin could not record as
/// expected would open a turn and a fresh budget. Such a block is reported and blocks nothing.
#[test]
fn a_cursor_block_klin_cannot_hand_off_is_reported_and_blocks_nothing() {
    let tree = cursor_failing();
    tree.write(".git/klin/handed", "");

    let run = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(!run.says("followup_message"), "{}", run.out);
    assert!(
        run.says("could not record the report the host will submit"),
        "{}",
        run.out
    );
}

/// A stop that lost the state lock hands Cursor no follow-up and leaves the turn stamp as the
/// stop holding the lock wrote it.
#[test]
fn a_cursor_stop_without_the_state_lock_tells_nothing_and_writes_no_stamp() {
    let tree = cursor_noted();
    let first = stop(&tree, A_CURSOR_STOP, &[]);
    assert!(!first.says("followup_message"), "{}", first.out);
    let stamp = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();

    let opened = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(tree.state("lock"));
    let Ok(lock) = opened else {
        panic!("the lock file could not be opened");
    };
    assert!(lock.lock().is_ok(), "another holder has the lock");
    tree.write("src/other.rs", "%%% not rust either %%%\n");

    let held = stop(&tree, A_CURSOR_STOP, &[]);
    assert_eq!(held.code, 0, "{}", held.out);
    assert!(!held.says("followup_message"), "{}", held.out);
    let after = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    assert_eq!(after, stamp, "a stop without the lock wrote the turn stamp");
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

/// A malformed event says nothing about the turn, so the guard allows and the stop is not
/// blocked. The ingress names the event it could not read on stderr alone. Spec 10.10.
#[test]
fn a_malformed_event_under_a_named_host_allows_and_never_blocks() {
    for event in ["", "not json", "{"] {
        let run = guard(&["--host", "claude"], event);
        assert_eq!(run.code, 0, "{event}: {}", run.out);
        assert_eq!(run.printed, "", "{event}: {}", run.out);
        assert!(run.says("could not read"), "{event}: {}", run.out);

        let stopped = stop(&failing(), event, &["--host", "claude"]);
        assert_ne!(stopped.code, 2, "{event}: {}", stopped.out);
    }
}
