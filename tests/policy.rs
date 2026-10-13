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
                          "count": 1, "reason": "Legacy code."}]}"#,
    );

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("build — pinned\n      pinned: build make"),
        "{}",
        run.out
    );
    assert!(
        run.says("accepted — 1 entry\n      escapes src/old.rs: x.unwrap() (count 1)"),
        "{}",
        run.out
    );

    let json = tree.run(&["policy", "--json"]).json();
    assert_eq!(json["build"]["provenance"], "pinned", "{json}");
    assert_eq!(json["build"]["value"], "make", "{json}");
    assert_eq!(json["accepted"][0]["file"], "src/old.rs", "{json}");
    assert_eq!(json["accepted"][0]["reason"], "Legacy code.", "{json}");
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

#[test]
fn policy_prints_the_limitations_of_an_integration_no_section_names_yet() {
    let tree = tree("{}");

    let run = tree.run(&["policy", "sarif"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("sarif — needs a section a person writes"),
        "{}",
        run.out
    );
    assert!(
        run.says("limitation: runs at klin check only, never at the Stop"),
        "{}",
        run.out
    );
}

/// A convention that leaves a scope key out runs under its default, which `policy` names as
/// built in under the convention. Spec 11.6.
#[test]
fn policy_names_a_scope_key_a_convention_leaves_out_as_built_in() {
    let tree = tree(
        r#"{"conventions": {
            "no-flags": {"text": "config::Flags", "in": "src", "remedy": "Use the context."}
        }}"#,
    );

    let run = tree.run(&["policy", "conventions", "no-flags"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("built-in: except nothing is taken out"),
        "{}",
        run.out
    );
    assert!(!run.says("built-in: in "), "{}", run.out);

    let json = tree.run(&["policy", "--json", "conventions"]).json();
    let values = capability(&json, "conventions")["values"].clone();
    let except = values
        .as_array()
        .into_iter()
        .flatten()
        .find(|value| value["key"] == "except")
        .cloned()
        .unwrap_or_default();
    assert_eq!(except["provenance"], "built-in", "{json}");
    assert_eq!(except["entry"], "no-flags", "{json}");
}

/// A dated schedule is pinned, and the JSON names the step in force as the value, so a reader
/// resolves no dates. Spec 11.6.
#[test]
fn policy_json_names_the_step_in_force_of_a_dated_schedule() {
    let tree = tree(r#"{"complexity": {"cc": {"2020-01-01": 12, "2099-01-01": 8}}}"#);

    let json = tree.run(&["policy", "--json", "complexity"]).json();
    let values = capability(&json, "complexity")["values"].clone();
    let cc = values
        .as_array()
        .into_iter()
        .flatten()
        .find(|value| value["key"] == "cc")
        .cloned()
        .unwrap_or_default();
    assert_eq!(cc["provenance"], "pinned", "{json}");
    assert_eq!(cc["value"], 12, "{json}");
    assert_eq!(cc["step"], "2020-01-01", "{json}");
    assert_eq!(cc["schedule"]["2099-01-01"], 8, "{json}");
}

/// A tree where nothing applies still has a policy to read: each capability and why it does
/// not run. Spec 11.6.
#[test]
fn policy_lists_every_capability_where_none_applies() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.base();

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("escapes — not-applicable"), "{}", run.out);
    assert!(
        run.says("layering — needs a section a person writes"),
        "{}",
        run.out
    );
}

/// Deriving the public surfaces parses the working tree, so a whole `policy` names where to read
/// them and parses nothing, and `klin policy public-api` lists them. Spec 11.6, ADR 0044.
#[test]
fn a_whole_policy_parses_no_source_for_the_public_surfaces() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    );
    tree.write("src/lib.rs", "pub fn exposed() {}\n");
    tree.base();

    let whole = tree.run(&["policy"]);
    assert_eq!(whole.code, 0, "{}", whole.out);
    let public_api = block(&whole.out, "public-api — runs");
    assert!(
        public_api.contains("which `klin policy public-api` lists"),
        "{}",
        whole.out
    );
    assert!(!public_api.contains("surface core"), "{}", whole.out);

    let named = tree.run(&["policy", "public-api"]);
    assert_eq!(named.code, 0, "{}", named.out);
    assert!(named.says("surface "), "{}", named.out);
}

fn value(json: &Value, name: &str, key: &str) -> Value {
    capability(json, name)["values"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|value| value["key"] == key)
        .cloned()
        .unwrap_or_default()
}

/// The JSON of `klin policy public-api` carries each derived surface and its items, so two
/// contracts never read alike. Spec 11.6, 11.7.
#[test]
fn policy_json_carries_each_public_surface_and_its_items() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n",
    );
    tree.write("src/lib.rs", "pub fn exposed() {}\n");
    tree.base();

    let json = tree.run(&["policy", "--json", "public-api"]).json();
    let surface = value(&json, "public-api", "surface");
    assert_eq!(surface["provenance"], "derived", "{json}");
    assert_eq!(surface["entry"], "core (Cargo.toml)", "{json}");
    assert_eq!(surface["value"]["items"][0]["path"], "exposed", "{json}");
}

/// A named convention is the whole scope of the JSON, as it is of the text.
#[test]
fn policy_json_of_one_convention_names_no_other() {
    let tree = tree(
        r#"{"conventions": {
            "no-todo": {"text": "TODO", "remedy": "Do it."},
            "no-fixme": {"text": "FIXME", "remedy": "Fix it."}
        }}"#,
    );

    let json = tree
        .run(&["policy", "--json", "conventions", "no-todo"])
        .json();
    let values = capability(&json, "conventions")["values"].to_string();
    assert!(values.contains("no-todo"), "{json}");
    assert!(!values.contains("no-fixme"), "{json}");
}

/// A built-in value is typed as a pinned one would be, and its words for a person sit apart.
#[test]
fn policy_json_types_a_built_in_value_as_a_pinned_one() {
    let tree = tree(r#"{"sarif": [{"name": "scan", "report": "scan.sarif"}]}"#);
    let omitted = value(
        &tree.run(&["policy", "--json", "scan"]).json(),
        "scan",
        "differential",
    );
    assert_eq!(omitted["provenance"], "built-in", "{omitted}");
    assert_eq!(omitted["value"], false, "{omitted}");
    assert_eq!(omitted["description"], "`false`", "{omitted}");

    tree.write(
        "klin.json",
        r#"{"sarif": [{"name": "scan", "report": "scan.sarif", "differential": false}]}"#,
    );
    let pinned = value(
        &tree.run(&["policy", "--json", "scan"]).json(),
        "scan",
        "differential",
    );
    assert_eq!(pinned["provenance"], "pinned", "{pinned}");
    assert_eq!(pinned["value"], omitted["value"], "{pinned}");
}

/// A configuration klin cannot read is still the policy document under `--json`, which names
/// the error and exits 2. Spec 11.6, 11.7.
#[test]
fn policy_json_names_a_configuration_error_in_the_policy_document() {
    let tree = tree(r#"{"complexity": {"nope": 1}}"#);

    let run = tree.run(&["policy", "--json"]);

    assert_eq!(run.code, 2, "{}", run.out);
    let json = run.json();
    assert_eq!(json["schema_version"], 1, "{json}");
    assert_eq!(json["command"], "policy", "{json}");
    assert_eq!(json["config"]["present"], true, "{json}");
    let keys: Vec<&String> = json
        .as_object()
        .into_iter()
        .flat_map(|all| all.keys())
        .collect();
    assert_eq!(
        keys,
        ["command", "config", "errors", "schema_version"],
        "{json}"
    );
    let error = &json["errors"][0];
    assert_eq!(error["kind"], "configuration", "{json}");
    assert_eq!(error["check"], Value::Null, "{json}");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|said| said.contains("nope")),
        "{json}"
    );
}

/// The text of a configuration klin cannot read is the `FAIL:` line, with exit 2. Spec 11.6.
#[test]
fn policy_text_names_a_configuration_error_and_exits_2() {
    let tree = tree(r#"{"complexity": {"nope": 1}}"#);

    let run = tree.run(&["policy"]);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.out.starts_with("FAIL: "), "{}", run.out);
    assert!(run.says("unknown field \"nope\""), "{}", run.out);
}

/// A name, an entry or a `--config` that `policy` cannot take is an invocation error in the
/// policy document, with exit 2. Spec 7.3, 11.7.
#[test]
fn policy_json_names_an_argument_it_cannot_take_as_an_invocation_error() {
    let tree = tree(
        r#"{"complexity": {"cc": 8}, "conventions": {"no-todo": {"text": "TODO", "remedy": "Do it."}}}"#,
    );

    for args in [
        &["policy", "--json", "nope"][..],
        &["policy", "--json", "complexity", "cc"],
        &["policy", "--json", "conventions", "nope"],
        &["policy", "--json", "--config", "missing.json"],
        &["policy", "--json", "--config", "."],
        &["policy", "--json", "public-api", "nope"],
    ] {
        let run = tree.run(args);

        assert_eq!(run.code, 2, "{args:?}: {}", run.out);
        let json = run.json();
        assert_eq!(json["command"], "policy", "{args:?}: {json}");
        assert_eq!(json["errors"][0]["kind"], "invocation", "{args:?}: {json}");
    }
}

/// The error document names the configuration the run loaded, as the policy document does, even
/// where no klin.json is written. Spec 11.7.
#[test]
fn policy_json_error_names_the_configuration_the_run_loaded() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    tree.base();

    let listed = tree.run(&["policy", "--json"]).json();
    let refused = tree.run(&["policy", "--json", "nope"]).json();

    assert_eq!(refused["errors"][0]["kind"], "invocation", "{refused}");
    assert_eq!(
        refused["config"]["path"], listed["config"]["path"],
        "{refused}"
    );
    assert_eq!(refused["config"]["present"], false, "{refused}");
}

/// `--suggest` derives layers from the tree's directories, lets each layer use the layers it
/// mostly reaches today, counts what that policy would hold, and writes nothing. Spec 11.6.
#[test]
fn suggest_prints_layers_and_the_held_count_of_each_edge_kind_and_writes_nothing() {
    let tree = Tree::new();
    tree.write(
        "app/page.ts",
        "import { button } from \"../components/button\";\nimport { util } from \"../lib/util\";\nexport const page = button + util;\n",
    );
    tree.write(
        "components/button.ts",
        "import { util } from \"../lib/util\";\nexport const button = util;\n",
    );
    tree.write(
        "components/card.ts",
        "import { util } from \"../lib/util\";\nexport const card = util;\n",
    );
    tree.write(
        "lib/util.ts",
        "import { button } from \"../components/button\";\nexport const util = 1;\nexport const twice = button;\n",
    );
    tree.base();

    let run = tree.run(&["policy", "--suggest"]);

    assert_eq!(run.code, 0, "{}", run.out);
    let start = run.out.find("\n{\n").expect("a JSON block");
    let suggested: Value = serde_json::Deserializer::from_str(&run.out[start..])
        .into_iter()
        .next()
        .expect("one JSON value")
        .expect("valid JSON");
    assert_eq!(
        suggested["layering"],
        serde_json::json!({
            "acyclic": true,
            "layers": {
                "app": {"in": "app", "can_use": ["components", "lib"]},
                "components": {"in": "components", "can_use": ["lib"]},
                "lib": {"in": "lib", "can_use": []},
            }
        }),
        "{}",
        run.out
    );
    assert!(
        run.says("held: layering 1 forbidden edge and 2 cycle edges"),
        "{}",
        run.out
    );
    assert!(!tree.path("klin.json").exists(), "{}", run.out);
    assert_eq!(tree.status(), "", "{}", run.out);
}

/// A directory no `in` can name, such as a Next.js dynamic route, gets no layer, so the
/// suggestion is a configuration klin accepts.
#[test]
fn suggest_names_no_layer_for_a_directory_a_path_cannot_name() {
    let tree = Tree::new();
    tree.write(
        "app/[id]/page.ts",
        "import { util } from \"../lib/util\";\nexport const page = util;\n",
    );
    tree.write("app/lib/util.ts", "export const util = 1;\n");
    tree.write(
        "app/ui/card.ts",
        "import { util } from \"../lib/util\";\nexport const card = util;\n",
    );
    tree.base();

    let run = tree.run(&["policy", "--suggest"]);

    assert_eq!(run.code, 0, "{}", run.out);
    let start = run.out.find("\n{\n").expect("a JSON block");
    let suggested: Value = serde_json::Deserializer::from_str(&run.out[start..])
        .into_iter()
        .next()
        .expect("one JSON value")
        .expect("valid JSON");
    let layers = suggested["layering"]["layers"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        layers.keys().collect::<Vec<_>>(),
        ["lib", "ui"],
        "{}",
        run.out
    );
    tree.write(
        "klin.json",
        &serde_json::json!({"layering": suggested["layering"]}).to_string(),
    );
    let accepted = tree.run(&["check", "layering"]);
    assert_eq!(accepted.code, 0, "{}", accepted.out);
}

/// `--suggest` derives the radius from history, and the radius cache it reads stays as it was,
/// because `policy` writes nothing to the state directory. Spec 11.6.
#[test]
fn suggest_writes_nothing_to_the_state_directory() {
    let tree = harness::history(60, 0);
    std::fs::create_dir_all(tree.state("")).expect("a state directory");

    let run = tree.run(&["policy", "--suggest"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("derived: radius lines"), "{}", run.out);
    let written: Vec<_> = std::fs::read_dir(tree.state(""))
        .expect("the state directory")
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    assert!(written.is_empty(), "{written:?}\n{}", run.out);
}

/// In a repository klin never ran in, `--suggest` leaves no state directory behind. Spec 11.6.
#[test]
fn suggest_creates_no_state_directory() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    tree.base();

    let run = tree.run(&["policy", "--suggest"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.state("").exists(), "{}", run.out);
}

/// Functions of `lines` lines each, under one file per function.
fn long_functions(tree: &Tree, under: &str, lines: usize) {
    for at in 0..20 {
        let body = "    let _ = 1;\n".repeat(lines);
        tree.write(
            &format!("src/{under}{at}.rs"),
            &format!("pub fn f{at}() {{\n{body}}}\n"),
        );
    }
}

/// On a branch whose HEAD holds more functions than its base, `--suggest` derives from the
/// commit `setup --pin` derives from, so it suggests what `--pin` would write. Spec 11.6.
#[test]
fn suggest_derives_from_the_commit_setup_pin_derives_from() {
    let tree = Tree::new();
    long_functions(&tree, "short", 5);
    tree.base();
    tree.git(&["checkout", "-q", "-b", "feature"]);
    long_functions(&tree, "long", 40);
    tree.commit("long functions");

    let suggested = tree.run(&["policy", "--suggest"]);
    let pinned = tree.run(&["setup", "--pin"]);

    assert_eq!(pinned.code, 0, "{}", pinned.out);
    let complexity = |out: &str| -> Vec<String> {
        provenance(out)
            .into_iter()
            .filter(|line| line.starts_with("derived: complexity"))
            .collect()
    };
    assert!(!complexity(&pinned.out).is_empty(), "{}", pinned.out);
    assert_eq!(
        complexity(&suggested.out),
        complexity(&pinned.out),
        "{}",
        suggested.out
    );
}

/// Each capability names how many files it judges in this tree, from the files it selects and
/// with no check run, and the reason where it does not apply. #587, spec 11.6.
#[test]
fn policy_names_what_each_check_judges_and_why_one_does_not_apply() {
    let tree = tree(r#"{"complexity": {"in": "src"}}"#);
    tree.write("src/other.rs", CLEAN);
    tree.write("scripts/tool.rs", CLEAN);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let complexity = block(&run.out, "complexity — runs");
    assert!(
        complexity.contains("judges: 2 files in this tree"),
        "{}",
        run.out
    );
    let escapes = block(&run.out, "escapes — runs");
    assert!(
        escapes.contains("judges: 3 files in this tree"),
        "{}",
        run.out
    );
    let lockfile = block(&run.out, "lockfile — not-applicable");
    assert!(
        lockfile.contains(
            "judges: 0 manifests, because the tree holds no manifest klin reads a lockfile for"
        ),
        "{}",
        run.out
    );
    let layering = block(&run.out, "layering — needs a section a person writes");
    assert!(
        layering.contains("judges: 0 files, because it runs only under a section a person writes"),
        "{}",
        run.out
    );

    let json = tree.run(&["policy", "--json"]).json();
    assert_eq!(
        capability(&json, "complexity")["judges"],
        serde_json::json!({ "count": 2, "of": "files", "reason": null })
    );
    assert_eq!(capability(&json, "lockfile")["judges"]["count"], 0);
}

/// What `policy` counts for a check is what a whole `klin check` measures, for each check that
/// runs here, so the two cannot disagree. #587, spec 11.6.
#[test]
fn policy_counts_what_a_check_measures() {
    let tree = tree("{}");
    tree.write("AGENTS.md", "Read the README.md first.\n");
    tree.write("tests/one.rs", "#[test]\nfn one() {}\n");
    tree.write("src/other.rs", CLEAN);
    tree.base();

    let policy = tree.run(&["policy", "--json"]).json();
    let check = tree.run(&["check", "--json"]).json();
    let mut compared = Vec::new();
    for row in policy["capabilities"].as_array().into_iter().flatten() {
        let name = row["name"].as_str().unwrap_or_default();
        if row["state"] != "active" || row["judges"]["count"].is_null() || name == "public-api" {
            continue;
        }
        let measured = &capability(&check, name)["coverage"]["measured"];
        assert_eq!(
            &row["judges"]["count"], measured,
            "{name}: {policy}\n{check}"
        );
        compared.push(name.to_string());
    }
    assert!(compared.len() >= 6, "{compared:?}");
}

/// A convention a person must settle leaves the section unable to run, so `policy` says it
/// judges no file until then, as `klin check conventions` refuses to run. Review 5500533966.
#[test]
fn policy_judges_no_convention_file_while_one_convention_is_unsettled() {
    let tree = tree(
        r#"{"conventions": {
          "no-bad": { "code": "fn (", "remedy": "Write a pattern klin can read." },
          "no-todo": { "text": "TODO", "remedy": "Do it." }
        }}"#,
    );

    let run = tree.run(&["policy", "conventions"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let conventions = block(&run.out, "conventions — runs");
    assert!(
        conventions.contains("judges: no file until a person settles this:"),
        "{}",
        run.out
    );
    let json = tree.run(&["policy", "--json", "conventions"]).json();
    assert_eq!(
        capability(&json, "conventions")["judges"]["count"],
        Value::Null
    );
}

/// `policy conventions NAME` counts the files that one convention judges. Review 5500533966.
#[test]
fn policy_of_one_convention_counts_only_its_files() {
    let tree = tree(
        r#"{"conventions": {
          "no-unwrap": { "text": "unwrap", "in": "src/lib.rs", "remedy": "Handle it." },
          "no-todo": { "text": "TODO", "remedy": "Do it." }
        }}"#,
    );
    tree.write("src/other.rs", CLEAN);

    let one = tree.run(&["policy", "conventions", "no-unwrap"]);
    assert_eq!(one.code, 0, "{}", one.out);
    let conventions = block(&one.out, "conventions — runs");
    assert!(
        conventions.contains("judges: 1 file in this tree"),
        "{}",
        one.out
    );
    let json = tree
        .run(&["policy", "--json", "conventions", "no-unwrap"])
        .json();
    assert_eq!(capability(&json, "conventions")["judges"]["count"], 1);
}
