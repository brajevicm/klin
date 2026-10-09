mod harness;

use harness::Tree;

fn judge(tree: &Tree, name: &str, ceiling: u32) -> harness::Run {
    tree.write(
        "klin.json",
        &format!(r#"{{"doc_size": {{"{name}": {ceiling}}}}}"#),
    );
    tree.run(&["check", "doc-size"])
}

#[test]
fn a_document_under_its_ceiling_passes_and_prints_both_numbers() {
    let tree = Tree::new();
    tree.words("small.md", 5);
    let run = judge(&tree, "small.md", 10);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("is 5 words, ceiling 10"), "{}", run.out);
}

#[test]
fn a_document_over_its_ceiling_fails_naming_the_count_and_the_ceiling() {
    let tree = Tree::new();
    tree.words("small.md", 5);
    let run = judge(&tree, "small.md", 4);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("is 5 words, over its ceiling of 4"), "{}", run.out);
}

#[test]
fn a_document_inside_the_margin_prints_the_ok_line_and_warns() {
    let tree = Tree::new();
    tree.words("margin.md", 99);
    let run = judge(&tree, "margin.md", 100);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: margin.md is 99 words, ceiling 100"),
        "{}",
        run.out
    );
    assert!(
        run.says("WARN: margin.md is 99 words, 1 from its ceiling of 100."),
        "{}",
        run.out
    );
}

#[test]
fn a_document_outside_the_margin_does_not_warn() {
    let tree = Tree::new();
    tree.words("outside.md", 97);
    let run = judge(&tree, "outside.md", 100);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("WARN:"), "{}", run.out);
}

#[test]
fn a_document_over_its_ceiling_fails_with_no_warn_in_its_place() {
    let tree = Tree::new();
    tree.words("over.md", 101);
    let run = judge(&tree, "over.md", 100);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("WARN:"), "{}", run.out);
}

#[test]
fn the_failure_says_what_fixes_the_code() {
    let tree = Tree::new();
    tree.words("over.md", 101);
    let run = judge(&tree, "over.md", 100);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Keep in this file what the task asked for"),
        "{}",
        run.out
    );
    assert!(
        run.says("Only a person raises the ceiling, in a reviewed commit"),
        "{}",
        run.out
    );
    assert!(run.says("docs/"), "{}", run.out);
    assert!(!run.says("--write-baseline"), "{}", run.out);
}

#[test]
fn a_config_of_two_documents_names_only_the_one_over_its_ceiling() {
    let tree = Tree::new();
    tree.words("small.md", 5);
    tree.words("big.md", 7);
    tree.write(
        "klin.json",
        r#"{"doc_size": {"small.md": 10, "big.md": 6}}"#,
    );

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("FAIL: big.md is 7 words, over its ceiling of 6"),
        "{}",
        run.out
    );
    assert!(!run.says("FAIL: small.md"), "{}", run.out);
    assert!(
        run.says("OK: small.md is 5 words, ceiling 10"),
        "{}",
        run.out
    );
}

#[test]
fn a_document_the_config_lists_but_the_tree_lacks_is_a_tool_error() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": {"gone.md": 10}}"#);
    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no such file"), "{}", run.out);
}

#[test]
fn a_missing_document_does_not_hide_the_failure_of_one_before_it() {
    let tree = Tree::new();
    tree.words("over.md", 5);
    tree.write(
        "klin.json",
        r#"{"doc_size": {"over.md": 3, "gone.md": 10}}"#,
    );

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL: over.md is 5 words"), "{}", run.out);
    assert!(run.says("no such file"), "{}", run.out);
}

#[test]
fn an_empty_map_of_documents_is_a_config_error() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": {}}"#);
    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("must pin at least one document"), "{}", run.out);
}

/// A pin names one document, and every instruction file at the tree root keeps the ceiling the
/// derivation commit gives it rather than leaving scrutiny. ADR 0040.
#[test]
fn a_pinned_document_sits_beside_the_derived_ones_it_does_not_name() {
    let tree = Tree::new();
    tree.words("README.md", 120);
    tree.words("AGENTS.md", 20);
    tree.base();
    tree.write("klin.json", r#"{"doc_size": {"README.md": 1200}}"#);
    tree.words("AGENTS.md", 60);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("pinned: doc_size README.md 1200"), "{}", run.out);
    assert!(
        run.says("derived: doc_size AGENTS.md 50, the word count at the derivation commit"),
        "{}",
        run.out
    );
    assert!(
        run.says("FAIL: AGENTS.md is 60 words, over its ceiling of 50."),
        "{}",
        run.out
    );
    assert!(
        run.says("OK: README.md is 120 words, ceiling 1200"),
        "{}",
        run.out
    );
}

/// With `{}` a ceiling is derived only for the agent instruction files, so a README that grows
/// past its word count at the base is not judged. #382.
#[test]
fn a_readme_that_grows_past_its_base_word_count_passes_with_no_pin() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("README.md", 120);
    tree.words("AGENTS.md", 20);
    tree.base();
    tree.words("README.md", 400);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("README.md"), "{}", run.out);
}

/// With `{}`, a tree whose only root document is a README holds nothing `doc-size` judges, so
/// the gate does not apply while `doc-citations` still runs. #382.
#[test]
fn a_readme_alone_under_an_empty_config_leaves_doc_size_not_applicable() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("README.md", 120);
    tree.base();

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("doc-size — not-applicable"), "{}", run.out);
    assert!(run.says("doc-citations — runs"), "{}", run.out);
}

/// With `{}`, each instruction file keeps its derived ceiling and fails past it. #382.
#[test]
fn an_instruction_file_that_grows_past_its_derived_ceiling_fails_with_no_pin() {
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let tree = Tree::new();
        tree.write("klin.json", "{}");
        tree.words(name, 120);
        tree.base();
        tree.words(name, 400);

        let run = tree.run(&["check", "doc-size"]);
        assert_eq!(run.code, 1, "{name}: {}", run.out);
        assert!(
            run.says(&format!(
                "FAIL: {name} is 400 words, over its ceiling of 150."
            )),
            "{name}: {}",
            run.out
        );
    }
}

/// A README the section pins is judged under its pin, as before #382.
#[test]
fn a_pinned_readme_is_judged_under_its_pin() {
    let tree = Tree::new();
    tree.words("README.md", 120);
    tree.base();
    tree.write("klin.json", r#"{"doc_size": {"README.md": 200}}"#);
    tree.words("README.md", 400);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("pinned: doc_size README.md 200"), "{}", run.out);
    assert!(
        run.says("FAIL: README.md is 400 words, over its ceiling of 200."),
        "{}",
        run.out
    );
}

#[test]
fn a_section_set_to_false_excludes_the_gate() {
    let tree = Tree::new();
    tree.words("README.md", 70);
    tree.write("src/lib.rs", "pub fn one() -> i32 {\n    1\n}\n");
    tree.write("klin.json", r#"{"doc_size": false}"#);
    tree.base();
    tree.words("README.md", 700);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
}

#[test]
fn this_repositorys_own_documents_are_under_their_ceilings() {
    let tree = Tree::new();
    tree.write("README.md", include_str!("../README.md"));
    tree.write("CONTEXT.md", include_str!("../CONTEXT.md"));
    tree.write("AGENTS.md", include_str!("../AGENTS.md"));
    tree.write("RELEASE_NOTES.md", include_str!("../RELEASE_NOTES.md"));
    tree.write("klin.json", include_str!("../klin.json"));

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_word_is_a_run_of_non_whitespace_so_markup_counts_and_unicode_spaces_split() {
    let tree = Tree::new();
    tree.write(
        "spaced.md",
        "# Title\n\none\u{2003}two\u{00A0}three `code` **bold**\n```\nx = 1\n```\n",
    );
    let at = judge(&tree, "spaced.md", 12);
    assert_eq!(at.code, 0, "{}", at.out);
    assert!(at.says("is 12 words, ceiling 12"), "{}", at.out);
    assert!(at.says("0 from its ceiling of 12"), "{}", at.out);

    let over = judge(&tree, "spaced.md", 11);
    assert_eq!(over.code, 1, "{}", over.out);
    assert!(
        over.says("is 12 words, over its ceiling of 11"),
        "{}",
        over.out
    );
}

#[test]
fn a_byte_that_is_not_utf8_is_read_as_one_word_not_an_error() {
    let tree = Tree::new();
    assert!(std::fs::write(tree.path("bytes.md"), b"one \xFF two\n").is_ok());
    let run = judge(&tree, "bytes.md", 3);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("is 3 words, ceiling 3"), "{}", run.out);
}

/// Each instruction file is judged on its own: one the derivation commit holds under its own
/// derived ceiling, wherever an `AGENTS.md` sits, and one it lacks under the 50-word default. The
/// #361 `new-claude` and `nested` routes fail. #435.
#[test]
fn each_instruction_file_takes_its_derived_ceiling_or_the_new_file_default() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("AGENTS.md", 82);
    tree.words("src/AGENTS.md", 173);
    tree.base();
    tree.words("AGENTS.md", 105);
    tree.words("src/AGENTS.md", 191);
    tree.words("CLAUDE.md", 47);
    tree.words("api/AGENTS.md", 71);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    for said in [
        "FAIL: AGENTS.md is 105 words, over its ceiling of 100.",
        "OK: src/AGENTS.md is 191 words, ceiling 200",
        "OK: CLAUDE.md is 47 words, ceiling 50",
        "FAIL: api/AGENTS.md is 71 words, over its ceiling of 50.",
        "derived: doc_size src/AGENTS.md 200, the word count at the derivation commit",
        "derived: doc_size api/AGENTS.md 50, the 50-word default for an instruction file the \
         derivation commit lacks",
    ] {
        assert!(run.says(said), "{said}: {}", run.out);
    }
}

/// A new nested `CLAUDE.md` is no instruction file by default, and a pin overrides the
/// automatic ceiling of a nested `AGENTS.md`. #435.
#[test]
fn a_pin_overrides_the_new_file_default_and_a_nested_claude_md_is_not_judged() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "doc_size": {"api/AGENTS.md": 80} }"#);
    tree.base();
    tree.words("api/AGENTS.md", 71);
    tree.words("api/CLAUDE.md", 400);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: api/AGENTS.md is 71 words, ceiling 80"),
        "{}",
        run.out
    );
    assert!(!run.says("api/CLAUDE.md"), "{}", run.out);
}

/// Many nested instruction files are each judged in one run. #435.
#[test]
fn a_hundred_nested_instruction_files_are_each_judged() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    for at in 0..100 {
        tree.words(&format!("pkg{at}/AGENTS.md"), 10);
    }
    tree.base();
    tree.words("pkg7/AGENTS.md", 60);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("FAIL: pkg7/AGENTS.md is 60 words, over its ceiling of 50."),
        "{}",
        run.out
    );
    assert!(
        run.says("OK: pkg99/AGENTS.md is 10 words, ceiling 50"),
        "{}",
        run.out
    );
}

/// A cached ceiling set that misses an instruction file the derivation commit holds is derived
/// again, so the held file never takes the new-file default. #435.
#[test]
fn a_cache_that_misses_a_held_instruction_file_is_derived_again() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("AGENTS.md", 10);
    tree.words("src/AGENTS.md", 173);
    tree.base();
    let cache = tree.state(&format!("cache/{}.json", tree.revision("HEAD")));
    let partial = format!(
        "{{\"version\":\"{}\",\"doc_size_instructions\":{{\"AGENTS.md\":50}}}}\n",
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        cache
            .parent()
            .is_some_and(|under| std::fs::create_dir_all(under).is_ok())
    );
    assert!(std::fs::write(&cache, partial).is_ok());
    tree.words("src/AGENTS.md", 191);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: src/AGENTS.md is 191 words, ceiling 200"),
        "{}",
        run.out
    );
}

/// A changed run reads only the instruction files the change set touched. #435.
#[test]
fn a_changed_run_judges_only_the_instruction_files_that_changed() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("AGENTS.md", 10);
    tree.words("pkg/AGENTS.md", 10);
    tree.base();
    tree.words("AGENTS.md", 60);

    let run = tree.run(&["check", "doc-size", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("FAIL: AGENTS.md is 60 words, over its ceiling of 50."),
        "{}",
        run.out
    );
    assert!(!run.says("pkg/AGENTS.md is"), "{}", run.out);
}

/// A changed run still reads every pinned document, so a pinned README the change set renamed
/// is followed to its new path and named for review, as a whole run does. #435, spec 7.3.
#[test]
fn a_changed_run_still_reports_a_pinned_document_that_was_renamed() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "doc_size": {"README.md": 100} }"#);
    tree.words("README.md", 10);
    tree.base();
    tree.write("docs/.keep", "");
    tree.git(&["mv", "README.md", "docs/README.md"]);

    let run = tree.run(&["check", "doc-size", "--changed"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("docs/README.md"), "{}", run.out);
    assert!(
        run.says("REVIEW: the pinned document README.md"),
        "{}",
        run.out
    );
}

/// A new pin that names a missing file is a config error on a changed run where only the
/// configuration changed. #435.
#[test]
fn a_changed_run_reports_a_new_pin_that_names_a_missing_file() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("AGENTS.md", 10);
    tree.base();
    tree.write("klin.json", r#"{ "doc_size": {"MISSING.md": 100} }"#);

    let run = tree.run(&["check", "doc-size", "--changed"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no such file"), "{}", run.out);
}
