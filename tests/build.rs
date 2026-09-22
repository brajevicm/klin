mod harness;

use harness::Tree;

const CLEAN: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;
const BUILD_BLOCKED: &str = ".git/klin/build-blocked";

const GATES: &str = r#""doc_size": {"README.md": 10},
  "complexity": { "in": "src", "cc": 8, "lines": 60 }"#;

fn tree(build: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", &format!("{{\n  {build}\n  {GATES}\n}}"));
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
fn a_build_entry_without_run_is_a_config_error() {
    let tree = tree(r#""build": [{"root": "api"}],"#);

    let first = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(first.code, 1, "{}", first.out);
    assert!(
        first.says("a \"build\" entry has no \"run\""),
        "{}",
        first.out
    );

    let second = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(second.code, 1, "{}", second.out);
    assert!(
        second.says("a \"build\" entry has no \"run\""),
        "{}",
        second.out
    );
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

/// Stops that each follow an edit, because a stop over an unchanged tree spends no block.
fn blocked(tree: &Tree, times: usize) {
    for at in 1..=times {
        tree.write("edited", &at.to_string());
        let run = stop(tree, A_STOP, &["gate", "--hook"]);
        assert_eq!(run.code, 2, "stop {at} of {times}: {}", run.out);
    }
}

#[test]
fn a_stop_over_an_unchanged_tree_is_reported_and_not_blocked_again() {
    let tree = tree(r#""build": "echo the-compiler-spoke; exit 1","#);

    let first = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(first.code, 2, "{}", first.out);
    assert!(first.says("block 1 of 8"), "{}", first.out);

    let again = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(again.code, 0, "{}", again.out);
    assert!(again.says("the-compiler-spoke"), "{}", again.out);
    assert!(again.says("did not change"), "{}", again.out);

    tree.write(
        "src/lib.rs",
        "pub fn simple(a: i32) -> i32 {\n    a + 2\n}\n",
    );
    let edited = stop(&tree, A_SECOND_STOP, &["gate", "--hook"]);
    assert_eq!(edited.code, 2, "{}", edited.out);
    assert!(edited.says("block 2 of 8"), "{}", edited.out);
}

#[test]
fn a_build_whose_command_is_missing_is_a_note_and_the_gates_run() {
    let tree = tree(r#""build": "klin-no-such-tool --noEmit","#);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(
        run.says("the build `klin-no-such-tool --noEmit` could not run"),
        "{}",
        run.out
    );
    assert!(
        run.says("Install the project's dependencies"),
        "{}",
        run.out
    );
    assert!(!run.says("does not build"), "{}", run.out);
}

#[test]
fn a_missing_command_skips_its_entry_and_a_failing_entry_after_it_still_blocks() {
    let tree = tree(
        r#""build": [
    {"root": "api", "run": "klin-no-such-tool"},
    {"root": "web", "run": "echo the-compiler-spoke; exit 1"}
  ],"#,
    );
    tree.write("api/keep", "");
    tree.write("web/keep", "");

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("the tree does not build"), "{}", run.out);
    assert!(run.says("the-compiler-spoke"), "{}", run.out);
}

#[test]
fn a_missing_build_command_is_told_when_nothing_blocks() {
    let tree = tree(r#""build": "klin-no-such-tool","#);

    let run = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("nothing blocks the stop"), "{}", run.out);
    assert!(run.says("could not run"), "{}", run.out);
}

#[test]
fn a_failing_build_blocks_eight_stops_under_one_prompt_and_the_ninth_reports() {
    let tree = tree(r#""build": "echo the-compiler-spoke; exit 1","#);
    blocked(&tree, 8);

    tree.write("edited", "9");
    let ninth = stop(&tree, A_STOP, &["gate", "--hook"]);
    assert_eq!(ninth.code, 0, "{}", ninth.out);
    assert!(ninth.says("the-compiler-spoke"), "{}", ninth.out);
    assert!(ninth.says("stops blocking"), "{}", ninth.out);
}

#[test]
fn a_new_prompt_restores_the_eight_build_blocks() {
    let tree = tree(r#""build": "exit 1","#);
    blocked(&tree, 8);
    tree.write("edited", "9");
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
    tree.write("edited", "9");
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

    tree.write("edited", "9");
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

/// A fake toolchain on the path, which records each command the hook runs so a test reads which
/// derived build ran where, and never runs a real compiler.
fn toolchain(tree: &Tree) -> String {
    for tool in ["cargo", "tsc", "go"] {
        tree.write(
            &format!("toolchain/{tool}"),
            &format!(
                "#!/bin/sh\necho \"{tool} $(basename \"$PWD\")\" >> \"{}\"\n",
                tree.at("ran")
            ),
        );
        let path = tree.path(&format!("toolchain/{tool}"));
        let Ok(held) = std::fs::metadata(&path) else {
            panic!("no {}", path.display())
        };
        let mut mode = held.permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
        assert!(std::fs::set_permissions(&path, mode).is_ok());
    }
    format!("{}:/usr/bin:/bin", tree.at("toolchain"))
}

fn derived(tree: &Tree, path: &str, args: &[&str]) -> harness::Run {
    harness::feed_with(tree.root(), &[("PATH", path)], args, A_STOP)
}

/// With no `build` key the hook builds with the command each standard manifest derives, one per
/// project of a monorepo, and the report says where each came from. ADR 0012, ADR 0040.
#[test]
fn with_no_build_key_the_hook_derives_one_command_per_project() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("api/Cargo.toml", "[package]\nname = \"api\"\n");
    tree.write("api/src/lib.rs", CLEAN);
    tree.write("web/package.json", "{\"name\": \"web\"}\n");
    tree.write("web/tsconfig.json", "{}\n");
    tree.write("web/src/index.ts", "export const a = 1;\n");
    tree.base();
    let path = toolchain(&tree);
    tree.write(
        "api/src/lib.rs",
        "pub fn simple(a: i32) -> i32 {\n    a + 2\n}\n",
    );

    let run = derived(&tree, &path, &["gate", "--hook", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "cargo api\ntsc web\n", "{}", run.out);
    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    assert!(
        journal.contains("\"rule\":\"one command per manifest\""),
        "{journal}"
    );
}

#[test]
fn a_derived_build_that_fails_blocks_the_stop_and_names_its_command() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    let path = toolchain(&tree);
    tree.write(
        "toolchain/cargo",
        "#!/bin/sh\necho the-compiler-spoke\nexit 1\n",
    );

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("$ cargo build --all-targets"), "{}", run.out);
    assert!(run.says("the-compiler-spoke"), "{}", run.out);
    assert!(
        run.says(
            "derived: build cargo build --all-targets from Cargo.toml, one command per manifest"
        ),
        "{}",
        run.out
    );
}

#[test]
fn a_derived_build_whose_tool_is_absent_names_the_manifest_and_runs_the_gates() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("package.json", "{\"name\": \"web\"}\n");
    tree.write("tsconfig.json", "{}\n");
    tree.write("src/index.ts", "export const a = 1;\n");
    tree.base();

    let run = derived(&tree, "/usr/bin:/bin", &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the build `tsc --noEmit` could not run"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: build tsc --noEmit from package.json beside tsconfig.json"),
        "{}",
        run.out
    );
    assert!(!run.says("does not build"), "{}", run.out);
}

#[test]
fn a_build_set_to_false_builds_nothing_where_a_manifest_would_derive_one() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"build": false}"#);
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.write("src/lib.rs", CLEAN);
    tree.base();

    let run = derived(&tree, "/usr/bin:/bin", &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("does not build"), "{}", run.out);
}

#[test]
fn a_pinned_command_replaces_the_derived_one() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"build": "echo pinned >> ran"}"#);
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    let path = toolchain(&tree);

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "pinned\n", "{}", run.out);
}

/// An executable script in the tree, as an installed compiler is.
fn installed(tree: &Tree, path: &str, body: &str) {
    tree.write(path, body);
    let at = tree.path(path);
    let Ok(held) = std::fs::metadata(&at) else {
        panic!("no {}", at.display())
    };
    let mut mode = held.permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
    assert!(std::fs::set_permissions(&at, mode).is_ok());
}

fn typescript(tree: &Tree, at: &str) {
    tree.write(&format!("{at}package.json"), "{\"name\": \"web\"}\n");
    tree.write(&format!("{at}tsconfig.json"), "{}\n");
    tree.write(&format!("{at}src/index.ts"), "export const a = 1;\n");
}

/// A derived TypeScript build runs the compiler the project installed beside its manifest, with
/// no compiler on `PATH`, and a compile error blocks the stop. Spec 9.3.
#[test]
fn a_derived_build_runs_the_compiler_installed_beside_the_manifest() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    installed(
        &tree,
        "node_modules/.bin/tsc",
        "#!/bin/sh\necho the-project-compiler-spoke\nexit 1\n",
    );
    tree.base();

    let run = derived(&tree, "/usr/bin:/bin", &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("the-project-compiler-spoke"), "{}", run.out);
    assert!(
        run.says("resolved: build node_modules/.bin/tsc --noEmit"),
        "{}",
        run.out
    );
    assert!(!run.says("could not run"), "{}", run.out);
}

/// Of two installed compilers the nearest one above the manifest runs, and the report names the
/// command it resolved, relative to the directory that entry runs in. Spec 5.4, 9.3.
#[test]
fn a_derived_build_runs_the_nearest_compiler_above_the_manifest() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "web/app/");
    for at in ["node_modules", "web/node_modules"] {
        installed(
            &tree,
            &format!("{at}/.bin/tsc"),
            &format!(
                "#!/bin/sh\necho \"{at}\" >> \"{}\"\nexit 1\n",
                tree.at("ran")
            ),
        );
    }
    tree.base();
    tree.write("web/app/src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, "/usr/bin:/bin", &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(ran(&tree), "web/node_modules\n", "{}", run.out);
    assert!(
        run.says("resolved: build ../node_modules/.bin/tsc --noEmit in web/app"),
        "{}",
        run.out
    );
    assert!(!run.says("could not run"), "{}", run.out);
}

/// With no compiler installed in the tree, a compiler on `PATH` still runs. Spec 9.3.
#[test]
fn a_derived_build_falls_back_to_the_compiler_on_the_path() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.base();
    let path = toolchain(&tree);
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(ran(&tree).starts_with("tsc "), "{}", ran(&tree));
}

/// A person's `build` string runs exactly as written, whatever the tree installed. Spec 5.2.
#[test]
fn a_configured_build_is_not_rewritten_by_an_installed_compiler() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"build": "tsc --noEmit"}"#);
    typescript(&tree, "");
    installed(&tree, "node_modules/.bin/tsc", "#!/bin/sh\nexit 1\n");
    tree.base();

    let run = derived(&tree, "/usr/bin:/bin", &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the build `tsc --noEmit` could not run"),
        "{}",
        run.out
    );
}

/// Resolution stops at the root klin measures: a compiler above that root is not the project's.
/// Spec 9.3.
#[test]
fn a_derived_build_does_not_reach_a_compiler_above_the_root() {
    let tree = Tree::new();
    tree.write("app/klin.json", "{}");
    typescript(&tree, "app/");
    installed(&tree, "node_modules/.bin/tsc", "#!/bin/sh\nexit 0\n");
    tree.base();
    tree.write("app/src/index.ts", "export const a = 2;\n");

    let run = harness::feed_with(
        &tree.path("app"),
        &[("PATH", "/usr/bin:/bin")],
        &["gate", "--hook"],
        A_STOP,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the build `tsc --noEmit` could not run"),
        "{}",
        run.out
    );
}

/// A compiler the project installed that cannot run is still the project's compiler: the build
/// fails and klin does not compile with the one on `PATH`. Spec 9.3.
#[test]
fn a_local_compiler_that_cannot_run_does_not_fall_through_to_the_path() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write("node_modules/.bin/tsc", "#!/bin/sh\nexit 0\n");
    tree.base();
    let path = toolchain(&tree);
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(ran(&tree), "", "{}", run.out);
}

/// A Plug'n'Play checkout installs no `node_modules`, so the compiler runs through Yarn's own
/// binary and no `PATH` compiler is reached. Spec 9.3.
#[test]
fn a_plug_and_play_checkout_runs_the_compiler_through_yarn() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    tree.write("yarn.lock", "# yarn lockfile v1\n");
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!(
            "#!/bin/sh\necho \"yarn $* $COREPACK_ENABLE_NETWORK\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        ran(&tree),
        "yarn bin tsc 0\nyarn run -B tsc --noEmit 0\n",
        "{}",
        run.out
    );
}

/// Resolution is for the tool a JavaScript package manager installs alone. A `node_modules/.bin`
/// entry named for another language's tool does not shadow that language's derived command.
/// Spec 9.3.
#[test]
fn an_installed_binary_does_not_shadow_a_derived_cargo_or_go_command() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("api/Cargo.toml", "[package]\nname = \"api\"\n");
    tree.write("api/src/lib.rs", CLEAN);
    tree.write("svc/go.mod", "module svc\n");
    tree.write("svc/main.go", "package main\n\nfunc main() {}\n");
    for tool in ["cargo", "go"] {
        installed(
            &tree,
            &format!("node_modules/.bin/{tool}"),
            "#!/bin/sh\nexit 1\n",
        );
    }
    tree.base();
    let path = toolchain(&tree);
    tree.write(
        "api/src/lib.rs",
        "pub fn simple(a: i32) -> i32 {\n    a + 2\n}\n",
    );

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "cargo api\ngo svc\n", "{}", run.out);
}

/// The derived value is the command the table names, whatever the checkout installed, so the
/// journal and the order the entries run in do not move when dependencies arrive. Spec 5.4.
#[test]
fn an_installed_compiler_does_not_change_the_derived_value_or_the_order() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.write("src/lib.rs", CLEAN);
    typescript(&tree, "web/");
    installed(
        &tree,
        "web/node_modules/.bin/tsc",
        &format!("#!/bin/sh\necho \"tsc web\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.base();
    let path = toolchain(&tree);
    tree.write(
        "src/lib.rs",
        "pub fn simple(a: i32) -> i32 {\n    a + 2\n}\n",
    );
    tree.write("web/src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let ran = ran(&tree);
    let mut lines = ran.lines();
    assert!(
        lines.next().unwrap_or_default().starts_with("cargo "),
        "{ran}"
    );
    assert_eq!(lines.next(), Some("tsc web"), "{ran}");
    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    assert!(journal.contains("cargo build --all-targets"), "{journal}");
    assert!(journal.contains("tsc --noEmit"), "{journal}");
    assert!(!journal.contains("node_modules"), "{journal}");
}

/// A broken install, whose `node_modules/.bin` link points nowhere, fails its own build. The
/// shell exits 127 for it, and that is an absent tool only for a command klin did not resolve.
/// Spec 9.3.
#[test]
fn a_broken_install_fails_its_own_build_and_is_not_an_absent_tool() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write("node_modules/.bin/keep", "");
    let bin = tree.path("node_modules/.bin/tsc");
    assert!(std::os::unix::fs::symlink("../typescript/bin/tsc", &bin).is_ok());
    tree.base();
    let path = toolchain(&tree);
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(!run.says("could not run"), "{}", run.out);
    assert_eq!(ran(&tree), "", "{}", run.out);
}

/// A Yarn that cannot run answers nothing about the checkout, so the build runs through it and
/// its own exit decides: the tree is unmeasured and klin compiles with no other compiler.
/// Spec 9.3.
#[test]
fn a_plug_and_play_checkout_with_no_yarn_is_unmeasured() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(".pnp.js", "// a Yarn 2.0 Plug'n'Play checkout\n");
    tree.write(".yarnrc.yml", "nodeLinker: pnp\n");
    tree.base();
    let path = toolchain(&tree);
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("could not run"), "{}", run.out);
    assert_eq!(ran(&tree), "", "{}", run.out);
}

/// A `node_modules` that holds the tool wins over a Plug'n'Play marker beside it, so a marker a
/// move away from Plug'n'Play left behind does not take the run. Spec 9.3.
#[test]
fn an_installed_tool_wins_over_a_plug_and_play_marker_beside_it() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(
        ".pnp.cjs",
        "// a marker a move away from Plug'n'Play left\n",
    );
    installed(
        &tree,
        "node_modules/.bin/tsc",
        &format!(
            "#!/bin/sh\necho \"the installed tool\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "the installed tool\n", "{}", run.out);
}

/// A Plug'n'Play checkout whose Yarn holds no such binary is a checkout with no project
/// compiler, so the compiler on `PATH` runs. #285, Spec 9.3.
#[test]
fn a_plug_and_play_checkout_whose_yarn_holds_no_tool_falls_back_to_the_path() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    tree.write("yarn.lock", "# yarn lockfile v1\n");
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        "#!/bin/sh\necho 'Usage Error: Couldn'\\''t find a binary named tsc' >&2\nexit 1\n",
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(ran(&tree).starts_with("tsc "), "{}", ran(&tree));
}

/// A marker an old install model left beside the manifest does not take the run from a tool
/// installed above it: the whole chain is searched for an installed tool first. Spec 9.3.
#[test]
fn a_nearer_marker_does_not_take_the_run_from_a_tool_installed_above_it() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "packages/app/");
    tree.write(
        "packages/app/.pnp.cjs",
        "// a marker an old install model left\n",
    );
    installed(
        &tree,
        "node_modules/.bin/tsc",
        &format!(
            "#!/bin/sh\necho \"the hoisted tool\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("packages/app/src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "the hoisted tool\n", "{}", run.out);
}

/// Yarn Classic writes the same `.pnp.js` and has no binaries-only form of `yarn run`, so a
/// Classic checkout is no model klin resolves and the tool on `PATH` runs. Spec 9.3.
#[test]
fn a_yarn_classic_checkout_is_not_resolved_through_yarn() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(".pnp.js", "// a Yarn Classic Plug'n'Play checkout\n");
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(ran(&tree).starts_with("tsc "), "{}", ran(&tree));
}

/// A repository inside a tree, with a base commit and a branch, as the harness makes for a whole
/// tree. This fixture needs the repository below the temporary directory, so that a compiler
/// outside the repository still sits inside the directory the test owns.
fn repository(at: &std::path::Path) {
    let git = |args: &[&str]| {
        let done = std::process::Command::new("git")
            .arg("-C")
            .arg(at)
            .args(args)
            .output();
        match done {
            Ok(done) if done.status.success() => (),
            other => panic!("git {}: {other:?}", args.join(" ")),
        }
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    let who = [
        "-c",
        "user.name=klin",
        "-c",
        "user.email=klin@example.com",
        "-c",
        "commit.gpgsign=false",
    ];
    git(&[&who[..], &["commit", "-q", "-m", "the base"]].concat());
    git(&["checkout", "-q", "-B", "work"]);
    git(&[
        &who[..],
        &["commit", "-q", "--allow-empty", "-m", "on the branch"],
    ]
    .concat());
}

/// Resolution never walks above the repository the configuration sits in, so a compiler outside
/// that repository is not the project's. Spec 5.1, 9.3.
#[test]
fn a_derived_build_does_not_reach_a_compiler_above_the_repository() {
    let tree = Tree::bare();
    tree.write("repo/klin.json", "{}");
    typescript(&tree, "repo/");
    installed(&tree, "node_modules/.bin/tsc", "#!/bin/sh\nexit 0\n");
    repository(&tree.path("repo"));
    tree.write("repo/src/index.ts", "export const a = 2;\n");

    let run = harness::feed_with(
        &tree.path("repo"),
        &[("PATH", "/usr/bin:/bin")],
        &["gate", "--hook"],
        A_STOP,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("the build `tsc --noEmit` could not run"),
        "{}",
        run.out
    );
}

/// The call klin makes between two states no file tells apart, where no configuration names the
/// model: a `node_modules` a move to Plug'n'Play left behind still wins over the marker, because
/// a binary is evidence that the tool is there to run. Spec 9.3.
#[test]
fn a_node_modules_left_behind_by_a_move_to_plug_and_play_still_wins() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "packages/app/");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    tree.write("yarn.lock", "# yarn lockfile v1\n");
    installed(
        &tree,
        "packages/app/node_modules/.bin/tsc",
        &format!(
            "#!/bin/sh\necho \"the tool left behind\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("packages/app/src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "the tool left behind\n", "{}", run.out);
}

/// A `.pnp.cjs` says how a project installs its tools and not whose project it is. A pnpm
/// checkout runs the tool through pnpm, and Yarn is never called. Spec 9.3.
#[test]
fn a_pnpm_plug_and_play_checkout_runs_the_tool_through_pnpm() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    tree.write("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    tree.base();
    let path = toolchain(&tree);
    for manager in ["yarn", "pnpm"] {
        installed(
            &tree,
            &format!("toolchain/{manager}"),
            &format!(
                "#!/bin/sh\necho \"{manager} $*\" >> \"{}\"\n",
                tree.at("ran")
            ),
        );
    }
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "pnpm exec tsc --noEmit\n", "{}", run.out);
}

/// A configuration that names the Plug'n'Play linker is the authority on the model, so a
/// `node_modules` an older model left behind does not take the run. Spec 9.3.
#[test]
fn a_named_plug_and_play_linker_wins_over_a_node_modules_left_behind() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "packages/app/");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    tree.write(".yarnrc.yml", "nodeLinker: pnp\n");
    installed(
        &tree,
        "packages/app/node_modules/.bin/tsc",
        &format!(
            "#!/bin/sh\necho \"the tool left behind\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn $*\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("packages/app/src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        ran(&tree),
        "yarn bin tsc\nyarn run -B tsc --noEmit\n",
        "{}",
        run.out
    );
}

/// A configuration that names the `node_modules` linker is the authority the other way: a marker
/// an older model left behind does not send the run through the package manager. Spec 9.3.
#[test]
fn a_named_node_modules_linker_wins_over_a_marker_left_behind() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(".pnp.cjs", "// a marker an older model left\n");
    tree.write(".yarnrc.yml", "nodeLinker: node-modules\n");
    installed(
        &tree,
        "node_modules/.bin/tsc",
        &format!(
            "#!/bin/sh\necho \"the installed tool\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn $*\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "the installed tool\n", "{}", run.out);
}

/// A linker a person wrote with a comment or quotation marks beside it still names the model.
/// Spec 9.3.
#[test]
fn a_named_linker_is_read_without_its_comment_or_quotation_marks() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    tree.write(".yarnrc.yml", "nodeLinker: \"pnp\" # zero installs\n");
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn $*\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        ran(&tree),
        "yarn bin tsc\nyarn run -B tsc --noEmit\n",
        "{}",
        run.out
    );
}

/// A manifest that pins a manager klin has no Plug'n'Play form for says the checkout is neither
/// Yarn's nor pnpm's, whatever lockfile an older manager left. Spec 9.3.
#[test]
fn a_pinned_manager_klin_does_not_read_leaves_the_tool_on_the_path() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "package.json",
        "{\"name\": \"web\", \"packageManager\": \"npm@10.9.0\"}\n",
    );
    tree.write("tsconfig.json", "{}\n");
    tree.write("src/index.ts", "export const a = 1;\n");
    tree.write(".pnp.cjs", "// a marker an older manager left\n");
    tree.write("yarn.lock", "# yarn lockfile v1\n");
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn $*\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(ran(&tree).starts_with("tsc "), "{}", ran(&tree));
}

/// The manager is read from the directory that holds the marker and upward, so a lockfile an
/// older manager left in a package below it does not name the run. Spec 9.3.
#[test]
fn a_lockfile_below_the_marker_does_not_name_the_manager() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    typescript(&tree, "packages/app/");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    tree.write("pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    tree.write(
        "packages/app/yarn.lock",
        "# a lockfile an older manager left\n",
    );
    tree.base();
    let path = toolchain(&tree);
    for manager in ["yarn", "pnpm"] {
        installed(
            &tree,
            &format!("toolchain/{manager}"),
            &format!(
                "#!/bin/sh\necho \"{manager} $*\" >> \"{}\"\n",
                tree.at("ran")
            ),
        );
    }
    tree.write("packages/app/src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "pnpm exec tsc --noEmit\n", "{}", run.out);
}

/// pnpm names its model in `pnpm-workspace.yaml`, and that naming decides against a
/// `node_modules` an older model left behind. Spec 9.3.
#[test]
fn a_pnpm_workspace_names_the_model_over_a_node_modules_left_behind() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "package.json",
        "{\"name\": \"web\", \"packageManager\": \"pnpm@12.0.0\"}\n",
    );
    tree.write("tsconfig.json", "{}\n");
    tree.write("src/index.ts", "export const a = 1;\n");
    tree.write("pnpm-workspace.yaml", "nodeLinker: pnp\n");
    tree.write(".pnp.cjs", "// a Plug'n'Play checkout\n");
    installed(
        &tree,
        "node_modules/.bin/tsc",
        &format!(
            "#!/bin/sh\necho \"the tool left behind\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/pnpm",
        &format!("#!/bin/sh\necho \"pnpm $*\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "pnpm exec tsc --noEmit\n", "{}", run.out);
}

/// klin reads the settings of the manager that owns the checkout and no other, so a `.yarnrc.yml`
/// an older manager left behind names no model and Yarn is never called. Spec 9.3.
#[test]
fn a_yarnrc_left_behind_does_not_name_the_model_of_a_pnpm_checkout() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "package.json",
        "{\"name\": \"web\", \"packageManager\": \"pnpm@12.0.0\"}\n",
    );
    tree.write("tsconfig.json", "{}\n");
    tree.write("src/index.ts", "export const a = 1;\n");
    tree.write(".yarnrc.yml", "nodeLinker: pnp\n");
    tree.write("pnpm-workspace.yaml", "nodeLinker: isolated\n");
    installed(
        &tree,
        "node_modules/.bin/tsc",
        &format!(
            "#!/bin/sh\necho \"the installed tool\" >> \"{}\"\n",
            tree.at("ran")
        ),
    );
    tree.base();
    let path = toolchain(&tree);
    for manager in ["yarn", "pnpm"] {
        installed(
            &tree,
            &format!("toolchain/{manager}"),
            &format!(
                "#!/bin/sh\necho \"{manager} $*\" >> \"{}\"\n",
                tree.at("ran")
            ),
        );
    }
    tree.write("src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(ran(&tree), "the installed tool\n", "{}", run.out);
}

/// A manifest that pins a manager klin has no Plug'n'Play form for answers for its own
/// directory, so a manager named above it does not take the run. Spec 9.3.
#[test]
fn a_pinned_manager_klin_does_not_read_stops_the_search_above_it() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "packages/app/package.json",
        "{\"name\": \"app\", \"packageManager\": \"npm@10.9.0\"}\n",
    );
    tree.write("packages/app/tsconfig.json", "{}\n");
    tree.write("packages/app/src/index.ts", "export const a = 1;\n");
    tree.write("package.json", "{\"name\": \"root\"}\n");
    tree.write("yarn.lock", "# yarn lockfile v1\n");
    tree.write(".yarnrc.yml", "nodeLinker: pnp\n");
    tree.base();
    let path = toolchain(&tree);
    installed(
        &tree,
        "toolchain/yarn",
        &format!("#!/bin/sh\necho \"yarn $*\" >> \"{}\"\n", tree.at("ran")),
    );
    tree.write("packages/app/src/index.ts", "export const a = 2;\n");

    let run = derived(&tree, &path, &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(ran(&tree).starts_with("tsc "), "{}", ran(&tree));
}
