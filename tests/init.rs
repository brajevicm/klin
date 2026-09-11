mod harness;

use harness::Tree;
use serde_json::Value;
use std::os::unix::fs::PermissionsExt;

#[path = "fixtures/escape_text.rs"]
mod text;

const TANGLED: &str = r#"fn tangled(a: i32) -> i32 {
    if a > 0 && a < 10 {
        for x in 0..a {
            if x == 3 { return 1; }
        }
    } else if a == 0 || a == -1 || a == -2 || a == -3 {
        return 2;
    }
    match a {
        1 => 1,
        9 => 0,
    }
}
"#;

/// A tree that already holds debt: a tangled function, an escape site and a long document.
fn in_debt() -> Tree {
    let tree = Tree::bare();
    tree.write("src/knot.rs", TANGLED);
    tree.write("src/lib.rs", text::WRAPPED);
    tree.write("tests/knot.rs", TANGLED);
    tree.words("README.md", 400);
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.base();
    tree
}

fn config(tree: &Tree) -> Value {
    let Ok(text) = std::fs::read_to_string(tree.path("klin.json")) else {
        panic!("no klin.json was written")
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(why) => panic!("klin.json is not JSON: {why}\n{text}"),
    }
}

#[test]
fn init_on_a_tree_in_debt_writes_a_config_that_gates_green() {
    let tree = in_debt();

    let written = tree.run(&["init"]);
    assert_eq!(written.code, 0, "{}", written.out);

    let gated = tree.run(&["gate", "--strict"]);
    assert_eq!(gated.code, 0, "{}", gated.out);
}

#[test]
fn init_writes_no_file_but_the_config() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.status(), "?? klin.json\n", "{}", run.out);
}

#[test]
fn init_writes_every_section_it_can_infer() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(
        config["complexity"]["roots"],
        serde_json::json!(["src", "tests"]),
        "{config}"
    );
    assert_eq!(
        config["escapes"]["roots"],
        serde_json::json!(["src", "tests"]),
        "{config}"
    );
    assert_eq!(
        config["escapes"]["languages"],
        serde_json::json!(["rust"]),
        "{config}"
    );
    assert_eq!(config["doc_size"][0]["file"], "README.md", "{config}");
    assert!(
        config["doc_size"][0]["ceiling"]
            .as_u64()
            .unwrap_or_default()
            >= 400,
        "{config}"
    );
    assert_eq!(config["doc_citations"][0]["file"], "README.md", "{config}");
    assert_eq!(
        config["doc_citations"][0]["roots"],
        serde_json::json!(["."]),
        "{config}"
    );
}

#[test]
fn init_writes_the_version_of_the_running_binary() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["version"], env!("CARGO_PKG_VERSION"), "{config}");
}

#[test]
fn init_writes_one_build_entry_per_manifest() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let build = config(&tree)["build"].clone();
    assert!(
        build
            .as_str()
            .unwrap_or_default()
            .starts_with("cargo build"),
        "{build}"
    );
}

#[test]
fn init_writes_a_build_entry_with_a_root_for_each_project_of_a_monorepo() {
    let tree = Tree::bare();
    tree.write("api/Cargo.toml", "[package]\nname = \"api\"\n");
    tree.write("api/src/lib.rs", "fn f() {}\n");
    tree.write("web/package.json", "{\"name\": \"web\"}\n");
    tree.write("web/tsconfig.json", "{}\n");
    tree.write("web/src/index.ts", "export const a = 1;\n");
    tree.write("service/go.mod", "module t\n");
    tree.write("service/main.go", "package main\n");
    tree.base();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let build = config(&tree)["build"].clone();
    let roots: Vec<&str> = build
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["root"].as_str())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(roots, ["api", "service", "web"], "{build}");
}

#[test]
fn init_leaves_a_config_that_already_exists_alone() {
    let tree = in_debt();
    let mine = r#"{ "project": "mine", "doc_size": [{"file": "README.md", "ceiling": 900}] }"#;
    tree.write("klin.json", mine);

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("already"), "{}", run.out);
    assert!(run.says("--add"), "{}", run.out);
    let kept = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    assert_eq!(kept, mine, "{}", run.out);
}

#[test]
fn add_fills_in_the_sections_the_config_does_not_name() {
    let tree = in_debt();
    tree.write(
        "klin.json",
        r#"{ "project": "mine", "doc_size": [{"file": "README.md", "ceiling": 900}] }"#,
    );

    let run = tree.run(&["init", "--add"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["project"], "mine", "{config}");
    assert_eq!(config["doc_size"][0]["ceiling"], 900, "{config}");
    assert!(config["escapes"].is_object(), "{config}");
    assert!(config["complexity"].is_object(), "{config}");
    assert!(config["doc_citations"].is_array(), "{config}");
}

#[test]
fn add_leaves_a_gate_a_person_excluded_alone() {
    let tree = in_debt();
    tree.write("klin.json", r#"{ "project": "mine", "escapes": false }"#);

    let run = tree.run(&["init", "--add"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["escapes"], Value::Bool(false), "{}", run.out);
}

/// A tree whose two documents let one entry be re-pinned while the other keeps a schedule.
fn two_documents() -> Tree {
    let tree = Tree::bare();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.words("README.md", 400);
    tree.words("CONTEXT.md", 200);
    tree.base();
    tree
}

fn entry(config: &Value, section: &str, file: &str) -> Value {
    let entries = config[section].as_array().cloned().unwrap_or_default();
    entries
        .into_iter()
        .find(|entry| entry["file"] == file)
        .unwrap_or_else(|| panic!("no {section} entry for {file} in {config}"))
}

/// `--force` re-pins what the tree says today. It keeps the accepted list, a dated schedule
/// and a gate a person switched off, because klin derives none of those. #107.
#[test]
fn force_re_pins_every_derivable_value_and_keeps_what_klin_cannot_derive() {
    let tree = two_documents();
    let accepted = serde_json::json!([{"gate": "escapes", "file": "src/lib.rs"}]);
    tree.write(
        "klin.json",
        r#"{
          "project": "old",
          "accepted": [{"gate": "escapes", "file": "src/lib.rs"}],
          "doc_size": [
            {"file": "README.md", "ceiling": 50},
            {"file": "CONTEXT.md", "ceiling": {"2020-01-01": 900}}
          ],
          "escapes": false
        }"#,
    );

    let run = tree.run(&["init", "--force"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_ne!(config["project"], "old", "{config}");
    assert!(
        entry(&config, "doc_size", "README.md")["ceiling"]
            .as_u64()
            .unwrap_or_default()
            >= 400,
        "{config}"
    );
    assert_eq!(
        entry(&config, "doc_size", "CONTEXT.md")["ceiling"],
        serde_json::json!({"2020-01-01": 900}),
        "{config}"
    );
    assert_eq!(config["accepted"], accepted, "{config}");
    assert_eq!(config["escapes"], Value::Bool(false), "{config}");
}

#[test]
fn force_edits_no_gitignore() {
    let tree = two_documents();
    tree.write(".gitignore", "/target\n");
    tree.commit("an ignore file");

    let run = tree.run(&["init", "--force"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.status(), "?? klin.json\n", "{}", run.out);
}

fn settings(tree: &Tree) -> Value {
    settings_at(&tree.path(".claude/settings.json"))
}

fn codex_settings(tree: &Tree) -> Value {
    settings_at(&tree.path(".codex/hooks.json"))
}

fn settings_at(path: &std::path::Path) -> Value {
    let Ok(text) = std::fs::read_to_string(path) else {
        panic!("no {} was written", path.display())
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(why) => panic!("{} is not JSON: {why}\n{text}", path.display()),
    }
}

/// The hook line klin writes for one klin command. Every line resolves the binary first.
fn line(arguments: &str) -> String {
    format!("command -v klin > /dev/null 2>&1 || exit 0; klin {arguments}")
}

/// Every command in one host event, so a test can count klin's entries and the ones it left.
fn commands(settings: &Value, event: &str) -> Vec<String> {
    settings["hooks"][event]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|entry| entry["hooks"].as_array().cloned().unwrap_or_default())
        .filter_map(|hook| hook["command"].as_str().map(str::to_string))
        .collect()
}

fn matchers(settings: &Value, event: &str) -> Vec<String> {
    settings["hooks"][event]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| entry["matcher"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn hooks_writes_klins_entries_for_claude_code_and_leaves_the_others_alone() {
    let tree = two_documents();
    tree.write(
        ".claude/settings.json",
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "cargo fmt"}]}]}}"#,
    );

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = settings(&tree);
    assert_eq!(
        commands(&settings, "Stop"),
        ["cargo fmt".to_string(), line("gate --hook --changed")],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "PreToolUse"),
        [line("guard")],
        "{settings}"
    );
    assert_eq!(
        matchers(&settings, "PreToolUse"),
        ["Write|Edit|MultiEdit|NotebookEdit|Bash".to_string()],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "SessionStart"),
        [line("radius")],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "UserPromptSubmit"),
        [line("radius")],
        "{settings}"
    );
}

#[test]
fn hooks_writes_klins_entries_for_codex_cli_and_leaves_the_others_alone() {
    let tree = two_documents();
    tree.write(
        ".codex/hooks.json",
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "cargo fmt"}]}]}}"#,
    );

    let run = tree.run(&["init", "--hooks", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = codex_settings(&tree);
    assert_eq!(
        commands(&settings, "Stop"),
        ["cargo fmt".to_string(), line("gate --hook --changed")],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "PreToolUse"),
        [line("guard")],
        "{settings}"
    );
    assert_eq!(
        matchers(&settings, "PreToolUse"),
        ["Bash|apply_patch|mcp__.*".to_string()],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "SessionStart"),
        [line("radius")],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "UserPromptSubmit"),
        [line("radius")],
        "{settings}"
    );
}

#[test]
fn hooks_adds_no_second_klin_entry_on_a_second_run() {
    let tree = two_documents();
    tree.write(".claude/settings.json", "{}\n");

    assert_eq!(tree.run(&["init", "--hooks"]).code, 0);
    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = settings(&tree);
    for event in ["Stop", "PreToolUse", "SessionStart", "UserPromptSubmit"] {
        assert_eq!(commands(&settings, event).len(), 1, "{event}: {settings}");
    }
}

#[test]
fn hooks_for_a_host_with_no_adapter_is_refused() {
    let tree = two_documents();

    for name in ["cursor", "borg"] {
        let run = tree.run(&["init", "--hooks", "--host", name]);
        assert_eq!(run.code, 2, "{name}: {}", run.out);
        assert!(run.says(name), "{name}: {}", run.out);
    }
}

#[test]
fn hooks_edits_no_gitignore_and_no_config() {
    let tree = two_documents();
    tree.write(".gitignore", "/target\n");
    tree.write(".claude/settings.json", "{}\n");
    tree.commit("an ignore file and a settings file");

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.status(), " M .claude/settings.json\n", "{}", run.out);
}

#[test]
fn init_infers_no_section_for_a_gate_it_cannot_survey() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config.get("sarif"), None, "{config}");
    assert_eq!(config.get("manifests"), None, "{config}");
}

/// `init` pins what history says, so a person can see the two numbers, edit them and put them
/// under review. The lines name them as derived and never as a gate. #92.
#[test]
fn init_pins_the_radius_values_history_derives() {
    let tree = harness::history(43, 6);

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["radius"]["lines"], 30, "{config}");
    assert_eq!(config["radius"]["directories"], 3, "{config}");
    assert!(
        run.says("derived: radius lines 30, the 90th percentile of the last 50 non-merge commits"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: radius directories 3, the 90th percentile"),
        "{}",
        run.out
    );

    let listed = tree.run(&["gate", "--list"]);
    assert!(!listed.says("radius"), "{}", listed.out);
}

#[test]
fn init_writes_no_radius_section_below_fifty_commits() {
    let tree = harness::history(42, 6);

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["radius"], Value::Null, "{}", run.out);
    assert!(
        run.says("derived: no \"radius\" section, because 49 non-merge commit(s) reach"),
        "{}",
        run.out
    );
}

#[test]
fn hooks_for_a_named_host_writes_a_file_the_tree_does_not_hold_yet() {
    let tree = two_documents();

    let run = tree.run(&["init", "--hooks", "--host", "claude"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

#[test]
fn hooks_with_no_host_at_the_root_is_refused() {
    let tree = two_documents();

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("--host"), "{}", run.out);
}

/// A key the survey does not derive is a person's, and `--force` re-pins around it. #107.
#[test]
fn force_keeps_an_exclusion_the_survey_does_not_derive() {
    let tree = two_documents();
    tree.write(
        "klin.json",
        r#"{
          "complexity": {"roots": ["src"], "exclude": ["src/generated/**"]},
          "escapes": {"roots": ["src"], "languages": ["rust"], "exclude": ["vendor/**"]}
        }"#,
    );

    let run = tree.run(&["init", "--force"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(
        config["complexity"]["exclude"],
        serde_json::json!(["src/generated/**"]),
        "{config}"
    );
    assert_eq!(
        config["escapes"]["exclude"],
        serde_json::json!(["vendor/**"]),
        "{config}"
    );
    assert!(config["complexity"]["ceilings"].is_object(), "{config}");
}

/// A hook that only mentions klin belongs to another tool, and reading it as klin's would
/// leave that event ungated. #107.
#[test]
fn hooks_adds_its_entry_beside_a_hook_that_only_mentions_klin() {
    let tree = two_documents();
    tree.write(
        ".claude/settings.json",
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command",
          "command": "/work/klin-ui/scripts/fmt.sh"}]}]}}"#,
    );

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "Stop"),
        [
            "/work/klin-ui/scripts/fmt.sh".to_string(),
            line("gate --hook --changed")
        ],
        "{}",
        run.out
    );
}

/// A tree that names a host klin cannot write yet still gets the hooks of the host it can.
#[test]
fn hooks_writes_the_host_it_can_and_notes_the_one_it_cannot() {
    let tree = two_documents();
    tree.write(".claude/settings.json", "{}\n");
    tree.write(".cursor/rules", "\n");

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("#67"), "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "PreToolUse"),
        [line("guard")],
        "{}",
        run.out
    );
}

#[test]
fn host_without_hooks_is_a_usage_error() {
    let tree = two_documents();

    let run = tree.run(&["init", "--host", "claude"]);
    assert_eq!(run.code, 2, "{}", run.out);
}

/// A home directory klin writes the user-level hook file into, and the tree it is run from.
fn a_home() -> (Tree, Tree) {
    let tree = two_documents();
    tree.write(".claude/settings.json", "{}\n");
    tree.commit("a settings file");
    let home = Tree::bare();
    home.write(".claude/settings.json", A_USER_FILE);
    (tree, home)
}

const A_USER_FILE: &str = r#"{"hooks": {"Stop": [{"hooks": [{"type": "command",
  "command": "cargo fmt"}]}]}}"#;

fn globally(tree: &Tree, home: &Tree, args: &[&str]) -> harness::Run {
    let at = home.root().display().to_string();
    tree.run_with(
        &[("HOME", at.as_str())],
        &[&["init", "--hooks", "--global"], args].concat(),
    )
}

/// One install covers every repository, and the tree's own file carries nothing. #137.
#[test]
fn hooks_global_writes_the_users_file_and_leaves_the_trees_alone() {
    let (tree, home) = a_home();

    let run = globally(&tree, &home, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".claude/settings.json"));
    assert_eq!(
        commands(&written, "Stop"),
        ["cargo fmt".to_string(), line("gate --hook --changed")],
        "{written}"
    );
    assert_eq!(
        commands(&written, "PreToolUse"),
        [line("guard")],
        "{written}"
    );
    assert!(run.says("covers every repository"), "{}", run.out);
    assert!(!run.says("Commit it"), "{}", run.out);
    assert_eq!(tree.status(), "", "{}", run.out);
}

#[test]
fn hooks_global_adds_no_second_entry_on_a_second_run() {
    let (tree, home) = a_home();
    home.write(".claude/settings.json", "{}\n");

    assert_eq!(globally(&tree, &home, &[]).code, 0);
    let run = globally(&tree, &home, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".claude/settings.json"));
    for event in ["Stop", "PreToolUse", "SessionStart", "UserPromptSubmit"] {
        assert_eq!(commands(&written, event).len(), 1, "{event}: {written}");
    }
}

#[test]
fn hooks_global_for_a_host_with_no_adapter_is_refused() {
    let (tree, home) = a_home();

    let run = globally(&tree, &home, &["--host", "cursor"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("#67"), "{}", run.out);
    assert!(run.says("--hooks --global --host"), "{}", run.out);
}

#[test]
fn hooks_global_writes_codex_cli_hooks_to_the_users_file() {
    let (tree, home) = a_home();

    let run = globally(&tree, &home, &["--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".codex/hooks.json"));
    assert_eq!(
        commands(&written, "Stop"),
        [line("gate --hook --changed")],
        "{written}"
    );
    assert_eq!(
        commands(&written, "PreToolUse"),
        [line("guard")],
        "{written}"
    );
    assert_eq!(tree.status(), "", "{}", run.out);
}

const A_CODEX_PLUGIN: &str = "[plugins.\"klin@klin\"]\nenabled = true\n";
const A_CODEX_PLUGIN_OFF: &str =
    "[plugins.\"klin@klin\"]\nenabled = false\n\n[plugins.\"other@klin\"]\n";

/// Codex CLI lists its plugins in `config.toml`, and a plugin table is on unless it says
/// `enabled = false`. Spec 19.3.
#[test]
fn hooks_adds_nothing_when_the_codex_plugin_is_enabled() {
    let tree = two_documents();
    tree.write(".codex/config.toml", A_CODEX_PLUGIN);

    let run = tree.run(&["init", "--hooks", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".codex/hooks.json").exists(), "{}", run.out);
    assert!(run.says("plugin"), "{}", run.out);
    assert!(run.says("config.toml"), "{}", run.out);
}

#[test]
fn hooks_writes_for_codex_when_its_plugin_table_is_switched_off() {
    let tree = two_documents();
    tree.write(".codex/config.toml", A_CODEX_PLUGIN_OFF);

    let run = tree.run(&["init", "--hooks", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&codex_settings(&tree), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

#[test]
fn hooks_global_adds_nothing_when_the_users_codex_config_enables_the_plugin() {
    let (tree, home) = a_home();
    home.write(".codex/config.toml", A_CODEX_PLUGIN);

    let run = globally(&tree, &home, &["--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!home.path(".codex/hooks.json").exists(), "{}", run.out);
    assert!(run.says("plugin"), "{}", run.out);
}

/// Codex CLI enables plugins in `config.toml`, not under a key in `hooks.json`. So Claude
/// Code's plugin key in the Codex file means nothing to klin.
#[test]
fn hooks_for_codex_ignore_claudes_plugin_key() {
    let tree = two_documents();
    tree.write(".codex/hooks.json", A_PLUGIN);

    let run = tree.run(&["init", "--hooks", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&codex_settings(&tree), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
    assert!(!run.says("plugin"), "{}", run.out);
}

#[test]
fn a_codex_hook_line_says_nothing_when_no_binary_resolves() {
    let tree = two_documents();
    tree.write(".codex/hooks.json", "{}\n");
    assert_eq!(tree.run(&["init", "--hooks", "--host", "codex"]).code, 0);

    let Some(command) = commands(&codex_settings(&tree), "Stop").into_iter().next() else {
        panic!("no Codex Stop hook was written")
    };
    let outcome = std::process::Command::new("/bin/sh")
        .args(["-c", &command])
        .env("PATH", "")
        .output();
    let Ok(done) = outcome else {
        panic!("the hook line could not run: {command}")
    };
    assert_eq!(done.status.code(), Some(0), "{command}");
    assert!(done.stdout.is_empty(), "{command}");
    assert!(done.stderr.is_empty(), "{command}");
}

const A_PLUGIN: &str = r#"{"enabledPlugins": {"klin@klin-marketplace": true}}"#;

/// The plugin registers the same four events, so a second copy of them runs klin twice on
/// every event. #147.
#[test]
fn hooks_adds_nothing_when_the_plugin_is_enabled_in_the_tree() {
    let tree = two_documents();
    tree.write(".claude/settings.json", A_PLUGIN);

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(settings(&tree)["hooks"], Value::Null, "{}", run.out);
    assert!(run.says("plugin"), "{}", run.out);
}

#[test]
fn hooks_adds_nothing_when_the_plugin_is_enabled_for_this_tree_alone() {
    let tree = two_documents();
    tree.write(".claude/settings.json", "{}\n");
    tree.write(".claude/settings.local.json", A_PLUGIN);

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(settings(&tree)["hooks"], Value::Null, "{}", run.out);
}

#[test]
fn hooks_writes_when_the_plugin_is_listed_but_switched_off() {
    let tree = two_documents();
    tree.write(
        ".claude/settings.json",
        r#"{"enabledPlugins": {"klin@klin-marketplace": false}}"#,
    );

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "PreToolUse").len(),
        1,
        "{}",
        run.out
    );
}

#[test]
fn hooks_global_adds_nothing_when_the_plugin_is_enabled_for_the_user() {
    let (tree, home) = a_home();
    home.write(".claude/settings.json", A_PLUGIN);

    let run = globally(&tree, &home, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".claude/settings.json"));
    assert_eq!(written["hooks"], Value::Null, "{}", run.out);
    assert!(run.says("plugin"), "{}", run.out);
}

/// A plugin one repository enables gates that repository, not the machine, so it does not
/// stand in the way of the user-level install. #147.
#[test]
fn hooks_global_writes_when_the_plugin_is_enabled_in_the_tree_alone() {
    let (tree, home) = a_home();
    tree.write(".claude/settings.json", A_PLUGIN);

    let run = globally(&tree, &home, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".claude/settings.json"));
    assert_eq!(commands(&written, "PreToolUse").len(), 1, "{}", run.out);
}

/// Every hook line klin writes resolves the binary first, so a machine that holds no klin
/// says nothing on every event of every session instead of failing. #147.
#[test]
fn a_written_hook_line_says_nothing_when_no_binary_resolves() {
    let tree = two_documents();
    tree.write(".claude/settings.json", "{}\n");
    assert_eq!(tree.run(&["init", "--hooks"]).code, 0);

    let settings = settings(&tree);
    for event in ["Stop", "PreToolUse", "SessionStart", "UserPromptSubmit"] {
        for command in commands(&settings, event) {
            let outcome = std::process::Command::new("/bin/sh")
                .args(["-c", &command])
                .env("PATH", "")
                .output();
            let Ok(done) = outcome else {
                panic!("the hook line could not run: {command}")
            };
            assert_eq!(done.status.code(), Some(0), "{event}: {command}");
            assert!(done.stdout.is_empty(), "{event}: {command}");
            assert!(done.stderr.is_empty(), "{event}: {command}");
        }
    }
}

/// A host reads its user file and the tree's together, so a global install klin wrote itself
/// duplicates every event when a tree write follows it. #147.
#[test]
fn hooks_adds_nothing_when_the_user_file_already_holds_klins_entries() {
    let (tree, home) = a_home();
    assert_eq!(globally(&tree, &home, &[]).code, 0);

    let at = home.root().display().to_string();
    let run = tree.run_with(&[("HOME", at.as_str())], &["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(settings(&tree)["hooks"], Value::Null, "{}", run.out);
    assert!(run.says("--global"), "{}", run.out);
}

/// A settings file kept in a dotfiles tree is a link. klin follows it, so the link survives
/// and the tree it points into holds the hooks. #147.
#[test]
fn hooks_follows_a_settings_file_that_is_a_link() {
    let tree = two_documents();
    let held = tree.write("dotfiles/settings.json", "{}\n");
    let link = tree.path(".claude/settings.json");
    assert!(std::fs::create_dir_all(tree.path(".claude")).is_ok());
    assert!(std::os::unix::fs::symlink(&held, &link).is_ok());
    let narrow = std::fs::Permissions::from_mode(0o600);
    assert!(std::fs::set_permissions(&held, narrow).is_ok());

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(link.is_symlink(), "{}", run.out);
    assert_eq!(
        commands(&settings_at(&held), "PreToolUse"),
        [line("guard")],
        "{}",
        run.out
    );
    let Ok(mode) = std::fs::metadata(&held) else {
        panic!("the settings file is gone")
    };
    assert_eq!(mode.permissions().mode() & 0o777, 0o600, "{}", run.out);
}
