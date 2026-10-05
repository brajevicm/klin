mod harness;

use std::path::Path;
use std::process::Command;

use harness::{Run, Tree};

const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const A_SESSION: &str = r#"{"hook_event_name": "SessionStart"}"#;
const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

fn radius(tree: &Tree, event: &str) -> Run {
    harness::feed(tree.root(), &["radius"], event)
}

fn stamp(tree: &Tree) -> serde_json::Value {
    let text = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    serde_json::from_str(&text).unwrap_or_default()
}

fn verdict(tree: &Tree, said: &str) {
    let mut held = stamp(tree);
    held["verdict"] = said.into();
    tree.write(".git/klin/turn", &held.to_string());
}

fn git_out(cwd: &Path, args: &[&str]) -> String {
    let done = Command::new("git").arg("-C").arg(cwd).args(args).output();
    match done {
        Ok(done) => String::from_utf8_lossy(&done.stdout).trim().to_string(),
        Err(why) => panic!("git {} could not run: {why}", args.join(" ")),
    }
}

#[test]
fn a_first_session_stamps_the_working_tree_over_head() {
    let tree = tree();
    tree.write("src/new.rs", CLEAN);
    tree.write("src/lib.rs", "fn changed() {}\n");

    let before = tree.status();
    let run = radius(&tree, A_SESSION);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.status(), before, "the stamp touched the working tree");

    let held = tree.revision("refs/worktree/klin/turn");
    assert!(!held.is_empty(), "the ref holds no stamp: {}", run.out);
    assert_eq!(tree.field("commit"), held);
    assert_eq!(
        git_out(tree.root(), &["rev-parse", "refs/worktree/klin/turn^"]),
        tree.revision("HEAD"),
        "HEAD is not the stamp's parent"
    );
    let listed = git_out(
        tree.root(),
        &["ls-tree", "-r", "--name-only", "refs/worktree/klin/turn"],
    );
    assert!(listed.contains("src/new.rs"), "{listed}");
    assert_eq!(
        git_out(tree.root(), &["show", "refs/worktree/klin/turn:src/lib.rs"]),
        "fn changed() {}"
    );
}

#[test]
fn an_ignored_file_stays_out_of_the_stamp() {
    let tree = tree();
    tree.write(".gitignore", "build/\n");
    tree.write("build/out.rs", CLEAN);

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    let listed = git_out(
        tree.root(),
        &["ls-tree", "-r", "--name-only", "refs/worktree/klin/turn"],
    );
    assert!(!listed.contains("build/out.rs"), "{listed}");
}

#[test]
fn a_red_verdict_keeps_the_stamp_and_a_green_one_moves_it() {
    let tree = tree();
    assert_eq!(radius(&tree, A_SESSION).code, 0);
    let first = tree.field("commit");
    verdict(&tree, "red");
    tree.write("src/lib.rs", "fn one() {}\n");

    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    assert_eq!(tree.field("commit"), first, "a red stop moved the stamp");
    assert_eq!(radius(&tree, A_SESSION).code, 0);
    assert_eq!(
        tree.field("commit"),
        first,
        "a session start moved the stamp after a red stop"
    );

    verdict(&tree, "green");
    tree.write("src/lib.rs", "fn two() {}\n");
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    let moved = tree.field("commit");
    assert_ne!(moved, first, "a green stop did not move the stamp");
    assert_eq!(tree.revision("refs/worktree/klin/turn"), moved);
    assert_eq!(
        git_out(tree.root(), &["show", "refs/worktree/klin/turn:src/lib.rs"]),
        "fn two() {}"
    );
}

#[test]
fn a_leftover_private_index_lock_does_not_keep_a_green_turn_open() {
    let tree = tree();
    assert_eq!(radius(&tree, A_SESSION).code, 0);
    let first = tree.field("commit");
    verdict(&tree, "green");
    tree.write(".git/klin/index.lock", "");
    tree.write("src/lib.rs", "fn next_turn() {}\n");

    for _ in 0..2 {
        let run = radius(&tree, A_PROMPT);
        assert_eq!(run.code, 0, "{}", run.out);
        assert!(!run.says("git could not stamp"), "{}", run.out);
        assert_ne!(tree.field("commit"), first, "a green turn stayed open");
    }
    assert_eq!(
        git_out(tree.root(), &["show", "refs/worktree/klin/turn:src/lib.rs"]),
        "fn next_turn() {}"
    );
}

#[test]
fn every_prompt_and_session_start_raises_the_counter() {
    let tree = tree();

    for expected in ["1", "2", "3"] {
        assert_eq!(radius(&tree, A_PROMPT).code, 0);
        assert_eq!(tree.field("prompts"), expected);
    }
    verdict(&tree, "red");
    assert_eq!(radius(&tree, A_SESSION).code, 0);
    assert_eq!(
        tree.field("prompts"),
        "4",
        "a stamp that stayed put lost the count"
    );
}

#[test]
fn a_turn_file_that_is_gone_is_restored_red_from_the_ref() {
    let tree = tree();
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    verdict(&tree, "green");
    let held = tree.field("commit");
    tree.remove(".git/klin/turn");

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("restored"), "{}", run.out);
    assert_eq!(tree.field("commit"), held, "the restored stamp moved");
    assert_eq!(tree.field("verdict"), "red");
}

#[test]
fn a_stamp_and_a_ref_that_are_both_gone_get_no_fresh_stamp() {
    let tree = tree();
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    tree.remove(".git/klin/turn");
    tree.git(&["update-ref", "-d", "refs/worktree/klin/turn"]);

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert_eq!(
        tree.revision("refs/worktree/klin/turn"),
        "",
        "a fresh stamp went over the deletion"
    );
    assert_eq!(tree.field("commit"), "");
}

#[test]
fn a_state_directory_another_command_made_is_still_a_first_session() {
    let tree = tree();
    tree.write(".git/klin/build-blocked", "");

    let run = radius(&tree, A_SESSION);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
    assert_eq!(
        tree.field("commit"),
        tree.revision("refs/worktree/klin/turn"),
        "a stop's own file cost the first prompt its stamp"
    );
}

#[test]
fn a_state_directory_that_is_gone_reads_the_ref_before_it_stamps_afresh() {
    let tree = tree();
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    let held = tree.field("commit");
    tree.write("src/lib.rs", "fn hidden_debt() {}\n");
    assert!(std::fs::remove_dir_all(tree.path(".git/klin")).is_ok());

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("restored"), "{}", run.out);
    assert_eq!(
        tree.field("commit"),
        held,
        "deleting the state directory photographed the debt"
    );
}

#[test]
fn a_stamp_that_stays_gone_says_so_on_every_prompt() {
    let tree = tree();
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    tree.remove(".git/klin/turn");
    tree.git(&["update-ref", "-d", "refs/worktree/klin/turn"]);
    assert!(radius(&tree, A_PROMPT).says("both gone"));

    let run = radius(&tree, A_PROMPT);
    assert!(run.says("both gone"), "{}", run.out);
    assert_eq!(tree.field("prompts"), "2");
    assert_eq!(tree.field("commit"), "");
}

#[test]
fn the_turn_file_is_written_whole_and_the_ref_never_leaves_the_tree() {
    let tree = tree();
    let clone = Tree::bare();
    tree.git(&["clone", "-q", ".", &clone.at("")]);

    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    assert!(tree.state("turn").is_file());
    assert!(
        !tree.state("turn.writing").exists(),
        "a half-written stamp stayed behind"
    );
    tree.git(&["remote", "add", "there", &clone.at("")]);
    tree.git(&["push", "-q", "there", "--all"]);
    assert_eq!(
        git_out(clone.root(), &["for-each-ref", "refs/worktree/"]),
        "",
        "the stamp ref was pushed"
    );
}

#[test]
fn two_worktrees_keep_their_own_stamp_through_a_prune() {
    let tree = tree();
    let beside = Tree::bare();
    let at = beside.path("side");
    tree.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "side",
        &at.display().to_string(),
    ]);
    assert!(std::fs::write(at.join("src/lib.rs"), "fn beside() {}\n").is_ok());

    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    assert_eq!(harness::feed(&at, &["radius"], A_PROMPT).code, 0);
    let mine = tree.revision("refs/worktree/klin/turn");
    let theirs = git_out(&at, &["rev-parse", "refs/worktree/klin/turn"]);
    assert_ne!(mine, theirs, "the two worktrees share one stamp");

    tree.git(&["gc", "--prune=now", "-q"]);
    assert_eq!(tree.revision("refs/worktree/klin/turn"), mine);
    assert_eq!(git_out(tree.root(), &["cat-file", "-t", &mine]), "commit");
    assert_eq!(
        git_out(&at, &["rev-parse", "refs/worktree/klin/turn"]),
        theirs,
        "a prune in one worktree moved another worktree's ref"
    );
}

#[test]
fn outside_a_repository_the_stamp_says_nothing_and_blocks_nothing() {
    let tree = Tree::bare();

    let run = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_person_resets_the_stamp_after_a_red_stop() {
    let tree = tree();
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    let first = tree.field("commit");
    verdict(&tree, "red");
    tree.write("src/lib.rs", "fn abandoned() {}\n");

    let run = tree.run(&["turn", "reset"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("a person moved"), "{}", run.out);
    assert_ne!(tree.field("commit"), first, "the reset left the stamp put");
    assert_eq!(tree.field("verdict"), "red");
    assert_eq!(
        git_out(tree.root(), &["show", "refs/worktree/klin/turn:src/lib.rs"]),
        "fn abandoned() {}"
    );
}

#[test]
fn a_reset_after_a_green_stop_leaves_a_red_stamp_the_next_prompt_keeps() {
    let tree = tree();
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    verdict(&tree, "green");

    assert_eq!(tree.run(&["turn", "reset"]).code, 0);
    assert_eq!(tree.field("verdict"), "red");
    let moved = tree.field("commit");
    tree.write("src/lib.rs", "fn after_the_reset() {}\n");

    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    assert_eq!(
        tree.field("commit"),
        moved,
        "the prompt after a reset moved the stamp over an unjudged tree"
    );
}

#[test]
fn a_reset_keeps_the_prompt_counter() {
    let tree = tree();
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    assert_eq!(radius(&tree, A_PROMPT).code, 0);

    assert_eq!(tree.run(&["turn", "reset"]).code, 0);
    assert_eq!(tree.field("prompts"), "2");
}

#[test]
fn a_reset_outside_a_repository_says_why() {
    let tree = Tree::bare();

    let run = tree.run(&["turn", "reset"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("git repository"), "{}", run.out);
}
