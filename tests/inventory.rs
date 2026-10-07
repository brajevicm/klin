mod harness;

use harness::Tree;

fn tree_with_a_test() -> Tree {
    let tree = Tree::new();
    tree.write("src/foo.py", "x = 1\n");
    tree.write("tests/test_foo.py", "def test_foo():\n    assert True\n");
    tree.base();
    tree
}

#[test]
fn a_deleted_test_file_blocks_the_stop_naming_the_path() {
    let tree = tree_with_a_test();
    tree.remove("tests/test_foo.py");
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("tests/test_foo.py:0  missing 1, was missing 0"),
        "{}",
        run.out
    );
    assert!(run.says(QUESTION), "{}", run.out);
}

#[test]
fn a_new_file_and_a_file_outside_the_roots_do_not_fail() {
    let tree = tree_with_a_test();
    tree.write("tests/test_bar.py", "def test_bar():\n    assert True\n");
    tree.remove("src/foo.py");
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_accepted_entry_keyed_by_the_path_holds_it() {
    let tree = tree_with_a_test();
    tree.remove("tests/test_foo.py");
    tree.write(
        "klin.json",
        r#"{"accepted": [{"gate": "inventory", "file": "tests/test_foo.py",
                          "text": "test file", "missing": 1}]}"#,
    );
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_deleted_test_whose_subject_went_too_is_a_note() {
    let tree = Tree::new();
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
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("src/foo_test.go  its subject src/foo.go went too"),
        "{}",
        run.out
    );
    assert!(run.says("a test_ or spec_ prefix"), "{}", run.out);
    assert!(
        run.says("(1 file(s) found, 0 measured, 0 excluded, 0 unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn with_no_configuration_the_derived_test_roots_are_judged_under_changed() {
    let tree = Tree::new();
    tree.write("README.md", "a tree with no klin.json\n");
    tree.write("src/foo.py", "x = 1\n");
    tree.write("tests/test_foo.py", "def test_foo():\n    assert True\n");
    tree.base();
    tree.remove("tests/test_foo.py");
    let run = tree.run(&["check", "inventory", "--changed"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("tests/test_foo.py:0  test file"), "{}", run.out);
}

#[test]
fn each_vanished_file_is_its_own_finding() {
    let tree = Tree::new();
    tree.write("tests/test_one.py", "def test_one():\n    pass\n");
    tree.write("tests/test_two.py", "def test_two():\n    pass\n");
    tree.base();
    tree.remove("tests/test_one.py");
    tree.remove("tests/test_two.py");
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("2 test site(s) got worse"), "{}", run.out);
    assert!(run.says("tests/test_one.py:0"), "{}", run.out);
    assert!(run.says("tests/test_two.py:0"), "{}", run.out);
}

/// A test beside its source is marked by its affix wherever it sits, so a Go package that keeps
/// its tests next to the code is judged with no configuration. Spec 8.2, ADR 0040.
#[test]
fn a_test_file_beside_its_source_is_judged_with_no_configuration() {
    let tree = Tree::new();
    tree.write("src/foo.go", "package src\n");
    tree.write("src/foo_test.go", "package src\n");
    tree.base();
    tree.remove("src/foo_test.go");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("src/foo_test.go:0  missing 1, was missing 0"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_under_a_test_root_at_the_tree_root_is_a_test_file() {
    let tree = Tree::new();
    tree.write("a_test.go", "package p\n");
    tree.write("sub/b_test.go", "package p\n");
    tree.write("notes.txt", "x\n");
    tree.base();
    tree.remove("notes.txt");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("notes.txt:0  missing 1, was missing 0"),
        "{}",
        run.out
    );
}

/// `except` takes a path out of the tests judged, and the base's own scope decides, so a
/// narrowing a person committed lets a deletion under it through. Spec 8.6.
#[test]
fn an_except_the_base_records_takes_a_path_out_of_the_tests_judged() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"inventory": {"except": "tests/legacy"}}"#);
    tree.write("tests/foo_test.go", "package tests\n");
    tree.write("tests/legacy/old_test.go", "package legacy\n");
    tree.base();
    tree.remove("tests/legacy/old_test.go");

    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("tests/legacy/old_test.go"), "{}", run.out);
}

#[test]
fn an_except_added_only_in_the_working_tree_does_not_let_a_deletion_through() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("tests/foo_test.go", "package tests\n");
    tree.write("tests/legacy/old_test.go", "package legacy\n");
    tree.base();
    tree.write("klin.json", r#"{"inventory": {"except": "tests/legacy"}}"#);
    tree.remove("tests/legacy/old_test.go");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("tests/legacy/old_test.go:0"), "{}", run.out);
}

#[test]
fn an_in_that_selects_no_test_is_a_tool_error() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"inventory": {"in": "docs"}}"#);
    tree.write("tests/foo_test.go", "package tests\n");
    tree.write("docs/guide.md", "A guide.\n");
    tree.base();

    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("\"inventory\" has an \"in\" scope with no applicable file"),
        "{}",
        run.out
    );
}

/// One test-recognition pattern of spec 8.2, with the two fixtures every pattern carries: a
/// deletion that must fail, and a move with the body unchanged that must stay green. Spec 17.
struct Pattern {
    marker: &'static str,
    file: &'static str,
    base: &'static str,
    stays: &'static str,
    moved_to: &'static str,
    moved: &'static str,
    site: &'static str,
}

const PATTERNS: &[Pattern] = &[
    Pattern {
        marker: "#[test]",
        file: "tests/suite.rs",
        base: "#[test]\nfn alpha() {\n    assert!(true);\n}\n\n#[test]\nfn beta() {\n    \
               assert!(1 == 1);\n}\n",
        stays: "#[test]\nfn alpha() {\n    assert!(true);\n}\n",
        moved_to: "tests/moved.rs",
        moved: "#[test]\nfn renamed() {\n    assert!(1 == 1);\n}\n",
        site: "fn beta() {",
    },
    Pattern {
        marker: "test_",
        file: "tests/test_foo.py",
        base: "def test_alpha():\n    assert True\n\n\ndef test_beta():\n    assert 1 == 1\n",
        stays: "def test_alpha():\n    assert True\n",
        moved_to: "tests/test_moved.py",
        moved: "def test_renamed():\n    assert 1 == 1\n",
        site: "def test_beta():",
    },
    Pattern {
        marker: "it(",
        file: "tests/foo.test.ts",
        base: "it(\"alpha\", () => {\n  check(1);\n});\n\nit(\"beta\", () => {\n  check(2);\n});\n",
        stays: "it(\"alpha\", () => {\n  check(1);\n});\n",
        moved_to: "tests/moved.test.ts",
        moved: "it(\"renamed\", () => {\n  check(2);\n});\n",
        site: "it(\"beta\", () => {",
    },
    Pattern {
        marker: "test(",
        file: "tests/bar.test.js",
        base: "test(\"alpha\", () => {\n  check(1);\n});\n\ntest(\"beta\", () => {\n  \
               check(2);\n});\n",
        stays: "test(\"alpha\", () => {\n  check(1);\n});\n",
        moved_to: "tests/moved.test.js",
        moved: "test(\"renamed\", () => {\n  check(2);\n});\n",
        site: "test(\"beta\", () => {",
    },
    Pattern {
        marker: "@Test",
        file: "tests/FooTest.java",
        base: "class FooTest {\n    @Test\n    void alpha() {\n        check(1);\n    }\n\n    \
               @Test\n    void beta() {\n        check(2);\n    }\n}\n",
        stays: "class FooTest {\n    @Test\n    void alpha() {\n        check(1);\n    }\n}\n",
        moved_to: "tests/MovedTest.java",
        moved: "class MovedTest {\n    @Test\n    void renamed() {\n        check(2);\n    }\n}\n",
        site: "void beta() {",
    },
    Pattern {
        marker: "func Test",
        file: "tests/foo_test.go",
        base: "package tests\n\nfunc TestAlpha(t *testing.T) {\n\tcheck(t, 1)\n}\n\nfunc \
               TestBeta(t *testing.T) {\n\tcheck(t, 2)\n}\n",
        stays: "package tests\n\nfunc TestAlpha(t *testing.T) {\n\tcheck(t, 1)\n}\n",
        moved_to: "tests/moved_test.go",
        moved: "package tests\n\nfunc TestRenamed(t *testing.T) {\n\tcheck(t, 2)\n}\n",
        site: "func TestBeta(t *testing.T) {",
    },
];

fn tree_with(pattern: &Pattern) -> Tree {
    let tree = Tree::new();
    tree.write(pattern.file, pattern.base);
    tree.base();
    tree
}

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const QUESTION: &str = "say why in your reply and stop again";

/// A stop in a repository that opted in. The hook reads nothing without `klin.json`, so a tree
/// that states no policy writes the empty one. ADR 0028.
fn stop(tree: &Tree) -> harness::Run {
    opted_in(tree);
    harness::feed(tree.root(), &["gate", "--hook"], A_STOP)
}

fn opted_in(tree: &Tree) {
    if !tree.path("klin.json").exists() {
        tree.write("klin.json", "{}");
    }
}

#[test]
fn deleting_a_test_function_from_a_file_that_stays_blocks_the_stop_and_asks_why() {
    for pattern in PATTERNS {
        let tree = tree_with(pattern);
        tree.write(pattern.file, pattern.stays);
        let run = stop(&tree);
        assert_eq!(run.code, 2, "{}: {}", pattern.marker, run.out);
        assert!(
            run.says(&format!("missing 1, was missing 0  {}", pattern.site)),
            "{}: {}",
            pattern.marker,
            run.out
        );
        assert!(run.says(QUESTION), "{}: {}", pattern.marker, run.out);
        assert!(!run.says("accepted"), "{}: {}", pattern.marker, run.out);
    }
}

#[test]
fn a_deleted_test_function_is_a_note_that_fails_nothing_outside_the_hook() {
    for pattern in PATTERNS {
        let tree = tree_with(pattern);
        tree.write("src/lib.rs", "pub fn kept() {}\n");
        tree.write(pattern.file, pattern.stays);
        let run = tree.run(&["check", "inventory"]);
        assert_eq!(run.code, 0, "{}: {}", pattern.marker, run.out);
        assert!(
            run.says("NOTE: 1 test site(s) the base holds went in this window:"),
            "{}: {}",
            pattern.marker,
            run.out
        );
        assert!(run.says(pattern.site), "{}: {}", pattern.marker, run.out);
    }
}

const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;

#[test]
fn the_stop_after_the_question_passes_and_leaves_a_green_verdict() {
    let tree = tree_with(&PATTERNS[0]);
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    let asked = stop(&tree);
    assert_eq!(asked.code, 2, "{}", asked.out);
    let again = harness::feed(tree.root(), &["gate", "--hook"], A_SECOND_STOP);
    assert_eq!(again.code, 0, "{}", again.out);
    assert_eq!(tree.field("verdict"), "green", "{}", again.out);
}

const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const A_SESSION: &str = r#"{"hook_event_name": "SessionStart"}"#;

fn second_stop(tree: &Tree) -> harness::Run {
    opted_in(tree);
    harness::feed(tree.root(), &["gate", "--hook"], A_SECOND_STOP)
}

/// What a stop hands the person through the host's `systemMessage`, or nothing.
fn told(run: &harness::Run) -> String {
    run.out
        .lines()
        .find_map(|line| {
            let held: serde_json::Value = serde_json::from_str(line).ok()?;
            held.get("systemMessage")?.as_str().map(str::to_string)
        })
        .unwrap_or_default()
}

#[test]
fn a_stop_that_lets_a_deletion_through_tells_the_person_which_test_went() {
    let tree = tree_with(&PATTERNS[0]);
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    assert_eq!(stop(&tree).code, 2);
    let again = second_stop(&tree);
    assert_eq!(again.code, 0, "{}", again.out);
    assert!(
        told(&again).contains("tests/suite.rs:7  fn beta() {"),
        "{}",
        again.out
    );
}

#[test]
fn a_prompt_between_two_stops_does_not_ask_about_the_same_test_again() {
    let tree = tree_with(&PATTERNS[0]);
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    assert_eq!(stop(&tree).code, 2);
    let prompt = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(prompt.code, 0, "{}", prompt.out);
    let after = stop(&tree);
    assert_eq!(after.code, 0, "{}", after.out);
    assert_eq!(tree.field("verdict"), "green", "{}", after.out);
}

#[test]
fn a_test_deleted_after_the_question_gets_its_own_question() {
    let tree = Tree::new();
    tree.write("tests/test_one.py", "def test_one():\n    assert True\n");
    tree.write("tests/test_two.py", "def test_two():\n    assert 2\n");
    tree.base();
    tree.remove("tests/test_one.py");
    assert_eq!(stop(&tree).code, 2);
    assert_eq!(harness::feed(tree.root(), &["radius"], A_PROMPT).code, 0);
    tree.remove("tests/test_two.py");
    let after = stop(&tree);
    assert_eq!(after.code, 2, "{}", after.out);
    assert!(
        after.says("tests/test_two.py:0  missing 1, was missing 0"),
        "{}",
        after.out
    );
    assert!(
        !after.says("tests/test_one.py:0  missing 1, was missing 0"),
        "{}",
        after.out
    );
}

fn three_tests() -> Tree {
    let tree = Tree::new();
    for name in ["one", "two", "three"] {
        tree.write(
            &format!("tests/test_{name}.py"),
            &format!("def test_{name}():\n    assert True\n"),
        );
    }
    tree.base();
    tree
}

#[test]
fn a_deleted_test_already_asked_about_is_no_reason_for_a_second_gate_block() {
    let tree = three_tests();
    tree.remove("tests/test_one.py");
    assert_eq!(stop(&tree).code, 2);

    tree.write("src/other.py", "y = 2\n");
    let after = second_stop(&tree);
    assert_eq!(after.code, 0, "{}", after.out);
    assert_eq!(tree.field("verdict"), "green", "{}", after.out);
}

#[test]
fn a_new_deletion_spends_a_gate_block_left_and_cannot_make_a_third() {
    let tree = three_tests();
    tree.remove("tests/test_one.py");
    let first = stop(&tree);
    assert_eq!(first.code, 2, "{}", first.out);
    assert!(first.says("gate block 1 of 2"), "{}", first.out);

    tree.remove("tests/test_two.py");
    let second = second_stop(&tree);
    assert_eq!(second.code, 2, "{}", second.out);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
    assert!(
        second.says("tests/test_two.py:0  missing 1, was missing 0"),
        "{}",
        second.out
    );

    tree.remove("tests/test_three.py");
    let third = second_stop(&tree);
    assert_eq!(third.code, 0, "{}", third.out);
    assert!(third.says("has blocked 2 stops"), "{}", third.out);
    assert!(
        third.says("tests/test_three.py:0  missing 1, was missing 0"),
        "{}",
        third.out
    );

    assert_eq!(harness::feed(tree.root(), &["radius"], A_PROMPT).code, 0);
    let later = stop(&tree);
    assert_eq!(later.code, 2, "{}", later.out);
    assert!(later.says("gate block 1 of 2"), "{}", later.out);
    assert!(
        later.says("tests/test_three.py:0  missing 1, was missing 0"),
        "{}",
        later.out
    );
    assert!(
        !later.says("tests/test_two.py:0  missing 1, was missing 0"),
        "{}",
        later.out
    );
}

#[test]
fn a_stop_whose_stamp_was_deleted_still_asks_about_a_deleted_test() {
    let tree = tree_with(&PATTERNS[0]);
    assert_eq!(harness::feed(tree.root(), &["radius"], A_SESSION).code, 0);
    tree.remove(".git/klin/turn");
    tree.git(&["update-ref", "-d", "refs/worktree/klin/turn"]);
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(QUESTION), "{}", run.out);
}

#[test]
fn an_accepted_entry_for_a_deleted_test_holds_it_under_strict_and_matches() {
    let tree = tree_with(&PATTERNS[0]);
    tree.write("src/lib.rs", "pub fn kept() {}\n");
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    tree.write(
        "klin.json",
        r#"{"accepted": [{"gate": "inventory", "file": "tests/suite.rs",
                          "text": "fn beta() {", "missing": 1}]}"#,
    );
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("matched nothing"), "{}", run.out);
}

#[test]
fn a_test_function_renamed_and_moved_with_its_body_unchanged_is_held() {
    for pattern in PATTERNS {
        let tree = tree_with(pattern);
        tree.write(pattern.file, pattern.stays);
        tree.write(pattern.moved_to, pattern.moved);
        let run = tree.run(&["check", "inventory"]);
        assert_eq!(run.code, 0, "{}: {}", pattern.marker, run.out);
    }
}

#[test]
fn an_accepted_entry_keyed_by_the_vanished_function_holds_it() {
    let tree = tree_with(&PATTERNS[0]);
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    tree.write(
        "klin.json",
        r#"{"accepted": [{"gate": "inventory", "file": "tests/suite.rs",
                          "text": "fn beta() {", "missing": 1}]}"#,
    );
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_deleted_test_function_whose_file_went_too_is_a_note() {
    let tree = Tree::new();
    tree.write("src/foo.go", "package src\n");
    tree.write(
        "src/foo_test.go",
        "package src\n\nfunc TestFoo(t *testing.T) {\n\tcheck(t)\n}\n",
    );
    tree.base();
    tree.remove("src/foo_test.go");
    tree.remove("src/foo.go");
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 deleted test function(s) whose file went in the same window"),
        "{}",
        run.out
    );
    assert!(
        run.says("src/foo_test.go:3  func TestFoo(t *testing.T) {  its file went too"),
        "{}",
        run.out
    );
}

#[test]
fn a_test_function_edited_as_it_moved_is_reported_as_gone() {
    let tree = tree_with(&PATTERNS[0]);
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    tree.write(
        "tests/moved.rs",
        "#[test]\nfn renamed() {\n    assert!(2 == 2);\n}\n",
    );
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("tests/suite.rs:7  missing 1, was missing 0  fn beta() {"),
        "{}",
        run.out
    );
}

#[test]
fn an_in_that_names_one_file_judges_the_functions_in_it() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"inventory": {"in": "tests/suite.rs"}}"#);
    tree.write("tests/suite.rs", PATTERNS[0].base);
    tree.base();
    tree.write("tests/suite.rs", PATTERNS[0].stays);
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("missing 1, was missing 0  fn beta() {"),
        "{}",
        run.out
    );
}

#[test]
fn a_function_whose_name_only_holds_a_marker_is_not_a_test_site() {
    let tree = Tree::new();
    tree.write(
        "tests/test_foo.py",
        "def test_alpha():\n    assert True\n\n\ndef helper(test_arg):\n    return it(test_arg)\n",
    );
    tree.base();
    tree.write("tests/test_foo.py", "def test_alpha():\n    assert True\n");
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("2 test site(s) the base holds"), "{}", run.out);
}

/// The test convention reads a declaration line, not a node kind, so a language whose
/// accessors are function nodes declares no test by holding one. Spec 8.2, ADR 0035.
#[test]
fn a_swift_accessor_is_not_a_test_site() {
    let tree = Tree::new();
    tree.write(
        "tests/Charge.swift",
        "var testValue: Int {\n    get { return 1 }\n}\n\nfunc test_charge() {\n    _ = 1\n}\n",
    );
    tree.base();
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 2 test site(s) the base holds, all held at the base"),
        "{}",
        run.out
    );
}

#[test]
fn a_test_file_no_grammar_reads_is_named_and_exits_two() {
    let tree = Tree::new();
    tree.write("tests/suite.rs", PATTERNS[0].base);
    tree.base();
    tree.write("tests/suite.rs", "%%% not rust %%%\n");
    let run = tree.run(&["check", "inventory"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("could not parse"), "{}", run.out);
    assert!(run.says("the Rust grammar rejected it"), "{}", run.out);
    assert!(
        run.says("(1 file(s) found, 0 measured, 0 excluded, 1 unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn a_test_file_no_grammar_read_at_the_base_either_is_a_note() {
    let tree = Tree::new();
    tree.write("tests/suite.rs", "%%% not rust %%%\n");
    tree.base();
    for args in [&["check", "inventory"][..], &["check", "inventory"]] {
        let run = tree.run(args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.out);
        assert!(
            run.says("NOTE: 1 file(s) the grammar could not parse"),
            "{args:?}: {}",
            run.out
        );
    }
}

#[test]
fn a_test_name_with_no_attribute_above_it_is_a_test_site() {
    let tree = Tree::new();
    tree.write(
        "tests/suite.rs",
        "pub fn test_alpha() {\n    let x = 1;\n}\n\npub fn test_beta() {\n    let y = 2;\n}\n",
    );
    tree.base();
    tree.write(
        "tests/suite.rs",
        "pub fn test_alpha() {\n    let x = 1;\n}\n",
    );
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("missing 1, was missing 0  pub fn test_beta() {"),
        "{}",
        run.out
    );
}

#[test]
fn deleting_a_tokio_test_from_a_file_that_stays_is_a_vanished_test_site() {
    let tree = Tree::new();
    tree.write(
        "tests/serve.rs",
        "#[tokio::test]\nasync fn alpha() {\n    serve(1).await;\n}\n\n#[tokio::test(\n    flavor = \
         \"multi_thread\",\n)]\nasync fn beta() {\n    serve(2).await;\n}\n",
    );
    tree.base();
    tree.write(
        "tests/serve.rs",
        "#[tokio::test]\nasync fn alpha() {\n    serve(1).await;\n}\n",
    );
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("missing 1, was missing 0  async fn beta() {"),
        "{}",
        run.out
    );
}

/// Spec 8.2: a stop that blocks records every finding it reported, so the report must name
/// every deletion that record holds. A window of more than a screenful still names each one,
/// and the stop after it lets all of them through.
#[test]
fn a_stop_that_blocks_names_every_deleted_test_it_then_lets_through() {
    let tree = Tree::new();
    for at in 0..25 {
        tree.write(
            &format!("tests/test_{at:02}.py"),
            &format!("def test_{at:02}():\n    assert True\n"),
        );
    }
    tree.base();
    for at in 0..25 {
        tree.remove(&format!("tests/test_{at:02}.py"));
    }
    let asked = stop(&tree);
    assert_eq!(asked.code, 2, "{}", asked.out);
    for at in 0..25 {
        let named = format!("tests/test_{at:02}.py:0  missing 1, was missing 0");
        assert!(asked.says(&named), "no {named} in: {}", asked.out);
    }
    let after = second_stop(&tree);
    assert_eq!(after.code, 0, "{}", after.out);
    assert_eq!(tree.field("verdict"), "green", "{}", after.out);
}

/// Spec 16.5: a hook run whose host event klin cannot read reports to stderr and exits 1. A
/// stop that only has something to tell takes that rule too, so a caller with no event reads
/// the note instead of a JSON object it cannot place.
#[test]
fn a_stop_with_no_host_event_writes_its_note_to_stderr_and_blocks_nothing() {
    let tree = tree_with(&PATTERNS[0]);
    tree.write(PATTERNS[0].file, PATTERNS[0].stays);
    assert_eq!(stop(&tree).code, 2);
    let after = harness::feed(tree.root(), &["gate", "--hook"], "");
    assert_eq!(after.code, 1, "{}", after.out);
    assert!(after.printed.is_empty(), "printed: {}", after.printed);
    assert!(after.says("went in this window"), "{}", after.out);
}

#[test]
fn an_oversized_test_source_preserves_the_named_resource_error() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "tests/test_bundle.js",
        &"test('works',()=>{});".repeat(4_000),
    );
    let run = tree.run(&["check", "inventory", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert!(report["errors"].as_array().expect("errors").iter().any(|error| {
        error["check"] == "inventory"
            && error["message"].as_str().is_some_and(|text| text.contains(
                "tests/test_bundle.js:1: source-line resource ceiling exceeded (84000 bytes; ceiling 65536 bytes)"
            ))
    }), "{}", run.out);
}
