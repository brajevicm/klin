mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use std::fs::File;

use harness::{Run, Tree};
use serde_json::Value;

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const CONFIG: &str = r#"{
  "project": "t",
  "escapes": { "in": "src" }
}"#;

/// A repository whose base holds clean sources, with one prompt's turn stamp already taken.
fn stamped() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    assert_eq!(harness::feed(tree.root(), &["radius"], A_PROMPT).code, 0);
    tree
}

fn stop(tree: &Tree) -> Run {
    harness::feed(tree.root(), &["gate", "--hook", "--changed"], A_STOP)
}

/// One prompt of the same turn, which raises the prompt counter and so hands the next stop a
/// fresh block budget.
fn prompt(tree: &Tree) {
    assert_eq!(harness::feed(tree.root(), &["radius"], A_PROMPT).code, 0);
}

fn accepting(file: &str) -> String {
    format!(
        r#"{{ "project": "t",
             "accepted": [{{"gate": "escapes", "file": "{file}", "text": {:?}, "count": 1}}],
             "escapes": {{ "in": "src" }} }}"#,
        text::ONE_SITE
    )
}

#[test]
fn a_commit_inside_the_turn_leaves_the_debt_new() {
    let tree = stamped();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.commit("the agent committed its own debt");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("src/lib.rs"), "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
}

#[test]
fn a_turn_that_changed_nothing_passes_without_a_note() {
    let tree = stamped();

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{}", run.out);
}

#[test]
fn every_stop_writes_its_verdict_into_the_turn_file() {
    let tree = stamped();

    assert_eq!(stop(&tree).code, 0);
    assert_eq!(tree.field("verdict"), "green");

    tree.write("src/lib.rs", text::WRAPPED);
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(tree.field("verdict"), "red");
}

#[test]
fn accepting_one_finding_holds_it_and_leaves_the_other_failing() {
    let tree = stamped();
    tree.write("src/a.rs", text::ONE);
    tree.write("src/b.rs", text::OTHER);

    let both = stop(&tree);
    assert_eq!(both.code, 2, "{}", both.out);
    assert!(both.says("src/a.rs"), "{}", both.out);

    tree.write("klin.json", &accepting("src/a.rs"));
    prompt(&tree);
    let left = stop(&tree);
    assert_eq!(left.code, 2, "{}", left.out);
    assert!(left.says("src/b.rs"), "{}", left.out);
    assert!(!left.says("src/a.rs"), "{}", left.out);
}

#[test]
fn a_turn_file_that_is_gone_is_restored_red_from_the_ref() {
    let tree = stamped();
    let held = tree.revision("refs/worktree/klin/turn");
    tree.write("src/lib.rs", text::WRAPPED);
    tree.remove(".git/klin/turn");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("restored"), "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert_eq!(tree.field("commit"), held);
    assert_eq!(tree.field("verdict"), "red");
}

#[test]
fn a_stamp_and_a_ref_that_are_both_gone_widen_the_window_to_the_branch() {
    let tree = stamped();
    tree.remove(".git/klin/turn");
    tree.git(&["update-ref", "-d", "refs/worktree/klin/turn"]);
    tree.write("src/lib.rs", text::WRAPPED);
    tree.commit("the debt the deletion meant to hide");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no turn stamp resolves"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
    assert!(run.says("src/lib.rs"), "{}", run.out);
    assert_eq!(tree.field("commit"), tree.revision("main"));
    assert_eq!(tree.field("verdict"), "red");
}

#[test]
fn a_deleted_stamp_falls_back_to_head_when_no_base_resolves() {
    let tree = Tree::bare();
    tree.git(&["init", "-q", "-b", "work"]);
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", CLEAN);
    tree.commit("the only commit, on a branch no base names");
    assert_eq!(harness::feed(tree.root(), &["radius"], A_PROMPT).code, 0);
    tree.remove(".git/klin/turn");
    tree.git(&["update-ref", "-d", "refs/worktree/klin/turn"]);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no base resolves"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
    assert_eq!(tree.field("commit"), tree.revision("HEAD"));
}

#[test]
fn a_stop_that_cannot_take_the_lock_writes_no_verdict_and_says_so() {
    let tree = stamped();
    assert_eq!(stop(&tree).code, 0);
    tree.write("src/lib.rs", text::WRAPPED);
    let Ok(taken) = File::create(tree.state("lock")) else {
        panic!("the lock file could not be made")
    };
    assert!(taken.lock().is_ok(), "the test could not hold the lock");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("wrote no verdict"), "{}", run.out);
    assert_eq!(
        tree.field("verdict"),
        "green",
        "a stop without the lock wrote over the verdict"
    );
}

/// A commit only a per-worktree ref reaches, which a sibling worktree's `git gc` can prune.
fn pruned(tree: &Tree) {
    let text = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    let mut held: Value = serde_json::from_str(&text).unwrap_or_default();
    held["commit"] = "0".repeat(40).into();
    tree.write(".git/klin/turn", &held.to_string());
}

#[test]
fn a_stamp_commit_git_no_longer_holds_comes_back_from_the_ref() {
    let tree = stamped();
    let ref_holds = tree.revision("refs/worktree/klin/turn");
    tree.write("src/lib.rs", text::WRAPPED);
    pruned(&tree);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("restored"), "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert_eq!(tree.field("commit"), ref_holds);
}

#[test]
fn a_stamp_commit_no_reference_holds_widens_the_window_to_the_branch() {
    let tree = stamped();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.commit("the debt a prune would have forgiven");
    pruned(&tree);
    tree.git(&["update-ref", "-d", "refs/worktree/klin/turn"]);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no turn stamp resolves"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
    assert!(run.says("src/lib.rs"), "{}", run.out);
    assert_eq!(tree.field("commit"), tree.revision("main"));
}

#[test]
fn a_state_directory_klin_cannot_keep_still_reads_the_stamp_from_the_ref() {
    let tree = stamped();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.write("a-file", "");
    let held = tree.at("a-file");

    let run = harness::feed_with(
        tree.root(),
        &[("KLIN_STATE_DIR", held.as_str())],
        &["gate", "--hook", "--changed"],
        A_STOP,
    );
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert!(run.says("wrote no verdict"), "{}", run.out);
}

#[test]
fn the_json_report_names_the_window_the_stop_used() {
    let tree = stamped();
    tree.write("src/lib.rs", text::WRAPPED);

    let run = harness::feed(
        tree.root(),
        &["gate", "--hook", "--changed", "--json"],
        A_STOP,
    );
    assert_eq!(run.code, 2, "{}", run.out);
    let object = run
        .out
        .lines()
        .find(|line| line.starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON object in:\n{}", run.out));
    let Ok(report) = serde_json::from_str::<Value>(object) else {
        panic!("not one JSON object: {}", run.out)
    };
    let window = report["window"].clone();
    assert_eq!(window["kind"], "turn", "{window}");
    assert_eq!(
        window["before"],
        tree.revision("refs/worktree/klin/turn"),
        "{window}"
    );
}

#[test]
fn a_run_by_hand_keeps_the_branch_window_the_stamp_did_not_touch() {
    let tree = stamped();
    assert_eq!(stop(&tree).code, 0);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
    assert!(run.says("the merge-base with main"), "{}", run.out);
    assert_eq!(
        tree.field("verdict"),
        "green",
        "a run by hand wrote a verdict"
    );
}

#[test]
fn a_stop_after_a_reset_compares_against_the_moved_stamp() {
    let tree = stamped();
    tree.write("src/lib.rs", text::WRAPPED);
    let failed = stop(&tree);
    assert_eq!(failed.code, 2, "{}", failed.out);
    assert!(!failed.says("turn reset"), "{}", failed.out);

    assert_eq!(tree.run(&["turn", "reset"]).code, 0);
    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
}
