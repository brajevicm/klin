mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

fn capability(json: &Value, name: &str) -> Value {
    json["capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["name"] == name)
        .cloned()
        .unwrap_or_default()
}

/// The indented lines under one row of the text, and nothing when the row is not there.
fn block(out: &str, row: &str) -> String {
    out.lines()
        .skip_while(|line| *line != row)
        .skip(1)
        .take_while(|line| line.starts_with(' '))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `derived:` and `pinned:` lines a run printed, without their indent.
fn provenance(out: &str) -> Vec<String> {
    out.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("derived:") || line.starts_with("pinned:"))
        .map(str::to_string)
        .collect()
}

/// `policy` derives from the configuration, the survey and the derivation commit, so a base it
/// could not lay out does not stop it. Spec 11.6.
#[test]
fn policy_lays_out_no_base_and_still_derives_each_value() {
    let tree = tree(r#"{"complexity": {"cc": 8}}"#);

    let run = tree.run_with(&[("TMPDIR", "/nonexistent/klin-tmp")], &["policy"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("pinned: complexity cc 8"), "{}", run.out);
    assert!(
        run.says("derived: complexity lines 25 (the floor of 25"),
        "{}",
        run.out
    );
    assert!(run.says("derived: doc_citations README.md"), "{}", run.out);
}

#[test]
fn policy_writes_nothing_to_the_state_directory() {
    let tree = tree(r#"{"complexity": {"cc": 8}}"#);

    let run = tree.run(&["policy"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.state("").exists(), "{}", run.out);
}

/// One function derives a value for `policy` and for a run, so the two print it alike.
#[test]
fn policy_of_one_section_prints_each_value_as_a_check_of_it_does() {
    let tree = tree(r#"{"complexity": {"cc": {"2020-01-01": 12, "2099-01-01": 8}}}"#);

    let check = tree.run(&["check", "complexity"]);
    let policy = tree.run(&["policy", "complexity"]);

    assert_eq!(policy.code, 0, "{}", policy.out);
    let checked = provenance(&check.out);
    assert!(!checked.is_empty(), "{}", check.out);
    assert_eq!(provenance(&policy.out), checked, "{}", policy.out);
}

#[test]
fn policy_prints_a_value_neither_derived_nor_pinned_as_built_in() {
    let tree = tree(r#"{"complexity": {"cc": 8}}"#);

    let run = tree.run(&["policy", "complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("built-in: complexity in the whole repository"),
        "{}",
        run.out
    );

    let json = tree.run(&["policy", "--json", "complexity"]).json();
    let values = capability(&json, "complexity")["values"].clone();
    let within = values
        .as_array()
        .into_iter()
        .flatten()
        .find(|value| value["key"] == "in")
        .cloned()
        .unwrap_or_default();
    assert_eq!(within["provenance"], "built-in", "{json}");
}

#[test]
fn policy_prints_each_capability_s_activation_and_one_that_does_not_apply() {
    let tree = tree(r#"{"complexity": {"cc": 8}}"#);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let complexity = block(&run.out, "complexity — runs");
    assert!(complexity.contains("activation: automatic"), "{}", run.out);
    let lockfile = block(&run.out, "lockfile — not-applicable");
    assert!(lockfile.contains("activation: automatic"), "{}", run.out);
    let layering = block(&run.out, "layering — needs a section a person writes");
    assert!(layering.contains("activation: policy"), "{}", run.out);

    let json = tree.run(&["policy", "--json"]).json();
    assert_eq!(capability(&json, "lockfile")["state"], "not-applicable");
    assert_eq!(capability(&json, "lockfile")["activation"], "automatic");
    assert_eq!(capability(&json, "layering")["state"], "needs-policy");
    assert_eq!(capability(&json, "complexity")["activation"], "automatic");
}

#[test]
fn policy_prints_the_build_and_the_accepted_list() {
    let tree = tree(
        r#"{"build": "make",
            "accepted": [{"gate": "escapes", "file": "src/old.rs", "text": "x.unwrap()",
                          "count": 1}]}"#,
    );

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("build — pinned\n      pinned: build make"),
        "{}",
        run.out
    );
    assert!(
        run.says("accepted — 1 entry\n      escapes src/old.rs: x.unwrap()"),
        "{}",
        run.out
    );

    let json = tree.run(&["policy", "--json"]).json();
    assert_eq!(json["build"]["provenance"], "pinned", "{json}");
    assert_eq!(json["build"]["value"], "make", "{json}");
    assert_eq!(json["accepted"][0]["file"], "src/old.rs", "{json}");
}

#[test]
fn policy_prints_the_build_the_manifests_derive() {
    let tree = tree("{}");
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(
            "build — derived\n      derived: build cargo build --all-targets from Cargo.toml, \
             one command per manifest"
        ),
        "{}",
        run.out
    );
    assert!(run.says("accepted — nothing is accepted"), "{}", run.out);
}

/// An integration runs at `klin check` alone, claims nothing of a file its report leaves out,
/// and runs a command klin does not vouch for. Spec 9.4.
#[test]
fn policy_prints_the_limitations_of_an_integration() {
    let tree = tree(r#"{"sarif": [{"name": "scan", "report": "scan.sarif", "run": "scan"}]}"#);

    let run = tree.run(&["policy", "scan"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("limitation: runs at klin check only, never at the Stop"),
        "{}",
        run.out
    );
    assert!(run.says("limitation: coverage unverified"), "{}", run.out);
    assert!(
        run.says("limitation: the command is the project's own"),
        "{}",
        run.out
    );

    let json = tree.run(&["policy", "--json", "scan"]).json();
    let limitations = capability(&json, "scan")["limitations"].clone();
    assert_eq!(limitations.as_array().map(Vec::len), Some(3), "{json}");
}
