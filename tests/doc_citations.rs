use crate::harness::Tree;

const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

#[test]
fn citations_that_resolve_pass_counting_them() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("src/views/page.ts", "export {}\n");
    tree.write("README.md", "hi\n");
    let doc = tree.write(
        "docs/arch.md",
        "The store is `src/store.py` and the page `src/views/page.ts:12`.\n\
         Run `python3 tool.py --all` or `make`; see `README.md`.\n\
         A `.env` is not cited; nor `foo.bar`.\n",
    );
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 0 citation(s) resolve nowhere ("),
        "{}",
        run.out
    );
}

#[test]
fn quiet_prints_nothing_on_success() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
        "--quiet",
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn a_moved_file_fails_naming_the_document_the_line_and_the_path() {
    let tree = Tree::new();
    tree.write("src/data/store.py", "x = 2\n");
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    let name = doc.display().to_string();
    let run = tree.run(&["doc-citations", "--file", &name, "--root", &tree.at("")]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(&format!(
            "{name}:1  not under the roots — likely src/data/store.py  src/store.py"
        )),
        "{}",
        run.out
    );
    assert!(run.says("1 new citation(s)"), "{}", run.out);
}

#[test]
fn the_failure_says_what_fixes_the_code() {
    let tree = Tree::new();
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("Point the citation"), "{}", run.out);
}

#[test]
fn a_bare_filename_resolves_when_exactly_one_file_under_the_roots_has_that_name() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    let doc = tree.write("docs/arch.md", "See `store.py` and `nowhere.py`.\n");
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new citation(s)"), "{}", run.out);
    assert!(
        run.says("no file of that name under the roots  nowhere.py"),
        "{}",
        run.out
    );
}

#[test]
fn two_candidates_is_ambiguity_reported_with_both() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("lib/store.py", "x = 3\n");
    let doc = tree.write("docs/arch.md", "See `store.py`.\n");
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("ambiguous — cite one"), "{}", run.out);
    assert!(run.says("lib/store.py"), "{}", run.out);
    assert!(run.says("src/store.py"), "{}", run.out);
}

#[test]
fn a_span_that_is_not_a_path_is_not_read() {
    let tree = Tree::new();
    let doc = tree.write(
        "docs/arch.md",
        "Run `python3 tool.py --all` or `make`; a `.env` is not cited; nor `foo.bar`.\n",
    );
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn a_wildcard_a_link_and_a_line_suffix_are_read_as_the_syntax_says() {
    let tree = Tree::new();
    tree.write("src/a.rs", "");
    let doc = tree.write(
        "docs/arch.md",
        "See `src/a.rs:12`, every `*.rs`, [the gone one](src/gone.rs) and `src/gone.rs:3`.\n",
    );
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 1 new citation(s)"), "{}", run.out);
    assert!(run.says("src/gone.rs"), "{}", run.out);
    assert!(!run.says("src/a.rs"), "{}", run.out);
    assert!(!run.says("*.rs"), "{}", run.out);
}

#[test]
fn a_missing_document_is_a_tool_error() {
    let tree = Tree::new();
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &tree.at("missing.md"),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no such file"), "{}", run.out);
}

#[test]
fn a_root_that_does_not_exist_is_a_tool_error() {
    let tree = Tree::new();
    let doc = tree.write("docs/arch.md", "hi\n");
    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at("no-such-dir"),
    ]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no such directory"), "{}", run.out);
}

/// With no section every Markdown file at the tree root is read against the whole tree, and a
/// document below the root is not. ADR 0040.
#[test]
fn every_document_at_the_tree_root_is_judged_with_no_configuration() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("README.md", "The store is `src/store.py`.\n");
    tree.write("CONTEXT.md", "See `store.py`.\n");
    tree.write("docs/arch.md", "The gone one is `src/gone.py`.\n");
    tree.base();

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(
            "derived: doc_citations CONTEXT.md, README.md, every Markdown file at the tree root"
        ),
        "{}",
        run.out
    );
    assert!(run.says("OK: 0 citation(s) resolve nowhere"), "{}", run.out);
    assert!(run.says("(2 file(s) found, 2 measured"), "{}", run.out);
}

#[test]
fn a_file_alone_resolves_against_the_tree_root() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");

    let run = tree.run(&["doc-citations", "--file", &doc.display().to_string()]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn a_stale_citation_the_base_holds_is_held_and_the_run_passes() {
    let tree = Tree::new();
    tree.write("ARCH.md", "The store is `src/store.py`.\n");
    tree.base();

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("\"status\":\"PASS\""), "{}", run.out);
    assert!(!run.says("src/store.py"), "{}", run.out);
}

#[test]
fn a_citation_added_in_the_working_tree_that_resolves_nowhere_fails() {
    let tree = Tree::new();
    tree.write("ARCH.md", "Nothing is cited here.\n");
    tree.base();
    tree.write(
        "ARCH.md",
        "Nothing is cited here.\nThe store is `src/store.py`.\n",
    );

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("ARCH.md:2  not under the roots  src/store.py"),
        "{}",
        run.out
    );
}

/// `doc-size` derives no ceiling for a README, and `doc-citations` still reads every Markdown
/// file at the tree root. #382.
#[test]
fn a_root_readme_doc_size_does_not_judge_is_still_read_for_citations() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("README.md", "Nothing is cited here.\n");
    tree.base();
    tree.write(
        "README.md",
        "Nothing is cited here.\nThe store is `src/store.py`.\n",
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-citations"), "{}", run.out);
    assert!(
        run.says("README.md:2  not under the roots  src/store.py"),
        "{}",
        run.out
    );
    assert!(!run.says("derived: doc_size"), "{}", run.out);
}

#[test]
fn a_citation_whose_target_moved_fails_and_the_remedy_names_the_moved_file() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("ARCH.md", "The store is `src/store.py`.\n");
    tree.base();
    tree.remove("src/store.py");
    tree.write("src/data/store.py", "x = 1\n");

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Delete the sentence only when the referenced content was intentionally removed and the sentence no longer applies"),
        "{}",
        run.out
    );
    assert!(
        run.says("A move or rename calls for updating the citation"),
        "{}",
        run.out
    );
    assert!(
        run.says("ARCH.md:1  not under the roots — likely src/data/store.py  src/store.py"),
        "{}",
        run.out
    );
}

#[test]
fn a_stale_citation_moved_to_another_line_is_held() {
    let tree = Tree::new();
    tree.write("ARCH.md", "The store is `src/store.py`.\n");
    tree.base();
    tree.write("ARCH.md", "One.\nTwo.\nThe store is `src/store.py`.\n");

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 1 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn the_same_stale_string_cited_once_more_is_worsened_with_the_count() {
    let tree = Tree::new();
    tree.write(
        "ARCH.md",
        "The store is `src/store.py`, and again `src/store.py`.\n",
    );
    tree.base();
    tree.write(
        "ARCH.md",
        "The store is `src/store.py`, and again `src/store.py`.\nOnce more: `src/store.py`.\n",
    );

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 citation(s) got worse"), "{}", run.out);
    assert!(
        run.says("not under the roots x3, was not under the roots x2"),
        "{}",
        run.out
    );
}

#[test]
fn a_stale_citation_fixed_in_the_working_tree_prints_nothing() {
    let tree = Tree::new();
    tree.write("ARCH.md", "The store is `src/store.py`.\n");
    tree.base();
    tree.write("src/store.py", "x = 1\n");

    let run = tree.run(&["doc-citations", "--quiet"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn under_changed_a_move_breaks_the_citation_of_a_document_the_window_did_not_touch() {
    let tree = Tree::new();
    tree.write("src/client.py", "x = 1\n");
    tree.write("src/index.py", "y = 1\n");
    tree.write("README.md", "The client is `src/client.py`.\n");
    tree.base();
    tree.remove("src/client.py");
    tree.write("src/transport/client.py", "x = 1\n");
    tree.write("src/index.py", "y = 2\n");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("README.md:1"), "{}", run.out);
    assert!(run.says("likely src/transport/client.py"), "{}", run.out);
}

#[test]
fn the_stop_hook_blocks_on_a_move_that_breaks_an_untouched_documents_citation() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/client.py", "x = 1\n");
    tree.write("src/index.py", "y = 1\n");
    tree.write("README.md", "The client is `src/client.py`.\n");
    tree.base();
    let prompt = crate::harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(prompt.code, 0, "{}", prompt.out);
    tree.remove("src/client.py");
    tree.write("src/transport/client.py", "x = 1\n");
    tree.write("src/index.py", "y = 2\n");

    let run = crate::harness::feed(tree.root(), &["gate", "--hook", "--changed"], A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("README.md:1"), "{}", run.out);
}

#[test]
fn under_changed_a_stale_citation_of_an_untouched_document_stays_held() {
    let tree = Tree::new();
    tree.write("src/index.py", "y = 1\n");
    tree.write("README.md", "The store is `src/store.py`.\n");
    tree.base();
    tree.write("src/index.py", "y = 2\n");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("README.md:"), "{}", run.out);
}

#[test]
fn under_changed_a_new_basename_clash_makes_an_untouched_citation_ambiguous() {
    let tree = Tree::new();
    tree.write("src/client.py", "x = 1\n");
    tree.write("README.md", "The client is `client.py`.\n");
    tree.base();
    tree.write("src/other/client.py", "x = 2\n");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("README.md:1"), "{}", run.out);
}

#[test]
fn under_changed_removing_a_basename_clash_leaves_an_untouched_citation_silent() {
    let tree = Tree::new();
    tree.write("src/client.py", "x = 1\n");
    tree.write("src/other/client.py", "x = 2\n");
    tree.write("README.md", "The client is `client.py`.\n");
    tree.base();
    tree.remove("src/other/client.py");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("README.md:"), "{}", run.out);
}

#[test]
fn a_document_the_window_added_has_no_base_and_its_stale_citations_are_new() {
    let tree = Tree::new();
    tree.write("ARCH.md", "Nothing here.\n");
    tree.base();
    tree.write("NEW.md", "The store is `src/store.py`.\n");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("NEW.md:1"), "{}", run.out);
}

#[test]
fn under_changed_the_base_side_reads_the_whole_base_tree_not_only_the_changed_files() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("ARCH.md", "The store is `src/store.py`.\n");
    tree.base();
    tree.remove("src/store.py");
    tree.write("ARCH.md", "A word.\nThe store is `src/store.py`.\n");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new citation(s)"), "{}", run.out);
}

#[test]
fn file_and_root_by_hand_judge_that_document_against_the_base() {
    let tree = Tree::new();
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.base();

    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &tree.at(""),
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 1 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn a_base_listing_git_refuses_is_a_tool_error_not_a_green_run() {
    let tree = Tree::new();
    let outside = Tree::bare();
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.base();

    let run = tree.run(&[
        "doc-citations",
        "--file",
        &doc.display().to_string(),
        "--root",
        &outside.at(""),
    ]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("could not be listed under"), "{}", run.out);
}

#[test]
fn a_document_deleted_in_the_window_neither_fails_nor_errors() {
    let tree = Tree::new();
    tree.write("README.md", "Nothing is cited here.\n");
    tree.write("ARCH.md", "The store is `src/store.py`.\n");
    tree.base();
    tree.remove("ARCH.md");

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("no such file"), "{}", run.out);
}
