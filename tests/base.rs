mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const CONFIG: &str = r#"{
  "project": "t",
  "escapes": { "roots": ["src"], "languages": ["rust"], "baseline": "quality/escapes-baseline.json" }
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
fn a_run_names_the_base_it_compares_against() {
    let tree = on_a_branch();

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("base: "), "{}", run.out);
    assert!(run.says("the merge-base with main"), "{}", run.out);
}

#[test]
fn the_json_run_names_the_base_too() {
    let tree = on_a_branch();

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let Ok(report) = serde_json::from_str::<Value>(run.out.trim()) else {
        panic!("not one JSON object: {}", run.out)
    };
    let base = report["base"].as_str().unwrap_or_default();
    assert!(base.contains("the merge-base with main"), "{base}");
}

#[test]
fn a_base_that_resolves_to_head_is_a_tool_error() {
    let tree = tree();
    tree.repository();
    tree.commit("everything on the default branch");

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("the base is HEAD"), "{}", run.out);
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
        r#"{ "project": "t", "doc_size": [{"file": "README.md", "ceiling": 10}] }"#,
    );
    tree.words("README.md", 5);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("base:"), "{}", run.out);
}
