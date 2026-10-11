mod harness;

use harness::Tree;

/// A klin that cannot ask GitHub for the newest release installs nothing and says why.
/// Spec 11.8.
#[test]
fn update_that_cannot_ask_github_says_so_and_installs_nothing() {
    let tree = Tree::bare();

    let run = tree.run_without_path(&["update"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("could not ask GitHub for the newest release"),
        "{}",
        run.out
    );
}
