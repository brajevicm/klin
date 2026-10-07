mod harness;

use harness::Tree;

const CONFIG: &str = r#"{
  "stubs": { "in": "src" }
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

    let run = tree.run(&["check", "stubs"]);
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

    let run = tree.run(&["check", "stubs"]);
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

    let run = tree.run(&["check", "stubs"]);
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

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:5"), "{}", run.out);
    assert!(!run.says("in tests skipped"), "{}", run.out);
}

#[test]
fn a_retired_project_pattern_is_rejected() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "patterns": { "placeholder":
               { "match": "PLACEHOLDER", "remedy": "write the code it stands for" } } } }"#,
    );
    tree.write("src/lib.rs", "fn f() {\n    PLACEHOLDER\n}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"patterns\""), "{}", run.out);
}

#[test]
fn a_retired_malformed_project_pattern_is_rejected_before_compilation() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "patterns": {"broken": "([unclosed"} } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"patterns\""), "{}", run.out);
}

#[test]
fn a_not_implemented_macro_is_a_stub_and_no_longer_an_escape() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "in": "src" }, "stubs": { "in": "src" } }"#,
    );
    tree.write(
        "src/lib.rs",
        "fn f() {\n    todo!()\n}\nfn g() {\n    unimplemented!()\n}\n",
    );

    let escapes = tree.run(&["check", "escapes"]);
    assert_eq!(escapes.code, 0, "{}", escapes.out);
    assert!(escapes.says("OK: 0 escape site(s)"), "{}", escapes.out);

    let stubs = tree.run(&["check", "stubs"]);
    assert_eq!(stubs.code, 1, "{}", stubs.out);
    assert!(stubs.says("2 new stub site(s)"), "{}", stubs.out);
    assert!(stubs.says("src/lib.rs:2"), "{}", stubs.out);
    assert!(stubs.says("src/lib.rs:5"), "{}", stubs.out);
}

/// Two lines per row: the one that fails, then one the row must leave alone.
fn rows() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "stubs": { "in": "src" } }"#);
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

    let run = tree.run(&["check", "stubs"]);
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

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:2  comment marker"), "{}", run.out);
}

#[test]
fn skip_test_idioms_is_refused_because_a_stub_in_a_test_is_a_stub() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "in": "src", "skip_test_idioms": true } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("skip_test_idioms"), "{}", run.out);
    assert!(run.says("it reads only: in, except"), "{}", run.out);
}

/// The body shapes of #114. Every shape is judged by the function walk, so a config that names
/// the language it is written in reads it.
fn shaped() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "stubs": { "in": "src" } }"#);
    tree
}

#[test]
fn a_pass_body_fails_and_the_same_declaration_with_a_body_stays_green() {
    let tree = shaped();
    tree.write("src/a.py", "def save(key):\n    pass\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/a.py:1  pass body — implement the body"),
        "{}",
        run.out
    );

    tree.write("src/a.py", "def save(key):\n    write(key)\n");
    let rewritten = tree.run(&["check", "stubs"]);
    assert_eq!(rewritten.code, 0, "{}", rewritten.out);
}

#[test]
fn an_elided_body_fails_and_a_comment_that_elides_nothing_stays_green() {
    let tree = shaped();
    tree.write(
        "src/a.rs",
        "fn f() {\n    // ...\n}\nfn g() {\n    // rest of the code\n}\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/a.rs:1  elided body — implement the body"),
        "{}",
        run.out
    );
    assert!(run.says("src/a.rs:4  elided body"), "{}", run.out);

    tree.write(
        "src/a.rs",
        "fn f() {\n    // the caller holds the lock\n    work();\n}\n",
    );
    let written = tree.run(&["check", "stubs"]);
    assert_eq!(written.code, 0, "{}", written.out);
}

#[test]
fn an_empty_test_body_fails_and_a_test_rewritten_with_the_same_declaration_stays_green() {
    let tree = shaped();
    tree.write("src/a.rs", "#[test]\nfn test_it() {}\nfn main() {}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/a.rs:2  empty test — write the assertion the test name promises"),
        "{}",
        run.out
    );
    assert!(run.says("1 new stub site(s)"), "{}", run.out);

    tree.write(
        "src/a.rs",
        "#[test]\nfn test_it() {\n    assert!(true);\n}\nfn main() {}\n",
    );
    let written = tree.run(&["check", "stubs"]);
    assert_eq!(written.code, 0, "{}", written.out);
}

#[test]
fn an_empty_body_under_a_multi_line_tokio_test_is_an_empty_test() {
    let tree = shaped();
    tree.write(
        "src/a.rs",
        "#[tokio::test(\n    flavor = \"multi_thread\",\n)]\nasync fn serves() {}\nfn main() {}\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.rs:4  empty test"), "{}", run.out);
    assert!(run.says("1 new stub site(s)"), "{}", run.out);
}

#[test]
fn an_empty_test_body_a_call_declares_fails() {
    let tree = shaped();
    tree.write(
        "src/a.ts",
        "it(\"does nothing\", () => {});\nfunction empty() {}\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.ts:1  empty test"), "{}", run.out);
    assert!(run.says("1 new stub site(s)"), "{}", run.out);
}

#[test]
fn a_body_shape_the_base_holds_is_held() {
    let tree = shaped();
    tree.write("src/a.py", "def save(key):\n    pass\n");
    tree.base();

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 1 stub site(s) in the tree, all held at the base"),
        "{}",
        run.out
    );
}

#[test]
fn pass_on_an_exception_class_and_on_an_abstract_declaration_is_not_a_stub() {
    let tree = shaped();
    tree.write(
        "src/a.py",
        "from abc import ABC, abstractmethod\nfrom typing import Protocol\n\n\n\
         class Missing(Exception):\n    pass\n\n\n\
         class Store(ABC):\n    @abstractmethod\n    def put(self, key):\n        pass\n\n\n\
         class Reader(Protocol):\n    def read(self) -> str:\n        pass\n",
    );
    tree.write("src/a.rs", "trait Store {\n    fn put(&self);\n}\n");
    tree.write(
        "src/a.ts",
        "interface Store {\n  put(key: string): void;\n}\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 stub site(s)"), "{}", run.out);
}

#[test]
fn a_marker_and_a_body_shape_on_one_declaration_line_are_two_sites() {
    let tree = shaped();
    tree.write("src/a.py", "def save(key):  # TODO write it\n    pass\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new stub site(s)"), "{}", run.out);
    assert!(
        run.says("src/a.py:1  comment marker, new on line 1"),
        "{}",
        run.out
    );
    assert!(run.says("src/a.py:1  pass body"), "{}", run.out);
}

#[test]
fn a_callback_on_the_line_of_a_test_declaration_is_not_an_empty_test() {
    let tree = shaped();
    tree.write(
        "src/a.ts",
        "it(\"logs\", () => withLogger(() => {}, run));\nit(\"returns\", () => value);\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 stub site(s)"), "{}", run.out);
}

#[test]
fn a_decorator_or_a_base_whose_text_only_spells_a_marker_does_not_hide_a_pass_body() {
    let tree = shaped();
    tree.write(
        "src/a.py",
        "@app.route(\"/overload\")\ndef handler():\n    pass\n\n\n\
         class Repo(StoreABC):\n    def put(self, key):\n        pass\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.py:2  pass body"), "{}", run.out);
    assert!(run.says("src/a.py:7  pass body"), "{}", run.out);
    assert!(run.says("2 new stub site(s)"), "{}", run.out);
}

#[test]
fn retired_custom_patterns_cannot_replace_the_built_in_detector() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "patterns": { "banned": "NOCOMMIT" } } }"#,
    );
    tree.write("src/a.py", "def save(key):\n    pass\n");
    tree.write("src/a.rs", "fn f() {\n    // ...\n}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"patterns\""), "{}", run.out);
}

#[test]
fn a_tree_with_no_stubs_section_gates_its_markers_over_what_the_survey_found() {
    let tree = Tree::new();
    tree.write("src/lib.rs", "fn f() {\n    todo!()\n}\n");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  stubs"), "{}", run.out);
    assert!(run.says("src/lib.rs:2  not implemented"), "{}", run.out);
    assert!(!run.says("derived: stubs"), "{}", run.out);
}

#[test]
fn unsupported_languages_are_ignored_by_the_built_in_detector() {
    let tree = Tree::new();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.write("src/app.swift", "func f() {}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("1 file(s) found, 1 measured"), "{}", run.out);
    assert!(!run.says("src/app.swift"), "{}", run.out);
}

#[test]
fn stubs_runs_automatically_when_the_tree_has_no_supported_file() {
    let tree = Tree::new();
    tree.write("src/app.swift", "func f() {}\n");

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("escapes — runs"), "{}", run.out);
    assert!(run.says("stubs — runs"), "{}", run.out);
}

#[test]
fn a_compact_scope_limits_the_built_in_detector() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "stubs": { "in": "src" } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.write("lib/todo.rs", "fn f() {\n    todo!()\n}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("lib/todo.rs"), "{}", run.out);
}

#[test]
fn a_typo_fix_inside_an_existing_marker_is_held() {
    let tree = tree();
    tree.write("src/lib.rs", "// FIXME: hadnle the error\nfn f() {}\n");
    tree.base();
    tree.write("src/lib.rs", "// FIXME: handle the error\nfn f() {}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("all held at the base"), "{}", run.out);
}

#[test]
fn a_new_marker_in_a_file_that_holds_one_raises_its_count_and_names_the_new_line() {
    let tree = tree();
    tree.write("src/lib.rs", "// FIXME: handle the error\nfn f() {}\n");
    tree.base();
    tree.write(
        "src/lib.rs",
        "// FIXME: handle the error\nfn f() {}\n\n// TODO: log it\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 stub site(s) got worse"), "{}", run.out);
    assert!(
        run.says("src/lib.rs:4  comment marker x2, new on line 4"),
        "{}",
        run.out
    );
}

#[test]
fn every_marker_line_the_base_file_lacks_is_named() {
    let tree = tree();
    tree.write("src/lib.rs", "// TODO: one\nfn f() {}\n");
    tree.base();
    tree.write(
        "src/lib.rs",
        "// TODO: uno\nfn f() {}\n// TODO: two\n// TODO: three\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/lib.rs:1  comment marker x3, new on lines 1, 3, 4"),
        "{}",
        run.out
    );
}

#[test]
fn an_edited_not_implemented_line_is_a_new_site() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {\n    todo!()\n}\n");
    tree.base();
    tree.write("src/lib.rs", "fn f() {\n    todo!(\"later\")\n}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new stub site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:2  not implemented"), "{}", run.out);
}

#[test]
fn a_marker_moved_within_a_file_is_held_and_one_moved_to_another_file_is_new_there() {
    let tree = tree();
    tree.write("src/a.rs", "// TODO: split this\nfn f() {}\n");
    tree.write("src/b.rs", "fn g() {}\n");
    tree.base();
    tree.write("src/a.rs", "fn f() {}\n// TODO: split this\n");

    let moved = tree.run(&["check", "stubs"]);
    assert_eq!(moved.code, 0, "{}", moved.out);

    tree.write("src/a.rs", "fn f() {}\n");
    tree.write("src/b.rs", "// TODO: split this\nfn g() {}\n");
    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new stub site(s)"), "{}", run.out);
    assert!(run.says("src/b.rs:1  comment marker"), "{}", run.out);
}

#[test]
fn an_accepted_marker_entry_names_the_row_and_holds_at_its_count() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "accepted": [{"gate": "stubs", "file": "src/lib.rs", "text": "comment marker", "count": 2}],
             "stubs": { "in": "src" } }"#,
    );
    tree.write("src/lib.rs", "// TODO: one\n// FIXME: two\nfn f() {}\n");

    let held = tree.run(&["check", "stubs"]);
    assert_eq!(held.code, 0, "{}", held.out);
    assert!(held.says("all on the accepted list"), "{}", held.out);

    tree.write(
        "src/lib.rs",
        "// TODO: one\n// FIXME: two\nfn f() {}\n// HACK: three\n",
    );
    let worse = tree.run(&["check", "stubs"]);
    assert_eq!(worse.code, 1, "{}", worse.out);
    assert!(worse.says("got worse"), "{}", worse.out);
}

#[test]
fn an_accepted_entry_for_a_line_that_held_a_marker_and_a_code_stub_holds_the_code_stub() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "accepted": [
               {"gate": "stubs", "file": "src/a.py", "text": "def save(key):  # TODO write it", "count": 2},
               {"gate": "stubs", "file": "src/a.rs", "text": "todo!() // TODO handle errors", "count": 2}],
             "stubs": { "in": "src" } }"#,
    );
    tree.write("src/a.py", "def save(key):  # TODO write it\n    pass\n");
    tree.write(
        "src/a.rs",
        "fn f() {\n    todo!() // TODO handle errors\n}\n",
    );

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new stub site(s)"), "{}", run.out);
    assert!(
        run.says("src/a.py:1  comment marker, new on line 1"),
        "{}",
        run.out
    );
    assert!(
        run.says("src/a.rs:2  comment marker, new on line 2"),
        "{}",
        run.out
    );
    assert!(!run.says("matched nothing this run"), "{}", run.out);
}

#[test]
fn an_accepted_entry_for_a_mixed_line_the_base_holds_is_stale_and_the_base_holds_both_sites() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "accepted": [
               {"gate": "stubs", "file": "src/a.py", "text": "def save(key):  # TODO write it", "count": 2},
               {"gate": "stubs", "file": "src/a.rs", "text": "todo!() // TODO handle errors", "count": 2}],
             "stubs": { "in": "src" } }"#,
    );
    tree.write("src/a.py", "def save(key):  # TODO write it\n    pass\n");
    tree.write(
        "src/a.rs",
        "fn f() {\n    todo!() // TODO handle errors\n}\n",
    );
    tree.base();

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("all held at the base"), "{}", run.out);
    assert!(
        run.says("NOTE: 2 accepted entries matched nothing this run"),
        "{}",
        run.out
    );
    assert!(
        run.says("the accepted list holds 2 entries that matched nothing"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_of_two_hundred_thousand_distinct_markers_is_judged_in_seconds() {
    let tree = tree();
    let markers: String = (0..200_000)
        .map(|at| format!("// TODO item {at}\n"))
        .collect();
    tree.write("src/lib.rs", &markers);
    tree.base();
    tree.write("src/lib.rs", &format!("{markers}// TODO one more\n"));

    let started = std::time::Instant::now();
    let run = tree.run(&["check", "stubs"]);
    assert!(started.elapsed().as_secs() < 30, "{}", run.out);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/lib.rs:200001  comment marker x200001, new on line 200001"),
        "{}",
        run.out
    );
}

#[test]
fn an_oversized_source_preserves_the_named_resource_error_in_stubs() {
    let tree = tree();
    tree.write("src/bundle.js", &"function bundled(){};".repeat(4_000));
    let message =
        "src/bundle.js:1: source-line resource ceiling exceeded (84000 bytes; ceiling 65536 bytes)";
    let run = tree.run(&["check", "stubs", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert!(
        report["errors"]
            .as_array()
            .expect("errors")
            .iter()
            .any(|error| {
                error["check"] == "stubs"
                    && error["message"]
                        .as_str()
                        .is_some_and(|text| text.contains(message))
            }),
        "{}",
        run.out
    );
    let direct = tree.run(&["check", "stubs"]);
    assert_eq!(direct.code, 2, "{}", direct.out);
    assert!(direct.says(message), "{}", direct.out);
}
