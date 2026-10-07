mod harness;

use harness::{Tree, run_from};

const ONE_DOC: &str = r#"{"doc_size": {"README.md": 10}}"#;

#[test]
fn finds_the_config_by_walking_up() {
    let tree = Tree::new();
    tree.write("klin.json", ONE_DOC);
    tree.words("README.md", 5);
    tree.write("a/b/keep.txt", "");

    let deep = tree.path("a/b");
    let run = run_from(&deep, &["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: README.md is 5 words, ceiling 10"),
        "{}",
        run.out
    );
}

#[test]
fn a_malformed_config_is_a_tool_error_naming_the_file() {
    let tree = Tree::new();
    tree.write("klin.json", "{not json");
    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(&tree.at("klin.json")), "{}", run.out);
}

#[test]
fn structural_config_shapes_fail_before_any_gate_runs() {
    for config in [r#"[]"#, r#"{"journal":{"prompt":"yes"}}"#] {
        let tree = Tree::new();
        tree.write("klin.json", config);
        tree.words("README.md", 1);

        let run = tree.run(&["check", "doc-size"]);
        assert_eq!(run.code, 2, "{config}: {}", run.out);
        assert!(run.says(&tree.at("klin.json")), "{config}: {}", run.out);
    }
}

#[test]
fn declared_shapes_accept_and_reject_the_same_config_families() {
    valid_shape_configs();
    false_disables_a_gate();
    invalid_shape_configs();
}

fn valid_shape_configs() {
    for (name, config) in [
        ("root", r#"{}"#),
        ("journal", r#"{"journal":{"prompt":false}}"#),
        (
            "scope",
            r#"{"complexity":{"in":"src","except":["src/generated"]}}"#,
        ),
        ("ceiling", r#"{"doc_size":{"README.md":{"2000-01-01":10}}}"#),
        (
            "convention",
            r#"{"conventions":{"no-todo":{"text":"TODO","remedy":"remove it"}}}"#,
        ),
        (
            "layering",
            r#"{"layering":{"layers":{"app":{"in":"src","can_use":null}}}}"#,
        ),
        (
            "sarif",
            r#"{"sarif":[{"name":"scanner","report":"report.sarif","differential":true}]}"#,
        ),
    ] {
        let tree = Tree::new();
        tree.words("README.md", 1);
        tree.write("src/lib.rs", "fn main() {}\n");
        tree.write("klin.json", config);

        let run = tree.run(&["policy"]);
        assert_eq!(run.code, 0, "valid {name}: {}", run.out);
    }
}

fn false_disables_a_gate() {
    let tree = Tree::new();
    tree.words("README.md", 1);
    tree.write("klin.json", r#"{"doc_size":false}"#);
    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "false disables: {}", run.out);
    assert!(run.says("doc-size — excluded"), "{}", run.out);
}

fn invalid_shape_configs() {
    for (name, config, fragment) in [
        ("root", r#"[]"#, "must be an object"),
        ("journal", r#"{"journal":{"prompt":"yes"}}"#, "prompt"),
        ("scope", r#"{"complexity":{"in":1}}"#, "in"),
        ("ceiling", r#"{"complexity":{"cc":"10"}}"#, "cc"),
        (
            "convention",
            r#"{"conventions":{"bad":{"text":"TODO","code":"TODO","remedy":"remove it"}}}"#,
            "exactly one",
        ),
        (
            "layering",
            r#"{"layering":{"layers":{"app":{"in":"src","can_use":true}}}}"#,
            "can_use",
        ),
        (
            "sarif",
            r#"{"sarif":[{"name":"scanner","report":"report.sarif","unknown":true}]}"#,
            "unknown field",
        ),
    ] {
        let tree = Tree::new();
        tree.write("klin.json", config);

        let run = tree.run(&["policy"]);
        assert_eq!(run.code, 2, "invalid {name}: {}", run.out);
        assert!(run.says(fragment), "invalid {name}: {}", run.out);
        assert!(
            run.says(&tree.at("klin.json")),
            "invalid {name}: {}",
            run.out
        );
    }
}

#[test]
fn config_flag_overrides_discovery() {
    let tree = Tree::new();
    tree.write("repo/klin.json", ONE_DOC);
    tree.words("repo/README.md", 5);
    tree.write("elsewhere/klin.json", r#"{"doc_size": false}"#);

    let run = run_from(
        &tree.path("elsewhere"),
        &["check", "doc-size", "--config", &tree.at("repo/klin.json")],
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
    tree.write("repo/klin.json", r#"{"doc_size": {"docs/guide.md": 3}}"#);
    tree.words("repo/docs/guide.md", 5);
    tree.words("docs/guide.md", 1);

    let run = run_from(
        tree.root(),
        &["check", "doc-size", "--config", &tree.at("repo/klin.json")],
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
        &format!(r#"{{"doc_size": {{{:?}: 10}}}}"#, doc.display().to_string()),
    );

    let run = run_from(&tree.path("repo"), &["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("is 5 words, ceiling 10"), "{}", run.out);
}

#[test]
fn a_retired_project_key_is_an_error_naming_the_file_and_the_key() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"project": "mine"}"#);
    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"project\""), "{}", run.out);
    assert!(run.says("delete the key"), "{}", run.out);
    assert!(run.says(&tree.at("klin.json")), "{}", run.out);
}

#[test]
fn an_empty_compact_source_policy_is_an_error() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"stubs": {}}"#);
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["check", "stubs"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("must state at least one of: in, except"),
        "{}",
        run.out
    );
}

/// The retired entry list names each document twice, as a key and as a `file`, so it is refused
/// whole with the map that replaces it. ADR 0040.
#[test]
fn a_doc_size_entry_list_is_an_error_naming_the_map_that_replaces_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": [{"file": "README.md", "ceiling": 10}]}"#,
    );
    tree.words("README.md", 5);
    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
    assert!(run.says(r#"{"README.md": 1200}"#), "{}", run.out);
}

#[test]
fn a_section_of_the_wrong_shape_is_an_error() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": {"file": "README.md", "ceiling": 10}}"#,
    );
    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
}

#[test]
fn a_tilde_path_in_the_config_expands_to_the_home_directory() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": {"~/klin-no-such-document.md": 10}}"#,
    );
    let home = Tree::bare();
    let at = home.root().display().to_string();

    let run = tree.run_with(&[("HOME", at.as_str())], &["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says(&home.at("klin-no-such-document.md")),
        "{}",
        run.out
    );
    assert!(!run.says("no such file: ~"), "{}", run.out);
}

#[test]
fn a_ceiling_of_the_wrong_type_says_it_is_malformed_not_absent() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("klin.json", r#"{"doc_size": {"README.md": "10"}}"#);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"README.md\""), "{}", run.out);
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

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"baseline\""), "{}", run.out);
    assert!(run.says("not a key klin reads"), "{}", run.out);
    assert!(run.says("\"accepted\""), "{}", run.out);
}

#[test]
fn a_section_naming_sources_says_the_key_is_now_roots() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"complexity": {"sources": ["src"], "ceilings": {"cc": 8, "lines": 60}}}"#,
    );
    tree.write("src/lib.rs", "fn f() {}\n");

    let run = tree.run(&["check", "complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"sources\""), "{}", run.out);
    assert!(run.says("\"roots\""), "{}", run.out);
}

#[test]
fn source_gates_reject_retired_repository_description() {
    for (section, field) in [
        ("complexity", "ceilings"),
        ("escapes", "patterns"),
        ("stubs", "languages"),
        ("dead_symbols", "roots"),
        ("reachability", "skip_dirs"),
    ] {
        let tree = Tree::new();
        tree.write("klin.json", &format!(r#"{{"{section}":{{"{field}":[]}}}}"#));

        let run = tree.run(&["policy"]);

        assert_eq!(run.code, 2, "{section}.{field}: {}", run.out);
        assert!(run.says(&format!("\"{field}\"")), "{}", run.out);
        assert!(run.says("in\" / \"except"), "{}", run.out);
    }
}

#[test]
fn a_version_key_is_a_tool_error_under_any_command() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"version": "0.1.1", "doc_size": {"README.md": 10}}"#,
    );
    tree.words("README.md", 5);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"version\""), "{}", run.out);
}

#[test]
fn an_unknown_top_level_key_is_a_tool_error_naming_the_file_and_the_key() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_sizes": [{"file": "README.md", "ceiling": 10}],
            "doc_size": {"README.md": 10}}"#,
    );
    tree.words("README.md", 5);

    for args in [&["check"][..], &["check"][..]] {
        let run = tree.run(args);
        assert_eq!(run.code, 2, "{args:?}: {}", run.out);
        assert!(run.says(&tree.at("klin.json")), "{args:?}: {}", run.out);
        assert!(run.says("\"doc_sizes\""), "{args:?}: {}", run.out);
        assert!(!run.says("gate(s)"), "{args:?}: {}", run.out);
    }
}

#[test]
fn a_schedule_with_no_step_due_today_is_a_tool_error_before_any_gate_runs() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"doc_size": {"README.md": {"2999-01-01": 10}}}"#,
    );
    tree.words("README.md", 5);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says(&tree.at("klin.json")), "{}", run.out);
    assert!(run.says("no step due"), "{}", run.out);
    assert!(!run.says("gate(s)"), "{}", run.out);
}

/// Every section that once held generated topology refuses it and names what replaced it, before
/// any gate runs. ADR 0040.
#[test]
fn a_section_that_describes_the_repository_is_refused_with_its_replacement() {
    for (config, said) in [
        (
            r#"{"doc_citations": [{"file": "README.md", "roots": ["."]}]}"#,
            "no longer accepts a list of entries",
        ),
        (
            r#"{"doc_citations": {"extensions": [".py"]}}"#,
            "reads no policy",
        ),
        (
            r#"{"inventory": [{"name": "tests", "path": "tests"}]}"#,
            "no longer accepts a list of entries",
        ),
        (
            r#"{"inventory": {"pattern": "*_test.go"}}"#,
            "no longer reads \"pattern\"",
        ),
        (
            r#"{"lockfile": {"manifests": ["Cargo.toml"]}}"#,
            "no longer reads \"manifests\"",
        ),
        (
            r#"{"lockfile": {"exclude": ["Cargo.toml"]}}"#,
            "no longer reads \"exclude\"",
        ),
    ] {
        let tree = Tree::new();
        tree.write("klin.json", config);

        let run = tree.run(&["policy"]);
        assert_eq!(run.code, 2, "{config}: {}", run.out);
        assert!(run.says(said), "{config}: {}", run.out);
    }
}

/// A misspelt field measures nothing, so it is refused, naming the field a person most likely
/// meant, at the top level and inside a section alike. #42.
#[test]
fn a_misspelt_key_or_field_names_the_one_it_most_likely_meant() {
    for (config, meant) in [
        (r#"{"doc_sizes": false}"#, "Did you mean \"doc_size\"?"),
        (
            r#"{"inventory": {"exept": "tests"}}"#,
            "Did you mean \"except\"?",
        ),
        (r#"{"radius": {"line": 10}}"#, "Did you mean \"lines\"?"),
        (
            r#"{"journal": {"promt": false}}"#,
            "Did you mean \"prompt\"?",
        ),
        (
            r#"{"build": [{"run": "make", "roto": "api"}]}"#,
            "Did you mean \"root\"?",
        ),
    ] {
        let tree = Tree::new();
        tree.write("klin.json", config);

        let run = tree.run(&["policy"]);
        assert_eq!(run.code, 2, "{config}: {}", run.out);
        assert!(run.says(meant), "{config}: {}", run.out);
    }
}

/// A `build` of the wrong type is refused when the config loads, so a run outside the hook sees
/// it too. Spec 5.2.
#[test]
fn a_build_of_the_wrong_type_is_a_config_error_before_any_gate_runs() {
    for config in [
        r#"{"build": 42}"#,
        r#"{"build": ["cargo build"]}"#,
        r#"{"build": true}"#,
    ] {
        let tree = Tree::new();
        tree.write("klin.json", config);

        let run = tree.run(&["policy"]);
        assert_eq!(run.code, 2, "{config}: {}", run.out);
        assert!(run.says("\"build\" is a command"), "{config}: {}", run.out);
    }
}
