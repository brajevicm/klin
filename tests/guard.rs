mod harness;

use harness::{Run, Tree, feed};

const ASK: &str = r#""permissionDecision":"ask""#;

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

#[track_caller]
fn denied(run: &Run, what: &str) {
    assert_eq!(run.code, 2, "{what}: {}", run.out);
    assert!(run.says("refused"), "{what}: {}", run.out);
}

/// An ask is exit 0 with the decision on stdout, and its reason quotes what matched.
#[track_caller]
fn asked(run: &Run, quoted: &str, what: &str) {
    assert_eq!(run.code, 0, "{what}: {}", run.out);
    assert!(run.says(ASK), "{what}: {}", run.out);
    assert!(run.says(&format!(r#"\"{quoted}\""#)), "{what}: {}", run.out);
}

#[track_caller]
fn allowed(run: &Run, what: &str) {
    assert_eq!(run.code, 0, "{what}: {}", run.out);
    assert!(!run.says(ASK), "{what}: {}", run.out);
}

#[test]
fn refuses_an_edit_of_the_configuration_or_the_hooks() {
    for (name, file) in [
        ("Edit", "/repo/klin.json"),
        ("Edit", "/repo/.claude/settings.json"),
        ("Write", "klin.json"),
    ] {
        denied(&edit(name, file), &format!("{name} {file}"));
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
        denied(&edit("Write", at), at);
    }
}

#[test]
fn refuses_a_redirect_onto_the_configuration_the_hooks_or_the_code_owners() {
    for command in [
        "echo '{}' > klin.json",
        "echo '[]' >>klin.json",
        "echo '{}' > .cursor/hooks.json",
        "echo '* @me' > docs/CODEOWNERS",
        "awk '{ print }' notes.txt > klin.json",
    ] {
        denied(&bash(command), command);
    }
}

#[test]
fn refuses_a_restore_of_a_whole_tree() {
    for command in [
        "git restore .",
        "git checkout .",
        "git checkout HEAD -- klin/",
    ] {
        denied(&bash(command), command);
    }
}

/// A restore of one guarded file is a write klin cannot tell from a mention, so it asks.
#[test]
fn asks_about_git_putting_back_old_content_of_a_guarded_file() {
    for (command, quoted) in [
        ("git checkout -- klin.json", "klin.json"),
        (
            "git checkout HEAD~3 -- .claude/settings.json",
            ".claude/settings.json",
        ),
        (
            "git -C /repo restore --source=HEAD~1 klin.json",
            "klin.json",
        ),
    ] {
        asked(&bash(command), quoted, command);
    }
}

#[test]
fn refuses_the_commands_only_a_person_runs() {
    for command in [
        "klin init",
        "klin init --add",
        "cd repo && target/debug/klin init --add --config klin.json",
        "klin turn reset",
        "target/debug/klin turn reset",
    ] {
        denied(&bash(command), command);
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
        denied(&edit(name, file), &format!("{name} {file}"));
    }
}

#[test]
fn the_refusal_names_what_it_protects() {
    let run = edit("Edit", "/repo/klin.json");
    assert!(run.says("klin.json"), "{}", run.out);
    assert!(run.says("hooks"), "{}", run.out);
    assert!(!run.says("baseline"), "{}", run.out);
}

/// Outside the reader list klin cannot tell a write from a mention, so a person decides.
#[test]
fn asks_about_a_command_outside_the_reader_list_that_names_a_guarded_path() {
    for (command, quoted) in [
        ("cd /x && sed -i '' 's/8/80/' klin.json", "klin.json"),
        ("cp /tmp/loose.json klin.json", "klin.json"),
        ("rm klin.json", "klin.json"),
        ("mv klin.json klin.json.bak", "klin.json"),
        ("truncate -s 0 klin.json", "klin.json"),
        (
            "tee .claude/settings.json < /tmp/loose.json",
            ".claude/settings.json",
        ),
        ("echo klin.json", "klin.json"),
        ("perl -i -pe 's/a/b/' klin.json", "klin.json"),
        ("ed klin.json", "klin.json"),
        ("patch -p1 klin.json < fix.diff", "klin.json"),
        ("git apply klin.json", "klin.json"),
        ("git stash pop klin.json", "klin.json"),
    ] {
        asked(&bash(command), quoted, command);
    }
}

#[test]
fn asks_about_an_edit_or_a_redirect_onto_klins_own_state_or_its_ref() {
    asked(
        &edit("Write", ".git/klin/turn"),
        ".git/klin/turn",
        "an edit",
    );
    asked(
        &edit("Edit", ".git/klin/build-blocked"),
        ".git/klin/build-blocked",
        "an edit",
    );
    for (command, quoted) in [
        ("echo x > .git/klin/turn", ".git/klin/turn"),
        ("rm -rf .git/klin", ".git/klin"),
        (
            "git update-ref -d refs/worktree/klin/turn",
            "refs/worktree/klin/turn",
        ),
        ("find .git/klin -delete", ".git/klin"),
    ] {
        asked(&bash(command), quoted, command);
    }
}

/// A verification file configures a check. klin cannot tell a loosening from a fix.
#[test]
fn asks_about_an_edit_or_a_command_that_names_a_verification_file() {
    for file in [
        ".eslintrc.json",
        "eslint.config.mjs",
        "pytest.ini",
        "jest.config.ts",
        ".coveragerc",
        "codecov.yml",
        ".github/workflows/ci.yml",
    ] {
        asked(&edit("Write", file), file, file);
        let command = format!("rm {file}");
        asked(&bash(&command), file, &command);
    }
}

/// `pyproject.toml` is a verification file only when it holds a check's table.
#[test]
fn asks_about_a_pyproject_that_carries_a_checks_table() {
    let tree = Tree::new();
    tree.write(
        "pyproject.toml",
        "[tool.pytest.ini_options]\naddopts = \"-q\"\n",
    );
    let event = r#"{"tool_name": "Edit", "tool_input": {"file_path": "pyproject.toml"}}"#;
    asked(
        &feed(tree.root(), &["guard"], event),
        "pyproject.toml",
        "a tabled pyproject",
    );

    let plain = Tree::new();
    plain.write("pyproject.toml", "[project]\nname = \"t\"\n");
    allowed(
        &feed(plain.root(), &["guard"], event),
        "a pyproject with no check",
    );
}

#[test]
fn allows_the_plumbing_that_reads_a_stamp() {
    for command in [
        "git rev-parse --absolute-git-dir",
        "git cat-file -p refs/worktree/klin/turn",
        "git for-each-ref refs/worktree/klin",
        "du -sh .git/klin",
        "find .git/klin -type f",
        "cat .git/klin/turn",
    ] {
        allowed(&bash(command), command);
    }
}

#[test]
fn allows_reading_a_guarded_file_or_a_verification_file() {
    for command in [
        "cat docs/CODEOWNERS",
        "cat .eslintrc.json",
        "grep -n threshold .coveragerc",
        "cat .github/workflows/ci.yml",
    ] {
        allowed(&bash(command), command);
    }
    allowed(&edit("Read", ".github/workflows/ci.yml"), "a read tool");
}

#[test]
fn allows_git_that_leaves_the_guarded_files_alone() {
    for command in [
        "git checkout feature-branch",
        "git checkout -b klin-work",
        "git restore src/main.rs",
        "git log -- klin.json",
    ] {
        allowed(&bash(command), command);
    }
}

#[test]
fn allows_a_command_naming_a_file_that_used_to_be_a_baseline() {
    for command in [
        "rm quality/escapes-baseline.json",
        "git rm quality/complexity-baseline.json",
        "klin escapes --write-baseline",
    ] {
        allowed(&bash(command), command);
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
        "cargo test > /tmp/out.txt",
    ] {
        allowed(&bash(command), command);
    }
}

#[test]
fn allows_git_add_and_commit_naming_the_config() {
    for command in ["git add klin.json", "git commit klin.json -m 'wip'"] {
        allowed(&bash(command), command);
    }
}

#[test]
fn allows_a_commit_message_that_mentions_the_config() {
    allowed(
        &bash("git commit -am 'fix the parser that reads klin.json'"),
        "a message",
    );
}

#[test]
fn asks_about_an_interpreter_a_reader_reaches_through_a_command_substitution() {
    for command in [
        "cat \"$(python3 -c \"open('klin.json','w')\")\"",
        "wc -l `perl -i -pe 's/a/b/' klin.json`",
    ] {
        asked(&bash(command), "klin.json", command);
    }
}

#[test]
fn asks_about_a_glob_that_matches_a_guarded_name() {
    for (command, quoted) in [
        ("perl -i -pe 's/a/b/' klin.*", "klin.*"),
        ("rm klin.js*n", "klin.js*n"),
        ("rm *.json", "*.json"),
        ("rm CODEOWNER*", "CODEOWNER*"),
        ("rm ?lin.json", "?lin.json"),
        ("rm klin.jso[n]", "klin.jso[n]"),
        ("rm C?DEOWNERS", "C?DEOWNERS"),
    ] {
        asked(&bash(command), quoted, command);
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
        allowed(&bash(command), command);
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
        allowed(&bash(command), command);
    }
}

/// The quote-blind split tears `git` off the front and loses the restore, so only the pass that
/// honours the quoting sees the whole command.
#[test]
fn refuses_a_whole_tree_restore_whose_argument_quotes_a_separator() {
    for command in [r#"git checkout "a|b" ."#, "git restore 'a;b' ."] {
        denied(&bash(command), command);
    }
}

/// An unbalanced quote says nothing about where an argument ends, so the guard falls back to
/// splitting on every separator rather than trusting the quote.
#[test]
fn asks_about_a_left_open_quote_that_hides_a_separator() {
    for command in [
        "cat \"unclosed | rm klin.json",
        "cat 'unclosed ; rm CODEOWNERS",
    ] {
        let run = bash(command);
        assert_eq!(run.code, 0, "{command}: {}", run.out);
        assert!(run.says(ASK), "{command}: {}", run.out);
    }
}

#[test]
fn allows_a_reader_that_carries_a_global_git_flag() {
    for command in [
        "git -C sub add klin.json",
        "git -c core.pager=cat log klin.json",
        "git --git-dir=/repo/.git diff klin.json",
    ] {
        allowed(&bash(command), command);
    }
}

#[test]
fn allows_a_line_continued_reader_command_that_names_the_config() {
    allowed(
        &bash("git commit \\\n  -m 'mentions klin.json' \\\n  -m 'a second paragraph'"),
        "a continued command",
    );
}

/// A heredoc body is data, and a redirect beside it is not.
#[test]
fn a_heredoc_body_is_data_and_the_redirect_beside_it_is_not() {
    for command in [
        "cat <<'EOF' > /tmp/notes\nsee klin.json for details\nEOF",
        "cat > /tmp/notes <<'EOF'\nsee klin.json for details\nEOF",
        "python3 - <<'EOF'\nopen('klin.json', 'w').write('{}')\nEOF",
        "cat <<-EOF > /tmp/notes\n\tklin.json\n\tEOF",
    ] {
        allowed(&bash(command), command);
    }
    denied(
        &bash("cat <<'EOF' > klin.json\nnothing to see\nEOF"),
        "a redirect beside a heredoc",
    );
    asked(
        &bash("cat <<'EOF' > /tmp/notes\nklin.json\nEOF\nrm klin.json"),
        "klin.json",
        "a command after the terminator",
    );
    asked(
        &bash("echo 'a<<b'\nrm klin.json"),
        "klin.json",
        "a `<<` that opens no body",
    );
    asked(
        &bash("echo \"a << EOF\"\nrm CODEOWNERS\nEOF"),
        "CODEOWNERS",
        "a quoted `<<`",
    );
}

/// `find` writes with more primaries than it deletes with, and each one takes it off the
/// reader list.
#[test]
fn asks_about_find_writing_a_guarded_file() {
    for (command, quoted) in [
        ("find . -fprint klin.json", "klin.json"),
        ("find . -name x -fprintf CODEOWNERS '%p'", "CODEOWNERS"),
        ("find . -fls .git/klin/listing", ".git/klin/listing"),
    ] {
        asked(&bash(command), quoted, command);
    }
}

/// A person's command is the same command behind an assignment or a runner.
#[test]
fn refuses_a_persons_command_behind_a_prefix() {
    for command in [
        "KLIN_STATE_DIR=/tmp/x klin init --add",
        "env klin turn reset",
        "npx klin init",
        "sudo klin init",
    ] {
        denied(&bash(command), command);
    }
}

#[test]
fn asks_about_the_state_directory_of_a_linked_worktree() {
    let command = "rm -rf .git/worktrees/wt1/klin";
    asked(&bash(command), ".git/worktrees/wt1/klin", command);
}

/// The override names a directory, and a path is under it or is not. An empty value names no
/// directory, so it must not turn every path into klin's own state.
#[test]
fn an_empty_state_directory_override_guards_nothing() {
    let tree = Tree::new();
    let event = r#"{"tool_name": "Edit", "tool_input": {"file_path": "src/main.rs"}}"#;
    for value in ["", ".", "state"] {
        let run = harness::feed_with(tree.root(), &[("KLIN_STATE_DIR", value)], &["guard"], event);
        allowed(&run, value);
    }

    let under = r#"{"tool_name": "Edit", "tool_input": {"file_path": "state/a1b2/turn"}}"#;
    let run = harness::feed_with(
        tree.root(),
        &[("KLIN_STATE_DIR", "state")],
        &["guard"],
        under,
    );
    asked(&run, "state/a1b2/turn", "a path under the override");
}

#[test]
fn allows_a_tool_call_that_leaves_the_guarded_files_alone() {
    for (name, file) in [
        ("Edit", "/repo/src/klin_of_life.rs"),
        ("Write", "/repo/docs/klin.md"),
        ("Read", "/repo/klin.json"),
    ] {
        allowed(&edit(name, file), &format!("{name} {file}"));
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

    let run = feed(
        tree.root(),
        &["guard"],
        r#"{"tool_name": "Edit", "tool_input": {"file_path": "src/main.rs"}}"#,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{}", run.out);

    let refused = feed(
        tree.root(),
        &["guard"],
        r#"{"tool_name": "Edit", "tool_input": {"file_path": "klin.json"}}"#,
    );
    assert_eq!(refused.code, 2, "{}", refused.out);
    assert!(refused.says("refused"), "{}", refused.out);
    assert!(!refused.says("could not be read"), "{}", refused.out);
}
