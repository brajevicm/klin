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
        r#"{{ "accepted": [{{"gate": "escapes", "file": "{file}", "text": {:?}, "count": 1}}],
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
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("FAIL"), "{}", run.out);
    assert!(run.says("wrote no verdict, spent no block"), "{}", run.out);
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
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert!(run.says("FAIL"), "{}", run.out);
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

    let run = tree.run(&["check", "--changed"]);
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

/// A repository whose turn stamp was taken over a branch tip the current checkout does not
/// hold: a session opened on one branch, and the worktree moved to a divergent one. #238.
fn left_behind() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree.write(
        "tests/test_only_on_work.py",
        "def test_only_on_work():\n    assert True\n",
    );
    tree.commit("the work the branch beside this one does not hold");
    prompt(&tree);
    tree.git(&["checkout", "-q", "main"]);
    tree
}

/// The advisory lock a stop takes over the state directory, held past the hook's budget.
fn held_lock(tree: &Tree) -> File {
    let Ok(taken) = File::create(tree.state("lock")) else {
        panic!("the lock file could not be made")
    };
    assert!(taken.lock().is_ok(), "the test could not hold the lock");
    taken
}

#[test]
fn a_stop_without_the_lock_restores_no_turn_file_from_the_ref() {
    let tree = stamped();
    let reference = tree.revision("refs/worktree/klin/turn");
    tree.remove(".git/klin/turn");
    let _lock = held_lock(&tree);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert!(!tree.state("turn").exists(), "{}", run.out);
    assert_eq!(tree.revision("refs/worktree/klin/turn"), reference);
}

#[test]
fn a_stop_without_the_lock_leaves_an_abandoned_stamp_and_its_refs_alone() {
    let tree = left_behind();
    let stamp = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    let reference = tree.revision("refs/worktree/klin/turn");
    let mark = tree.revision("refs/worktree/klin/mark");
    let _lock = held_lock(&tree);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
    let after = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    assert_eq!(after, stamp, "a stop without the lock replaced the stamp");
    assert_eq!(tree.revision("refs/worktree/klin/turn"), reference);
    assert_eq!(tree.revision("refs/worktree/klin/mark"), mark);
}

/// The per-turn records a stop leaves on the stamp, so one test can prove the fallback drops
/// every one of them rather than carrying them into the branch it judges instead.
fn records(tree: &Tree) {
    let text = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    let mut held: Value = serde_json::from_str(&text).unwrap_or_default();
    held["asked"] = Value::from(vec!["src/lib.rs:1"]);
    held["intervened"] = true.into();
    held["followup"] = 7.into();
    tree.write(".git/klin/turn", &held.to_string());
}

#[test]
fn a_turn_the_checkout_left_behind_judges_the_current_branch() {
    let tree = left_behind();
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("HEAD no longer holds"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
    assert!(!run.says("test_only_on_work"), "{}", run.out);
    assert_eq!(tree.field("commit"), tree.revision("main"));
    assert_eq!(tree.field("verdict"), "red");
}

#[test]
fn the_stop_after_the_fallback_reports_nothing_the_branch_beside_it_held() {
    let tree = left_behind();
    tree.write("src/lib.rs", text::WRAPPED);
    assert_eq!(stop(&tree).code, 2);

    let run = stop(&tree);
    assert!(run.says("window: turn"), "{}", run.out);
    assert!(!run.says("test_only_on_work"), "{}", run.out);
    assert!(!run.says("HEAD no longer holds"), "{}", run.out);
}

#[test]
fn the_fallback_keeps_the_counter_and_drops_what_the_turn_it_left_held() {
    let tree = left_behind();
    records(&tree);
    tree.write("src/lib.rs", "pub fn on_main() -> i32 {\n    1\n}\n");

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.field("prompts"), "1");
    assert_eq!(tree.field("asked"), "");
    assert_eq!(tree.field("intervened"), "");
    assert_eq!(tree.field("followup"), "");
    assert_eq!(tree.field("mark"), "");
    assert_eq!(tree.revision("refs/worktree/klin/mark"), "");
}

#[test]
fn the_ref_cannot_bring_back_a_turn_the_checkout_left_behind() {
    let tree = left_behind();
    tree.write("src/lib.rs", text::WRAPPED);
    assert_eq!(stop(&tree).code, 2);
    assert_eq!(tree.revision("refs/worktree/klin/turn"), "");
    tree.remove(".git/klin/turn");

    let run = stop(&tree);
    assert!(run.says("no turn stamp resolves"), "{}", run.out);
    assert!(!run.says("test_only_on_work"), "{}", run.out);
    assert_eq!(tree.field("commit"), tree.revision("main"));
}

/// A stamp restored from the ref reads its parent as `commit^`, which names the stamped HEAD
/// for a synthetic stamp alone. A ref left pointing at an ordinary commit would name the commit
/// before the base, so a branch that forked there would pass the lineage test while HEAD held
/// no base at all. #238.
#[test]
fn a_branch_that_forked_before_the_base_cannot_restore_the_window_the_fallback_wrote() {
    let tree = Tree::bare();
    tree.repository();
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", CLEAN);
    tree.commit("the base");
    tree.write(
        "src/only_on_main.rs",
        "fn second(a: i32) -> i32 {\n    a + 2\n}\n",
    );
    tree.commit("a second commit on main");
    tree.git(&["checkout", "-q", "-b", "work"]);
    tree.write("src/work.rs", "fn third(a: i32) -> i32 {\n    a + 3\n}\n");
    tree.commit("the work the branch beside this one does not hold");
    prompt(&tree);
    tree.git(&["checkout", "-q", "main"]);
    assert_eq!(stop(&tree).code, 0);
    tree.remove(".git/klin/turn");
    tree.git(&["checkout", "-q", "-b", "older", "main~1"]);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no turn stamp resolves"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
}

#[test]
fn a_detached_head_outside_the_turn_history_judges_the_current_branch() {
    let tree = stamped();
    tree.git(&["checkout", "-q", "--detach", "main"]);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("HEAD no longer holds"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
}

#[test]
fn a_reset_that_drops_the_commit_the_turn_started_from_judges_the_branch() {
    let tree = stamped();
    tree.git(&["reset", "-q", "--hard", "main"]);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("HEAD no longer holds"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
}

#[test]
fn a_rebase_that_drops_the_commit_the_turn_started_from_judges_the_branch() {
    let tree = stamped();
    tree.git(&["config", "user.name", "klin"]);
    tree.git(&["config", "user.email", "klin@example.com"]);
    tree.git(&["checkout", "-q", "main"]);
    tree.write(
        "src/only_on_main.rs",
        "fn other(a: i32) -> i32 {\n    a + 2\n}\n",
    );
    tree.commit("a commit the branch does not hold");
    tree.git(&["checkout", "-q", "work"]);
    tree.git(&["rebase", "-q", "main"]);
    tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("HEAD no longer holds"), "{}", run.out);
    assert!(run.says("window: branch"), "{}", run.out);
}

#[test]
fn a_branch_made_at_the_same_head_keeps_the_turn_window() {
    let tree = stamped();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.git(&["checkout", "-q", "-b", "elsewhere"]);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert_eq!(
        tree.field("commit"),
        tree.revision("refs/worktree/klin/turn")
    );
}

#[test]
fn a_branch_that_descends_from_the_turn_keeps_the_turn_window() {
    let tree = stamped();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.commit("the agent committed its own debt");
    tree.git(&["checkout", "-q", "-b", "downstream"]);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert!(run.says("src/lib.rs"), "{}", run.out);
}
