mod harness;

use harness::Tree;

const CONFIG: &str = r#"{"inventory": [{"name": "tests", "path": "tests"}]}"#;

fn tree_with_a_test() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write("src/foo.py", "x = 1\n");
    tree.write("tests/test_foo.py", "def test_foo():\n    assert True\n");
    tree.base();
    tree
}

#[test]
fn a_deleted_test_file_fails_as_worsened_naming_the_path() {
    let tree = tree_with_a_test();
    tree.remove("tests/test_foo.py");
    let run = tree.run(&["gate", "--gate", "inventory"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 test file(s) got worse"), "{}", run.out);
    assert!(
        run.says("tests/test_foo.py:0  missing 1, was missing 0"),
        "{}",
        run.out
    );
}

#[test]
fn a_new_file_and_a_file_outside_the_roots_do_not_fail() {
    let tree = tree_with_a_test();
    tree.write("tests/test_bar.py", "def test_bar():\n    assert True\n");
    tree.remove("src/foo.py");
    let run = tree.run(&["gate", "--gate", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_accepted_entry_keyed_by_the_path_holds_it() {
    let tree = tree_with_a_test();
    tree.remove("tests/test_foo.py");
    tree.write(
        "klin.json",
        r#"{"inventory": [{"name": "tests", "path": "tests"}],
            "accepted": [{"gate": "inventory", "file": "tests/test_foo.py",
                          "text": "test file", "missing": 1}]}"#,
    );
    let run = tree.run(&["gate", "--gate", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_deleted_test_whose_subject_went_too_is_a_note() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"inventory": [{"name": "src", "path": "src", "pattern": "*_test.go"}]}"#,
    );
    tree.write(
        "src/foo.go",
        "package src
",
    );
    tree.write(
        "src/foo_test.go",
        "package src
",
    );
    tree.base();
    tree.remove("src/foo_test.go");
    tree.remove("src/foo.go");
    let run = tree.run(&["gate", "--gate", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("src/foo_test.go  its subject src/foo.go went too"),
        "{}",
        run.out
    );
    assert!(run.says("a test_ or spec_ prefix"), "{}", run.out);
}

#[test]
fn with_no_configuration_the_derived_test_roots_are_judged_under_changed() {
    let tree = Tree::new();
    tree.write("README.md", "a tree with no klin.json\n");
    tree.write("src/foo.py", "x = 1\n");
    tree.write("tests/test_foo.py", "def test_foo():\n    assert True\n");
    tree.base();
    tree.remove("tests/test_foo.py");
    let run = tree.run(&["gate", "--gate", "inventory", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("tests/test_foo.py:0  missing 1, was missing 0"),
        "{}",
        run.out
    );
}

#[test]
fn each_vanished_file_is_its_own_finding() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write("tests/test_one.py", "def test_one():\n    pass\n");
    tree.write("tests/test_two.py", "def test_two():\n    pass\n");
    tree.base();
    tree.remove("tests/test_one.py");
    tree.remove("tests/test_two.py");
    let run = tree.run(&["gate", "--gate", "inventory"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 test file(s) got worse"), "{}", run.out);
    assert!(run.says("tests/test_one.py:0"), "{}", run.out);
    assert!(run.says("tests/test_two.py:0"), "{}", run.out);
}

#[test]
fn a_pattern_limits_the_entry_to_the_basenames_it_matches() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"inventory": [{"name": "tests", "path": "tests", "pattern": "*_test.go"}]}"#,
    );
    tree.write("tests/foo_test.go", "package tests\n");
    tree.write("tests/helper.go", "package tests\n");
    tree.base();
    tree.remove("tests/helper.go");
    let run = tree.run(&["gate", "--gate", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
}
