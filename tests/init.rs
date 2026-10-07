mod harness;

use harness::Tree;
use serde_json::Value;

#[path = "fixtures/escape_text.rs"]
mod text;

const TANGLED: &str = r#"fn tangled(a: i32) -> i32 {
    if a > 0 && a < 10 {
        for x in 0..a {
            if x == 3 { return 1; }
        }
    } else if a == 0 || a == -1 || a == -2 || a == -3 {
        return 2;
    }
    match a {
        1 => 1,
        9 => 0,
    }
}
"#;

/// A tree that already holds debt: a tangled function, an escape site and a long document.
fn in_debt() -> Tree {
    let tree = Tree::bare();
    tree.write("src/knot.rs", TANGLED);
    tree.write("src/lib.rs", text::WRAPPED);
    tree.write("tests/knot.rs", TANGLED);
    tree.words("AGENTS.md", 400);
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.base();
    tree
}

fn config(tree: &Tree) -> Value {
    let Ok(text) = std::fs::read_to_string(tree.path("klin.json")) else {
        panic!("no klin.json was written")
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(why) => panic!("klin.json is not JSON: {why}\n{text}"),
    }
}

#[test]
fn setup_on_a_tree_in_debt_writes_a_config_that_gates_green() {
    let tree = in_debt();

    let written = tree.run(&["setup"]);
    assert_eq!(written.code, 0, "{}", written.out);

    let gated = tree.run(&["check"]);
    assert_eq!(gated.code, 0, "{}", gated.out);
}

#[test]
fn setup_run_twice_changes_nothing_the_second_time() {
    let tree = in_debt();

    let first = tree.run(&["setup"]);
    assert_eq!(first.code, 0, "{}", first.out);
    tree.commit("setup");

    let second = tree.run(&["setup"]);
    assert_eq!(second.code, 0, "{}", second.out);
    assert_eq!(tree.status(), "", "{}", second.out);
}

/// Plain `setup` writes the repository's opt-in marker and nothing it can derive. ADR 0028,
/// ADR 0040.
#[test]
fn setup_writes_the_empty_opt_in_marker() {
    let tree = in_debt();

    let run = tree.run(&["setup"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree), serde_json::json!({}), "{}", run.out);
}

#[test]
fn setup_leaves_a_config_that_already_exists_alone() {
    let tree = in_debt();
    let mine = r#"{ "doc_size": {"README.md": 900} }"#;
    tree.write("klin.json", mine);

    let run = tree.run(&["setup"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("already"), "{}", run.out);
    let kept = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    assert_eq!(kept, mine, "{}", run.out);
}

#[test]
fn pin_fills_in_the_guardrails_the_config_does_not_state() {
    let tree = in_debt();
    tree.write("klin.json", r#"{ "doc_size": {"README.md": 900} }"#);

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["doc_size"]["README.md"], 900, "{config}");
    assert!(config["complexity"]["cc"].is_u64(), "{config}");
    assert!(config["complexity"]["lines"].is_u64(), "{config}");
    assert!(run.says("derived: complexity cc"), "{}", run.out);
}

#[test]
fn pin_writes_no_test_lines_because_klin_never_derives_it() {
    let tree = in_debt();

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert!(config["complexity"]["lines"].is_u64(), "{config}");
    assert!(config["complexity"].get("test_lines").is_none(), "{config}");
}

#[test]
fn pin_leaves_a_gate_a_person_excluded_alone() {
    let tree = in_debt();
    tree.write("klin.json", r#"{ "escapes": false, "complexity": false }"#);

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["escapes"], Value::Bool(false), "{}", run.out);
    assert_eq!(config["complexity"], Value::Bool(false), "{}", run.out);
}

/// A tree whose two documents let one entry be re-pinned while the other keeps a schedule.
fn two_documents() -> Tree {
    let tree = Tree::bare();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
    tree.words("README.md", 400);
    tree.words("CONTEXT.md", 200);
    tree.base();
    tree
}

/// `--pin` adds today's guardrails and keeps every value a person wrote: a pinned ceiling, a
/// dated schedule, the accepted list, a gate switched off and the journal preference. #180.
#[test]
fn pin_keeps_every_value_a_person_wrote() {
    let tree = two_documents();
    let held = serde_json::json!({
        "accepted": [{"gate": "escapes", "file": "src/lib.rs", "text": "x", "count": 1}],
        "doc_size": {"README.md": 50, "CONTEXT.md": {"2020-01-01": 900}},
        "escapes": false,
        "journal": {"prompt": false}
    });
    tree.write("klin.json", &held.to_string());

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    for key in ["accepted", "doc_size", "escapes", "journal"] {
        assert_eq!(config[key], held[key], "{key}: {config}");
    }
    assert!(config["complexity"]["cc"].is_u64(), "{config}");
}

#[test]
fn pin_edits_no_gitignore() {
    let tree = two_documents();
    tree.write(".gitignore", "/target\n");
    tree.commit("an ignore file");

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.status().contains(".gitignore"), "{}", run.out);
}

/// A pin is a guardrail a person owns, and nothing that describes the repository: no build
/// command, document topology, manifest, test root or source section. ADR 0040.
#[test]
fn pin_writes_only_stable_guardrails() {
    let tree = in_debt();

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    let written: Vec<&String> = config
        .as_object()
        .map(|held| held.keys().collect())
        .unwrap_or_default();
    assert_eq!(written, ["complexity", "doc_size"], "{config}");
    assert_eq!(
        config["complexity"].as_object().map(|held| held.len()),
        Some(2),
        "{config}"
    );
    assert!(
        config["doc_size"]["AGENTS.md"].as_u64().unwrap_or_default() >= 400,
        "{config}"
    );

    let gated = tree.run(&["check"]);
    assert_eq!(gated.code, 0, "{}", gated.out);
}

/// `--pin` pins what a run derives, so a README gets no ceiling beside the instruction files.
/// #382.
#[test]
fn pin_writes_a_document_ceiling_only_for_the_instruction_files() {
    let tree = Tree::bare();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.words("AGENTS.md", 120);
    tree.words("CLAUDE.md", 20);
    tree.words("README.md", 400);
    tree.base();

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        config(&tree)["doc_size"],
        serde_json::json!({"AGENTS.md": 150, "CLAUDE.md": 50}),
        "{}",
        run.out
    );
}

/// A cache of this version that names a README is not this commit's derivation, so `--pin`
/// derives again, writes no README ceiling and leaves none in the cache. #382.
#[test]
fn pin_writes_no_readme_ceiling_a_cache_of_this_version_still_holds() {
    let tree = Tree::bare();
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.words("AGENTS.md", 120);
    tree.words("README.md", 400);
    tree.base();
    let cache = tree.state(&format!("cache/{}.json", tree.revision("HEAD")));
    let stale = format!(
        "{{\"version\":\"{}\",\"doc_size_instructions\":{{\"AGENTS.md\":999,\"README.md\":450}}}}\n",
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        cache
            .parent()
            .is_some_and(|under| std::fs::create_dir_all(under).is_ok())
    );
    assert!(std::fs::write(&cache, stale).is_ok());

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        config(&tree)["doc_size"],
        serde_json::json!({"AGENTS.md": 150}),
        "{}",
        run.out
    );
    let held: Value = serde_json::from_str(&std::fs::read_to_string(&cache).unwrap_or_default())
        .unwrap_or_default();
    assert_eq!(
        held["doc_size_instructions"],
        serde_json::json!({"AGENTS.md": 150}),
        "{held}"
    );
}

/// `setup --pin` pins what history says, so a person can see the two numbers, edit them and put them
/// under review. The lines name them as derived and never as a gate. #92.
#[test]
fn pin_writes_the_radius_values_history_derives() {
    let tree = harness::history(43, 6);

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["radius"]["lines"], 30, "{config}");
    assert_eq!(config["radius"]["directories"], 3, "{config}");
    assert!(
        run.says("derived: radius lines 30, the 90th percentile of the last 50 non-merge commits"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: radius directories 3, the 90th percentile"),
        "{}",
        run.out
    );

    let listed = tree.run(&["policy"]);
    assert!(!listed.says("radius"), "{}", listed.out);
}

#[test]
fn pin_writes_no_radius_section_below_fifty_commits() {
    let tree = harness::history(42, 6);

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["radius"], Value::Null, "{}", run.out);
    assert!(
        run.says("derived: no \"radius\" pinned, because 49 non-merge commit(s) reach"),
        "{}",
        run.out
    );
}

/// Retired source-topology keys are refused instead of silently surviving a rewrite. #179.
#[test]
fn pin_refuses_retired_source_topology() {
    let tree = two_documents();
    tree.write(
        "klin.json",
        r#"{
          "complexity": {"roots": ["src"], "exclude": ["src/generated/**"]},
          "escapes": {"roots": ["src"], "languages": ["rust"], "exclude": ["vendor/**"]}
        }"#,
    );

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads"), "{}", run.out);
}

#[test]
fn setup_omits_automatic_source_sections() {
    let tree = in_debt();

    let run = tree.run(&["setup"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config.get("stubs"), None, "{config}");
}

#[test]
fn pin_refuses_the_retired_stubs_shape() {
    let tree = in_debt();
    tree.write(
        "klin.json",
        r#"{ "stubs": { "roots": ["old"], "languages": ["go"] } }"#,
    );

    let run = tree.run(&["setup", "--pin"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads"), "{}", run.out);
}

/// The snapshot flags are gone with the snapshot they wrote (#180), and the old hook flags are
/// gone because setup owns host integration through `--host` (#216).
#[test]
fn the_retired_flags_are_usage_errors() {
    for flag in ["--add", "--force", "--hooks", "--global"] {
        let tree = in_debt();

        let run = tree.run(&["setup", flag]);
        assert_eq!(run.code, 2, "{flag}: {}", run.out);
        assert!(!tree.path("klin.json").exists(), "{flag}: {}", run.out);
    }
}
