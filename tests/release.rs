mod harness;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use harness::{AGENT, Run, STOP, Tree, feed_as_with};

const NEWER: &str = "https://github.com/brajevicm/klin/releases/tag/v99.0.0";

/// A klin the installer placed: a copy of the binary under test with a `klin-update` beside it,
/// a `curl` on PATH that answers `answer` and records each call, and a cache of its own.
struct Installed {
    home: Tree,
    tree: Tree,
}

impl Installed {
    fn new(answer: &str) -> Installed {
        let home = Tree::bare();
        let klin = home.path("installed/klin");
        fs::create_dir_all(home.path("installed")).expect("installed directory");
        if fs::hard_link(harness::binary(), &klin).is_err() {
            fs::copy(harness::binary(), &klin).expect("copy klin");
        }
        executable(&home, "installed/klin-update", "#!/bin/sh\nexit 0\n");
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
        Installed { home, tree }
    }

    fn run(&self, extra: &[(&str, &str)], args: &[&str], stdin: &str) -> Run {
        let path = format!("{}:/usr/bin:/bin", self.home.at("bin"));
        let cache = self.home.at("cache");
        let mut environment = vec![
            ("PATH", path.as_str()),
            ("KLIN_CACHE_DIR", cache.as_str()),
            ("CI", ""),
            ("KLIN_NO_UPDATE_CHECK", ""),
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

    fn fetched(&self) -> usize {
        fs::read_to_string(self.home.path("calls")).map_or(0, |calls| calls.lines().count())
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

/// A newer release names itself in `klin status`, with the two commands that install it.
/// Spec 11.9.
#[test]
fn status_names_a_newer_release_and_the_commands_that_install_it() {
    let installed = Installed::new(&answers(NEWER));

    let run = installed.status(&[]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(&format!(
            "klin 99.0.0 is available (you have {}). Run klin update, then klin setup.",
            env!("CARGO_PKG_VERSION")
        )),
        "{}",
        run.out
    );
}

/// The status document carries the newest release klin knows of. Spec 11.7, 11.9.
#[test]
fn status_json_reports_the_latest_known_release() {
    let installed = Installed::new(&answers(NEWER));

    let run = installed.run(&[], &["status", "--json"], "");

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.json()["klin"]["latest_known"], "99.0.0", "{}", run.out);
}

/// A person command asks GitHub at most once a day, and the next one reads the cache.
/// Spec 11.9.
#[test]
fn status_fetches_at_most_once_a_day() {
    let installed = Installed::new(&answers(NEWER));

    installed.status(&[]);
    let again = installed.status(&[]);

    assert_eq!(installed.fetched(), 1);
    assert!(again.says("klin 99.0.0 is available"), "{}", again.out);
}

/// The agent hears of a newer release once, and is told not to install it itself. Spec 11.9.
#[test]
fn stop_tells_the_agent_of_a_newer_release_once() {
    let installed = Installed::new(&answers(NEWER));
    installed.status(&[]);

    let first = installed.stop();
    let second = installed.stop();

    assert_eq!(first.code, 0, "{}", first.out);
    assert!(
        first.says("klin 99.0.0 is available. Tell the person to run klin update, then klin setup. Do not run it yourself."),
        "{}",
        first.out
    );
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(!second.says("99.0.0"), "{}", second.out);
}

/// A Stop never uses the network, even with no cache. Spec 1, 11.9.
#[test]
fn stop_never_fetches() {
    let installed = Installed::new(&answers(NEWER));

    let run = installed.stop();

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(installed.fetched(), 0);
    assert!(!run.says("99.0.0"), "{}", run.out);
}

/// A person who turned the check off, and a CI job, see no fetch and no notice. Spec 11.9.
#[test]
fn status_does_not_fetch_when_the_check_is_off_or_in_ci() {
    for off in [("KLIN_NO_UPDATE_CHECK", "1"), ("CI", "true")] {
        let installed = Installed::new(&answers(NEWER));

        let run = installed.status(&[off]);

        assert_eq!(run.code, 0, "{}", run.out);
        assert_eq!(installed.fetched(), 0, "{off:?}");
        assert!(!run.says("is available"), "{off:?}: {}", run.out);
    }
}

/// A plugin's binary has no updater beside it, so it neither fetches nor tells. Spec 11.9.
#[test]
fn a_binary_without_an_updater_beside_it_does_not_fetch() {
    let installed = Installed::new(&answers(NEWER));
    installed.home.remove("installed/klin-update");

    let run = installed.status(&[]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(installed.fetched(), 0);
}

/// A failed fetch changes nothing that `klin status` prints or returns. Spec 11.9.
#[test]
fn a_failed_fetch_leaves_status_as_it_was() {
    let installed = Installed::new("echo 'curl: (6) Could not resolve host' >&2; exit 6");

    let failed = installed.status(&[]);
    let off = installed.status(&[("KLIN_NO_UPDATE_CHECK", "1")]);

    assert_eq!(installed.fetched(), 1);
    assert_eq!(failed.code, off.code, "{}", failed.out);
    assert_eq!(failed.out, off.out);
}

/// `klin update --check` asks GitHub every time, names both versions and installs nothing.
/// Spec 11.8.
#[test]
fn update_check_names_both_versions_and_installs_nothing() {
    let installed = Installed::new(&answers(NEWER));
    executable(
        &installed.home,
        "installed/klin-update",
        &format!(
            "#!/bin/sh\necho installed >> '{}'\n",
            installed.home.at("updated")
        ),
    );

    let first = installed.run(&[("CI", "true")], &["update", "--check"], "");
    installed.run(&[], &["update", "--check"], "");

    assert_eq!(first.code, 0, "{}", first.out);
    assert!(first.says(env!("CARGO_PKG_VERSION")), "{}", first.out);
    assert!(first.says("99.0.0"), "{}", first.out);
    assert!(first.says(NEWER), "{}", first.out);
    assert_eq!(installed.fetched(), 2);
    assert!(!installed.home.path("updated").exists());
}
