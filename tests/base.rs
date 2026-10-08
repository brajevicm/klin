mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const CONFIG: &str = r#"{
  "escapes": { "in": "src" }
}"#;

fn tree() -> Tree {
    let tree = Tree::bare();
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", CLEAN);
    tree
}

fn on_a_branch() -> Tree {
    let tree = tree();
    tree.repository();
    tree.base();
    tree.write("src/work.rs", CLEAN);
    tree
}

#[test]
fn a_run_names_the_window_it_compares_against() {
    let tree = on_a_branch();

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("window: branch — base "), "{}", run.out);
    assert!(run.says("the merge-base with main"), "{}", run.out);
}

#[test]
fn the_json_run_names_the_window_too() {
    let tree = on_a_branch();

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let Ok(report) = serde_json::from_str::<Value>(run.out.trim()) else {
        panic!("not one JSON object: {}", run.out)
    };
    let window = report["window"].clone();
    assert_eq!(window["kind"], "branch", "{window}");
    assert_eq!(window["before"], tree.revision("main"), "{window}");
    assert!(
        window["how"]
            .as_str()
            .unwrap_or_default()
            .contains("the merge-base with main"),
        "{window}"
    );
}

/// A tree on the default branch, whose one commit the remote already has.
fn at_the_remote_tip() -> Tree {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");
    tree.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    tree
}

#[test]
fn a_base_that_resolves_to_head_at_the_remote_tip_runs_the_gates() {
    let tree = at_the_remote_tip();

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

/// A local source whose HEAD the remote default branch does not hold hides the unpushed
/// commits, so the comparison is a hole, not a pass. Spec 6.5.
#[test]
fn a_local_base_equal_to_head_with_unpushed_commits_is_comparison_unproven() {
    let tree = at_the_remote_tip();
    tree.git(&["checkout", "-q", "-b", "trunk"]);
    tree.write("src/work.rs", CLEAN);
    tree.commit("a commit the remote does not have");

    let run = tree.run_with(&[("GITHUB_BASE_REF", "trunk")], &["check"]);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("HOLE: comparison-unproven"), "{}", run.out);
    assert!(run.says("origin/main does not hold"), "{}", run.out);
    assert!(run.says("a commit the remote does not have"), "{}", run.out);

    let json = tree.run_with(&[("GITHUB_BASE_REF", "trunk")], &["check", "--json"]);
    assert_eq!(json.code, 3, "{}", json.out);
    let report = json.json();
    assert_eq!(report["measurement"], "incomplete", "{report}");
    assert_eq!(
        report["measurements"][0]["holes"][0]["reason"], "comparison-unproven",
        "{report}"
    );
}

/// With no remote at all, nothing proves what to compare, and the run passes with a note.
/// Spec 6.5.
#[test]
fn a_base_equal_to_head_with_no_remote_passes_with_a_note() {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE: the base is HEAD"), "{}", run.out);
    assert!(run.says("no remote proves what to compare"), "{}", run.out);
}

#[test]
fn a_local_branch_named_like_a_remote_one_does_not_count_as_the_remote() {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");
    tree.git(&["branch", "origin/main"]);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("no remote proves what to compare"), "{}", run.out);
}

/// The pull request target must be fetched, so a `GITHUB_BASE_REF` that does not resolve is an
/// error that names `fetch-depth`, and never a fall back to another base. Spec 6.5.
#[test]
fn a_pull_request_target_that_does_not_resolve_names_fetch_depth() {
    let tree = on_a_branch();

    let run = tree.run_with(&[("GITHUB_BASE_REF", "release")], &["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("release does not resolve"), "{}", run.out);
    assert!(run.says("fetch-depth: 0"), "{}", run.out);
}

/// A tree whose push event names a commit no history holds, as a force-push leaves it.
fn pushed_from_a_rewritten_commit() -> Tree {
    let tree = on_a_branch();
    tree.commit("work on the branch");
    tree.write(
        "event.json",
        "{\"before\": \"1234567890123456789012345678901234567890\"}",
    );
    tree
}

#[test]
fn a_push_base_a_force_push_rewrote_away_falls_back_to_the_merge_base_with_a_note() {
    let tree = pushed_from_a_rewritten_commit();

    let run = tree.run_with(&[("GITHUB_EVENT_PATH", &tree.at("event.json"))], &["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the merge-base with main"), "{}", run.out);
    assert!(
        run.says("NOTE: the push started from 1234567, which this repository no longer holds"),
        "{}",
        run.out
    );
}

#[test]
fn a_push_base_missing_from_a_shallow_clone_names_fetch_depth() {
    let tree = pushed_from_a_rewritten_commit();
    let head = tree.revision("HEAD");
    tree.write(".git/shallow", &format!("{head}\n"));

    let run = tree.run_with(&[("GITHUB_EVENT_PATH", &tree.at("event.json"))], &["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("1234567890123456789012345678901234567890 does not resolve"),
        "{}",
        run.out
    );
    assert!(run.says("fetch-depth: 0"), "{}", run.out);
}

/// The check document records HEAD and whether the tree was dirty, and no tree hash. Spec 6.5.
#[test]
fn the_json_tree_names_head_and_whether_the_tree_was_dirty() {
    let tree = on_a_branch();

    let run = tree.run(&["check", "--json"]);
    let report = run.json();
    assert_eq!(
        report["tree"],
        serde_json::json!({"head": tree.revision("HEAD"), "dirty": true}),
        "{report}"
    );
}

#[test]
fn a_dirty_tree_whose_base_resolves_to_head_runs_the_gates() {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");
    tree.write("src/work.rs", CLEAN);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_repository_whose_only_reference_is_head_is_a_tool_error() {
    let tree = tree();
    tree.repository();
    tree.commit("the one commit");
    tree.git(&["checkout", "-q", "--detach"]);
    tree.git(&["branch", "-q", "-D", "main"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no base commit"), "{}", run.out);
    assert!(
        run.says("none of these resolved: origin/main, origin/master, main, master"),
        "{}",
        run.out
    );
}

#[test]
fn a_tree_that_is_not_a_repository_is_a_tool_error() {
    let tree = tree();

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no base commit"), "{}", run.out);
}

#[test]
fn under_a_pull_request_the_base_is_the_target_tip() {
    let tree = tree();
    tree.repository();
    tree.commit("base");
    tree.git(&["checkout", "-q", "-b", "release"]);
    tree.write("src/release.rs", CLEAN);
    tree.commit("a commit only the target has");
    tree.git(&["checkout", "-q", "main"]);
    tree.git(&["checkout", "-q", "-b", "work"]);
    tree.write("src/work.rs", CLEAN);
    tree.commit("work on the branch");

    let run = tree.run_with(&[("GITHUB_BASE_REF", "release")], &["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the tip of release"), "{}", run.out);
    let tip = tree.revision("release");
    assert!(run.says(&tip[..7]), "{}\nwanted {tip}", run.out);
}

#[test]
fn under_a_push_the_base_is_the_commit_the_event_names() {
    let tree = tree();
    tree.repository();
    tree.commit("before the push");
    let before = tree.revision("HEAD");
    tree.write("src/first.rs", CLEAN);
    tree.commit("first of two");
    tree.write("src/second.rs", CLEAN);
    tree.commit("second of two");
    tree.write("event.json", &format!("{{\"before\": \"{before}\"}}"));

    let run = tree.run_with(&[("GITHUB_EVENT_PATH", &tree.at("event.json"))], &["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the commit this push started from"), "{}", run.out);
    assert!(run.says(&before[..7]), "{}\nwanted {before}", run.out);
}

#[test]
fn a_push_event_naming_no_previous_commit_falls_back_to_the_branch() {
    let tree = on_a_branch();
    tree.write(
        "event.json",
        "{\"before\": \"0000000000000000000000000000000000000000\"}",
    );

    let run = tree.run_with(&[("GITHUB_EVENT_PATH", &tree.at("event.json"))], &["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the merge-base with main"), "{}", run.out);
}

#[test]
fn a_gate_that_does_not_compare_against_the_base_needs_no_base() {
    let tree = Tree::bare();
    tree.write(
        "klin.json",
        r#"{ "doc_citations": false,
             "doc_size": {"README.md": 10},
             "escapes": false, "stubs": false, "complexity": false,
             "dead_symbols": false, "reachability": false }"#,
    );
    tree.words("README.md", 5);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("window:"), "{}", run.out);
}

/// A repository whose worktree root holds one klin.json and a subdirectory holds another.
fn in_a_subdirectory(root: Option<&str>) -> Tree {
    let tree = Tree::new();
    if let Some(root) = root {
        tree.write("klin.json", root);
    }
    tree.write("proj/klin.json", r#"{"doc_size": false}"#);
    tree.words("README.md", 5);
    tree.write("proj/notes.txt", "kept\n");
    tree.base();
    tree
}

/// Every command reads the worktree root's klin.json, wherever it starts, and names a nested one
/// as ignored rather than reading it. Spec 5.1.
#[test]
fn a_config_below_the_worktree_root_is_ignored_and_named() {
    let tree = in_a_subdirectory(Some(r#"{"doc_size": {"README.md": 10}}"#));

    let run = harness::run_from(&tree.path("proj"), &["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: README.md is 5 words, ceiling 10"),
        "{}",
        run.out
    );
    assert!(
        run.says(&format!("{} is ignored", tree.at("proj/klin.json"))),
        "{}",
        run.out
    );
}

/// A klin.json in a subdirectory alone turns nothing on. The session start in that directory
/// tells the person to move it, and every other event answers nothing. Spec 5.1.
#[test]
fn a_config_below_the_worktree_root_alone_tells_the_hooks_to_move_it() {
    let tree = in_a_subdirectory(None);
    let at = tree.path("proj");

    let session = harness::feed(&at, harness::AGENT, harness::SESSION_START);
    assert_eq!(session.code, 0, "{}", session.out);
    assert!(session.says("systemMessage"), "{}", session.out);
    assert!(session.says(&tree.at("proj/klin.json")), "{}", session.out);
    assert!(session.says("Move the file there"), "{}", session.out);

    let stop = harness::feed(&at, harness::AGENT, harness::STOP);
    assert_eq!(stop.code, 0, "{}", stop.out);
    assert_eq!(stop.out, "", "{}", stop.out);
    assert!(!tree.path(".git/klin").exists(), "a hook wrote state");
}

#[test]
fn a_scope_outside_the_tree_klin_compares_is_a_tool_error() {
    let tree = Tree::new();
    let elsewhere = Tree::bare();
    elsewhere.write("far/lib.rs", CLEAN);
    let outside = elsewhere.at("far");
    tree.write(
        "klin.json",
        &format!(r#"{{ "escapes": {{ "in": {outside:?} }} }}"#),
    );
    tree.write("src/lib.rs", CLEAN);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("is absolute"), "{}", run.out);
}
