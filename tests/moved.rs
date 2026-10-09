//! Moved and deleted policy paths: a pinned `in` path, a pinned document or a convention's `in`
//! path whose files the change renamed, deleted, or that selects nothing in either tree, and a
//! file renamed out of a scope that still selects other files. Spec 7.3.

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

fn reviews<'a>(report: &'a Value, kind: &str) -> Vec<&'a Value> {
    report["reviews"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|review| review["kind"] == kind)
        .collect()
}

#[test]
fn a_shell_script_renamed_into_a_skipped_directory_is_a_review_item() {
    for extension in ["sh", "bash", "zsh"] {
        let tree = pinned(CONFIG);
        tree.write(
            &format!("scripts/deploy.{extension}"),
            "set +e\necho deployed\n",
        );
        tree.base();
        std::fs::create_dir_all(tree.path("scripts/out")).unwrap_or_default();
        let to = format!("scripts/out/deploy.{extension}");
        tree.git(&["mv", &format!("scripts/deploy.{extension}"), &to]);

        let report = tree.run(&["check", "--json"]).json();
        let hidden = reviews(&report, "moved-skipped");
        assert_eq!(hidden.len(), 1, "{extension}: {report}");
        assert_eq!(hidden[0]["file"], to.as_str(), "{extension}: {report}");
    }
}

#[test]
fn the_last_source_file_renamed_into_a_skipped_directory_is_still_reported() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("lib/a.rs", &many());
    tree.base();
    std::fs::create_dir_all(tree.path("out")).unwrap_or_default();
    tree.git(&["mv", "lib/a.rs", "out/a.rs"]);

    let report = tree.run(&["check", "--json"]).json();
    let hidden = reviews(&report, "moved-skipped");
    assert_eq!(hidden.len(), 1, "{report}");
    assert_eq!(hidden[0]["file"], "out/a.rs", "{report}");

    let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
    assert!(
        stop.says("NOTE: lib/a.rs moved to out/a.rs"),
        "{}",
        stop.out
    );
}

#[test]
fn a_pin_whose_files_moved_into_a_skipped_directory_does_not_claim_they_are_measured() {
    let tree = pinned(CONFIG);
    std::fs::create_dir_all(tree.path("src/out")).unwrap_or_default();
    tree.git(&["mv", "src/core/a.rs", "src/out/a.rs"]);
    tree.git(&["mv", "src/core/b.rs", "src/out/b.rs"]);

    let report = tree.run(&["check", "--json"]).json();
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    let said = pins[0]["text"].as_str().unwrap_or_default();
    assert!(!said.contains("measures them there"), "{said}");
    assert!(
        said.contains("2 file(s) moved under a directory every walk skips"),
        "{said}"
    );
    assert_eq!(reviews(&report, "moved-skipped").len(), 2, "{report}");
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

const DOCUMENT: &str = r#"{"doc_size": {"docs/guide.md": 100}}"#;

/// A document of one word per line, so git still detects a rename after a few words are added.
fn document(words: usize) -> String {
    "word\n".repeat(words)
}

/// A base whose `doc_size` pins `docs/guide.md` at 100 words, which it is under, beside source.
fn documented(words: usize) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", DOCUMENT);
    tree.write("docs/guide.md", &document(words));
    tree.write("src/lib.rs", SIMPLE);
    tree.base();
    tree
}

#[test]
fn renaming_a_pinned_document_keeps_it_measured_and_adds_a_moved_pin() {
    let tree = documented(90);
    tree.git(&["mv", "docs/guide.md", "docs/manual.md"]);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["judgement"], "review", "{report}");
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    assert_eq!(pins[0]["check"], "doc-size", "{report}");
    assert_eq!(pins[0]["file"], "docs/guide.md", "{report}");
    assert_eq!(
        pins[0]["reason"], "docs/guide.md -> docs/manual.md",
        "{report}"
    );

    let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
    assert_eq!(stop.code, 0, "{}", stop.out);
    assert!(!stop.says("NOTE: the pinned document"), "{}", stop.out);

    tree.write("docs/manual.md", &document(110));
    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("docs/manual.md"), "{}", run.out);
    assert!(
        run.says("REVIEW: the pinned document docs/guide.md"),
        "{}",
        run.out
    );
}

#[test]
fn a_renamed_pinned_document_over_its_ceiling_in_the_base_is_held() {
    let tree = documented(150);
    tree.git(&["mv", "docs/guide.md", "docs/manual.md"]);

    let run = tree.run(&["check", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("docs/manual.md is 150 words, over its ceiling of 100, held at the base"),
        "{}",
        run.out
    );
}

#[test]
fn deleting_a_pinned_document_is_a_moved_pin_and_no_error() {
    let tree = documented(90);
    tree.remove("docs/guide.md");

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["execution"], "ok", "{report}");
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    let said = pins[0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("1 file(s) went with no rename"), "{said}");

    let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
    assert_eq!(stop.code, 0, "{}", stop.out);
    assert!(
        stop.says("NOTE: the pinned document docs/guide.md"),
        "{}",
        stop.out
    );
}

#[test]
fn a_pinned_document_this_change_wrote_that_names_no_file_stays_an_error() {
    let tree = documented(90);
    tree.write("klin.json", r#"{"doc_size": {"docs/typo.md": 100}}"#);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(moved_pins(&run.json()).is_empty(), "{}", run.out);
}

const CONVENTION: &str = r#"{"conventions": {"no-spawn": {
    "code": "Command::new($$$ARGS)",
    "in": "src/core",
    "remedy": "Use the shared boundary."
}}}"#;

const SPAWN: &str = "pub fn spawn() {\n    Command::new(\"git\");\n}\n";

#[test]
fn renaming_the_files_a_convention_in_names_keeps_them_measured_and_adds_a_moved_pin() {
    let tree = pinned(CONVENTION);
    std::fs::create_dir_all(tree.path("src/engine")).unwrap_or_default();
    tree.git(&["mv", "src/core/a.rs", "src/engine/a.rs"]);
    tree.git(&["mv", "src/core/b.rs", "src/engine/b.rs"]);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["judgement"], "review", "{report}");
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    assert_eq!(pins[0]["check"], "conventions", "{report}");
    assert_eq!(pins[0]["file"], "src/core", "{report}");
    let said = pins[0]["text"].as_str().unwrap_or_default();
    assert!(said.contains("convention \"no-spawn\""), "{said}");

    let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
    assert_eq!(stop.code, 0, "{}", stop.out);
    assert!(!stop.says("NOTE: the pinned"), "{}", stop.out);

    tree.write("src/engine/a.rs", &format!("{}{SPAWN}", many()));
    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/engine/a.rs"), "{}", run.out);
    assert!(run.says("Use the shared boundary."), "{}", run.out);
}

#[test]
fn deleting_the_files_a_convention_in_names_is_a_moved_pin_and_no_error() {
    let tree = pinned(CONVENTION);
    tree.remove("src/core/a.rs");
    tree.remove("src/core/b.rs");

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(report["execution"], "ok", "{report}");
    assert_eq!(moved_pins(&report).len(), 1, "{report}");

    let stop = harness::feed(tree.root(), harness::AGENT, A_STOP);
    assert_eq!(stop.code, 0, "{}", stop.out);
    assert!(
        stop.says("NOTE: the pinned \\\"in\\\" path src/core of convention"),
        "{}",
        stop.out
    );
}

#[test]
fn a_pinned_document_under_a_skipped_directory_is_still_measured() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"doc_size": {"build/notes.md": 100}}"#);
    tree.write("build/notes.md", &document(90));
    tree.write("src/lib.rs", SIMPLE);
    tree.base();
    tree.write("build/notes.md", &document(110));

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(moved_pins(&run.json()).is_empty(), "{}", run.out);
}

#[test]
fn a_convention_in_path_the_base_holds_that_selects_nothing_in_either_tree_is_a_moved_pin() {
    let tree = pinned(&CONVENTION.replace("src/core", "src/gone"));

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let pins = moved_pins(&report);
    assert_eq!(pins.len(), 1, "{report}");
    assert_eq!(pins[0]["file"], "src/gone", "{report}");
}
