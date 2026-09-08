mod harness;

use harness::Tree;

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
    assert!(run.says("all 3 cited path(s) resolve"), "{}", run.out);
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
        run.says(&format!("{name}:1  `src/store.py` — not under the roots")),
        "{}",
        run.out
    );
    assert!(
        run.says("1 path(s) that resolve nowhere under"),
        "{}",
        run.out
    );
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
    assert!(!run.says("`store.py`"), "{}", run.out);
    assert!(
        run.says("`nowhere.py` — no file of that name under the roots"),
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
    assert!(run.says("`store.py` — ambiguous"), "{}", run.out);
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
    assert!(run.says("all 0 cited path(s) resolve"), "{}", run.out);
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
    assert!(run.says("OK: docs/arch.md"), "{}", run.out);
    assert!(run.says("OK: README.md — all 0"), "{}", run.out);
}

#[test]
fn a_file_alone_reads_that_documents_config_entry() {
    let tree = Tree::new();
    tree.write("src/store.py", "x = 1\n");
    let doc = tree.write("docs/arch.md", "The store is `src/store.py`.\n");
    tree.write(
        "klin.json",
        r#"{"doc_citations": [{"file": "docs/arch.md", "roots": ["."]}]}"#,
    );

    let run = tree.run(&["doc-citations", "--file", &doc.display().to_string()]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("all 1 cited path(s) resolve"), "{}", run.out);
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
    assert!(run.says("all 1 cited path(s) resolve"), "{}", run.out);
}
