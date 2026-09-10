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
fn refuses_an_edit_of_the_configuration() {
    for (name, file) in [("Edit", "/repo/klin.json"), ("Write", "klin.json")] {
        denied(&edit(name, file), &format!("{name} {file}"));
    }
    let run = tool("NotebookEdit", "notebook_path", "/repo/klin.json");
    assert_eq!(run.code, 2, "{}", run.out);
}

#[test]
fn refuses_a_redirect_onto_the_configuration() {
    for command in [
        "echo '{}' > klin.json",
        "echo '[]' >>klin.json",
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
fn the_refusal_names_what_it_protects() {
    let run = edit("Edit", "/repo/klin.json");
    assert!(run.says("klin.json"), "{}", run.out);
    assert!(run.says("reviewed commit"), "{}", run.out);
    assert!(!run.says("hooks"), "{}", run.out);
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
        ("tee klin.json < /tmp/loose.json", "klin.json"),
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
        ("rm ?lin.json", "?lin.json"),
        ("rm klin.jso[n]", "klin.jso[n]"),
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
        "cat 'unclosed ; rm klin.json",
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
        &bash("echo \"a << EOF\"\nrm klin.json\nEOF"),
        "klin.json",
        "a quoted `<<`",
    );
}

/// `find` writes with more primaries than it deletes with, and each one takes it off the
/// reader list.
#[test]
fn asks_about_find_writing_a_guarded_file() {
    for (command, quoted) in [
        ("find . -fprint klin.json", "klin.json"),
        ("find . -name x -fprintf klin.json '%p'", "klin.json"),
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

/// Only the configuration is guarded. A hook file, the code owners, a lint, test or coverage
/// configuration, a workflow, and klin's own state are ordinary files.
#[test]
fn allows_an_edit_of_everything_the_configuration_is_not() {
    for file in [
        ".claude/settings.json",
        "/Users/someone/.claude/settings.json",
        ".cursor/hooks.json",
        ".codex/config.toml",
        "CODEOWNERS",
        ".github/CODEOWNERS",
        ".eslintrc.json",
        "eslint.config.mjs",
        "pytest.ini",
        "jest.config.ts",
        ".coveragerc",
        "codecov.yml",
        ".github/workflows/ci.yml",
        ".git/klin/turn",
        "pyproject.toml",
    ] {
        allowed(&edit("Write", file), file);
        let command = format!("rm {file}");
        allowed(&bash(&command), &command);
    }
}

/// klin's own state carries a stamp a report restores, so a write to it is nobody's question.
#[test]
fn allows_a_command_that_writes_klins_own_state() {
    for command in [
        "echo x > .git/klin/turn",
        "rm -rf .git/klin",
        "git update-ref -d refs/worktree/klin/turn",
        "find .git/klin -delete",
        "rm -rf .git/worktrees/wt1/klin",
    ] {
        allowed(&bash(command), command);
    }
}

/// A command substitution starts a fresh quoting context, so the `<<` inside one opens a
/// heredoc even though a double quote wraps it. Without this the body is read as commands.
#[test]
fn a_heredoc_that_opens_inside_a_command_substitution_still_holds_data() {
    for command in [
        "gh issue create --body \"$(cat <<'EOF'\nsee klin.json for the shape\nEOF\n)\"",
        "gh issue create --body \"`cat <<'EOF'\nsee klin.json for the shape\nEOF\n`\"",
    ] {
        allowed(&bash(command), command);
    }
    asked(
        &bash("echo 'a $(cat <<EOF'\nrm klin.json\nEOF"),
        "klin.json",
        "a single-quoted substitution opens no body",
    );
    asked(
        &bash("echo \"$(date) << EOF\"\nrm klin.json\nEOF"),
        "klin.json",
        "a `<<` back inside the double quote the substitution closed",
    );
    asked(
        &bash("gh issue create klin.json --body \"$(cat <<'EOF'\nnotes\nEOF\n)\""),
        "klin.json",
        "a guarded name in the command words beside the heredoc",
    );
}
