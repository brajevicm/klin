mod harness;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use harness::Tree;
use serde_json::Value;

const PLUGIN: &str = "plugins/klin";
const WRAPPER: &str = "plugins/klin/bin/klin";
const HOOKS: &str = "plugins/klin/hooks/hooks.json";
const CURSOR_HOOKS: &str = "plugins/klin/hooks/cursor.json";
const MANIFEST: &str = "plugins/klin/.claude-plugin/plugin.json";
const CURSOR_MANIFEST: &str = "plugins/klin/.cursor-plugin/plugin.json";
const MARKET: &str = ".claude-plugin/marketplace.json";
const CODEX_MARKET: &str = ".agents/plugins/marketplace.json";
const CURSOR_MARKET: &str = ".cursor-plugin/marketplace.json";
const TAGGED_MARKETS: [(&str, &str); 2] =
    [(MARKET, "plugins/klin"), (CODEX_MARKET, "./plugins/klin")];
const README: &str = "README.md";
const DIST_WORKSPACE: &str = "dist-workspace.toml";
const SHARED_MATCHER: &str = "Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*";
/// The version the wrapper pins, which every test fetches into a cache of its own.
const PINNED: &str = env!("CARGO_PKG_VERSION");
const SHELL: &str = "/bin/sh";
/// The tools the wrapper needs and nothing a person installed.
const SYSTEM_PATH: &str = "/usr/bin:/bin";

struct Ran {
    code: i32,
    /// What the run put on stdout, which is the channel spec 9.1 gives this text.
    printed: String,
    /// Both channels together, for an assertion that does not judge the channel.
    out: String,
}

#[test]
fn the_hooks_carry_the_three_commands() {
    let matcher = json(HOOKS)["hooks"]["PreToolUse"][0]["matcher"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    assert!(hook("SessionStart").ends_with("\"$k\" radius"));
    assert!(hook("UserPromptSubmit").ends_with("\"$k\" radius"));
    assert!(hook("PreToolUse").ends_with("\"$k\" guard"));
    assert!(hook("Stop").ends_with("\"$k\" gate --hook --changed"));
    assert_eq!(matcher, SHARED_MATCHER);
}

#[test]
fn the_cursor_plugin_keeps_the_generated_hook_shape() {
    let tree = Tree::new();
    let run = tree.run(&["install", "--host", "cursor"]);
    assert_eq!(run.code, 0, "{}", run.out);

    let generated = json_file(&tree.path(".cursor/hooks.json"), "generated Cursor hooks");
    let shipped = json(CURSOR_HOOKS);
    assert_eq!(shipped["version"], generated["version"]);

    let shipped_events = hook_events(&shipped, "shipped Cursor hooks");
    let generated_events = hook_events(&generated, "generated Cursor hooks");
    assert_eq!(
        shipped_events.keys().collect::<Vec<_>>(),
        generated_events.keys().collect::<Vec<_>>()
    );
    for event in shipped_events.keys() {
        assert_eq!(
            cursor_matchers(&shipped, event),
            cursor_matchers(&generated, event),
            "{event} matcher differs"
        );
    }
}

/// Codex CLI reads the same manifest, and is told where the hooks are rather than left to look.
#[test]
fn the_manifest_names_the_hooks_file() {
    let named = json(MANIFEST)["hooks"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(at(&format!("{PLUGIN}/{named}")).is_file(), "{named}");
}

/// Cursor must not discover Claude Code's nested `hooks/hooks.json`, so the Cursor manifest
/// names the flat file.
#[test]
fn the_cursor_manifest_names_the_cursor_hooks_file() {
    let named = json(CURSOR_MANIFEST)["hooks"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert_eq!(named, "./hooks/cursor.json", "{named}");
    assert!(at(&format!("{PLUGIN}/{named}")).is_file(), "{named}");
}

/// Claude Code and Codex read a marketplace from the default branch, so each entry names the
/// plugin at the release tag the manifests pin, and a commit to `main` reaches a plugin user
/// only once `main` names the next tag. Each entry spells the path as its host documents it:
/// Claude Code the bare path, Codex the `./` form. ADR 0029, spec 19.2.
#[test]
fn the_claude_code_and_codex_marketplaces_name_the_plugin_at_the_pinned_release() {
    for (market, path) in TAGGED_MARKETS {
        let entry = json(market)["plugins"][0].clone();

        assert_eq!(entry["name"], "klin", "{market}");
        assert_eq!(
            entry["source"],
            serde_json::json!({
                "source": "git-subdir",
                "url": "https://github.com/brajevicm/klin.git",
                "path": path,
                "ref": format!("v{PINNED}"),
            }),
            "{market}"
        );
    }
}

/// The release commit rewrites each marketplace ref to the new tag and nothing else. The test
/// applies the rewrites `Cargo.toml` configures, as cargo-release does, for a version no
/// release has. ADR 0029, spec 19.2.
#[test]
fn the_release_rewrites_each_marketplace_ref_to_the_new_tag() {
    let version = "9.9.9";

    for (market, _) in TAGGED_MARKETS {
        let mut tagged = json(market);
        tagged["plugins"][0]["source"]["ref"] = Value::from(format!("v{version}"));

        let rewritten = match serde_json::from_str::<Value>(&after_cargo_release(market, version)) {
            Ok(held) => held,
            Err(why) => panic!("the release leaves {market} unreadable: {why}"),
        };
        assert_eq!(rewritten, tagged, "{market}");
    }
}

/// cargo-release pushes nothing, so the release commit reaches `main` only through the
/// promotion after the release smoke. ADR 0029, spec 19.2.
#[test]
fn cargo_release_pushes_nothing_by_itself() {
    let push = cargo_release_config()
        .get("push")
        .and_then(cargo_toml::Value::as_bool);

    assert_eq!(push, Some(false), "cargo-release pushes the release itself");
}

/// A release stays off `releases/latest`, the installer and `klin update` until the release
/// smoke passed: `cut-release` makes it a draft prerelease, `dist` publishes that draft and
/// leaves both flags alone, and `promote-release` marks it Latest. ADR 0029, spec 19.2.
#[test]
fn a_release_becomes_latest_only_at_the_promotion() {
    let published = text(".github/workflows/release.yml");
    let drafted = line_with(".github/workflows/cut-release.yml", "gh release create");
    let promoted = line_with(".github/workflows/promote-release.yml", "gh release edit");

    assert!(
        text(DIST_WORKSPACE)
            .lines()
            .any(|line| line.trim() == "create-release = false"),
        "dist creates the release itself, as Latest"
    );
    for flag in ["gh release create", "--latest", "--prerelease=false"] {
        assert!(
            !published.contains(flag),
            "dist's release workflow runs {flag}"
        );
    }
    for flag in ["--draft", "--prerelease"] {
        assert!(
            drafted.contains(flag),
            "cut-release omits {flag}: {drafted}"
        );
    }
    for flag in ["--prerelease=false", "--latest"] {
        assert!(
            promoted.contains(flag),
            "promote-release omits {flag}: {promoted}"
        );
    }
}

#[test]
fn the_cursor_marketplace_entry_points_at_the_same_plugin() {
    let entry = json(CURSOR_MARKET)["plugins"][0].clone();
    let path = entry["source"].as_str().unwrap_or_default().to_string();

    assert_eq!(entry["name"].as_str().unwrap_or_default(), "klin");
    assert_eq!(path, PLUGIN, "Cursor documents the bare form");
    assert!(
        at(&path).join(".cursor-plugin/plugin.json").is_file(),
        "{}",
        path
    );
}

#[test]
fn the_plugin_carries_the_skill_and_the_two_commands() {
    for named in [
        "skills/klin/SKILL.md",
        "commands/gate.md",
        "commands/gates.md",
    ] {
        let path = at(&format!("{PLUGIN}/{named}"));
        assert!(path.is_file(), "{} is missing", path.display());
    }
    let skill = text(&format!("{PLUGIN}/skills/klin/SKILL.md"));

    assert!(skill.contains("klin-installer.sh"), "{skill}");
}

/// The install commands the README prints name the marketplace and the plugin that ship here,
/// so a rename of either one fails in this test rather than in a person's session. Spec 19.2.
#[test]
fn the_readme_install_commands_name_the_shipped_plugin() {
    let market = name(MARKET);
    let plugin = name(MANIFEST);
    let named = format!("{plugin}@{market}");
    let readme = text(README);

    assert_eq!(name(CODEX_MARKET), market, "the two marketplaces differ");
    assert_eq!(
        name(CURSOR_MARKET),
        market,
        "the Cursor marketplace differs"
    );
    for command in [
        "/plugin marketplace add brajevicm/klin".to_string(),
        format!("/plugin install {named}"),
        "codex plugin marketplace add brajevicm/klin".to_string(),
        format!("codex plugin add {named}"),
    ] {
        assert!(readme.contains(&command), "the README omits {command}");
    }
    for said in [
        "https://github.com/brajevicm/klin",
        "~/.cursor/plugins/local/klin",
    ] {
        assert!(readme.contains(said), "the README omits {said}");
    }
}

/// The README tells a Codex user to trust the installed hooks and to start a fresh session,
/// because Codex skips an untrusted plugin's hooks. Spec 19.2.
#[test]
fn the_readme_names_the_codex_hook_trust_step() {
    let readme = text(README);

    assert!(
        readme
            .contains("Run `/hooks`, review and trust the klin hooks, then start a fresh session."),
        "the README omits the Codex hook trust step"
    );
}

/// The README leads with one command run from the repository: the installer, then the
/// installed binary by its full path, because PATH does not hold it until a new terminal. It
/// offers the three plugins after it as the host-managed alternative, gives the repository
/// opt-in a plugin user takes, and points any other harness at the harness protocol without
/// promising that the binary alone connects it. Spec 19.0, 19.1, 19.4, ADR 0053, ADR 0056.
#[test]
fn the_readme_leads_with_the_cli_and_offers_the_plugins_after_it() {
    let readme = text(README);
    let installer = readme.find("klin-installer.sh").unwrap_or(usize::MAX);
    let plugin = readme.find("/plugin install").unwrap_or(0);

    assert!(
        installer < plugin,
        "the README does not lead with the installer"
    );
    assert!(
        text(DIST_WORKSPACE).contains(r#"install-path = "~/.local/bin""#),
        "the installer puts klin somewhere other than the path the README runs it from and \
         the hook lines look in"
    );
    for said in [
        "**Claude Code · Codex · Cursor**",
        "`--host claude`",
        "### Or use your host's plugin",
        "A plugin's checks stay quiet until the repository opts in",
        "`{}` is a complete configuration.",
        "`klin update`",
        "### Other coding agents",
        "docs/HARNESS_INTEGRATION.md",
        "harness protocol",
        "Installing the klin binary alone does not connect another harness.",
        "does not make that harness first-class",
    ] {
        assert!(readme.contains(said), "the README omits {said}");
    }
}

/// A `curl` that stands in for the release download: it answers the installer URL alone, with
/// the body given, and fails like `curl -f` on anything else.
fn fake_curl(body: &str) -> String {
    format!("#!/bin/sh\ncase \"$*\" in *klin-installer.sh*) ;; *) exit 22 ;; esac\n{body}\n")
}

/// An installer that puts a klin in `~/.local/bin` which records where it ran and with what.
const AN_INSTALLER: &str = r#"cat <<'SH'
mkdir -p "$HOME/.local/bin"
printf '#!/bin/sh\necho "$PWD $*" > "$HOME/ran"\n' > "$HOME/.local/bin/klin"
chmod +x "$HOME/.local/bin/klin"
SH"#;

/// The README's install is one command run from the repository, in any shell a person types
/// it into, and it runs klin only once the installer succeeded. A download that fails runs no klin, not even one an earlier install
/// left in `~/.local/bin`. The installed klin runs by its full path, before PATH holds it. #318.
#[test]
fn the_readmes_install_runs_klin_only_after_the_installer_succeeded() {
    let script = block(&text(README), "klin-installer.sh");
    for (curl, left_behind, runs) in [("exit 22", true, false), (AN_INSTALLER, false, true)] {
        let work = Tree::bare();
        work.write("your-repo/.keep", "");
        work.write("bin/curl", &fake_curl(curl));
        executable(&work.path("bin/curl"));
        let home = Tree::bare();
        if left_behind {
            home.write(
                ".local/bin/klin",
                "#!/bin/sh\necho \"$PWD $*\" > \"$HOME/ran\"\n",
            );
            executable(&home.path(".local/bin/klin"));
        }

        let path = format!("{}:/usr/bin:/bin", work.path("bin").display());
        let home_dir = home.root().display().to_string();
        let run = ran(
            SHELL,
            &["-c", &script],
            work.root(),
            &[("PATH", path.as_str()), ("HOME", home_dir.as_str())],
        );
        let klin_ran = fs::read_to_string(home.path("ran")).unwrap_or_default();
        assert_eq!(run.code == 0, runs, "{curl}: {}", run.out);
        assert_eq!(
            klin_ran.trim().ends_with("/your-repo install"),
            runs,
            "{curl}: {klin_ran}{}",
            run.out
        );
    }
}

/// The README's Cursor fallback is a copy a person may run twice, so it must leave one usable
/// plugin and never nest one plugin inside another. Spec 19.2.
#[test]
fn the_readmes_cursor_copy_leaves_one_plugin_when_it_runs_twice() {
    let tree = Tree::bare();
    let home = tree.root().display().to_string();
    let script = cursor_copy(&format!(
        "cp -R \"{}\" \"$d/plugins\"",
        at("plugins").display()
    ));

    for _ in 0..2 {
        let run = ran(
            SHELL,
            &["-c", &script],
            tree.root(),
            &[("PATH", SYSTEM_PATH), ("HOME", &home)],
        );
        assert_eq!(run.code, 0, "{}", run.out);
    }

    let installed = tree.path(".cursor/plugins/local/klin");
    assert!(
        installed.join(".cursor-plugin/plugin.json").is_file(),
        "the copy left no plugin manifest"
    );
    assert!(
        !installed.join("klin").exists(),
        "a second run nested the plugin inside the first copy"
    );
}

/// The README's Cursor copy is also its update, so a fetch that fails leaves the plugin already
/// installed as it was and says so with a failing status. Spec 19.2.
#[test]
fn the_readmes_cursor_copy_keeps_the_installed_plugin_when_the_fetch_fails() {
    let tree = Tree::bare();
    let home = tree.root().display().to_string();
    let kept = tree.write(
        ".cursor/plugins/local/klin/.cursor-plugin/plugin.json",
        "{\"name\":\"klin\"}",
    );

    let run = ran(
        SHELL,
        &["-c", &cursor_copy("false")],
        tree.root(),
        &[("PATH", SYSTEM_PATH), ("HOME", &home)],
    );

    assert_ne!(run.code, 0, "the failed copy reported success: {}", run.out);
    assert!(
        kept.is_file(),
        "the failed copy removed the installed plugin"
    );
}

/// The README's Cursor copy with its fetch replaced by `fetch`, so a test runs the commands a
/// person is given against a release it controls.
fn cursor_copy(fetch: &str) -> String {
    let clone =
        format!("git clone --depth 1 --branch v{PINNED} https://github.com/brajevicm/klin \"$d\"");
    let script = block(&text(README), "~/.cursor/plugins/local/klin");
    assert!(
        script.contains(&clone),
        "the README copy clones no release: {script}"
    );
    script.replace(&clone, fetch)
}

/// The fenced block of a document that holds one line, so a test runs the commands a person
/// is given rather than a copy of them.
fn block(document: &str, holds: &str) -> String {
    for fenced in document.split("\n```").skip(1).step_by(2) {
        let lines: Vec<&str> = fenced.lines().skip(1).collect();
        if lines.iter().any(|line| line.contains(holds)) {
            return lines.join("\n");
        }
    }
    panic!("no fenced block holds {holds}")
}

#[test]
fn the_plugin_pins_the_crate_version() {
    let carried = json(MANIFEST)["version"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    assert_eq!(carried, PINNED, "the plugin pins another version");
    let cursor = json(CURSOR_MANIFEST)["version"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert_eq!(cursor, PINNED, "the Cursor plugin pins another version");
}

/// The README's Cursor copy clones the release the manifests pin, so a copy run again takes a
/// released plugin and never a wrapper from an unreleased branch. ADR 0029, spec 19.2.
#[test]
fn the_readmes_cursor_copy_clones_the_pinned_release() {
    let copy = block(&text(README), "~/.cursor/plugins/local/klin");

    assert!(
        copy.contains(&format!(
            "git clone --depth 1 --branch v{PINNED} https://github.com/brajevicm/klin"
        )),
        "{copy}"
    );
}

/// The wrapper reads the version from the plugin manifest beside it, so a wrapper without one
/// has nothing to fetch and says so. Spec 19.2.
#[test]
fn a_wrapper_without_its_manifest_says_so_and_lets_the_turn_end() {
    let tree = Tree::bare();
    let copied = tree.write("bin/klin", &text(WRAPPER));
    executable(&copied);

    let run = ran(
        &copied.display().to_string(),
        &["--version"],
        tree.root(),
        &[],
    );

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        notice(&run.printed).contains("names no version"),
        "{}",
        run.out
    );
}

#[test]
fn the_first_run_fetches_the_pinned_release_and_the_second_fetches_nothing() {
    let tree = Tree::bare();
    let archive = release(&tree, "the-fetched-binary");

    let first = fetch(&tree, &["--version"]);
    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        first.printed.contains("the-fetched-binary"),
        "{}",
        first.out
    );
    assert!(cached(&tree).is_file(), "nothing landed in the cache");

    remove(&archive);
    remove(&checksum(&archive));
    let second = fetch(&tree, &["--version"]);

    assert_eq!(second.code, 0, "{}", second.out);
    assert!(
        second.printed.contains("the-fetched-binary"),
        "{}",
        second.out
    );
}

#[test]
fn a_checksum_that_does_not_match_installs_nothing_and_lets_the_turn_end() {
    let tree = Tree::bare();
    let archive = release(&tree, "the-binary-that-never-runs");
    tree.write(&relative(&checksum(&archive)), "0000000000000000  klin\n");

    let run = fetch(&tree, &["--version"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        notice(&run.printed).contains("could not be installed"),
        "{}",
        run.out
    );
    assert!(!cached(&tree).exists(), "a binary stayed in the cache");
    assert_eq!(held(&tree), "", "the scratch directory stayed behind");
}

#[test]
fn a_download_that_fails_prints_one_line_and_lets_the_turn_end() {
    let tree = Tree::bare();

    let run = fetch(&tree, &["gate", "--hook", "--changed"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out.lines().count(), 1, "{}", run.out);
    assert!(
        notice(&run.printed).contains("could not be installed"),
        "{}",
        run.out
    );
    assert_eq!(held(&tree), "", "the scratch directory stayed behind");
}

/// The install hint is a `systemMessage`, the one shape both hosts show on a Stop that exits 0.
/// Codex rejects plain text there, and Claude Code writes it to the debug log alone.
#[test]
fn the_stop_says_how_to_install_klin_when_it_is_on_no_path() {
    let tree = Tree::bare();
    tree.write("klin.json", "{}\n");

    let run = without_klin(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    let said = notice(&run.printed);
    assert!(said.contains("klin-installer.sh"), "{}", run.out);
}

/// Codex substitutes the literal `${CLAUDE_PLUGIN_ROOT}` into a plugin's hook line, so a shell
/// default such as `${CLAUDE_PLUGIN_ROOT:-}` is left as it is and expands to nothing. Claude
/// Code exports the variable, and reads the bare form the same way.
#[test]
fn the_hook_lines_name_the_plugin_root_in_the_form_both_hosts_substitute() {
    for event in ["SessionStart", "UserPromptSubmit", "PreToolUse", "Stop"] {
        let line = hook(event);
        assert!(
            line.contains("${CLAUDE_PLUGIN_ROOT}/bin/klin"),
            "{event}: {line}"
        );
        assert!(!line.contains("CLAUDE_PLUGIN_ROOT:-"), "{event}: {line}");
    }
}

#[test]
fn the_cursor_hook_lines_name_the_cursor_plugin_root() {
    for event in [
        "sessionStart",
        "beforeSubmitPrompt",
        "preToolUse",
        "beforeShellExecution",
        "beforeMCPExecution",
        "stop",
    ] {
        let line = cursor_hook(event);
        assert!(
            line.contains("${CURSOR_PLUGIN_ROOT}/bin/klin"),
            "{event}: {line}"
        );
        assert!(!line.contains("CLAUDE_PLUGIN_ROOT"), "{event}: {line}");
        assert!(!line.contains("CURSOR_PLUGIN_ROOT:-"), "{event}: {line}");
    }
}

/// Cursor submits a stop's `followup_message` as the next prompt, so the install hint in the
/// Cursor stop line is never one: it would hand the installer to the agent. Spec 19.2.
#[test]
fn the_cursor_stop_hands_the_installer_to_no_agent() {
    let line = cursor_hook("stop");

    assert!(line.contains("klin-installer.sh"), "{line}");
    assert!(!line.contains("followup_message"), "{line}");
}

/// The `systemMessage` of a JSON notice on stdout, or a panic naming what was printed instead.
fn notice(printed: &str) -> String {
    let Ok(held) = serde_json::from_str::<serde_json::Value>(printed.trim()) else {
        panic!("stdout is not a JSON notice: {printed}")
    };
    held["systemMessage"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// A tree that never opted in hears nothing, even from a host whose hooks are installed for
/// every repository a person opens. ADR 0028, spec 5.1.
#[test]
fn the_stop_says_nothing_in_a_tree_that_holds_no_configuration() {
    let tree = Tree::bare();

    let run = without_klin(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "the stop spoke in a tree that did not opt in");
}

/// A machine with no network, or a release it cannot reach, still has whatever a person
/// installed by another route. The wrapper runs that before it gives up. Spec 19.2.
#[test]
fn a_download_that_fails_runs_a_klin_on_path_instead() {
    let tree = Tree::bare();
    let on_path = tree.write("bin/klin", "#!/bin/sh\necho the-path-binary\n");
    executable(&on_path);
    let base = format!("file://{}", tree.path("release").display());
    let path = format!(
        "{}:{}",
        tree.path("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let run = ran(
        &at(WRAPPER).display().to_string(),
        &["--version"],
        tree.root(),
        &[
            ("PATH", &path),
            ("KLIN_RELEASE_BASE_URL", &base),
            ("KLIN_CACHE_DIR", &tree.path("cache").display().to_string()),
        ],
    );

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.printed.trim(), "the-path-binary", "{}", run.out);
}

/// A plugin update pins a new version, and the fetch that installs it removes the ones before
/// it that nothing ran for a week. Another host's plugin may pin another version into the same
/// cache, and a version it ran this week stays, so the two do not fetch in turn. Spec 19.2.
#[test]
fn a_fetch_removes_the_other_cached_versions_nothing_ran_this_week() {
    let tree = Tree::bare();
    release(&tree, "the-fetched-binary");
    tree.write("cache/bin/0.0.1/klin", "#!/bin/sh\necho stale\n");
    tree.write("cache/bin/0.0.2/klin", "#!/bin/sh\necho recent\n");
    let aged = ran(
        "touch",
        &[
            "-t",
            "202001010000",
            &tree.path("cache/bin/0.0.1").display().to_string(),
        ],
        tree.root(),
        &[],
    );
    assert_eq!(aged.code, 0, "{}", aged.out);

    let run = fetch(&tree, &["--version"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        held(&tree),
        format!("0.0.2 {PINNED}"),
        "the cache kept a stale version"
    );
}

/// A plugin user with no `klin` of their own hears once that the CLI exists, at the first turn
/// whose radius printed nothing, and never again on that machine. Spec 19.2.
#[test]
fn the_wrapper_names_the_cli_once_to_a_person_without_one() {
    let tree = Tree::bare();
    release_running(&tree, "true");

    let first = fetch(&tree, &["radius"]);
    let second = fetch(&tree, &["radius"]);

    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        notice(&first.printed).contains("klin-installer.sh"),
        "{}",
        first.out
    );
    assert!(!first.printed.contains("followup_message"), "{}", first.out);
    assert_eq!(second.printed, "", "the hint came twice");
}

/// Cursor shows no message at a prompt, so under Cursor the hint waits for another host.
#[test]
fn the_wrapper_names_the_cli_to_nobody_under_cursor() {
    let tree = Tree::bare();
    release_running(&tree, "true");
    let base = format!("file://{}", tree.path("release").display());

    let run = ran(
        &at(WRAPPER).display().to_string(),
        &["radius"],
        tree.root(),
        &[
            ("PATH", SYSTEM_PATH),
            ("KLIN_RELEASE_BASE_URL", &base),
            ("KLIN_CACHE_DIR", &tree.path("cache").display().to_string()),
            ("CURSOR_VERSION", "3.20.21"),
        ],
    );

    assert_eq!(run.printed, "", "{}", run.out);
}

/// A radius run marks its version as used, so a later fetch of another pin keeps it.
#[test]
fn a_radius_run_keeps_its_version_in_the_cache() {
    let tree = Tree::bare();
    release(&tree, "the-fetched-binary");
    assert_eq!(fetch(&tree, &["--version"]).code, 0);
    let pinned = tree.path(&format!("cache/bin/{PINNED}"));
    let aged = ran(
        "touch",
        &["-t", "202001010000", &pinned.display().to_string()],
        tree.root(),
        &[],
    );
    assert_eq!(aged.code, 0, "{}", aged.out);

    assert_eq!(fetch(&tree, &["radius"]).code, 0);

    let used = fs::metadata(&pinned)
        .and_then(|held| held.modified())
        .unwrap_or_else(|why| panic!("{why}"));
    let week = std::time::Duration::from_secs(7 * 24 * 60 * 60);
    assert!(
        used.elapsed().is_ok_and(|age| age < week),
        "the radius run left its version looking unused"
    );
}

/// The hint never displaces what radius printed, and a person who has a `klin` of their own
/// never hears it.
#[test]
fn the_wrapper_names_the_cli_to_nobody_it_would_interrupt() {
    let tree = Tree::bare();
    release(&tree, "the-radius-note");
    let spoke = fetch(&tree, &["radius"]);
    assert_eq!(spoke.printed.trim(), "the-radius-note", "{}", spoke.out);

    let decoy = tree.write("path/klin", "#!/bin/sh\n");
    executable(&decoy);
    release_running(&tree, "true");
    let base = format!("file://{}", tree.path("release").display());
    let path = format!("{}:{SYSTEM_PATH}", tree.path("path").display());
    let own = ran(
        &at(WRAPPER).display().to_string(),
        &["radius"],
        tree.root(),
        &[
            ("PATH", &path),
            ("KLIN_RELEASE_BASE_URL", &base),
            ("KLIN_CACHE_DIR", &tree.path("fresh").display().to_string()),
        ],
    );
    assert_eq!(own.printed, "", "{}", own.out);
}

/// The hook runs the plugin's own wrapper before any `klin` on PATH, so the version the plugin
/// pins is the one Claude Code runs, whatever an installer left in `~/.local/bin`. Spec 19.2.
#[test]
fn the_hook_runs_the_plugin_wrapper_before_a_klin_on_path() {
    let tree = Tree::bare();
    release(&tree, "the-plugin-binary");
    let decoy = tree.write("path/klin", "#!/bin/sh\necho the-path-binary\n");
    executable(&decoy);
    let path = format!(
        "{}:/usr/bin:/bin",
        decoy.parent().unwrap_or(tree.root()).display()
    );
    let base = format!("file://{}", tree.path("release").display());

    let run = ran(
        SHELL,
        &["-c", &hook("UserPromptSubmit")],
        tree.root(),
        &[
            ("PATH", &path),
            ("CLAUDE_PLUGIN_ROOT", &at(PLUGIN).display().to_string()),
            ("KLIN_RELEASE_BASE_URL", &base),
            ("KLIN_CACHE_DIR", &tree.path("cache").display().to_string()),
        ],
    );

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.printed.contains("the-plugin-binary"), "{}", run.out);
    assert!(!run.printed.contains("the-path-binary"), "{}", run.out);
}

/// The stop hook as the plugin ships it, run where no `klin` resolves on PATH.
fn without_klin(tree: &Tree) -> Ran {
    let empty = tree.path("empty");
    made(&empty);
    ran(
        SHELL,
        &["-c", &hook("Stop")],
        tree.root(),
        &[
            ("PATH", &empty.display().to_string()),
            ("CLAUDE_PROJECT_DIR", &tree.root().display().to_string()),
        ],
    )
}

/// A release the wrapper can fetch over `file://`: one archive holding a `klin` that prints
/// `says`, and the checksum file beside it.
fn release(tree: &Tree, says: &str) -> PathBuf {
    release_running(tree, &format!("echo {says}"))
}

/// A release whose `klin` runs `body`.
fn release_running(tree: &Tree, body: &str) -> PathBuf {
    let written = tree.write("release/stage/klin", &format!("#!/bin/sh\n{body}\n"));
    let stage = written
        .parent()
        .unwrap_or(tree.root())
        .display()
        .to_string();
    let archive = tree.path(&format!("release/klin-{}.tar.xz", triple()));
    let made = ran(
        "tar",
        &["-cJf", &archive.display().to_string(), "-C", &stage, "klin"],
        tree.root(),
        &[],
    );
    assert_eq!(made.code, 0, "{}", made.out);
    tree.write(
        &relative(&checksum(&archive)),
        &format!("{}  klin-{}.tar.xz\n", digest(&archive), triple()),
    );
    archive
}

/// The wrapper run against the tree's own release, on a PATH that resolves no `klin`, so the
/// fetch is the only route and a binary a person installed does not stand in for it.
fn fetch(tree: &Tree, args: &[&str]) -> Ran {
    let base = format!("file://{}", tree.path("release").display());
    let cache = tree.path("cache").display().to_string();
    ran(
        &at(WRAPPER).display().to_string(),
        args,
        tree.root(),
        &[
            ("PATH", SYSTEM_PATH),
            ("KLIN_RELEASE_BASE_URL", &base),
            ("KLIN_CACHE_DIR", &cache),
        ],
    )
}

fn cached(tree: &Tree) -> PathBuf {
    tree.path(&format!("cache/bin/{PINNED}/klin"))
}

/// Every name the cache holds under `bin`, so a test can pin that a failed fetch left neither
/// a binary nor a scratch directory.
fn held(tree: &Tree) -> String {
    let Ok(entries) = fs::read_dir(tree.path("cache/bin")) else {
        return String::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names.join(" ")
}

fn checksum(archive: &Path) -> PathBuf {
    PathBuf::from(format!("{}.sha256", archive.display()))
}

/// The tail of a path inside the tree, which is what `Tree::write` takes.
fn relative(path: &Path) -> String {
    let named = path.display().to_string();
    match named.split_once("/release/") {
        Some((_, tail)) => format!("release/{tail}"),
        None => named,
    }
}

fn digest(path: &Path) -> String {
    let named = path.display().to_string();
    let held = match Command::new("shasum").args(["-a", "256", &named]).output() {
        Ok(done) if done.status.success() => done.stdout,
        _ => summed(&named),
    };
    String::from_utf8_lossy(&held)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string()
}

fn summed(named: &str) -> Vec<u8> {
    match Command::new("sha256sum").arg(named).output() {
        Ok(done) if done.status.success() => done.stdout,
        _ => panic!("this host has neither shasum nor sha256sum"),
    }
}

/// The release target the wrapper resolves for the host running the test.
fn triple() -> String {
    let system = match std::env::consts::OS {
        "macos" => "apple-darwin",
        _ => "unknown-linux-gnu",
    };
    format!("{}-{system}", std::env::consts::ARCH)
}

fn hook(event: &str) -> String {
    json(HOOKS)["hooks"][event][0]["hooks"][0]["command"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn cursor_hook(event: &str) -> String {
    json(CURSOR_HOOKS)["hooks"][event][0]["command"]
        .as_str()
        .unwrap_or_default()
        .to_string()
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

fn name(relative: &str) -> String {
    json(relative)["name"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// The host canary is a vendor-drift alarm, not a gate on a person's pull request: a red host
/// is the vendor's change, not the branch's. It runs only when a person dispatches it (#232),
/// and `docs/HOST_COMPATIBILITY.md` records what it proves.
#[test]
fn the_host_canary_stays_out_of_pull_request_gating() {
    let workflow = text(".github/workflows/host-compatibility.yml");

    assert!(
        workflow.contains("workflow_dispatch:"),
        "the canary cannot be dispatched by hand"
    );
    assert!(
        !workflow.contains("pull_request"),
        "the canary gates pull requests"
    );
    assert!(
        !text(".github/workflows/quality.yml").contains("host-canary"),
        "PR gating runs the host canary"
    );
}

fn json(relative: &str) -> serde_json::Value {
    match serde_json::from_str(&text(relative)) {
        Ok(held) => held,
        Err(why) => panic!("{relative} is not JSON: {why}"),
    }
}

fn line_with(relative: &str, holds: &str) -> String {
    match text(relative).lines().find(|line| line.contains(holds)) {
        Some(line) => line.to_string(),
        None => panic!("no line of {relative} holds {holds}"),
    }
}

fn cargo_release_config() -> cargo_toml::Value {
    let manifest = match cargo_toml::Manifest::from_str(&text("Cargo.toml")) {
        Ok(manifest) => manifest,
        Err(why) => panic!("Cargo.toml is not a manifest: {why}"),
    };
    match manifest
        .package
        .and_then(|package| package.metadata)
        .and_then(|metadata| metadata.get("release").cloned())
    {
        Some(release) => release,
        None => panic!("Cargo.toml has no [package.metadata.release]"),
    }
}

fn after_cargo_release(relative: &str, version: &str) -> String {
    let rewrites: Vec<cargo_toml::Value> = cargo_release_config()
        .get("pre-release-replacements")
        .and_then(cargo_toml::Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|rewrite| rewrite.get("file").and_then(cargo_toml::Value::as_str) == Some(relative))
        .collect();
    assert!(
        !rewrites.is_empty(),
        "the release does not rewrite {relative}"
    );

    rewrites.iter().fold(text(relative), |held, rewrite| {
        let field = |key: &str| {
            rewrite
                .get(key)
                .and_then(cargo_toml::Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let search = match regex::Regex::new(&field("search")) {
            Ok(search) => search,
            Err(why) => panic!("the release searches {relative} with a bad pattern: {why}"),
        };
        assert_eq!(
            rewrite
                .get("exactly")
                .and_then(cargo_toml::Value::as_integer),
            Some(1),
            "the release does not rewrite {relative} exactly once"
        );
        assert_eq!(
            search.find_iter(&held).count(),
            1,
            "the release pattern misses {relative}"
        );
        search
            .replace_all(&held, field("replace").replace("{{version}}", version))
            .into_owned()
    })
}

fn json_file(path: &Path, label: &str) -> Value {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(why) => panic!("{label} could not be read: {why}"),
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(why) => panic!("{label} are not JSON: {why}"),
    }
}

fn hook_events<'a>(settings: &'a Value, label: &str) -> &'a serde_json::Map<String, Value> {
    match settings["hooks"].as_object() {
        Some(events) => events,
        None => panic!("{label} have no event map"),
    }
}

fn text(relative: &str) -> String {
    let path = at(relative);
    match fs::read_to_string(&path) {
        Ok(held) => held,
        Err(why) => panic!("read {}: {why}", path.display()),
    }
}

fn at(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn made(path: &Path) {
    if let Err(why) = fs::create_dir_all(path) {
        panic!("create {}: {why}", path.display());
    }
}

fn executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Err(why) = fs::set_permissions(path, fs::Permissions::from_mode(0o755)) {
        panic!("chmod {}: {why}", path.display());
    }
}

fn remove(path: &Path) {
    if let Err(why) = fs::remove_file(path) {
        panic!("remove {}: {why}", path.display());
    }
}

fn ran(program: &str, args: &[&str], cwd: &Path, environment: &[(&str, &str)]) -> Ran {
    let done = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_remove("CURSOR_VERSION")
        .envs(environment.iter().copied())
        .output();
    match done {
        Ok(done) => {
            let printed = String::from_utf8_lossy(&done.stdout).to_string();
            let out = printed.clone() + &String::from_utf8_lossy(&done.stderr);
            Ran {
                code: done.status.code().unwrap_or(-1),
                printed,
                out,
            }
        }
        Err(why) => panic!("{program} could not run: {why}"),
    }
}
