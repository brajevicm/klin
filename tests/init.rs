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

/// Plain `init` writes the repository's opt-in marker and nothing it can derive. ADR 0028,
/// ADR 0040.
#[test]
fn init_writes_the_empty_opt_in_marker() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree), serde_json::json!({}), "{}", run.out);
    assert!(run.says("--pin"), "{}", run.out);
}

#[test]
fn init_leaves_a_config_that_already_exists_alone() {
    let tree = in_debt();
    let mine = r#"{ "doc_size": {"README.md": 900} }"#;
    tree.write("klin.json", mine);

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("already"), "{}", run.out);
    assert!(run.says("--pin"), "{}", run.out);
    let kept = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    assert_eq!(kept, mine, "{}", run.out);
}

#[test]
fn pin_fills_in_the_guardrails_the_config_does_not_state() {
    let tree = in_debt();
    tree.write("klin.json", r#"{ "doc_size": {"README.md": 900} }"#);

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["doc_size"]["README.md"], 900, "{config}");
    assert!(config["complexity"]["cc"].is_u64(), "{config}");
    assert!(config["complexity"]["lines"].is_u64(), "{config}");
    assert!(run.says("derived: complexity cc"), "{}", run.out);
}

#[test]
fn pin_leaves_a_gate_a_person_excluded_alone() {
    let tree = in_debt();
    tree.write("klin.json", r#"{ "escapes": false, "complexity": false }"#);

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["escapes"], Value::Bool(false), "{}", run.out);
    assert_eq!(config["complexity"], Value::Bool(false), "{}", run.out);
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

/// `--pin` adds today's guardrails and keeps every value a person wrote: a pinned ceiling, a
/// dated schedule, the accepted list, a gate switched off and the journal preference. #180.
#[test]
fn pin_keeps_every_value_a_person_wrote() {
    let tree = two_documents();
    let held = serde_json::json!({
        "accepted": [{"gate": "escapes", "file": "src/lib.rs", "text": "x", "count": 1}],
        "doc_size": {"README.md": 50, "CONTEXT.md": {"2020-01-01": 900}},
        "escapes": false,
        "journal": {"prompt": false}
    });
    tree.write("klin.json", &held.to_string());

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    for key in ["accepted", "doc_size", "escapes", "journal"] {
        assert_eq!(config[key], held[key], "{key}: {config}");
    }
    assert!(config["complexity"]["cc"].is_u64(), "{config}");
}

#[test]
fn pin_edits_no_gitignore() {
    let tree = two_documents();
    tree.write(".gitignore", "/target\n");
    tree.commit("an ignore file");

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.status(), "?? klin.json\n", "{}", run.out);
}

fn settings(tree: &Tree) -> Value {
    settings_at(&tree.path(".claude/settings.json"))
}

fn cursor_settings(tree: &Tree) -> Value {
    settings_at(&tree.path(".cursor/hooks.json"))
}

/// Cursor stores one command per event, not a nested `hooks` array.
fn cursor_commands(settings: &Value, event: &str) -> Vec<String> {
    settings["hooks"][event]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| entry["command"].as_str().map(str::to_string))
        .collect()
}

fn cursor_matchers(settings: &Value, event: &str) -> Vec<String> {
    settings["hooks"][event]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| entry["matcher"].as_str().map(str::to_string))
        .collect()
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
        ["Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*".to_string()],
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
        ["Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*".to_string()],
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

    let run = tree.run(&["init", "--hooks", "--host", "borg"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("borg"), "{}", run.out);
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

/// A pin is a guardrail a person owns, and nothing that describes the repository: no build
/// command, document topology, manifest, test root or source section. ADR 0040.
#[test]
fn pin_writes_only_stable_guardrails() {
    let tree = in_debt();

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    let written: Vec<&String> = config
        .as_object()
        .map(|held| held.keys().collect())
        .unwrap_or_default();
    assert_eq!(written, ["complexity", "doc_size"], "{config}");
    assert_eq!(
        config["complexity"].as_object().map(|held| held.len()),
        Some(2),
        "{config}"
    );
    assert!(
        config["doc_size"]["README.md"].as_u64().unwrap_or_default() >= 400,
        "{config}"
    );

    let gated = tree.run(&["gate", "--strict"]);
    assert_eq!(gated.code, 0, "{}", gated.out);
}

/// `init` pins what history says, so a person can see the two numbers, edit them and put them
/// under review. The lines name them as derived and never as a gate. #92.
#[test]
fn pin_writes_the_radius_values_history_derives() {
    let tree = harness::history(43, 6);

    let run = tree.run(&["init", "--pin"]);
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
fn pin_writes_no_radius_section_below_fifty_commits() {
    let tree = harness::history(42, 6);

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["radius"], Value::Null, "{}", run.out);
    assert!(
        run.says("derived: no \"radius\" pinned, because 49 non-merge commit(s) reach"),
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

/// Retired source-topology keys are refused instead of silently surviving a rewrite. #179.
#[test]
fn pin_refuses_retired_source_topology() {
    let tree = two_documents();
    tree.write(
        "klin.json",
        r#"{
          "complexity": {"roots": ["src"], "exclude": ["src/generated/**"]},
          "escapes": {"roots": ["src"], "languages": ["rust"], "exclude": ["vendor/**"]}
        }"#,
    );

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads"), "{}", run.out);
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

#[test]
fn hooks_writes_klins_entries_for_cursor_and_leaves_the_others_alone() {
    let tree = two_documents();
    tree.write(
        ".cursor/hooks.json",
        r#"{"version":1,"hooks":{"stop":[{"command":"cargo fmt"}]}}"#,
    );

    let run = tree.run(&["init", "--hooks", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = cursor_settings(&tree);
    assert_eq!(settings["version"], 1, "{settings}");
    assert_eq!(
        cursor_commands(&settings, "stop"),
        ["cargo fmt".to_string(), line("gate --hook --changed")],
        "{settings}"
    );
    assert_eq!(
        cursor_commands(&settings, "preToolUse"),
        [line("guard")],
        "{settings}"
    );
    assert_eq!(
        cursor_matchers(&settings, "preToolUse"),
        ["Write|Edit|Delete".to_string()],
        "{settings}"
    );
    assert_eq!(
        cursor_commands(&settings, "beforeShellExecution"),
        [line("guard")],
        "{settings}"
    );
    // A shell or MCP event carries no tool name, so a tool matcher there would gate nothing.
    for event in ["beforeShellExecution", "beforeMCPExecution"] {
        assert_eq!(
            cursor_matchers(&settings, event),
            Vec::<String>::new(),
            "{event}: {settings}"
        );
    }
    assert_eq!(
        cursor_commands(&settings, "sessionStart"),
        [line("radius")],
        "{settings}"
    );
    assert!(!tree.path(".claude/settings.json").exists(), "{}", run.out);
}

#[test]
fn hooks_detects_cursor_from_its_marker() {
    let tree = two_documents();
    tree.write(".cursor/rules", "\n");

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        cursor_commands(&cursor_settings(&tree), "stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

/// A tree that names two hosts gets the hooks of both, each in its host's own shape.
#[test]
fn hooks_writes_every_host_the_tree_names() {
    let tree = two_documents();
    tree.write(".claude/settings.json", "{}\n");
    tree.write(".cursor/rules", "\n");

    let run = tree.run(&["init", "--hooks"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "PreToolUse"),
        [line("guard")],
        "{}",
        run.out
    );
    assert_eq!(
        cursor_commands(&cursor_settings(&tree), "stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

const CURSOR_EVENTS: &[&str] = &[
    "sessionStart",
    "beforeSubmitPrompt",
    "preToolUse",
    "beforeShellExecution",
    "beforeMCPExecution",
    "stop",
];

#[test]
fn hooks_adds_no_second_cursor_entry_on_a_second_run() {
    let tree = two_documents();
    tree.write(".cursor/hooks.json", "{}\n");

    assert_eq!(tree.run(&["init", "--hooks", "--host", "cursor"]).code, 0);
    let run = tree.run(&["init", "--hooks", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = cursor_settings(&tree);
    for event in CURSOR_EVENTS {
        assert_eq!(
            cursor_commands(&settings, event).len(),
            1,
            "{event}: {settings}"
        );
    }
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
fn hooks_global_writes_cursor_hooks_to_the_users_file() {
    let (tree, home) = a_home();

    let run = globally(&tree, &home, &["--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".cursor/hooks.json"));
    assert_eq!(
        cursor_commands(&written, "stop"),
        [line("gate --hook --changed")],
        "{written}"
    );
    assert_eq!(
        cursor_commands(&written, "preToolUse"),
        [line("guard")],
        "{written}"
    );
    assert_eq!(tree.status(), "", "{}", run.out);
}

#[test]
fn hooks_global_for_a_host_with_no_adapter_is_refused() {
    let (tree, home) = a_home();

    let run = globally(&tree, &home, &["--host", "borg"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no host klin knows"), "{}", run.out);
}

const A_CURSOR_PLUGIN: &str = r#"{"name":"klin","version":"0.1.1"}"#;

/// Cursor documents local development plugins under `plugins/local/<name>`.
#[test]
fn hooks_adds_nothing_when_the_local_cursor_plugin_is_installed() {
    let tree = two_documents();
    tree.write(
        ".cursor/plugins/local/klin/.cursor-plugin/plugin.json",
        A_CURSOR_PLUGIN,
    );

    let run = tree.run(&["init", "--hooks", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".cursor/hooks.json").exists(), "{}", run.out);
    assert!(run.says("plugin"), "{}", run.out);
}

/// Cursor marketplace installs observed in 3.20.21 live under
/// `plugins/cache/<marketplace>/<plugin>/<revision>`.
#[test]
fn hooks_adds_nothing_when_a_marketplace_cursor_plugin_is_installed() {
    let tree = two_documents();
    let home = Tree::bare();
    home.write(
        ".cursor/plugins/cache/team-marketplace/klin/revision/.cursor-plugin/plugin.json",
        A_CURSOR_PLUGIN,
    );
    let home_at = home.root().display().to_string();

    let run = tree.run_with(
        &[("HOME", home_at.as_str())],
        &["init", "--hooks", "--host", "cursor"],
    );

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".cursor/hooks.json").exists(), "{}", run.out);
    assert!(run.says("plugin"), "{}", run.out);
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

#[test]
fn init_omits_automatic_source_sections() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config.get("stubs"), None, "{config}");
}

#[test]
fn pin_refuses_the_retired_stubs_shape() {
    let tree = in_debt();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "roots": ["old"], "languages": ["go"] } }"#,
    );

    let run = tree.run(&["init", "--pin"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads"), "{}", run.out);
}

/// The snapshot flags are gone with the snapshot they wrote. #180.
#[test]
fn the_retired_add_and_force_flags_are_usage_errors() {
    for flag in ["--add", "--force"] {
        let tree = in_debt();

        let run = tree.run(&["init", flag]);
        assert_eq!(run.code, 2, "{flag}: {}", run.out);
        assert!(!tree.path("klin.json").exists(), "{flag}: {}", run.out);
    }
}
