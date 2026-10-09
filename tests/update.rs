mod harness;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use harness::Tree;

/// `klin update` hands the run to `klin-update` and returns what it returned. Spec 19.5.
#[test]
fn update_runs_the_updater_on_path_and_passes_its_exit_code_through() {
    let tree = Tree::bare();
    let updater = tree.write(
        "bin/klin-update",
        "#!/bin/sh\necho updated to the newest release\nexit 3\n",
    );
    if let Err(why) = fs::set_permissions(&updater, fs::Permissions::from_mode(0o755)) {
        panic!("chmod {}: {why}", updater.display());
    }

    let run = tree.run_with(&[("PATH", &tree.at("bin"))], &["update"]);

    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("updated to the newest release"), "{}", run.out);
}

/// A binary the plugin wrapper fetched has no updater beside it, and the answer names the route
/// that has one. Spec 19.5.
#[test]
fn update_without_an_updater_says_so_and_names_the_installer() {
    let tree = Tree::bare();

    let run = tree.run_without_path(&["update"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("klin-installer.sh"), "{}", run.out);
}

/// After an update, an integration that is not `current` is named with `klin setup`, which
/// reconciles it. Spec 11.8.
#[test]
fn update_names_setup_when_an_integration_is_not_current() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    fs::create_dir_all(tree.path(".claude")).expect("claude directory");
    let updater = tree.write("bin/klin-update", "#!/bin/sh\nexit 0\n");
    if let Err(why) = fs::set_permissions(&updater, fs::Permissions::from_mode(0o755)) {
        panic!("chmod {}: {why}", updater.display());
    }

    let path = format!("{}:/usr/bin:/bin", tree.at("bin"));
    let stale = tree.run_with(&[("PATH", &path)], &["update"]);
    assert_eq!(stale.code, 0, "{}", stale.out);
    assert!(stale.says("run klin setup"), "{}", stale.out);
    assert!(stale.says("claude (project) is missing"), "{}", stale.out);

    assert_eq!(tree.run(&["setup", "--host", "claude"]).code, 0);
    let current = tree.run_with(&[("PATH", &path)], &["update"]);
    assert_eq!(current.code, 0, "{}", current.out);
    assert!(!current.says("klin setup"), "{}", current.out);
}

/// A copy in one person's home is reconciled by `klin setup --user`, so the hint names that
/// command and the host, and never sends the person to write the repository's files. Spec 11.8.
#[test]
fn update_names_setup_user_for_a_skill_without_hook_lines() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    let home = Tree::bare();
    let at = home.root().display().to_string();
    let setup = tree.run_with(
        &[("HOME", at.as_str())],
        &["setup", "--user", "--host", "claude"],
    );
    assert_eq!(setup.code, 0, "{}", setup.out);
    home.remove(".claude/settings.json");
    let updater = tree.write("bin/klin-update", "#!/bin/sh\nexit 0\n");
    if let Err(why) = fs::set_permissions(&updater, fs::Permissions::from_mode(0o755)) {
        panic!("chmod {}: {why}", updater.display());
    }

    let path = format!("{}:/usr/bin:/bin", tree.at("bin"));
    let run = tree.run_with(&[("PATH", &path), ("HOME", at.as_str())], &["update"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("run klin setup --user --host claude"),
        "{}",
        run.out
    );
    assert!(run.says("claude (user) is conflict"), "{}", run.out);
    assert!(!run.says("this repository's integration"), "{}", run.out);
}
