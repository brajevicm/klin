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
fn refuses_a_command_that_edits_the_configuration_or_the_hooks() {
    for command in [
        "cd /x && sed -i '' 's/8/80/' klin.json",
        "echo '[]' >>klin.json",
        "cp /tmp/loose.json klin.json",
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
fn refuses_an_edit_of_the_configuration_or_the_hooks() {
    for (name, file) in [
        ("Edit", "/repo/klin.json"),
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

/// GitHub reads the code owners from three places, so guarding one spelling guards nothing.
#[test]
fn refuses_an_edit_of_the_code_owners_wherever_they_sit() {
    for at in [
        "CODEOWNERS",
        ".github/CODEOWNERS",
        "docs/CODEOWNERS",
        "/repo/CODEOWNERS",
    ] {
        let run = edit("Write", at);
        assert_eq!(run.code, 2, "{at}: {}", run.out);
        assert!(run.says("refused"), "{at}: {}", run.out);

        let run = bash(&format!("echo '* @me' > {at}"));
        assert_eq!(run.code, 2, "{at}: {}", run.out);
    }
}

#[test]
fn allows_reading_the_code_owners() {
    let run = bash("cat docs/CODEOWNERS");
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn refuses_git_putting_back_old_content_of_a_guarded_file() {
    for command in [
        "git checkout -- klin.json",
        "git checkout HEAD~3 -- .claude/settings.json",
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
fn the_refusal_names_what_it_protects() {
    let run = edit("Edit", "/repo/klin.json");
    assert!(run.says("klin.json"), "{}", run.out);
    assert!(run.says("hooks"), "{}", run.out);
    assert!(!run.says("baseline"), "{}", run.out);
}

#[test]
fn refuses_the_command_that_fills_in_the_configuration() {
    for command in [
        "klin init --add",
        "cd repo && target/debug/klin init --add --config klin.json",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
    let allowed = bash("klin init");
    assert_eq!(allowed.code, 0, "{}", allowed.out);
}

#[test]
fn allows_a_command_naming_a_file_that_used_to_be_a_baseline() {
    for command in [
        "rm quality/escapes-baseline.json",
        "git rm quality/complexity-baseline.json",
        "klin escapes --write-baseline",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

#[test]
fn allows_a_command_that_only_reads_what_is_guarded() {
    for command in [
        "klin doc-size --quiet",
        "cat klin.json",
        "cat klin.json > /tmp/copy.json",
        "git diff klin/",
        "grep klin.json src/",
        "rg klin.json",
        "grep -rn baseline src/",
        "rm /tmp/scratch.json && cat klin.json",
        "cargo test > /tmp/out.txt",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

#[test]
fn allows_git_add_and_commit_naming_the_config() {
    for command in ["git add klin.json", "git commit klin.json -m 'wip'"] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

#[test]
fn allows_a_commit_message_that_mentions_the_config() {
    let run = bash("git commit -am 'fix the parser that reads klin.json'");
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn refuses_echo_naming_the_config() {
    let run = bash("echo klin.json");
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("refused"), "{}", run.out);
}

#[test]
fn refuses_every_route_an_allowlist_of_writers_missed() {
    for command in [
        "perl -i -pe 's/a/b/' klin.json",
        "python3 - <<'EOF'\nopen('klin.json', 'w').write('{}')\nEOF",
        "ed klin.json",
        "awk '{ print }' notes.txt > klin.json",
        "patch -p1 klin.json < fix.diff",
        "git apply klin.json",
        "git stash pop klin.json",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

#[test]
fn refuses_an_interpreter_a_reader_reaches_through_a_command_substitution() {
    for command in [
        "cat \"$(python3 -c \"open('klin.json','w')\")\"",
        "wc -l `perl -i -pe 's/a/b/' klin.json`",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

#[test]
fn refuses_a_glob_that_matches_a_guarded_name() {
    for command in [
        "perl -i -pe 's/a/b/' klin.*",
        "rm klin.js*n",
        "rm *.json",
        "rm CODEOWNER*",
        "rm ?lin.json",
        "rm klin.jso[n]",
        "rm C?DEOWNERS",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

/// A `*` with nothing before it names no guarded file, so it must not stand for one.
#[test]
fn allows_a_glob_with_nothing_before_the_star() {
    for command in [
        "ffmpeg *.rs out",
        "ffmpeg src/a.rs out",
        "find src tests -name '*.rs' | sort",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

/// A quoted separator is a character in an argument, not the end of a command, so the reader
/// at the front of the command keeps its exemption.
#[test]
fn allows_a_reader_whose_argument_quotes_a_separator() {
    for command in [
        r#"grep -rn "aaa\|bbb" --include=*.rs ."#,
        r#"grep -rn "aaa" --include=*.rs ."#,
        r#"grep -rn "aaa\|bbb" src"#,
        "grep -rn 'a;b' --include=*.rs .",
        r#"cat "a|klin.json""#,
        "cat 'a;klin.json'",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

/// The quote-blind split tears `git` off the front and loses the restore, so only the pass that
/// honours the quoting sees the whole command.
#[test]
fn refuses_a_whole_tree_restore_whose_argument_quotes_a_separator() {
    for command in [r#"git checkout "a|b" ."#, "git restore 'a;b' ."] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

/// An unbalanced quote says nothing about where an argument ends, so the guard falls back to
/// splitting on every separator rather than trusting the quote.
#[test]
fn refuses_a_left_open_quote_that_hides_a_separator() {
    for command in [
        "cat \"unclosed | rm klin.json",
        "cat 'unclosed ; rm CODEOWNERS",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(run.says("refused"), "{command}: {}", run.out);
    }
}

#[test]
fn allows_a_reader_that_carries_a_global_git_flag() {
    for command in [
        "git -C sub add klin.json",
        "git -c core.pager=cat log klin.json",
        "git --git-dir=/repo/.git diff klin.json",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
    }
}

#[test]
fn allows_a_line_continued_reader_command_that_names_the_config() {
    let run = bash("git commit \\\n  -m 'mentions klin.json' \\\n  -m 'a second paragraph'");
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn refuses_a_heredoc_body_that_names_a_guarded_file() {
    let run = bash("cat <<'EOF' > /tmp/notes\nsee klin.json for details\nEOF");
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("refused"), "{}", run.out);
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
