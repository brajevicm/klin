mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const CONFIG: &str = r#"{
  "project": "t",
  "escapes": { "roots": ["src"], "languages": ["rust"] }
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

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("window: branch — base "), "{}", run.out);
    assert!(run.says("the merge-base with main"), "{}", run.out);
}

#[test]
fn the_json_run_names_the_window_too() {
    let tree = on_a_branch();

    let run = tree.run(&["gate", "--json"]);
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

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_base_that_resolves_to_head_ahead_of_the_remote_tip_is_a_tool_error() {
    let tree = at_the_remote_tip();
    tree.git(&["checkout", "-q", "-b", "trunk"]);
    tree.write("src/work.rs", CLEAN);
    tree.commit("a commit the remote does not have");

    let run = tree.run_with(&[("GITHUB_BASE_REF", "trunk")], &["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("origin/main does not hold"), "{}", run.out);
    assert!(run.says("a commit the remote does not have"), "{}", run.out);
}

#[test]
fn a_base_that_resolves_to_head_with_no_remote_tip_is_a_strict_failure_only() {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");

    let loose = tree.run(&["escapes"]);
    assert_eq!(loose.code, 0, "{}", loose.out);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 2, "{}", strict.out);
    assert!(
        strict.says("no remote default branch resolves"),
        "{}",
        strict.out
    );
}

#[test]
fn a_local_branch_named_like_a_remote_one_does_not_count_as_the_remote() {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");
    tree.git(&["branch", "origin/main"]);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 2, "{}", strict.out);
    assert!(
        strict.says("no remote default branch resolves"),
        "{}",
        strict.out
    );
}

#[test]
fn a_dirty_tree_whose_base_resolves_to_head_runs_the_gates() {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");
    tree.write("src/work.rs", CLEAN);

    let run = tree.run(&["gate"]);
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

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no base commit"), "{}", run.out);
}

#[test]
fn a_tree_that_is_not_a_repository_is_a_tool_error() {
    let tree = tree();

    let run = tree.run(&["gate"]);
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

    let run = tree.run_with(&[("GITHUB_BASE_REF", "release")], &["gate"]);
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

    let run = tree.run_with(&[("GITHUB_EVENT_PATH", &tree.at("event.json"))], &["gate"]);
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

    let run = tree.run_with(&[("GITHUB_EVENT_PATH", &tree.at("event.json"))], &["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the merge-base with main"), "{}", run.out);
}

#[test]
fn a_gate_that_does_not_compare_against_the_base_needs_no_base() {
    let tree = Tree::bare();
    tree.write(
        "klin.json",
        r#"{ "project": "t", "doc_citations": false,
             "doc_size": [{"file": "README.md", "ceiling": 10}] }"#,
    );
    tree.words("README.md", 5);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("window:"), "{}", run.out);
}

/// A repository whose klin.json sits in a subdirectory, with the debt already at the base.
fn in_a_subdirectory() -> Tree {
    let tree = Tree::bare();
    tree.write("proj/klin.json", CONFIG);
    tree.write("proj/src/lib.rs", text::WRAPPED);
    tree.write("README.md", "the tree above the project\n");
    tree.base();
    tree
}

fn in_the_project(tree: &Tree, args: &[&str]) -> harness::Run {
    harness::run_from(&tree.path("proj"), args)
}

#[test]
fn a_config_below_the_repository_root_holds_the_debt_the_base_holds() {
    let tree = in_a_subdirectory();
    tree.write("proj/src/other.rs", CLEAN);

    let whole = in_the_project(&tree, &["gate"]);
    assert_eq!(whole.code, 0, "{}", whole.out);
    assert!(!whole.says("src/lib.rs"), "{}", whole.out);
}

#[test]
fn a_config_below_the_repository_root_scopes_a_changed_run_the_same_way() {
    let tree = in_a_subdirectory();
    tree.write("proj/src/lib.rs", text::WRAPPED_WITH_A_NOTE);

    let scoped = in_the_project(&tree, &["gate", "--changed"]);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
    assert!(!scoped.says("src/lib.rs:2"), "{}", scoped.out);
}

#[test]
fn a_root_outside_the_tree_klin_compares_is_a_tool_error() {
    let tree = Tree::new();
    let elsewhere = Tree::bare();
    elsewhere.write("far/lib.rs", CLEAN);
    let outside = elsewhere.at("far");
    tree.write(
        "klin.json",
        &format!(r#"{{ "escapes": {{ "roots": [{outside:?}], "languages": ["rust"] }} }}"#),
    );
    tree.write("src/lib.rs", CLEAN);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("outside the tree klin compares"), "{}", run.out);
}
