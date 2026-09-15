mod harness;

use std::fs;
use std::path::PathBuf;

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

fn changed(tree: &Tree) -> Run {
    tree.run(&[
        "gate",
        "--json",
        "--changed",
        "--gate",
        "dead-symbols",
        "--gate",
        "reachability",
    ])
}

fn dead_symbols(tree: &Tree) -> Run {
    tree.run(&["gate", "--json", "--changed", "--gate", "dead-symbols"])
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
    for row in report["gates"].as_array_mut().into_iter().flatten() {
        if let Some(fields) = row.as_object_mut() {
            fields.remove("ms");
            fields.remove("facts");
            fields.remove("names");
        }
    }
    json!({"code": run.code, "report": report})
}

fn facts(run: &Run, gate: &str) -> Value {
    let report = run.json();
    report["gates"]
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
    let referenced = dead_symbols(&tree);
    assert_eq!(referenced.code, 0, "{}", referenced.out);
    assert_eq!(cached_files(&tree), [cache(&tree).join(&first)]);

    tree.write("src/lib.rs", "pub fn api() {}\n");
    tree.write("src/caller.rs", "fn main() {}\n");
    tree.base();
    let second = tree.revision("main");
    tree.write("src/lib.rs", "pub fn api() {}\nfn helper() {}\n");
    assert!(fs::copy(cache(&tree).join(&first), cache(&tree).join(&second)).is_ok());
    let run = dead_symbols(&tree);

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
    let referenced = dead_symbols(&tree);
    assert_eq!(referenced.code, 0, "{}", referenced.out);

    tree.git(&["config", "filter.drop.smudge", "sed -e s/helper//"]);
    tree.git(&["config", "filter.drop.clean", "cat"]);
    tree.write(".git/info/attributes", "src/caller.rs filter=drop\n");
    let warm = dead_symbols(&tree);
    assert!(fs::remove_dir_all(cache(&tree)).is_ok());
    let cold = dead_symbols(&tree);

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
        let run = dead_symbols(&tree);
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
        let run = dead_symbols(&tree);
        assert_eq!(run.code, 0, "{}", run.out);
    }

    assert!(!cache(&tree).join(&bases[0]).exists());
    tree.git(&["checkout", "-q", "-b", "evicted", &bases[0]]);
    tree.write("src/lib.rs", "pub fn api_0() {}\nfn newly_changed() {}\n");

    let evicted = dead_symbols(&tree);
    assert_eq!(evicted.json()["window"]["before"], bases[0].as_str());
    assert_eq!(counted(&facts(&evicted, "dead-symbols"))[3], 0);
    assert_eq!(evicted.code, 1, "{}", evicted.out);

    let warm = dead_symbols(&tree);
    assert!(counted(&facts(&warm, "dead-symbols"))[3] > 0);
    assert_eq!(judged(&evicted), judged(&warm));

    assert!(fs::remove_dir_all(cache(&tree)).is_ok());
    let cold = dead_symbols(&tree);
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
    assert_eq!(harness::feed(tree.root(), &["radius"], A_PROMPT).code, 0);
    tree.write("src/lib.rs", "pub fn api() {}\nfn debt() {}\n");
    let stop = || {
        let run = harness::feed(
            tree.root(),
            &[
                "gate",
                "--hook",
                "--changed",
                "--json",
                "--gate",
                "dead-symbols",
            ],
            A_STOP,
        );
        let report: Value = run
            .out
            .lines()
            .find_map(|line| serde_json::from_str(line).ok())
            .unwrap_or_else(|| panic!("no report in {}", run.out));
        let row = &report["gates"][0];
        (
            report["status"].clone(),
            report["window"]["before"].clone(),
            row["facts"]["cached"].clone(),
        )
    };

    let first = stop();
    let second = stop();
    assert_eq!(harness::feed(tree.root(), &["radius"], A_PROMPT).code, 0);
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
