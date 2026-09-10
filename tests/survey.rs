mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::{Run, Tree};

const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const TANGLED: &str = "fn knot(a: i32) -> i32 {\n    if a > 0 && a < 10 {\n        for x in 0..a {\n            if x == 3 { return 1; }\n        }\n    } else if a == 0 || a == -1 {\n        return 2;\n    }\n    match a {\n        1 => 1,\n        2 => 2,\n        3 => 3,\n        4 => 4,\n        5 => 5,\n        _ => 0,\n    }\n}\n";
const MIDDLING: &str = "fn mid(a: i32) -> i32 {\n    if a > 1 { return 1; }\n    if a > 2 { return 2; }\n    if a > 3 { return 3; }\n    if a > 4 { return 4; }\n    if a > 5 { return 5; }\n    if a > 6 { return 6; }\n    if a > 7 { return 7; }\n    if a > 8 { return 8; }\n    0\n}\n";
const MANIFEST: &str = "[package]\nname = \"t\"\nversion = \"0.1.0\"\n";

/// A project klin can survey whole: source, a test root, a document and a manifest, with the
/// whole of it already at the base so nothing in it is new.
fn project() -> Tree {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("Cargo.toml", MANIFEST);
    tree.write("src/lib.rs", CLEAN);
    tree.write("tests/lib_test.rs", CLEAN);
    tree.base();
    tree
}

fn gate(tree: &Tree) -> Run {
    tree.run(&["gate"])
}

#[test]
fn a_tree_with_no_configuration_runs_every_derivable_gate() {
    let tree = project();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(run.says("ok    doc-citations"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(run.says("4 gate(s), all passed."), "{}", run.out);
}

#[test]
fn every_derived_value_prints_with_the_rule_that_produced_it() {
    let tree = project();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(
            "derived: complexity roots src, tests, the shallowest directories that hold nothing \
             but source"
        ),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: escapes languages rust, the languages of the files under those roots"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: doc_size README.md, every Markdown file at the tree root"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: build cargo build --all-targets, one command per manifest"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: test roots tests, the roots that match a language's test convention"),
        "{}",
        run.out
    );
}

#[test]
fn a_key_the_config_pins_prints_as_pinned_beside_the_derived_ones() {
    let tree = project();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "ceilings": {"cc": 12, "lines": 90} } }"#,
    );

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("pinned: complexity cc 12"), "{}", run.out);
    assert!(run.says("pinned: complexity lines 90"), "{}", run.out);
    assert!(
        run.says("derived: complexity roots src, tests"),
        "{}",
        run.out
    );
}

#[test]
fn a_section_set_to_false_excludes_its_gate_with_nothing_else_configured() {
    let tree = project();
    tree.write("klin.json", r#"{ "escapes": false }"#);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
    assert!(
        run.says("3 gate(s), 1 excluded, all passed."),
        "{}",
        run.out
    );
}

#[test]
fn the_survey_skips_the_default_set_and_every_path_gitignore_excludes() {
    let tree = project();
    tree.write("node_modules/pkg/index.js", "const a = 1;\n");
    tree.write(".gitignore", "generated/\n");
    tree.write("generated/made.rs", CLEAN);
    tree.base();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("node_modules"), "{}", run.out);
    assert!(!run.says("generated"), "{}", run.out);
    assert!(
        run.says("derived: complexity roots src, tests"),
        "{}",
        run.out
    );
}

#[test]
fn the_survey_finds_one_root_per_package_of_a_monorepo() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("packages/a/package.json", "{\"name\": \"a\"}\n");
    tree.write("packages/a/tsconfig.json", "{}\n");
    tree.write("packages/a/src/one.ts", "export const one = 1;\n");
    tree.write("packages/b/Cargo.toml", MANIFEST);
    tree.write("packages/b/src/lib.rs", CLEAN);
    tree.base();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: complexity roots packages/a/src, packages/b/src"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: escapes languages rust, typescript"),
        "{}",
        run.out
    );
    assert!(run.says("derived: build tsc --noEmit"), "{}", run.out);
    assert!(run.says("cargo build --all-targets"), "{}", run.out);
}

/// The promise of ADR 0016: a tree already in debt is green against itself with no
/// configuration at all, because a root the derivation commit held is held debt and not new.
#[test]
fn a_tree_in_debt_is_green_against_itself_with_no_configuration() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/held.rs", text::WRAPPED);
    tree.write("src/knot.rs", TANGLED);
    tree.base();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
}

/// A gates entry states how a check runs, so klin derives no section beside it and the whole
/// tree is not measured twice under two names.
#[test]
fn a_gates_entry_leaves_its_check_underived() {
    let tree = project();
    tree.write(
        "klin.json",
        r#"{ "gates": [{"name": "complexity", "check": "complexity",
                        "with": {"roots": ["src"], "ceilings": {"cc": 8, "lines": 60}}}] }"#,
    );

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let rows = run
        .out
        .lines()
        .filter(|line| line.starts_with("complexity"))
        .count();
    assert_eq!(rows, 1, "{}", run.out);
}

/// The ceiling comes from the derivation commit, so growing a document past it fails rather
/// than moving the number the run judges against.
#[test]
fn a_derived_document_ceiling_comes_from_the_derivation_commit() {
    let tree = project();

    let green = tree.run(&["doc-size"]);
    assert_eq!(green.code, 0, "{}", green.out);
    assert!(
        green.says("README.md is 5 words, ceiling 50"),
        "{}",
        green.out
    );

    tree.words("README.md", 400);
    let grown = tree.run(&["doc-size"]);
    assert_eq!(grown.code, 1, "{}", grown.out);
    assert!(
        grown.says("README.md is 400 words, over its ceiling of 50"),
        "{}",
        grown.out
    );
}

#[test]
fn a_root_that_first_appears_in_the_working_tree_is_measured_and_its_sites_are_new() {
    let tree = project();
    tree.write("extra/risky.rs", text::WRAPPED);

    let run = gate(&tree);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  escapes"), "{}", run.out);
    assert!(run.says("extra/risky.rs"), "{}", run.out);
    assert!(
        run.says("derived: escapes roots extra, src, tests"),
        "{}",
        run.out
    );
}

/// The launder the rule closes: park code where the derivation commit's survey holds no root,
/// let the base pick it up, then make the directory a root and read the site as held.
#[test]
fn a_site_under_a_root_the_derivation_commit_did_not_hold_is_new_whatever_the_base_holds() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.write("parked/risky.rs", text::WRAPPED);
    tree.base();
    tree.remove("parked/risky.rs");
    tree.commit("the derivation commit holds no parked root");
    tree.write("parked/risky.rs", text::WRAPPED);

    let run = gate(&tree);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  escapes"), "{}", run.out);
    assert!(run.says("parked/risky.rs"), "{}", run.out);
}

#[test]
fn a_language_that_first_appears_in_the_working_tree_is_measured_on_that_run() {
    let tree = project();
    tree.write(
        "src/app.py",
        "def risky(a):\n    return a  # type: ignore\n",
    );

    let run = gate(&tree);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("derived: escapes languages python, rust"),
        "{}",
        run.out
    );
    assert!(run.says("src/app.py"), "{}", run.out);
}

fn cache(tree: &Tree) -> std::path::PathBuf {
    tree.state(&format!("cache/{}.json", tree.revision("HEAD")))
}

#[test]
fn the_survey_is_cached_under_the_derivation_commit_and_read_back() {
    let tree = project();
    assert_eq!(gate(&tree).code, 0);
    let file = cache(&tree);
    assert!(file.is_file(), "{} was not written", file.display());

    let held = std::fs::read_to_string(&file).unwrap_or_default();
    assert!(held.contains("\"survey\""), "{held}");
    assert!(held.contains("\"complexity\""), "{held}");
    assert!(held.contains("\"doc_size\""), "{held}");
    assert!(held.contains(env!("CARGO_PKG_VERSION")), "{held}");

    written(
        &file,
        &format!(
            "{{\"version\":\"{}\",\"survey\":{{\"roots\":[\"src\"],\
             \"languages\":[\"python\"],\"documents\":[],\"test_roots\":[],\
             \"manifests\":[]}}}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    );
    let again = gate(&tree);
    assert_eq!(again.code, 0, "{}", again.out);
    assert!(
        again.says("derived: escapes languages python, rust"),
        "{}",
        again.out
    );
}

fn written(file: &std::path::Path, text: &str) {
    assert!(
        std::fs::write(file, text).is_ok(),
        "{} could not be written",
        file.display()
    );
}

#[test]
fn a_cache_another_version_wrote_and_one_that_is_unreadable_are_surveyed_again() {
    let tree = project();
    assert_eq!(gate(&tree).code, 0);
    let file = cache(&tree);

    for text in [
        "{\"version\":\"0.0.0\",\"survey\":{\"roots\":[]}}\n",
        "not json",
    ] {
        written(&file, text);
        let run = gate(&tree);
        assert_eq!(run.code, 0, "{}", run.out);
        assert!(
            run.says("derived: complexity roots src, tests"),
            "{}",
            run.out
        );
    }
}

#[test]
fn a_new_commit_is_a_new_derivation_commit_and_a_new_cache_entry() {
    let tree = project();
    assert_eq!(gate(&tree).code, 0);
    let first = cache(&tree);
    assert!(first.is_file(), "{} was not written", first.display());

    tree.write("src/more.rs", CLEAN);
    tree.commit("another commit");
    assert_eq!(gate(&tree).code, 0);
    let second = cache(&tree);
    assert_ne!(first, second);
    assert!(second.is_file(), "{} was not written", second.display());
}

#[test]
fn init_pins_what_the_run_derives() {
    let tree = project();

    let written = tree.run(&["init"]);
    assert_eq!(written.code, 0, "{}", written.out);
    let held = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    let config: serde_json::Value = serde_json::from_str(&held).unwrap_or_default();
    assert_eq!(config["complexity"]["roots"][0], "src");
    assert_eq!(config["complexity"]["roots"][1], "tests");
    assert_eq!(config["escapes"]["languages"][0], "rust");
    assert_eq!(config["doc_size"][0]["file"], "README.md");
    assert_eq!(config["doc_citations"][0]["file"], "README.md");
    assert_eq!(config["build"], "cargo build --all-targets");

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("derived: complexity roots"), "{}", run.out);
}

#[test]
fn an_unknown_key_is_still_an_error_with_no_section_beside_it() {
    let tree = project();
    tree.write("klin.json", r#"{ "nonsense": 1 }"#);

    let run = gate(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("\"nonsense\" is not a key klin reads"),
        "{}",
        run.out
    );
}

#[test]
fn a_baseline_key_is_still_an_error_saying_the_key_is_gone() {
    let tree = project();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "roots": ["src"], "baseline": "was.json" } }"#,
    );

    let run = gate(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("names a \"baseline\""), "{}", run.out);
}

/// One copy of a function per name, so a base can hold as many as a percentile needs.
fn many(source: &str, count: usize) -> String {
    (0..count)
        .map(|at| source.replacen("fn ", &format!("fn at{at}_"), 1))
        .collect()
}

fn short(tree: &Tree) -> String {
    tree.revision("HEAD")[..7].to_string()
}

/// The ceiling is the nearest-rank 95th percentile of the derivation commit's own functions,
/// and the floor wins wherever that percentile falls below it. Fifty functions of three
/// complexities put a different value at each neighbouring rank, so a rank one out fails.
#[test]
fn a_derived_ceiling_is_the_percentile_of_the_derivation_commit() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/clean.rs", &many(CLEAN, 47));
    tree.write("src/mid.rs", MIDDLING);
    tree.write("src/knot.rs", &many(TANGLED, 2));
    tree.base();
    let at = short(&tree);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(&format!(
            "derived: complexity cc 9 (95th percentile of 50 functions at {at}, floor 5)"
        )),
        "{}",
        run.out
    );
    assert!(
        run.says(&format!(
            "derived: complexity lines 25 (the floor of 25, over 50 function(s) at {at})"
        )),
        "{}",
        run.out
    );
}

#[test]
fn below_fifty_functions_the_floor_is_the_ceiling() {
    let tree = project();
    let at = short(&tree);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(&format!(
            "derived: complexity cc 5 (the floor of 5, over 2 function(s) at {at})"
        )),
        "{}",
        run.out
    );
    assert!(
        run.says(&format!(
            "derived: complexity lines 25 (the floor of 25, over 2 function(s) at {at})"
        )),
        "{}",
        run.out
    );
}

/// A function the derivation commit does not hold never enters the percentile, so a directory
/// an agent makes a root cannot raise the ceiling that judges what it holds.
#[test]
fn a_function_only_in_the_working_tree_does_not_move_the_percentile() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.base();
    tree.write("extra/knot.rs", &many(TANGLED, 50));

    let run = gate(&tree);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("derived: complexity cc 5 (the floor of 5, over 50 function(s) at"),
        "{}",
        run.out
    );
    assert!(run.says("FAIL  complexity"), "{}", run.out);
}

/// A document the derivation commit lacks has no ceiling that is not read out of the working
/// tree, so klin names it and judges nothing, until the commit holds it.
#[test]
fn a_document_the_derivation_commit_lacks_is_a_note_and_is_not_judged() {
    let tree = project();
    tree.words("CHANGELOG.md", 400);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: doc_size CHANGELOG.md is 400 words and is not judged"),
        "{}",
        run.out
    );
    assert!(!run.says("CHANGELOG.md is 400 words, over"), "{}", run.out);

    tree.write(
        "klin.json",
        r#"{ "doc_size": [{"file": "CHANGELOG.md", "ceiling": 900}] }"#,
    );
    let stated = gate(&tree);
    assert_eq!(stated.code, 0, "{}", stated.out);
    assert!(
        !stated.says("NOTE: doc_size CHANGELOG.md"),
        "{}",
        stated.out
    );
    tree.remove("klin.json");

    tree.commit("the derivation commit holds it now");
    let held = tree.run(&["doc-size"]);
    assert_eq!(held.code, 0, "{}", held.out);
    assert!(
        held.says("CHANGELOG.md is 400 words, ceiling 450"),
        "{}",
        held.out
    );
}

/// Pinning one ceiling and leaving the other to the survey is allowed, and the run says which
/// is which.
#[test]
fn a_ceiling_pinned_beside_a_derived_one_is_used_as_written() {
    let tree = project();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "ceilings": {"cc": 12} } }"#,
    );
    tree.write("src/knot.rs", TANGLED);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("pinned: complexity cc 12"), "{}", run.out);
    assert!(
        run.says("derived: complexity lines 25 (the floor of 25, over 2 function(s) at"),
        "{}",
        run.out
    );

    tree.write(
        "klin.json",
        r#"{ "complexity": { "roots": ["src"], "ceilings": {"cc": 12} } }"#,
    );
    let beside_a_pinned_root = gate(&tree);
    assert_eq!(beside_a_pinned_root.code, 0, "{}", beside_a_pinned_root.out);
    assert!(
        beside_a_pinned_root.says("derived: complexity lines 25 (the floor of 25"),
        "{}",
        beside_a_pinned_root.out
    );
}

#[test]
fn a_new_function_over_the_derived_ceiling_fails() {
    let tree = project();
    tree.write("src/knot.rs", TANGLED);

    let run = gate(&tree);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  complexity"), "{}", run.out);
    assert!(run.says("src/knot.rs:1  cc 12, 17 lines"), "{}", run.out);
}

/// A tree whose documents the derivation commit all lacks still has a doc-size gate, so a run
/// under `--strict` has its decision for it and no ceiling is read out of the working tree.
#[test]
fn a_tree_whose_documents_are_all_new_still_gates_on_doc_size() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree.words("README.md", 80);

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(
        run.says("NOTE: doc_size README.md is 80 words"),
        "{}",
        run.out
    );
}

/// A file the gate never judges must not set the ceiling the judged files are held to, or
/// excluding generated code would loosen the gate instead of narrowing it.
#[test]
fn a_file_the_section_excludes_is_out_of_the_percentile_too() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("src/big.rs", &many(TANGLED, 50));
    tree.write(
        "klin.json",
        r#"{ "complexity": { "exclude": ["big.rs"] } }"#,
    );
    tree.base();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: complexity cc 5 (the floor of 5, over 50 function(s) at"),
        "{}",
        run.out
    );
}
