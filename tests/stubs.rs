mod harness;

use harness::Tree;

const CONFIG: &str = r#"{
  "project": "t",
  "stubs": { "roots": ["src"], "languages": ["rust"] }
}"#;

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree
}

#[test]
fn a_new_marker_fails_with_the_site_and_its_remedy() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {\n    todo!()\n}\n");

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 1 new stub site(s)"), "{}", run.out);
    assert!(
        run.says("src/lib.rs:2  not implemented — implement the body"),
        "{}",
        run.out
    );
}

#[test]
fn a_marker_the_base_holds_is_held() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {\n    todo!()\n}\n");
    tree.base();

    let run = tree.run(&["stubs", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 1 stub site(s) in the tree, all held at the base"),
        "{}",
        run.out
    );
}

#[test]
fn a_marker_inside_a_string_literal_is_not_a_stub() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        "fn f() -> &'static str {\n    let note = \"todo!() and // TODO\";\n    note\n}\n",
    );

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/lib.rs"), "{}", run.out);
}

#[test]
fn a_marker_inside_an_inline_test_module_is_a_stub() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        todo!()\n    }\n}\n",
    );

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:5"), "{}", run.out);
    assert!(!run.says("in inline Rust tests skipped"), "{}", run.out);
}

#[test]
fn a_project_pattern_is_matched_with_its_own_remedy() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "roots": ["src"], "languages": ["rust"],
             "patterns": { "placeholder":
               { "match": "PLACEHOLDER", "remedy": "write the code it stands for" } } } }"#,
    );
    tree.write("src/lib.rs", "fn f() {\n    PLACEHOLDER\n}\n");

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/lib.rs:2  placeholder — write the code it stands for"),
        "{}",
        run.out
    );
}

#[test]
fn a_project_pattern_that_is_not_a_regex_is_refused_naming_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "roots": ["src"], "patterns": {"broken": "([unclosed"} } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("broken"), "{}", run.out);
    assert!(run.says("not a regular expression"), "{}", run.out);
}

#[test]
fn a_not_implemented_macro_is_a_stub_and_no_longer_an_escape() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["rust"] },
             "stubs": { "roots": ["src"], "languages": ["rust"] } }"#,
    );
    tree.write(
        "src/lib.rs",
        "fn f() {\n    todo!()\n}\nfn g() {\n    unimplemented!()\n}\n",
    );

    let escapes = tree.run(&["escapes"]);
    assert_eq!(escapes.code, 0, "{}", escapes.out);
    assert!(escapes.says("OK: 0 escape site(s)"), "{}", escapes.out);

    let stubs = tree.run(&["stubs"]);
    assert_eq!(stubs.code, 1, "{}", stubs.out);
    assert!(stubs.says("2 new stub site(s)"), "{}", stubs.out);
    assert!(stubs.says("src/lib.rs:2"), "{}", stubs.out);
    assert!(stubs.says("src/lib.rs:5"), "{}", stubs.out);
}

/// Two lines per row: the one that fails, then one the row must leave alone.
fn rows() -> Tree {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "roots": ["src"],
             "languages": ["rust", "python", "go", "typescript"] } }"#,
    );
    tree.write(
        "src/a.rs",
        "fn f() {\n    todo!()\n}\n// TODO: fix this\nlet todos = f();\n// a todo list\n",
    );
    tree.write(
        "src/a.py",
        "def f():\n    raise NotImplementedError\n# FIXME later\ndef g():\n    raise ValueError(\"nope\")\n# a fixme-free note\n",
    );
    tree.write(
        "src/a.go",
        "func f() {\n\tpanic(\"not implemented\")\n}\n// XXX revisit\nfunc g() {\n\tpanic(\"bad state\")\n}\n// see xxx below\n",
    );
    tree.write(
        "src/a.ts",
        "function f() {\n  throw new Error(\"not implemented\");\n}\n// HACK around it\nfunction g() {\n  throw new Error(\"bad input\");\n}\n// a hack of a name\n",
    );
    tree
}

#[test]
fn every_marker_row_fails_on_its_own_line_and_leaves_a_legitimate_one_alone() {
    let tree = rows();

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    for expected in [
        "src/a.rs:2  not implemented",
        "src/a.rs:4  comment marker",
        "src/a.py:2  not implemented",
        "src/a.py:3  comment marker",
        "src/a.go:2  not implemented",
        "src/a.go:4  comment marker",
        "src/a.ts:2  not implemented",
        "src/a.ts:4  comment marker",
    ] {
        assert!(run.says(expected), "missing {expected}\n{}", run.out);
    }
    assert!(run.says("8 new stub site(s)"), "{}", run.out);
}

#[test]
fn a_quoted_slash_ahead_of_a_comment_marker_does_not_hide_it() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        "fn f() {\n    get(\"https://example.com/x\"); // TODO handle the error\n}\n",
    );

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:2  comment marker"), "{}", run.out);
}

#[test]
fn skip_rust_tests_is_refused_because_a_stub_in_a_test_is_a_stub() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "roots": ["src"], "languages": ["rust"], "skip_rust_tests": true } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["stubs"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("skip_rust_tests"), "{}", run.out);
    assert!(run.says("Delete the key."), "{}", run.out);
}
