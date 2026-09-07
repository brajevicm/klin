mod harness;

use harness::{Run, Tree, feed};

fn guard(event: &str) -> Run {
    feed(Tree::new().root(), &["guard"], event)
}

fn bash(command: &str) -> Run {
    guard(&format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": {command:?}}}}}"#
    ))
}

fn tool(name: &str, key: &str, file: &str) -> Run {
    guard(&format!(
        r#"{{"tool_name": {name:?}, "tool_input": {{{key:?}: {file:?}}}}}"#
    ))
}

fn edit(name: &str, file: &str) -> Run {
    tool(name, "file_path", file)
}

#[test]
fn refuses_a_command_that_writes_a_baseline() {
    for command in [
        "klin escapes --write-baseline",
        "cd repo && klin complexity --quiet --write-baseline",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

#[test]
fn refuses_a_command_that_edits_the_configuration_a_baseline_or_the_hooks() {
    for command in [
        "cd /x && sed -i '' 's/8/80/' klin.json",
        "echo '[]' > klin/escapes-baseline.json",
        "echo '[]' >>klin.json",
        "cp /tmp/loose.json klin/complexity-baseline.json",
        "rm klin.json",
        "mv klin.json klin.json.bak",
        "truncate -s 0 klin.json",
        "tee .claude/settings.json < /tmp/loose.json",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

#[test]
fn refuses_an_edit_of_the_configuration_a_baseline_or_the_hooks() {
    for (name, file) in [
        ("Edit", "/repo/klin.json"),
        ("Write", "/repo/klin/escapes-baseline.json"),
        ("Edit", "/repo/.claude/settings.json"),
        ("Write", "klin.json"),
    ] {
        let run = edit(name, file);
        assert_eq!(run.code, 2, "{name} {file}: {}", run.out);
        assert!(run.says("refused"), "{name} {file}: {}", run.out);
    }
    let run = tool("NotebookEdit", "notebook_path", "/repo/klin.json");
    assert_eq!(run.code, 2, "{}", run.out);
}

#[test]
fn refuses_git_putting_back_old_content_of_a_guarded_file() {
    for command in [
        "git checkout -- klin.json",
        "git checkout HEAD~3 -- .claude/settings.json",
        "git restore klin/escapes-baseline.json",
        "git -C /repo restore --source=HEAD~1 klin.json",
        "git restore .",
        "git checkout .",
        "git checkout HEAD -- klin/",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

#[test]
fn allows_git_that_leaves_the_guarded_files_alone() {
    for command in [
        "git checkout feature-branch",
        "git checkout -b klin-work",
        "git restore src/main.rs",
        "git log -- klin.json",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

#[test]
fn refuses_an_edit_of_the_hook_settings_of_cursor_and_codex() {
    for (name, file) in [
        ("Edit", "/repo/.cursor/hooks.json"),
        ("Write", "/Users/someone/.cursor/hooks.json"),
        ("Edit", "/repo/.codex/config.toml"),
        ("Write", "/Users/someone/.codex/config.toml"),
    ] {
        let run = edit(name, file);
        assert_eq!(run.code, 2, "{name} {file}: {}", run.out);
        assert!(run.says("refused"), "{name} {file}: {}", run.out);
    }
    let run = bash("echo '{}' > .cursor/hooks.json");
    assert_eq!(run.code, 2, "{}", run.out);
}

#[test]
fn the_refusal_names_what_it_protects_and_never_the_command_that_rewrites_a_baseline() {
    let run = edit("Edit", "/repo/klin.json");
    assert!(run.says("klin.json"), "{}", run.out);
    assert!(run.says("baseline"), "{}", run.out);
    assert!(!run.says("--write-baseline"), "{}", run.out);
}

#[test]
fn allows_a_command_that_only_reads_what_is_guarded() {
    for command in [
        "klin doc-size --quiet",
        "cat klin.json",
        "cat klin.json > /tmp/copy.json",
        "git diff klin/",
        "sed -n '1,5p' klin.json",
        "grep -rn baseline src/",
        "rm /tmp/scratch.json && cat klin.json",
        "cargo test > /tmp/out.txt",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

#[test]
fn allows_a_tool_call_that_leaves_the_guarded_files_alone() {
    for (name, file) in [
        ("Edit", "/repo/src/klin_of_life.rs"),
        ("Write", "/repo/docs/klin.md"),
        ("Read", "/repo/klin.json"),
    ] {
        let run = edit(name, file);
        assert_eq!(run.code, 0, "{name} {file}: {}", run.out);
    }
}

#[test]
fn an_event_it_cannot_read_is_allowed_through() {
    for event in [
        "not json",
        "",
        "{}",
        r#"{"tool_name": "Bash"}"#,
        r#"{"tool_name": "Bash", "tool_input": {"command": null}}"#,
        r#"{"tool_input": {"pattern": "klin.json"}}"#,
    ] {
        let run = guard(event);
        assert_eq!(run.code, 0, "{event}: {}", run.out);
    }
}

#[test]
fn it_never_reads_the_configuration() {
    let tree = Tree::new();
    tree.write("klin.json", "{not json at all");

    let allowed = feed(
        tree.root(),
        &["guard"],
        r#"{"tool_name": "Edit", "tool_input": {"file_path": "src/main.rs"}}"#,
    );
    assert_eq!(allowed.code, 0, "{}", allowed.out);
    assert_eq!(allowed.out, "", "{}", allowed.out);

    let refused = feed(
        tree.root(),
        &["guard"],
        r#"{"tool_name": "Edit", "tool_input": {"file_path": "klin.json"}}"#,
    );
    assert_eq!(refused.code, 2, "{}", refused.out);
    assert!(refused.says("refused"), "{}", refused.out);
    assert!(!refused.says("could not be read"), "{}", refused.out);
}
