use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::harness::Tree;

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
