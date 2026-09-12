mod harness;

use harness::Tree;

const CLEAN: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;
const BUILD_BLOCKED: &str = ".git/klin/build-blocked";

const GATES: &str = r#""doc_size": [{"file": "README.md", "ceiling": 10}],
  "complexity": { "roots": ["src"], "ceilings": {"cc": 8, "lines": 60} }"#;

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
    tree.write("api/Cargo.toml", "[package]\nname = \"api\"\n");
    tree.write("web/package.json", "{}\n");
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
    assert_eq!(report["exit"], serde_json::json!(2), "{report}");
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

const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;

fn blocked(tree: &Tree, times: usize) {
    for at in 1..=times {
        let run = stop(tree, A_STOP, &["gate", "--hook"]);
        assert_eq!(run.code, 2, "stop {at} of {times}: {}", run.out);
    }
}

#[test]
fn a_failing_build_blocks_eight_stops_under_one_prompt_and_the_ninth_reports() {
    let tree = tree(r#""build": "echo the-compiler-spoke; exit 1","#);
    blocked(&tree, 8);

    let ninth = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(ninth.code, 0, "{}", ninth.out);
    assert!(ninth.says("the-compiler-spoke"), "{}", ninth.out);
    assert!(ninth.says("stops blocking"), "{}", ninth.out);
}

#[test]
fn a_new_prompt_restores_the_eight_build_blocks() {
    let tree = tree(r#""build": "exit 1","#);
    blocked(&tree, 8);
    let spent = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(spent.code, 0, "{}", spent.out);

    let prompt = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(prompt.code, 0, "{}", prompt.out);
    let after = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(after.code, 2, "{}", after.out);
}

#[test]
fn a_passing_build_inside_one_prompt_does_not_restore_the_build_blocks() {
    let tree = tree(r#""build": "test ! -f fails","#);
    tree.write("fails", "");
    blocked(&tree, 8);

    tree.remove("fails");
    let green = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(green.code, 0, "{}", green.out);

    tree.write("fails", "");
    let after = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(after.code, 0, "{}", after.out);
    assert!(after.says("stops blocking"), "{}", after.out);
}

#[test]
fn a_build_failure_writes_a_red_verdict_and_the_next_prompt_keeps_the_stamp() {
    let tree = tree(r#""build": "exit 1","#);

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(tree.field("verdict"), "red", "{}", run.out);

    let held = tree.field("commit");
    let prompt = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(prompt.code, 0, "{}", prompt.out);
    assert_eq!(tree.field("commit"), held, "a red build moved the stamp");
}

#[test]
fn the_gates_one_block_is_spent_apart_from_the_build_blocks() {
    let tree = tree(r#""build": "test ! -f fails","#);
    tree.write("fails", "");
    tree.words("README.md", 30);

    let build = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(build.code, 2, "{}", build.out);

    tree.remove("fails");
    let gate = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(gate.code, 2, "{}", gate.out);
    assert!(gate.says("FAIL  doc-size"), "{}", gate.out);

    let again = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(again.code, 0, "{}", again.out);
    assert!(again.says("not blocking a second time"), "{}", again.out);
}

#[test]
fn the_ninth_build_failure_under_json_records_that_klin_stopped_blocking() {
    let tree = tree(r#""build": "exit 1","#);
    blocked(&tree, 8);

    let ninth = stop(&tree, A_STOP, &["gate", "--hook", "--json"]);
    assert_eq!(ninth.code, 0, "{}", ninth.out);
    let report: serde_json::Value = match serde_json::from_str(ninth.out.trim()) {
        Ok(report) => report,
        Err(why) => panic!("{why} — the run printed:\n{}", ninth.out),
    };
    assert!(
        report["notes"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("stops blocking"),
        "{report}"
    );
    assert_eq!(report["exit"], serde_json::json!(0), "{report}");
    assert_eq!(report["status"], "ERROR", "{report}");
}

#[test]
fn a_build_count_klin_cannot_write_reports_the_failure_and_blocks_nothing() {
    let tree = tree(r#""build": "echo the-compiler-spoke; exit 1","#);
    let held = tree.path(BUILD_BLOCKED);
    assert!(std::fs::create_dir_all(&held).is_ok(), "{}", held.display());

    for at in 1..=3 {
        let run = stop(&tree, A_STOP, &["gate", "--hook"]);
        assert_eq!(run.code, 0, "stop {at}: {}", run.out);
        assert!(run.says("the-compiler-spoke"), "stop {at}: {}", run.out);
        assert!(run.says("blocks nothing"), "stop {at}: {}", run.out);
    }
}

#[test]
fn a_passing_stop_leaves_the_gates_one_block_unspent() {
    let tree = tree(r#""build": "test ! -f fails","#);
    tree.write("fails", "");

    let build = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(build.code, 2, "{}", build.out);

    tree.remove("fails");
    let green = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(green.code, 0, "{}", green.out);

    tree.words("README.md", 30);
    let gate = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(gate.code, 2, "{}", gate.out);
    assert!(!gate.says("not blocking a second time"), "{}", gate.out);
}
