mod harness;

use harness::Tree;

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;
const BUILD_BLOCKED: &str = ".klin-build-blocked";

const GATES: &str = r#""doc_size": [{"file": "README.md", "ceiling": 10}],
  "complexity": { "sources": ["src"], "ceilings": {"cc": 8, "lines": 60} }"#;

fn tree(build: &str) -> Tree {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        &format!("{{\n  \"project\": \"t\",\n  {build}\n  {GATES}\n}}"),
    );
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

fn stop(tree: &Tree, event: &str, args: &[&str]) -> harness::Run {
    harness::feed(tree.root(), args, event)
}

#[test]
fn a_failing_build_blocks_the_stop_and_no_gate_runs() {
    let tree = tree(r#""build": "echo the-compiler-spoke; exit 1","#);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("the tree does not build"), "{}", run.out);
    assert!(run.says("the-compiler-spoke"), "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
}

#[test]
fn a_failing_build_blocks_the_second_stop_too() {
    let tree = tree(r#""build": "exit 1","#);

    let run = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("the tree does not build"), "{}", run.out);
}

#[test]
fn a_turn_that_failed_to_build_is_still_blocked_when_a_gate_fails() {
    let tree = tree(r#""build": "test ! -f fails","#);
    tree.write("fails", "");
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(first.code, 2, "{}", first.out);
    assert!(first.says("the tree does not build"), "{}", first.out);

    assert!(std::fs::remove_file(tree.path("fails")).is_ok());
    let second = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(second.code, 2, "{}", second.out);
    assert!(second.says("FAIL  doc-size"), "{}", second.out);
    assert!(!second.says("not blocking a second time"), "{}", second.out);
}

#[test]
fn a_config_with_no_build_key_runs_the_gates_with_no_build_step() {
    let tree = tree("");

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(BUILD_BLOCKED).exists());
}

const TWO_PROJECTS: &str = r#""build": [
    {"root": "api", "run": "echo api >> ../ran"},
    {"root": "web", "run": "echo web >> ../ran"}
  ],"#;

fn ran(tree: &Tree) -> String {
    std::fs::read_to_string(tree.path("ran")).unwrap_or_default()
}

fn monorepo(build: &str) -> Tree {
    let tree = tree(build);
    tree.write("api/Cargo.toml", "");
    tree.write("web/package.json", "");
    tree.base();
    tree
}

#[test]
fn changed_runs_only_the_entries_whose_root_holds_a_changed_file() {
    let tree = monorepo(TWO_PROJECTS);
    tree.write("api/src/lib.rs", CLEAN);

    let run = stop(&tree, A_STOP, &["gate", "--hook", "--changed"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "api\n");
}

#[test]
fn a_changed_file_under_no_root_runs_every_entry() {
    let tree = monorepo(TWO_PROJECTS);
    tree.write("src/work.rs", CLEAN);

    let run = stop(&tree, A_STOP, &["gate", "--hook", "--changed"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "api\nweb\n");
}

#[test]
fn without_changed_every_entry_runs_in_order() {
    let tree = monorepo(TWO_PROJECTS);
    tree.write("api/src/lib.rs", CLEAN);

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "api\nweb\n");
}

#[test]
fn the_first_failing_entry_blocks_and_the_ones_after_it_do_not_run() {
    let tree = monorepo(
        r#""build": [
    {"root": "api", "run": "echo api-broke; exit 1"},
    {"root": "web", "run": "echo web >> ../ran"}
  ],"#,
    );

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("api-broke"), "{}", run.out);
    assert_eq!(ran(&tree), "");
}

#[test]
fn a_rename_out_of_a_root_builds_the_root_it_left_as_well() {
    let tree = monorepo(TWO_PROJECTS);
    tree.write("api/src/moves.rs", CLEAN);
    tree.base();
    tree.git(&["mv", "api/src/moves.rs", "web/moves.rs"]);

    let run = stop(&tree, A_STOP, &["gate", "--hook", "--changed"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "api\nweb\n");
}

#[test]
fn a_build_klin_cannot_read_reports_and_blocks_only_the_first_stop() {
    let tree = tree(r#""build": [{"root": "api"}],"#);

    let first = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(first.code, 2, "{}", first.out);
    assert!(first.says("has no \"run\""), "{}", first.out);

    let second = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(second.says("not blocking a second time"), "{}", second.out);
}

#[test]
fn a_failing_build_under_json_prints_one_json_object() {
    let tree = tree(r#""build": "echo the-compiler-spoke; exit 1","#);

    let run = stop(&tree, A_STOP, &["gate", "--hook", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report: serde_json::Value = match serde_json::from_str(run.out.trim()) {
        Ok(report) => report,
        Err(why) => panic!("{why} — the run printed:\n{}", run.out),
    };
    assert_eq!(report["status"], "ERROR", "{report}");
    assert!(
        report["findings"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("the-compiler-spoke"),
        "{report}"
    );
}

#[test]
fn a_build_key_is_not_read_outside_the_hook() {
    let tree = tree(r#""build": "exit 1","#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
}
