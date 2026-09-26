mod harness;

use harness::{Run, Tree};
use serde_json::{Value, json};

const OVER: &str = r#"{ "doc_size": {"README.md": 10} }"#;

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", OVER);
    tree.words("README.md", 5);
    tree.base();
    tree
}

fn claude(event: &str, session: &str, prompt: &str) -> Value {
    json!({
        "session_id": session,
        "prompt_id": prompt,
        "hook_event_name": event,
        "transcript_path": "/tmp/t.jsonl",
        "cwd": "/tmp",
        "permission_mode": "default",
    })
}

fn with(mut event: Value, extra: Value) -> String {
    if let (Some(event), Some(extra)) = (event.as_object_mut(), extra.as_object()) {
        event.extend(extra.clone());
    }
    event.to_string()
}

fn run(tree: &Tree, args: &[&str], event: &str) -> Run {
    harness::feed(tree.root(), args, event)
}

/// The host fires every copy of one event at once, so the copies start together.
fn together(tree: &Tree, args: &[&str], event: &str) -> [Run; 2] {
    std::thread::scope(|scope| {
        let first = scope.spawn(|| run(tree, args, event));
        let second = scope.spawn(|| run(tree, args, event));
        [first.join().expect("first"), second.join().expect("second")]
    })
}

fn journal(tree: &Tree, kind: &str) -> Vec<Value> {
    std::fs::read_to_string(tree.state("journal.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|line| line["kind"] == kind)
        .collect()
}

fn prompts(tree: &Tree) -> u64 {
    tree.field("prompts").parse().unwrap_or(0)
}

fn a_prompt(session: &str, prompt: &str) -> String {
    with(
        claude("UserPromptSubmit", session, prompt),
        json!({"prompt": "go"}),
    )
}

#[test]
fn two_copies_of_one_prompt_move_the_counter_once() {
    let tree = tree();
    let before = prompts(&tree);

    let copies = together(&tree, &["radius"], &a_prompt("s1", "p1"));

    assert!(copies.iter().all(|copy| copy.code == 0));
    assert_eq!(
        prompts(&tree),
        before + 1,
        "the counter moved once per copy"
    );
    assert_eq!(journal(&tree, "prompt").len(), 1);
}

/// A copy of an event that arrives after the first copy finished is still that event.
#[test]
fn a_late_copy_of_a_prompt_yields_and_prints_nothing() {
    let tree = tree();
    let before = prompts(&tree);

    run(&tree, &["radius"], &a_prompt("s1", "p1"));
    let late = run(&tree, &["radius"], &a_prompt("s1", "p1"));

    assert_eq!(late.code, 0, "{}", late.out);
    assert_eq!(late.out, "", "a yielding copy spoke");
    assert_eq!(prompts(&tree), before + 1);
}

#[test]
fn two_prompts_and_two_sessions_on_one_worktree_each_move_the_counter() {
    let tree = tree();
    let before = prompts(&tree);

    run(&tree, &["radius"], &a_prompt("s1", "p1"));
    run(&tree, &["radius"], &a_prompt("s1", "p2"));
    run(&tree, &["radius"], &a_prompt("s2", "p1"));

    assert_eq!(prompts(&tree), before + 3);
}

#[test]
fn two_copies_of_one_failing_stop_run_one_gate_and_still_block() {
    let tree = tree();
    tree.words("README.md", 40);
    let stop = with(
        claude("Stop", "s1", "p1"),
        json!({"stop_hook_active": false, "last_assistant_message": "done"}),
    );

    let copies = together(&tree, &["gate", "--hook", "--changed"], &stop);

    let blocked: Vec<&Run> = copies.iter().filter(|copy| copy.code == 2).collect();
    assert_eq!(blocked.len(), 1, "{:?}", copies.map(|copy| copy.out));
    assert!(blocked[0].says("doc_size"), "{}", blocked[0].out);
    let yielded = copies
        .iter()
        .find(|copy| copy.code != 2)
        .expect("a copy yields");
    assert_eq!((yielded.code, yielded.out.as_str()), (0, ""));
    assert_eq!(journal(&tree, "stop").len(), 1, "two gates ran");
}

/// Two stops under one prompt differ in what the host says about them, so the second is gated.
#[test]
fn a_second_stop_under_one_prompt_is_not_a_copy_of_the_first() {
    let tree = tree();
    tree.words("README.md", 40);
    let first = with(
        claude("Stop", "s1", "p1"),
        json!({"stop_hook_active": false, "last_assistant_message": "done"}),
    );
    let second = with(
        claude("Stop", "s1", "p1"),
        json!({"stop_hook_active": true, "last_assistant_message": "fixed"}),
    );

    run(&tree, &["gate", "--hook", "--changed"], &first);
    run(&tree, &["gate", "--hook", "--changed"], &second);

    assert_eq!(journal(&tree, "stop").len(), 2);
}

#[test]
fn two_copies_of_one_refused_call_both_refuse_and_journal_once() {
    let tree = tree();
    let call = with(
        claude("PreToolUse", "s1", "p1"),
        json!({
            "tool_name": "Write",
            "tool_input": {"file_path": tree.at("klin.json"), "content": "{}"},
            "tool_use_id": "toolu_1",
        }),
    );

    let copies = together(&tree, &["guard"], &call);

    assert!(
        copies.iter().all(|copy| copy.code == 2),
        "{:?}",
        copies.map(|copy| copy.out)
    );
    assert_eq!(journal(&tree, "guard").len(), 1);
}

#[test]
fn two_refused_calls_each_journal_a_refusal() {
    let tree = tree();
    let call = |id: &str| {
        with(
            claude("PreToolUse", "s1", "p1"),
            json!({
                "tool_name": "Write",
                "tool_input": {"file_path": tree.at("klin.json"), "content": "{}"},
                "tool_use_id": id,
            }),
        )
    };

    run(&tree, &["guard"], &call("toolu_1"));
    run(&tree, &["guard"], &call("toolu_2"));

    assert_eq!(journal(&tree, "guard").len(), 2);
}

#[test]
fn two_copies_of_one_codex_prompt_move_the_counter_once() {
    let tree = tree();
    let before = prompts(&tree);
    let prompt = json!({
        "session_id": "s1",
        "turn_id": "t1",
        "hook_event_name": "UserPromptSubmit",
        "model": "m",
        "permission_mode": "default",
        "prompt": "go",
    })
    .to_string();

    run(&tree, &["radius"], &prompt);
    run(&tree, &["radius"], &prompt);

    assert_eq!(prompts(&tree), before + 1);
}

#[test]
fn two_copies_of_one_cursor_prompt_move_the_counter_once() {
    let tree = tree();
    let before = prompts(&tree);
    let prompt = json!({
        "conversation_id": "c1",
        "generation_id": "g1",
        "hook_event_name": "beforeSubmitPrompt",
        "cursor_version": "3.21.18",
        "workspace_roots": [tree.root()],
        "prompt": "go",
    })
    .to_string();

    run(&tree, &["radius"], &prompt);
    run(&tree, &["radius"], &prompt);

    assert_eq!(prompts(&tree), before + 1);
}

/// Two events that name the same fields are a model turn apart, and the second is gated.
#[test]
fn the_same_stop_again_after_the_copies_settled_is_gated() {
    let tree = tree();
    tree.words("README.md", 40);
    let stop = with(
        claude("Stop", "s1", "p1"),
        json!({"stop_hook_active": true, "last_assistant_message": "done"}),
    );

    run(&tree, &["gate", "--hook", "--changed"], &stop);
    std::thread::sleep(std::time::Duration::from_millis(2100));
    run(&tree, &["gate", "--hook", "--changed"], &stop);

    assert_eq!(journal(&tree, "stop").len(), 2);
}
