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
    tree.words("README.md", 400);
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
fn init_on_a_tree_in_debt_writes_a_config_that_gates_green() {
    let tree = in_debt();

    let written = tree.run(&["init"]);
    assert_eq!(written.code, 0, "{}", written.out);

    let gated = tree.run(&["gate", "--strict"]);
    assert_eq!(gated.code, 0, "{}", gated.out);
}

#[test]
fn init_writes_no_file_but_the_config() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.status(), "?? klin.json\n", "{}", run.out);
}

#[test]
fn init_writes_every_section_it_can_infer() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(
        config["complexity"]["roots"],
        serde_json::json!(["src", "tests"]),
        "{config}"
    );
    assert_eq!(
        config["escapes"]["roots"],
        serde_json::json!(["src", "tests"]),
        "{config}"
    );
    assert_eq!(
        config["escapes"]["languages"],
        serde_json::json!(["rust"]),
        "{config}"
    );
    assert_eq!(config["doc_size"][0]["file"], "README.md", "{config}");
    assert!(
        config["doc_size"][0]["ceiling"]
            .as_u64()
            .unwrap_or_default()
            >= 400,
        "{config}"
    );
    assert_eq!(config["doc_citations"][0]["file"], "README.md", "{config}");
    assert_eq!(
        config["doc_citations"][0]["roots"],
        serde_json::json!(["."]),
        "{config}"
    );
}

#[test]
fn init_writes_the_version_of_the_running_binary() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["version"], env!("CARGO_PKG_VERSION"), "{config}");
}

#[test]
fn init_writes_one_build_entry_per_manifest() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let build = config(&tree)["build"].clone();
    assert!(
        build
            .as_str()
            .unwrap_or_default()
            .starts_with("cargo build"),
        "{build}"
    );
}

#[test]
fn init_writes_a_build_entry_with_a_root_for_each_project_of_a_monorepo() {
    let tree = Tree::bare();
    tree.write("api/Cargo.toml", "[package]\nname = \"api\"\n");
    tree.write("api/src/lib.rs", "fn f() {}\n");
    tree.write("web/package.json", "{\"name\": \"web\"}\n");
    tree.write("web/tsconfig.json", "{}\n");
    tree.write("web/src/index.ts", "export const a = 1;\n");
    tree.write("service/go.mod", "module t\n");
    tree.write("service/main.go", "package main\n");
    tree.base();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let build = config(&tree)["build"].clone();
    let roots: Vec<&str> = build
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry["root"].as_str())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(roots, ["api", "service", "web"], "{build}");
}

#[test]
fn init_leaves_a_config_that_already_exists_alone() {
    let tree = in_debt();
    let mine = r#"{ "project": "mine", "doc_size": [{"file": "README.md", "ceiling": 900}] }"#;
    tree.write("klin.json", mine);

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("already"), "{}", run.out);
    assert!(run.says("--add"), "{}", run.out);
    let kept = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    assert_eq!(kept, mine, "{}", run.out);
}

#[test]
fn add_fills_in_the_sections_the_config_does_not_name() {
    let tree = in_debt();
    tree.write(
        "klin.json",
        r#"{ "project": "mine", "doc_size": [{"file": "README.md", "ceiling": 900}] }"#,
    );

    let run = tree.run(&["init", "--add"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config["project"], "mine", "{config}");
    assert_eq!(config["doc_size"][0]["ceiling"], 900, "{config}");
    assert!(config["escapes"].is_object(), "{config}");
    assert!(config["complexity"].is_object(), "{config}");
    assert!(config["doc_citations"].is_array(), "{config}");
}

#[test]
fn add_leaves_a_gate_a_person_excluded_alone() {
    let tree = in_debt();
    tree.write("klin.json", r#"{ "project": "mine", "escapes": false }"#);

    let run = tree.run(&["init", "--add"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["escapes"], Value::Bool(false), "{}", run.out);
}

#[test]
fn init_takes_no_force_flag() {
    let tree = in_debt();

    let run = tree.run(&["init", "--force"]);
    assert_eq!(run.code, 2, "{}", run.out);
}

#[test]
fn init_infers_no_section_for_a_gate_it_cannot_survey() {
    let tree = in_debt();

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let config = config(&tree);
    assert_eq!(config.get("sarif"), None, "{config}");
    assert_eq!(config.get("manifests"), None, "{config}");
}

/// `init` pins what history says, so a person can see the two numbers, edit them and put them
/// under review. The lines name them as derived and never as a gate. #92.
#[test]
fn init_pins_the_radius_values_history_derives() {
    let tree = harness::history(43, 6);

    let run = tree.run(&["init"]);
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

    let listed = tree.run(&["gate", "--list"]);
    assert!(!listed.says("radius"), "{}", listed.out);
}

#[test]
fn init_writes_no_radius_section_below_fifty_commits() {
    let tree = harness::history(42, 6);

    let run = tree.run(&["init"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(config(&tree)["radius"], Value::Null, "{}", run.out);
    assert!(
        run.says("derived: no \"radius\" section, because 49 non-merge commit(s) reach"),
        "{}",
        run.out
    );
}
