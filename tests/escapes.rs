mod harness;

use harness::Tree;

const CONFIG: &str = r#"{
  "project": "t",
  "escapes": { "roots": ["src"], "baseline": "detent/escapes-baseline.json" }
}"#;

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("quality.json", CONFIG);
    tree
}

fn baseline(entries: &str) -> String {
    format!(r#"{{ "entries": [{entries}] }}"#)
}

fn entry(file: &str, text: &str, line: u64, escape: &str, count: u64) -> String {
    format!(
        r#"{{"file": {file:?}, "text": {text:?}, "line": {line}, "escape": {escape:?}, "count": {count}}}"#
    )
}

#[test]
fn a_site_not_in_the_baseline_is_new_and_fails() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {\n    x.unwrap();\n}\n");
    tree.write("detent/escapes-baseline.json", &baseline(""));

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 1 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:2"), "{}", run.out);
    assert!(run.says("x.unwrap();"), "{}", run.out);
}

#[test]
fn a_missing_baseline_reads_as_empty_so_every_site_is_new() {
    let tree = tree();
    tree.write("src/lib.rs", "x.expect(\"boom\");\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
}

#[test]
fn write_baseline_accepts_what_exists_today_and_the_rerun_holds() {
    let tree = tree();
    tree.write(
        "src/lib.rs",
        "fn f() {\n    a.unwrap();\n    b.expect(\"no\");\n    #[allow(dead_code)]\n}\n",
    );

    let written = tree.run(&["escapes", "--write-baseline"]);
    assert_eq!(written.code, 0, "{}", written.out);
    assert!(written.says("3 escape site(s) accepted"), "{}", written.out);

    let stored = std::fs::read_to_string(tree.path("detent/escapes-baseline.json")).expect("read");
    assert!(stored.contains("\"provenance\""), "{stored}");
    assert!(stored.contains("\"entries\""), "{stored}");
    assert!(stored.contains("\"escape\""), "{stored}");
    assert!(stored.contains("\"count\""), "{stored}");

    let rerun = tree.run(&["escapes"]);
    assert_eq!(rerun.code, 0, "{}", rerun.out);
    assert!(
        rerun.says("OK: 3 escape site(s) in the tree"),
        "{}",
        rerun.out
    );
    assert!(!rerun.says("NOTE"), "{}", rerun.out);
}

#[test]
fn a_ratcheted_count_that_rose_is_worse_and_fails() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap().unwrap();\n");
    tree.write(
        "detent/escapes-baseline.json",
        &baseline(&entry("src/lib.rs", "a.unwrap().unwrap();", 1, "unwrap", 1)),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("got worse — the ratchet only tightens"),
        "{}",
        run.out
    );
    assert!(run.says("unwrap x2, was unwrap"), "{}", run.out);
}

#[test]
fn a_count_that_fell_is_improved_a_note_locally_and_a_failure_under_strict() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap();\n");
    tree.write(
        "detent/escapes-baseline.json",
        &baseline(&entry("src/lib.rs", "a.unwrap();", 1, "unwrap", 3)),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("improved"), "{}", run.out);
    assert!(run.says("detent escapes --write-baseline"), "{}", run.out);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
    assert!(strict.says("looser than the code"), "{}", strict.out);
}

#[test]
fn an_entry_no_finding_matched_is_stale_a_note_locally_and_a_failure_under_strict() {
    let tree = tree();
    tree.write("src/lib.rs", "fn f() {\n    a.unwrap();\n}\n");
    tree.run(&["escapes", "--write-baseline"]);
    tree.write("src/lib.rs", "fn f() {\n}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("matched nothing this run"), "{}", run.out);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
}

#[test]
fn a_value_the_entry_never_recorded_is_not_compared() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap().unwrap();\n");
    tree.write(
        "detent/escapes-baseline.json",
        &baseline(
            r#"{"file": "src/lib.rs", "text": "a.unwrap().unwrap();", "line": 1, "escape": "unwrap"}"#,
        ),
    );

    let run = tree.run(&["escapes", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("worse"), "{}", run.out);
}

#[test]
fn a_line_with_two_escape_kinds_counts_only_the_recorded_kind() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap(); b.expect(\"x\");\n");
    tree.write(
        "detent/escapes-baseline.json",
        &baseline(&entry(
            "src/lib.rs",
            "a.unwrap(); b.expect(\"x\");",
            1,
            "unwrap",
            1,
        )),
    );

    let run = tree.run(&["escapes", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_bare_list_baseline_is_not_supported() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap();\n");
    tree.write("detent/escapes-baseline.json", "[]");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("provenance"), "{}", run.out);
}

#[test]
fn failure_output_never_prints_the_command_that_rewrites_a_baseline() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap();\nb.unwrap();\n");
    tree.write(
        "detent/escapes-baseline.json",
        &baseline(&entry("src/lib.rs", "z.unwrap();", 9, "unwrap", 1)),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("Fix what the escape hides"), "{}", run.out);
    assert!(!run.says("--write-baseline"), "{}", run.out);
}

#[test]
fn an_inserted_third_twin_is_the_new_one_not_a_neighbour() {
    let tree = tree();
    let twins = "x.unwrap();\nfn a() {}\nfn b() {}\nfn c() {}\nx.unwrap();\n";
    tree.write("src/lib.rs", twins);
    tree.run(&["escapes", "--write-baseline"]);
    let inserted = "x.unwrap();\nfn a() {}\nx.unwrap();\nfn b() {}\nfn c() {}\nx.unwrap();\n";
    tree.write("src/lib.rs", inserted);

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new escape site(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:3"), "{}", run.out);
    assert!(!run.says("matched nothing"), "{}", run.out);
    assert!(!run.says("worse"), "{}", run.out);
}

#[test]
fn a_shared_value_keeps_a_moved_twin_matched_over_a_nearer_entry() {
    let tree = tree();
    let lines: Vec<&str> = std::iter::repeat_n("fn pad() {}", 18)
        .chain(["a.unwrap(); b.unwrap();"])
        .chain(std::iter::repeat_n("fn pad() {}", 30))
        .chain(["a.unwrap(); b.unwrap();"])
        .collect();
    tree.write("src/lib.rs", &(lines.join("\n") + "\n"));
    tree.write(
        "detent/escapes-baseline.json",
        &baseline(&format!(
            "{}, {}",
            entry("src/lib.rs", "a.unwrap(); b.unwrap();", 3, "unwrap", 2),
            entry("src/lib.rs", "a.unwrap(); b.unwrap();", 20, "unwrap", 1)
        )),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("src/lib.rs:50"), "{}", run.out);
    assert!(!run.says("src/lib.rs:19  unwrap x2, was"), "{}", run.out);
}

#[test]
fn only_restricts_findings_and_entries_so_untouched_files_are_not_stale() {
    let tree = tree();
    tree.write("src/a.rs", "a.unwrap();\n");
    tree.write("src/b.rs", "b.unwrap();\n");
    tree.run(&["escapes", "--write-baseline"]);
    tree.write("src/a.rs", "fn fixed() {}\n");

    let all = tree.run(&["escapes", "--strict"]);
    assert_eq!(all.code, 1, "{}", all.out);

    let only = tree.run(&["escapes", "--strict", "--only", "src/b.rs"]);
    assert_eq!(only.code, 0, "{}", only.out);
    assert!(!only.says("matched nothing"), "{}", only.out);
}

#[test]
fn provenance_drift_names_what_changed_a_note_locally_and_a_failure_under_strict() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap();\n");
    tree.run(&["escapes", "--write-baseline"]);
    let stored = std::fs::read_to_string(tree.path("detent/escapes-baseline.json")).expect("read");
    tree.write(
        "detent/escapes-baseline.json",
        &stored.replace("\"version\": \"1\"", "\"version\": \"0\""),
    );

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("measured by escapes 0, this run by 1"),
        "{}",
        run.out
    );
    assert!(run.says("may not be comparable"), "{}", run.out);

    let strict = tree.run(&["escapes", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
}

#[test]
fn a_config_change_drifts() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap();\n");
    tree.run(&["escapes", "--write-baseline"]);
    tree.write(
        "quality.json",
        &CONFIG.replace(r#"["src"]"#, r#"["src", "lib"]"#),
    );
    tree.write("lib/other.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("different gate configuration"), "{}", run.out);
}

#[test]
fn a_missing_baseline_key_is_a_tool_error_naming_it() {
    let tree = Tree::new();
    tree.write("quality.json", r#"{"escapes": {"roots": ["src"]}}"#);
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("baseline"), "{}", run.out);
}

#[test]
fn a_baseline_name_the_guard_cannot_recognise_is_refused() {
    let tree = Tree::new();
    tree.write(
        "quality.json",
        r#"{"escapes": {"roots": ["src"], "baseline": "detent/debt.json"}}"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("guard"), "{}", run.out);
    assert!(run.says("baseline"), "{}", run.out);
}

#[test]
fn a_clean_quiet_run_prints_nothing() {
    let tree = tree();
    tree.write("src/lib.rs", "a.unwrap();\n");
    tree.run(&["escapes", "--write-baseline"]);

    let run = tree.run(&["escapes", "--quiet", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{}", run.out);
}
