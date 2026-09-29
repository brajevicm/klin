mod harness;

use harness::Tree;
use serde_json::{Value, json};

const GATES: [&str; 3] = ["complexity", "dead-symbols", "reachability"];

/// A base with three reached command files, so `reachability` derives a family, under this
/// configuration.
fn commands(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    for name in ["alpha", "beta", "gamma"] {
        tree.write(
            &format!("src/commands/{name}_command.rs"),
            &format!("pub fn run_{name}() {{}}\n"),
        );
    }
    tree.write(
        "src/main.rs",
        "fn main() { run_alpha(); run_beta(); run_gamma(); }\n",
    );
    tree
}

fn judged(tree: &Tree, flags: &[&str], gates: &[&str]) -> Value {
    let mut args = vec!["gate", "--json"];
    args.extend_from_slice(flags);
    for gate in gates {
        args.extend(["--gate", gate]);
    }
    tree.run(&args).json()
}

/// Every structural gate in one run, each gate's own part of the report the same as that gate
/// run alone, so no gate's findings or coverage turn on the other gates a run holds.
fn together(tree: &Tree, flags: &[&str]) -> Value {
    let report = judged(tree, flags, &GATES);
    for gate in GATES {
        let alone = judged(tree, flags, &[gate]);
        assert_eq!(part(&report, gate), part(&alone, gate), "{gate}: {report}");
    }
    report
}

fn part(report: &Value, gate: &str) -> Value {
    let mut row = row(report, gate).clone();
    if let Some(fields) = row.as_object_mut() {
        fields.remove("ms");
        fields.remove("facts");
        fields.remove("names");
    }
    json!({
        "row": row,
        "findings": entries(report, "findings", gate),
        "notes": entries(report, "notes", gate),
    })
}

fn entries(report: &Value, list: &str, gate: &str) -> Vec<Value> {
    report[list]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["gate"] == gate)
        .cloned()
        .collect()
}

fn row<'a>(report: &'a Value, gate: &str) -> &'a Value {
    report["gates"]
        .as_array()
        .and_then(|gates| gates.iter().find(|row| row["name"] == gate))
        .unwrap_or_else(|| panic!("no {gate} row in {report}"))
}

fn found(report: &Value, gate: &str) -> Vec<String> {
    entries(report, "findings", gate)
        .iter()
        .map(|finding| format!("{}:{}", finding["file"], finding["text"]))
        .collect()
}

fn measured(report: &Value, gate: &str) -> u64 {
    row(report, gate)["coverage"]["measured"]
        .as_u64()
        .unwrap_or_else(|| panic!("no {gate} coverage in {report}"))
}

fn extracted(report: &Value, gate: &str) -> (u64, u64) {
    let facts = &row(report, gate)["facts"];
    match (facts["extracted"].as_u64(), facts["shared"].as_u64()) {
        (Some(extracted), Some(shared)) => (extracted, shared),
        _ => panic!("no {gate} facts in {report}"),
    }
}

#[test]
fn gates_under_one_scope_judge_as_each_does_alone() {
    let tree = commands(
        r#"{"complexity":{"in":"src"},"dead_symbols":{"in":"src"},"reachability":{"in":"src"}}"#,
    );
    tree.base();
    tree.write("src/lib.rs", "fn unused() {}\n");

    let report = together(&tree, &[]);

    assert_eq!(row(&report, "dead-symbols")["status"], "FAIL", "{report}");
    assert_eq!(
        found(&report, "dead-symbols"),
        [r#""src/lib.rs":"fn unused() {}""#]
    );
    assert_eq!(row(&report, "reachability")["status"], "ok", "{report}");
}

#[test]
fn gates_under_overlapping_scopes_judge_as_each_does_alone() {
    let tree = commands(
        r#"{"complexity":{"in":["src","web"]},"dead_symbols":{"in":"src"},"reachability":{"in":"src/commands"}}"#,
    );
    tree.write("web/app.ts", "function idle() {}\n");
    tree.base();
    tree.write("src/lib.rs", "fn unused() {}\n");

    let report = together(&tree, &[]);

    assert_eq!(
        found(&report, "dead-symbols"),
        [r#""src/lib.rs":"fn unused() {}""#]
    );
    assert_eq!(measured(&report, "complexity"), 6, "{report}");
    assert_eq!(measured(&report, "dead-symbols"), 5, "{report}");
}

#[test]
fn gates_under_disjoint_scopes_judge_as_each_does_alone() {
    let tree = commands(r#"{"dead_symbols":{"in":"web"},"reachability":{"in":"src"}}"#);
    tree.write("web/app.ts", "export function app() {}\n");
    tree.base();
    tree.write("web/lib.ts", "function unused() {}\n");
    tree.write("src/lib.rs", "fn unused() {}\n");

    let report = together(&tree, &[]);

    assert_eq!(
        found(&report, "dead-symbols"),
        [r#""web/lib.ts":"function unused() {}""#]
    );
    assert_eq!(measured(&report, "dead-symbols"), 2, "{report}");
    assert_eq!(row(&report, "reachability")["status"], "ok", "{report}");
}

#[test]
fn a_language_only_complexity_reads_leaves_the_structural_coverage_alone() {
    let tree = commands("{}");
    tree.write("tools/job.py", "def job():\n    return 1\n");
    tree.base();

    let report = together(&tree, &[]);

    assert_eq!(measured(&report, "complexity"), 5, "{report}");
    assert_eq!(measured(&report, "dead-symbols"), 4, "{report}");
    let coverage = &row(&report, "dead-symbols")["coverage"];
    assert_eq!(coverage["not_measured"], 0, "{report}");
    assert_eq!(measured(&report, "reachability"), 3, "{report}");
}

#[test]
fn a_file_one_gate_excepts_never_resolves_a_name_for_it_when_another_gate_reads_it() {
    let tree = commands(r#"{"dead_symbols":{"except":"generated"}}"#);
    tree.write(
        "generated/calls.rs",
        "fn helper() {}\nfn calls() { helper(); }\n",
    );
    tree.base();
    tree.write("src/lib.rs", "fn helper() {}\n");

    let report = together(&tree, &[]);

    assert_eq!(
        found(&report, "dead-symbols"),
        [r#""src/lib.rs":"fn helper() {}""#]
    );
    let coverage = &row(&report, "dead-symbols")["coverage"];
    assert_eq!(coverage["excluded"], 1, "{report}");
    assert_eq!(row(&report, "reachability")["status"], "ok", "{report}");
}

#[test]
fn a_file_the_grammar_rejects_is_unparsed_in_each_gate_that_reads_it() {
    let tree = commands("{}");
    tree.base();
    tree.write("src/broken.rs", "fn broken( {\n");

    let report = together(&tree, &[]);

    assert_eq!(row(&report, "complexity")["status"], "ERR", "{report}");
    assert_eq!(row(&report, "dead-symbols")["status"], "ERR", "{report}");
    assert_eq!(row(&report, "reachability")["status"], "ok", "{report}");
}

#[test]
fn a_changed_run_scopes_findings_and_resolves_against_the_whole_selection() {
    let tree = commands("{}");
    tree.write(
        "src/main.rs",
        "fn main() { run_alpha(); run_beta(); run_gamma(); helper(); }\n",
    );
    tree.base();
    tree.write(
        "src/commands/alpha_command.rs",
        "pub fn run_alpha() {}\nfn helper() {}\nfn unused() {}\n",
    );

    let report = together(&tree, &["--changed"]);

    assert_eq!(
        found(&report, "dead-symbols"),
        [r#""src/commands/alpha_command.rs":"fn unused() {}""#]
    );
    assert_eq!(row(&report, "reachability")["status"], "ok", "{report}");
}

#[test]
fn a_file_only_one_tree_holds_is_judged_as_each_gate_does_alone() {
    let tree = commands("{}");
    tree.write("src/lib.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "pub fn call() { helper(); }\n");
    tree.base();
    tree.remove("src/caller.rs");
    tree.write("src/fresh.rs", "fn fresh() {}\n");

    let report = together(&tree, &[]);

    assert_eq!(
        found(&report, "dead-symbols"),
        [
            r#""src/fresh.rs":"fn fresh() {}""#,
            r#""src/lib.rs":"fn helper() {}""#
        ]
    );
    let lost = entries(&report, "findings", "dead-symbols");
    assert!(
        lost.iter()
            .any(|finding| finding["values"]["lost_reference"] == "src/caller.rs"),
        "{report}"
    );
}

#[test]
fn a_file_two_structural_gates_read_is_extracted_once_per_tree() {
    let tree = commands("{}");
    tree.base();

    let report = judged(&tree, &[], &GATES);

    assert_eq!(extracted(&report, "dead-symbols"), (8, 0), "{report}");
    assert_eq!(
        row(&report, "dead-symbols")["facts"]["reads"],
        8,
        "{report}"
    );
    assert_eq!(
        row(&report, "dead-symbols")["facts"]["parses"],
        8,
        "{report}"
    );
    assert_eq!(extracted(&report, "reachability"), (0, 8), "{report}");
    assert_eq!(
        row(&report, "reachability")["facts"]["reads"],
        0,
        "{report}"
    );
    assert_eq!(
        row(&report, "reachability")["facts"]["parses"],
        0,
        "{report}"
    );
    assert!(row(&report, "complexity")["facts"].is_null(), "{report}");
    let alone = judged(&tree, &[], &["reachability"]);
    assert_eq!(extracted(&alone, "reachability"), (8, 0), "{alone}");
}

#[test]
fn a_strict_changed_run_extracts_both_trees_for_dead_symbols() {
    let tree = commands("{}");
    tree.base();
    tree.write(
        "src/commands/alpha_command.rs",
        "pub fn run_alpha() {}\npub fn also() {}\n",
    );

    let report = judged(&tree, &["--changed", "--strict"], &["dead-symbols"]);

    assert_eq!(extracted(&report, "dead-symbols"), (8, 0), "{report}");
}

#[test]
fn a_changed_run_shares_one_whole_base_between_structural_gates() {
    let tree = commands("{}");
    tree.base();
    tree.write(
        "src/commands/alpha_command.rs",
        "pub fn run_alpha() {}\npub fn also() {}\n",
    );

    let run = tree.run(&[
        "gate",
        "--json",
        "--changed",
        "--gate",
        "dead-symbols",
        "--gate",
        "reachability",
    ]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("\"extracted\":0"), "{}", run.out);
    let report = run.json();

    assert_eq!(extracted(&report, "dead-symbols"), (5, 3), "{report}");
    assert_eq!(extracted(&report, "reachability"), (0, 8), "{report}");
}

#[test]
fn a_caller_only_turn_judges_the_whole_family_off_the_shared_extraction() {
    let tree = commands("{}");
    tree.base();
    tree.write("src/main.rs", "fn main() { run_gamma(); }\n");

    let report = judged(&tree, &["--changed"], &["dead-symbols", "reachability"]);

    assert_eq!(row(&report, "reachability")["status"], "FAIL", "{report}");
    assert_eq!(
        found(&report, "reachability"),
        [
            r#""src/commands/alpha_command.rs":"file""#,
            r#""src/commands/beta_command.rs":"file""#
        ],
        "{report}"
    );
    assert_eq!(extracted(&report, "reachability"), (0, 8), "{report}");
    assert_eq!(measured(&report, "reachability"), 3, "{report}");
}

#[test]
fn a_name_resolving_gate_records_what_each_index_holds_and_what_each_part_took() {
    let tree = commands("{}");
    tree.base();
    tree.write(
        "src/commands/alpha_command.rs",
        "pub fn run_alpha() {}\nfn unused() {}\n",
    );

    let report = judged(&tree, &["--changed"], &GATES);

    for gate in ["dead-symbols", "reachability"] {
        let names = &row(&report, gate)["names"];
        assert_eq!(indexed(&names["before"]), [4, 4, 3, 4], "{gate}: {report}");
        assert_eq!(indexed(&names["after"]), [4, 5, 3, 5], "{gate}: {report}");
        assert!(names["base_ms"].is_u64(), "{gate}: {report}");
    }
    assert!(row(&report, "dead-symbols")["names"]["lost_ms"].is_u64());
    assert!(row(&report, "reachability")["names"]["lost_ms"].is_null());
    assert!(row(&report, "complexity")["names"].is_null(), "{report}");
    let layout = &row(&report, "dead-symbols")["names"]["layout"];
    for part in [
        "worktree_add_ms",
        "changes_ms",
        "renames_ms",
        "cache_name_ms",
        "ignored_ms",
        "walk_ms",
    ] {
        assert!(layout[part].is_u64(), "{part}: {report}");
    }
    assert!(
        row(&report, "reachability")["names"]["layout"].is_null(),
        "the run lays the base out once, so one row carries the parts: {report}"
    );
}

#[test]
fn the_structural_footprint_counts_each_extern_crate_with_its_names_and_nesting() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "src/lib.rs",
        "mod outer {\n    extern crate serde as json;\n}\n",
    );
    tree.base();

    let report = judged(&tree, &[], &["dead-symbols"]);

    let held = &row(&report, "dead-symbols")["footprint"];
    assert_eq!(held["files"], 2, "{report}");
    assert_eq!(held["extern_crates"], 2, "{report}");
    assert_eq!(held["extern_crate_bytes"], 18, "{report}");
    assert_eq!(held["nestings"], 2, "{report}");
    assert_eq!(held["nesting_entries"], 2, "{report}");
    assert_eq!(held["nesting_bytes"], 10, "{report}");
    assert!(held["sizes"]["extern_crate"].as_u64() > Some(0), "{report}");
}

#[test]
fn the_structural_footprint_counts_what_the_facts_of_one_run_hold() {
    let tree = commands("{}");
    tree.write(
        "src/held.rs",
        "mod inner {\n    pub struct Held;\n    impl Held {\n        pub fn take(&self, at: usize) -> usize { at }\n    }\n}\n",
    );
    tree.base();

    let report = judged(&tree, &[], &["dead-symbols", "reachability"]);

    let held = &row(&report, "dead-symbols")["footprint"];
    assert_eq!(held["files"], 10, "{report}");
    assert_eq!(held["declarations"], 12, "{report}");
    assert_eq!(held["references"], 14, "{report}");
    assert_eq!(
        row(&report, "dead-symbols")["names"]["after"]["references"],
        6,
        "the index keeps one site per file and line: {report}"
    );
    assert_eq!(held["declaration_name_bytes"], 76, "{report}");
    assert_eq!(held["module_declarations"], 2, "{report}");
    assert_eq!(held["owners"], 2, "{report}");
    assert_eq!(held["owner_bytes"], 8, "{report}");
    assert_reference_footprint(held, &report);
    assert_eq!(held["nestings"], 4, "{report}");
    assert_eq!(held["nesting_entries"], 4, "{report}");
    assert_eq!(held["nesting_bytes"], 20, "{report}");
    assert_eq!(held["exported_aliases"], 0, "{report}");
    assert!(held["reference_name_bytes"].as_u64() > Some(0), "{report}");
    assert!(
        held["declaration_text_bytes"].as_u64() > Some(0),
        "{report}"
    );
    assert!(held["signatures"].as_u64() > Some(0), "{report}");
    assert!(held["path_bytes"].as_u64() > Some(0), "{report}");
    for size in [
        "name",
        "file_facts",
        "declaration",
        "reference",
        "import",
        "module_declaration",
        "export",
        "export_leaf",
    ] {
        assert!(held["sizes"][size].as_u64() > Some(0), "{size}: {report}");
    }
    assert!(
        row(&report, "reachability")["footprint"].is_null(),
        "{report}"
    );
}

fn assert_reference_footprint(held: &Value, report: &Value) {
    assert!(
        held["reference_distinct_names"].as_u64() > Some(0),
        "{report}"
    );
    assert!(
        held["reference_canonical_allocations"].as_u64()
            >= held["reference_distinct_names"].as_u64(),
        "{report}"
    );
    assert!(
        held["reference_canonical_allocation_ratio_milli"].as_u64() >= Some(1_000),
        "{report}"
    );
    assert!(
        held["reference_canonical_bytes"].as_u64() > Some(0),
        "{report}"
    );
    assert!(
        held["reference_representation_before_bytes"].as_u64()
            > held["reference_representation_after_bytes"].as_u64(),
        "{report}"
    );
    assert!(
        held["reference_canonical_allocation_ratio"].as_f64() >= Some(1.0),
        "{report}"
    );
}

#[test]
fn repeated_reference_names_share_storage_across_tree_files() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"dead_symbols":{"in":"src"}}"#);
    tree.write("src/one.rs", "fn one() { helper(); }\n");
    tree.write("src/two.rs", "fn two() { helper(); }\n");
    tree.base();

    let report = judged(&tree, &[], &["dead-symbols"]);
    let footprint = &row(&report, "dead-symbols")["footprint"];
    assert_eq!(footprint["references"], 4, "{report}");
    assert_eq!(footprint["reference_distinct_names"], 1, "{report}");
    assert_eq!(footprint["reference_canonical_allocations"], 2, "{report}");
}

/// One tree's index as `[files, declarations, references, distinct_names]`, with its timings
/// required beside them.
fn indexed(tree: &Value) -> [u64; 4] {
    for timing in ["measure_ms", "index_ms", "query_ms"] {
        assert!(tree[timing].is_u64(), "no {timing} in {tree}");
    }
    ["files", "declarations", "references", "distinct_names"].map(|field| {
        tree[field]
            .as_u64()
            .unwrap_or_else(|| panic!("{field}: {tree}"))
    })
}
