use crate::harness::Tree;

#[test]
fn version_prints_the_version_the_release_was_built_from() {
    let tree = Tree::bare();
    let run = tree.run(&["--version"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(env!("CARGO_PKG_VERSION")),
        "no version in: {}",
        run.out
    );
}
