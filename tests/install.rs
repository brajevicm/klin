mod harness;

use harness::Tree;
use serde_json::Value;

const CANONICAL_SKILL: &str = include_str!("../plugins/klin/skills/klin/SKILL.md");

/// A repository klin can install into: a base commit, and nothing else.
fn a_repository() -> Tree {
    let tree = Tree::new();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree
}

/// The environment one run needs to read a home directory of the test's own.
fn home_of(home: &Tree) -> String {
    home.root().display().to_string()
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

fn skill_at(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|why| panic!("{}: {why}", path.display()))
}

fn settings(tree: &Tree) -> Value {
    settings_at(&tree.path(".claude/settings.json"))
}

fn codex_settings(tree: &Tree) -> Value {
    settings_at(&tree.path(".codex/hooks.json"))
}

fn cursor_settings(tree: &Tree) -> Value {
    settings_at(&tree.path(".cursor/hooks.json"))
}

/// The hook line klin writes for one klin command. Every line resolves the binary first, and
/// the stop says how to install it where the repository opted in.
fn line(arguments: &str) -> String {
    let missing = match arguments.starts_with("gate") {
        true => format!(
            "{{ r=$(git rev-parse --show-toplevel 2>/dev/null) && [ -f \"$r/klin.json\" ] && echo \
             '{{\"systemMessage\":\"{MISSING}\"}}'; exit 0; }}"
        ),
        false => "exit 0".to_string(),
    };
    format!("command -v klin > /dev/null 2>&1 || {missing}; klin {arguments}")
}

const MISSING: &str = "klin is not installed. Install it with: curl --proto =https --tlsv1.2 \
                       -LsSf https://github.com/brajevicm/klin/releases/latest/download/\
                       klin-installer.sh | sh";

fn entries(settings: &Value, event: &str) -> Vec<Value> {
    settings["hooks"][event]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// Every command in one host event, so a test can count klin's entries and the ones it left.
fn commands(settings: &Value, event: &str) -> Vec<String> {
    entries(settings, event)
        .iter()
        .flat_map(|entry| entry["hooks"].as_array().cloned().unwrap_or_default())
        .filter_map(|hook| hook["command"].as_str().map(str::to_string))
        .collect()
}

fn matchers(settings: &Value, event: &str) -> Vec<String> {
    entries(settings, event)
        .iter()
        .filter_map(|entry| entry["matcher"].as_str().map(str::to_string))
        .collect()
}

/// Cursor stores one command per event, not a nested `hooks` array.
fn cursor_commands(settings: &Value, event: &str) -> Vec<String> {
    entries(settings, event)
        .iter()
        .filter_map(|entry| entry["command"].as_str().map(str::to_string))
        .collect()
}

fn cursor_matchers(settings: &Value, event: &str) -> Vec<String> {
    matchers(settings, event)
}

const CLAUDE_EVENTS: &[&str] = &["SessionStart", "UserPromptSubmit", "PreToolUse", "Stop"];

const CURSOR_EVENTS: &[&str] = &[
    "sessionStart",
    "beforeSubmitPrompt",
    "preToolUse",
    "beforeShellExecution",
    "beforeMCPExecution",
    "stop",
];

const SHARED_MATCHER: &str = "Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*";

const A_PLUGIN: &str = r#"{"enabledPlugins": {"klin@klin-marketplace": true}}"#;

/// An entry of another tool's, which every reconciliation keeps.
const ANOTHER_TOOL: &str = r#"{"hooks": {"Stop": [{"hooks": [{"type": "command",
  "command": "cargo fmt"}]}]}}"#;

/// A teammate who clones a repository with klin's committed hooks and has no klin on PATH hears
/// at the stop how to install it. The notice is never a `followup_message`, which Cursor submits
/// as the next prompt and would hand the installer to the agent. The session may start below
/// the root, as Codex and Claude Code both allow, so the stop looks for the marker at the Git
/// root. A tree that never opted in stays silent. Spec 19.3.
#[test]
fn the_committed_stop_says_how_to_install_klin_where_none_resolves() {
    let tree = a_repository();
    let run = tree.run(&[
        "install", "--host", "claude", "--host", "codex", "--host", "cursor",
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    let stops = [
        commands(&settings(&tree), "Stop"),
        commands(&codex_settings(&tree), "Stop"),
        cursor_commands(&cursor_settings(&tree), "stop"),
    ]
    .concat();
    let nested = tree.path("apps/web");
    std::fs::create_dir_all(&nested).unwrap_or_else(|why| panic!("{why}"));

    for stop in &stops {
        let said = without_klin(&nested, stop);
        let Ok(notice) = serde_json::from_str::<Value>(said.trim()) else {
            panic!("the stop printed no JSON notice: {said}\n{stop}")
        };
        assert!(
            notice["systemMessage"]
                .as_str()
                .is_some_and(|text| text.contains("klin-installer.sh")),
            "{said}"
        );
        assert!(notice.get("followup_message").is_none(), "{said}");
    }
    std::fs::remove_file(tree.path("klin.json")).unwrap_or_else(|why| panic!("{why}"));
    for stop in &stops {
        assert_eq!(without_klin(&nested, stop), "", "{stop}");
    }
}

/// What a hook line prints on stdout from `cwd`, a session that started there, where PATH
/// resolves git and no `klin`.
fn without_klin(cwd: &std::path::Path, line: &str) -> String {
    let done = std::process::Command::new("/bin/sh")
        .args(["-c", line])
        .current_dir(cwd)
        .env("PATH", "/usr/bin:/bin")
        .env("CLAUDE_PROJECT_DIR", cwd)
        .output()
        .unwrap_or_else(|why| panic!("sh could not run: {why}"));
    assert!(done.status.success(), "{line}");
    String::from_utf8_lossy(&done.stdout).to_string()
}

/// The marker belongs to the repository root, not to the directory the command was run from.
#[test]
fn install_opts_the_repository_in_from_a_nested_directory() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    tree.write("apps/web/index.ts", "export const one = 1;\n");

    let run = harness::run_from(&tree.path("apps/web"), &["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(tree.path("klin.json").is_file(), "{}", run.out);
    assert!(!tree.path("apps/web/klin.json").exists(), "{}", run.out);
    assert!(run.says("repository opted in"), "{}", run.out);
}

/// The marker is human-owned, so opting in never replaces what it holds. ADR 0028.
#[test]
fn install_keeps_the_configuration_a_person_wrote() {
    let tree = a_repository();
    tree.write("klin.json", r#"{"escapes": false}"#);
    tree.write(".claude/settings.json", "{}\n");

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let held = settings_at(&tree.path("klin.json"));
    assert_eq!(held["escapes"], Value::Bool(false), "{held}");
    assert!(run.says("already opts this repository in"), "{}", run.out);
}

#[test]
fn install_writes_klins_entries_for_claude_code_and_leaves_the_others_alone() {
    let tree = a_repository();
    tree.write(".claude/settings.json", ANOTHER_TOOL);

    let run = tree.run(&["install"]);
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
        [SHARED_MATCHER],
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
fn install_writes_klins_entries_for_codex_cli() {
    let tree = a_repository();
    tree.write(".codex/hooks.json", ANOTHER_TOOL);

    let run = tree.run(&["install", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = codex_settings(&tree);
    assert_eq!(
        commands(&settings, "Stop"),
        ["cargo fmt".to_string(), line("gate --hook --changed")],
        "{settings}"
    );
    assert_eq!(
        matchers(&settings, "PreToolUse"),
        [SHARED_MATCHER],
        "{settings}"
    );
}

#[test]
fn install_writes_klins_entries_for_cursor_in_its_own_shape() {
    let tree = a_repository();
    tree.write(
        ".cursor/hooks.json",
        r#"{"version":1,"hooks":{"stop":[{"command":"cargo fmt"}]}}"#,
    );

    let run = tree.run(&["install", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = cursor_settings(&tree);
    assert_eq!(settings["version"], 1, "{settings}");
    assert_eq!(
        cursor_commands(&settings, "stop"),
        ["cargo fmt".to_string(), line("gate --hook --changed")],
        "{settings}"
    );
    assert_eq!(
        cursor_matchers(&settings, "preToolUse"),
        ["Write|Edit|Delete".to_string()],
        "{settings}"
    );
    for event in ["beforeShellExecution", "beforeMCPExecution"] {
        assert_eq!(
            cursor_commands(&settings, event),
            [line("guard")],
            "{event}: {settings}"
        );
        assert_eq!(
            cursor_matchers(&settings, event),
            Vec::<String>::new(),
            "{event}: {settings}"
        );
    }
    assert!(!tree.path(".claude/settings.json").exists(), "{}", run.out);
}

/// A tree that names two hosts gets the hooks of both, each in its host's own shape.
#[test]
fn install_reconciles_every_host_the_repository_names() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    tree.write(".cursor/rules", "\n");

    let run = tree.run(&["install"]);
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
    assert!(run.says("codex: no integration requested."), "{}", run.out);
}

#[test]
fn install_narrows_to_the_host_that_is_named() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    tree.write(".cursor/rules", "\n");

    let run = tree.run(&["install", "--host", "claude"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".cursor/hooks.json").exists(), "{}", run.out);
}

#[test]
fn install_refuses_a_host_klin_does_not_know() {
    let tree = a_repository();

    let run = tree.run(&["install", "--host", "borg"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("borg"), "{}", run.out);
    assert!(run.says("claude"), "{}", run.out);
    assert!(!tree.path("klin.json").exists(), "{}", run.out);
}

/// A repository that proves no host is not guessed at: a hook file klin invented gates nothing.
#[test]
fn install_with_no_provable_host_names_the_supported_ones() {
    let tree = a_repository();

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("--host"), "{}", run.out);
    for host in ["claude", "codex", "cursor"] {
        assert!(run.says(host), "{host}: {}", run.out);
    }
}

/// An enabled plugin is host evidence on its own, so a repository with no marker directory is
/// still reported rather than refused.
#[test]
fn install_proves_a_host_from_an_enabled_plugin_alone() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(".claude/settings.json", A_PLUGIN);

    let at = home_of(&home);
    let run = tree.run_with(&[("HOME", at.as_str())], &["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("claude: hooks supplied by the klin plugin"),
        "{}",
        run.out
    );
    assert!(!tree.path(".claude/settings.json").exists(), "{}", run.out);
    assert!(tree.path("klin.json").is_file(), "{}", run.out);
}

/// The plugin registers the same events, so a second copy of them runs klin twice on every
/// event. #147.
#[test]
fn install_adds_no_hooks_where_the_plugin_owns_the_host() {
    let tree = a_repository();
    tree.write(".claude/settings.json", A_PLUGIN);

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(settings(&tree)["hooks"], Value::Null, "{}", run.out);
    assert!(run.says("supplied by the klin plugin"), "{}", run.out);
    assert!(run.says(".claude/settings.json"), "{}", run.out);
    assert!(
        !tree.path(".claude/skills/klin/SKILL.md").exists(),
        "{}",
        run.out
    );
}

#[test]
fn install_writes_the_canonical_skill_once_for_codex_and_cursor() {
    let tree = a_repository();

    let run = tree.run(&["install", "--host", "codex", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        skill_at(&tree.path(".agents/skills/klin/SKILL.md")),
        CANONICAL_SKILL
    );
    assert_eq!(
        run.out.matches(".agents/skills/klin/SKILL.md").count(),
        1,
        "{}",
        run.out
    );
}

#[test]
fn install_user_writes_the_canonical_shared_skill() {
    let tree = a_repository();
    let home = Tree::bare();
    let at = home_of(&home);

    let run = tree.run_with(
        &[("HOME", at.as_str())],
        &["install", "--user", "--host", "codex", "--host", "cursor"],
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        skill_at(&home.path(".agents/skills/klin/SKILL.md")),
        CANONICAL_SKILL
    );
    assert!(
        !tree.path(".agents/skills/klin/SKILL.md").exists(),
        "{}",
        run.out
    );
    assert!(
        run.says("do not reach a cloud or remote agent"),
        "{}",
        run.out
    );
}

#[test]
fn install_refuses_a_different_skill_before_writing_anything() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    tree.write(".claude/skills/klin/SKILL.md", "a person's skill\n");

    let run = tree.run(&["install", "--host", "claude"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("SKILL.md"), "{}", run.out);
    assert!(run.says("refusing to overwrite"), "{}", run.out);
    assert!(!tree.path("klin.json").exists(), "{}", run.out);
    assert_eq!(
        skill_at(&tree.path(".claude/skills/klin/SKILL.md")),
        "a person's skill\n"
    );
    assert_eq!(skill_at(&tree.path(".claude/settings.json")), "{}\n");
}

#[test]
fn install_refuses_a_shared_skill_conflict_before_writing_either_host() {
    let tree = a_repository();
    tree.write(".agents/skills/klin/SKILL.md", "a person's shared skill\n");

    let run = tree.run(&["install", "--host", "codex", "--host", "cursor"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(".agents/skills/klin/SKILL.md"), "{}", run.out);
    assert!(!tree.path("klin.json").exists(), "{}", run.out);
    assert!(!tree.path(".codex/hooks.json").exists(), "{}", run.out);
    assert!(!tree.path(".cursor/hooks.json").exists(), "{}", run.out);
    assert_eq!(
        skill_at(&tree.path(".agents/skills/klin/SKILL.md")),
        "a person's shared skill\n"
    );
}

/// The matcher of an older klin is stale, and the reconciler brings it to today's contract
/// rather than leaving the host with the entry it already holds. #214.
#[test]
fn install_replaces_a_stale_matcher_of_klins_own() {
    let tree = a_repository();
    let stale = serde_json::json!({"hooks": {"PreToolUse": [
        {"matcher": "Write|Edit", "hooks": [{"type": "command", "command": line("guard")}]}
    ]}});
    tree.write(".claude/settings.json", &stale.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = settings(&tree);
    assert_eq!(
        matchers(&settings, "PreToolUse"),
        [SHARED_MATCHER],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "PreToolUse"),
        [line("guard")],
        "{settings}"
    );
}

/// A stale command of klin's is replaced where it stands, so a person's entry that follows it
/// keeps its place.
#[test]
fn install_replaces_a_stale_command_of_klins_own_in_place() {
    let tree = a_repository();
    let stale = serde_json::json!({"hooks": {"Stop": [
        {"hooks": [{"type": "command", "command": "klin gate --hook"}]},
        {"hooks": [{"type": "command", "command": "cargo fmt"}]}
    ]}});
    tree.write(".claude/settings.json", &stale.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "Stop"),
        [line("gate --hook --changed"), "cargo fmt".to_string()],
        "{}",
        run.out
    );
}

/// A partial install is repaired: one klin hook does not mean the host is complete.
#[test]
fn install_repairs_a_partial_install() {
    let tree = a_repository();
    let partial = serde_json::json!({"hooks": {"Stop": [
        {"hooks": [{"type": "command", "command": line("gate --hook --changed")}]}
    ]}});
    tree.write(".claude/settings.json", &partial.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = settings(&tree);
    for event in CLAUDE_EVENTS {
        assert_eq!(commands(&settings, event).len(), 1, "{event}: {settings}");
    }
}

/// Two copies of one entry run the lifecycle twice, so the reconciler keeps one.
#[test]
fn install_removes_a_duplicate_entry_of_klins() {
    let tree = a_repository();
    let doubled = serde_json::json!({"hooks": {"Stop": [
        {"hooks": [{"type": "command", "command": line("gate --hook --changed")}]},
        {"hooks": [{"type": "command", "command": line("gate --hook --changed")}]}
    ]}});
    tree.write(".claude/settings.json", &doubled.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

/// An event klin no longer writes keeps a person's entries and loses klin's.
#[test]
fn install_removes_klins_entry_from_an_event_it_no_longer_writes() {
    let tree = a_repository();
    let retired = serde_json::json!({"hooks": {
        "PostToolUse": [{"hooks": [{"type": "command", "command": line("guard")}]}],
        "SubagentStop": [
            {"hooks": [{"type": "command", "command": line("gate --hook --changed")}]},
            {"hooks": [{"type": "command", "command": "cargo fmt"}]}
        ]
    }});
    tree.write(".claude/settings.json", &retired.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = settings(&tree);
    assert_eq!(settings["hooks"]["PostToolUse"], Value::Null, "{settings}");
    assert_eq!(
        commands(&settings, "SubagentStop"),
        ["cargo fmt".to_string()],
        "{settings}"
    );
}

/// A hook that runs another klin command is a person's, and the reconciler leaves it alone.
#[test]
fn install_keeps_a_hook_that_runs_another_klin_command() {
    let tree = a_repository();
    let held = serde_json::json!({"hooks": {"SessionStart": [
        {"hooks": [{"type": "command", "command": "klin stats"}]}
    ]}});
    tree.write(".claude/settings.json", &held.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "SessionStart"),
        ["klin stats".to_string(), line("radius")],
        "{}",
        run.out
    );
}

/// A hook that only mentions klin belongs to another tool, and reading it as klin's would
/// leave that event ungated. #107.
#[test]
fn install_keeps_a_hook_that_only_mentions_klin() {
    let tree = a_repository();
    tree.write(
        ".claude/settings.json",
        r#"{"hooks": {"Stop": [{"hooks": [{"type": "command",
          "command": "/work/klin-ui/scripts/fmt.sh"}]}]}}"#,
    );

    let run = tree.run(&["install"]);
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

/// A second run over a complete install writes no file at all, so git sees nothing.
#[test]
fn a_second_complete_run_changes_nothing() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    tree.write(".cursor/rules", "\n");
    assert_eq!(tree.run(&["install"]).code, 0);
    tree.commit("the hooks klin wrote");

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.status(), "", "{}", run.out);
    assert!(run.says("already current"), "{}", run.out);
}

#[test]
fn a_second_cursor_run_adds_no_second_entry() {
    let tree = a_repository();
    tree.write(".cursor/hooks.json", "{}\n");
    assert_eq!(tree.run(&["install", "--host", "cursor"]).code, 0);

    let run = tree.run(&["install", "--host", "cursor"]);
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

/// Everything a run can decide is decided before it writes, so a host file klin cannot read
/// leaves the repository as it was.
#[test]
fn install_writes_nothing_when_a_host_file_cannot_be_read() {
    let tree = a_repository();
    tree.write(".claude/settings.json", r#"{"hooks": []}"#);

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(!tree.path("klin.json").exists(), "{}", run.out);
    assert_eq!(
        settings(&tree)["hooks"],
        serde_json::json!([]),
        "{}",
        run.out
    );
}

/// Every hook line klin writes resolves the binary first, so a machine that holds no klin says
/// nothing on every event of every session instead of failing, in a tree that did not opt in.
/// #147.
#[test]
fn a_written_hook_line_says_nothing_when_no_binary_resolves() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    assert_eq!(tree.run(&["install"]).code, 0);
    std::fs::remove_file(tree.path("klin.json")).unwrap_or_else(|why| panic!("{why}"));

    let settings = settings(&tree);
    for event in CLAUDE_EVENTS {
        for command in commands(&settings, event) {
            let outcome = std::process::Command::new("/bin/sh")
                .args(["-c", &command])
                .current_dir(tree.root())
                .env("PATH", "")
                .env_remove("CLAUDE_PROJECT_DIR")
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

/// A settings file kept in a dotfiles tree is a link. klin follows it, so the link survives
/// and the tree it points into holds the hooks. #147.
#[test]
fn install_follows_a_settings_file_that_is_a_link() {
    use std::os::unix::fs::PermissionsExt;
    let tree = a_repository();
    let held = tree.write("dotfiles/settings.json", "{}\n");
    let link = tree.path(".claude/settings.json");
    assert!(std::fs::create_dir_all(tree.path(".claude")).is_ok());
    assert!(std::os::unix::fs::symlink(&held, &link).is_ok());
    let narrow = std::fs::Permissions::from_mode(0o600);
    assert!(std::fs::set_permissions(&held, narrow).is_ok());

    let run = tree.run(&["install"]);
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

/// One user-scope install covers every repository, and no repository file carries klin. #137.
#[test]
fn install_user_writes_the_persons_own_file_and_leaves_the_repository_alone() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(".claude/settings.json", "{}\n");

    let at = home_of(&home);
    let run = tree.run_with(&[("HOME", at.as_str())], &["install", "--user"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".claude/settings.json"));
    assert_eq!(
        commands(&written, "PreToolUse"),
        [line("guard")],
        "{written}"
    );
    assert_eq!(
        skill_at(&home.path(".claude/skills/klin/SKILL.md")),
        CANONICAL_SKILL
    );
    assert!(!tree.path(".claude/settings.json").exists(), "{}", run.out);
    assert!(
        run.says("every repository you open on this machine"),
        "{}",
        run.out
    );
}

/// User scope is one machine, and the words for it never promise a cloud or remote agent.
#[test]
fn install_user_writes_no_configuration_beside_the_home_directory() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(".claude/settings.json", "{}\n");

    let at = home_of(&home);
    let run = tree.run_with(&[("HOME", at.as_str())], &["install", "--user"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!home.path("klin.json").exists(), "{}", run.out);
    assert!(tree.path("klin.json").is_file(), "{}", run.out);
    assert!(
        run.says("do not reach a cloud or remote agent"),
        "{}",
        run.out
    );
}

/// Outside a repository the project form has no root to write, and says which form does.
#[test]
fn install_outside_a_repository_is_refused() {
    let tree = Tree::bare();

    let run = tree.run(&["install", "--host", "claude"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("--user"), "{}", run.out);
}

#[test]
fn install_user_outside_a_repository_opts_no_repository_in() {
    let tree = Tree::bare();
    let home = Tree::bare();

    let at = home_of(&home);
    let run = tree.run_with(
        &[("HOME", at.as_str())],
        &["install", "--user", "--host", "claude"],
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("no repository was opted in"), "{}", run.out);
    assert_eq!(
        commands(&settings_at(&home.path(".claude/settings.json")), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

/// A host reads its user file and the repository's together, so a project write over a
/// user-scope install would double every event. #147.
#[test]
fn install_adds_nothing_where_the_persons_own_file_already_holds_klins_entries() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(".claude/settings.json", "{}\n");
    let at = home_of(&home);
    let environment = [("HOME", at.as_str())];
    assert_eq!(tree.run_with(&environment, &["install", "--user"]).code, 0);

    let run = tree.run_with(&environment, &["install", "--host", "claude"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".claude/settings.json").exists(), "{}", run.out);
    assert!(run.says("klin install --user"), "{}", run.out);
}

const A_CURSOR_PLUGIN: &str = r#"{"name":"klin","version":"0.1.1"}"#;

/// Cursor documents local development plugins under `plugins/local/<name>`.
#[test]
fn install_adds_nothing_when_the_local_cursor_plugin_is_installed() {
    let tree = a_repository();
    tree.write(
        ".cursor/plugins/local/klin/.cursor-plugin/plugin.json",
        A_CURSOR_PLUGIN,
    );

    let run = tree.run(&["install", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".cursor/hooks.json").exists(), "{}", run.out);
    assert!(
        !tree.path(".agents/skills/klin/SKILL.md").exists(),
        "{}",
        run.out
    );
    assert!(run.says("plugin"), "{}", run.out);
}

/// Cursor records no enabled state klin can read, so a plugin copy on disk holds the hooks back
/// whether or not Cursor loads it. The run names the copy and how to move to committed hooks,
/// and once the copy is gone the same command writes them. Spec 19.3.
#[test]
fn a_cursor_plugin_copy_names_its_removal_and_then_the_hooks_are_written() {
    let tree = a_repository();
    tree.write(
        ".cursor/plugins/local/klin/.cursor-plugin/plugin.json",
        A_CURSOR_PLUGIN,
    );

    let held = tree.run(&["install", "--host", "cursor"]);
    assert_eq!(held.code, 0, "{}", held.out);
    assert!(
        held.says(&tree.at(".cursor/plugins/local/klin")),
        "{}",
        held.out
    );
    assert!(held.says("remove it"), "{}", held.out);

    std::fs::remove_dir_all(tree.path(".cursor/plugins")).unwrap_or_else(|why| panic!("{why}"));
    let moved = tree.run(&["install", "--host", "cursor"]);
    assert_eq!(moved.code, 0, "{}", moved.out);
    assert!(tree.path(".cursor/hooks.json").is_file(), "{}", moved.out);
    assert!(
        tree.path(".agents/skills/klin/SKILL.md").is_file(),
        "{}",
        moved.out
    );
}

/// Cursor marketplace installs observed in 3.20.21 live under
/// `plugins/cache/<marketplace>/<plugin>/<revision>`.
#[test]
fn install_adds_nothing_when_a_marketplace_cursor_plugin_is_installed() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(
        ".cursor/plugins/cache/team-marketplace/klin/revision/.cursor-plugin/plugin.json",
        A_CURSOR_PLUGIN,
    );
    let at = home_of(&home);

    let run = tree.run_with(&[("HOME", at.as_str())], &["install", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".cursor/hooks.json").exists(), "{}", run.out);
    assert!(
        !tree.path(".agents/skills/klin/SKILL.md").exists(),
        "{}",
        run.out
    );
    assert!(run.says("plugin"), "{}", run.out);
    assert!(run.says("in Cursor"), "{}", run.out);
    assert!(!run.says("remove it"), "{}", run.out);
}

/// klin reads the two layouts Cursor installs a plugin into and no other, so a klin manifest
/// elsewhere under `plugins`, such as a marketplace's own source, holds no hooks back. Spec 19.3.
#[test]
fn install_writes_for_cursor_past_a_klin_manifest_cursor_did_not_install() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(
        ".cursor/plugins/marketplaces/team/plugins/klin/.cursor-plugin/plugin.json",
        A_CURSOR_PLUGIN,
    );
    let at = home_of(&home);

    let run = tree.run_with(&[("HOME", at.as_str())], &["install", "--host", "cursor"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(tree.path(".cursor/hooks.json").is_file(), "{}", run.out);
}

const A_CODEX_PLUGIN: &str = "[plugins.\"klin@klin\"]\nenabled = true\n";
const A_CODEX_PLUGIN_OFF: &str =
    "[plugins.\"klin@klin\"]\nenabled = false\n\n[plugins.\"other@klin\"]\n";

/// Codex CLI lists its plugins in `config.toml`, and a plugin table is on unless it says
/// `enabled = false`. Spec 19.3.
#[test]
fn install_adds_nothing_when_the_codex_plugin_is_enabled() {
    let tree = a_repository();
    tree.write(".codex/config.toml", A_CODEX_PLUGIN);

    let run = tree.run(&["install", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.path(".codex/hooks.json").exists(), "{}", run.out);
    assert!(
        !tree.path(".agents/skills/klin/SKILL.md").exists(),
        "{}",
        run.out
    );
    assert!(run.says("config.toml"), "{}", run.out);
}

#[test]
fn install_writes_for_codex_when_its_plugin_table_is_switched_off() {
    let tree = a_repository();
    tree.write(".codex/config.toml", A_CODEX_PLUGIN_OFF);

    let run = tree.run(&["install", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&codex_settings(&tree), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

/// Codex CLI enables plugins in `config.toml`, not under a key in `hooks.json`. So Claude
/// Code's plugin key in the Codex file means nothing to klin.
#[test]
fn install_for_codex_ignores_claudes_plugin_key() {
    let tree = a_repository();
    tree.write(".codex/hooks.json", A_PLUGIN);

    let run = tree.run(&["install", "--host", "codex"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&codex_settings(&tree), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

#[test]
fn install_writes_where_the_plugin_is_listed_but_switched_off() {
    let tree = a_repository();
    tree.write(
        ".claude/settings.json",
        r#"{"enabledPlugins": {"klin@klin-marketplace": false}}"#,
    );

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "PreToolUse"),
        [line("guard")],
        "{}",
        run.out
    );
}

/// A plugin one repository enables gates that repository, not the machine, so it does not
/// stand in the way of the user-scope install. #147.
#[test]
fn install_user_writes_where_the_plugin_is_enabled_in_the_repository_alone() {
    let tree = a_repository();
    tree.write(".claude/settings.json", A_PLUGIN);
    let home = Tree::bare();
    home.write(".claude/settings.json", "{}\n");
    let at = home_of(&home);

    let run = tree.run_with(&[("HOME", at.as_str())], &["install", "--user"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".claude/settings.json"));
    assert_eq!(
        commands(&written, "PreToolUse"),
        [line("guard")],
        "{written}"
    );
}

#[test]
fn install_user_adds_nothing_when_the_persons_plugin_is_enabled() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(".claude/settings.json", A_PLUGIN);
    let at = home_of(&home);

    let run = tree.run_with(&[("HOME", at.as_str())], &["install", "--user"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let written = settings_at(&home.path(".claude/settings.json"));
    assert_eq!(written["hooks"], Value::Null, "{}", run.out);
    assert!(
        !home.path(".claude/skills/klin/SKILL.md").exists(),
        "{}",
        run.out
    );
    assert!(run.says("plugin"), "{}", run.out);
}

#[test]
fn install_user_writes_codex_hooks_to_the_persons_own_file() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(".codex/hooks.json", "{}\n");
    let at = home_of(&home);

    let run = tree.run_with(
        &[("HOME", at.as_str())],
        &["install", "--user", "--host", "codex"],
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings_at(&home.path(".codex/hooks.json")), "Stop"),
        [line("gate --hook --changed")],
        "{}",
        run.out
    );
}

/// A settings key that is not klin's survives a reconciliation whole.
#[test]
fn install_keeps_the_settings_keys_that_are_not_klins() {
    let tree = a_repository();
    let held = serde_json::json!({
        "permissions": {"allow": ["Bash(git status:*)"]},
        "enabledPlugins": {"klin@klin-marketplace": false}
    });
    tree.write(".claude/settings.json", &held.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = settings(&tree);
    assert_eq!(settings["permissions"], held["permissions"], "{settings}");
    assert_eq!(
        settings["enabledPlugins"], held["enabledPlugins"],
        "{settings}"
    );
}

/// A write can still fail after an earlier one landed, and the run then names the files it did
/// not write rather than reporting a complete install.
#[test]
fn install_names_what_it_did_not_write_when_a_write_fails() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    assert!(std::fs::create_dir_all(tree.path(".cursor/hooks.json")).is_ok());

    let run = tree.run(&["install", "--host", "cursor", "--host", "claude"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("repository opted in"), "{}", run.out);
    assert!(run.says("incomplete: klin did not write"), "{}", run.out);
    assert!(run.says(".cursor/hooks.json"), "{}", run.out);
    assert!(run.says(".claude/settings.json"), "{}", run.out);
    assert_eq!(settings(&tree)["hooks"], Value::Null, "{}", run.out);
}

#[test]
fn install_names_a_host_file_it_wrote_before_the_skill_failed() {
    let tree = a_repository();
    tree.write(".claude/settings.json", "{}\n");
    assert!(std::fs::create_dir_all(tree.path(".claude/skills/klin/SKILL.writing")).is_ok());

    let run = tree.run(&["install", "--host", "claude"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(tree.path("klin.json").is_file(), "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "Stop"),
        [line("gate --hook --changed")]
    );
    assert!(run.says(".claude/skills/klin/SKILL.md"), "{}", run.out);
    assert!(
        run.says("; it wrote") && run.says(".claude/settings.json"),
        "{}",
        run.out
    );
}

/// An event that is not a list of entries belongs to whatever wrote it, and klin writes no
/// entry there, so the reconciler leaves it whole rather than refusing the run.
#[test]
fn install_leaves_an_event_it_does_not_write_and_cannot_read() {
    let tree = a_repository();
    let held = serde_json::json!({"hooks": {"Notification": {"note": "another tool's shape"}}});
    tree.write(".claude/settings.json", &held.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let settings = settings(&tree);
    assert_eq!(
        settings["hooks"]["Notification"], held["hooks"]["Notification"],
        "{settings}"
    );
    assert_eq!(
        commands(&settings, "PreToolUse"),
        [line("guard")],
        "{settings}"
    );
}

/// klin writes the marker, the host's file and its skill. It never edits `.gitignore`, because
/// klin writes nothing the working tree can see.
#[test]
fn install_writes_the_marker_and_the_host_file_and_nothing_else() {
    let tree = a_repository();
    tree.write(".gitignore", "/target\n");
    tree.write(".claude/settings.json", "{}\n");
    tree.commit("an ignore file and a settings file");

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        tree.status(),
        " M .claude/settings.json\n?? .claude/skills/\n?? klin.json\n",
        "{}",
        run.out
    );
}

/// A host nests several commands under one entry, and a person's command may sit beside
/// klin's there. klin takes out its own command and leaves the person's where it was.
#[test]
fn install_keeps_a_persons_command_that_shares_an_entry_with_klins() {
    let tree = a_repository();
    let shared = serde_json::json!({"hooks": {"Stop": [{"hooks": [
        {"type": "command", "command": "cargo fmt --check"},
        {"type": "command", "command": "klin gate --hook"},
        {"type": "command", "command": "npm run lint"}
    ]}]}});
    tree.write(".claude/settings.json", &shared.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let held = commands(&settings(&tree), "Stop");
    assert!(held.contains(&"cargo fmt --check".to_string()), "{held:?}");
    assert!(held.contains(&"npm run lint".to_string()), "{held:?}");
    assert!(held.contains(&line("gate --hook --changed")), "{held:?}");
    assert!(!held.contains(&"klin gate --hook".to_string()), "{held:?}");
}

/// The same on an event klin no longer writes: klin's command goes and the person's stays.
#[test]
fn install_keeps_a_persons_command_on_an_event_it_no_longer_writes() {
    let tree = a_repository();
    let shared = serde_json::json!({"hooks": {"SubagentStop": [{"hooks": [
        {"type": "command", "command": "klin gate --hook"},
        {"type": "command", "command": "cargo fmt --check"}
    ]}]}});
    tree.write(".claude/settings.json", &shared.to_string());

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        commands(&settings(&tree), "SubagentStop"),
        ["cargo fmt --check".to_string()],
        "{}",
        run.out
    );
}

/// The marker is committed and the user-scope host files are not, so the marker's own line
/// says to commit it and the closing line speaks only of the host files.
#[test]
fn install_user_says_to_commit_the_marker_it_wrote() {
    let tree = a_repository();
    let home = Tree::bare();
    home.write(".claude/settings.json", "{}\n");
    let at = home_of(&home);

    let run = tree.run_with(&[("HOME", at.as_str())], &["install", "--user"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("commit it, so the repository stays opted in"),
        "{}",
        run.out
    );
    assert!(
        run.says("These host files cover every repository"),
        "{}",
        run.out
    );
}

/// A run that wrote the marker and no host file must not tell a person to commit hooks that
/// were never written.
#[test]
fn install_speaks_of_no_hooks_where_the_plugin_owns_every_host() {
    let tree = a_repository();
    tree.write(".claude/settings.json", A_PLUGIN);

    let run = tree.run(&["install"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(tree.path("klin.json").is_file(), "{}", run.out);
    assert!(!run.says("Commit the host files"), "{}", run.out);
    assert!(run.says("already current"), "{}", run.out);
}
