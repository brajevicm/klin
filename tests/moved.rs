//! Moved and deleted policy paths: a pinned `in` path whose files the change renamed, deleted,
//! or that selects nothing in either tree, and a file renamed out of a scope that still selects
//! other files. Spec 7.3.

mod harness;

use harness::Tree;
use serde_json::Value;

const CONFIG: &str = r#"{"complexity": {"in": "src/core", "cc": 8, "lines": 60}}"#;
const SIMPLE: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

/// A function with cyclomatic complexity 13, over the pinned ceiling of 8.
fn complex() -> String {
    let arms: String = (0..12)
        .map(|at| format!("    if a == {at} {{\n        return {at};\n    }}\n"))
        .collect();
    format!("pub fn knotted(a: i32) -> i32 {{\n{arms}    a\n}}\n")
}

/// Simple functions enough that one complex function added keeps git's rename above 50%.
fn many() -> String {
    (0..30)
        .map(|at| format!("pub fn simple_{at}(a: i32) -> i32 {{\n    a + {at}\n}}\n"))
        .collect()
}

/// A base whose `complexity` pins `src/core`, which holds two simple files.
fn pinned(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.write("src/core/a.rs", &many());
    tree.write("src/core/b.rs", SIMPLE);
    tree.write("src/other.rs", SIMPLE);
    tree.base();
    tree
}

fn moved_pins(report: &Value) -> Vec<&Value> {
    report["reviews"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|review| review["kind"] == "moved-pin")
        .collect()
}

#[test]
fn renaming_the_files_a_pinned_in_names_keeps_them_measured_and_adds_a_moved_pin() {
    let tree = pinned(CONFIG);
    std::fs::create_dir_all(tree.path("src/engine")).unwrap_or_default();
    tree.git(&["mv", "src/core/a.rs", "src/engine/a.rs"]);
    tree.git(&["mv", "src/core/b.rs", "src/engine/b.rs"]);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["judgement"], "review", "{report}");
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    assert_eq!(pins[0]["check"], "complexity", "{report}");
    assert_eq!(pins[0]["file"], "src/core", "{report}");
    let said = pins[0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("src/engine/a.rs, src/engine/b.rs"), "{said}");
    assert_eq!(
        pins[0]["reason"], "src/core/a.rs -> src/engine/a.rs, src/core/b.rs -> src/engine/b.rs",
        "{report}"
    );

    tree.write("src/engine/a.rs", &format!("{}{}", many(), complex()));
    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  complexity"), "{}", run.out);
    assert!(run.says("src/engine/a.rs"), "{}", run.out);
    assert!(
        run.says("REVIEW: the pinned \"in\" path src/core"),
        "{}",
        run.out
    );
}

#[test]
fn deleting_the_files_a_pinned_in_names_is_a_moved_pin_and_no_error() {
    let tree = pinned(CONFIG);
    tree.remove("src/core/a.rs");
    tree.remove("src/core/b.rs");

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["execution"], "ok", "{report}");
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    let said = pins[0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("2 file(s) went with no rename"), "{said}");

    let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
    assert_eq!(stop.code, 0, "{}", stop.out);
    assert!(
        stop.says("NOTE: the pinned \\\"in\\\" path src/core"),
        "{}",
        stop.out
    );
}

#[test]
fn a_pin_the_base_holds_that_selects_nothing_in_either_tree_is_a_moved_pin() {
    let tree = pinned(r#"{"complexity": {"in": "src/gone", "cc": 8, "lines": 60}}"#);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    assert_eq!(pins[0]["file"], "src/gone", "{report}");
}

#[test]
fn a_pin_this_change_wrote_that_selects_nothing_stays_a_configuration_error() {
    let tree = pinned(CONFIG);
    tree.write(
        "klin.json",
        r#"{"complexity": {"in": "src/typo", "cc": 8, "lines": 60}}"#,
    );

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(
        run.json()["errors"][0]["kind"],
        "configuration",
        "{}",
        run.out
    );
}

#[test]
fn a_complex_function_in_a_file_moved_out_of_an_in_scope_still_fails() {
    let tree = pinned(CONFIG);
    std::fs::create_dir_all(tree.path("src/elsewhere")).unwrap_or_default();
    tree.git(&["mv", "src/core/a.rs", "src/elsewhere/a.rs"]);
    tree.write("src/elsewhere/a.rs", &format!("{}{}", many(), complex()));

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    let finding = report["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|finding| finding["file"] == "src/elsewhere/a.rs")
        .unwrap_or_else(|| panic!("no finding at src/elsewhere/a.rs in {report}"));
    assert_eq!(finding["check"], "complexity", "{report}");
    assert_eq!(finding["values"]["moved_out_of_scope"], true, "{report}");
    assert!(moved_pins(&report).is_empty(), "{report}");

    let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
    assert_eq!(stop.code, 2, "{}", stop.out);
    assert!(stop.says("src/elsewhere/a.rs"), "{}", stop.out);
}

#[test]
fn a_file_renamed_into_a_skipped_directory_is_a_review_item_and_a_stop_note() {
    for config in [CONFIG, r#"{"complexity": {"cc": 8, "lines": 60}}"#] {
        let tree = pinned(config);
        std::fs::create_dir_all(tree.path("src/out")).unwrap_or_default();
        tree.git(&["mv", "src/core/a.rs", "src/out/a.rs"]);
        tree.write("src/out/a.rs", &format!("{}{}", many(), complex()));

        let run = tree.run(&["check", "--json"]);
        assert_eq!(run.code, 0, "{config}: {}", run.out);
        let report = run.json();
        assert_eq!(report["judgement"], "review", "{config}: {report}");
        let hidden: Vec<&Value> = report["reviews"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|review| review["kind"] == "moved-skipped")
            .collect();
        assert_eq!(hidden.len(), 1, "{config}: {report}");
        assert_eq!(hidden[0]["file"], "src/out/a.rs", "{config}: {report}");
        assert_eq!(
            hidden[0]["reason"], "src/core/a.rs -> src/out/a.rs",
            "{config}: {report}"
        );

        let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
        assert_eq!(stop.code, 0, "{config}: {}", stop.out);
        assert!(
            stop.says("NOTE: src/core/a.rs moved to src/out/a.rs"),
            "{config}: {}",
            stop.out
        );
    }
}

#[test]
fn a_file_no_language_reads_renamed_into_a_skipped_directory_says_nothing() {
    let tree = pinned(CONFIG);
    tree.write("tests/data.json", "{}\n");
    tree.base();
    std::fs::create_dir_all(tree.path("tests/fixtures")).unwrap_or_default();
    tree.git(&["mv", "tests/data.json", "tests/fixtures/data.json"]);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["judgement"], "pass", "{report}");
}

/// A Stop builds before it measures, so a file the build writes or rewrites is measured even
/// where a section states an `in`. Spec 6.4.
#[test]
fn a_stop_measures_what_its_build_wrote_under_an_in_scope() {
    for target in ["src/core/made.rs", "src/core/a.rs"] {
        let config = format!(
            r#"{{"build": "cp made.txt {target}",
                 "complexity": {{"in": "src/core", "cc": 8, "lines": 60}}}}"#
        );
        let tree = pinned(&config);
        tree.write("made.txt", &format!("{}{}", many(), complex()));
        tree.write("src/core/b.rs", &format!("{SIMPLE}\n"));

        let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
        assert_eq!(stop.code, 2, "{target}: {}", stop.out);
        assert!(stop.says(target), "{target}: {}", stop.out);
    }
}

/// A moved pin excuses only its own path: a nonexistent path the change wrote beside it is still
/// a configuration error. Spec 7.3.
#[test]
fn a_new_path_beside_a_moved_pin_that_selects_nothing_is_a_configuration_error() {
    let tree = pinned(CONFIG);
    std::fs::create_dir_all(tree.path("src/engine")).unwrap_or_default();
    tree.git(&["mv", "src/core/a.rs", "src/engine/a.rs"]);
    tree.git(&["mv", "src/core/b.rs", "src/engine/b.rs"]);
    tree.write(
        "klin.json",
        r#"{"complexity": {"in": ["src/core", "src/typo"], "cc": 8, "lines": 60}}"#,
    );

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert_eq!(report["errors"][0]["kind"], "configuration", "{report}");
    assert_eq!(report["errors"][0]["check"], "complexity", "{report}");
}

/// A run that selects one check reports only that check's moved pins. Spec 7.3, 11.3.
#[test]
fn a_selected_check_reports_no_moved_pin_of_a_check_it_did_not_select() {
    let tree = pinned(
        r#"{"complexity": {"in": "src/core", "cc": 8, "lines": 60},
            "doc_size": {"README.md": 10}}"#,
    );
    tree.words("README.md", 5);
    tree.base();
    std::fs::create_dir_all(tree.path("src/engine")).unwrap_or_default();
    tree.git(&["mv", "src/core/a.rs", "src/engine/a.rs"]);
    tree.git(&["mv", "src/core/b.rs", "src/engine/b.rs"]);

    let run = tree.run(&["check", "doc-size", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["judgement"], "pass", "{report}");
    assert!(moved_pins(&report).is_empty(), "{report}");
}
