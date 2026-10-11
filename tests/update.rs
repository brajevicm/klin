mod harness;

use harness::Tree;

/// A binary the plugin wrapper fetched has no updater beside it, and the answer names the route
/// that has one. Spec 19.5.
#[test]
fn update_without_an_updater_says_so_and_names_the_installer() {
    let tree = Tree::bare();

    let run = tree.run_without_path(&["update"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("klin-installer.sh"), "{}", run.out);
}
