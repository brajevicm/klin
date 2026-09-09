mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::Tree;

const CONFIG: &str = r#"{
  "project": "t",
  "escapes": { "roots": ["src"], "languages": ["rust"] }
}"#;

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree
}

fn accepted(entries: &str) -> String {
    format!(
        r#"{{ "project": "t", "accepted": [{entries}],
             "escapes": {{ "roots": ["src"], "languages": ["rust"] }} }}"#
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
    assert!(run.says("names none of the values"), "{}", run.out);
    assert!(run.says("count"), "{}", run.out);
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
fn one_language_named_twice_is_read_once_and_counted_once() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["javascript", "typescript"] } }"#,
    );
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
    tree.write(
        "klin.json",
        r#"{ "project": "t",
             "escapes": { "roots": ["src"],
                          "languages": ["python", "typescript", "swift", "rust", "go",
                                        "kotlin", "java", "ruby", "shell"] } }"#,
    );
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
        "src/thing.rs:6  todo",
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
fn a_project_pattern_is_read_alongside_the_built_in_sets() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["rust"],
             "patterns": {"todo bang": "TODO!"} } }"#,
    );
    tree.write("src/lib.rs", "a.unwrap();\n// TODO! later\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:1  unwrap"), "{}", run.out);
    assert!(run.says("src/lib.rs:2  todo bang"), "{}", run.out);
}

#[test]
fn a_project_pattern_alone_reads_every_file_under_the_roots() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "patterns": {"todo bang": "TODO!"} } }"#,
    );
    tree.write("src/notes.txt", "TODO! later\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/notes.txt:1  todo bang"), "{}", run.out);
}

#[test]
fn a_default_skipped_directory_is_not_read_and_skip_dirs_adds_to_the_list() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["."], "languages": ["typescript"],
             "skip_dirs": ["legacy"] } }"#,
    );
    tree.write("node_modules/dep/index.ts", "const z: any = 1;\n");
    tree.write("legacy/old.ts", "const y: any = 1;\n");
    tree.write("web/new.ts", "const x: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("web/new.ts:1  any"), "{}", run.out);
    assert!(!run.says("node_modules"), "{}", run.out);
    assert!(!run.says("legacy"), "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
}

#[test]
fn a_file_matching_an_exclude_glob_is_not_read() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["typescript"],
             "exclude": ["*.test.ts", "*/generated/*"] } }"#,
    );
    tree.write("src/thing.ts", "const a: any = 1;\n");
    tree.write("src/thing.test.ts", "const b: any = 1;\n");
    tree.write("src/generated/api.ts", "const c: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/thing.ts:1"), "{}", run.out);
    assert!(!run.says("thing.test.ts"), "{}", run.out);
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
    assert!(
        rerun.says("(2 in inline Rust tests skipped)"),
        "{}",
        rerun.out
    );
}

#[test]
fn skip_rust_tests_turned_off_judges_the_test_module_too() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["rust"], "skip_rust_tests": false } }"#,
    );
    tree.write("src/lib.rs", CFG_TEST);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:11"), "{}", run.out);
    assert!(run.says("src/lib.rs:12"), "{}", run.out);
    assert!(!run.says("in inline Rust tests skipped"), "{}", run.out);
}

#[test]
fn an_unknown_language_is_refused_naming_the_ones_that_exist() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["cobol"] } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("cobol"), "{}", run.out);
    assert!(run.says("python"), "{}", run.out);
}

#[test]
fn a_section_naming_nothing_to_look_for_is_refused() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": [] } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("nothing to look for"), "{}", run.out);
}

#[test]
fn a_project_pattern_that_is_not_a_regex_is_refused_naming_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "patterns": {"broken": "([unclosed"} } }"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("broken"), "{}", run.out);
}

#[test]
fn javascript_is_read_by_the_typescript_set() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["javascript"] } }"#,
    );
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
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("(2 in inline Rust tests skipped)"), "{}", run.out);
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
    assert!(run.says("(1 in inline Rust tests skipped)"), "{}", run.out);
}

#[test]
fn a_hidden_directory_is_read_unless_the_default_list_or_skip_dirs_names_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["."], "languages": ["shell"],
             "skip_dirs": ["scripts"] } }"#,
    );
    tree.write(".github/workflows/ci.sh", "make test || true\n");
    tree.write(".git/hooks/pre-commit.sh", "lint || true\n");
    tree.write(".config/scripts/setup.sh", "install || true\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says(".github/workflows/ci.sh:1"), "{}", run.out);
    assert!(!run.says(".git/hooks"), "{}", run.out);
    assert!(!run.says("setup.sh"), "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
}

#[test]
fn an_exclude_glob_honours_a_character_class() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["typescript"],
             "exclude": ["*.spec.[jt]s", "[!a]?.gen.ts"] } }"#,
    );
    tree.write("src/a.spec.ts", "const a: any = 1;\n");
    tree.write("src/b.spec.js", "const b: any = 1;\n");
    tree.write("src/c.spec.tsx", "const c: any = 1;\n");
    tree.write("src/zz.gen.ts", "const z: any = 1;\n");
    tree.write("src/aa.gen.ts", "const y: any = 1;\n");
    tree.write("src/plain.ts", "const p: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/plain.ts:1"), "{}", run.out);
    assert!(run.says("src/aa.gen.ts:1"), "{}", run.out);
    assert!(run.says("src/c.spec.tsx:1"), "{}", run.out);
    assert!(!run.says("a.spec.ts"), "{}", run.out);
    assert!(!run.says("b.spec.js"), "{}", run.out);
    assert!(!run.says("zz.gen.ts"), "{}", run.out);
    assert!(run.says("3 new escape site(s)"), "{}", run.out);
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
        "src/alt.rs:1  todo",
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
    tree.write(
        "klin.json",
        r#"{ "escapes": { "roots": ["src"], "languages": ["typescript"] } }"#,
    );
    tree.write("src/a.mts", "const a: any = 1;\n");
    tree.write("src/b.cts", "const b: any = 1;\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.mts:1"), "{}", run.out);
    assert!(run.says("src/b.cts:1"), "{}", run.out);
}
