mod harness;

use harness::{Tree, run_from};

#[test]
fn a_document_under_its_ceiling_passes_and_prints_both_numbers() {
    let tree = Tree::new();
    let doc = tree.words("small.md", 5);
    let run = tree.run(&[
        "doc-size",
        "--file",
        &doc.display().to_string(),
        "--ceiling",
        "10",
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("is 5 words, ceiling 10"), "{}", run.out);
}

#[test]
fn quiet_prints_nothing_on_success() {
    let tree = Tree::new();
    let doc = tree.words("small.md", 5);
    let run = tree.run(&[
        "doc-size",
        "--file",
        &doc.display().to_string(),
        "--ceiling",
        "10",
        "--quiet",
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn a_document_over_its_ceiling_fails_naming_the_count_and_the_ceiling() {
    let tree = Tree::new();
    let doc = tree.words("small.md", 5);
    let run = tree.run(&[
        "doc-size",
        "--file",
        &doc.display().to_string(),
        "--ceiling",
        "4",
    ]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("is 5 words, over its ceiling of 4"), "{}", run.out);
}

#[test]
fn a_missing_document_is_a_tool_error() {
    let tree = Tree::new();
    let run = tree.run(&[
        "doc-size",
        "--file",
        &tree.at("missing.md"),
        "--ceiling",
        "10",
    ]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no such file"), "{}", run.out);
}

#[test]
fn a_document_inside_the_margin_warns_even_under_quiet() {
    let tree = Tree::new();
    let doc = tree.words("margin.md", 99);
    let name = doc.display().to_string();
    let run = tree.run(&["doc-size", "--file", &name, "--ceiling", "100", "--quiet"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(&format!(
            "WARN: {name} is 99 words, 1 from its ceiling of 100."
        )),
        "{}",
        run.out
    );
}

#[test]
fn a_document_inside_the_margin_prints_the_ok_line_too() {
    let tree = Tree::new();
    let doc = tree.words("margin.md", 99);
    let name = doc.display().to_string();
    let run = tree.run(&["doc-size", "--file", &name, "--ceiling", "100"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(&format!("OK: {name} is 99 words, ceiling 100")),
        "{}",
        run.out
    );
    assert!(run.says("WARN:"), "{}", run.out);
}

#[test]
fn a_document_outside_the_margin_prints_nothing_under_quiet() {
    let tree = Tree::new();
    let doc = tree.words("outside.md", 97);
    let run = tree.run(&[
        "doc-size",
        "--file",
        &doc.display().to_string(),
        "--ceiling",
        "100",
        "--quiet",
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn a_document_over_its_ceiling_fails_with_no_warn_in_its_place() {
    let tree = Tree::new();
    let doc = tree.words("over.md", 101);
    let run = tree.run(&[
        "doc-size",
        "--file",
        &doc.display().to_string(),
        "--ceiling",
        "100",
    ]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("WARN:"), "{}", run.out);
}

#[test]
fn the_failure_says_what_fixes_the_code() {
    let tree = Tree::new();
    let doc = tree.words("over.md", 101);
    let run = tree.run(&[
        "doc-size",
        "--file",
        &doc.display().to_string(),
        "--ceiling",
        "100",
    ]);
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
        r#"{"doc_size": [{"file": "small.md", "ceiling": 10}, {"file": "big.md", "ceiling": 6}]}"#,
    );

    let run = tree.run(&["doc-size"]);
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
fn file_without_ceiling_takes_the_ceiling_from_the_config() {
    let tree = Tree::new();
    let doc = tree.words("small.md", 5);
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "small.md", "ceiling": 10}]}"#,
    );

    let run = tree.run(&["doc-size", "--file", &doc.display().to_string()]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("is 5 words, ceiling 10"), "{}", run.out);
}

#[test]
fn file_the_config_does_not_list_is_a_tool_error_naming_it() {
    let tree = Tree::new();
    let doc = tree.words("stray.md", 5);
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "small.md", "ceiling": 10}]}"#,
    );

    let run = tree.run(&["doc-size", "--file", &doc.display().to_string()]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("stray.md"), "{}", run.out);
    assert!(run.says("--ceiling"), "{}", run.out);
}

#[test]
fn a_document_the_config_lists_but_the_tree_lacks_is_a_tool_error() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "gone.md", "ceiling": 10}]}"#,
    );
    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no such file"), "{}", run.out);
}

#[test]
fn an_empty_list_of_documents_passes() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": []}"#);
    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn this_repositorys_own_documents_are_under_their_ceilings() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let run = run_from(repo, &["doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
}
