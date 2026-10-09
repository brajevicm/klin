mod harness;

use harness::{Tree, feed};
use serde_json::Value;

const STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

fn status(tree: &Tree) -> Value {
    let run = tree.run(&["status", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    run.json()
}

fn state(document: &Value, host: &str) -> Vec<String> {
    document["integrations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["host"] == host)
        .map(|row| format!("{} {} {}", row["scope"], row["route"], row["state"]).replace('"', ""))
        .collect()
}

/// Every file under the state directory, so a test can pin that `status` wrote none.
fn files(tree: &Tree) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![tree.path(".git/klin")];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            match path.is_dir() {
                true => pending.push(path),
                false => found.push(format!(
                    "{} {:?}",
                    path.display(),
                    entry.metadata().and_then(|held| held.modified()).ok()
                )),
            }
        }
    }
    found.sort();
    found
}

/// Each host's integration reads as `current` after `setup`, `missing` where the repository
/// proves the host and holds no hooks, and `conflict` where klin's hook lines differ from what
/// this klin writes. Spec 11.4.
#[test]
fn status_reports_each_integrations_state() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    std::fs::create_dir_all(tree.path(".codex")).expect("codex directory");
    std::fs::create_dir_all(tree.path(".cursor")).expect("cursor directory");
    let setup = tree.run(&["setup", "--host", "claude"]);
    assert_eq!(setup.code, 0, "{}", setup.out);
    tree.write(
        ".cursor/hooks.json",
        r#"{"version": 1, "hooks": {"stop": [{"command": "klin gate --hook --changed"}]}}"#,
    );

    let document = status(&tree);
    assert_eq!(document["command"], "status", "{document}");
    assert_eq!(document["schema_version"], 1, "{document}");
    assert_eq!(
        state(&document, "claude"),
        ["project hooks current"],
        "{document}"
    );
    assert_eq!(
        state(&document, "codex"),
        ["project hooks missing"],
        "{document}"
    );
    assert_eq!(
        state(&document, "cursor"),
        ["project hooks conflict"],
        "{document}"
    );

    let text = tree.run(&["status"]);
    assert_eq!(text.code, 0, "{}", text.out);
    assert!(
        text.says("integration: codex project hooks: missing"),
        "{}",
        text.out
    );
}

/// The local window is the stamp's verdict, and `status` reads it without writing anything to
/// the state directory. `report` reads the journal the same way. Spec 11.4, 13.2.
#[test]
fn status_and_report_read_the_window_and_write_nothing() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": {"README.md": 3}}"#);
    tree.words("README.md", 2);
    tree.base();
    assert_eq!(status(&tree)["window"], Value::Null);

    tree.words("README.md", 30);
    let stop = feed(tree.root(), harness::AGENT, STOP);
    assert_eq!(stop.code, 2, "{}", stop.out);
    let before = files(&tree);

    let document = status(&tree);
    assert_eq!(document["window"]["verdict"], "red", "{document}");
    assert_eq!(document["last_stop"]["verdict"], "red", "{document}");
    assert_eq!(document["last_stop"]["historical"], true, "{document}");
    assert_eq!(document["config"]["valid"], true, "{document}");
    let text = tree.run(&["status"]);
    assert!(text.says("window: red"), "{}", text.out);
    assert_eq!(files(&tree), before, "status wrote to the state directory");

    for args in [
        &["report"][..],
        &["report", "--details"],
        &["report", "--json"],
        &["report", "--since", "7d"],
    ] {
        let run = tree.run(args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.out);
    }
    assert_eq!(files(&tree), before, "report wrote to the state directory");
}

/// Without a klin.json `status` still reads, and says what it runs under. Spec 5.1, 11.4.
#[test]
fn status_without_a_config_says_so() {
    let tree = Tree::new();

    let document = status(&tree);
    assert_eq!(document["config"]["present"], false, "{document}");
    let text = tree.run(&["status"]);
    assert!(text.says("config: none, running under {}"), "{}", text.out);
    assert!(!tree.path(".git/klin").exists(), "status wrote state");
}

/// An invalid configuration is reported, not raised. Spec 11.4.
#[test]
fn status_reports_an_invalid_config_and_exits_zero() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"no_such_key": 1}"#);

    let document = status(&tree);
    assert_eq!(document["config"]["valid"], false, "{document}");
    assert!(
        document["config"]["error"]
            .as_str()
            .unwrap_or_default()
            .contains("no_such_key"),
        "{document}"
    );
}

#[test]
fn status_outside_a_repository_is_an_invocation_error() {
    let run = Tree::bare().run(&["status"]);
    assert_eq!(run.code, 2, "{}", run.out);
}

/// Only the exact skill `setup` writes is current. A deleted or unreadable skill is a conflict,
/// so `klin update` names `klin setup`. Spec 11.4, 11.8.
#[test]
fn status_reads_a_missing_or_unreadable_skill_as_a_conflict() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    assert_eq!(tree.run(&["setup", "--host", "claude"]).code, 0);
    let skill = ".claude/skills/klin/SKILL.md";

    tree.remove(skill);
    let missing = status(&tree);
    assert_eq!(
        state(&missing, "claude"),
        ["project hooks conflict"],
        "{missing}"
    );

    tree.write(skill, "");
    std::fs::set_permissions(
        tree.path(skill),
        std::os::unix::fs::PermissionsExt::from_mode(0o000),
    )
    .expect("chmod");
    let unreadable = status(&tree);
    assert_eq!(
        state(&unreadable, "claude"),
        ["project hooks conflict"],
        "{unreadable}"
    );
}

/// An earlier text of klin's skill is a conflict `setup` replaces, which reads apart from a skill
/// a person changed. Every text the skill held in git history is one klin knows, so a release
/// that changes the skill cannot leave the one before it behind. Spec 11.4, B.19.3.
#[test]
fn status_names_every_earlier_skill_as_one_setup_replaces() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    assert_eq!(tree.run(&["setup", "--host", "claude"]).code, 0);
    let skill = ".claude/skills/klin/SKILL.md";

    for text in harness::earlier_skills() {
        tree.write(skill, &text);
        let run = tree.run(&["status"]);
        assert!(run.says("klin setup replaces it"), "{text}\n{}", run.out);
    }

    tree.write(skill, "a person's skill\n");
    let changed = tree.run(&["status"]);
    assert!(changed.says("differs from klin's skill"), "{}", changed.out);
    assert!(!changed.says("klin setup replaces it"), "{}", changed.out);
}

/// klin's skill at a scope whose hook file lost klin's lines is the rest of an install, and reads
/// as a conflict with its own detail. A skill another host's hooks still use is not alone.
/// Spec 11.4.
#[test]
fn status_reads_klins_skill_without_hook_lines_as_a_conflict() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    let home = Tree::bare();
    let at = home.root().display().to_string();
    std::fs::create_dir_all(home.path(".cursor")).expect("cursor directory");
    let setup = tree.run_with(
        &[("HOME", at.as_str())],
        &["setup", "--user", "--host", "claude", "--host", "codex"],
    );
    assert_eq!(setup.code, 0, "{}", setup.out);
    home.remove(".claude/settings.json");

    let run = tree.run_with(&[("HOME", at.as_str())], &["status", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let document = run.json();
    assert_eq!(
        state(&document, "claude"),
        ["user hooks conflict"],
        "{document}"
    );
    assert_eq!(
        state(&document, "codex"),
        ["user hooks current"],
        "{document}"
    );
    assert!(state(&document, "cursor").is_empty(), "{document}");

    let text = tree.run_with(&[("HOME", at.as_str())], &["status"]);
    assert!(text.says("holds none of klin's hook lines"), "{}", text.out);
}

/// The same skill alone in a repository reads as a conflict too, not as `missing`: the skill is
/// part of a copy, and that copy lost its hook lines. Spec 11.4.
#[test]
fn status_reads_klins_skill_without_hook_lines_in_a_repository_as_a_conflict() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    assert_eq!(tree.run(&["setup", "--host", "claude"]).code, 0);
    tree.remove(".claude/settings.json");

    let document = status(&tree);
    assert_eq!(
        state(&document, "claude"),
        ["project hooks conflict"],
        "{document}"
    );
    let text = tree.run(&["status"]);
    assert!(text.says("holds none of klin's hook lines"), "{}", text.out);
}

/// Codex and Cursor read one skill. Alone beside one proven host it names that host; beside two
/// it names neither, so no hint asks for a host the person never used. A hook file klin cannot
/// read is named as such. Spec 11.4.
#[test]
fn status_names_a_shared_skill_only_for_its_one_proven_host() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    let home = Tree::bare();
    let at = home.root().display().to_string();
    let setup = tree.run_with(
        &[("HOME", at.as_str())],
        &["setup", "--user", "--host", "codex"],
    );
    assert_eq!(setup.code, 0, "{}", setup.out);
    home.remove(".codex/hooks.json");
    let rows = |document: &Value| [state(document, "codex"), state(document, "cursor")];

    let run = tree.run_with(&[("HOME", at.as_str())], &["status", "--json"]);
    let alone = run.json();
    assert_eq!(
        rows(&alone),
        [vec!["user hooks conflict".to_string()], vec![]],
        "{alone}"
    );

    std::fs::create_dir_all(home.path(".cursor")).expect("cursor directory");
    let run = tree.run_with(&[("HOME", at.as_str())], &["status", "--json"]);
    let shared = run.json();
    assert_eq!(rows(&shared), [Vec::<String>::new(), vec![]], "{shared}");

    std::fs::remove_dir(home.path(".cursor")).expect("cursor directory");
    home.write(".codex/hooks.json", "{,}");
    let text = tree.run_with(&[("HOME", at.as_str())], &["status"]);
    assert!(text.says("could not be read"), "{}", text.out);
}
