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

/// The configuration is the agent's to edit, by a tool or by the shell. Spec 10.8.
#[test]
fn an_agent_may_edit_the_configuration() {
    let tree = Tree::new();
    for (name, file) in [
        ("Edit", tree.at("klin.json")),
        ("Write", "klin.json".into()),
    ] {
        let run = edit_in(&tree, name, &file);
        assert_eq!(run.out, "", "{name} {file}: {}", run.out);
        allowed(&run, &format!("{name} {file}"));
    }
    for command in [
        "echo '{}' > klin.json",
        "tee klin.json",
        "cp a klin.json",
        "sed -i s/a/b/ klin.json",
    ] {
        let run = bash_in(&tree, command);
        assert_eq!(run.out, "", "{command}: {}", run.out);
        allowed(&run, command);
    }
}

/// Without `klin.json` klin answers nothing, so a command that deletes or moves it is put to
/// the person. Another project's `klin.json` is an ordinary file. ADR 0074.
#[test]
fn asks_before_a_command_deletes_or_moves_the_configuration() {
    let tree = Tree::new();
    for (command, quoted) in [
        ("rm klin.json", "klin.json"),
        ("rm -f ./klin.json", "./klin.json"),
        ("unlink klin.json", "klin.json"),
        ("shred -u klin.json", "klin.json"),
        ("truncate -s 0 klin.json", "klin.json"),
        ("mv klin.json old.json", "klin.json"),
        ("mv -f klin.json old/", "klin.json"),
        ("mv -t old klin.json", "klin.json"),
        ("mv -h klin.json old.json", "klin.json"),
        ("mv --no-copy klin.json old.json", "klin.json"),
        ("sudo rm klin.json", "klin.json"),
    ] {
        asked(&bash_in(&tree, command), quoted, command);
    }
    for command in [
        "mv new.json klin.json",
        "mv -f new.json klin.json",
        "mv -h new.json klin.json",
        "mv --no-copy new.json klin.json",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
    tree.write("sub/klin.json", "{}\n");
    allowed(&bash_in(&tree, "rm sub/klin.json"), "rm sub/klin.json");
}

/// `rm` and `mv` act on a link itself. Removing a link to `klin.json` leaves klin on, and
/// removing a `klin.json` that is a link turns it off. ADR 0074.
#[test]
fn a_link_to_the_configuration_is_its_own_file() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    std::os::unix::fs::symlink(tree.path("klin.json"), tree.path("alias.json")).unwrap();
    std::fs::hard_link(tree.path("klin.json"), tree.path("hard.json")).unwrap();
    for command in [
        "rm alias.json",
        "mv alias.json backup.json",
        "rm hard.json",
        "mv hard.json backup.json",
    ] {
        allowed(&bash_in(&tree, command), command);
    }

    tree.write("inner/notes.md", "one\n");
    tree.write("sub/notes.md", "one\n");
    std::os::unix::fs::symlink("../inner", tree.path("sub/alias")).unwrap();
    for command in [
        "rm sub/alias/../klin.json",
        "mv sub/alias/../klin.json backup.json",
    ] {
        asked(&bash_in(&tree, command), "sub/alias/../klin.json", command);
    }

    let linked = Tree::new();
    linked.write("real.json", "{}\n");
    std::os::unix::fs::symlink(linked.path("real.json"), linked.path("klin.json")).unwrap();
    asked(
        &bash_in(&linked, "rm klin.json"),
        "klin.json",
        "rm a linked klin.json",
    );
}

/// A case-insensitive file system removes `klin.json` under any spelling of its name. ADR 0074.
#[test]
fn asks_before_a_command_removes_the_configuration_under_another_case() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    if !tree.path("KLIN.json").exists() {
        return;
    }
    for command in ["rm KLIN.json", "mv Klin.json old.json"] {
        asked(
            &bash_in(&tree, command),
            command.split(' ').nth(1).unwrap(),
            command,
        );
    }
}

#[test]
fn refuses_an_edit_of_klins_own_state() {
    let tree = Tree::new();
    for (name, file) in [
        ("Edit", tree.at(".git/klin/turn")),
        ("Write", ".git/klin/turn".into()),
    ] {
        denied(&edit_in(&tree, name, &file), &format!("{name} {file}"));
    }
    let run = tool_in(&tree, "NotebookEdit", "notebook_path", ".git/klin/turn");
    assert_eq!(run.code, 2, "{}", run.out);
}

#[test]
fn refuses_a_redirect_onto_klins_own_state() {
    for command in [
        "echo '{}' > .git/klin/turn",
        "echo '[]' >>.git/klin/turn",
        "awk '{ print }' notes.txt > .git/klin/turn",
        "echo '{}' > ./.git/klin/turn",
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
        "klin setup --user --host claude",
        "klin update",
        "env klin update",
        "KLIN_AUTO_UPDATE=1 klin __release update",
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
    let run = edit("Edit", ".git/klin/turn");
    assert!(run.says(".git/klin"), "{}", run.out);
    assert!(run.says("own record of the turn"), "{}", run.out);
    assert!(!run.says("hooks"), "{}", run.out);
    assert!(!run.says("baseline"), "{}", run.out);
}

/// Every argument of one of these is written, so a guarded path among them is a write klin can
/// prove. ADR 0033.
#[test]
fn asks_about_a_command_that_writes_every_argument_it_takes() {
    let tree = Tree::new();
    for command in [
        "rm .git/klin/turn",
        "rmdir .git/klin/turn",
        "unlink .git/klin/turn",
        "shred .git/klin/turn",
        "mv .git/klin/turn .git/klin/turn.bak",
        "truncate -s 0 .git/klin/turn",
        "tee .git/klin/turn < /tmp/loose.json",
        "sed -i '' 's/8/80/' .git/klin/turn",
        "perl -i -pe 's/a/b/' .git/klin/turn",
    ] {
        asked(&bash_in(&tree, command), ".git/klin/turn", command);
    }
    asked(
        &bash_in(&tree, &format!("rm {}", tree.at(".git/klin/turn"))),
        &tree.at(".git/klin/turn"),
        "an absolute path",
    );
    asked(
        &bash_in(&tree, "rm -f ./.git/klin/turn"),
        "./.git/klin/turn",
        "a dot",
    );
}

/// `cp` and `install` read their sources, so a backup of klin's own state through one of them is
/// not a write. ADR 0033, ADR 0068.
#[test]
fn allows_a_command_that_reads_the_argument_it_is_given() {
    for command in [
        "cp .git/klin/turn /tmp/backup",
        "install -m 644 .git/klin/turn /tmp/copy.json",
        "ed .git/klin/turn",
        "patch -p1 .git/klin/turn < fix.diff",
        "git apply .git/klin/turn",
        "git stash pop .git/klin/turn",
        "git checkout -- .git/klin/turn",
        "git restore --source=HEAD~1 .git/klin/turn",
        "find . -delete",
        "sed -n 2p .git/klin/turn",
        "perl -pe 's/a/b/' .git/klin/turn",
    ] {
        allowed(&bash(command), command);
    }
}

/// The guard answers only where it can prove a write, so a command that merely names the
/// state is ordinary work. ADR 0033.
#[test]
fn allows_a_command_that_only_names_klins_state() {
    for command in [
        "echo .git/klin/turn",
        "gh issue create --title 'the .git/klin/turn shape' --body 'see .git/klin/turn'",
        "cat .git/klin/turn",
        "cat .git/klin/turn > /tmp/copy.json",
        "grep .git/klin/turn src/",
        "python3 x.py .git/klin/turn",
        "git add .git/klin/turn",
        "git commit .git/klin/turn -m 'wip'",
        "git commit -am 'fix the parser that reads .git/klin/turn'",
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
        "rm .git/klin/tur*",
        "rm .git/klin/?urn",
        "rm .git/klin/tur[n]",
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
        "cat \"unclosed | rm .git/klin/turn",
        "cat 'unclosed ; rm .git/klin/turn",
        "rm .git/$(echo klin)/turn",
        "rm `echo .git/klin/turn`",
        "rm ${CONFIG}",
        "rm .git/klin/turn\\",
        "cat <<< x > .git/klin/turn",
        "cat <<'EOF' > .git/klin/turn\nnever closed",
        "cd /elsewhere && rm .git/klin/turn",
        "cd sub; rm .git/klin/turn",
        "rm $DIR/../.git/klin/turn",
        "sh -c \"$CMD\"",
        "eval \"$CMD\"",
        "sh -c 'rm .git/$(echo klin)/turn'",
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
        "time `echo` klin update",
        "KLIN_STATE_DIR=/tmp/x klin setup --pin",
        "env klin setup --pin",
        "npx klin setup",
        "sudo klin setup",
        "cd sub && klin update",
    ] {
        denied(&bash(command), command);
    }
}

#[test]
fn allows_a_tool_call_that_leaves_the_guarded_files_alone() {
    for (name, file) in [
        ("Edit", "src/klin_of_life.rs"),
        ("Write", "docs/klin.md"),
        ("Read", ".git/klin/turn"),
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
        r#"{"tool_input": {"pattern": ".git/klin/turn"}}"#,
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

    let refused = edit_in(&tree, "Edit", ".git/klin/turn");
    assert_eq!(refused.code, 2, "{}", refused.out);
    assert!(refused.says("refused"), "{}", refused.out);
    assert!(!refused.says("could not be read"), "{}", refused.out);
}

/// A redirect onto klin's own state is a deny and a proven writer of it an ask. ADR 0032, ADR
/// 0033.
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
fn an_edit_to_klins_own_state_is_refused_and_names_no_command() {
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
        edit_in(&tree, "Write", ".git/klin/turn").says("No command moves it"),
        "the refusal names a command that moves the stamp"
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
        "sub/.git/klin/turn",
        "/elsewhere/.git/klin/turn",
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
/// nothing about the state directory. Spec 5.1.
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
    let event =
        pre_tool(r#"{"tool_name": "Bash", "tool_input": {"command": "rm .git/klin/turn"}}"#);
    let up =
        pre_tool(r#"{"tool_name": "Bash", "tool_input": {"command": "rm ../.git/klin/turn"}}"#);

    allowed(
        &feed(&tree.path("sub"), harness::AGENT, &event),
        "another project's state one directory down",
    );
    asked(
        &feed(&tree.path("sub"), harness::AGENT, &up),
        "../.git/klin/turn",
        "the tree's own state from below",
    );
}

/// A `>` inside an argument is a character of that argument, not a redirect, so a message that
/// holds one is ordinary work. ADR 0033.
#[test]
fn allows_a_quoted_angle_bracket_that_is_not_a_redirect() {
    for command in [
        r#"git commit -m "moved notes > .git/klin/turn""#,
        r#"gh issue create --body "notes > .git/klin/turn""#,
        "echo 'a > .git/klin/turn'",
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
        "sudo rm .git/klin/turn",
        "env rm -f .git/klin/turn",
        "time rm .git/klin/turn",
        "KLIN_STATE_DIR=/tmp/x rm .git/klin/turn",
    ] {
        asked(&bash_in(&tree, command), ".git/klin/turn", command);
    }
}

/// `cp` and `install` write their destination, and a destination that is a directory takes each
/// source's name. ADR 0068.
#[test]
fn asks_about_the_destination_of_a_copy() {
    let tree = Tree::new();
    for (command, named) in [
        ("cp source .git/klin/turn", ".git/klin/turn"),
        ("cp -p source ./.git/klin/turn", "./.git/klin/turn"),
        ("install source .git/klin/turn", ".git/klin/turn"),
        (
            "install -m 644 /tmp/loose.json .git/klin/turn",
            ".git/klin/turn",
        ),
        (
            "install -pm644 /tmp/loose.json .git/klin/turn",
            ".git/klin/turn",
        ),
        ("cp --suffix .bak source .git/klin/turn", ".git/klin/turn"),
        ("cp /tmp/turn .git/klin", ".git/klin/turn"),
        ("cp -t .git/klin /tmp/turn", ".git/klin/turn"),
        (
            "cp --target-directory=.git/klin notes.md",
            ".git/klin/notes.md",
        ),
        ("install -d .git/klin/extra", ".git/klin/extra"),
        ("cp -n source .git/klin/new", ".git/klin/new"),
        ("cp --update source .git/klin/new", ".git/klin/new"),
    ] {
        asked(&bash_in(&tree, command), named, command);
    }
    let absolute = format!("cp source {}", tree.at(".git/klin/turn"));
    asked(
        &bash_in(&tree, &absolute),
        &tree.at(".git/klin/turn"),
        "an absolute destination",
    );
}

/// An option the guard cannot read leaves the destination unproven, so the guard does not guess.
/// ADR 0068.
#[test]
fn allows_a_copy_whose_destination_it_cannot_place() {
    for command in [
        "install -S .bak source .git/klin/turn",
        "install -T source .git/klin/turn",
        "cp --target=.git/klin /tmp/turn",
        "cp -Q source .git/klin/turn",
        "cp -T /tmp/klin .git",
    ] {
        allowed(&bash(command), command);
    }
}

/// A `cd` the guard can place moves the directory every later relative path resolves from. ADR
/// 0068.
#[test]
fn a_static_cd_moves_where_a_relative_path_resolves() {
    let tree = Tree::new();
    tree.write("sub/notes.md", "one\n");
    for (command, named) in [
        (
            "cd sub && sed -i s/a/b/ ../.git/klin/turn",
            "../.git/klin/turn",
        ),
        ("cd sub\nrm ../.git/klin/turn", "../.git/klin/turn"),
        ("cd -P sub && cd .. && rm .git/klin/turn", ".git/klin/turn"),
        (
            "cd sub && make | tee ../.git/klin/turn",
            "../.git/klin/turn",
        ),
    ] {
        asked(&bash_in(&tree, command), named, command);
    }
    for command in [
        "cd sub; echo x > ../.git/klin/turn",
        "cd sub && echo x > ../.git/klin/turn",
    ] {
        denied(&bash_in(&tree, command), command);
    }
    let absolute = format!("cd {} && rm ../.git/klin/turn", tree.at("sub"));
    asked(
        &bash_in(&tree, &absolute),
        "../.git/klin/turn",
        "an absolute cd",
    );
    for command in [
        "cd sub && rm .git/klin/turn",
        "cd \"$DIR\" && rm ../.git/klin/turn",
        "cd $DIR; echo x > ../.git/klin/turn",
        "cd ~ && rm ../.git/klin/turn",
        "cd - && rm ../.git/klin/turn",
        "cd sub || exit 1; rm ../.git/klin/turn",
        "cd sub && make || rm ../.git/klin/turn",
        "cd sub & rm ../.git/klin/turn",
        "echo | cd sub && rm ../.git/klin/turn",
        "(cd sub); rm ../.git/klin/turn",
        "pushd sub && rm ../.git/klin/turn",
        ". ./env.sh && rm .git/klin/turn",
        "f() { cd sub; }; f; rm ../.git/klin/turn",
        "cd missing && rm ../.git/klin/turn",
        "false && cd sub; echo x > ../.git/klin/turn",
        "eval 'cd sub' | echo x > ../.git/klin/turn",
        "eval 'cd sub' & echo x > ../.git/klin/turn",
        "false && eval 'cd sub'; echo x > ../.git/klin/turn",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
}

/// A command that runs in a child shell cannot move the directory of the one that started it.
/// ADR 0068.
#[test]
fn a_child_shell_keeps_its_cd_to_itself() {
    let tree = Tree::new();
    tree.write("sub/notes.md", "one\n");
    asked(
        &bash_in(&tree, "sh -c 'cd sub && rm ../.git/klin/turn'"),
        "../.git/klin/turn",
        "a cd inside the child",
    );
    allowed(
        &bash_in(&tree, "sh -c 'cd sub'; rm ../.git/klin/turn"),
        "a cd that stayed in the child",
    );
    asked(
        &bash_in(&tree, "eval 'cd sub'; rm ../.git/klin/turn"),
        "../.git/klin/turn",
        "eval runs in the same shell",
    );
}

/// Shell text the guard can read in full is read again as the command it runs. ADR 0068.
#[test]
fn reads_a_literal_nested_command() {
    let tree = Tree::new();
    for command in [
        "sh -c 'rm .git/klin/turn'",
        "sudo bash -lc 'rm -f .git/klin/turn'",
        "eval 'rm .git/klin/turn'",
        "eval rm .git/klin/turn",
        "sh <<'EOF'\nrm .git/klin/turn\nEOF",
        "bash -s <<EOF\ntrue\nrm .git/klin/turn\nEOF",
    ] {
        asked(&bash_in(&tree, command), ".git/klin/turn", command);
    }
    for command in [
        "bash -c \"echo x > .git/klin/turn\"",
        "eval echo x '>' .git/klin/turn",
        "cat <<'EOF' > .git/klin/turn\nnothing to see\nEOF",
    ] {
        denied(&bash_in(&tree, command), command);
    }
    denied(
        &bash_in(&tree, "sh -c 'klin setup'"),
        "klin's own command nested",
    );
    for command in [
        "cat <<'EOF'\nrm .git/klin/turn\nEOF",
        "sh script.sh .git/klin/turn",
        "sh -o errexit -c 'rm .git/klin/turn'",
        "sh -c \"echo y $X > .git/klin/turn\"",
        "eval \"echo y $X > .git/klin/turn\"",
        "bash <<EOF\necho y $X > .git/klin/turn\nEOF",
        "echo 'sh -c \"rm .git/klin/turn\"'",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
}

/// A guarded name inside a script, a comment or a quoted argument is text, not a write. ADR 0068.
#[test]
fn a_mention_of_klins_state_is_not_a_write() {
    for command in [
        "sed -i 's/a/b/; s| .git/klin/turn |x|' notes.md",
        "perl -i -pe 's| .git/klin/turn |x|' notes.md",
        "rm notes.txt # .git/klin/turn",
        "rm \".git/klin backup\"",
        "echo x > notes.md # > .git/klin/turn",
    ] {
        allowed(&bash(command), command);
    }
}

/// A quoted path holds its spaces, and resolves as the shell would. ADR 0068.
#[test]
fn a_quoted_path_with_spaces_resolves_as_one_path() {
    let tree = Tree::new();
    tree.write("my dir/notes.md", "one\n");
    denied(
        &bash_in(&tree, "echo x > \"my dir/../.git/klin/turn\""),
        "a quoted redirect target",
    );
    asked(
        &bash_in(&tree, "cd 'my dir' && cp ../notes.md ../.git/klin/turn"),
        "../.git/klin/turn",
        "a quoted cd",
    );
}

/// Ordinary writes to the source tree stay ordinary work. ADR 0068.
#[test]
fn allows_ordinary_source_writes() {
    let tree = Tree::new();
    tree.write("src/main.rs", "fn main() {}\n");
    for command in [
        "echo x > src/main.rs",
        "cp src/main.rs src/copy.rs",
        "install -m 644 src/main.rs /tmp/main.rs",
        "cd src && rm main.rs",
        "sh -c 'cp src/main.rs src/b.rs'",
        "cp .git/klin/turn src/",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
}

/// A tool's own path is not shell text, so a `$` in it is a character and the path still
/// resolves. ADR 0068.
#[test]
fn an_edit_path_holding_a_dollar_still_resolves() {
    let tree = Tree::new();
    denied(
        &edit_in(
            &tree,
            "Write",
            &format!("{}/$x/../.git/klin/turn", tree.root().display()),
        ),
        "a literal dollar directory",
    );
}

/// `-n` and `-u` may leave an existing destination as it is, so only a destination that is not
/// there yet is a proven write. ADR 0068.
#[test]
fn a_copy_that_may_keep_the_destination_proves_only_a_new_one() {
    let tree = Tree::new();
    tree.write(".git/klin/turn", "x\n");
    for command in [
        "cp -n source .git/klin/turn",
        "cp --no-clobber source .git/klin/turn",
        "cp -u source .git/klin/turn",
        "cp --update source .git/klin/turn",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
}

/// Shell text nested a few levels deep is still read, and a long chain of `eval` stays unread
/// instead of growing the guard's work with its square. ADR 0068.
#[test]
fn reads_nested_text_to_a_bounded_depth() {
    let tree = Tree::new();
    asked(
        &bash_in(&tree, "eval eval eval rm .git/klin/turn"),
        ".git/klin/turn",
        "three nested evals",
    );
    let chain = format!("{}rm .git/klin/turn", "eval ".repeat(20_000));
    allowed(&bash_in(&tree, &chain), "a chain of twenty thousand evals");
}

/// A copy of a directory's contents writes names the guard does not read from the command, so
/// it proves only that the destination directory is written under. ADR 0068.
#[test]
fn a_copy_of_directory_contents_proves_only_its_destination() {
    let tree = Tree::new();
    tree.write("staging/.git/klin/turn", "{}\n");
    for command in [
        "cp -a staging/. .",
        "cp -R staging/ .",
        "cp -r . ../elsewhere",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
    for (command, named) in [
        ("cp -a staging/. .git/klin", ".git/klin"),
        ("cp -R -t .git/klin/backup staging/", ".git/klin/backup"),
    ] {
        asked(&bash_in(&tree, command), named, command);
    }
}

/// `--parents` keeps each source's path under the destination directory. ADR 0068.
#[test]
fn a_copy_with_parents_writes_the_source_path_under_the_destination() {
    let tree = Tree::new();
    tree.write("staging/.git/klin/turn", "x\n");
    asked(
        &bash_in(&tree, "cd staging && cp --parents .git/klin/turn .."),
        "../.git/klin/turn",
        "a state path rebuilt under the root",
    );
}

/// Overwrite options settle in order, and `--update=all` overwrites. ADR 0068.
#[test]
fn the_overwrite_options_of_a_copy_settle_in_order() {
    let tree = Tree::new();
    tree.write(".git/klin/turn", "x\n");
    for command in [
        "cp --update=all source .git/klin/turn",
        "cp -n -i source .git/klin/turn",
        "cp --no-clobber --interactive source .git/klin/turn",
    ] {
        asked(&bash_in(&tree, command), ".git/klin/turn", command);
    }
    for command in [
        "cp --update=none source .git/klin/turn",
        "cp --update=older source .git/klin/turn",
        "cp -i -n source .git/klin/turn",
        "cp -n -f source .git/klin/turn",
        "cp --update=sometimes source .git/klin/turn",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
}

/// A `cd` is logical, as the shell's is by default, so `..` leaves a link the way it came in.
/// `-P` follows the link. A file path still resolves where the process stands. ADR 0068.
#[test]
fn a_cd_through_a_link_comes_back_the_way_it_went() {
    let tree = Tree::new();
    tree.write("inner/deep/notes.md", "one\n");
    std::os::unix::fs::symlink(tree.path("inner/deep"), tree.path("link")).unwrap();
    denied(
        &bash_in(&tree, "cd link && cd .. && echo x > .git/klin/turn"),
        "a logical cd back out of a link",
    );
    for command in [
        "cd -P link && cd .. && echo x > .git/klin/turn",
        "cd link && echo x > ../.git/klin/turn",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
}

/// A sourced script may move the directory however it is spelled. ADR 0068.
#[test]
fn a_sourced_script_leaves_the_directory_unproven() {
    let tree = Tree::new();
    tree.write("sub/notes.md", "one\n");
    for command in [
        "builtin source ./enter-sub.sh; echo x > .git/klin/turn",
        "builtin . ./enter-sub.sh; echo x > .git/klin/turn",
        "command source ./enter-sub.sh; echo x > .git/klin/turn",
    ] {
        allowed(&bash_in(&tree, command), command);
    }
}

/// An absolute `cd` places the directory even after a `cd` the guard could not follow. ADR 0068.
#[test]
fn an_absolute_cd_places_the_directory_again() {
    let tree = Tree::new();
    let command = format!(
        "cd \"$DIR\"; cd {} && rm .git/klin/turn",
        tree.root().display()
    );
    asked(
        &bash_in(&tree, &command),
        ".git/klin/turn",
        "an absolute cd after a lost one",
    );
}
