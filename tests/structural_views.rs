mod harness;

use harness::{Run, Tree};
use serde_json::{Value, json};

const CONFIG: &str = r#"{"dead_symbols":{"in":["src","web"]}}"#;
const REACHABILITY_CONFIG: &str = r#"{"reachability":{"in":"src"}}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

/// Every caller's view of one before and after pair. With `KLIN_DIFF_BIN` naming an earlier
/// build, that build judges a second copy of the same trees, and the two views must match.
fn views(scenario: fn(&Tree)) -> Value {
    views_for(
        CONFIG,
        "dead-symbols",
        &["dead-symbols", "--report"],
        scenario,
    )
}

fn reachability_views(scenario: fn(&Tree)) -> Value {
    views_for(
        REACHABILITY_CONFIG,
        "reachability",
        &["reachability"],
        scenario,
    )
}

fn views_for(config: &str, gate_name: &str, report: &[&str], scenario: fn(&Tree)) -> Value {
    let seen = observed(&harness::binary(), config, gate_name, report, scenario);
    if let Ok(other) = std::env::var("KLIN_DIFF_BIN") {
        assert_eq!(
            seen,
            observed(&other, config, gate_name, report, scenario),
            "{other} judged differently"
        );
    }
    seen
}

fn observed(
    klin: &str,
    config: &str,
    gate_name: &str,
    report: &[&str],
    scenario: fn(&Tree),
) -> Value {
    let tree = Tree::new();
    tree.write("klin.json", config);
    scenario(&tree);
    let run = |args: &[&str]| harness::feed_as(klin, tree.root(), args, A_STOP);
    let judged = |flags: &[&str]| {
        let mut args = vec!["gate", "--json", "--gate", gate_name];
        args.extend_from_slice(flags);
        run(&args)
    };
    let gate = |flags: &[&str]| normalized(&judged(flags));
    let whole = gate(&[]);
    let strict = gate(&["--strict"]);
    let first = judged(&["--changed"]);
    let changed = normalized(&first);
    let base = tree.revision("main");
    let report = run(report).out.replace(&base[..7], "BASE");
    let hook = gate(&["--hook", "--changed"]);
    let again = judged(&["--changed"]);
    assert_eq!(
        normalized(&again),
        changed,
        "a changed run over the structural cache judged differently"
    );
    read_from_the_cache(&first, &again);
    json!({"whole": whole, "strict": strict, "changed": changed, "report": report, "hook": hook})
}

/// A repeated changed run takes from the structural cache every base outcome the first run
/// extracted beyond the changed files. A build that records no `cached` is not asked.
fn read_from_the_cache(first: &Run, again: &Run) {
    let facts = |run: &Run| {
        let report: Value = run
            .out
            .lines()
            .find_map(|line| serde_json::from_str(line).ok())
            .unwrap_or_default();
        report["gates"][0]["facts"].clone()
    };
    let (first, again) = (facts(first), facts(again));
    let Some(cached) = again["cached"].as_u64() else {
        return;
    };
    assert_eq!(
        first["extracted"].as_u64(),
        again["extracted"]
            .as_u64()
            .map(|extracted| extracted + cached),
        "{first} then {again}"
    );
}

fn normalized(run: &Run) -> Value {
    let mut report: Value = run
        .out
        .lines()
        .find_map(|line| serde_json::from_str(line).ok())
        .unwrap_or_default();
    if let Some(window) = report["window"].as_object_mut() {
        window.remove("before");
    }
    for row in report["gates"].as_array_mut().into_iter().flatten() {
        if let Some(fields) = row.as_object_mut() {
            fields.remove("ms");
            fields.remove("facts");
        }
    }
    json!({"code": run.code, "report": report})
}

/// One caller's verdict, findings and notes, one line each.
fn lines(view: &Value) -> Vec<String> {
    let report = &view["report"];
    let mut out = vec![format!("{} {}", report["status"], view["code"])];
    for finding in report["findings"].as_array().into_iter().flatten() {
        out.push(format!(
            "{} {}:{} {} {}",
            text(&finding["outcome"]),
            text(&finding["file"]),
            finding["line"],
            text(&finding["text"]),
            finding["values"]
        ));
    }
    for note in report["notes"].as_array().into_iter().flatten() {
        out.push(format!(
            "note {} {}",
            text(&note["file"]),
            text(&note["text"])
        ));
    }
    out
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

fn reached_commands(tree: &Tree) {
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
}

#[test]
fn reachability_uses_a_changed_outside_caller_for_an_unchanged_member() {
    let seen = reachability_views(|tree| {
        reached_commands(tree);
        tree.base();
        tree.write("src/main.rs", "fn main() { run_beta(); run_gamma(); }\n");
        tree.write("src/commands/epsilon_command.rs", "pub fn run_extra() {}\n");
    });

    assert!(
        lines(&seen["whole"]).iter().any(
            |line| line.contains("src/commands/alpha_command.rs") && line.contains("unreached")
        ),
        "{seen}"
    );
    assert_eq!(lines(&seen["strict"]), lines(&seen["whole"]));
    assert_eq!(
        lines(&seen["changed"]),
        [
            r#""FAIL" 1"#,
            r#"new src/commands/epsilon_command.rs:0 file {"sibling":"src/commands/beta_command.rs","unreached":1}"#,
        ]
    );
    assert_eq!(
        lines(&seen["hook"])[1..],
        lines(&seen["changed"])[1..],
        "{seen}"
    );
}

#[test]
fn reachability_keeps_an_unchanged_caller_for_a_changed_member() {
    let seen = reachability_views(|tree| {
        reached_commands(tree);
        tree.base();
        tree.write(
            "src/commands/alpha_command.rs",
            "pub fn run_alpha() {}\npub fn also() {}\n",
        );
    });

    assert_eq!(lines(&seen["whole"]), [r#""PASS" 0"#]);
    assert_eq!(lines(&seen["changed"]), [r#""PASS" 0"#]);
}

#[test]
fn reachability_keeps_ambiguous_references_and_sibling_evidence_stable() {
    let seen = reachability_views(|tree| {
        reached_commands(tree);
        tree.base();
        tree.write("src/commands/delta_command.rs", "pub fn run_alpha() {}\n");
        tree.write("src/commands/epsilon_command.rs", "pub fn run_extra() {}\n");
    });

    assert_eq!(
        lines(&seen["whole"]),
        [
            r#""FAIL" 1"#,
            r#"new src/commands/epsilon_command.rs:0 file {"sibling":"src/commands/beta_command.rs","unreached":1}"#,
        ]
    );
    assert_eq!(lines(&seen["changed"]), lines(&seen["whole"]), "{seen}");
    assert_eq!(
        lines(&seen["hook"])[1..],
        lines(&seen["whole"])[1..],
        "{seen}"
    );
}

#[test]
fn reachability_keeps_deletions_and_renames_on_the_same_view() {
    let seen = reachability_views(|tree| {
        reached_commands(tree);
        tree.base();
        tree.remove("src/commands/alpha_command.rs");
        assert!(std::fs::create_dir_all(tree.path("src/other")).is_ok());
        tree.git(&[
            "mv",
            "src/commands/beta_command.rs",
            "src/other/beta_command.rs",
        ]);
        tree.git(&[
            "mv",
            "src/commands/gamma_command.rs",
            "src/commands/gamma_command.ts",
        ]);
    });

    assert_eq!(lines(&seen["whole"]), [r#""PASS" 0"#]);
    assert_eq!(lines(&seen["changed"]), [r#""PASS" 0"#]);
}

#[test]
fn reachability_moving_into_and_out_of_scope_judges_each_tree_under_its_own_scope() {
    let seen = reachability_views(|tree| {
        reached_commands(tree);
        tree.write("lib/tool.rs", "pub fn run_tool() {}\n");
        tree.base();
        tree.git(&["mv", "lib/tool.rs", "src/commands/tool_command.rs"]);
        tree.write("src/commands/tool_command.rs", "pub fn run_new_tool() {}\n");
        assert!(std::fs::create_dir_all(tree.path("tools")).is_ok());
        tree.git(&[
            "mv",
            "src/commands/alpha_command.rs",
            "tools/alpha_command.rs",
        ]);
    });
    let expected = [
        r#""FAIL" 1"#,
        r#"new src/commands/tool_command.rs:0 file {"sibling":"src/commands/beta_command.rs","unreached":1}"#,
    ];
    assert_eq!(lines(&seen["whole"]), expected);
    assert_eq!(lines(&seen["changed"]), expected, "{seen}");
    assert_eq!(
        lines(&seen["hook"])[1..],
        lines(&seen["changed"])[1..],
        "{seen}"
    );
}

#[test]
fn reachability_keeps_unparsed_and_unsupported_coverage_stable() {
    let seen = reachability_views(|tree| {
        reached_commands(tree);
        tree.base();
        tree.write("src/commands/delta_command.rs", "pub fn broken( {\n");
        tree.write("src/commands/tool.py", "def tool():\n    return 1\n");
    });

    assert!(
        lines(&seen["whole"])[0] == r#""ERROR" 2"#
            && lines(&seen["whole"])
                .iter()
                .any(|line| line.contains("delta_command.rs")),
        "{seen}"
    );
    assert!(
        lines(&seen["strict"])[0] == r#""ERROR" 2"#
            && lines(&seen["strict"])
                .iter()
                .any(|line| line.contains("delta_command.rs")),
        "{seen}"
    );
    assert!(
        lines(&seen["changed"])[0] == r#""ERROR" 2"#
            && lines(&seen["changed"])
                .iter()
                .any(|line| line.contains("delta_command.rs")),
        "{seen}"
    );
    assert_eq!(lines(&seen["hook"])[0], r#""PASS" 1"#);
    assert!(
        lines(&seen["hook"])
            .iter()
            .any(|line| line.contains("delta_command.rs") && line.contains("grammar")),
        "{seen}"
    );
    assert_eq!(
        ["whole", "strict", "changed", "hook"].map(|view| {
            seen[view]["report"]["gates"][0]["coverage"]["not_measured"]
                .as_u64()
                .unwrap_or(u64::MAX)
        }),
        [0, 0, 0, 0],
        "{seen}"
    );
}

#[test]
fn a_cached_structural_view_keeps_imports_modules_and_coverage_together() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write(
        "src/lib.rs",
        "use crate::child::helper;\nmod child;\nfn root() { helper(); }\n",
    );
    tree.write("src/child.rs", "pub fn helper() {}\n");
    tree.write("src/broken.rs", "fn broken( {\n");
    tree.base();
    tree.write("src/changed.rs", "fn changed() {}\n");

    let run = || tree.run(&["gate", "--json", "--changed", "--gate", "dead-symbols"]);
    let first = run();
    let again = run();
    let report = |run: &Run| {
        run.out
            .lines()
            .find_map(|line| serde_json::from_str::<Value>(line).ok())
            .unwrap_or_default()
    };
    let (first_report, again_report) = (report(&first), report(&again));

    assert_eq!(normalized(&first), normalized(&again));
    assert!(
        again_report["gates"][0]["facts"]["cached"]
            .as_u64()
            .is_some_and(|cached| cached > 0),
        "{again_report}"
    );
    assert_eq!(
        first_report["gates"][0]["coverage"],
        again_report["gates"][0]["coverage"]
    );
}

#[test]
fn an_edit_is_judged_against_every_unchanged_declaration_and_reference() {
    let seen = views(|tree| {
        tree.write("src/lib.rs", "fn helper() {}\nfn spare() {}\n");
        tree.write("src/caller.rs", "fn main() { helper(); spare(); }\n");
        tree.write("src/other.rs", "fn untouched() {}\n");
        tree.write("src/uses.rs", "pub fn uses() { kept(); }\n");
        tree.base();
        tree.write("src/caller.rs", "fn main() { spare(); }\n");
        tree.write(
            "src/lib.rs",
            "fn helper() {}\nfn spare() {}\nfn kept() {}\nfn fresh() {}\n",
        );
    });

    assert_eq!(
        lines(&seen["whole"]),
        [
            r#""FAIL" 1"#,
            r#"new src/lib.rs:4 fn fresh() {} {"dead":1}"#,
            r#"worsened src/lib.rs:1 fn helper() {} {"dead":1,"lost_reference":"src/caller.rs"}"#,
            "note  1 dead symbol(s) the base already held:\n  src/other.rs:1  fn untouched() {}",
        ]
    );
    assert_eq!(lines(&seen["changed"]), lines(&seen["whole"])[..3]);
    assert_eq!(lines(&seen["hook"])[1..], lines(&seen["whole"])[1..3]);
}

#[test]
fn an_addition_exists_only_in_the_after_view_and_resolves_unchanged_references() {
    let seen = views(|tree| {
        tree.write("src/main.rs", "fn main() { later(); }\n");
        tree.write("src/lib.rs", "fn lonely() {}\n");
        tree.base();
        tree.write("src/later.rs", "fn later() {}\n");
        tree.write(
            "src/added.rs",
            "pub fn call() { lonely(); }\nfn added() {}\n",
        );
    });

    assert_eq!(
        lines(&seen["whole"]),
        [
            r#""FAIL" 1"#,
            r#"new src/added.rs:2 fn added() {} {"dead":1}"#,
            "note  1 dead symbol(s) the base already held:\n  src/lib.rs:1  fn lonely() {}",
        ]
    );
    assert_eq!(lines(&seen["changed"]), lines(&seen["whole"])[..2]);
}

#[test]
fn a_deletion_exists_only_in_the_before_view_and_names_the_lost_reference() {
    let seen = views(|tree| {
        tree.write("src/lib.rs", "fn helper() {}\n");
        tree.write("src/caller.rs", "fn main() { helper(); }\n");
        tree.write("src/gone.rs", "fn gone() {}\n");
        tree.base();
        tree.remove("src/caller.rs");
        tree.remove("src/gone.rs");
    });

    assert_eq!(
        lines(&seen["whole"]),
        [
            r#""FAIL" 1"#,
            r#"worsened src/lib.rs:1 fn helper() {} {"dead":1,"lost_reference":"src/caller.rs"}"#,
            "note  1 dead symbol(s) the base already held:\n  src/gone.rs:1  fn gone() {}",
        ]
    );
    assert_eq!(
        lines(&seen["changed"]),
        [
            r#""PASS" 0"#,
            "note  1 dead symbol(s) the base already held:\n  src/gone.rs:1  fn gone() {}",
        ]
    );
}

#[test]
fn a_same_extension_rename_keeps_its_inherited_debt() {
    let seen = views(|tree| {
        tree.write("src/old_name.rs", "fn debt() {}\npub fn api() {}\n");
        tree.base();
        tree.git(&["mv", "src/old_name.rs", "src/new_name.rs"]);
    });

    let held = [
        r#""PASS" 0"#,
        "note  1 dead symbol(s) the base already held:\n  src/new_name.rs:1  fn debt() {}",
    ];
    assert_eq!(lines(&seen["whole"]), held);
    assert_eq!(lines(&seen["changed"]), held);
}

#[test]
fn an_extension_changing_rename_reads_the_base_bytes_under_the_new_grammar() {
    let seen = views(|tree| {
        tree.write(
            "web/cast.ts",
            "function cast(value: unknown) { return <string>value; }\nexport const used = cast(1);\nfunction stale() {}\n",
        );
        tree.write(
            "web/view.ts",
            "function old() {}\nexport function view() { return 1; }\n",
        );
        tree.base();
        tree.git(&["mv", "web/cast.ts", "web/cast.tsx"]);
        tree.git(&["mv", "web/view.ts", "web/view.tsx"]);
    });

    let unparsed = "unparsed web/cast.tsx:null the TSX grammar rejected it null";
    let held = "note  1 dead symbol(s) the base already held:\n  web/view.tsx:1  function old() {}";
    assert_eq!(lines(&seen["whole"]), [r#""ERROR" 2"#, unparsed, held]);
    assert_eq!(lines(&seen["changed"]), [r#""ERROR" 2"#, unparsed, held]);
    assert_eq!(
        lines(&seen["hook"])[1..],
        ["note web/cast.tsx the TSX grammar rejected it", held]
    );
}

#[test]
fn moving_into_and_out_of_scope_judges_each_tree_under_its_own_scope() {
    let seen = views(|tree| {
        tree.write("lib/moved.rs", "fn moved_debt() {}\n");
        tree.write("src/lib.rs", "fn helper() {}\n");
        tree.write("src/caller.rs", "fn main() { helper(); }\n");
        tree.base();
        tree.git(&["mv", "lib/moved.rs", "src/moved.rs"]);
        assert!(std::fs::create_dir_all(tree.path("tools")).is_ok());
        tree.git(&["mv", "src/caller.rs", "tools/caller.rs"]);
    });

    assert_eq!(
        lines(&seen["whole"]),
        [
            r#""PASS" 0"#,
            "note  2 dead symbol(s) the base already held:\n  src/lib.rs:1  fn helper() {}\n  src/moved.rs:1  fn moved_debt() {}",
        ]
    );
}

#[test]
fn a_duplicate_name_in_an_unchanged_file_keeps_an_added_declaration_alive() {
    let seen = views(|tree| {
        tree.write("src/main.rs", "fn main() { same(); }\n");
        tree.write("src/one.rs", "fn same() {}\n");
        tree.base();
        tree.write("src/two.rs", "fn same() {}\n");
    });

    assert_eq!(lines(&seen["whole"]), [r#""PASS" 0"#]);
    assert_eq!(lines(&seen["changed"]), [r#""PASS" 0"#]);
}

#[test]
fn an_unsupported_language_stays_outside_dead_symbol_coverage() {
    let seen = views(|tree| {
        tree.write("src/lib.rs", "pub fn api() {}\n");
        tree.write("src/job.py", "def job():\n    return 1\n");
        tree.base();
        tree.write("src/job.py", "def job():\n    return 2\n");
        tree.write("src/tool.py", "def tool():\n    return 1\n");
    });

    let coverage = &seen["whole"]["report"]["gates"][0]["coverage"];
    assert_eq!(coverage["found"], 1, "{seen}");
    assert_eq!(coverage["not_measured"], 0, "{seen}");
    assert_eq!(lines(&seen["changed"]), [r#""PASS" 0"#]);
}

#[test]
fn an_unparsed_file_is_named_by_each_caller_as_before() {
    let seen = views(|tree| {
        tree.write("src/old_broken.rs", "fn old( {\n");
        tree.write("src/lib.rs", "pub fn api() {}\n");
        tree.base();
        tree.write("src/new_broken.rs", "fn new( {\n");
    });

    let new = "unparsed src/new_broken.rs:null the Rust grammar rejected it null";
    let old = "unparsed src/old_broken.rs:null the Rust grammar rejected it null";
    assert_eq!(lines(&seen["whole"]), [r#""ERROR" 2"#, new, old]);
    assert_eq!(lines(&seen["strict"]), [r#""ERROR" 2"#, new, old]);
    assert_eq!(lines(&seen["changed"]), [r#""ERROR" 2"#, new]);
    assert_eq!(
        lines(&seen["hook"]),
        [
            r#""PASS" 1"#,
            "note src/new_broken.rs the Rust grammar rejected it"
        ]
    );
}

#[test]
fn a_case_only_rename_git_does_not_see_is_read_from_the_working_tree() {
    let seen = views(|tree| {
        tree.write("src/lib.rs", "fn helper() {}\n");
        tree.write("src/Caller.rs", "fn main() { helper(); }\n");
        tree.base();
        tree.write("src/lib.rs", "fn helper() {}\npub fn api() {}\n");
        assert!(std::fs::rename(tree.path("src/Caller.rs"), tree.path("src/caller.rs")).is_ok());
        tree.write("src/caller.rs", "fn main() {}\n");
    });

    let worsened =
        r#"worsened src/lib.rs:1 fn helper() {} {"dead":1,"lost_reference":"src/Caller.rs"}"#;
    assert_eq!(lines(&seen["changed"])[..2], [r#""FAIL" 1"#, worsened]);
    assert_eq!(lines(&seen["hook"])[1], worsened);
}
