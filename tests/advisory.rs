mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use std::path::Path;
use std::process::Command;

use harness::{Run, Tree};
use serde_json::Value;

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_CONTINUATION: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const CONFIG: &str = r#"{ "escapes": { "in": "src" } }"#;
const MOVED: &str = "the history moved";

/// A tree whose `origin` is a bare repository on disk, so a pull and a push are the real git
/// commands, and a second clone of it that commits as someone else.
struct Remote {
    tree: Tree,
    bare: tempfile::TempDir,
}

impl Remote {
    /// The agent on `work`, `origin/main` the default branch, and one prompt's stamp taken.
    fn new() -> Remote {
        let remote = Remote::unstamped(CONFIG);
        prompt(&remote.tree);
        remote
    }

    fn unstamped(config: &str) -> Remote {
        let tree = Tree::new();
        tree.write("klin.json", config);
        tree.write("src/lib.rs", CLEAN);
        tree.base();
        Remote::adopt(tree)
    }

    /// The same remote around a tree a test already built and based.
    fn adopt(tree: Tree) -> Remote {
        tree.git(&["config", "user.name", "klin"]);
        tree.git(&["config", "user.email", "klin@example.com"]);
        let bare = tempfile::tempdir().expect("temporary directory");
        git(bare.path(), &["init", "-q", "--bare", "-b", "main"]);
        tree.git(&[
            "remote",
            "add",
            "origin",
            &bare.path().display().to_string(),
        ]);
        tree.git(&["push", "-q", "origin", "main"]);
        tree.git(&["fetch", "-q", "origin"]);
        tree.git(&["remote", "set-head", "origin", "main"]);
        Remote { tree, bare }
    }

    /// A commit someone else pushes to `main`, holding a finding of its own.
    fn incoming(&self) {
        self.pushed("src/theirs.rs", text::ONE);
    }

    /// Someone force-pushes `main` back one commit, and the agent fetches it.
    fn rewound(&self) {
        let other = tempfile::tempdir().expect("temporary directory");
        let at = other.path().join("clone");
        git(
            other.path(),
            &[
                "clone",
                "-q",
                &self.bare.path().display().to_string(),
                "clone",
            ],
        );
        git(&at, &["push", "-q", "--force", "origin", "HEAD~1:main"]);
        self.tree.git(&["fetch", "-q", "origin"]);
    }

    /// A commit someone else pushes to `main` that writes one file.
    fn pushed(&self, file: &str, contents: &str) {
        let other = tempfile::tempdir().expect("temporary directory");
        let at = other.path().join("clone");
        git(
            other.path(),
            &[
                "clone",
                "-q",
                &self.bare.path().display().to_string(),
                "clone",
            ],
        );
        std::fs::write(at.join(file), contents).expect("write");
        git(&at, &["add", "-A"]);
        git(
            &at,
            &[
                "-c",
                "user.name=other",
                "-c",
                "user.email=other@example.com",
                "commit",
                "-q",
                "-m",
                "someone else's work",
            ],
        );
        git(&at, &["push", "-q", "origin", "main"]);
    }
}

fn git(at: &Path, args: &[&str]) {
    let done = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git");
    assert!(
        done.status.success(),
        "git {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&done.stderr)
    );
}

/// What a git command printed, whether or not it succeeded.
fn output(at: &Path, args: &[&str]) -> String {
    let done = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git");
    String::from_utf8_lossy(&done.stdout).to_string()
}

fn stop(tree: &Tree) -> Run {
    harness::feed(tree.root(), harness::AGENT, A_STOP)
}

fn prompt(tree: &Tree) {
    assert_eq!(harness::feed(tree.root(), harness::AGENT, A_PROMPT).code, 0);
}

/// The newest stop line of the journal.
fn last_stop(tree: &Tree) -> Value {
    let text = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .rfind(|line| line["kind"] == "stop")
        .unwrap_or_default()
}

/// An advisory stop: it blocks nothing for the finding it tells, says the history moved, writes
/// its reason, and takes a fresh stamp, so the stop after it is ordinary and finds nothing.
fn assert_advisory(tree: &Tree, reason: &str) {
    let before = tree.field("commit");
    let run = stop(tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(MOVED), "{}", run.out);
    assert!(run.says("klin check"), "{}", run.out);
    let line = last_stop(tree);
    assert_eq!(line["verdict"], "advisory", "{line}");
    assert_eq!(line["advisory"], reason, "{line}");
    assert_ne!(tree.field("commit"), before, "no fresh stamp");
    assert_eq!(tree.field("verdict"), "pending");
    assert_eq!(tree.field("asked"), "");

    let next = stop(tree);
    assert_eq!(next.code, 0, "{}", next.out);
    assert!(!next.says(MOVED), "{}", next.out);
    assert_ne!(last_stop(tree)["verdict"], "advisory");
}

#[test]
fn a_merge_of_the_default_branch_fetched_before_the_stamp_makes_one_stop_advisory() {
    let remote = Remote::unstamped(CONFIG);
    remote.incoming();
    remote.tree.git(&["fetch", "-q", "origin"]);
    prompt(&remote.tree);
    remote
        .tree
        .git(&["merge", "-q", "--no-edit", "origin/main"]);

    let run = stop(&remote.tree);
    assert!(run.says("src/theirs.rs"), "{}", run.out);
    let line = last_stop(&remote.tree);
    assert_eq!(line["verdict"], "advisory", "{line}");
    assert_eq!(line["advisory"], "incoming-commits", "{line}");
}

#[test]
fn a_pull_that_merges_the_default_branch_makes_the_stop_advisory() {
    let remote = Remote::new();
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);

    assert_advisory(&remote.tree, "incoming-commits");
}

#[test]
fn a_fast_forward_pull_of_the_default_branch_makes_the_stop_advisory() {
    let remote = Remote::unstamped(CONFIG);
    remote.tree.git(&["checkout", "-q", "main"]);
    prompt(&remote.tree);
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--ff-only", "origin", "main"]);

    assert_advisory(&remote.tree, "incoming-commits");
}

#[test]
fn a_pull_rebase_onto_the_default_branch_makes_the_stop_advisory() {
    let remote = Remote::new();
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--rebase", "origin", "main"]);

    assert_advisory(&remote.tree, "incoming-commits");
}

#[test]
fn an_advisory_stop_tells_the_finding_as_a_note_and_never_blocks() {
    let remote = Remote::new();
    remote.incoming();
    remote.tree.write("src/mine.rs", text::WRAPPED);
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);

    let run = stop(&remote.tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("src/mine.rs"), "{}", run.out);
    assert!(run.says("src/theirs.rs"), "{}", run.out);
    assert!(!run.says("gate block"), "{}", run.out);
}

#[test]
fn a_switch_to_a_branch_without_the_stamps_parent_makes_the_stop_advisory() {
    let remote = Remote::new();
    remote.tree.git(&["checkout", "-q", "main"]);

    assert_advisory(&remote.tree, "branch-changed");
}

#[test]
fn a_branch_made_at_the_same_head_keeps_the_turn_window() {
    let remote = Remote::new();
    remote.tree.git(&["switch", "-q", "-c", "another"]);
    remote.tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&remote.tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert_eq!(last_stop(&remote.tree)["verdict"], "red");
}

#[test]
fn a_stamp_and_a_ref_that_are_both_gone_make_the_stop_advisory() {
    let remote = Remote::new();
    remote.tree.remove(".git/klin/turn");
    remote
        .tree
        .git(&["update-ref", "-d", "refs/worktree/klin/turn"]);

    assert_advisory(&remote.tree, "stamp-missing");
}

/// The agent's own commit after the stamp, which the stamp is then taken over, so a rewrite of
/// that commit drops the stamp's parent from HEAD history.
fn committed(remote: &Remote) {
    remote.tree.write("notes.txt", "the agent's notes\n");
    remote.tree.commit("the agent's own commit");
    restamp(&remote.tree);
}

/// A green Stop and the prompt after it, which moves the stamp onto HEAD as it stands. A prompt
/// alone keeps a stamp no Stop judged yet.
fn restamp(tree: &Tree) {
    let run = stop(tree);
    assert_eq!(run.code, 0, "{}", run.out);
    prompt(tree);
    assert_eq!(
        tree.field("parent"),
        tree.revision("HEAD"),
        "the stamp did not move"
    );
}

/// The rewritten commit's red finding, judged against the stamp as any stop is.
fn assert_turn_blocks(tree: &Tree) {
    tree.write("src/lib.rs", text::WRAPPED);
    let run = stop(tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert!(!run.says(MOVED), "{}", run.out);
    assert_eq!(last_stop(tree)["verdict"], "red");
}

const EDIT_FIRST: &str = "sequence.editor=sed -i.bak -e 1s/^pick/edit/";

#[test]
fn a_paused_interactive_rebase_is_not_advisory() {
    let remote = Remote::new();
    committed(&remote);
    remote
        .tree
        .git(&["-c", EDIT_FIRST, "rebase", "-q", "-i", "HEAD~1"]);
    remote
        .tree
        .git(&["commit", "-q", "--amend", "-m", "reworded while paused"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn an_interactive_rebase_of_the_turns_commits_keeps_the_turn_window() {
    let remote = Remote::new();
    committed(&remote);
    remote
        .tree
        .git(&["-c", EDIT_FIRST, "rebase", "-q", "-i", "HEAD~1"]);
    remote
        .tree
        .git(&["commit", "-q", "--amend", "-m", "reworded"]);
    remote.tree.git(&["rebase", "--continue"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn an_amend_keeps_the_turn_window() {
    let remote = Remote::new();
    committed(&remote);
    remote
        .tree
        .git(&["commit", "-q", "--amend", "-m", "amended"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn a_soft_reset_keeps_the_turn_window() {
    let remote = Remote::new();
    committed(&remote);
    remote.tree.git(&["reset", "-q", "--soft", "HEAD~1"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn with_no_reflog_a_moved_merge_base_alone_makes_the_stop_advisory() {
    let remote = Remote::unstamped(CONFIG);
    remote
        .tree
        .git(&["config", "core.logAllRefUpdates", "false"]);
    std::fs::remove_dir_all(remote.tree.path(".git/logs")).expect("the reflogs");
    prompt(&remote.tree);
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);

    assert_advisory(&remote.tree, "incoming-commits");
}

#[test]
fn the_agents_own_push_never_makes_a_stop_advisory() {
    let remote = Remote::new();
    committed(&remote);
    remote.tree.git(&["push", "-q", "-u", "origin", "work"]);
    assert_turn_blocks(&remote.tree);
}

#[test]
fn a_push_of_the_default_branch_from_it_is_not_advisory() {
    let remote = Remote::unstamped(CONFIG);
    remote.tree.git(&["checkout", "-q", "main"]);
    prompt(&remote.tree);
    remote.tree.write("notes.txt", "the agent's notes\n");
    remote.tree.commit("the agent's own commit on main");
    remote.tree.git(&["push", "-q", "origin", "main"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn a_build_failure_at_an_advisory_stop_still_blocks_and_takes_no_fresh_stamp() {
    let remote = Remote::unstamped(r#"{ "build": "exit 1", "escapes": { "in": "src" } }"#);
    prompt(&remote.tree);
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);
    let before = remote.tree.field("commit");

    let run = stop(&remote.tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("does not build"), "{}", run.out);
    assert_eq!(remote.tree.field("commit"), before);
    assert_eq!(last_stop(&remote.tree)["verdict"], "red");
}

#[test]
fn an_advisory_stop_tells_an_unasked_deleted_test_and_klin_check_reviews_it() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/foo.py", "x = 1\n");
    tree.write("tests/test_foo.py", "def test_foo():\n    assert True\n");
    tree.base();
    let remote = Remote::adopt(tree);
    prompt(&remote.tree);
    remote.tree.remove("tests/test_foo.py");
    remote.tree.commit("the agent deleted a test");
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);

    let run = stop(&remote.tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(MOVED), "{}", run.out);
    assert!(run.says("tests/test_foo.py"), "{}", run.out);
    assert_eq!(last_stop(&remote.tree)["verdict"], "advisory");

    let check = remote.tree.run(&["check", "inventory", "--json"]);
    let report = check.json();
    assert_eq!(report["judgement"], "review", "{report}");
}

#[test]
fn a_host_continuation_after_an_advisory_stop_is_an_ordinary_stop() {
    let remote = Remote::new();
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);
    assert_eq!(stop(&remote.tree).code, 0);
    remote.tree.write("src/lib.rs", text::WRAPPED);

    let run = harness::feed(remote.tree.root(), harness::AGENT, A_CONTINUATION);
    assert!(!run.says(MOVED), "{}", run.out);
    assert!(run.says("src/lib.rs"), "{}", run.out);
    assert_eq!(last_stop(&remote.tree)["verdict"], "red");
}

#[test]
fn a_repository_with_no_remote_takes_no_advisory_stop() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree.git(&["config", "user.name", "klin"]);
    tree.git(&["config", "user.email", "klin@example.com"]);
    prompt(&tree);
    tree.git(&["checkout", "-q", "main"]);
    tree.write("src/theirs.rs", text::ONE);
    tree.commit("a commit on main");
    tree.git(&["checkout", "-q", "work"]);
    tree.git(&["merge", "-q", "--no-edit", "main"]);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(!run.says(MOVED), "{}", run.out);
    assert_eq!(last_stop(&tree)["verdict"], "red");
}

#[test]
fn a_stop_where_nothing_moved_starts_no_git_merge_base() {
    let remote = Remote::new();
    let trace = remote.tree.path("trace.log");
    let trace = trace.display().to_string();

    let run = harness::feed_with(
        remote.tree.root(),
        &[("GIT_TRACE", trace.as_str())],
        harness::AGENT,
        A_STOP,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    let traced = std::fs::read_to_string(&trace).unwrap_or_default();
    assert!(traced.contains("git"), "nothing traced: {traced}");
    assert!(!traced.contains("merge-base"), "{traced}");
}

#[test]
fn a_state_directory_deleted_while_the_ref_survives_restores_the_stamp() {
    let remote = Remote::new();
    let held = remote.tree.revision("refs/worktree/klin/turn");
    std::fs::remove_dir_all(remote.tree.path(".git/klin")).expect("the state directory");
    remote.tree.write("src/lib.rs", text::WRAPPED);

    let run = stop(&remote.tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("restored"), "{}", run.out);
    assert!(run.says("window: turn"), "{}", run.out);
    assert_eq!(remote.tree.field("commit"), held);
}

#[test]
fn status_names_the_default_branch_and_the_last_advisory_stop() {
    let remote = Remote::new();
    let status = remote.tree.run(&["status", "--json"]).json();
    assert_eq!(
        status["window"]["default_branch"], "refs/remotes/origin/main",
        "{status}"
    );
    assert_eq!(status["window"]["last_advisory"], Value::Null, "{status}");

    remote.tree.git(&["checkout", "-q", "main"]);
    assert_eq!(stop(&remote.tree).code, 0);
    let status = remote.tree.run(&["status", "--json"]).json();
    let last = &status["window"]["last_advisory"];
    assert_eq!(last["reason"], "branch-changed", "{status}");
    assert!(last["time"].is_u64(), "{status}");

    let text = remote.tree.run(&["status"]);
    assert!(text.says("refs/remotes/origin/main"), "{}", text.out);
    assert!(text.says("branch-changed"), "{}", text.out);
}

#[test]
fn the_reset_the_cache_clean_and_the_radius_report_commands_are_gone() {
    let tree = Tree::new();
    for args in [
        &["turn", "reset"][..],
        &["cache", "clean"][..],
        &["radius", "--report"][..],
    ] {
        let run = tree.run(args);
        assert_eq!(run.code, 2, "{args:?}: {}", run.out);
        assert!(run.says("unrecognized subcommand"), "{args:?}: {}", run.out);
    }
}

#[test]
fn a_merge_that_stopped_on_a_conflict_is_advisory_once_committed() {
    let remote = Remote::new();
    remote
        .tree
        .write("src/lib.rs", "fn mine(a: i32) -> i32 {\n    a + 2\n}\n");
    remote.tree.commit("the agent's own change");
    remote.pushed("src/lib.rs", "fn theirs(a: i32) -> i32 {\n    a + 3\n}\n");
    output(
        remote.tree.root(),
        &["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"],
    );
    remote
        .tree
        .write("src/lib.rs", "fn both(a: i32) -> i32 {\n    a + 5\n}\n");
    remote.tree.git(&["add", "-A"]);
    remote.tree.git(&["commit", "-q", "--no-edit"]);

    assert_advisory(&remote.tree, "incoming-commits");
}

#[test]
fn a_stop_that_lost_the_lock_blocks_nothing_on_a_history_move_and_writes_nothing() {
    let remote = Remote::new();
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);
    let before = std::fs::read_to_string(remote.tree.state("turn")).unwrap_or_default();
    let Ok(lock) = std::fs::File::create(remote.tree.state("lock")) else {
        panic!("the lock file could not be made")
    };
    assert!(lock.lock().is_ok(), "the test could not hold the lock");

    let run = stop(&remote.tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(MOVED), "{}", run.out);
    let after = std::fs::read_to_string(remote.tree.state("turn")).unwrap_or_default();
    assert_eq!(after, before, "a stop without the lock replaced the stamp");
}

#[test]
fn a_journal_line_names_an_advisory_reason_only_when_the_stop_was_advisory() {
    let remote = Remote::unstamped(r#"{ "build": "exit 1", "escapes": { "in": "src" } }"#);
    prompt(&remote.tree);
    remote.tree.git(&["checkout", "-q", "main"]);

    assert_eq!(stop(&remote.tree).code, 2);
    let line = last_stop(&remote.tree);
    assert_eq!(line["verdict"], "red", "{line}");
    assert_eq!(line.get("advisory"), None, "{line}");
}

#[test]
fn the_fresh_stamp_is_the_tree_the_stop_measured_and_holds_no_build_output() {
    let remote = Remote::unstamped(r#"{ "build": "touch built.txt", "escapes": { "in": "src" } }"#);
    prompt(&remote.tree);
    remote.tree.git(&["checkout", "-q", "main"]);

    assert_eq!(stop(&remote.tree).code, 0);
    assert_eq!(last_stop(&remote.tree)["verdict"], "advisory");
    assert!(
        remote.tree.path("built.txt").is_file(),
        "the build did not run"
    );
    let commit = remote.tree.field("commit");
    let listed = output(
        remote.tree.root(),
        &["ls-tree", "-r", "--name-only", &commit],
    );
    assert!(listed.contains("src/lib.rs"), "{listed}");
    assert!(!listed.contains("built.txt"), "{listed}");
}

/// A repository that keeps its refs, and HEAD's reflog with them, in a reftable, with the agent
/// on `main` and one prompt's stamp taken.
fn reftable() -> Remote {
    reftable_with(&[])
}

/// The same, with git configuration set before the first commit writes any reflog.
fn reftable_with(config: &[(&str, &str)]) -> Remote {
    let tree = Tree::bare();
    tree.git(&["init", "-q", "--ref-format=reftable", "-b", "main"]);
    for (key, value) in config {
        tree.git(&["config", key, value]);
    }
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", CLEAN);
    tree.commit("the base");
    let remote = Remote::adopt(tree);
    prompt(&remote.tree);
    remote
}

#[test]
fn in_a_reftable_repository_the_agents_own_push_of_main_is_not_advisory() {
    let remote = reftable();
    remote.tree.write("notes.txt", "the agent's notes\n");
    remote.tree.commit("the agent's own commit on main");
    remote.tree.git(&["push", "-q", "origin", "main"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn in_a_reftable_repository_a_pull_of_main_is_advisory() {
    let remote = reftable();
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--ff-only", "origin", "main"]);

    assert_advisory(&remote.tree, "incoming-commits");
}

/// The agent on the default branch, level with `origin/main`, and one prompt's stamp taken.
fn on_main() -> Remote {
    let remote = Remote::unstamped(CONFIG);
    remote.tree.git(&["checkout", "-q", "main"]);
    prompt(&remote.tree);
    remote
}

#[test]
fn a_soft_reset_on_the_default_branch_keeps_the_turn_window() {
    let remote = on_main();
    remote.tree.git(&["reset", "-q", "--soft", "HEAD~1"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn an_amend_after_a_push_of_the_default_branch_keeps_the_turn_window() {
    let remote = on_main();
    remote.tree.write("notes.txt", "the agent's notes\n");
    remote.tree.commit("the agent's own commit on main");
    remote.tree.git(&["push", "-q", "origin", "main"]);
    restamp(&remote.tree);
    remote
        .tree
        .git(&["commit", "-q", "--amend", "-m", "amended after the push"]);

    assert_turn_blocks(&remote.tree);
}

#[test]
fn a_reset_of_the_branch_onto_unrelated_history_is_lost_history() {
    let remote = Remote::new();
    remote
        .tree
        .git(&["checkout", "-q", "--orphan", "unrelated"]);
    remote
        .tree
        .commit("history the default branch does not share");
    remote
        .tree
        .git(&["checkout", "-q", "-B", "work", "unrelated"]);

    assert_advisory(&remote.tree, "history-lost");
}

#[test]
fn in_a_reftable_repository_with_no_reflog_a_pull_of_main_is_advisory() {
    let remote = reftable_with(&[("core.logAllRefUpdates", "false")]);
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--ff-only", "origin", "main"]);

    assert_advisory(&remote.tree, "incoming-commits");
}

#[test]
fn in_a_reftable_repository_a_paused_interactive_rebase_is_not_advisory() {
    let remote = reftable();
    committed(&remote);
    remote
        .tree
        .git(&["-c", EDIT_FIRST, "rebase", "-q", "-i", "HEAD~1"]);
    remote
        .tree
        .git(&["commit", "-q", "--amend", "-m", "reworded while paused"]);

    assert_turn_blocks(&remote.tree);
}

fn cursor_stop(generation: &str) -> String {
    format!(
        r#"{{"hook_event_name":"stop","cursor_version":"3.20.21","conversation_id":"s1","session_id":"s1","generation_id":"{generation}","loop_count":0}}"#
    )
}

/// Cursor tells a note as a `followup_message` it submits as the next prompt, which a Stop
/// without the lock cannot record, so it tells nothing. That Stop takes no fresh stamp either,
/// so the next Stop that holds the lock is advisory again and tells it. Spec 6.6, 9.1.
#[test]
fn on_cursor_a_stop_that_lost_the_lock_leaves_the_advisory_note_to_the_next_stop() {
    let remote = Remote::new();
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);
    let Ok(lock) = std::fs::File::create(remote.tree.state("lock")) else {
        panic!("the lock file could not be made")
    };
    assert!(lock.lock().is_ok(), "the test could not hold the lock");

    let quiet = harness::feed(remote.tree.root(), harness::AGENT, &cursor_stop("g1"));
    assert!(!quiet.printed.contains("followup_message"), "{}", quiet.out);
    let line = last_stop(&remote.tree);
    assert_eq!(line["advisory"], "incoming-commits", "{line}");
    drop(lock);

    let told = harness::feed(remote.tree.root(), harness::AGENT, &cursor_stop("g2"));
    assert!(told.printed.contains("followup_message"), "{}", told.out);
    assert!(told.says(MOVED), "{}", told.out);
    assert_eq!(last_stop(&remote.tree)["verdict"], "advisory");
}

/// The agent on `main` one commit past the base, level with `origin/main`, so a rewind of one
/// commit leaves a tree that still opts in, and one prompt's stamp taken.
fn ahead_on_main() -> Remote {
    let remote = Remote::unstamped(CONFIG);
    remote.incoming();
    remote.tree.git(&["checkout", "-q", "main"]);
    remote
        .tree
        .git(&["pull", "-q", "--ff-only", "origin", "main"]);
    prompt(&remote.tree);
    remote
}

#[test]
fn a_reset_onto_a_default_branch_someone_rewound_is_lost_history() {
    let remote = ahead_on_main();
    remote.rewound();
    remote.tree.git(&["reset", "-q", "--hard", "origin/main"]);

    assert_advisory(&remote.tree, "history-lost");
}

#[test]
fn a_stop_between_the_fetch_of_a_rewind_and_the_reset_onto_it_does_not_adopt_it() {
    let remote = ahead_on_main();
    remote.rewound();
    let between = stop(&remote.tree);
    assert_eq!(between.code, 0, "{}", between.out);
    assert_ne!(last_stop(&remote.tree)["verdict"], "advisory");
    remote.tree.git(&["reset", "-q", "--hard", "origin/main"]);

    assert_advisory(&remote.tree, "history-lost");
}

#[test]
fn status_names_an_advisory_stop_that_lost_the_lock() {
    let remote = Remote::new();
    remote.incoming();
    remote
        .tree
        .git(&["pull", "-q", "--no-rebase", "--no-edit", "origin", "main"]);
    let Ok(lock) = std::fs::File::create(remote.tree.state("lock")) else {
        panic!("the lock file could not be made")
    };
    assert!(lock.lock().is_ok(), "the test could not hold the lock");
    assert_eq!(stop(&remote.tree).code, 0);
    drop(lock);

    let status = remote.tree.run(&["status", "--json"]).json();
    let last = &status["window"]["last_advisory"];
    assert_eq!(last["reason"], "incoming-commits", "{status}");
}
