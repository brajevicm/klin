mod harness;

use harness::Tree;

const CONFIG: &str = r#"{"doc_citations": [{"file": "docs/arch.md", "roots": ["."]}]}"#;

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
        run.says("OK: 0 citation(s) resolve nowhere, all held at the base"),
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

#[test]
fn the_configs_list_judges_each_document_against_its_roots() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("README.md", "hi\n");
    tree.write(
        "klin.json",
        r#"{"doc_citations": [{"file": "docs/arch.md", "roots": ["."]},
                               {"file": "README.md", "roots": ["src"], "extensions": [".py"]}]}"#,
    );

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn a_file_alone_reads_that_documents_config_entry() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("klin.json", CONFIG);

    let run = tree.run(&["doc-citations", "--file", &doc.display().to_string()]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn a_file_with_neither_a_root_nor_a_config_entry_is_a_tool_error() {
    let tree = Tree::new();
    let doc = tree.write("docs/arch.md", "hi\n");
    tree.write("klin.json", r#"{"doc_citations": []}"#);
    let run = tree.run(&["doc-citations", "--file", &doc.display().to_string()]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no \"doc_citations\" entry"), "{}", run.out);
    assert!(run.says("--root"), "{}", run.out);
}

#[test]
fn extensions_key_replaces_the_default_list_rather_than_adding_to_it() {
    let tree = Tree::new();
    tree.write("src/store.rs", "fn main() {}\n");
    tree.write("src/store.py", "x = 1\n");
    tree.write("docs/arch.md", "See `src/store.rs` and `src/store.py`.\n");
    tree.write(
        "klin.json",
        r#"{"doc_citations": [{"file": "docs/arch.md", "roots": ["."], "extensions": [".py"]}]}"#,
    );

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn a_stale_citation_the_base_holds_is_held_and_the_run_passes() {
    let tree = Tree::new();
    tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("klin.json", CONFIG);
    tree.base();

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("\"status\":\"PASS\""), "{}", run.out);
    assert!(!run.says("src/store.py"), "{}", run.out);
}

#[test]
fn a_citation_added_in_the_working_tree_that_resolves_nowhere_fails() {
    let tree = Tree::new();
    tree.write("docs/arch.md", "Nothing is cited here.\n");
    tree.write("klin.json", CONFIG);
    tree.base();
    tree.write(
        "docs/arch.md",
        "Nothing is cited here.\nThe store is `src/store.py`.\n",
    );

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("docs/arch.md:2  not under the roots  src/store.py"),
        "{}",
        run.out
    );
}

#[test]
fn a_citation_whose_target_moved_fails_and_the_remedy_names_the_moved_file() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("klin.json", CONFIG);
    tree.base();
    tree.remove("src/store.py");
    tree.write("src/data/store.py", "x = 1\n");

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("docs/arch.md:1  not under the roots — likely src/data/store.py  src/store.py"),
        "{}",
        run.out
    );
}

#[test]
fn a_stale_citation_moved_to_another_line_is_held() {
    let tree = Tree::new();
    tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("klin.json", CONFIG);
    tree.base();
    tree.write("docs/arch.md", "One.\nTwo.\nThe store is `src/store.py`.\n");

    let run = tree.run(&["doc-citations"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 1 citation(s) resolve nowhere"), "{}", run.out);
}

#[test]
fn the_same_stale_string_cited_once_more_is_worsened_with_the_count() {
    let tree = Tree::new();
    tree.write(
        "docs/arch.md",
        "The store is `src/store.py`, and again `src/store.py`.\n",
    );
    tree.write("klin.json", CONFIG);
    tree.base();
    tree.write(
        "docs/arch.md",
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
    tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("klin.json", CONFIG);
    tree.base();
    tree.write("src/store.py", "x = 1\n");

    let run = tree.run(&["doc-citations", "--quiet"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn under_changed_a_document_the_window_did_not_touch_is_out_of_scope() {
    let tree = Tree::new();
    tree.write("docs/held.md", "The store is `src/store.py`.\n");
    tree.write("docs/edited.md", "Nothing here.\n");
    tree.write(
        "klin.json",
        r#"{"doc_citations": [{"file": "docs/held.md", "roots": ["."]},
                               {"file": "docs/edited.md", "roots": ["."]}]}"#,
    );
    tree.base();
    tree.write("docs/edited.md", "Nothing here.\nNow `src/gone.py`.\n");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("docs/edited.md:2"), "{}", run.out);
    assert!(!run.says("docs/held.md"), "{}", run.out);
}

#[test]
fn a_document_the_window_added_has_no_base_and_its_stale_citations_are_new() {
    let tree = Tree::new();
    tree.write("docs/arch.md", "Nothing here.\n");
    tree.write(
        "klin.json",
        r#"{"doc_citations": [{"file": "docs/arch.md", "roots": ["."]},
                               {"file": "docs/new.md", "roots": ["."]}]}"#,
    );
    tree.base();
    tree.write("docs/new.md", "The store is `src/store.py`.\n");

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("docs/new.md:1"), "{}", run.out);
}

#[test]
fn under_changed_the_base_side_reads_the_whole_base_tree_not_only_the_changed_files() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("klin.json", CONFIG);
    tree.base();
    tree.remove("src/store.py");
    tree.write("docs/arch.md", "A word.\nThe store is `src/store.py`.\n");

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
    tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write("klin.json", CONFIG);
    tree.base();
    tree.remove("docs/arch.md");

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("no such file"), "{}", run.out);
}
