mod harness;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use harness::Tree;

const PLUGIN: &str = "plugins/claude-code";
const WRAPPER: &str = "plugins/claude-code/bin/klin";
const HOOKS: &str = "plugins/claude-code/hooks/hooks.json";
const MANIFEST: &str = "plugins/claude-code/.claude-plugin/plugin.json";
const MARKET: &str = ".claude-plugin/marketplace.json";
/// The version the wrapper pins, which every test fetches into a cache of its own.
const PINNED: &str = env!("CARGO_PKG_VERSION");
const SHELL: &str = "/bin/sh";

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
    assert!(matcher.contains("Edit"), "{matcher}");
    assert!(matcher.contains("Bash"), "{matcher}");
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

#[test]
fn the_plugin_and_the_wrapper_pin_the_crate_version() {
    let carried = json(MANIFEST)["version"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    assert_eq!(carried, PINNED, "the plugin pins another version");
    assert!(
        text(WRAPPER).contains(&format!("VERSION=\"{PINNED}\"")),
        "the wrapper pins another version"
    );
}

#[test]
fn the_marketplace_entry_points_at_the_plugin() {
    let entry = json(MARKET)["plugins"][0].clone();
    let source = entry["source"].as_str().unwrap_or_default().to_string();
    let named = at(&source).join(".claude-plugin/plugin.json");

    assert_eq!(entry["name"].as_str().unwrap_or_default(), "klin");
    assert!(named.is_file(), "{} is missing", named.display());
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
        run.printed.contains("could not be installed"),
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
        run.printed.contains("could not be installed"),
        "{}",
        run.out
    );
    assert_eq!(held(&tree), "", "the scratch directory stayed behind");
}

#[test]
fn the_stop_says_how_to_install_klin_when_it_is_on_no_path() {
    let tree = Tree::bare();
    tree.write("klin.json", "{}\n");

    let run = without_klin(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.printed.contains("klin-installer.sh"), "{}", run.out);
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

/// A plugin update pins a new version, and the fetch that installs it removes the ones before
/// it, so the cache holds one binary. Spec 19.2.
#[test]
fn a_fetch_removes_the_other_cached_versions() {
    let tree = Tree::bare();
    release(&tree, "the-fetched-binary");
    tree.write("cache/bin/0.0.1/klin", "#!/bin/sh\necho stale\n");

    let run = fetch(&tree, &["--version"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        held(&tree),
        PINNED,
        "the cache holds more than the pinned version"
    );
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
    let written = tree.write("release/stage/klin", &format!("#!/bin/sh\necho {says}\n"));
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

fn fetch(tree: &Tree, args: &[&str]) -> Ran {
    let base = format!("file://{}", tree.path("release").display());
    let cache = tree.path("cache").display().to_string();
    ran(
        &at(WRAPPER).display().to_string(),
        args,
        tree.root(),
        &[("KLIN_RELEASE_BASE_URL", &base), ("KLIN_CACHE_DIR", &cache)],
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

fn json(relative: &str) -> serde_json::Value {
    match serde_json::from_str(&text(relative)) {
        Ok(held) => held,
        Err(why) => panic!("{relative} is not JSON: {why}"),
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
