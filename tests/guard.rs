mod harness;

use harness::{Run, Tree, feed, feed_with};

const ASK: &str = r#""permissionDecision":"ask""#;

/// A pre-tool event through the ingress, in a tree that opted in: the guard answers nothing in
/// a tree whose worktree root holds no `klin.json`. Spec 5.1, 10.8.
fn guard(tree: &Tree, event: &str) -> Run {
    if !tree.path("klin.json").exists() {
        tree.write("klin.json", "{}\n");
    }
    feed(tree.root(), harness::AGENT, &pre_tool(event))
}

/// The event as Claude Code sends it before a tool runs. Text that is no JSON object is sent as
/// it is.
fn pre_tool(event: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(event) {
        Ok(serde_json::Value::Object(mut held)) => {
            held.insert("hook_event_name".into(), "PreToolUse".into());
            serde_json::Value::Object(held).to_string()
        }
        _ => event.to_string(),
    }
}

fn bash_in(tree: &Tree, command: &str) -> Run {
    guard(
        tree,
        &format!(r#"{{"tool_name": "Bash", "tool_input": {{"command": {command:?}}}}}"#),
    )
}

fn bash(command: &str) -> Run {
    bash_in(&Tree::new(), command)
}

fn tool_in(tree: &Tree, name: &str, key: &str, file: &str) -> Run {
    guard(
        tree,
        &format!(r#"{{"tool_name": {name:?}, "tool_input": {{{key:?}: {file:?}}}}}"#),
    )
}

fn edit_in(tree: &Tree, name: &str, file: &str) -> Run {
    tool_in(tree, name, "file_path", file)
}

fn edit(name: &str, file: &str) -> Run {
    edit_in(&Tree::new(), name, file)
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
    let tree = Tree::new();
    for (name, file) in [
        ("Edit", tree.at("klin.json")),
        ("Write", "klin.json".into()),
    ] {
        denied(&edit_in(&tree, name, &file), &format!("{name} {file}"));
    }
    let run = tool_in(&tree, "NotebookEdit", "notebook_path", "klin.json");
    assert_eq!(run.code, 2, "{}", run.out);
}

/// The guard resolves the path it is given, so a configuration of another project inside the
/// tree is an ordinary file. ADR 0033.
#[test]
fn allows_an_edit_of_another_projects_configuration() {
    let tree = Tree::new();
    for file in [
        "sub/klin.json",
        "vendor/a/klin.json",
        "/elsewhere/klin.json",
    ] {
        allowed(&edit_in(&tree, "Write", file), file);
        let command = format!("rm {file}");
        allowed(&bash_in(&tree, &command), &command);
    }
}

#[test]
fn refuses_a_redirect_onto_the_configuration() {
    for command in [
        "echo '{}' > klin.json",
        "echo '[]' >>klin.json",
        "awk '{ print }' notes.txt > klin.json",
        "echo '{}' > ./klin.json",
    ] {
        denied(&bash(command), command);
    }
}

#[test]
fn refuses_the_commands_only_a_person_runs() {
    for command in [
        "klin setup",
        "klin setup --pin",
        "cd repo && target/debug/klin setup --pin --config klin.json",
        "klin turn reset",
        "target/debug/klin turn reset",
        "klin setup --user --host claude",
        "klin update",
        "env klin update",
        "klin __agent event",
        "echo '{}' | klin __agent event --host claude",
        "target/debug/klin __agent",
    ] {
        denied(&bash(command), command);
    }
}

/// A deny names the person who runs the command and never a command that accepts debt.
/// Spec 10.8.
#[test]
fn the_refusal_of_update_and_the_ingress_names_who_runs_them() {
    let update = bash("klin update");
    assert!(update.says("Only a person runs it"), "{}", update.out);
    let ingress = bash("klin __agent event");
    assert!(ingress.says("Only the host runs it"), "{}", ingress.out);
}

#[test]
fn the_refusal_names_what_it_protects() {
    let run = edit("Edit", "klin.json");
    assert!(run.says("klin.json"), "{}", run.out);
    assert!(run.says("reviewed commit"), "{}", run.out);
    assert!(!run.says("hooks"), "{}", run.out);
    assert!(!run.says("baseline"), "{}", run.out);
}

/// Every argument of one of these is written, so a guarded path among them is a write klin can
/// prove. ADR 0033.
#[test]
fn asks_about_a_command_that_writes_every_argument_it_takes() {
    let tree = Tree::new();
    for command in [
        "rm klin.json",
        "rmdir klin.json",
        "unlink klin.json",
        "shred klin.json",
        "mv klin.json klin.json.bak",
        "truncate -s 0 klin.json",
        "tee klin.json < /tmp/loose.json",
        "sed -i '' 's/8/80/' klin.json",
        "perl -i -pe 's/a/b/' klin.json",
    ] {
        asked(&bash_in(&tree, command), "klin.json", command);
    }
    asked(
        &bash_in(&tree, &format!("rm {}", tree.at("klin.json"))),
        &tree.at("klin.json"),
        "an absolute path",
    );
    asked(&bash_in(&tree, "rm -f ./klin.json"), "./klin.json", "a dot");
}

/// `cp` and `install` read their first argument, so a backup of klin's own state through one of
/// them is not a write. ADR 0033.
#[test]
fn allows_a_command_that_reads_the_argument_it_is_given() {
    for command in [
        "cp .git/klin/turn /tmp/backup",
        "cp /tmp/loose.json klin.json",
        "install -m 644 /tmp/loose.json klin.json",
        "ed klin.json",
        "patch -p1 klin.json < fix.diff",
        "git apply klin.json",
        "git stash pop klin.json",
        "git checkout -- klin.json",
        "git restore --source=HEAD~1 klin.json",
        "find . -delete",
        "sed -n 2p klin.json",
        "perl -pe 's/a/b/' klin.json",
    ] {
        allowed(&bash(command), command);
    }
}

/// The guard answers only where it can prove a write, so a command that merely names the
/// configuration is ordinary work. ADR 0033.
#[test]
fn allows_a_command_that_only_names_the_configuration() {
    for command in [
        "echo klin.json",
        "gh issue create --title 'the klin.json shape' --body 'see klin.json'",
        "cat klin.json",
        "cat klin.json > /tmp/copy.json",
        "grep klin.json src/",
        "python3 x.py klin.json",
        "git add klin.json",
        "git commit klin.json -m 'wip'",
        "git commit -am 'fix the parser that reads klin.json'",
        "cargo test > /tmp/out.txt",
        "klin doc-size --quiet",
        "klin escapes --write-baseline",
    ] {
        allowed(&bash(command), command);
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
        "git update-ref -d refs/worktree/klin/turn",
    ] {
        allowed(&bash(command), command);
    }
}

/// A token holding a wildcard proves nothing about the file it expands to, so the guard
/// matches no path against it. ADR 0033.
#[test]
fn allows_a_token_that_holds_a_wildcard() {
    for command in [
        "rm *.json",
        "rm klin.js*n",
        "rm ?lin.json",
        "rm klin.jso[n]",
        "rm *",
        "echo '{}' > *",
        "ffmpeg ** out",
        "find src tests -name '*.rs' | sort",
    ] {
        allowed(&bash(command), command);
    }
}

/// Shell the guard cannot read in full leaves every path in the command unproven. ADR 0033.
#[test]
fn allows_a_command_it_cannot_read_in_full() {
    for command in [
        "cat \"unclosed | rm klin.json",
        "cat 'unclosed ; rm klin.json",
        "rm $(echo klin).json",
        "rm `echo klin.json`",
        "rm ${CONFIG}",
        "rm klin.json\\",
        "cat <<'EOF' > klin.json\nnothing to see\nEOF",
        "cd /elsewhere && rm klin.json",
        "cd sub; rm klin.json",
    ] {
        allowed(&bash(command), command);
    }
}

/// A `cd` and a command substitution suppress path matching alone. klin's own subcommands name
/// no path, so their refusal stands behind either. ADR 0033.
#[test]
fn refuses_a_persons_command_behind_a_prefix() {
    for command in [
        "env $(echo) klin setup",
        "sudo $(pwd) klin setup",
        "time `echo` klin turn reset",
        "KLIN_STATE_DIR=/tmp/x klin setup --pin",
        "env klin turn reset",
        "npx klin setup",
        "sudo klin setup",
        "cd sub && klin turn reset",
    ] {
        denied(&bash(command), command);
    }
}

#[test]
fn allows_a_tool_call_that_leaves_the_guarded_files_alone() {
    for (name, file) in [
        ("Edit", "src/klin_of_life.rs"),
        ("Write", "docs/klin.md"),
        ("Read", "klin.json"),
    ] {
        allowed(&edit(name, file), &format!("{name} {file}"));
    }
}

#[test]
fn an_event_it_cannot_read_is_allowed_through() {
    let tree = Tree::new();
    for event in [
        "not json",
        "",
        "{}",
        r#"{"tool_name": "Bash"}"#,
        r#"{"tool_name": "Bash", "tool_input": {"command": null}}"#,
        r#"{"tool_input": {"pattern": "klin.json"}}"#,
    ] {
        let run = guard(&tree, event);
        assert_eq!(run.code, 0, "{event}: {}", run.out);
    }
}

#[test]
fn it_never_reads_the_configuration() {
    let tree = Tree::new();
    tree.write("klin.json", "{not json at all");

    let run = edit_in(&tree, "Edit", "src/main.rs");
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{}", run.out);

    let refused = edit_in(&tree, "Edit", "klin.json");
    assert_eq!(refused.code, 2, "{}", refused.out);
    assert!(refused.says("refused"), "{}", refused.out);
    assert!(!refused.says("could not be read"), "{}", refused.out);
}

/// klin's own state takes the three decisions the configuration takes. ADR 0032, ADR 0033.
#[test]
fn a_command_that_writes_klins_own_state_is_denied_or_asked_about() {
    let tree = Tree::new();
    denied(
        &bash_in(&tree, "echo x > .git/klin/turn"),
        "a redirect onto the stamp",
    );
    for (command, named) in [
        ("rm -rf .git/klin", ".git/klin"),
        ("rm .git/klin/turn", ".git/klin/turn"),
        ("sed -i s/red/green/ .git/klin/turn", ".git/klin/turn"),
    ] {
        asked(&bash_in(&tree, command), named, command);
    }
}

#[test]
fn an_edit_to_klins_own_state_is_refused_and_names_the_command_a_person_runs() {
    let tree = Tree::new();
    denied(
        &edit_in(&tree, "Write", ".git/klin/turn"),
        "a stamp written by hand",
    );
    denied(
        &edit_in(&tree, "Edit", &tree.at(".git/klin/build-blocked")),
        "an absolute state path",
    );
    assert!(
        edit_in(&tree, "Write", ".git/klin/turn").says("klin turn reset"),
        "the refusal names no command a person runs"
    );
}

#[test]
fn a_reader_of_klins_own_state_is_allowed() {
    let tree = Tree::new();
    allowed(
        &bash_in(&tree, "cat .git/klin/turn"),
        "a reader of the stamp",
    );
    allowed(
        &edit_in(&tree, "Read", ".git/klin/turn"),
        "the read tool on the stamp",
    );
}

/// The guard resolves this tree's own state directory, so a path that only looks like one is an
/// ordinary file. ADR 0033.
#[test]
fn a_path_that_only_looks_like_klins_state_is_allowed() {
    let tree = Tree::new();
    for file in [
        ".github/workflows/klin.yml",
        "vendor/klin/notes.md",
        ".git/COMMIT_EDITMSG",
        ".git/worktrees/second/klin/turn",
    ] {
        allowed(&edit_in(&tree, "Write", file), file);
    }
}

/// A hook file, the code owners, a lint, test or coverage configuration and a workflow are
/// ordinary files. ADR 0027.
#[test]
fn allows_an_edit_of_everything_the_configuration_is_not() {
    let tree = Tree::new();
    for file in [
        ".claude/settings.json",
        ".cursor/hooks.json",
        ".codex/config.toml",
        "CODEOWNERS",
        ".github/CODEOWNERS",
        ".eslintrc.json",
        "pytest.ini",
        "codecov.yml",
        ".github/workflows/ci.yml",
        "pyproject.toml",
        "quality/escapes-baseline.json",
    ] {
        allowed(&edit_in(&tree, "Write", file), file);
        let command = format!("rm {file}");
        allowed(&bash_in(&tree, &command), &command);
    }
}

/// Outside a git repository the opt-in walk finds no worktree root, so the guard answers
/// nothing, about the state directory or a configuration. Spec 5.1.
#[test]
fn outside_a_repository_it_answers_nothing() {
    let tree = Tree::bare();
    for command in ["rm -rf .git/klin", "echo x > .git/klin/turn"] {
        allowed(&bash_in(&tree, command), command);
    }
    allowed(
        &edit_in(&tree, "Write", ".git/klin/turn"),
        "a stamp by hand",
    );
    allowed(&edit_in(&tree, "Write", "klin.json"), "the configuration");
}

/// klin's own state moves with `KLIN_STATE_DIR`, and the guard answers about where the state
/// is rather than about the name `.git/klin`. Section 9.4.
#[test]
fn it_guards_where_the_state_is_and_not_the_name() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    let held = Tree::bare();
    let under = held.root().display().to_string();
    let run = feed_with(
        tree.root(),
        &[("KLIN_STATE_DIR", under.as_str())],
        harness::AGENT,
        &pre_tool(r#"{"tool_name": "Bash", "tool_input": {"command": "rm -rf .git/klin"}}"#),
    );
    allowed(&run, "the default place the override left empty");
}

/// A relative path in a command names a file from where the guard runs, and the guarded set is
/// the tree root's. So the same word means a different file one directory down. ADR 0033.
#[test]
fn a_relative_path_names_a_file_from_where_the_guard_runs() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("sub/notes.md", "one\n");
    let event = pre_tool(r#"{"tool_name": "Bash", "tool_input": {"command": "rm klin.json"}}"#);
    let up = pre_tool(r#"{"tool_name": "Bash", "tool_input": {"command": "rm ../klin.json"}}"#);

    allowed(
        &feed(&tree.path("sub"), harness::AGENT, &event),
        "another project's configuration one directory down",
    );
    asked(
        &feed(&tree.path("sub"), harness::AGENT, &up),
        "../klin.json",
        "the tree's own configuration from below",
    );
}

/// A `>` inside an argument is a character of that argument, not a redirect, so a message that
/// holds one is ordinary work. ADR 0033.
#[test]
fn allows_a_quoted_angle_bracket_that_is_not_a_redirect() {
    for command in [
        r#"git commit -m "moved notes > klin.json""#,
        r#"gh issue create --body "notes > klin.json""#,
        "echo 'a > klin.json'",
    ] {
        allowed(&bash(command), command);
    }
}

/// A writer is found behind the prefixes klin's own name is found behind, so a runner in front
/// of it does not carry the write past the guard. ADR 0033.
#[test]
fn asks_about_a_writer_behind_a_prefix() {
    let tree = Tree::new();
    for command in [
        "sudo rm klin.json",
        "env rm -f klin.json",
        "time rm klin.json",
        "KLIN_STATE_DIR=/tmp/x rm klin.json",
    ] {
        asked(&bash_in(&tree, command), "klin.json", command);
    }
}
