mod harness;

#[path = "fixtures/escape_text.rs"]
mod text;

use harness::{Run, Tree};
use serde_json::Value;

const SESSION: &str = r#"{"hook_event_name": "SessionStart"}"#;
const STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const CLEAN: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const TANGLED: &str = "pub fn knot(a: i32) -> i32 {\n    if a > 0 && a < 10 {\n        for x in 0..a {\n            if x == 3 { return 1; }\n        }\n    } else if a == 0 || a == -1 {\n        return 2;\n    }\n    match a {\n        1 => 1,\n        2 => 2,\n        3 => 3,\n        4 => 4,\n        5 => 5,\n        _ => 0,\n    }\n}\n";
const MIDDLING: &str = "pub fn mid(a: i32) -> i32 {\n    if a > 1 { return 1; }\n    if a > 2 { return 2; }\n    if a > 3 { return 3; }\n    if a > 4 { return 4; }\n    if a > 5 { return 5; }\n    if a > 6 { return 6; }\n    if a > 7 { return 7; }\n    if a > 8 { return 8; }\n    0\n}\n";
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

fn stop(tree: &Tree) -> Run {
    harness::feed(tree.root(), &["gate", "--hook"], STOP)
}

#[test]
fn a_tree_with_no_configuration_runs_every_derivable_gate() {
    let tree = project();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(run.says("ok    doc-citations"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    stubs"), "{}", run.out);
    assert!(run.says("ok    inventory"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(run.says("ok    dead-symbols"), "{}", run.out);
    assert!(run.says("ok    lockfile"), "{}", run.out);
    assert!(run.says("ok    public-api"), "{}", run.out);
    assert!(run.says("10 gate(s), all passed."), "{}", run.out);
}

#[test]
fn every_derived_value_prints_with_the_rule_that_produced_it() {
    let tree = project();

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("derived: complexity cc 5"), "{}", run.out);
    assert!(run.says("derived: complexity lines 25"), "{}", run.out);
    assert!(!run.says("derived: escapes"), "{}", run.out);
    assert!(!run.says("derived: dead_symbols"), "{}", run.out);
    assert!(
        run.says("derived: doc_size README.md 50, the word count at the derivation commit"),
        "{}",
        run.out
    );
    assert!(!run.says("derived: build"), "{}", run.out);
    assert!(
        run.says("derived: test roots tests, the roots that match a language's test convention"),
        "{}",
        run.out
    );
}

fn find(entries: &[Value], matches: impl Fn(&Value) -> bool) -> &Value {
    entries
        .iter()
        .find(|entry| matches(entry))
        .unwrap_or_else(|| panic!("no matching entry in {entries:?}"))
}

/// The whole of the first acceptance criterion of #152: one JSON entry per `derived:` line the
/// text report prints, counted from the two reports of one tree rather than from a number a
/// test holds.
#[test]
fn the_json_derived_list_is_as_long_as_the_reports_derived_lines() {
    let tree = project();

    let text = gate(&tree);
    let run = tree.run(&["gate", "--json"]);
    assert_eq!(text.code, 0, "{}", text.out);
    assert_eq!(run.code, 0, "{}", run.out);
    let said = text
        .out
        .lines()
        .filter(|line| line.trim_start().starts_with("derived:"))
        .count();
    assert!(said > 0, "{}", text.out);
    let report = run.json();
    assert_eq!(
        report["derived"].as_array().map(Vec::len),
        Some(said),
        "{report}\n{}",
        text.out
    );
}

#[test]
fn every_derived_line_has_a_matching_json_entry() {
    let tree = project();

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let derived = report["derived"]
        .as_array()
        .unwrap_or_else(|| panic!("no derived array in {report}"));

    let cc = find(derived, |e| {
        e["section"] == "complexity" && e["key"] == "cc"
    });
    let rule = cc["rule"].as_str().unwrap_or_default();
    assert!(rule.contains("the floor of 5"), "{report}");

    let doc_size = find(derived, |e| e["section"] == "doc_size");
    assert_eq!(doc_size["key"], "README.md", "{report}");
    assert_eq!(doc_size["value"], 50, "{report}");
    assert!(
        !derived.iter().any(|entry| entry["section"] == "build"),
        "{report}"
    );

    let test_roots = find(derived, |e| {
        e["rule"] == "the roots that match a language's test convention"
    });
    assert_eq!(test_roots["section"], "inventory", "{report}");
    assert_eq!(test_roots["key"], "test roots", "{report}");
    assert_eq!(
        test_roots["value"],
        serde_json::json!(["tests"]),
        "{report}"
    );
}

#[test]
fn a_key_the_config_pins_prints_as_pinned_beside_the_derived_ones() {
    let tree = project();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "cc": 12, "lines": 90 } }"#,
    );

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("pinned: complexity cc 12"), "{}", run.out);
    assert!(run.says("pinned: complexity lines 90"), "{}", run.out);
    assert!(!run.says("derived: complexity"), "{}", run.out);
}

#[test]
fn a_section_set_to_false_excludes_its_gate_with_nothing_else_configured() {
    let tree = project();
    tree.write("klin.json", r#"{ "escapes": false }"#);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
    assert!(
        run.says("9 gate(s), 1 excluded, all passed."),
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
    assert!(!run.says("derived: complexity roots"), "{}", run.out);
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
    assert!(run.says("2 file(s) found, 2 measured"), "{}", run.out);
    assert!(
        run.says("derived: lockfile manifests packages/a/package.json, packages/b/Cargo.toml"),
        "{}",
        run.out
    );
}

fn derives_test_roots(tree: &Tree, roots: &str) {
    let run = gate(tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says(&format!(
            "derived: test roots {roots}, the roots that match a language's test convention"
        )),
        "{}",
        run.out
    );
}

#[test]
fn a_build_script_at_a_crate_root_keeps_its_tests_as_a_test_root() {
    let tree = project();
    tree.write("build.rs", "fn main() {}\n");
    tree.base();

    derives_test_roots(&tree, "tests");
}

#[test]
fn a_workspace_member_with_its_own_build_script_keeps_its_tests_as_a_test_root() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n");
    tree.write("a/Cargo.toml", MANIFEST);
    tree.write("a/build.rs", "fn main() {}\n");
    tree.write("a/src/lib.rs", CLEAN);
    tree.write("a/tests/lib_test.rs", CLEAN);
    tree.write("b/Cargo.toml", MANIFEST);
    tree.write("b/src/lib.rs", CLEAN);
    tree.write("b/tests/lib_test.rs", CLEAN);
    tree.base();

    derives_test_roots(&tree, "a/tests, b/tests");
}

#[test]
fn a_crate_below_a_directory_that_holds_a_script_keeps_its_tests_as_a_test_root() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("install.sh", "echo hi\n");
    tree.write("rust/Cargo.toml", MANIFEST);
    tree.write("rust/build.rs", "fn main() {}\n");
    tree.write("rust/src/lib.rs", CLEAN);
    tree.write("rust/tests/lib_test.rs", CLEAN);
    tree.base();

    derives_test_roots(&tree, "rust/tests");
}

#[test]
fn a_config_file_beside_a_packages_manifest_keeps_its_tests_as_a_test_root() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("package.json", "{\"name\": \"p\"}\n");
    tree.write("jest.config.js", "module.exports = {};\n");
    tree.write("src/index.js", "export const one = 1;\n");
    tree.write("tests/index.test.js", "test(\"one\", () => {});\n");
    tree.base();

    derives_test_roots(&tree, "tests");
}

#[test]
fn a_directory_another_test_root_holds_is_no_test_root_of_its_own() {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("Cargo.toml", "[workspace]\nmembers = [\"tests\"]\n");
    tree.write("tests/Cargo.toml", MANIFEST);
    tree.write("tests/build.rs", "fn main() {}\n");
    tree.write("tests/src/lib.rs", CLEAN);
    tree.write("tests/tests/it.rs", CLEAN);
    tree.base();

    derives_test_roots(&tree, "tests");
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
    assert!(!run.says("derived: escapes"), "{}", run.out);
}

/// The launder the rule closes: park code where the derivation commit's survey holds no root,
/// let the base pick it up, then make the directory a root and read the site as held.
#[test]
fn a_site_under_a_root_the_derivation_commit_did_not_hold_is_new_whatever_the_base_holds() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.write("parked/risky.rs", text::WRAPPED);
    tree.base();
    tree.remove("parked/risky.rs");
    tree.commit("the derivation commit holds no parked root");
    tree.write("parked/risky.rs", text::WRAPPED);
    assert_eq!(harness::feed(tree.root(), &["radius"], SESSION).code, 0);
    tree.write("parked/risky.rs", text::WRAPPED_WITH_A_NOTE);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
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
    assert!(!run.says("derived: escapes"), "{}", run.out);
    assert!(run.says("src/app.py"), "{}", run.out);
}

fn cache(tree: &Tree) -> std::path::PathBuf {
    tree.state(&format!("cache/{}.json", tree.revision("main")))
}

#[test]
fn the_survey_is_cached_under_the_derivation_commit_and_read_back() {
    let tree = project();
    assert_eq!(gate(&tree).code, 0);
    let file = cache(&tree);
    assert!(file.is_file(), "{} was not written", file.display());

    let held = std::fs::read_to_string(&file).unwrap_or_default();
    assert!(held.contains("\"survey\""), "{held}");
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
    assert!(!again.says("derived: escapes"), "{}", again.out);
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
        assert!(run.says("derived: complexity cc 5"), "{}", run.out);
    }
}

#[test]
fn a_new_base_is_a_new_derivation_commit_and_a_new_cache_entry() {
    let tree = project();
    assert_eq!(gate(&tree).code, 0);
    let first = cache(&tree);
    assert!(first.is_file(), "{} was not written", first.display());

    tree.write("src/more.rs", CLEAN);
    tree.commit("a commit on the branch");
    assert_eq!(gate(&tree).code, 0);
    assert_eq!(cache(&tree), first);
    let head = tree.state(&format!("cache/{}.json", tree.revision("HEAD")));
    assert!(!head.is_file(), "{} was written", head.display());

    tree.write("src/other.rs", CLEAN);
    tree.base();
    assert_eq!(gate(&tree).code, 0);
    let second = cache(&tree);
    assert_ne!(first, second);
    assert!(second.is_file(), "{} was not written", second.display());
}

#[test]
fn pin_writes_the_ceilings_the_run_derives_and_no_topology() {
    let tree = project();

    let written = tree.run(&["init", "--pin"]);
    assert_eq!(written.code, 0, "{}", written.out);
    let held = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    let config: serde_json::Value = serde_json::from_str(&held).unwrap_or_default();
    assert_eq!(config["complexity"]["cc"], 5, "{config}");
    assert_eq!(config["complexity"]["lines"], 25, "{config}");
    assert_eq!(config["doc_size"]["README.md"], 50, "{config}");
    for retired in ["escapes", "doc_citations", "build", "lockfile", "inventory"] {
        assert!(config.get(retired).is_none(), "{retired}: {config}");
    }

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("pinned: complexity cc 5"), "{}", run.out);
    assert!(!run.says("derived: complexity"), "{}", run.out);
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
    tree.revision("main")[..7].to_string()
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

/// Fifty functions whose 95th percentile is cc 9, committed as the base, and a committed change
/// that would put the percentile at the tangled functions' cc 12.
fn a_change_that_would_raise_its_own_ceiling() -> Tree {
    let tree = Tree::new();
    tree.words("README.md", 5);
    tree.write("src/clean.rs", &many(CLEAN, 47));
    tree.write("src/mid.rs", MIDDLING);
    tree.write("src/knot.rs", &many(TANGLED, 2));
    tree.base();
    tree.write("src/more.rs", &many(TANGLED, 50));
    tree.commit("a change that would move the percentile");
    tree
}

fn derived_at_the_base(tree: &Tree, run: &Run) {
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(&format!(
            "derived: complexity cc 9 (95th percentile of 50 functions at {}, floor 5)",
            short(tree)
        )),
        "{}",
        run.out
    );
    assert!(run.says("src/more.rs"), "{}", run.out);
}

#[test]
fn a_branch_run_derives_from_the_base_and_not_from_a_committed_change() {
    let tree = a_change_that_would_raise_its_own_ceiling();

    derived_at_the_base(&tree, &gate(&tree));
}

#[test]
fn a_branch_run_by_hand_derives_from_the_base_and_not_from_a_turn_stamp() {
    let tree = a_change_that_would_raise_its_own_ceiling();
    assert_eq!(harness::feed(tree.root(), &["radius"], SESSION).code, 0);

    derived_at_the_base(&tree, &gate(&tree));
}

#[test]
fn a_push_run_derives_from_the_commit_the_push_started_from() {
    let tree = a_change_that_would_raise_its_own_ceiling();
    tree.write(
        "event.json",
        &format!("{{\"before\": \"{}\"}}", tree.revision("main")),
    );

    let run = tree.run_with(&[("GITHUB_EVENT_PATH", &tree.at("event.json"))], &["gate"]);
    assert!(run.says("the commit this push started from"), "{}", run.out);
    derived_at_the_base(&tree, &run);
}

#[test]
fn a_document_ceiling_comes_from_the_same_base_as_the_complexity_ceiling() {
    let tree = project();
    tree.words("README.md", 400);
    tree.commit("a document that would raise its own ceiling");

    let run = gate(&tree);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("README.md is 400 words, over its ceiling of 50"),
        "{}",
        run.out
    );
    assert!(
        run.says(&format!("over 2 function(s) at {})", short(&tree))),
        "{}",
        run.out
    );
}

#[test]
fn a_check_run_on_its_own_derives_from_the_same_base_as_the_gate() {
    let tree = a_change_that_would_raise_its_own_ceiling();

    derived_at_the_base(&tree, &tree.run(&["gate", "--gate", "complexity"]));
    derived_at_the_base(&tree, &tree.run(&["complexity"]));
}

#[test]
fn doc_size_run_on_its_own_derives_from_the_base() {
    let tree = project();
    tree.words("README.md", 400);
    tree.commit("a document that would raise its own ceiling");

    let run = tree.run(&["doc-size"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("README.md is 400 words, over its ceiling of 50"),
        "{}",
        run.out
    );
}

#[test]
fn a_stop_whose_state_directory_is_unusable_still_derives_from_the_stamps_parent() {
    let tree = a_change_that_would_raise_its_own_ceiling();
    tree.git(&["reset", "-q", "--hard", "HEAD~1"]);
    tree.write("klin.json", "{}");
    tree.commit("the configuration");
    let parent = tree.revision("HEAD")[..7].to_string();
    assert_eq!(harness::feed(tree.root(), &["radius"], SESSION).code, 0);
    tree.write("src/more.rs", &many(TANGLED, 50));
    tree.commit("a change inside the turn that would move the percentile");
    tree.write(".git/unusable", "a file where the state directory would go");

    let run = harness::feed_with(
        tree.root(),
        &[("KLIN_STATE_DIR", &tree.at(".git/unusable"))],
        &["gate", "--hook"],
        STOP,
    );
    assert!(run.says("window: turn"), "{}", run.out);
    assert!(
        run.says(&format!(
            "derived: complexity cc 9 (95th percentile of 50 functions at {parent}, floor 5)"
        )),
        "{}",
        run.out
    );
}

fn pinned_cc(tree: &Tree) -> Value {
    let written = tree.run(&["init", "--pin"]);
    assert_eq!(written.code, 0, "{}", written.out);
    let held = std::fs::read_to_string(tree.path("klin.json")).unwrap_or_default();
    let config: Value = serde_json::from_str(&held).unwrap_or_default();
    config["complexity"]["cc"].clone()
}

#[test]
fn init_pin_with_no_turn_stamp_derives_from_head_and_not_from_the_base() {
    let tree = a_change_that_would_raise_its_own_ceiling();

    assert_eq!(pinned_cc(&tree), 12);
}

#[test]
fn init_pin_derives_from_the_stamps_parent_and_not_from_the_base_or_head() {
    let tree = a_change_that_would_raise_its_own_ceiling();
    assert_eq!(harness::feed(tree.root(), &["radius"], SESSION).code, 0);
    tree.write("src/plain.rs", &many(CLEAN, 1000));
    tree.commit("a later change that would put the percentile at the floor");

    assert_eq!(pinned_cc(&tree), 12);
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

    tree.write("klin.json", r#"{ "doc_size": {"CHANGELOG.md": 900} }"#);
    let stated = gate(&tree);
    assert_eq!(stated.code, 0, "{}", stated.out);
    assert!(
        !stated.says("NOTE: doc_size CHANGELOG.md"),
        "{}",
        stated.out
    );
    tree.remove("klin.json");

    tree.base();
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
    tree.write("klin.json", r#"{ "complexity": { "cc": 12 } }"#);
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
        r#"{ "complexity": { "in": "src", "cc": 12 } }"#,
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
        r#"{ "complexity": { "except": "src/big.rs" } }"#,
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

#[test]
fn an_uncommitted_scope_edit_changes_judgment_but_not_the_ceiling() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("src/big.rs", &many(TANGLED, 50));
    tree.write(
        "klin.json",
        r#"{ "complexity": { "except": "src/big.rs" } }"#,
    );
    tree.base();
    tree.write("klin.json", r#"{ "complexity": { "in": "src" } }"#);

    let run = gate(&tree);
    assert_ne!(run.code, 2, "{}", run.out);
    assert!(
        run.says("derived: complexity cc 5 (the floor of 5, over 50 function(s) at"),
        "{}",
        run.out
    );
    assert!(
        run.says("today's complexity scope") && run.says("scope recorded at"),
        "{}",
        run.out
    );
    assert!(run.says("recorded scope: except src/big.rs"), "{}", run.out);

    let json = tree.run(&["gate", "--json"]);
    let report = json.json();
    let derived = report["derived"]
        .as_array()
        .unwrap_or_else(|| panic!("no derived values in {report}"));
    let cc = find(derived, |entry| {
        entry["section"] == "complexity" && entry["key"] == "cc"
    });
    assert!(
        cc["rule"]
            .as_str()
            .is_some_and(|rule| rule.contains("recorded scope: except src/big.rs")),
        "{report}"
    );
    assert!(
        report["notes"]
            .as_array()
            .is_some_and(|notes| notes.iter().any(|note| note["text"]
                .as_str()
                .is_some_and(|text| text.contains("today's complexity scope")))),
        "{report}"
    );
}

#[test]
fn a_commit_inside_the_turn_does_not_recalibrate_until_the_stamp_moves() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("src/big.rs", &many(TANGLED, 50));
    tree.write(
        "klin.json",
        r#"{ "complexity": { "except": "src/big.rs" } }"#,
    );
    tree.base();
    assert_eq!(harness::feed(tree.root(), &["radius"], SESSION).code, 0);
    tree.write("klin.json", r#"{ "complexity": { "in": "src" } }"#);
    tree.commit("widen the recorded scope later");

    let held = stop(&tree);
    assert!(
        held.says("derived: complexity cc 5 (the floor of 5, over 50 function(s) at"),
        "{}",
        held.out
    );

    assert_eq!(tree.run(&["turn", "reset"]).code, 0);
    let moved = stop(&tree);
    assert_eq!(moved.code, 0, "{}", moved.out);
    let file = tree.state(&format!("cache/{}.json", tree.revision("HEAD")));
    let derived: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap_or_default())
        .unwrap_or_default();
    assert_eq!(derived["complexity"]["cc"], 12, "{derived}");
}

#[test]
fn an_unreadable_recorded_config_falls_back_to_the_whole_repository_loudly() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("src/big.rs", &many(TANGLED, 50));
    tree.write("klin.json", "not json");
    tree.base();
    tree.write("klin.json", r#"{ "complexity": { "in": "src/clean.rs" } }"#);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("derived: complexity cc 12 ("), "{}", run.out);
    assert!(
        run.says("recorded complexity policy could not be read as compact scope")
            && run.says("whole repository"),
        "{}",
        run.out
    );
}

#[test]
fn a_pre_compact_recorded_scope_falls_back_to_the_whole_repository() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("src/big.rs", &many(TANGLED, 50));
    tree.write(
        "klin.json",
        r#"{ "complexity": { "roots": ["src/clean.rs"] } }"#,
    );
    tree.base();
    tree.write("klin.json", r#"{ "complexity": { "in": "src/clean.rs" } }"#);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("derived: complexity cc 12 ("), "{}", run.out);
    assert!(run.says("recorded scope: whole repository"), "{}", run.out);
}

#[test]
fn a_complexity_sample_is_read_back_from_the_cache_whatever_its_scope() {
    for config in [
        "{}",
        r#"{ "complexity": { "in": "src/clean.rs" } }"#,
        r#"{ "complexity": { "except": "src/other.rs" } }"#,
    ] {
        let tree = Tree::new();
        tree.write("src/clean.rs", &many(CLEAN, 50));
        tree.write("klin.json", config);
        tree.base();

        let first = gate(&tree);
        assert_eq!(first.code, 0, "{config}: {}", first.out);
        assert!(first.says("derived: complexity cc 5 ("), "{}", first.out);
        let file = cache(&tree);
        let mut held: Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap_or_default())
                .unwrap_or_default();
        held["complexity"]["cc"] = 77.into();
        written(&file, &held.to_string());

        let second = gate(&tree);
        assert_eq!(second.code, 0, "{config}: {}", second.out);
        assert!(
            second.says("derived: complexity cc 77 ("),
            "{config}: {}",
            second.out
        );
    }
}

#[test]
fn an_in_of_the_repository_root_is_the_whole_repository() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("klin.json", r#"{ "complexity": { "in": "." } }"#);
    tree.base();
    tree.write("klin.json", "{}");

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("recorded scope: whole repository"), "{}", run.out);
    assert!(!run.says("today's complexity scope"), "{}", run.out);
}

#[test]
fn narrowing_today_keeps_the_recorded_ceiling_and_strict_lost_coverage() {
    let tree = Tree::new();
    tree.write("src/a.rs", &many(CLEAN, 25));
    tree.write("src/b.rs", &many(CLEAN, 25));
    tree.write("klin.json", r#"{ "complexity": { "in": "src" } }"#);
    tree.base();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "except": "src/a.rs" } }"#,
    );

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("derived: complexity cc 5"), "{}", run.out);
    assert!(run.says("today's complexity scope"), "{}", run.out);
    assert!(run.says("src/a.rs was measured at the base"), "{}", run.out);
}

#[test]
fn a_recorded_scope_written_another_way_is_the_same_scope() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": ["src", "tests"], "except": ["src/b.rs", "src/a.rs"] } }"#,
    );
    tree.base();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": ["tests", "src/x", "src"], "except": ["src/a.rs", "docs", "src/b.rs", "src/a.rs"] } }"#,
    );

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("recorded scope: in src, tests; except src/a.rs, src/b.rs"),
        "{}",
        run.out
    );
    assert!(!run.says("today's complexity scope"), "{}", run.out);
}

#[test]
fn a_recorded_scope_that_selects_no_function_derives_the_floors() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("docs/guide.md", "A guide.\n");
    tree.write("klin.json", r#"{ "complexity": { "in": "docs" } }"#);
    tree.base();
    tree.write("klin.json", r#"{ "complexity": { "in": "src" } }"#);

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: complexity cc 5 (the floor of 5, over 0 function(s) at"),
        "{}",
        run.out
    );
    assert!(run.says("recorded scope: in docs"), "{}", run.out);
    assert!(
        !run.says("could not be read as compact scope"),
        "{}",
        run.out
    );
}

#[test]
fn a_recorded_complexity_set_to_false_falls_back_to_the_whole_repository_loudly() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write("src/big.rs", &many(TANGLED, 50));
    tree.write("klin.json", r#"{ "complexity": false }"#);
    tree.base();
    tree.write("klin.json", "{}");

    let run = gate(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("derived: complexity cc 12 ("), "{}", run.out);
    assert!(run.says("recorded scope: whole repository"), "{}", run.out);
    assert!(
        run.says("complexity section is not a compact policy object"),
        "{}",
        run.out
    );
}

#[test]
fn a_stop_nothing_blocks_tells_the_person_the_recorded_scope_lags() {
    let tree = Tree::new();
    tree.write("src/clean.rs", &many(CLEAN, 50));
    tree.write(
        "klin.json",
        r#"{ "complexity": { "except": "src/big.rs" } }"#,
    );
    tree.base();
    assert_eq!(harness::feed(tree.root(), &["radius"], SESSION).code, 0);
    tree.write(
        "klin.json",
        r#"{ "complexity": { "except": ["src/big.rs", "src/gone.rs"] } }"#,
    );

    let run = harness::feed(
        tree.root(),
        &["gate", "--hook"],
        r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#,
    );
    assert_eq!(run.code, 0, "{}", run.out);
    let told = run
        .printed
        .lines()
        .find_map(|line| {
            let held: Value = serde_json::from_str(line).ok()?;
            held.get("systemMessage")?.as_str().map(str::to_string)
        })
        .unwrap_or_default();
    assert!(told.contains("today's complexity scope"), "{}", run.out);
}
