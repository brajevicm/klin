mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::Tree;

const CONFIG: &str = r#"{
  "escapes": { "in": "src" }
}"#;

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree
}

fn accepted(entries: &str) -> String {
    format!(
        r#"{{ "accepted": [{entries}],
             "escapes": {{ "in": "src" }} }}"#
    )
}

#[test]
fn a_site_the_base_does_not_hold_is_new_and_fails() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {\n    x.unwrap();\n}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 1 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:2"), "{}", run.out);
    assert!(run.says("x.unwrap();"), "{}", run.out);
}

#[test]
fn a_base_that_holds_no_source_makes_every_site_new() {
    let tree = tree();
    tree.write("src/lib.rs", "x.expect(\"boom\");\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
}

#[test]
fn the_sites_the_base_holds_pass() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        "fn f() {\n    a.unwrap();\n    b.expect(\"no\");\n    #[allow(dead_code)]\n}\n",
    );
    tree.base();

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 3 escape site(s) in the tree, all held at the base"),
        "{}",
        run.out
    );
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn a_ratcheted_count_that_rose_since_the_base_is_worse_and_fails() {
    let tree = tree();
    tree.write("src/lib.rs", text::DOUBLED);
    tree.base();
    tree.write("src/lib.rs", text::DOUBLED_TWICE);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("got worse — the ratchet only tightens"),
        "{}",
        run.out
    );
    assert!(run.says("unwrap x4, was unwrap x2"), "{}", run.out);
}

#[test]
fn a_count_that_fell_since_the_base_passes_with_nothing_to_say() {
    let tree = tree();
    tree.write("src/lib.rs", text::DOUBLED_TWICE);
    tree.base();
    tree.write("src/lib.rs", text::DOUBLED_PADDED);

    let run = tree.run(&["escapes", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn an_accepted_escape_holds_at_its_count_and_fails_above_it() {
    let tree = tree();
    tree.write(
        "klin.json",
        &accepted(&format!(
            r#"{{"gate": "escapes", "file": "src/lib.rs", "text": {:?}, "count": 1}}"#,
            text::ONE_SITE
        )),
    );
    tree.write("src/lib.rs", text::ONE);

    let held = tree.run(&["escapes"]);
    assert_eq!(held.code, 0, "{}", held.out);

    tree.write("src/lib.rs", text::TWO_ON_TWO_LINES);
    let worse = tree.run(&["escapes"]);
    assert_eq!(worse.code, 1, "{}", worse.out);
    assert!(worse.says("got worse"), "{}", worse.out);
}

#[test]
fn an_accepted_escape_that_matches_nothing_is_a_note_and_a_strict_failure() {
    let tree = tree();
    tree.write(
        "klin.json",
        &accepted(&format!(
            r#"{{"gate": "escapes", "file": "src/gone.rs", "text": {:?}, "count": 1}}"#,
            text::ONE_SITE
        )),
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("matched nothing this run"), "{}", run.out);
    assert!(run.says("src/gone.rs"), "{}", run.out);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
    assert!(strict.says("Delete the line"), "{}", strict.out);
}

#[test]
fn a_site_the_base_held_and_the_code_fixed_passes_with_nothing_to_say() {
    let tree = tree();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.base();
    tree.write("src/lib.rs", "fn f() {\n}\n");

    let run = tree.run(&["escapes", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn an_accepted_entry_that_names_no_value_is_a_tool_error() {
    let tree = tree();
    tree.write(
        "klin.json",
        &accepted(&format!(
            r#"{{"gate": "escapes", "file": "src/lib.rs", "text": {:?}}}"#,
            text::DOUBLED_SITE
        )),
    );
    tree.write("src/lib.rs", text::DOUBLED);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("does not give a number for count"), "{}", run.out);
}

#[test]
fn a_file_git_ignores_is_not_judged_because_the_base_holds_no_copy_of_it() {
    let tree = tree();
    tree.write(".gitignore", "src/generated.rs\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree.write("src/generated.rs", text::ONE);

    let whole = tree.run(&["escapes"]);
    assert_eq!(whole.code, 0, "{}", whole.out);
    assert!(!whole.says("src/generated.rs"), "{}", whole.out);

    let scoped = tree.run(&["escapes", "--only", "src/generated.rs"]);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
}

#[test]
fn one_file_is_read_once_and_counted_once() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/c.ts", "const c: any = 3;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/c.ts:1  any  "), "{}", run.out);
    assert!(!run.says("any x2"), "{}", run.out);
}

#[test]
fn a_line_carrying_two_escape_kinds_counts_both_under_the_first() {
    let tree = tree();
    tree.write("src/lib.rs", text::TWO_KINDS);
    tree.base();

    let run = tree.run(&["escapes", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 1 escape site(s)"), "{}", run.out);
}

#[test]
fn failure_output_names_the_fix_and_no_command_that_records_debt() {
    let tree = tree();
    tree.write("src/lib.rs", text::TWO_FILES);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("Fix what the escape hides"), "{}", run.out);
    assert!(!run.says("--write-baseline"), "{}", run.out);
    assert!(!run.says("Tighten"), "{}", run.out);
}

#[test]
fn only_restricts_findings_and_the_base_so_untouched_files_are_out_of_scope() {
    let tree = tree();
    tree.write("src/a.rs", text::ONE);
    tree.write("src/b.rs", text::OTHER);
    tree.base();
    tree.write("src/a.rs", text::FIXED_AND_BOTH);

    let all = tree.run(&["escapes"]);
    assert_eq!(all.code, 1, "{}", all.out);

    let only = tree.run(&["escapes", "--only", "src/b.rs"]);
    assert_eq!(only.code, 0, "{}", only.out);
}

#[test]
fn a_clean_quiet_run_prints_nothing() {
    let tree = tree();
    tree.write("src/lib.rs", text::ONE);
    tree.base();

    let run = tree.run(&["escapes", "--quiet", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{}", run.out);
}

fn spread() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write(
        "src/thing.py",
        "x = 1  # type: ignore\ny = 1  # noqa\nz = 1  # pragma: no cover\n@skip\ndef f():\n    try:\n        pass\n    except:\n        pass\n",
    );
    tree.write(
        "src/thing.ts",
        "const a: any = 1;\n// @ts-ignore\nconst b = a!.c;\nit.skip('x', () => {});\n// eslint-disable-next-line\n",
    );
    tree.write(
        "src/thing.swift",
        "let a = try! f()\nlet b = c as! D\nlet d = e!.f\n// swiftlint:disable all\nlet g: @unchecked Sendable = h\ntry XCTSkip(\"no\")\n",
    );
    tree.write(
        "src/thing.rs",
        "let a = b.unwrap();\nlet c = d.expect(\"no\");\nunsafe {\n}\n#[allow(dead_code)]\ntodo!();\n#[ignore]\n",
    );
    tree.write("src/thing.go", "// nolint\nt.Skip()\n");
    tree.write("src/thing.kt", "val a = b!!\n@Suppress(\"x\")\n@Ignore\n");
    tree.write("src/thing.java", "@SuppressWarnings(\"x\")\n@Disabled\n");
    tree.write("src/thing.rb", "# rubocop:disable Style\nskip\n");
    tree.write(
        "src/go.sh",
        "rm -f x || true\n# shellcheck disable=SC2086\n",
    );
    tree
}

#[test]
fn every_built_in_language_finds_and_names_its_escapes() {
    let tree = spread();

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    for expected in [
        "src/thing.py:1  type ignore",
        "src/thing.py:2  noqa",
        "src/thing.py:3  no cover",
        "src/thing.py:4  skipped test",
        "src/thing.py:8  bare except",
        "src/thing.ts:1  any",
        "src/thing.ts:2  ts-ignore",
        "src/thing.ts:3  non-null assertion",
        "src/thing.ts:4  skipped test",
        "src/thing.ts:5  eslint-disable",
        "src/thing.swift:1  force try",
        "src/thing.swift:2  force cast",
        "src/thing.swift:3  force unwrap",
        "src/thing.swift:4  swiftlint:disable",
        "src/thing.swift:5  unchecked Sendable",
        "src/thing.swift:6  skipped test",
        "src/thing.rs:1  unwrap",
        "src/thing.rs:2  expect",
        "src/thing.rs:3  unsafe",
        "src/thing.rs:5  allow",
        "src/thing.rs:7  skipped test",
        "src/thing.go:1  nolint",
        "src/thing.go:2  skipped test",
        "src/thing.kt:1  not-null assertion",
        "src/thing.kt:2  suppress",
        "src/thing.kt:3  skipped test",
        "src/thing.java:1  suppress warnings",
        "src/thing.java:2  skipped test",
        "src/thing.rb:1  rubocop:disable",
        "src/thing.rb:2  skipped test",
        "src/go.sh:1  errors ignored",
        "src/go.sh:2  shellcheck disable",
    ] {
        assert!(run.says(expected), "missing {expected}\n{}", run.out);
    }
}

#[test]
fn list_languages_prints_the_built_in_pattern_sets() {
    let tree = Tree::new();

    let run = tree.run(&["escapes", "--list-languages"]);
    assert_eq!(run.code, 0, "{}", run.out);
    for expected in [
        "go",
        "java",
        "kotlin",
        "python",
        "ruby",
        "rust",
        "shell",
        "swift",
        "typescript",
        "force unwrap",
        "bare except",
        "nolint",
    ] {
        assert!(run.says(expected), "missing {expected}\n{}", run.out);
    }
}

#[test]
fn a_site_shifted_by_an_edit_above_it_still_matches_by_its_line_text() {
    let tree = tree();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.base();
    tree.write(
        "src/lib.rs",
        "// a header\n// and more\nfn f() {\n    a.unwrap();\n}\n",
    );

    let run = tree.run(&["escapes", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn a_renamed_file_is_measured_at_its_old_path() {
    let tree = tree();
    tree.write("src/lib.rs", text::WRAPPED);
    tree.base();
    tree.git(&["mv", "src/lib.rs", "src/moved.rs"]);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/moved.rs"), "{}", run.out);
}

#[test]
fn the_same_line_twice_in_one_file_is_one_site_whose_count_ratchets() {
    let tree = tree();
    tree.write("src/lib.rs", text::ONE_PADDED);
    tree.base();
    tree.write("src/lib.rs", text::TWO_PADDED);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("unwrap x2, was unwrap"), "{}", run.out);
    assert!(!run.says("new escape site(s)"), "{}", run.out);
}

#[test]
fn a_retired_project_pattern_is_rejected() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "patterns": {"todo bang": "TODO!"} } }"#,
    );
    tree.write("src/lib.rs", "a.unwrap();\n// TODO! later\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"patterns\""), "{}", run.out);
}

#[test]
fn an_explicit_scope_with_only_unsupported_files_is_a_configuration_error() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/notes.txt", "TODO! later\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("has an \"in\" scope with no applicable file"),
        "{}",
        run.out
    );
}

#[test]
fn a_default_skipped_directory_is_not_read() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{}"#);
    tree.write("node_modules/dep/index.ts", "const z: any = 1;\n");
    tree.write("legacy/old.ts", "const y: any = 1;\n");
    tree.write("web/new.ts", "const x: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("web/new.ts:1  any"), "{}", run.out);
    assert!(!run.says("node_modules"), "{}", run.out);
    assert!(run.says("legacy/old.ts:1"), "{}", run.out);
    assert!(run.says("2 new escape site(s)"), "{}", run.out);
}

#[test]
fn except_drops_a_subtree() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "in": "src", "except": "src/generated" } }"#,
    );
    tree.write("src/thing.ts", "const a: any = 1;\n");
    tree.write("src/thing.test.ts", "const b: any = 1;\n");
    tree.write("src/generated/api.ts", "const c: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/thing.ts:1"), "{}", run.out);
    assert!(run.says("thing.test.ts"), "{}", run.out);
    assert!(!run.says("generated"), "{}", run.out);
}

const CFG_TEST: &str = r#"pub fn read() -> i32 {
    let v: Result<i32, ()> = Ok(1);
    v.unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let x: Result<i32, ()> = Ok(1);
        x.unwrap();
        x.expect("no");
    }
}

pub fn after() -> i32 {
    let v: Result<i32, ()> = Ok(1);
    v.expect("appended below the tests")
}
"#;

#[test]
fn a_site_inside_a_cfg_test_module_is_not_a_production_site() {
    let tree = tree();
    tree.write("src/lib.rs", CFG_TEST);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:3  unwrap"), "{}", run.out);
    assert!(run.says("src/lib.rs:18  expect"), "{}", run.out);
    assert!(!run.says("src/lib.rs:11"), "{}", run.out);
    assert!(!run.says("src/lib.rs:12"), "{}", run.out);

    tree.base();
    let rerun = tree.run(&["escapes"]);
    assert_eq!(rerun.code, 0, "{}", rerun.out);
    assert!(rerun.says("(2 in Rust tests skipped)"), "{}", rerun.out);
}

#[test]
fn skip_rust_tests_turned_off_judges_the_test_module_too() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "in": "src", "skip_rust_tests": false } }"#,
    );
    tree.write("src/lib.rs", CFG_TEST);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:11"), "{}", run.out);
    assert!(run.says("src/lib.rs:12"), "{}", run.out);
    assert!(!run.says("in Rust tests skipped"), "{}", run.out);
}

#[test]
fn a_retired_language_selector_is_rejected() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "languages": ["cobol"] } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"languages\""), "{}", run.out);
}

#[test]
fn an_empty_retired_language_selector_is_rejected() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "languages": [] } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"languages\""), "{}", run.out);
}

#[test]
fn a_retired_malformed_project_pattern_is_rejected_before_compilation() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "patterns": {"broken": "([unclosed"} } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"patterns\""), "{}", run.out);
}

#[test]
fn javascript_is_read_by_the_typescript_set() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/thing.js", "it.only('x', () => {});\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/thing.js:1  skipped test"), "{}", run.out);
}

#[test]
fn a_cfg_test_module_behind_stacked_attributes_is_still_a_test_module() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        "#[cfg(test)]\n#[allow(clippy::all)]\nmod tests {\n    fn t() {\n        x.unwrap();\n    }\n}\n",
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:2  allow"), "{}", run.out);
    assert!(!run.says("src/lib.rs:5"), "{}", run.out);
}

#[test]
fn a_comment_between_the_attribute_and_the_module_does_not_end_the_range() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        "#[cfg(test)]\n// a note about the tests\nmod tests {\n    fn t() {\n        x.unwrap();\n    }\n}\n",
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("(1 in Rust tests skipped)"), "{}", run.out);
}

#[test]
fn hidden_directories_are_read_except_for_the_default_skip_list() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{}"#);
    tree.write(".github/workflows/ci.sh", "make test || true\n");
    tree.write(".git/hooks/pre-commit.sh", "lint || true\n");
    tree.write(".config/scripts/setup.sh", "install || true\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says(".github/workflows/ci.sh:1"), "{}", run.out);
    assert!(!run.says(".git/hooks"), "{}", run.out);
    assert!(run.says("setup.sh"), "{}", run.out);
    assert!(run.says("2 new escape site(s)"), "{}", run.out);
}

#[test]
fn a_retired_exclude_glob_is_rejected() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "exclude": ["*.spec.[jt]s", "[!a]?.gen.ts"] } }"#,
    );
    tree.write("src/a.spec.ts", "const a: any = 1;\n");
    tree.write("src/b.spec.js", "const b: any = 1;\n");
    tree.write("src/c.spec.tsx", "const c: any = 1;\n");
    tree.write("src/zz.gen.ts", "const z: any = 1;\n");
    tree.write("src/aa.gen.ts", "const y: any = 1;\n");
    tree.write("src/plain.ts", "const p: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"exclude\""), "{}", run.out);
}

#[test]
fn every_alternative_inside_a_pattern_matches_too() {
    let tree = spread();
    tree.write(
        "src/alt.py",
        "@pytest.mark.skip\ndef a():\n    pass\npytest.skip(\"x\")\n@unittest.skip(\"y\")\ndef b():\n    pass\n",
    );
    tree.write(
        "src/alt.ts",
        "const a = b as any;\nconst c = <any>d;\nxit('x', () => {});\ndescribe.only('y', () => {});\ntest.skip('z', () => {});\n",
    );
    tree.write("src/alt.rs", "unimplemented!();\n#![allow(dead_code)]\n");
    tree.write("src/alt.go", "t.SkipNow()\nt.Skipf(\"x\")\n");
    tree.write("src/alt.rb", "xit 'x'\npending 'y'\n");
    tree.write("src/alt.sh", "set +e\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    for expected in [
        "src/alt.py:1  skipped test",
        "src/alt.py:4  skipped test",
        "src/alt.py:5  skipped test",
        "src/alt.ts:1  any",
        "src/alt.ts:2  any",
        "src/alt.ts:3  skipped test",
        "src/alt.ts:4  skipped test",
        "src/alt.ts:5  skipped test",
        "src/alt.rs:2  allow",
        "src/alt.go:1  skipped test",
        "src/alt.go:2  skipped test",
        "src/alt.rb:1  skipped test",
        "src/alt.rb:2  skipped test",
        "src/alt.sh:1  errors ignored",
    ] {
        assert!(run.says(expected), "missing {expected}\n{}", run.out);
    }
}

#[test]
fn a_module_typescript_file_is_scanned_like_any_other_typescript_file() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/a.mts", "const a: any = 1;\n");
    tree.write("src/b.cts", "const b: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.mts:1"), "{}", run.out);
    assert!(run.says("src/b.cts:1"), "{}", run.out);
}

#[test]
fn a_focused_or_expected_failure_test_is_a_new_escape_and_holds_at_the_base() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write(
        "src/spec.ts",
        "fit('x', () => {});\n  fdescribe('y', () => {});\nconst y = fit(points);\n",
    );
    tree.write(
        "src/thing.py",
        "@pytest.mark.xfail\ndef a():\n    return 1\n@pytest.mark.skipif(WINDOWS)\ndef b():\n    return 2\n",
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/spec.ts:1  focused test"), "{}", run.out);
    assert!(run.says("src/spec.ts:2  focused test"), "{}", run.out);
    assert!(run.says("src/thing.py:1  expected failure"), "{}", run.out);
    assert!(run.says("3 new escape site(s)"), "{}", run.out);
    assert!(!run.says("src/spec.ts:3"), "{}", run.out);
    assert!(!run.says("src/thing.py:4"), "{}", run.out);

    tree.base();
    let held = tree.run(&["escapes", "--strict"]);
    assert_eq!(held.code, 0, "{}", held.out);
}

const MIXED: &str = concat!("a.", "unwrap(); b.", "expect(\"x\");");

#[test]
fn repeated_lines_of_two_kinds_fail_as_one_site_labelled_by_the_first_pattern_with_every_match_counted()
 {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        &format!("{MIXED}\nfn pad() {{}}\n    {MIXED}\n"),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 1 new escape site(s)"), "{}", run.out);
    assert!(
        run.says(&format!("src/lib.rs:1  unwrap x4  {MIXED}")),
        "{}",
        run.out
    );
    assert!(!run.says("src/lib.rs:3"), "{}", run.out);
    assert!(!run.says("expect "), "{}", run.out);
}

#[test]
fn a_file_measured_at_the_base_and_excluded_now_is_a_note_naming_it() {
    let tree = tree();
    tree.write("src/gone.rs", "fn f() {}\n");
    tree.base();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "in": "src", "except": "src/gone.rs" } }"#,
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: src/gone.rs was measured at the base"),
        "{}",
        run.out
    );
    assert!(run.says("an exclusion drops it now"), "{}", run.out);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 2, "{}", strict.out);
}

#[test]
fn an_accepted_entry_for_a_retired_row_names_the_row_and_where_it_went() {
    let tree = tree();
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "escapes", "file": "src/gone.rs", "text": "fn vanished() {",
                "escape": "todo", "count": 1}"#,
        ),
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("matched nothing this run"), "{}", run.out);
    assert!(run.says("\"todo\" is a row klin retired"), "{}", run.out);
    assert!(run.says("\"stub\": \"not implemented\""), "{}", run.out);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
    assert!(
        strict.says("\"todo\" is a row klin retired"),
        "{}",
        strict.out
    );
    assert!(
        strict.says("\"stub\": \"not implemented\""),
        "{}",
        strict.out
    );
}

#[test]
fn an_accepted_entry_for_the_narrowed_skipped_test_row_names_skipif() {
    let tree = tree();
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "escapes", "file": "src/gone.py", "text": "def vanished():",
                "escape": "skipped test", "count": 1}"#,
        ),
    );
    tree.write("src/lib.py", "def f():\n    return 1\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("pytest.mark.skipif"), "{}", run.out);
}

#[test]
fn an_accepted_entry_that_matches_nothing_for_another_reason_names_no_row() {
    let tree = tree();
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "escapes", "file": "src/gone.rs", "text": "fn vanished() {",
                "escape": "unwrap", "count": 1}"#,
        ),
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("matched nothing this run"), "{}", run.out);
    assert!(!run.says("is a row klin"), "{}", run.out);
}

#[test]
fn a_live_row_of_another_language_is_not_read_as_the_retired_one() {
    let tree = tree();
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "escapes", "file": "src/gone.rs", "text": "fn vanished() {",
                "escape": "skipped test", "count": 1}"#,
        ),
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("matched nothing this run"), "{}", run.out);
    assert!(!run.says("is a row klin"), "{}", run.out);
}

const INTEGRATION_TEST: &str = r#"use demo::wrap;

#[test]
fn keeps() {
    let lines = wrap("ab");
    assert_eq!(lines.last().unwrap(), "ab");
    lines.first().expect("one line");
}
"#;

#[test]
fn unwrap_and_expect_in_a_file_under_a_test_root_are_left_out_by_default() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "src/lib.rs",
        "pub fn wrap(t: &str) -> Vec<String> { vec![t.to_string()] }\n",
    );
    tree.write("tests/render.rs", INTEGRATION_TEST);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("(2 in Rust tests skipped)"), "{}", run.out);
}

#[test]
fn a_skipped_test_under_a_test_root_is_still_an_escape() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn wrap() {}\n");
    tree.write(
        "tests/render.rs",
        concat!(
            "#[test]\n#[ign",
            "ore]\nfn slow() {\n    let x: Option<i32> = None;\n    x.unwrap();\n}\n"
        ),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
    assert!(run.says("tests/render.rs:2  skipped test"), "{}", run.out);
    assert!(!run.says("tests/render.rs:5"), "{}", run.out);
}

#[test]
fn a_cfg_attr_whose_predicate_always_holds_is_a_skipped_test() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        concat!(
            "#[test]\n#[cfg_attr(not(any()), ign",
            "ore = \"slow\")]\nfn slow() {}\n\n#[test]\n#[cfg_attr(all(), ign",
            "ore)]\nfn slower() {}\n\n#[test]\n#[cfg_attr(all(not(any()), any(all())), ign",
            "ore)]\nfn slowest() {}\n"
        ),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("3 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:2  skipped test"), "{}", run.out);
    assert!(run.says("src/lib.rs:6  skipped test"), "{}", run.out);
    assert!(run.says("src/lib.rs:10  skipped test"), "{}", run.out);
}

#[test]
fn a_cfg_attr_whose_predicate_may_not_hold_is_no_skipped_test() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        concat!(
            "#[test]\n#[cfg_attr(windows, ign",
            "ore)]\nfn unix_only() {}\n\n#[test]\n#[cfg_attr(any(), ign",
            "ore)]\nfn everywhere() {}\n"
        ),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("0 escape site(s)"), "{}", run.out);
}

#[test]
fn a_skipped_test_inside_an_inline_test_module_is_still_an_escape() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        concat!(
            "#[cfg(test)]\nmod tests {\n    #[test]\n    #[ign",
            "ore]\n    fn t() {\n        x.unwrap();\n    }\n}\n"
        ),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:4  skipped test"), "{}", run.out);
    assert!(!run.says("src/lib.rs:6"), "{}", run.out);
}

#[test]
fn allow_and_unsafe_in_rust_tests_remain_escapes() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "src/lib.rs",
        concat!(
            "#[cfg(test)]\nmod tests {\n    #[all",
            "ow(dead_code)]\n    fn t() {\n        unsa",
            "fe { raw() }\n    }\n}\n"
        ),
    );
    tree.write(
        "tests/render.rs",
        concat!(
            "#[test]\n#[all",
            "ow(unused)]\nfn t() {\n    unsa",
            "fe { raw() }\n}\n"
        ),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("4 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:3  allow"), "{}", run.out);
    assert!(run.says("src/lib.rs:5  unsafe"), "{}", run.out);
    assert!(run.says("tests/render.rs:2  allow"), "{}", run.out);
    assert!(run.says("tests/render.rs:4  unsafe"), "{}", run.out);
}

#[test]
fn skip_rust_tests_turned_off_judges_a_file_under_a_test_root_too() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "skip_rust_tests": false } }"#,
    );
    tree.write("src/lib.rs", CFG_TEST);
    tree.write("tests/render.rs", INTEGRATION_TEST);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("6 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:11  unwrap"), "{}", run.out);
    assert!(run.says("tests/render.rs:6  unwrap"), "{}", run.out);
    assert!(run.says("tests/render.rs:7  expect"), "{}", run.out);
    assert!(!run.says("in Rust tests skipped"), "{}", run.out);
}

#[test]
fn production_rust_beside_a_test_root_is_judged_as_before() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "src/lib.rs",
        "pub fn f(x: Option<i32>) -> i32 {\n    x.unwrap()\n}\n",
    );
    tree.write(
        "src/other.rs",
        "pub fn g(x: Option<i32>) -> i32 {\n    x.expect(\"g\")\n}\n",
    );
    tree.write("tests/render.rs", INTEGRATION_TEST);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("2 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:2  unwrap"), "{}", run.out);
    assert!(run.says("src/other.rs:2  expect"), "{}", run.out);
    assert!(!run.says("tests/render.rs"), "{}", run.out);
}

#[test]
fn a_root_that_stops_being_test_only_has_its_new_production_unwrap_judged() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "cases/only_test.rs",
        "#[test]\nfn t() {\n    let x: Option<i32> = None;\n    x.unwrap();\n}\n",
    );
    tree.base();
    tree.write(
        "cases/runtime.rs",
        "pub fn load(x: Option<i32>) -> i32 {\n    x.unwrap()\n}\n",
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("cases/runtime.rs:2  unwrap"), "{}", run.out);
    assert!(!run.says("in Rust tests skipped"), "{}", run.out);
}
