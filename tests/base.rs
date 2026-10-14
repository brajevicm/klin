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

#[test]
fn a_merge_base_a_shallow_clone_cannot_compute_names_fetch_depth() {
    let tree = on_a_branch();
    tree.commit("work on the branch");
    let head = tree.revision("HEAD");
    tree.write(".git/shallow", &format!("{head}\n"));

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("the merge-base with main does not resolve"),
        "{}",
        run.out
    );
    assert!(run.says("fetch-depth: 0"), "{}", run.out);
}

/// `child` cut from `parent`, which holds an escape that `main` does not, with both remote
/// branches fetched. Spec 6.5.
fn stacked() -> Tree {
    let tree = tree();
    tree.repository();
    tree.commit("the default branch");
    tree.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    tree.git(&["checkout", "-q", "-b", "parent"]);
    tree.write("src/theirs.rs", text::ONE);
    tree.commit("the parent branch's work");
    tree.git(&["update-ref", "refs/remotes/origin/parent", "HEAD"]);
    tree.git(&["checkout", "-q", "-b", "child"]);
    tree.write("src/work.rs", CLEAN);
    tree.commit("the child branch's work");
    tree
}

#[test]
fn a_branch_stacked_on_another_remote_branch_is_judged_against_it() {
    let tree = stacked();

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the merge-base with origin/parent (inferred)"),
        "{}",
        run.out
    );
}

#[test]
fn the_branch_base_key_overrides_the_inferred_base() {
    let tree = stacked();
    tree.git(&["config", "branch.child.klin-base", "origin/main"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("the merge-base with origin/main (branch.child.klin-base)"),
        "{}",
        run.out
    );
}

#[test]
fn a_branch_base_key_that_does_not_resolve_falls_back_to_the_inferred_base() {
    let tree = stacked();
    tree.git(&["config", "branch.child.klin-base", "gone"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("branch.child.klin-base names gone, which does not resolve"),
        "{}",
        run.out
    );
    assert!(run.says("origin/parent (inferred)"), "{}", run.out);
}

#[test]
fn a_fork_point_two_remote_branches_hold_is_judged_against_the_default_branch() {
    let tree = stacked();
    tree.git(&[
        "update-ref",
        "refs/remotes/origin/twin",
        "refs/remotes/origin/parent",
    ]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("the merge-base with origin/main"), "{}", run.out);
    assert!(
        run.says("origin/parent, origin/twin all hold"),
        "{}",
        run.out
    );
}

#[test]
fn the_pull_request_target_wins_over_the_branch_base() {
    let tree = stacked();
    tree.git(&["config", "branch.child.klin-base", "origin/parent"]);

    let run = tree.run_with(&[("GITHUB_BASE_REF", "main")], &["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("the tip of main, the pull request target"),
        "{}",
        run.out
    );
}

/// A branch cut from the default branch tracks it, and the default branch's last merge holds a
/// branch that is still on the remote. The default branch is the base. Spec 6.5.
#[test]
fn a_branch_that_tracks_the_default_branch_is_judged_against_it() {
    let tree = stacked();
    tree.git(&["checkout", "-q", "main"]);
    tree.git(&[
        "-c",
        "user.name=klin",
        "-c",
        "user.email=klin@example.com",
        "merge",
        "-q",
        "--no-ff",
        "--no-edit",
        "parent",
    ]);
    tree.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    tree.git(&["checkout", "-q", "-b", "tracking", "origin/main"]);
    tree.git(&[
        "config",
        "remote.origin.fetch",
        "+refs/heads/*:refs/remotes/origin/*",
    ]);
    tree.git(&["config", "branch.tracking.remote", "origin"]);
    tree.git(&["config", "branch.tracking.merge", "refs/heads/main"]);
    tree.write("src/next.rs", CLEAN);
    tree.commit("work on the default branch");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the merge-base with origin/main"), "{}", run.out);
    assert!(!run.says("inferred"), "{}", run.out);
}

fn merged(tree: &Tree, branch: &str) {
    tree.git(&[
        "-c",
        "user.name=klin",
        "-c",
        "user.email=klin@example.com",
        "merge",
        "-q",
        "--no-edit",
        branch,
    ]);
}

/// A merge brings in a branch tip that is not the fork point: the fork point stays on the
/// first-parent chain. Spec 6.5.
#[test]
fn a_stacked_branch_that_merged_the_default_branch_stays_stacked() {
    let tree = stacked();
    tree.git(&["checkout", "-q", "main"]);
    tree.write("src/later.rs", CLEAN);
    tree.commit("later work on the default branch");
    tree.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    tree.git(&["checkout", "-q", "child"]);
    merged(&tree, "origin/main");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the merge-base with origin/parent (inferred)"),
        "{}",
        run.out
    );
}

/// The merged branch's work still reads as new, as it does on any branch cut from the default
/// branch. Spec 6.5, 18.7.
#[test]
fn a_branch_cut_from_the_default_branch_that_merged_a_teammates_branch_is_not_stacked() {
    let tree = stacked();
    tree.git(&["checkout", "-q", "-b", "mine", "main"]);
    tree.write("src/mine.rs", CLEAN);
    tree.commit("my work");
    merged(&tree, "origin/parent");
    tree.write("src/after.rs", CLEAN);
    tree.commit("more of my work");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("the merge-base with origin/main"), "{}", run.out);
    assert!(!run.says("inferred"), "{}", run.out);
}

#[test]
fn a_copy_of_the_branch_on_another_remote_is_not_its_base() {
    let tree = stacked();
    tree.git(&["update-ref", "refs/remotes/fork/child", "HEAD"]);
    tree.write("src/unpushed.rs", CLEAN);
    tree.commit("work the fork does not hold");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the merge-base with origin/parent (inferred)"),
        "{}",
        run.out
    );
}

#[test]
fn a_branch_base_key_that_names_a_tag_falls_back_to_the_inferred_base() {
    let tree = stacked();
    tree.git(&["tag", "v1", "main"]);
    tree.git(&["config", "branch.child.klin-base", "v1"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("branch.child.klin-base names v1, which does not resolve"),
        "{}",
        run.out
    );
}

/// Another branch that holds HEAD, such as a snapshot pushed from this one, would hide every
/// commit of the branch as a base. It also holds the fork point, so klin infers nothing and the
/// default branch is the base. Spec 6.5.
#[test]
fn a_branch_base_key_that_already_holds_head_is_refused() {
    let tree = stacked();
    tree.write("src/mine.rs", text::ONE);
    tree.commit("work with an escape");
    tree.git(&["update-ref", "refs/remotes/origin/snapshot", "HEAD"]);
    tree.git(&["config", "branch.child.klin-base", "origin/snapshot"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("branch.child.klin-base names origin/snapshot, which already holds HEAD"),
        "{}",
        run.out
    );
    assert!(run.says("the merge-base with origin/main"), "{}", run.out);
}

/// `origin/HEAD` names `origin/main`, so a legacy `origin/master` that holds the fork point is an
/// ordinary branch: the fork point is ambiguous, and the note names both. Spec 6.5.
#[test]
fn a_fallback_default_name_past_the_remote_head_is_an_ordinary_branch() {
    let tree = stacked();
    tree.git(&[
        "symbolic-ref",
        "refs/remotes/origin/HEAD",
        "refs/remotes/origin/main",
    ]);
    tree.git(&[
        "update-ref",
        "refs/remotes/origin/master",
        "refs/remotes/origin/parent",
    ]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("origin/master, origin/parent all hold"),
        "{}",
        run.out
    );
}

/// `origin/child` is behind HEAD, so it does not hold HEAD, and is still this branch's own copy:
/// as a base it would hide the escape in the pushed commit. Spec 6.5.
#[test]
fn a_branch_base_key_that_names_a_copy_behind_head_is_refused() {
    let tree = stacked();
    tree.write("src/mine.rs", text::ONE);
    tree.commit("pushed work with an escape");
    tree.git(&["update-ref", "refs/remotes/origin/child", "HEAD"]);
    tree.write("src/next.rs", CLEAN);
    tree.commit("unpushed clean work");
    tree.git(&["config", "branch.child.klin-base", "origin/child"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("branch.child.klin-base names origin/child, which is this branch or a copy of it"),
        "{}",
        run.out
    );
    assert!(run.says("origin/parent (inferred)"), "{}", run.out);
}

/// `origin/team/child` is another branch, not a copy of `child`. Spec 6.5.
#[test]
fn a_parent_whose_name_ends_in_the_branch_name_is_still_its_base() {
    let tree = tree();
    tree.repository();
    tree.commit("the default branch");
    tree.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    tree.git(&["checkout", "-q", "-b", "team/child"]);
    tree.write("src/theirs.rs", text::ONE);
    tree.commit("the team branch's work");
    tree.git(&["update-ref", "refs/remotes/origin/team/child", "HEAD"]);
    tree.git(&["checkout", "-q", "-b", "child"]);
    tree.write("src/work.rs", CLEAN);
    tree.commit("the child branch's work");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the merge-base with origin/team/child (inferred)"),
        "{}",
        run.out
    );
}

/// A remote's name may hold a slash, so `upstream/fork/child` is a copy of `child` where
/// `upstream/fork` is a remote: neither its inference nor its key may make the copy the base and
/// hide the escape the copy holds. Spec 6.5.
#[test]
fn a_copy_on_a_remote_whose_name_holds_a_slash_is_not_its_base() {
    let tree = stacked();
    tree.git(&[
        "remote",
        "add",
        "upstream/fork",
        "https://example.invalid/fork.git",
    ]);
    tree.write("src/mine.rs", text::ONE);
    tree.commit("pushed work with an escape");
    tree.git(&["update-ref", "refs/remotes/upstream/fork/child", "HEAD"]);
    tree.write("src/next.rs", CLEAN);
    tree.commit("unpushed clean work");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("origin/parent (inferred)"), "{}", run.out);

    tree.git(&["config", "branch.child.klin-base", "upstream/fork/child"]);
    let keyed = tree.run(&["check"]);
    assert_eq!(keyed.code, 1, "{}", keyed.out);
    assert!(
        keyed.says("names upstream/fork/child, which is this branch or a copy of it"),
        "{}",
        keyed.out
    );
}
