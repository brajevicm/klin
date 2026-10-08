mod harness;

use std::fs;
use std::path::{Path, PathBuf};

use harness::{Run, Tree};
use serde_json::{Value, json};

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;

fn commands() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    for name in ["alpha", "beta", "gamma"] {
        tree.write(
            &format!("src/commands/{name}_command.rs"),
            &format!("pub fn run_{name}() {{}}\n"),
        );
    }
    tree.write(
        "src/main.rs",
        "fn main() { run_alpha(); run_beta(); run_gamma(); helper(); }\n",
    );
    tree.write("src/lib.rs", "fn helper() {}\n");
    tree.base();
    tree.write(
        "src/commands/alpha_command.rs",
        "pub fn run_alpha() {}\nfn unused() {}\n",
    );
    tree
}

/// A first Stop over the tree at `cwd`, judged against the branch base, as the report it
/// recorded, with the code `check` gives its status. Only the hook reads the structural cache,
/// since `check` checks the base out whole.
fn stop(tree: &Tree, cwd: &Path) -> Run {
    let _ = fs::remove_file(tree.state("turn"));
    let (run, line) = harness::stop_report(cwd, A_STOP, &[]);
    assert!(line.is_object(), "no report in {}", run.out);
    let report: serde_json::Map<String, Value> = [
        "derived", "findings", "gates", "notes", "status", "summary", "window",
    ]
    .into_iter()
    .map(|key| (key.to_string(), line[key].clone()))
    .collect();
    let out = Value::Object(report).to_string();
    Run {
        code: match line["status"].as_str() {
            Some("PASS") => 0,
            Some("FAIL") => 1,
            _ => 2,
        },
        printed: out.clone(),
        out,
    }
}

fn changed(tree: &Tree) -> Run {
    stop(tree, tree.root())
}

fn cache(tree: &Tree) -> PathBuf {
    tree.state("cache/structural")
}

fn cached_files(tree: &Tree) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(cache(tree))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// The verdict, findings, notes and coverage of a run, without the timing and fact counters
/// that are allowed to differ.
fn judged(run: &Run) -> Value {
    let mut report = run.json();
    let rows = match report["diagnostics"]["gates"].is_array() {
        true => &mut report["diagnostics"]["gates"],
        false => &mut report["gates"],
    };
    for row in rows.as_array_mut().into_iter().flatten() {
        for group in ["graph", "surface"] {
            if let Some(held) = row[group].as_object_mut() {
                held.remove("ms");
            }
        }
        if let Some(fields) = row.as_object_mut() {
            fields.remove("ms");
            fields.remove("facts");
            fields.remove("names");
            fields.remove("footprint");
        }
    }
    json!({"code": run.code, "report": report})
}

fn facts(run: &Run, gate: &str) -> Value {
    let report = run.json();
    harness::gate_rows(&report)
        .as_array()
        .and_then(|gates| gates.iter().find(|row| row["name"] == gate))
        .map(|row| row["facts"].clone())
        .unwrap_or_else(|| panic!("no {gate} row in {report}"))
}

fn counted(facts: &Value) -> [u64; 4] {
    ["reads", "parses", "extracted", "cached"].map(|field| {
        facts[field]
            .as_u64()
            .unwrap_or_else(|| panic!("no {field} in {facts}"))
    })
}

#[test]
fn a_repeated_changed_run_reads_the_base_facts_from_the_cache_and_judges_the_same() {
    let tree = commands();
    let status = tree.status();

    let cold = changed(&tree);
    let warm = changed(&tree);
    assert!(fs::remove_dir_all(cache(&tree)).is_ok());
    let again = changed(&tree);

    assert_eq!(cold.code, 1, "{}", cold.out);
    assert!(cold.says("fn unused() {}"), "{}", cold.out);
    assert_eq!(judged(&warm), judged(&cold));
    assert_eq!(judged(&again), judged(&cold));
    assert_eq!(counted(&facts(&cold, "dead-symbols")), [6, 6, 6, 0]);
    assert_eq!(counted(&facts(&warm, "dead-symbols")), [2, 2, 2, 4]);
    assert_eq!(counted(&facts(&again, "dead-symbols")), [6, 6, 6, 0]);
    assert_eq!(counted(&facts(&warm, "reachability")), [0, 0, 0, 0]);
    assert_eq!(tree.status(), status, "the cache touched the working tree");
}

#[test]
fn a_cached_decode_shares_reference_names_across_files() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"dead_symbols":{"in":"src"}}"#);
    tree.write("src/one.rs", "fn one() { helper(); }\n");
    tree.write("src/two.rs", "fn two() { helper(); }\n");
    tree.write("src/changed.rs", "fn old() {}\n");
    tree.base();
    tree.write("src/changed.rs", "fn changed() {}\n");

    changed(&tree);
    let warm = changed(&tree);
    let report = warm.json();
    let row = harness::gate_rows(&report)
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| row["name"] == "dead-symbols")
        .unwrap_or_else(|| panic!("no dead-symbols row in {report}"));
    assert!(row["facts"]["cached"].as_u64() >= Some(2), "{report}");
    assert_eq!(row["footprint"]["references"], 2, "{report}");
    assert_eq!(
        row["footprint"]["reference_canonical_allocations"], 1,
        "{report}"
    );
}

#[test]
fn a_cache_klin_cannot_read_whole_is_extracted_again_and_written_again() {
    let tree = commands();
    let cold = changed(&tree);
    let written = cached_files(&tree);
    assert_eq!(written.len(), 1, "{written:?}");
    let bytes = fs::read(&written[0]).unwrap_or_default();
    let mut flipped = bytes.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 1;
    let mut inside = bytes.clone();
    inside[bytes.len() / 2] ^= 1;

    for damaged in [
        Vec::new(),
        bytes[..bytes.len() / 2].to_vec(),
        flipped,
        inside,
        [bytes.as_slice(), b"more"].concat(),
        b"not a cache".to_vec(),
    ] {
        assert!(fs::write(&written[0], &damaged).is_ok());
        let run = changed(&tree);
        assert_eq!(judged(&run), judged(&cold));
        assert_eq!(counted(&facts(&run, "dead-symbols"))[3], 0);
        assert_eq!(fs::read(&written[0]).unwrap_or_default(), bytes);
    }
}

#[test]
fn a_cache_written_for_another_base_is_never_read_for_this_one() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { helper(); }\n");
    tree.base();
    let first = tree.revision("main");
    tree.write("src/lib.rs", "pub fn api() {}\nfn helper() {}\n");
    let referenced = changed(&tree);
    assert_eq!(referenced.code, 0, "{}", referenced.out);
    assert_eq!(cached_files(&tree), [cache(&tree).join(&first)]);

    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() {}\n");
    tree.base();
    let second = tree.revision("main");
    tree.write("src/lib.rs", "pub fn api() {}\nfn helper() {}\n");
    assert!(fs::copy(cache(&tree).join(&first), cache(&tree).join(&second)).is_ok());
    let run = changed(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("fn helper() {}"), "{}", run.out);
    assert_eq!(counted(&facts(&run, "dead-symbols"))[3], 0);
}

#[test]
fn a_checkout_setting_that_changes_the_base_bytes_is_judged_as_without_a_cache() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { helper(); }\n");
    tree.base();
    tree.write("src/lib.rs", "pub fn api() {}\nfn helper() {}\n");
    let referenced = changed(&tree);
    assert_eq!(referenced.code, 0, "{}", referenced.out);

    tree.git(&["config", "filter.drop.smudge", "sed -e s/helper//"]);
    tree.git(&["config", "filter.drop.clean", "cat"]);
    tree.write(".git/info/attributes", "src/caller.rs filter=drop\n");
    let warm = changed(&tree);
    assert!(fs::remove_dir_all(cache(&tree)).is_ok());
    let cold = changed(&tree);

    assert_eq!(cold.code, 1, "{}", cold.out);
    assert!(cold.says("fn helper() {}"), "{}", cold.out);
    assert_eq!(judged(&warm), judged(&cold));
}

#[test]
fn the_four_newest_bases_keep_a_cache_and_an_older_one_is_removed() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let mut bases = Vec::new();
    for at in 0..6 {
        tree.write("src/lib.rs", &format!("pub fn api_{at}() {{}}\n"));
        tree.base();
        bases.push(tree.revision("main"));
        let run = changed(&tree);
        assert_eq!(run.code, 0, "{}", run.out);
    }

    let mut newest: Vec<PathBuf> = bases[2..]
        .iter()
        .map(|base| cache(&tree).join(base))
        .collect();
    newest.sort();
    assert_eq!(cached_files(&tree), newest);
}

#[test]
fn an_evicted_base_falls_back_to_cold_and_keeps_the_same_verdict() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/caller.rs", "fn main() {}\n");
    let mut bases = Vec::new();
    for at in 0..6 {
        tree.write("src/lib.rs", &format!("pub fn api_{at}() {{}}\n"));
        tree.base();
        bases.push(tree.revision("main"));
        let run = changed(&tree);
        assert_eq!(run.code, 0, "{}", run.out);
    }

    assert!(!cache(&tree).join(&bases[0]).exists());
    tree.git(&["checkout", "-q", "-b", "evicted", &bases[0]]);
    tree.write("src/lib.rs", "pub fn api_0() {}\nfn newly_changed() {}\n");

    let evicted = changed(&tree);
    assert_eq!(evicted.json()["window"]["before"], bases[0].as_str());
    assert_eq!(counted(&facts(&evicted, "dead-symbols"))[3], 0);
    assert_eq!(evicted.code, 1, "{}", evicted.out);

    let warm = changed(&tree);
    assert!(counted(&facts(&warm, "dead-symbols"))[3] > 0);
    assert_eq!(judged(&evicted), judged(&warm));

    assert!(fs::remove_dir_all(cache(&tree)).is_ok());
    let cold = changed(&tree);
    assert_eq!(judged(&evicted), judged(&cold));
    assert_eq!(counted(&facts(&cold, "dead-symbols"))[3], 0);
}

#[test]
fn repeated_red_stops_keep_the_turn_base_through_a_prompt_and_a_branch_switch() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/main.rs", "fn main() {}\n");
    tree.base();
    assert_eq!(harness::feed(tree.root(), harness::AGENT, A_PROMPT).code, 0);
    tree.write("src/lib.rs", "pub fn api() {}\nfn debt() {}\n");
    let stop = || {
        let (_, report) = harness::stop_report(tree.root(), A_STOP, &[]);
        let row = harness::gate_rows(&report)
            .as_array()
            .and_then(|gates| gates.iter().find(|row| row["name"] == "dead-symbols"))
            .unwrap_or_else(|| panic!("no dead-symbols row in {report}"));
        (
            report["status"].clone(),
            report["window"]["before"].clone(),
            row["facts"]["cached"].clone(),
        )
    };

    let first = stop();
    let second = stop();
    assert_eq!(harness::feed(tree.root(), harness::AGENT, A_PROMPT).code, 0);
    tree.git(&["checkout", "-q", "-b", "elsewhere"]);
    let third = stop();

    assert_eq!(first.0, "FAIL", "{first:?}");
    assert_eq!(first.2, 0, "{first:?}");
    for later in [&second, &third] {
        assert_eq!((&later.0, &later.1), (&first.0, &first.1), "{later:?}");
        assert!(
            later.2.as_u64().is_some_and(|cached| cached > 0),
            "{later:?}"
        );
    }
}

/// The whole base's layout as a run recorded it, off the row of the gate that laid it out.
fn layout(run: &Run) -> Value {
    let report = run.json();
    harness::gate_rows(&report)
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row["names"]["layout"].clone())
        .find(|held| held.is_object())
        .unwrap_or_else(|| panic!("no whole-base layout in {report}"))
}

/// One run warm, which lays the whole base out without checking the base commit out, beside the
/// same run over a removed cache, which checks it out whole. The two must agree on the findings,
/// the notes, the coverage and the exit code. #203.
fn light_beside_whole(tree: &Tree, run: impl Fn(&Tree) -> Run) -> Run {
    run(tree);
    let warm = run(tree);
    assert!(
        layout(&warm)["written"].is_u64(),
        "the warm run checked the base out whole: {}",
        warm.out
    );
    assert!(fs::remove_dir_all(cache(tree)).is_ok());
    let whole = run(tree);
    assert!(
        layout(&whole)["written"].is_null(),
        "the run without a cache laid the base out light: {}",
        whole.out
    );
    assert_eq!(judged(&warm), judged(&whole));
    warm
}

const LAYERS: &str = r#"{"layering":{"layers":{"ui":{"in":"src/ui","can_use":[]},"domain":{"in":"src/domain","can_use":[]}}}}"#;

#[test]
fn a_manifest_the_working_tree_changed_is_read_from_the_base_on_a_light_layout() {
    let tree = Tree::new();
    tree.write("klin.json", LAYERS);
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    tree.write("src/lib.rs", "mod domain;\nmod ui;\n");
    tree.write(
        "src/other.rs",
        "mod domain;\nmod ui;\n#[path = \"ui/shared.rs\"]\nmod shared;\n",
    );
    tree.write("src/ui/mod.rs", "pub fn show() {}\n");
    tree.write("src/ui/shared.rs", "pub fn shared() {}\n");
    tree.write(
        "src/domain/mod.rs",
        "pub fn rule() { crate::shared::shared(); }\n",
    );
    tree.base();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[lib]\npath = \"src/other.rs\"\n",
    );

    let warm = light_beside_whole(&tree, changed);

    assert!(warm.says("layering"), "{}", warm.out);
}

#[test]
fn a_manifest_the_working_tree_renamed_is_read_from_the_base_at_its_base_path() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"layering":{"layers":{"ui":{"in":"lib/src/ui","can_use":[]},"domain":{"in":"lib/src/domain","can_use":[]}}}}"#);
    tree.write(
        "lib/Cargo.toml",
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    tree.write("lib/src/lib.rs", "mod domain;\nmod ui;\n");
    tree.write("lib/src/ui/mod.rs", "pub fn show() {}\n");
    tree.write(
        "lib/src/domain/mod.rs",
        "pub fn rule() { crate::ui::show(); }\n",
    );
    tree.write("src/extra.rs", "pub fn extra() {}\n");
    tree.write("src/user.rs", "fn main() { extra(); }\n");
    tree.base();
    for under in ["app/src/ui", "app/src/domain"] {
        assert!(fs::create_dir_all(tree.path(under)).is_ok());
    }
    for name in [
        "Cargo.toml",
        "src/lib.rs",
        "src/ui/mod.rs",
        "src/domain/mod.rs",
    ] {
        tree.git(&["mv", &format!("lib/{name}"), &format!("app/{name}")]);
    }
    tree.write(
        "klin.json",
        r#"{"layering":{"layers":{"ui":{"in":"app/src/ui","can_use":[]},"domain":{"in":"app/src/domain","can_use":[]}}}}"#,
    );

    let warm = light_beside_whole(&tree, changed);

    assert!(warm.says("layering"), "{}", warm.out);
}

#[test]
fn the_base_attributes_convert_the_base_bytes_where_the_working_tree_changed_them() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.git(&["config", "filter.drop.smudge", "sed -e s/helper//"]);
    tree.git(&["config", "filter.drop.clean", "cat"]);
    tree.write(
        ".gitattributes",
        "src/caller.rs filter=drop\n*.rs eol=crlf\n",
    );
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { helper(); }\n");
    tree.base();
    tree.write(".gitattributes", "*.rs eol=lf\n");
    tree.write("src/lib.rs", "pub fn api() {}\nfn helper() {}\n");

    let warm = light_beside_whole(&tree, changed);

    assert_eq!(warm.code, 1, "{}", warm.out);
    assert!(warm.says("fn helper() {}"), "{}", warm.out);
}

#[test]
fn a_renamed_attributes_file_converts_a_cache_miss_before_the_rename_moves_it() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.git(&["config", "filter.drop.smudge", "sed -e s/helper//"]);
    tree.git(&["config", "filter.drop.clean", "cat"]);
    tree.write("src/.gitattributes", "caller.rs filter=drop\n");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { helper(); }\n");
    tree.base();

    tree.write("src/caller.rs", "fn main() { helper(); helper(); }\n");
    assert_eq!(changed(&tree).code, 0);
    tree.write("src/caller.rs", "fn main() { helper(); }\n");
    assert!(fs::create_dir_all(tree.path("other")).is_ok());
    tree.git(&["mv", "src/.gitattributes", "other/.gitattributes"]);
    tree.write("src/lib.rs", "pub fn api() {}\nfn helper() {}\n");

    let warm = changed(&tree);
    assert!(
        layout(&warm)["written"].is_u64(),
        "the warm run checked the base out whole: {}",
        warm.out
    );
    assert!(fs::remove_dir_all(cache(&tree)).is_ok());
    let whole = changed(&tree);

    assert_eq!(judged(&warm), judged(&whole));
    assert_eq!(warm.code, 1, "{}", warm.out);
    assert!(warm.says("fn helper() {}"), "{}", warm.out);
}

#[test]
fn a_base_file_the_cache_lacks_is_written_from_the_index() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { api(); }\n");
    tree.base();

    tree.write("src/caller.rs", "fn main() { api(); api(); }\n");
    assert_eq!(changed(&tree).code, 0);
    tree.write("src/caller.rs", "fn main() { api(); }\n");
    tree.write("src/lib.rs", "pub fn api() {}\nfn spare() {}\n");

    let warm = changed(&tree);
    assert!(layout(&warm)["written"].is_u64(), "{}", warm.out);
    assert!(fs::remove_dir_all(cache(&tree)).is_ok());

    assert_eq!(judged(&warm), judged(&changed(&tree)));
    assert_eq!(warm.code, 1, "{}", warm.out);
    assert!(warm.says("fn spare() {}"), "{}", warm.out);
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_the_base_holds_is_no_file_of_the_base_tree() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { api(); }\n");
    let linked = std::os::unix::fs::symlink("lib.rs", tree.path("src/linked.rs"));
    assert!(linked.is_ok(), "a symbolic link");
    tree.base();
    tree.write("src/lib.rs", "pub fn api() {}\nfn spare() {}\n");

    let warm = light_beside_whole(&tree, changed);

    assert_eq!(warm.code, 1, "{}", warm.out);
    assert!(warm.says("fn spare() {}"), "{}", warm.out);
}

#[test]
fn a_sparse_checkout_the_light_layout_does_not_read_checks_the_base_out_whole() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { api(); }\n");
    tree.base();
    tree.write("src/lib.rs", "pub fn api() {}\nfn spare() {}\n");

    changed(&tree);
    let light = changed(&tree);
    assert!(layout(&light)["written"].is_u64(), "{}", light.out);

    for (set, sparse) in [
        ("TRUE", true),
        ("on", true),
        ("1", true),
        ("off", false),
        ("no", false),
        ("0", false),
    ] {
        tree.git(&["config", "core.sparseCheckout", set]);
        changed(&tree);
        let run = changed(&tree);
        assert_eq!(
            layout(&run)["written"].is_null(),
            sparse,
            "{set}: {}",
            run.out
        );
        assert_eq!(judged(&run), judged(&light), "{set}");
    }
}

/// A value git will not read as a boolean stops git itself, so the run reports an error and
/// measures nothing. It never reads the value as false and lays the base out light on that
/// reading.
#[test]
fn a_boolean_git_refuses_never_lets_the_light_layout_guess() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { api(); }\n");
    tree.base();
    tree.write("src/lib.rs", "pub fn api() {}\nfn spare() {}\n");

    changed(&tree);
    assert!(layout(&changed(&tree))["written"].is_u64());

    for name in ["core.sparseCheckout", "core.symlinks"] {
        tree.git(&["config", name, "banana"]);
        let run = changed(&tree);
        tree.git(&["config", "--unset", name]);
        assert_eq!(run.code, 2, "{name}: {}", run.out);
        assert!(!run.says("\"written\""), "{name}: {}", run.out);
    }
}

#[cfg(unix)]
#[test]
fn a_symlink_setting_git_spells_another_way_reads_as_git_reads_it() {
    for set in ["on", "off"] {
        let tree = Tree::new();
        tree.write("klin.json", "{}");
        tree.git(&["config", "core.symlinks", set]);
        tree.write("src/lib.rs", "pub fn api() {}\n");
        tree.write("src/caller.rs", "fn main() { api(); }\n");
        let linked = std::os::unix::fs::symlink("lib.rs", tree.path("src/linked.rs"));
        assert!(linked.is_ok(), "a symbolic link");
        tree.base();
        tree.write("src/lib.rs", "pub fn api() {}\nfn spare() {}\n");

        let warm = light_beside_whole(&tree, changed);

        assert_eq!(warm.code, 1, "{set}: {}", warm.out);
        assert!(warm.says("fn spare() {}"), "{set}: {}", warm.out);
    }
}

#[test]
fn the_base_scope_comes_from_the_base_configuration_on_a_light_layout() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"dead_symbols":{"in":["src"]}}"#);
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { api(); }\n");
    tree.write(
        "web/widget.ts",
        "function spare() {}\nexport function used() {}\n",
    );
    tree.write("web/main.ts", "import { used } from './widget';\nused();\n");
    tree.base();
    tree.write("klin.json", r#"{"dead_symbols":{"in":["src","web"]}}"#);
    tree.write("src/lib.rs", "pub fn api() {}\nfn spare() {}\n");

    let warm = light_beside_whole(&tree, changed);

    assert_eq!(warm.code, 1, "{}", warm.out);
}

#[test]
fn a_strict_run_a_whole_run_and_a_damaged_cache_check_the_base_out_whole() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() { api(); }\n");
    tree.base();
    tree.write("src/lib.rs", "pub fn api() {}\nfn spare() {}\n");

    changed(&tree);
    let light = changed(&tree);
    let strict = tree.run(&["check", "--json", "--changed", "dead-symbols"]);
    let whole = tree.run(&["check", "--json", "dead-symbols"]);
    for file in cached_files(&tree) {
        assert!(fs::write(&file, b"not a cache").is_ok());
    }
    let damaged = changed(&tree);

    assert!(layout(&light)["written"].is_u64(), "{}", light.out);
    for run in [&strict, &whole, &damaged] {
        assert!(layout(run)["written"].is_null(), "{}", run.out);
    }
    assert_eq!(judged(&damaged), judged(&light));
    assert_eq!(strict.code, light.code, "{}", strict.out);
}
