mod harness;

use harness::Tree;

fn tree(conventions: &str) -> Tree {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        &format!(r#"{{ "conventions": {conventions} }}"#),
    );
    tree
}

const NO_OLD_FLAGS: &str = r#"{
  "no-old-flags": { "text": "config::Flags", "remedy": "Use the explicit execution context." }
}"#;

#[test]
fn a_new_literal_the_base_does_not_hold_fails_with_the_convention_and_its_remedy() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "use crate::config::Flags;\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("no-old-flags"), "{}", run.out);
    assert!(run.says("src/lib.rs:1"), "{}", run.out);
    assert!(run.says("use crate::config::Flags;"), "{}", run.out);
    assert!(
        run.says("Use the explicit execution context."),
        "{}",
        run.out
    );
}

#[test]
fn a_literal_the_base_holds_passes() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "use crate::config::Flags;\n");
    tree.base();

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: no-old-flags"), "{}", run.out);
}

#[test]
fn the_gate_runs_every_convention_the_section_names() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "use crate::config::Flags;\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  conventions"), "{}", run.out);
    assert!(
        run.says("Use the explicit execution context."),
        "{}",
        run.out
    );
}

#[test]
fn a_duplicate_occurrence_on_one_line_raises_the_count() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "use config::Flags;\n");
    tree.base();
    tree.write(
        "src/lib.rs",
        "use config::Flags;\nfn f() {}\nuse config::Flags;\n",
    );

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(
        run.says("no-old-flags x2, was no-old-flags x1"),
        "{}",
        run.out
    );
}

#[test]
fn the_configuration_that_states_a_convention_is_not_judged_by_it() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("klin.json:"), "{}", run.out);
}

#[test]
fn a_literal_holds_its_regex_metacharacters_as_text() {
    let tree = tree(r#"{ "no-star": { "text": "a.b(c)*", "remedy": "Stop." } }"#);
    tree.write("src/one.txt", "axb(c)\naxb(cc)\n");
    tree.base();
    tree.write("src/two.txt", "call a.b(c)* here\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new site(s)"), "{}", run.out);
    assert!(run.says("src/two.txt:1"), "{}", run.out);
}

#[test]
fn a_literal_in_a_comment_is_still_a_text_match() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "// config::Flags was retired\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/lib.rs:1"), "{}", run.out);
}

#[test]
fn a_site_that_moves_to_another_line_of_its_file_holds() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "use crate::config::Flags;\n");
    tree.base();
    tree.write("src/lib.rs", "\n\n// moved\nuse crate::config::Flags;\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_site_that_moves_to_another_file_is_new() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/a.rs", "use crate::config::Flags;\n");
    tree.write("src/b.rs", "fn b() {}\n");
    tree.base();
    tree.write("src/a.rs", "fn a() {}\n");
    tree.write("src/b.rs", "use crate::config::Flags;\nfn b() {}\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/b.rs:1"), "{}", run.out);
    assert!(run.says("nothing matched"), "{}", run.out);
}

#[test]
fn a_file_renamed_inside_the_scope_keeps_its_sites() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/a.rs", "use crate::config::Flags;\nfn body() {}\n");
    tree.base();
    tree.git(&["mv", "src/a.rs", "src/b.rs"]);

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: no-old-flags: 1 site(s)"), "{}", run.out);
}

#[test]
fn a_file_renamed_out_of_except_brings_its_sites_in_as_new() {
    let tree =
        tree(r#"{ "c": { "text": "FORBIDDEN", "except": "src/allowed", "remedy": "Do." } }"#);
    tree.write("src/allowed/a.rs", "FORBIDDEN\nfn body() {}\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree.git(&["mv", "src/allowed/a.rs", "src/a.rs"]);

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.rs:1"), "{}", run.out);
    assert!(run.says("nothing matched"), "{}", run.out);
}

#[test]
fn a_site_copied_into_another_file_is_new() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/a.rs", "use crate::config::Flags;\n");
    tree.base();
    tree.write("src/b.rs", "use crate::config::Flags;\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new site(s)"), "{}", run.out);
    assert!(run.says("src/b.rs:1"), "{}", run.out);
}

#[test]
fn a_site_the_code_removed_passes_with_nothing_to_say() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/a.rs", "use crate::config::Flags;\n");
    tree.base();
    tree.write("src/a.rs", "fn a() {}\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

fn judged(tree: &Tree) -> harness::Run {
    tree.run(&["check", "conventions", "--json"])
}

/// Every finding a run reported, as the convention and the site, sorted.
fn sites(run: &harness::Run) -> Vec<String> {
    let report = run.json();
    let mut out: Vec<String> = report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|finding| {
            format!(
                "{} {}:{}",
                finding["values"]["convention"].as_str().unwrap_or_default(),
                finding["file"].as_str().unwrap_or_default(),
                finding["line"]
            )
        })
        .collect();
    out.sort();
    out
}

fn refused(conventions: &str) -> harness::Run {
    let tree = tree(conventions);
    tree.write("src/lib.rs", "fn f() {}\n");
    let run = tree.run(&["check", "conventions"]);
    assert_eq!(run.code, 2, "{conventions}\n{}", run.out);
    run
}

#[test]
fn an_unknown_field_names_the_convention_and_the_key_it_most_likely_meant() {
    let run = refused(
        r#"{ "single-parser-owner": { "code": "Parser::new()", "exlude": "src/syntax", "remedy": "Use it." } }"#,
    );

    assert!(
        run.says("convention \"single-parser-owner\" has unknown field \"exlude\""),
        "{}",
        run.out
    );
    assert!(run.says("Did you mean \"except\"?"), "{}", run.out);
}

#[test]
fn a_convention_reads_no_key_beyond_its_own_seven() {
    for key in [
        "name",
        "rules",
        "matcher",
        "kind",
        "type",
        "pattern",
        "structural",
        "path",
        "roots",
        "languages",
        "exclude",
        "exclude_except",
        "skip_dirs",
        "regex",
        "example",
    ] {
        let run = refused(&format!(
            r#"{{ "c": {{ "text": "x", "remedy": "Do.", "{key}": "y" }} }}"#
        ));
        assert!(
            run.says(&format!("convention \"c\" has unknown field \"{key}\"")),
            "{key}: {}",
            run.out
        );
    }
}

#[test]
fn a_convention_without_a_remedy_is_refused() {
    for rule in [r#"{ "text": "x" }"#, r#"{ "text": "x", "remedy": "  " }"#] {
        let run = refused(&format!(r#"{{ "c": {rule} }}"#));
        assert!(
            run.says("convention \"c\" has no \"remedy\""),
            "{}",
            run.out
        );
    }
}

#[test]
fn a_convention_states_exactly_one_matcher() {
    let none = refused(r#"{ "c": { "remedy": "Do." } }"#);
    assert!(
        none.says("convention \"c\" defines none of: text, code, files"),
        "{}",
        none.out
    );

    let both = refused(r#"{ "c": { "text": "x", "code": "x()", "remedy": "Do." } }"#);
    assert!(
        both.says("convention \"c\" defines both \"text\" and \"code\""),
        "{}",
        both.out
    );
    assert!(
        both.says("Choose exactly one of: text, code, files."),
        "{}",
        both.out
    );
}

#[test]
fn a_language_on_a_rule_that_is_not_code_is_refused() {
    for matcher in [r#""text": "x""#, r#""files": "*.tmp""#] {
        let run = refused(&format!(
            r#"{{ "c": {{ {matcher}, "language": "rust", "remedy": "Do." }} }}"#
        ));
        assert!(
            run.says("convention \"c\" sets \"language\" on a rule that is not \"code\""),
            "{}",
            run.out
        );
    }
}

#[test]
fn a_language_no_code_pattern_is_written_in_is_refused() {
    let run = refused(r#"{ "c": { "code": "open()", "language": "go", "remedy": "Do." } }"#);

    assert!(
        run.says("convention \"c\" names language \"go\""),
        "{}",
        run.out
    );
    assert!(run.says("one of: rust, typescript"), "{}", run.out);
}

#[test]
fn a_scope_path_that_is_absolute_outside_the_repository_or_a_glob_is_refused() {
    for (path, why) in [
        ("/src", "is absolute"),
        ("../other", "does not name a path inside the repository"),
        ("src/../../x", "does not name a path inside the repository"),
        ("src/*.rs", "is not a path"),
    ] {
        for key in ["in", "except"] {
            let run = refused(&format!(
                r#"{{ "c": {{ "text": "x", "{key}": "{path}", "remedy": "Do." }} }}"#
            ));
            assert!(
                run.says(&format!("has an \"{key}\" path \"{path}\" that {why}")),
                "{}",
                run.out
            );
        }
    }
}

#[test]
fn a_scope_that_is_not_a_path_or_a_list_of_paths_is_refused() {
    for scope in ["[]", "3", r#"["src", 3]"#] {
        let run = refused(&format!(
            r#"{{ "c": {{ "text": "x", "in": {scope}, "remedy": "Do." }} }}"#
        ));
        assert!(
            run.says("has an \"in\" that is not a repository-relative path or a non-empty list"),
            "{}",
            run.out
        );
    }
}

#[test]
fn a_code_pattern_the_language_cannot_read_is_refused() {
    let run = refused(r#"{ "c": { "code": "Command::new(", "remedy": "Do." } }"#);

    assert!(
        run.says("convention \"c\" has a \"code\" pattern that is not rust code"),
        "{}",
        run.out
    );
}

#[test]
fn a_files_glob_that_could_match_no_path_is_refused() {
    let unclosed = refused(r#"{ "c": { "files": "src/[ab", "remedy": "Do." } }"#);
    assert!(
        unclosed.says("with a \"[\" that does not close"),
        "{}",
        unclosed.out
    );

    let absolute = refused(r#"{ "c": { "files": "/tmp/*", "remedy": "Do." } }"#);
    assert!(absolute.says("that is absolute"), "{}", absolute.out);
}

#[test]
fn a_section_that_is_not_a_flat_map_of_conventions_is_refused() {
    let wrapped = refused(r#"{ "rules": [{ "name": "c", "text": "x", "remedy": "Do." }] }"#);
    assert!(
        wrapped.says("convention \"rules\" must be an object"),
        "{}",
        wrapped.out
    );

    let listed = refused(r#"[{ "name": "c", "text": "x", "remedy": "Do." }]"#);
    assert!(
        listed.says("\"conventions\" is an object of convention names"),
        "{}",
        listed.out
    );
}

#[test]
fn in_takes_a_directory_with_everything_below_it_and_a_file_as_itself() {
    let tree = tree(
        r#"{
          "dir": { "text": "FORBIDDEN", "in": "src/cli", "remedy": "Do." },
          "file": { "text": "FORBIDDEN", "in": "src/main.rs", "remedy": "Do." }
        }"#,
    );
    for file in [
        "src/cli/mod.rs",
        "src/cli/deep/run.rs",
        "src/client.rs",
        "src/main.rs",
        "src/main.rs.bak",
    ] {
        tree.write(file, "FORBIDDEN\n");
    }

    assert_eq!(
        sites(&judged(&tree)),
        [
            "dir src/cli/deep/run.rs:1",
            "dir src/cli/mod.rs:1",
            "file src/main.rs:1"
        ]
    );
}

#[test]
fn several_in_paths_are_one_scope_and_except_takes_out_a_file_or_a_subtree() {
    let tree = tree(
        r#"{ "c": {
          "text": "FORBIDDEN",
          "in": ["src/cli", "crates/core"],
          "except": ["src/cli/legacy", "crates/core/allowed.rs"],
          "remedy": "Do."
        } }"#,
    );
    for file in [
        "src/cli/a.rs",
        "src/cli/legacy/b.rs",
        "crates/core/c.rs",
        "crates/core/allowed.rs",
        "src/other.rs",
    ] {
        tree.write(file, "FORBIDDEN\n");
    }

    assert_eq!(
        sites(&judged(&tree)),
        ["c crates/core/c.rs:1", "c src/cli/a.rs:1"]
    );
}

#[test]
fn a_scope_path_is_read_from_the_repository_root_wherever_klin_runs_and_never_as_a_basename() {
    let tree = tree(r#"{ "c": { "text": "FORBIDDEN", "except": "lib.rs", "remedy": "Do." } }"#);
    tree.write("src/lib.rs", "FORBIDDEN\n");
    tree.write("lib.rs", "FORBIDDEN\n");

    assert_eq!(sites(&judged(&tree)), ["c src/lib.rs:1"]);
    let below = harness::run_from(&tree.path("src"), &["check", "conventions", "--json"]);
    assert_eq!(sites(&below), ["c src/lib.rs:1"]);
}

#[test]
fn a_skipped_ignored_or_hidden_path_is_not_judged() {
    let tree = tree(r#"{ "c": { "text": "FORBIDDEN", "remedy": "Do." } }"#);
    tree.write(".gitignore", "gen/\n");
    tree.write("target/debug/out.rs", "FORBIDDEN\n");
    tree.write("gen/made.rs", "FORBIDDEN\n");
    tree.write(".github/ci.yml", "FORBIDDEN\n");
    tree.write("src/lib.rs", "FORBIDDEN\n");

    assert_eq!(sites(&judged(&tree)), ["c src/lib.rs:1"]);
}

#[test]
fn an_in_path_that_names_nothing_fails_and_an_except_path_that_names_nothing_is_a_note() {
    let tree = tree(
        r#"{
          "c": { "text": "FORBIDDEN", "in": "src/mian.rs", "remedy": "Do." },
          "d": { "text": "FORBIDDEN", "except": "src/gone", "remedy": "Do." }
        }"#,
    );
    tree.write("src/main.rs", "FORBIDDEN\n");

    let run = tree.run(&["check", "conventions"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("FAIL: convention \"c\" has an \"in\" path \"src/mian.rs\" that names nothing"),
        "{}",
        run.out
    );
    assert!(
        run.says("NOTE: convention \"d\" has an \"except\" path \"src/gone\" that names nothing"),
        "{}",
        run.out
    );

    let detail = tree.run(&["policy", "conventions", "c"]);
    assert!(
        detail.says("  The \"in\" path src/mian.rs matches nothing in the tree.\n"),
        "{}",
        detail.out
    );
}

#[test]
fn each_in_path_that_names_nothing_is_a_configuration_error_even_beside_a_finding() {
    let tree = tree(
        r#"{
          "c": { "text": "FORBIDDEN", "in": "src/mian.rs", "remedy": "Do." },
          "d": { "text": "FORBIDDEN", "in": "src/gone.rs", "remedy": "Do." }
        }"#,
    );
    tree.write("src/main.rs", "FORBIDDEN\n");

    let run = tree.run(&["check", "conventions", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let errors = |report: &serde_json::Value| {
        report["errors"]
            .as_array()
            .expect("errors")
            .iter()
            .filter(|error| error["kind"] == "configuration")
            .count()
    };
    assert_eq!(errors(&run.json()), 2, "{}", run.out);

    let beside = tree_with_finding();
    let run = beside.run(&["check", "conventions", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(errors(&run.json()), 1, "{}", run.out);
}

fn tree_with_finding() -> Tree {
    let tree = tree(
        r#"{
          "c": { "text": "FORBIDDEN", "in": "src/mian.rs", "remedy": "Do." },
          "e": { "text": "FORBIDDEN", "remedy": "Do." }
        }"#,
    );
    tree.write("src/main.rs", "FORBIDDEN\n");
    tree
}

const SCRATCH: &str = r#"{
  "no-scratch-files": { "files": "**/scratch.*", "remedy": "Remove temporary scratch files." }
}"#;

#[test]
fn a_new_path_a_files_glob_matches_fails_and_one_the_base_holds_passes() {
    let tree = tree(SCRATCH);
    tree.write("scratch.txt", "held\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();

    let held = tree.run(&["check", "conventions"]);
    assert_eq!(held.code, 0, "{}", held.out);

    tree.write("src/deep/scratch.rs", "new\n");
    tree.write("src/scratchpad.rs", "not a match\n");
    let new = tree.run(&["check", "conventions"]);
    assert_eq!(new.code, 1, "{}", new.out);
    assert!(new.says("src/deep/scratch.rs:0"), "{}", new.out);
    assert!(!new.says("src/scratchpad.rs"), "{}", new.out);
    assert!(new.says("Remove temporary scratch files."), "{}", new.out);
}

#[test]
fn a_path_renamed_into_a_files_glob_fails() {
    let tree = tree(SCRATCH);
    tree.write("src/notes.txt", "kept\n");
    tree.base();
    tree.git(&["mv", "src/notes.txt", "src/scratch.txt"]);

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/scratch.txt:0"), "{}", run.out);
}

#[test]
fn a_files_glob_reads_no_ignored_path_and_keeps_to_in_and_except() {
    let tree = tree(
        r#"{ "c": { "files": "**/*.tmp", "in": "src", "except": "src/cache", "remedy": "Do." } }"#,
    );
    tree.write(".gitignore", "*.log.tmp\n");
    for file in [
        "src/a.tmp",
        "src/b.log.tmp",
        "src/cache/c.tmp",
        "docs/d.tmp",
        "node_modules/e.tmp",
    ] {
        tree.write(file, "");
    }

    assert_eq!(sites(&judged(&tree)), ["c src/a.tmp:0"]);
}

const GIT: &str = r#"{
  "single-git-boundary": {
    "code": "Command::new(\"git\")",
    "except": ["src/project/git.rs", "tests"],
    "remedy": "Use the shared Git boundary."
  }
}"#;

#[test]
fn a_code_pattern_matches_code_and_not_a_comment_or_a_string_that_reads_like_it() {
    let tree = tree(GIT);
    tree.write(
        "src/project/git.rs",
        "fn git() { Command::new(\"git\"); }\n",
    );
    tree.write("tests/it.rs", "fn t() { Command::new(\"git\"); }\n");
    tree.write(
        "src/run.rs",
        "fn run() {\n    // Command::new(\"git\")\n    let said = \"Command::new(\\\"git\\\")\";\n    Command::new(\"git\").arg(said);\n}\n",
    );

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new site(s)"), "{}", run.out);
    assert!(run.says("src/run.rs:4"), "{}", run.out);
    assert!(run.says("Use the shared Git boundary."), "{}", run.out);
}

#[test]
fn a_code_site_the_base_holds_passes() {
    let tree = tree(GIT);
    tree.write("src/run.rs", "fn run() { Command::new(\"git\"); }\n");
    tree.base();

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: single-git-boundary: 1 site(s)"),
        "{}",
        run.out
    );
}

#[test]
fn a_code_pattern_that_matches_nothing_now_is_a_convention_all_the_same() {
    let tree = tree(
        r#"{ "retired": { "code": "old_api::call($$$ARGS)", "language": "rust", "remedy": "Call new_api." } }"#,
    );
    tree.write("README.md", "no source here\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: retired: 0 site(s)"), "{}", run.out);
}

#[test]
fn a_rust_pattern_holds_one_node_a_list_of_nodes_and_a_block() {
    let tree = tree(
        r#"{
          "one": { "code": "$VALUE.clone()", "remedy": "Borrow it." },
          "list": { "code": "log($$$ARGS)", "remedy": "Use the logger." },
          "block": { "code": "if $COND { $$$BODY }", "remedy": "Match on it." }
        }"#,
    );
    tree.write(
        "src/lib.rs",
        "fn f() {\n    a.clone();\n    log();\n    log(1, 2, 3);\n    if ready { go(); }\n}\n",
    );

    assert_eq!(
        sites(&judged(&tree)),
        [
            "block src/lib.rs:5",
            "list src/lib.rs:3",
            "list src/lib.rs:4",
            "one src/lib.rs:2"
        ]
    );
}

#[test]
fn typescript_and_tsx_are_one_language_and_holes_work_in_both() {
    let tree = tree(
        r#"{
          "no-console": { "code": "console.log($$$ARGS)", "remedy": "Use the logger." },
          "no-raw-div": { "code": "<div>{$CHILD}</div>", "remedy": "Use Box." }
        }"#,
    );
    tree.write("web/a.ts", "console.log(1, 2);\n// console.log(3)\n");
    tree.write(
        "web/b.tsx",
        "const e = <div>{value}</div>;\nconsole.log(e);\n",
    );

    assert_eq!(
        sites(&judged(&tree)),
        [
            "no-console web/a.ts:1",
            "no-console web/b.tsx:2",
            "no-raw-div web/b.tsx:1"
        ]
    );
    let report = tree.run(&["policy", "conventions", "no-console"]);
    assert!(
        report.says("in TypeScript. The language is derived from the files in scope.\n"),
        "{}",
        report.out
    );
}

#[test]
fn a_scope_in_two_languages_asks_for_one_and_a_language_the_convention_names_settles_it() {
    let tree = tree(
        r#"{ "no-direct-open": { "code": "open($PATH)", "remedy": "Use the file boundary." } }"#,
    );
    tree.write("src/lib.rs", "fn f() { open(p); }\n");
    let one = tree.run(&["check", "conventions"]);
    assert_eq!(one.code, 1, "{}", one.out);

    tree.write("web/a.ts", "open(p);\n");
    let two = tree.run(&["check", "conventions"]);
    assert_eq!(two.code, 2, "{}", two.out);
    assert!(
        two.says("convention \"no-direct-open\" applies to more than one structural language"),
        "{}",
        two.out
    );
    assert!(two.says("  rust\n"), "{}", two.out);
    assert!(two.says("  typescript\n"), "{}", two.out);
    assert!(
        two.says("Add \"language\": \"rust\" or narrow \"in\"."),
        "{}",
        two.out
    );

    tree.write(
        "klin.json",
        r#"{ "conventions": { "no-direct-open": { "code": "open($PATH)", "language": "typescript", "remedy": "Use the file boundary." } } }"#,
    );
    let named = tree.run(&["check", "conventions"]);
    assert_eq!(named.code, 1, "{}", named.out);
    assert!(named.says("web/a.ts:1"), "{}", named.out);
    assert!(!named.says("src/lib.rs"), "{}", named.out);
}

#[test]
fn a_scope_with_no_source_a_pattern_reads_asks_for_a_language_or_a_scope() {
    let tree = tree(r#"{ "c": { "code": "open($PATH)", "in": "docs", "remedy": "Do." } }"#);
    tree.write("docs/a.md", "open(p)\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("convention \"c\" applies to no structural language"),
        "{}",
        run.out
    );
    assert!(run.says("Add \"language\""), "{}", run.out);
}

#[test]
fn the_language_comes_from_the_scope_and_never_from_the_grammar_that_reads_the_pattern() {
    let tree = tree(r#"{ "c": { "code": "function $NAME() {}", "remedy": "Do." } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("has a \"code\" pattern that is not rust code"),
        "{}",
        run.out
    );
}

#[test]
fn a_new_source_file_the_grammar_rejects_is_an_unreadable_review_item() {
    let tree = tree(GIT);
    tree.write("src/broken.rs", "fn broken( {\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("REVIEW: src/broken.rs is not measured (unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn a_source_file_the_grammar_rejected_at_the_base_too_is_a_coverage_note() {
    let tree = tree(GIT);
    tree.write("src/broken.rs", "fn broken( {\n");
    tree.base();

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: src/broken.rs is not measured (unreadable)"),
        "{}",
        run.out
    );
}

const TWO_ON_ONE_LINE: &str = r#"
  "no-git": { "text": "Command::new(\"git\")", "remedy": "Use the Git boundary." },
  "no-command": { "code": "Command::new($PROGRAM)", "remedy": "Use the process boundary." }
"#;

#[test]
fn two_conventions_on_one_line_are_two_findings_each_with_its_own_remedy() {
    let tree = tree(&format!("{{ {} }}", TWO_ON_ONE_LINE));
    tree.write("src/lib.rs", "fn f() { Command::new(\"git\"); }\n");

    let run = judged(&tree);
    let report = run.json();
    let Some(findings) = report["findings"].as_array() else {
        panic!("no findings in {}", run.out)
    };

    assert_eq!(findings.len(), 2, "{}", run.out);
    assert_ne!(findings[0]["id"], findings[1]["id"], "{}", run.out);
    let advice: Vec<(&str, &str)> = findings
        .iter()
        .map(|finding| {
            (
                finding["values"]["logical_gate"]
                    .as_str()
                    .unwrap_or_default(),
                finding["remedy"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        advice.contains(&("conventions/no-git", "Use the Git boundary.")),
        "{}",
        run.out
    );
    assert!(
        advice.contains(&("conventions/no-command", "Use the process boundary.")),
        "{}",
        run.out
    );
}

#[test]
fn accepting_one_convention_at_a_site_leaves_the_other_failing_there() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        &format!(
            r#"{{ "conventions": {{ {} }},
                 "accepted": [{{ "gate": "conventions/no-git", "file": "src/lib.rs",
                                 "text": "fn f() {{ Command::new(\"git\"); }}", "count": 1 }}] }}"#,
            TWO_ON_ONE_LINE
        ),
    );
    tree.write("src/lib.rs", "fn f() { Command::new(\"git\"); }\n");

    let run = tree.run(&["check", "conventions"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("OK: no-git: 1 site(s)"), "{}", run.out);
    assert!(run.says("the convention no-command forbids"), "{}", run.out);
}

#[test]
fn an_accepted_entry_for_a_convention_the_section_no_longer_defines_is_refused() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "conventions": { "renamed": { "text": "x", "remedy": "Do." } },
             "accepted": [{ "gate": "conventions/old-name", "file": "src/lib.rs", "text": "x", "count": 1 }] }"#,
    );
    tree.write("src/lib.rs", "x\n");

    let run = tree.run(&["check"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("the accepted entry for conventions/old-name names no convention"),
        "{}",
        run.out
    );
}

#[test]
fn a_json_finding_carries_its_convention_matcher_language_count_and_remedy() {
    let tree = tree(GIT);
    tree.write("src/run.rs", "fn run() { Command::new(\"git\"); }\n");

    let run = judged(&tree);
    let report = run.json();
    let finding = &report["findings"][0];

    assert_eq!(finding["check"], "conventions", "{}", run.out);
    assert_eq!(finding["file"], "src/run.rs", "{}", run.out);
    assert_eq!(finding["line"], 1, "{}", run.out);
    assert_eq!(
        finding["text"], "fn run() { Command::new(\"git\"); }",
        "{}",
        run.out
    );
    let values = &finding["values"];
    assert_eq!(values["convention"], "single-git-boundary", "{}", run.out);
    assert_eq!(
        values["logical_gate"], "conventions/single-git-boundary",
        "{}",
        run.out
    );
    assert_eq!(values["matcher"], "code", "{}", run.out);
    assert_eq!(values["language"], "rust", "{}", run.out);
    assert_eq!(values["count"], 1, "{}", run.out);
    assert_eq!(
        values["remedy"], "Use the shared Git boundary.",
        "{}",
        run.out
    );
    assert_eq!(
        finding["remedy"], "Use the shared Git boundary.",
        "{}",
        run.out
    );
}

#[test]
fn the_policy_explains_scope_language_and_remedy() {
    let tree = tree(
        r#"{ "single-parser-owner": { "code": "Parser::new()", "except": "src/syntax", "remedy": "Use the shared syntax parser." } }"#,
    );
    tree.write("src/syntax/mod.rs", "fn p() { Parser::new(); }\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    let before = tree.status();

    let run = tree.run(&["policy", "conventions", "single-parser-owner"]);

    assert_eq!(run.code, 0, "{}", run.out);
    for line in [
        "single-parser-owner:\n",
        "  Forbids Parser::new() in the repository except src/syntax.\n",
        "  Reads as an expression or a type in Rust. The language is derived from the files in scope.\n",
        "  Fix: Use the shared syntax parser.\n",
    ] {
        assert!(run.says(line), "{line}: {}", run.out);
    }
    for engine in ["ast-grep", "tree-sitter", "Tree-sitter"] {
        assert!(!run.says(engine), "{}", run.out);
    }
    assert_eq!(tree.status(), before, "the policy wrote to the tree");
}
#[test]
fn the_policy_says_a_language_the_convention_names_is_pinned() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "conventions": { "legacy-direct-git": { "code": "Command::new(\"git\")", "language": "rust", "remedy": "Use the shared Git boundary." } } }"#,
    );
    tree.write("src/legacy/a.rs", "fn a() { Command::new(\"git\"); }\n");

    let run = tree.run(&["policy", "conventions", "legacy-direct-git"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("The language is pinned in klin.json.\n"),
        "{}",
        run.out
    );
}
#[test]
fn the_rules_klin_means_to_enforce_on_itself_read_as_policy_and_catch_their_violations() {
    let tree = tree(
        r#"{
          "single-git-boundary": { "code": "Command::new(\"git\")", "except": ["src/project/git.rs", "tests"], "remedy": "Use the shared Git boundary." },
          "single-parser-owner": { "code": "Parser::new()", "except": "src/syntax", "remedy": "Use the shared syntax parser." },
          "exhaustive-cli-dispatch": { "code": "_ => Ok(0)", "in": "src/main.rs", "remedy": "Handle every CLI command explicitly." },
          "no-old-flags": { "text": "config::Flags", "remedy": "Use the explicit execution context." },
          "no-records-side-channel": { "code": "RefCell<Records>", "remedy": "Write through the explicit check Sink." }
        }"#,
    );
    tree.write(
        "src/project/git.rs",
        "fn git() { Command::new(\"git\"); }\n",
    );
    tree.write("src/syntax/mod.rs", "fn parse() { Parser::new(); }\n");
    tree.write("tests/it.rs", "fn t() { Command::new(\"git\"); }\n");
    tree.write(
        "src/main.rs",
        "fn main() {\n    match command {\n        One => Ok(1),\n    }\n}\n",
    );
    tree.base();

    let clean = tree.run(&["check", "conventions"]);
    assert_eq!(clean.code, 0, "{}", clean.out);

    tree.write(
        "src/main.rs",
        "fn main() {\n    match command {\n        One => Ok(1),\n        _ => Ok(0),\n    }\n}\n",
    );
    tree.write(
        "src/check.rs",
        "use crate::config::Flags;\nstruct Held(RefCell<Records>);\nfn run() { Command::new(\"git\"); Parser::new(); }\n",
    );
    assert_eq!(
        sites(&judged(&tree)),
        [
            "exhaustive-cli-dispatch src/main.rs:4",
            "no-old-flags src/check.rs:1",
            "no-records-side-channel src/check.rs:2",
            "single-git-boundary src/check.rs:3",
            "single-parser-owner src/check.rs:3"
        ]
    );
}

#[test]
fn the_policy_says_why_a_pattern_cannot_run_and_what_to_do() {
    let tree = tree(
        r#"{
          "no-bad": { "code": "fn (", "remedy": "Write a pattern klin can read." },
          "no-git": { "code": "Command::new(\"git\")", "remedy": "Use the Git boundary." }
        }"#,
    );
    tree.write("src/lib.rs", "fn f() { Command::new(\"git\"); }\n");

    let run = tree.run(&["policy", "conventions", "no-bad"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(
            "klin can't read this pattern as Rust code in any of these places: code as written, \
             an expression, a match arm, a type, or a field."
        ),
        "{}",
        run.out
    );
    assert!(
        run.says("  Fix: Write a pattern klin can read.\n"),
        "{}",
        run.out
    );
    assert!(!run.says("no-git"), "{}", run.out);
}
#[test]
fn the_policy_refuses_a_name_the_section_does_not_define() {
    let tree = tree(NO_OLD_FLAGS);
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["policy", "conventions", "no-old-flag"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("no convention is named \"no-old-flag\" — the conventions are no-old-flags"),
        "{}",
        run.out
    );
}

#[test]
fn an_accepted_entry_for_a_section_set_to_false_stops_no_other_gate() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "conventions": false, "doc_size": {"README.md": 10},
             "accepted": [{ "gate": "conventions/old", "file": "src/lib.rs", "text": "x", "count": 1 }] }"#,
    );
    tree.words("README.md", 5);

    let run = tree.run(&["check", "doc-size"]);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_fragment_reads_as_the_match_arm_it_is_whatever_its_spacing() {
    let tree = tree(
        r#"{ "exhaustive-cli-dispatch": { "code": "_ => Ok(0)", "in": "src/main.rs", "remedy": "Handle every CLI command explicitly." } }"#,
    );
    tree.write(
        "src/main.rs",
        "fn main() {\n    match command {\n        One => Ok(1),\n        _ => Ok(0),\n    }\n    match other { _ => Ok(1) }\n    match x { _=>Ok(0) }\n    // _ => Ok(0)\n}\n",
    );

    assert_eq!(
        sites(&judged(&tree)),
        [
            "exhaustive-cli-dispatch src/main.rs:4",
            "exhaustive-cli-dispatch src/main.rs:7"
        ]
    );
    let report = tree.run(&["policy", "conventions", "exhaustive-cli-dispatch"]);
    assert!(
        report.says("Reads as a match arm in Rust."),
        "{}",
        report.out
    );
}

#[test]
fn a_fragment_reads_as_the_type_it_is_wherever_the_type_is_written() {
    let tree = tree(
        r#"{ "no-records-side-channel": { "code": "RefCell<$T>", "remedy": "Write through the explicit check Sink." } }"#,
    );
    tree.write(
        "src/lib.rs",
        "struct Run { records: RefCell<Records> }\nstruct Held(Rc<RefCell<Records>>);\nfn g(c: Cell<u8>) {}\nconst S: &str = \"RefCell<Records>\";\n",
    );

    assert_eq!(
        sites(&judged(&tree)),
        [
            "no-records-side-channel src/lib.rs:1",
            "no-records-side-channel src/lib.rs:2"
        ]
    );
    let report = tree.run(&["policy", "conventions", "no-records-side-channel"]);
    assert!(report.says("Reads as a type in Rust."), "{}", report.out);
}

#[test]
fn a_fragment_with_two_readings_matches_through_both() {
    let tree = tree(r#"{ "no-array-of-foo": { "code": "Array<Foo>", "remedy": "Use FooList." } }"#);
    tree.write(
        "web/a.ts",
        "let a: Array<Foo> = [];\nconst n = Array<Foo>;\nlet b: Array<Bar> = [];\n",
    );

    assert_eq!(
        sites(&judged(&tree)),
        ["no-array-of-foo web/a.ts:1", "no-array-of-foo web/a.ts:2"]
    );
    let report = tree.run(&["policy", "conventions", "no-array-of-foo"]);
    assert!(
        report.says("Reads as code as written or a type in TypeScript."),
        "{}",
        report.out
    );
}

#[test]
fn a_node_two_readings_match_counts_once_and_two_nested_nodes_count_twice() {
    let tree = tree(
        r##"{
          "no-derive": { "code": "#[derive($$$TRAITS)]", "remedy": "Write the impl." },
          "no-nested-clone": { "code": "$VALUE.clone()", "remedy": "Borrow it." }
        }"##,
    );
    tree.write(
        "src/lib.rs",
        "#[derive(Debug)]\nstruct S;\nfn f() {\n    a.clone().clone();\n}\n",
    );

    let run = judged(&tree);
    let report = run.json();
    let mut counts: Vec<String> = report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|finding| {
            format!(
                "{} {} x{}",
                finding["values"]["convention"].as_str().unwrap_or_default(),
                finding["line"],
                finding["values"]["count"]
            )
        })
        .collect();
    counts.sort();

    assert_eq!(
        counts,
        ["no-derive 1 x1", "no-nested-clone 4 x2"],
        "{}",
        run.out
    );
}

#[test]
fn a_fragment_no_reading_holds_names_every_reading_klin_tried() {
    let run = refused(r#"{ "c": { "code": "fn (", "remedy": "Do." } }"#);

    assert!(
        run.says(
            "the Rust grammar reads it as none of: code as written, an expression, a match arm, a type, a field"
        ),
        "{}",
        run.out
    );
}

#[test]
fn a_pattern_that_is_only_a_hole_is_refused_because_it_matches_everything() {
    let run = refused(r#"{ "c": { "code": "$ANYTHING", "remedy": "Do." } }"#);

    assert!(run.says("reads it as none of"), "{}", run.out);
}
