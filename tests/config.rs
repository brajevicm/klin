mod harness;

use harness::{Tree, run_from};

const ONE_DOC: &str = r#"{"doc_size": [{"file": "README.md", "ceiling": 10}]}"#;

#[test]
fn finds_the_config_by_walking_up() {
    let tree = Tree::new();
    tree.write("klin.json", ONE_DOC);
    tree.words("README.md", 5);
    tree.write("a/b/keep.txt", "");

    let deep = tree.path("a/b");
    let run = run_from(&deep, &["doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: README.md is 5 words, ceiling 10"),
        "{}",
        run.out
    );
}

#[test]
fn no_config_above_the_working_directory_is_a_tool_error() {
    let tree = Tree::new();
    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("klin.json"), "{}", run.out);
    assert!(run.says(&tree.root().display().to_string()), "{}", run.out);
}

#[test]
fn a_malformed_config_is_a_tool_error_naming_the_file() {
    let tree = Tree::new();
    tree.write("klin.json", "{not json");
    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(&tree.at("klin.json")), "{}", run.out);
}

#[test]
fn config_flag_overrides_discovery() {
    let tree = Tree::new();
    tree.write("repo/klin.json", ONE_DOC);
    tree.words("repo/README.md", 5);
    tree.write("elsewhere/klin.json", r#"{"doc_size": []}"#);

    let run = run_from(
        &tree.path("elsewhere"),
        &["doc-size", "--config", &tree.at("repo/klin.json")],
    );
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: README.md is 5 words, ceiling 10"),
        "{}",
        run.out
    );
}

#[test]
fn paths_resolve_against_the_configs_own_directory() {
    let tree = Tree::new();
    tree.write(
        "repo/klin.json",
        r#"{"doc_size": [{"file": "docs/guide.md", "ceiling": 3}]}"#,
    );
    tree.words("repo/docs/guide.md", 5);
    tree.words("docs/guide.md", 1);

    let run = run_from(
        tree.root(),
        &["doc-size", "--config", &tree.at("repo/klin.json")],
    );
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("FAIL: docs/guide.md is 5 words, over its ceiling of 3."),
        "{}",
        run.out
    );
}

#[test]
fn an_absolute_path_in_the_config_passes_through() {
    let tree = Tree::new();
    let doc = tree.words("outside.md", 5);
    tree.write(
        "repo/klin.json",
        &format!(
            r#"{{"doc_size": [{{"file": {:?}, "ceiling": 10}}]}}"#,
            doc.display().to_string()
        ),
    );

    let run = run_from(&tree.path("repo"), &["doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("is 5 words, ceiling 10"), "{}", run.out);
}

#[test]
fn a_missing_section_is_an_error_naming_the_section() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity": {}}"#);
    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
    assert!(run.says(&tree.at("klin.json")), "{}", run.out);
}

#[test]
fn a_missing_key_is_an_error_naming_the_key_not_a_default() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": [{"file": "README.md"}]}"#);
    tree.words("README.md", 5);
    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"ceiling\""), "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
}

#[test]
fn a_section_of_the_wrong_shape_is_an_error() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": {"file": "README.md", "ceiling": 10}}"#,
    );
    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
}

#[test]
fn a_tilde_path_in_the_config_expands_to_the_home_directory() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "~/klin-no-such-document.md", "ceiling": 10}]}"#,
    );
    let home = std::env::home_dir().expect("a home directory");

    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says(&home.join("klin-no-such-document.md").display().to_string()),
        "{}",
        run.out
    );
    assert!(!run.says("~"), "{}", run.out);
}

#[test]
fn a_key_of_the_wrong_type_says_it_is_malformed_not_absent() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "README.md", "ceiling": "10"}]}"#,
    );

    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"ceiling\""), "{}", run.out);
    assert!(run.says("whole number"), "{}", run.out);
    assert!(!run.says("has no"), "{}", run.out);
}

#[test]
fn a_section_naming_a_baseline_says_the_key_is_not_one_klin_reads() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"escapes": {"roots": ["src"], "languages": ["rust"],
             "baseline": "quality/escapes-baseline.json"}}"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"baseline\""), "{}", run.out);
    assert!(run.says("not a key klin reads"), "{}", run.out);
    assert!(run.says("\"accepted\""), "{}", run.out);
}
