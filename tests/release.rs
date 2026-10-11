mod harness;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use harness::{AGENT, Run, SESSION_START, STOP, Tree, feed_as_with};

const NEWER: &str = "https://github.com/brajevicm/klin/releases/tag/v99.0.0";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One test at a time in this file. A pipe the harness opens while another thread starts a
/// process can leak into that process and its detached child, which then holds the pipe open
/// for as long as its `curl` runs, and a timing test would read that as klin waiting.
static ALONE: Mutex<()> = Mutex::new(());

/// A klin the installer placed: a copy of the binary under test with a `klin-update` beside it
/// that records each run, a `curl` on PATH that answers `answer` and records each call, and a
/// cache of its own.
struct Installed {
    home: Tree,
    tree: Tree,
    plugin: bool,
    _alone: MutexGuard<'static, ()>,
}

impl Installed {
    fn new(answer: &str) -> Installed {
        let alone = ALONE.lock().unwrap_or_else(PoisonError::into_inner);
        let home = Tree::bare();
        let klin = home.path("installed/klin");
        fs::create_dir_all(home.path("installed")).expect("installed directory");
        fs::create_dir_all(home.path("user")).expect("home directory");
        if fs::hard_link(harness::binary(), &klin).is_err() {
            fs::copy(harness::binary(), &klin).expect("copy klin");
        }
        let updated = home.at("updated");
        executable(
            &home,
            "installed/klin-update",
            &format!("#!/bin/sh\necho updated >> '{updated}'\n"),
        );
        let calls = home.at("calls");
        executable(
            &home,
            "bin/curl",
            &format!("#!/bin/sh\necho called >> '{calls}'\n{answer}\n"),
        );
        let tree = Tree::new();
        tree.write("klin.json", "{}\n");
        tree.write("README.md", "# readme\n");
        tree.base();
        Installed {
            home,
            tree,
            plugin: false,
            _alone: alone,
        }
    }

    /// The same binary as a plugin's wrapper runs it: no updater beside it, and `KLIN_PLUGIN`.
    fn plugin(answer: &str) -> Installed {
        let installed = Installed::new(answer);
        installed.home.remove("installed/klin-update");
        Installed {
            plugin: true,
            ..installed
        }
    }

    fn run(&self, extra: &[(&str, &str)], args: &[&str], stdin: &str) -> Run {
        let path = format!("{}:/usr/bin:/bin", self.home.at("bin"));
        let cache = self.home.at("cache");
        let user = self.home.at("user");
        let mut environment = vec![
            ("PATH", path.as_str()),
            ("KLIN_CACHE_DIR", cache.as_str()),
            ("HOME", user.as_str()),
            ("CI", ""),
            ("KLIN_NO_UPDATE_CHECK", ""),
            ("KLIN_AUTO_UPDATE", ""),
            ("KLIN_PLUGIN", if self.plugin { "1" } else { "" }),
        ];
        environment.extend_from_slice(extra);
        feed_as_with(
            &self.home.at("installed/klin"),
            self.tree.root(),
            &environment,
            args,
            stdin,
        )
    }

    fn status(&self, extra: &[(&str, &str)]) -> Run {
        self.run(extra, &["status"], "")
    }

    fn stop(&self) -> Run {
        self.run(&[], AGENT, STOP)
    }

    fn session(&self, extra: &[(&str, &str)]) -> Run {
        self.run(extra, AGENT, SESSION_START)
    }

    /// A `klin status` that starts the day's ask, and the wait until its child cached an answer.
    fn warmed(&self) {
        self.status(&[]);
        let cache = self.home.path("cache/latest.json");
        until(|| fs::read_to_string(&cache).is_ok_and(|text| text.contains("99.0.0")));
    }

    fn fetched(&self) -> usize {
        lines(&self.home.path("calls"))
    }

    /// A record left by a klin that ran as `version` before this one, as an update leaves it.
    fn ran_before(&self, record: &str, version: &str) {
        self.home.write(
            &format!("cache/{record}"),
            &format!("{{\"seen\":\"{version}\"}}"),
        );
    }
}

fn executable(tree: &Tree, relative: &str, text: &str) {
    let path = tree.write(relative, text);
    if let Err(why) = fs::set_permissions(&path, fs::Permissions::from_mode(0o755)) {
        panic!("chmod {}: {why}", path.display());
    }
}

fn answers(url: &str) -> String {
    format!("printf '%s' '{url}'")
}

fn lines(file: &Path) -> usize {
    fs::read_to_string(file).map_or(0, |text| text.lines().count())
}

/// Wait up to five seconds for a detached child to do its work.
fn until(done: impl Fn() -> bool) {
    let start = Instant::now();
    while !done() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Long enough for a detached child that should not run to have run.
fn settle() {
    std::thread::sleep(Duration::from_millis(500));
}

/// A newer release names itself in `klin status` with the one command that installs it.
/// Spec 11.9.
#[test]
fn status_names_a_newer_release_and_klin_update() {
    let installed = Installed::new(&answers(NEWER));
    installed.warmed();

    let run = installed.status(&[]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(&format!(
            "klin 99.0.0 is available (you have {VERSION}). Run klin update."
        )),
        "{}",
        run.out
    );
    assert!(!run.says("klin setup"), "{}", run.out);
}

/// The status document carries the newest release and whether it is newer, so no reader
/// compares versions. Spec 11.7, 11.9.
#[test]
fn status_json_reports_the_latest_release_and_that_it_is_newer() {
    let installed = Installed::new(&answers(NEWER));
    installed.warmed();

    let run = installed.run(&[], &["status", "--json"], "");

    assert_eq!(run.code, 0, "{}", run.out);
    let klin = &run.json()["klin"];
    assert_eq!(klin["latest_known"], "99.0.0", "{}", run.out);
    assert_eq!(klin["update_available"], true, "{}", run.out);
}

/// A session start starts the day's ask and returns before a slow `curl` answers. Spec 11.9.
#[test]
fn session_start_asks_without_waiting_for_the_network() {
    let installed = Installed::new(&format!("sleep 3\n{}", answers(NEWER)));

    let start = Instant::now();
    let run = installed.session(&[]);
    let took = start.elapsed();

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(took < Duration::from_secs(2), "{took:?}");
    until(|| installed.fetched() == 1);
    assert_eq!(installed.fetched(), 1);
}

/// A Stop never runs `curl`, even with no cache. Spec 1, 11.9.
#[test]
fn stop_never_asks() {
    let installed = Installed::new(&answers(NEWER));

    let run = installed.stop();
    settle();

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(installed.fetched(), 0);
}

/// Under auto-update, a session start runs the updater beside the binary for a newer release,
/// once. Spec 11.9.
#[test]
fn auto_update_runs_the_updater_once_for_a_newer_release() {
    let installed = Installed::new(&answers(NEWER));
    installed.warmed();
    let updated = installed.home.path("updated");

    installed.session(&[("KLIN_AUTO_UPDATE", "1")]);
    until(|| lines(&updated) == 1);
    installed.session(&[("KLIN_AUTO_UPDATE", "1")]);
    settle();

    assert_eq!(lines(&updated), 1);
}

/// Without auto-update, nothing installs a release on its own. Spec 11.9.
#[test]
fn without_auto_update_nothing_installs() {
    let installed = Installed::new(&answers(NEWER));
    installed.warmed();

    installed.session(&[]);
    settle();

    assert!(!installed.home.path("updated").exists());
}

/// The person hears of a newer release once, through a Stop's notice. Spec 10.7, 11.9.
#[test]
fn stop_tells_the_person_of_a_newer_release_once() {
    let installed = Installed::new(&answers(NEWER));
    installed.warmed();

    let first = installed.stop();
    let second = installed.stop();

    assert_eq!(first.code, 0, "{}", first.out);
    assert!(first.says(r#"{"systemMessage":"#), "{}", first.out);
    assert!(
        first.says("klin 99.0.0 is available (you have"),
        "{}",
        first.out
    );
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(!second.says("99.0.0"), "{}", second.out);
}

/// After the binary replaced an older one, the next Stop tells the person once, with the
/// versions and the release notes. Spec 11.9.
#[test]
fn stop_tells_the_person_once_that_klin_updated() {
    let installed = Installed::new(&answers(NEWER));
    installed.ran_before("release-person.json", "0.0.1");

    let first = installed.stop();
    let second = installed.stop();

    assert!(
        first.says(&format!("klin updated to {VERSION} (from 0.0.1).")),
        "{}",
        first.out
    );
    assert!(first.says("Release notes: "), "{}", first.out);
    assert!(!first.says("klin setup"), "{}", first.out);
    assert!(!second.says("klin updated"), "{}", second.out);
}

/// An update that left a repository copy of klin's integration behind names `klin setup` in
/// that notice. Spec 11.9.
#[test]
fn the_update_notice_names_setup_when_a_repository_copy_is_behind() {
    let installed = Installed::new(&answers(NEWER));
    let setup = installed.run(&[], &["setup", "--host", "claude"], "");
    assert_eq!(setup.code, 0, "{}", setup.out);
    installed
        .tree
        .write(".claude/skills/klin/SKILL.md", "an earlier klin's skill\n");
    installed.ran_before("release-person.json", "0.0.1");

    let run = installed.stop();

    assert!(run.says("klin updated to"), "{}", run.out);
    assert!(
        run.says("Run klin setup and commit the changed hook files."),
        "{}",
        run.out
    );
}

/// A host the repository proves but never installed asks for nothing to commit after an
/// update. Spec 11.9.
#[test]
fn the_update_notice_does_not_name_setup_for_a_host_with_no_copy() {
    let installed = Installed::new(&answers(NEWER));
    fs::create_dir_all(installed.tree.path(".claude")).expect("claude directory");
    installed.ran_before("release-person.json", "0.0.1");

    let run = installed.stop();

    assert!(run.says("klin updated to"), "{}", run.out);
    assert!(!run.says("klin setup"), "{}", run.out);
}

/// A binary older than the one last seen is a downgrade, which is no update news. Spec 11.9.
#[test]
fn a_downgrade_is_not_an_update() {
    let installed = Installed::new(&answers(NEWER));
    installed.ran_before("release-person.json", "999.0.0");

    let first = installed.stop();
    let second = installed.stop();

    assert!(!first.says("klin updated"), "{}", first.out);
    assert!(!second.says("klin updated"), "{}", second.out);
}

/// Two commands at once start one ask, because the child that finds the lock taken does
/// nothing. Spec 11.9.
#[test]
fn two_commands_at_once_ask_once() {
    let installed = Installed::new(&format!("sleep 1\n{}", answers(NEWER)));

    installed.status(&[]);
    installed.status(&[]);
    until(|| installed.fetched() == 1);
    std::thread::sleep(Duration::from_millis(1500));

    assert_eq!(installed.fetched(), 1);
}

/// The hidden child keeps the off switches too, when a person runs it by hand. Spec 11.9.
#[test]
fn the_child_asks_and_updates_nothing_in_ci() {
    let installed = Installed::new(&answers(NEWER));

    installed.run(
        &[("CI", "true"), ("KLIN_AUTO_UPDATE", "1")],
        &["__release"],
        "",
    );

    assert_eq!(installed.fetched(), 0);
    assert!(!installed.home.path("updated").exists());
}

/// After the binary replaced an older one, the next session start tells the agent once which
/// version judges the repository now. Spec 11.9.
#[test]
fn session_start_tells_the_agent_once_that_klin_updated() {
    let installed = Installed::new(&answers(NEWER));
    installed.ran_before("release-agent.json", "0.0.1");

    let first = installed.session(&[]);
    let second = installed.session(&[]);

    assert!(
        first.says(&format!(
            "klin {VERSION} judges this repository now (it was 0.0.1)."
        )),
        "{}",
        first.out
    );
    assert!(!second.says("judges this repository"), "{}", second.out);
}

/// The agent hears of a newer release once, at a session start, and is told not to install it
/// itself, because the guard denies `klin update` to it. Spec 10.8, 11.9.
#[test]
fn session_start_tells_the_agent_of_a_newer_release_once() {
    let installed = Installed::new(&answers(NEWER));
    installed.warmed();

    let first = installed.session(&[]);
    let second = installed.session(&[]);

    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        first.says(
            "klin 99.0.0 is available. Tell the person to run klin update. Do not run it \
             yourself."
        ),
        "{}",
        first.out
    );
    assert!(!second.says("99.0.0"), "{}", second.out);
}

/// A person who turned the check off, and a CI job, see no ask, no update and no notice.
/// Spec 11.9.
#[test]
fn nothing_asks_updates_or_tells_when_the_check_is_off_or_in_ci() {
    for off in [("KLIN_NO_UPDATE_CHECK", "1"), ("CI", "true")] {
        let installed = Installed::new(&answers(NEWER));

        let status = installed.status(&[off]);
        let session = installed.session(&[off, ("KLIN_AUTO_UPDATE", "1")]);
        settle();

        assert_eq!(status.code, 0, "{}", status.out);
        assert_eq!(installed.fetched(), 0, "{off:?}");
        assert!(!installed.home.path("updated").exists(), "{off:?}");
        assert!(!status.says("is available"), "{off:?}: {}", status.out);
        assert!(!session.says("klin"), "{off:?}: {}", session.out);
    }
}

/// A plugin's binary has no updater beside it, so it neither asks nor tells. Spec 11.9.
#[test]
fn a_binary_without_an_updater_beside_it_does_not_ask() {
    let installed = Installed::new(&answers(NEWER));
    installed.home.remove("installed/klin-update");

    let run = installed.status(&[]);
    settle();

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(installed.fetched(), 0);
}

/// A failed ask changes nothing that `klin status` prints or returns, and counts as the day's
/// ask, so an offline machine does not ask on every command. Spec 11.9.
#[test]
fn a_failed_ask_leaves_status_as_it_was_and_is_not_retried_the_same_day() {
    let installed = Installed::new("echo 'curl: (6) Could not resolve host' >&2; exit 6");

    let failed = installed.status(&[]);
    until(|| installed.fetched() == 1);
    let again = installed.status(&[]);
    let off = installed.status(&[("KLIN_NO_UPDATE_CHECK", "1")]);
    settle();

    assert_eq!(installed.fetched(), 1);
    assert_eq!(failed.code, off.code, "{}", failed.out);
    assert_eq!(failed.out, off.out);
    assert_eq!(again.out, off.out);
}

/// `klin setup` also starts the day's ask. Spec 11.9.
#[test]
fn setup_asks_for_the_newest_release() {
    let installed = Installed::new(&answers(NEWER));

    let run = installed.run(&[], &["setup", "--host", "claude"], "");
    until(|| installed.fetched() == 1);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(installed.fetched(), 1);
}

/// `klin update --check` asks GitHub every time and waits, names both versions and installs
/// nothing. Spec 11.8.
#[test]
fn update_check_names_both_versions_and_installs_nothing() {
    let installed = Installed::new(&answers(NEWER));

    let first = installed.run(&[("CI", "true")], &["update", "--check"], "");
    installed.run(&[], &["update", "--check"], "");

    assert_eq!(first.code, 0, "{}", first.out);
    assert!(first.says(VERSION), "{}", first.out);
    assert!(first.says("99.0.0"), "{}", first.out);
    assert!(first.says(NEWER), "{}", first.out);
    assert_eq!(installed.fetched(), 2);
    assert!(!installed.home.path("updated").exists());
}

/// A plugin's binary tells the person to update the plugin, not to run `klin update`, which a
/// plugin does not carry. Spec 11.9.
#[test]
fn a_plugin_status_names_the_plugin_update() {
    let installed = Installed::plugin(&answers(NEWER));
    installed.warmed();

    let run = installed.status(&[]);

    assert!(
        run.says("klin 99.0.0 is available (you have"),
        "{}",
        run.out
    );
    assert!(
        run.says("Update the klin plugin in your host."),
        "{}",
        run.out
    );
    assert!(!run.says("Run klin update"), "{}", run.out);
}

/// Under Claude Code, the Stop names the plugin step and the auto-update switch, once.
/// Spec 11.9.
#[test]
fn a_plugin_stop_names_the_claude_code_plugin_step_once() {
    let installed = Installed::plugin(&answers(NEWER));
    installed.warmed();

    let first = installed.stop();
    let second = installed.stop();

    assert!(
        first.says("In Claude Code, run /plugin, open Marketplaces, select klin"),
        "{}",
        first.out
    );
    assert!(first.says("Enable auto-update there"), "{}", first.out);
    assert!(!second.says("99.0.0"), "{}", second.out);
}

/// The agent hears that the person updates the plugin. Spec 10.8, 11.9.
#[test]
fn a_plugin_session_start_tells_the_agent_to_ask_for_a_plugin_update() {
    let installed = Installed::plugin(&answers(NEWER));
    installed.warmed();

    let run = installed.session(&[]);

    assert!(
        run.says(
            "klin 99.0.0 is available. Tell the person to update the klin plugin. Do not do it \
             yourself."
        ),
        "{}",
        run.out
    );
}

/// A plugin's binary never runs an updater, auto-update or not, because its host owns the
/// plugin. Spec 11.9.
#[test]
fn a_plugin_never_updates_itself() {
    let installed = Installed::plugin(&answers(NEWER));
    installed.warmed();
    executable(
        &installed.home,
        "bin/klin-update",
        &format!(
            "#!/bin/sh\necho updated >> '{}'\n",
            installed.home.at("updated")
        ),
    );

    installed.session(&[("KLIN_AUTO_UPDATE", "1")]);
    settle();

    assert!(!installed.home.path("updated").exists());
}

/// An update that left a copy in the person's home behind names `klin setup --user` with its
/// host, and asks for no commit, because nothing in the repository changed. Spec 11.8, 11.9.
#[test]
fn the_update_notice_names_setup_user_when_a_home_copy_is_behind() {
    let installed = Installed::new(&answers(NEWER));
    let setup = installed.run(&[], &["setup", "--user", "--host", "claude"], "");
    assert_eq!(setup.code, 0, "{}", setup.out);
    installed.home.write(
        "user/.claude/skills/klin/SKILL.md",
        "an earlier klin's skill\n",
    );
    installed.ran_before("release-person.json", "0.0.1");

    let run = installed.stop();

    assert!(run.says("klin updated to"), "{}", run.out);
    assert!(
        run.says("Run klin setup --user --host claude."),
        "{}",
        run.out
    );
    assert!(!run.says("commit"), "{}", run.out);
}

/// `klin update --auto on` turns auto-update on without an environment variable, which a host
/// started outside a shell would not pass to its hooks, and `--auto off` turns it off.
/// Spec 11.8, 11.9.
#[test]
fn update_auto_turns_auto_update_on_and_off() {
    let installed = Installed::new(&answers(NEWER));
    installed.warmed();
    let updated = installed.home.path("updated");

    let off = installed.run(&[], &["update", "--auto", "off"], "");
    installed.session(&[]);
    settle();
    assert_eq!(off.code, 0, "{}", off.out);
    assert!(!updated.exists());

    let on = installed.run(&[], &["update", "--auto", "on"], "");
    let status = installed.run(&[], &["status", "--json"], "");
    installed.session(&[]);
    until(|| lines(&updated) == 1);

    assert_eq!(on.code, 0, "{}", on.out);
    assert!(on.says("auto-update is on"), "{}", on.out);
    assert_eq!(status.json()["klin"]["auto_update"], true, "{}", status.out);
    assert_eq!(lines(&updated), 1);
}

/// A plugin's binary refuses the setting and names the host's own plugin auto-update.
/// Spec 11.9.
#[test]
fn update_auto_on_a_plugin_names_the_hosts_switch() {
    let installed = Installed::plugin(&answers(NEWER));

    let run = installed.run(&[], &["update", "--auto", "on"], "");

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("its host updates the plugin"), "{}", run.out);
    assert!(!installed.home.path("cache/auto-update").exists());
}
