//! Evidence klin could not measure: a `measurement-lost` FAIL where the change made a file the
//! base measured unmeasurable, an `unmeasured` review item where the change opened a gap without
//! a clear agent cause, and a coverage note for klin's own limit the change did not open.
//! Spec 7.2.

mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "pub fn one(a: i32) -> i32 {\n    a + 1\n}\n";
const BROKEN: &str = "pub fn one(a: i32 -> i32 {\n    a + 1\n}\n";
const CONFIG: &str = r#"{"doc_size": {"README.md": 10}}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

/// A tree whose base holds a clean source file and a short document, under `config`.
fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

fn checked(tree: &Tree, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["check", "--json"];
    all.extend_from_slice(args);
    let run = tree.run(&all);
    (run.code, run.json())
}

fn list<'a>(report: &'a Value, key: &str) -> &'a [Value] {
    report[key]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("no list {key} in {report}"))
}

fn row<'a>(report: &'a Value, name: &str) -> &'a Value {
    list(report, "capabilities")
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("no row {name} in {report}"))
}

fn lost(report: &Value) -> Vec<&Value> {
    list(report, "findings")
        .iter()
        .filter(|finding| finding["kind"] == "measurement-lost")
        .collect()
}

fn stop(tree: &Tree) -> harness::Run {
    harness::feed(tree.root(), &["gate", "--hook"], A_STOP)
}

#[test]
fn a_file_the_change_made_unparseable_is_one_measurement_lost_fail() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", BROKEN);

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(report["judgement"], "fail", "{report}");
    assert_eq!(report["execution"], "ok", "{report}");
    let found = lost(&report);
    assert_eq!(found.len(), 1, "{report}");
    let finding = found[0];
    assert_eq!(finding["check"], Value::Null, "{report}");
    assert_eq!(finding["outcome"], "new", "{report}");
    assert_eq!(finding["file"], "src/lib.rs", "{report}");
    assert_eq!(finding["line"], Value::Null, "{report}");
    assert_eq!(finding["text"], "src/lib.rs", "{report}");
    assert_eq!(finding["values"]["reason"], "parse", "{report}");
    assert_eq!(finding["values"]["line"], 1, "{report}");
    assert!(finding["values"]["column"].is_u64(), "{report}");
    assert!(
        finding["remedy"]
            .as_str()
            .is_some_and(|remedy| remedy.contains("line 1")),
        "{report}"
    );
    let built_in = row(&report, "measurement-lost");
    assert_eq!(built_in["kind"], "built-in", "{report}");
    assert_eq!(built_in["judgement"], "fail", "{report}");
    assert!(report["not_measured"].as_u64() >= Some(1), "{report}");
}

#[test]
fn a_lost_file_has_one_id_whatever_gates_the_run_selects() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", BROKEN);

    let ids: Vec<Value> = [&[][..], &["complexity"], &["dead-symbols"]]
        .iter()
        .map(|gates| {
            let (code, report) = checked(&tree, gates);
            assert_eq!(code, 1, "{gates:?}: {report}");
            let found = lost(&report);
            assert_eq!(found.len(), 1, "{gates:?}: {report}");
            found[0]["id"].clone()
        })
        .collect();

    assert!(ids[0].is_string(), "{ids:?}");
    assert!(ids.iter().all(|id| *id == ids[0]), "{ids:?}");
}

#[test]
fn a_lost_file_prints_its_own_row_and_no_hole_under_the_gates() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", BROKEN);

    let run = tree.run(&["check"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  measurement-lost"), "{}", run.out);
    assert!(run.says("src/lib.rs"), "{}", run.out);
    assert!(!run.says("could not parse"), "{}", run.out);
}

#[test]
fn a_run_that_selects_no_capability_reading_the_file_reports_no_loss() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", BROKEN);

    let (code, report) = checked(&tree, &["doc-size"]);

    assert_eq!(code, 0, "{report}");
    assert!(lost(&report).is_empty(), "{report}");
}

#[test]
fn a_new_file_whose_parse_has_an_error_node_is_one_unmeasured_review_item() {
    let tree = tree(CONFIG);
    tree.write("src/new.rs", BROKEN);

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert_eq!(report["judgement"], "review", "{report}");
    assert!(lost(&report).is_empty(), "{report}");
    let reviews = list(&report, "reviews");
    assert_eq!(reviews.len(), 1, "{report}");
    assert_eq!(reviews[0]["kind"], "unmeasured", "{report}");
    assert_eq!(reviews[0]["reason"], "unreadable", "{report}");
    assert_eq!(reviews[0]["file"], "src/new.rs", "{report}");
    assert_eq!(report["not_measured"], 1, "{report}");
    assert!(
        row(&report, "complexity")["coverage"]["gaps"] == 1,
        "{report}"
    );
}

#[test]
fn a_file_the_base_could_not_parse_either_is_a_coverage_note() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.write("src/odd.rs", BROKEN);
    tree.base();

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert_eq!(report["judgement"], "pass", "{report}");
    assert!(list(&report, "reviews").is_empty(), "{report}");
    let notes: Vec<&Value> = list(&report, "notes")
        .iter()
        .filter(|note| note["coverage"] == true)
        .collect();
    assert_eq!(notes.len(), 1, "{report}");
    assert_eq!(notes[0]["kind"], "unreadable", "{report}");
    assert_eq!(notes[0]["file"], "src/odd.rs", "{report}");
    assert_eq!(report["not_measured"], 1, "{report}");
    assert!(
        row(&report, "complexity")["coverage"]["limits"] == 1,
        "{report}"
    );
}

#[test]
fn a_lost_file_blocks_the_stop() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", BROKEN);

    let run = stop(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("measurement-lost"), "{}", run.out);
    assert!(run.says("src/lib.rs"), "{}", run.out);
    assert!(!run.says("klin policy"), "{}", run.out);
}

#[test]
fn an_opened_gap_is_a_note_at_the_stop_and_does_not_block() {
    let tree = tree(CONFIG);
    tree.write("src/new.rs", BROKEN);

    let run = stop(&tree);

    assert_ne!(run.code, 2, "{}", run.out);
    assert!(run.says("src/new.rs"), "{}", run.out);
}

/// A comment line of exactly this many bytes, which is at the source-line ceiling and not over.
fn comment(bytes: usize) -> String {
    format!("// {}", "a".repeat(bytes - 3))
}

#[test]
fn a_line_over_the_ceiling_added_to_a_measured_file_is_a_measurement_lost_fail() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", &format!("{CLEAN}{}\n", comment(65_537)));

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    let found = lost(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["values"]["reason"], "line-ceiling", "{report}");
}

#[test]
fn a_new_file_with_a_line_over_the_ceiling_is_a_resource_limit_review_item() {
    let tree = tree(CONFIG);
    tree.write("src/bundle.js", &comment(65_537));

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    let reviews = list(&report, "reviews");
    assert_eq!(reviews.len(), 1, "{report}");
    assert_eq!(reviews[0]["reason"], "resource-limit", "{report}");
}

#[test]
fn a_crlf_working_tree_does_not_trip_the_line_ceiling() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", &format!("{CLEAN}{}\n", comment(65_536)));
    tree.base();
    tree.write(
        "src/lib.rs",
        &format!("{CLEAN}{}\n", comment(65_536)).replace('\n', "\r\n"),
    );

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert!(lost(&report).is_empty(), "{report}");
    assert!(list(&report, "reviews").is_empty(), "{report}");
}

const CARGO: &str = "[package]\nname = \"t\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const CARGO_LOCK: &str = "version = 3\n\n[[package]]\nname = \"t\"\nversion = \"0.1.0\"\n";

/// A Rust package whose base holds a manifest and the lockfile beside it.
fn package() -> Tree {
    let tree = tree(CONFIG);
    tree.write("Cargo.toml", CARGO);
    tree.write("Cargo.lock", CARGO_LOCK);
    tree.base();
    tree
}

#[test]
fn a_manifest_the_change_made_invalid_is_a_measurement_lost_fail() {
    let tree = package();
    tree.write("Cargo.toml", "[package\nname = \"t\"\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    let found = lost(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], "Cargo.toml", "{report}");
    assert_eq!(found[0]["values"]["reason"], "manifest", "{report}");
}

#[test]
fn a_lockfile_the_change_made_invalid_is_a_measurement_lost_fail() {
    let tree = package();
    tree.write("Cargo.lock", "[[package\n");

    let (code, report) = checked(&tree, &["lockfile"]);

    assert_eq!(code, 1, "{report}");
    let found = lost(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0]["file"], "Cargo.lock", "{report}");
}

#[test]
fn a_new_manifest_klin_cannot_parse_is_an_unreadable_review_item() {
    let tree = package();
    tree.write("tools/package.json", "{ not json");

    let (code, report) = checked(&tree, &["lockfile"]);

    assert_eq!(code, 0, "{report}");
    let reviews = list(&report, "reviews");
    assert_eq!(reviews.len(), 1, "{report}");
    assert_eq!(reviews[0]["file"], "tools/package.json", "{report}");
    assert_eq!(reviews[0]["reason"], "unreadable", "{report}");
}

fn lost_for(report: &Value, file: &str) -> Vec<String> {
    lost(report)
        .iter()
        .filter(|finding| finding["file"] == file)
        .filter_map(|finding| finding["values"]["reason"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_nul_byte_in_a_measured_file_is_a_lost_form() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", &format!("{CLEAN}\0\n"));

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/lib.rs"), ["form"], "{report}");
}

#[test]
fn a_measured_file_the_change_re_encodes_is_a_lost_form() {
    let tree = tree(CONFIG);
    let utf16: String = CLEAN.chars().flat_map(|unit| [unit, '\0']).collect();
    tree.write("src/lib.rs", &utf16);

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/lib.rs"), ["form"], "{report}");
}

#[cfg(unix)]
#[test]
fn a_measured_file_the_change_replaces_with_a_symbolic_link_is_a_lost_form() {
    let tree = tree(CONFIG);
    tree.write("src/other.rs", CLEAN);
    tree.remove("src/lib.rs");
    assert!(std::os::unix::fs::symlink("other.rs", tree.path("src/lib.rs")).is_ok());

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/lib.rs"), ["form"], "{report}");
}

#[test]
fn a_binary_attribute_the_change_gives_a_measured_file_is_a_lost_form() {
    let tree = tree(CONFIG);
    tree.write(".gitattributes", "src/lib.rs binary\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/lib.rs"), ["form"], "{report}");
}

#[test]
fn a_minus_text_attribute_makes_no_file_lost() {
    let tree = tree(CONFIG);
    tree.write(".gitattributes", "* -text\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert!(lost(&report).is_empty(), "{report}");
    assert_eq!(report["not_measured"], 0, "{report}");
}

#[test]
fn an_inherited_filter_is_a_coverage_note_and_an_added_one_is_a_lost_form() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.write("src/art.rs", CLEAN);
    tree.write(".gitattributes", "src/art.rs filter=lfs\n");
    tree.base();

    let (code, report) = checked(&tree, &[]);
    assert_eq!(code, 0, "{report}");
    let notes: Vec<&Value> = list(&report, "notes")
        .iter()
        .filter(|note| note["coverage"] == true)
        .collect();
    assert_eq!(notes.len(), 1, "{report}");
    assert_eq!(notes[0]["kind"], "filtered", "{report}");
    assert_eq!(notes[0]["file"], "src/art.rs", "{report}");

    tree.write(
        ".gitattributes",
        "src/art.rs filter=lfs\nsrc/lib.rs filter=lfs\n",
    );
    let (code, report) = checked(&tree, &[]);
    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/lib.rs"), ["form"], "{report}");
    assert!(lost_for(&report, "src/art.rs").is_empty(), "{report}");
}

#[test]
fn a_new_path_with_a_filter_is_a_filtered_review_item() {
    let tree = tree(CONFIG);
    tree.write("src/art.rs", CLEAN);
    tree.write(".gitattributes", "src/art.rs filter=lfs\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    let reviews = list(&report, "reviews");
    assert_eq!(reviews.len(), 1, "{report}");
    assert_eq!(reviews[0]["reason"], "filtered", "{report}");
}

#[test]
fn a_case_only_rename_is_not_a_lost_file() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", "mod caller;\npub use caller::one;\n");
    tree.write("src/Caller.rs", CLEAN);
    tree.base();
    assert!(std::fs::rename(tree.path("src/Caller.rs"), tree.path("src/caller.rs")).is_ok());

    for args in [&["--changed"][..], &[]] {
        let (code, report) = checked(&tree, args);
        assert_ne!(code, 2, "{args:?}: {report}");
        assert!(lost(&report).is_empty(), "{args:?}: {report}");
        assert!(list(&report, "reviews").is_empty(), "{args:?}: {report}");
        assert_eq!(report["not_measured"], 0, "{args:?}: {report}");
    }
}

const HELD: &str = r#"{"doc_size": {"README.md": 10},
  "accepted": [{"gate": "measurement-lost", "file": "src/lib.rs", "reason": "grammar lag"}]}"#;

#[test]
fn an_accepted_entry_holds_a_lost_file_for_every_capability() {
    let tree = tree(HELD);
    tree.write("src/lib.rs", BROKEN);

    for gates in [&[][..], &["complexity"], &["dead-symbols"]] {
        let (code, report) = checked(&tree, gates);
        assert_eq!(code, 0, "{gates:?}: {report}");
        let found = lost(&report);
        assert_eq!(found.len(), 1, "{gates:?}: {report}");
        assert_eq!(found[0]["outcome"], "held", "{gates:?}: {report}");
        assert_eq!(found[0]["matched"]["accepted"], true, "{gates:?}: {report}");
        assert_eq!(
            row(&report, "measurement-lost")["judgement"],
            "pass",
            "{report}"
        );
    }
    let stop = stop(&tree);
    assert_ne!(stop.code, 2, "{}", stop.out);
}

#[test]
fn an_accepted_entry_stays_matched_while_the_file_still_fails_to_parse() {
    let tree = tree(HELD);
    tree.write("src/lib.rs", BROKEN);
    tree.base();

    let (code, report) = checked(&tree, &[]);
    assert_eq!(code, 0, "{report}");
    assert!(list(&report, "reviews").is_empty(), "{report}");

    tree.write("src/lib.rs", CLEAN);
    let (code, report) = checked(&tree, &[]);
    assert_eq!(code, 0, "{report}");
    let reviews = list(&report, "reviews");
    assert_eq!(reviews.len(), 1, "{report}");
    assert_eq!(reviews[0]["kind"], "unmatched-accepted", "{report}");
    assert_eq!(reviews[0]["file"], "src/lib.rs", "{report}");
}

#[test]
fn measurement_lost_is_a_reserved_name() {
    let tree = tree(
        r#"{"doc_size": {"README.md": 10},
  "sarif": [{"name": "measurement-lost", "report": "out.sarif"}]}"#,
    );

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 2, "{report}");
    assert_eq!(report["errors"][0]["kind"], "configuration", "{report}");
    assert!(
        report["errors"][0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("measurement-lost")),
        "{report}"
    );
}

#[test]
fn a_stop_that_does_not_block_tells_the_person_how_to_hold_a_file_the_grammar_lags_on() {
    let tree = tree(CONFIG);
    tree.write("src/lib.rs", BROKEN);

    let run = harness::feed(
        tree.root(),
        &["gate", "--hook"],
        r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#,
    );

    assert_ne!(run.code, 2, "{}", run.out);
    assert!(run.printed.contains("systemMessage"), "{}", run.out);
    assert!(run.printed.contains("klin policy"), "{}", run.out);
}

#[test]
fn klin_policy_shows_how_a_person_holds_a_lost_file() {
    let tree = tree(CONFIG);

    let run = tree.run(&["policy"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("measurement-lost — built-in"), "{}", run.out);
    assert!(
        run.says(r#"{"gate": "measurement-lost", "file": PATH}"#),
        "{}",
        run.out
    );
}

#[test]
fn a_new_file_in_a_language_klin_does_not_read_is_counted_in_not_read() {
    let tree = tree(CONFIG);
    tree.write("src/tool.sh", "echo (\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert!(list(&report, "reviews").is_empty(), "{report}");
    assert_eq!(report["not_measured"], 0, "{report}");
    assert_eq!(
        row(&report, "complexity")["coverage"]["not_read"],
        1,
        "{report}"
    );
}

#[test]
fn a_new_file_a_structural_capability_does_not_read_is_counted_in_not_read() {
    let tree = tree(CONFIG);
    tree.write("src/tool.go", "package tool\n\nfunc Run() {}\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert!(list(&report, "reviews").is_empty(), "{report}");
    assert_eq!(
        row(&report, "dead-symbols")["coverage"]["not_read"],
        1,
        "{report}"
    );
    assert_eq!(
        row(&report, "complexity")["coverage"]["not_read"],
        0,
        "{report}"
    );
}

const LAYERS: &str = r#"{"doc_size": {"README.md": 10}, "layering": {"layers": {
  "domain": {"in": "src/domain", "can_use": []},
  "ui": {"in": "src/ui", "can_use": ["domain"]}}}}"#;

/// A crate whose library root attaches a domain layer and a ui layer, and a second root that
/// attaches only the ui layer, so a manifest that moves the library root narrows the scope.
fn two_roots() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", LAYERS);
    tree.words("README.md", 5);
    tree.write("Cargo.toml", CARGO);
    tree.write("src/lib.rs", "mod domain;\nmod ui;\n");
    tree.write("src/other.rs", "mod ui;\n");
    tree.write("src/ui/mod.rs", "pub fn show() {}\n");
    tree.write("src/domain/mod.rs", "pub fn rule() {}\n");
    tree.base();
    tree.write(
        "Cargo.toml",
        &format!("{CARGO}[lib]\npath = \"src/other.rs\"\n"),
    );
    tree
}

#[test]
fn a_clean_file_that_left_a_scope_a_manifest_narrowed_is_a_left_scope_review_item() {
    let tree = two_roots();

    let (code, report) = checked(&tree, &["layering"]);

    assert_eq!(code, 0, "{report}");
    let left: Vec<&Value> = list(&report, "reviews")
        .iter()
        .filter(|review| review["reason"] == "left-scope")
        .collect();
    assert!(
        left.iter()
            .any(|review| review["file"] == "src/domain/mod.rs"),
        "{report}"
    );
}

#[test]
fn a_file_that_left_a_scope_a_manifest_narrowed_fails_where_the_base_scope_finds_a_new_finding() {
    let tree = two_roots();
    tree.write(
        "src/domain/mod.rs",
        "pub fn rule() { crate::ui::show(); }\n",
    );

    let (code, report) = checked(&tree, &["layering"]);

    assert_eq!(code, 1, "{report}");
    assert!(
        list(&report, "findings")
            .iter()
            .any(|finding| finding["check"] == "layering" && finding["file"] == "src/domain/mod.rs"),
        "{report}"
    );
    assert!(
        !list(&report, "reviews")
            .iter()
            .any(|review| review["file"] == "src/domain/mod.rs"),
        "{report}"
    );
}

#[test]
fn a_file_that_left_a_scope_narrowed_in_klin_json_is_a_coverage_note() {
    let tree = tree(CONFIG);
    tree.write("src/other.rs", CLEAN);
    tree.base();
    tree.write(
        "klin.json",
        r#"{"doc_size": {"README.md": 10}, "complexity": {"except": "src/other.rs"}}"#,
    );

    let (code, report) = checked(&tree, &["complexity"]);

    assert_eq!(code, 0, "{report}");
    assert!(list(&report, "reviews").is_empty(), "{report}");
    let notes: Vec<&Value> = list(&report, "notes")
        .iter()
        .filter(|note| note["coverage"] == true)
        .collect();
    assert_eq!(notes.len(), 1, "{report}");
    assert_eq!(notes[0]["kind"], "left-scope", "{report}");
    assert_eq!(notes[0]["file"], "src/other.rs", "{report}");
}

#[test]
fn rename_detection_ignores_the_persons_rename_limit() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    let body: String = (0..30).map(|at| format!("pub fn f{at}() {{}}\n")).collect();
    tree.write("src/a.rs", &body);
    tree.write("src/b.rs", &body.replace("pub fn", "pub fn b_"));
    tree.base();
    tree.git(&["config", "diff.renameLimit", "1"]);
    tree.git(&["mv", "src/a.rs", "src/c.rs"]);
    tree.git(&["mv", "src/b.rs", "src/d.rs"]);
    tree.write("src/c.rs", &format!("{body}pub fn broken( {{\n"));
    tree.write(
        "src/d.rs",
        &format!(
            "{}pub fn extra() {{}}\n",
            body.replace("pub fn", "pub fn b_")
        ),
    );

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/c.rs"), ["parse"], "{report}");
}

#[test]
fn an_encoding_the_change_gives_a_measured_file_is_a_lost_form() {
    let tree = tree(CONFIG);
    tree.write(
        ".gitattributes",
        "src/lib.rs working-tree-encoding=SHIFT-JIS\n",
    );

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/lib.rs"), ["form"], "{report}");
}

#[test]
fn a_file_lost_to_its_attributes_makes_no_other_capability_fail() {
    let tree = tree(CONFIG);
    tree.write(".gitattributes", "src/lib.rs binary\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    let failing: Vec<&Value> = list(&report, "findings")
        .iter()
        .filter(|finding| finding["kind"] != "measurement-lost")
        .collect();
    assert!(failing.is_empty(), "{report}");
}

#[test]
fn a_plain_move_of_a_file_the_change_breaks_is_still_a_lost_file() {
    let body: String = (0..20).map(|at| format!("pub fn f{at}() {{}}\n")).collect();
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", &body);
    tree.base();
    assert!(std::fs::rename(tree.path("src/lib.rs"), tree.path("src/moved.rs")).is_ok());
    tree.write("src/moved.rs", &format!("{body}pub fn broken( {{\n"));

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/moved.rs"), ["parse"], "{report}");
}

#[test]
fn a_run_that_reads_no_held_file_reports_no_unmatched_entry() {
    let tree = tree(HELD);
    tree.write("src/lib.rs", BROKEN);

    let (code, report) = checked(&tree, &["doc-size"]);

    assert_eq!(code, 0, "{report}");
    assert!(list(&report, "reviews").is_empty(), "{report}");
}

#[test]
fn a_binary_attribute_the_change_gives_a_measured_file_blocks_the_stop() {
    let tree = tree(CONFIG);
    tree.write(".gitattributes", "src/lib.rs binary\n");

    let run = stop(&tree);

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("measurement-lost"), "{}", run.out);
}

fn left_scope(report: &Value, file: &str) -> Vec<String> {
    list(report, "reviews")
        .iter()
        .filter(|review| review["reason"] == "left-scope" && review["file"] == file)
        .filter_map(|review| review["kind"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn an_unrelated_klin_json_edit_does_not_quiet_a_manifest_that_narrowed_a_scope() {
    let tree = two_roots();
    tree.write(
        "klin.json",
        &LAYERS.replace("\"README.md\": 10", "\"README.md\": 11"),
    );

    let (code, report) = checked(&tree, &["layering"]);

    assert_eq!(code, 0, "{report}");
    assert_eq!(
        left_scope(&report, "src/domain/mod.rs"),
        ["unmeasured"],
        "{report}"
    );
}

#[test]
fn an_exclusion_in_one_gate_does_not_cover_what_a_manifest_took_from_another() {
    let tree = two_roots();
    tree.write(
        "klin.json",
        &LAYERS.replace(
            "\"layering\"",
            "\"complexity\": {\"except\": \"src/domain\"}, \"layering\"",
        ),
    );

    let (code, report) = checked(&tree, &["complexity", "layering"]);

    assert_eq!(code, 0, "{report}");
    assert_eq!(
        left_scope(&report, "src/domain/mod.rs"),
        ["unmeasured"],
        "{report}"
    );
}

#[test]
fn a_single_star_attribute_stays_inside_one_directory() {
    let tree = tree(CONFIG);
    tree.write("src/nested/module.rs", CLEAN);
    tree.base();
    tree.write(".gitattributes", "src/*.rs binary\n");

    let (_, report) = checked(&tree, &[]);

    assert_eq!(lost_for(&report, "src/lib.rs"), ["form"], "{report}");
    assert!(
        lost_for(&report, "src/nested/module.rs").is_empty(),
        "{report}"
    );
}

#[test]
fn a_double_star_attribute_reaches_nested_directories() {
    let tree = tree(CONFIG);
    tree.write("src/nested/module.rs", CLEAN);
    tree.base();
    tree.write(".gitattributes", "src/**/*.rs binary\n");

    let (_, report) = checked(&tree, &[]);

    assert_eq!(
        lost_for(&report, "src/nested/module.rs"),
        ["form"],
        "{report}"
    );
}

#[test]
fn a_quoted_attribute_pattern_names_a_path_with_a_space() {
    let tree = tree(CONFIG);
    tree.write("src/my file.rs", CLEAN);
    tree.base();
    tree.write(".gitattributes", "\"src/my file.rs\" binary\n");

    let (_, report) = checked(&tree, &[]);

    assert_eq!(lost_for(&report, "src/my file.rs"), ["form"], "{report}");
}

#[test]
fn a_deeper_attributes_file_overrides_a_shallower_one() {
    let tree = tree(CONFIG);
    tree.write(".gitattributes", "*.rs binary\n");
    tree.write("src/.gitattributes", "*.rs diff\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert!(lost(&report).is_empty(), "{report}");
}

#[test]
fn an_attribute_on_a_file_outside_every_scope_makes_no_lost_file() {
    let tree = tree(r#"{"doc_size": {"README.md": 10}, "complexity": {"in": "src"}}"#);
    tree.write("docs/x.rs", CLEAN);
    tree.base();
    tree.write(".gitattributes", "docs/x.rs binary\n");

    let (code, report) = checked(&tree, &["complexity"]);

    assert_eq!(code, 0, "{report}");
    assert!(lost(&report).is_empty(), "{report}");
}

#[test]
fn a_filter_added_to_a_file_the_base_could_not_parse_stays_a_coverage_note() {
    let tree = tree(CONFIG);
    tree.write("src/bad.rs", BROKEN);
    tree.base();
    tree.write(".gitattributes", "src/bad.rs filter=lfs\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert!(lost(&report).is_empty(), "{report}");
}

#[test]
fn an_attribute_cannot_hide_a_file_from_a_convention() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"conventions": {"no-forbidden": {"code": "forbidden()", "in": "src", "remedy": "x"}}}"#,
    );
    tree.write("src/lib.rs", "pub fn one() {}\n");
    tree.write("src/two.rs", "pub fn two() {}\n");
    tree.base();
    tree.write(".gitattributes", "src/two.rs binary\n");
    tree.write("src/two.rs", "pub fn two() { forbidden() }\n");

    let (code, report) = checked(&tree, &["conventions"]);

    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/two.rs"), ["form"], "{report}");
}

#[cfg(unix)]
#[test]
fn finding_changes_runs_no_filter_program() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    let body: String = (0..20).map(|at| format!("pub fn f{at}() {{}}\n")).collect();
    tree.write("src/lib.rs", &body);
    tree.write(".gitattributes", "*.txt filter=spy\n");
    tree.write("notes.txt", "one\n");
    tree.base();
    let ran = tree.path("filter-ran");
    tree.git(&[
        "config",
        "filter.spy.clean",
        &format!("sh -c 'touch {}; cat'", ran.display()),
    ]);
    tree.write("notes.txt", "two\n");
    tree.write("other.txt", "new\n");
    assert!(std::fs::rename(tree.path("src/lib.rs"), tree.path("src/moved.rs")).is_ok());
    tree.write("src/moved.rs", &format!("{body}pub fn broken( {{\n"));

    let (code, report) = checked(&tree, &[]);

    assert!(!ran.exists(), "a filter ran: {report}");
    assert_eq!(code, 1, "{report}");
    assert_eq!(lost_for(&report, "src/moved.rs"), ["parse"], "{report}");
}

#[test]
fn a_bracket_alone_in_an_attribute_pattern_is_a_literal() {
    let tree = tree(CONFIG);
    tree.write(".gitattributes", "[ binary\n[! binary\nsrc/[ binary\n");

    let (code, report) = checked(&tree, &[]);

    assert_eq!(code, 0, "{report}");
    assert!(lost(&report).is_empty(), "{report}");
}

#[test]
fn a_pattern_of_many_stars_is_decided_quickly() {
    let tree = tree(CONFIG);
    let stars = "*a".repeat(60);
    tree.write(".gitattributes", &format!("src/{stars}b binary\n"));

    let started = std::time::Instant::now();
    let (code, report) = checked(&tree, &[]);

    assert!(started.elapsed().as_secs() < 30, "{report}");
    assert_eq!(code, 0, "{report}");
}

#[cfg(unix)]
#[test]
fn a_filter_driver_named_with_an_equals_sign_does_not_run() {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.write(".gitattributes", "*.txt filter=a=b\n");
    tree.write("notes.txt", "one\n");
    tree.base();
    let ran = tree.path("filter-ran");
    tree.git(&[
        "config",
        "filter.a=b.clean",
        &format!("sh -c 'touch {}; cat'", ran.display()),
    ]);
    tree.write("notes.txt", "two\n");

    let (_, report) = checked(&tree, &[]);

    assert!(!ran.exists(), "a filter ran: {report}");
}
