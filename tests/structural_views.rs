mod harness;

use harness::{Run, Tree};
use serde_json::{Value, json};

const CONFIG: &str = r#"{"dead_symbols":{"in":["src","web"]}}"#;
const REACHABILITY_CONFIG: &str = r#"{"reachability":{"in":"src"}}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

/// Every caller's view of one before and after pair. With `KLIN_DIFF_BIN` naming an earlier
/// build, that build judges a second copy of the same trees, and the two views must match.
fn views(scenario: fn(&Tree)) -> Value {
    views_for(CONFIG, "dead-symbols", scenario)
}

fn reachability_views(scenario: fn(&Tree)) -> Value {
    views_for(REACHABILITY_CONFIG, "reachability", scenario)
}

fn views_for(config: &str, gate_name: &str, scenario: fn(&Tree)) -> Value {
    let seen = observed(&harness::binary(), config, gate_name, scenario);
    if let Ok(other) = std::env::var("KLIN_DIFF_BIN") {
        assert_eq!(
            seen,
            observed(&other, config, gate_name, scenario),
            "{other} judged differently"
        );
    }
    seen
}

fn observed(klin: &str, config: &str, gate_name: &str, scenario: fn(&Tree)) -> Value {
    let tree = Tree::new();
    tree.write("klin.json", config);
    scenario(&tree);
    let check = |flags: &[&str]| {
        let mut args = vec!["check", "--json", gate_name];
        args.extend_from_slice(flags);
        normalized(&harness::feed_as(klin, tree.root(), &args, ""), gate_name)
    };
    let whole = check(&[]);
    let changed = check(&["--changed"]);
    let first = stop(klin, &tree);
    let again = stop(klin, &tree);
    assert_eq!(
        normalized(&again, gate_name)["report"],
        normalized(&first, gate_name)["report"],
        "a stop over the structural cache judged differently"
    );
    read_from_the_cache(&row(&first, gate_name), &row(&again, gate_name));
    json!({"whole": whole, "changed": changed, "hook": normalized(&first, gate_name)})
}

/// A first Stop over the tree, judged against the branch base, as the report it printed or,
/// for a green stop that prints none, the one the journal records. The hook runs every gate,
/// and only the hook reads the structural cache.
fn stop(klin: &str, tree: &Tree) -> Run {
    let _ = std::fs::remove_file(tree.state("turn"));
    let run = harness::feed_as(klin, tree.root(), harness::AGENT, A_STOP);
    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let line: Value = run
        .out
        .lines()
        .find(|line| line.starts_with('{') && !line.contains("\"systemMessage\""))
        .or_else(|| journal.lines().last())
        .and_then(|line| serde_json::from_str(line).ok())
        .unwrap_or_else(|| panic!("no report in {}", run.out));
    let document = match line["result"].is_object() {
        true => &line["result"],
        false => &line,
    };
    let mut report: serde_json::Map<String, Value> =
        ["findings", "notes", "status", "execution", "judgement"]
            .into_iter()
            .filter(|key| !document[*key].is_null())
            .map(|key| (key.to_string(), document[key].clone()))
            .collect();
    report.insert("gates".to_string(), harness::gate_rows(&line).clone());
    let out = Value::Object(report).to_string();
    Run {
        code: run.code,
        printed: out.clone(),
        out,
    }
}

fn report(run: &Run) -> Value {
    run.out
        .lines()
        .find_map(|line| serde_json::from_str(line).ok())
        .unwrap_or_default()
}

/// The row of one gate in a run's report.
fn row(run: &Run, gate_name: &str) -> Value {
    let report = report(run);
    gate_rows(&report)
        .as_array()
        .and_then(|gates| gates.iter().find(|row| row["name"] == gate_name))
        .cloned()
        .unwrap_or_else(|| panic!("no {gate_name} row in {report}"))
}

/// The gate rows of the check document's diagnostics, or of the hook's report.
fn gate_rows(report: &Value) -> &Value {
    match report["diagnostics"]["gates"].is_array() {
        true => &report["diagnostics"]["gates"],
        false => &report["gates"],
    }
}

/// A repeated stop takes from the structural cache every base outcome the first stop extracted
/// beyond the changed files, or read from the cache a `klin check --changed` before it wrote.
/// A build that records no `cached` is not asked.
fn read_from_the_cache(first: &Value, again: &Value) {
    let (first, again) = (&first["facts"], &again["facts"]);
    if again["cached"].is_null() {
        return;
    }
    let read = |facts: &Value| {
        facts["extracted"].as_u64().unwrap_or(0) + facts["cached"].as_u64().unwrap_or(0)
    };
    assert_eq!(read(first), read(again), "{first} then {again}");
}

/// The verdict, and the findings, notes and row of one gate, without what a run may vary.
fn normalized(run: &Run, gate_name: &str) -> Value {
    let mut report = report(run);
    if let Some(window) = report["window"].as_object_mut() {
        window.remove("before");
    }
    for list in ["findings", "reviews", "notes", "errors", "gates"] {
        if let Some(held) = report[list].as_array_mut() {
            held.retain(|item| {
                item.get("check").is_some_and(Value::is_null)
                    || ["check", "gate", "name"]
                        .iter()
                        .any(|key| item[*key] == gate_name)
            });
        }
    }
    let rows = match report["diagnostics"]["gates"].is_array() {
        true => &mut report["diagnostics"]["gates"],
        false => &mut report["gates"],
    };
    untimed(rows);
    json!({"code": run.code, "report": report})
}

/// Gate rows without what a run may vary.
fn untimed(rows: &mut Value) {
    for row in rows.as_array_mut().into_iter().flatten() {
        if let Some(fields) = row.as_object_mut() {
            for varies in ["ms", "facts", "names", "footprint"] {
                fields.remove(varies);
            }
        }
    }
}

/// One caller's verdict, findings and notes, one line each.
fn lines(view: &Value) -> Vec<String> {
    let report = &view["report"];
    let mut out = vec![format!("{} {}", verdict(report), view["code"])];
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
    for review in report["reviews"].as_array().into_iter().flatten() {
        out.push(format!(
            "review {} {}",
            text(&review["file"]),
            text(&review["reason"])
        ));
    }
    for note in report["notes"].as_array().into_iter().flatten() {
        let said = match &note["text"] {
            Value::Null => &note["message"],
            said => said,
        };
        out.push(format!("note {} {}", text(&note["file"]), text(said)));
    }
    for error in report["errors"].as_array().into_iter().flatten() {
        out.push(format!(
            "error {} {} {}",
            text(&error["kind"]),
            text(&error["file"]),
            text(&error["message"])
        ));
    }
    out
}

/// The hook's status row, or the same word for the check document's axes.
fn verdict(report: &Value) -> Value {
    match (&report["status"], text(&report["execution"])) {
        (Value::Null, "error") => json!("ERROR"),
        (Value::Null, _) => json!(text(&report["judgement"]).to_uppercase()),
        (status, _) => status.clone(),
    }
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
    assert_eq!(lines(&seen["changed"]), lines(&seen["whole"]), "{seen}");
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

    let review = "review src/commands/delta_command.rs unreadable";
    assert_eq!(lines(&seen["whole"]), [r#""REVIEW" 0"#, review], "{seen}");
    assert_eq!(lines(&seen["changed"]), [r#""REVIEW" 0"#, review], "{seen}");
    assert_eq!(lines(&seen["hook"])[0], r#""REVIEW" 0"#);
    assert_eq!(
        ["whole", "changed", "hook"].map(|view| {
            gate_rows(&seen[view]["report"])[0]["coverage"]["not_measured"]
                .as_u64()
                .unwrap_or(u64::MAX)
        }),
        [0, 0, 0],
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

    let first = stop(&harness::binary(), &tree);
    let again = stop(&harness::binary(), &tree);
    let (first_row, again_row) = (row(&first, "dead-symbols"), row(&again, "dead-symbols"));
    let counted = |row: &Value| {
        ["reads", "parses", "extracted", "cached"]
            .map(|field| row["facts"][field].as_u64().unwrap_or(u64::MAX))
    };

    assert_eq!(
        normalized(&first, "dead-symbols")["report"],
        normalized(&again, "dead-symbols")["report"]
    );
    assert_eq!(counted(&first_row), [4, 4, 4, 0], "{first_row}");
    assert_eq!(counted(&again_row), [1, 1, 1, 3], "{again_row}");
    assert_eq!(first_row["coverage"], again_row["coverage"]);
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

/// `klin check --changed` takes the Stop's changed-run path: a declaration that a changed caller
/// left dead in an unchanged file is the same finding in both, and the check reads the
/// structural cache the Stop wrote. Spec 11.3.
#[test]
fn changed_reports_what_the_stop_reports_and_reads_its_structural_cache() {
    let seen = views(|tree| {
        tree.write("src/lib.rs", "fn helper() {}\n");
        tree.write("src/caller.rs", "fn main() { helper(); }\n");
        tree.write("src/other.rs", "pub fn untouched() {}\n");
        tree.base();
        tree.write("src/caller.rs", "fn main() {}\n");
    });

    let worsened =
        r#"worsened src/lib.rs:1 fn helper() {} {"dead":1,"lost_reference":"src/caller.rs"}"#;
    assert_eq!(lines(&seen["changed"]), [r#""FAIL" 1"#, worsened], "{seen}");
    assert_eq!(lines(&seen["hook"])[1..], [worsened], "{seen}");

    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write("src/lib.rs", "fn helper() {}\n");
    tree.write("src/caller.rs", "fn main() { helper(); }\n");
    tree.write("src/other.rs", "pub fn untouched() {}\n");
    tree.base();
    tree.write("src/caller.rs", "fn main() {}\n");
    stop(&harness::binary(), &tree);
    let checked = harness::feed_as(
        &harness::binary(),
        tree.root(),
        &["check", "--json", "--changed", "dead-symbols"],
        "",
    );
    let facts = &row(&checked, "dead-symbols")["facts"];
    assert!(facts["cached"].as_u64().unwrap_or(0) > 0, "{facts}");
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
    assert_eq!(lines(&seen["changed"]), lines(&seen["whole"]));
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
    assert_eq!(lines(&seen["changed"]), lines(&seen["whole"]));
    assert_eq!(lines(&seen["hook"])[1..], lines(&seen["whole"])[1..]);
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
fn an_extension_changing_rename_the_new_grammar_rejects_is_a_lost_measurement() {
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

    let lost = r#"new web/cast.tsx:null web/cast.tsx {"column":33,"line":1,"reason":"parse"}"#;
    let held = "note  1 dead symbol(s) the base already held:\n  web/view.tsx:1  function old() {}";
    assert_eq!(lines(&seen["whole"]), [r#""FAIL" 1"#, lost, held]);
    assert_eq!(lines(&seen["changed"]), [r#""FAIL" 1"#, lost, held]);
    assert_eq!(lines(&seen["hook"]), [r#""FAIL" 2"#, lost, held]);
}

/// A file moved into the scope joins it, and a file moved out keeps the scope of its base path,
/// so its reference still keeps `helper` alive. Spec 7.3.
#[test]
fn a_file_moved_out_of_scope_keeps_its_base_scope_and_one_moved_in_joins_it() {
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
            "note  1 dead symbol(s) the base already held:\n  src/moved.rs:1  fn moved_debt() {}",
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

    let coverage = &seen["whole"]["report"]["diagnostics"]["gates"][0]["coverage"];
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

    let new = "review src/new_broken.rs unreadable";
    let old = "note src/old_broken.rs src/old_broken.rs is not measured (unreadable) — the Rust grammar finds an error at line 1, column 1";
    assert_eq!(lines(&seen["whole"]), [r#""REVIEW" 0"#, new, old]);
    assert_eq!(lines(&seen["changed"]), [r#""REVIEW" 0"#, new]);
    assert_eq!(lines(&seen["hook"]), [r#""REVIEW" 0"#]);
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
    assert_eq!(lines(&seen["changed"])[1], worsened);
    assert_eq!(lines(&seen["hook"])[1], worsened);
}
