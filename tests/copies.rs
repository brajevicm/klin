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

/// Every claim as the copies left it `by` ago, so the event reads as settled without a wait.
fn aged(tree: &Tree, by: std::time::Duration) {
    let then = std::time::SystemTime::now() - by;
    for entry in std::fs::read_dir(tree.state("claims")).expect("claims") {
        std::fs::File::options()
            .write(true)
            .open(entry.expect("claim").path())
            .and_then(|claim| claim.set_modified(then))
            .expect("aged");
    }
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
fn concurrent_distinct_sessions_serialize_the_prompt_counter() {
    let tree = tree();
    let before = prompts(&tree);
    tree.write(".git/klin/lock", "");
    let lock = std::fs::File::options()
        .write(true)
        .open(tree.state("lock"))
        .expect("state lock");
    lock.lock().expect("hold state lock");
    let events = [a_prompt("s1", "p1"), a_prompt("s2", "p2")];
    let (sent, received) = std::sync::mpsc::channel();

    std::thread::scope(|scope| {
        for event in &events {
            let sent = sent.clone();
            let tree = &tree;
            scope.spawn(move || sent.send(run(tree, &["radius"], event)).expect("result"));
        }
        // Both events must have arrived before we release the deliberately held lock.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let arrived =
            || std::fs::read_dir(tree.state("claims")).map_or(0, |entries| entries.count()) == 2;
        while !arrived() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let both_arrived = arrived();
        let early = received.recv_timeout(std::time::Duration::from_secs(1));
        let held = prompts(&tree);
        lock.unlock().expect("release state lock");
        assert!(both_arrived, "both distinct events must arrive");
        assert!(
            early.is_err(),
            "a prompt wrote while the transaction lock was held"
        );
        assert_eq!(held, before);
        for _ in 0..2 {
            let run = received
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("prompt");
            assert_eq!(run.code, 0, "{}", run.out);
        }
    });

    assert_eq!(prompts(&tree), before + 2);
    let lines = journal(&tree, "prompt");
    assert_eq!(lines.len(), 2);
    assert_ne!(lines[0]["prompt"], lines[1]["prompt"]);
}

#[test]
fn a_slow_prompt_capture_does_not_cost_a_failing_stop_its_block() {
    let tree = tree();
    tree.words("README.md", 40);
    tree.write(".git/klin/index.captures/lock", "");
    let capture = std::fs::File::options()
        .write(true)
        .open(tree.state("index.captures/lock"))
        .expect("capture lock");
    capture.lock().expect("hold capture lock");
    let prompt = a_prompt("s1", "p1");
    let stop = with(
        claude("Stop", "s2", "p2"),
        json!({"stop_hook_active": false, "last_assistant_message": "done"}),
    );

    std::thread::scope(|scope| {
        let prompted = scope.spawn(|| run(&tree, &["radius"], &prompt));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while tree.state("claims").read_dir().map_or(0, Iterator::count) == 0
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let stopped = run(&tree, &["gate", "--hook", "--changed"], &stop);
        capture.unlock().expect("release capture lock");
        assert_eq!(stopped.code, 2, "{}", stopped.out);
        assert!(stopped.says("doc_size"), "{}", stopped.out);
        assert!(!stopped.says("held the state directory"), "{}", stopped.out);
        assert_eq!(prompted.join().expect("prompt").code, 0);
    });
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
    aged(&tree, std::time::Duration::from_secs(3));
    run(&tree, &["gate", "--hook", "--changed"], &stop);

    assert_eq!(journal(&tree, "stop").len(), 2);
}

/// Cursor sends a shell call to its own `beforeShellExecution` hook, and to a hook it imported
/// from Claude Code's settings as `preToolUse` with the `Shell` tool. Measured on Cursor 3.22.7.
#[test]
fn a_cursor_shell_call_seen_by_a_native_and_an_imported_copy_journals_once() {
    let tree = tree();
    let common = json!({
        "conversation_id": "c1",
        "generation_id": "g1",
        "session_id": "c1",
        "model": "m",
        "cursor_version": "3.22.7",
        "workspace_roots": [tree.root()],
        "transcript_path": null,
        "user_email": null,
    });
    let imported = with(
        common.clone(),
        json!({
            "hook_event_name": "preToolUse",
            "tool_name": "Shell",
            "tool_input": {"command": "rm klin.json", "cwd": "", "timeout": 30000},
            "tool_use_id": "tool_1",
            "cwd": "",
        }),
    );
    let native = with(
        common,
        json!({
            "hook_event_name": "beforeShellExecution",
            "command": "rm klin.json",
            "cwd": "",
            "sandbox": false,
        }),
    );

    let first = run(&tree, &["guard"], &imported);
    let second = run(&tree, &["guard"], &native);

    assert_eq!(
        (first.code, second.code),
        (2, 2),
        "{}\n{}",
        first.out,
        second.out
    );
    assert_eq!(journal(&tree, "guard").len(), 1);
}
