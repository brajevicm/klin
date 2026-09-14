mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const EVERY_GATE: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "doc_citations": [{"file": "README.md", "roots": ["."]}],
  "escapes": { "in": "src" },
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

/// A config whose escapes scope holds no applicable file, so that gate errors.
const A_BROKEN_GATE: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "doc_citations": false,
  "escapes": { "in": "missing" },
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

/// A config holding one accepted escape that no site in the tree matches.
const AN_UNMATCHED_ACCEPTED: &str = r#"{
  "project": "t",
  "accepted": [{"gate": "escapes", "file": "src/gone.rs", "text": "the line that held it",
                "count": 1}],
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "doc_citations": [{"file": "README.md", "roots": ["."]}],
  "escapes": { "in": "src" },
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree.write("src/work.rs", CLEAN);
    tree
}

/// A repository the survey finds nothing in: no source, no document and no manifest, so no
/// gate is derivable and the config alone says what runs.
fn nothing_to_survey(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree
}

/// A repository with a document and no source, so the survey supplies the document gates and
/// cannot supply escapes or complexity.
fn without_source(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.base();
    tree
}

fn at(run: &harness::Run, text: &str) -> usize {
    run.out
        .find(text)
        .unwrap_or_else(|| panic!("{text:?} is absent from:\n{}", run.out))
}

#[test]
fn every_configured_gate_runs_in_ladder_order() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        at(&run, "doc-size") < at(&run, "doc-citations"),
        "{}",
        run.out
    );
    assert!(
        at(&run, "doc-citations") < at(&run, "escapes"),
        "{}",
        run.out
    );
    assert!(at(&run, "escapes") < at(&run, "complexity"), "{}", run.out);
}

#[test]
fn a_gate_the_config_does_not_name_runs_over_the_section_the_survey_derives() {
    let tree = tree(r#"{ "project": "t", "doc_size": [{"file": "README.md", "ceiling": 10}] }"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    stubs"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(!run.says("derived: escapes"), "{}", run.out);
    assert!(run.says("7 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_gate_the_survey_cannot_supply_does_not_run() {
    let tree =
        without_source(r#"{ "project": "t", "doc_size": [{"file": "README.md", "ceiling": 10}] }"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(!run.says("complexity"), "{}", run.out);
    assert!(run.says("2 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_status_row_per_gate_and_a_summary_line() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(run.says("ok    doc-citations"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(run.says("7 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_passing_gate_prints_a_row_and_the_one_ok_line_under_it() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(
        run.says("OK: 0 escape site(s) in the tree, all held at the base"),
        "{}",
        run.out
    );
    assert!(!run.says("FAIL"), "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn a_failing_gate_prints_its_full_output_under_its_row() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(
        run.says("README.md is 30 words, over its ceiling of 10"),
        "{}",
        run.out
    );
    assert!(run.says("7 gate(s), 1 failed."), "{}", run.out);
}

#[test]
fn every_gate_runs_even_when_an_earlier_one_failed() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    let source = format!("pub fn f() {{\n    x.{};\n}}\n", "unwrap()");
    tree.write("src/lib.rs", &source);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("FAIL  escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(run.says("7 gate(s), 2 failed."), "{}", run.out);
}

#[test]
fn a_tool_error_is_distinguishable_from_a_gate_failure() {
    let tree = tree(A_BROKEN_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("6 gate(s), 1 excluded, 1 failed, 1 tool error."),
        "{}",
        run.out
    );
}

#[test]
fn a_tool_error_alone_exits_two() {
    let tree = tree(A_BROKEN_GATE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("6 gate(s), 1 excluded, 1 tool error."),
        "{}",
        run.out
    );
}

/// The one row per gate --list prints, without the per-key lines under each and without the
/// state directory line that follows them all.
fn rows(run: &harness::Run) -> String {
    run.out
        .lines()
        .filter(|line| !line.starts_with("state: ") && !line.starts_with(' '))
        .map(|line| line.to_string() + "\n")
        .collect()
}

#[test]
fn list_prints_the_configured_gates_and_runs_none_of_them() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — runs\ndoc-citations — runs\nescapes — runs\nstubs — runs\n\
         complexity — runs\ndead-symbols — runs\nreachability — runs\n\
         lockfile — needs a section a person writes\n\
         inventory — needs a section a person writes\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n",
        "{:?}",
        run.out
    );
    assert!(!run.says("ok    doc-size"), "{}", run.out);
}

#[test]
fn list_does_not_derive_source_policy() {
    let tree = tree(
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "complexity": { "in": "src", "cc": 8 } }"#,
    );

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("pinned: doc_size README.md"), "{}", run.out);
    assert!(run.says("complexity — runs"), "{}", run.out);
    assert!(run.says("escapes — runs"), "{}", run.out);
    assert!(!run.says("derived: complexity"), "{}", run.out);
    assert!(!run.says("derived: escapes"), "{}", run.out);
}

#[test]
fn list_puts_the_excluded_gates_before_the_ones_that_need_a_section() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — runs\ndoc-citations — runs\ncomplexity — excluded\n\
         lockfile — needs a section a person writes\n\
         escapes — needs a section a person writes\n\
         stubs — needs a section a person writes\n\
         inventory — needs a section a person writes\n\
         dead-symbols — needs a section a person writes\n\
         reachability — needs a section a person writes\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n",
        "{:?}",
        run.out
    );
}

#[test]
fn list_ends_with_the_state_directory() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let last = run.out.trim_end().lines().last().unwrap_or_default();
    assert!(last.starts_with("state: "), "{}", run.out);
    assert!(last.contains("klin"), "{}", run.out);
}

#[test]
fn gate_by_name_runs_only_that_gate() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate", "--gate", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
    assert!(run.says("1 gate(s), all passed."), "{}", run.out);
}

#[test]
fn gate_by_name_is_repeatable_and_keeps_ladder_order() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--gate", "complexity", "--gate", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(at(&run, "doc-size") < at(&run, "complexity"), "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(run.says("2 gate(s), all passed."), "{}", run.out);
}

#[test]
fn a_gate_name_the_config_does_not_configure_is_a_tool_error() {
    let tree =
        without_source(r#"{ "project": "t", "doc_size": [{"file": "README.md", "ceiling": 10}] }"#);

    let run = tree.run(&["gate", "--gate", "escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no gate named escapes"), "{}", run.out);
    assert!(run.says("doc-size"), "{}", run.out);
}

#[test]
fn a_config_that_configures_no_gate_is_a_tool_error() {
    let tree = nothing_to_survey(r#"{ "project": "t" }"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("configures no gate"), "{}", run.out);
}

#[test]
fn a_section_named_after_the_command_is_a_tool_error() {
    let tree = tree(r#"{ "project": "t", "doc-size": [{"file": "README.md", "ceiling": 1}] }"#);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"doc-size\" is what the command"), "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
}

#[test]
fn list_says_no_gate_is_configured_rather_than_printing_nothing() {
    let tree = nothing_to_survey(r#"{ "project": "t" }"#);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("configures no gate"), "{}", run.out);
}

#[test]
fn strict_reaches_the_gates_that_take_it() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);

    let loose = tree.run(&["gate"]);
    assert_eq!(loose.code, 0, "{}", loose.out);
    assert!(loose.says("ok    escapes"), "{}", loose.out);
    assert!(loose.says("matched nothing this run"), "{}", loose.out);

    let strict = tree.run(&["gate", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
    assert!(strict.says("FAIL  escapes"), "{}", strict.out);
}

#[test]
fn the_config_flag_names_the_klin_json_every_gate_runs_under() {
    let tree = tree(EVERY_GATE);
    tree.write("elsewhere/klin.json", EVERY_GATE);
    tree.words("elsewhere/README.md", 30);
    tree.write("elsewhere/src/lib.rs", CLEAN);

    let run = tree.run(&["gate", "--config", &tree.at("elsewhere/klin.json")]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
}

const AN_ESCAPE: &str = include_str!("fixtures/an_escape.rs");

fn tangled(name: &str) -> String {
    let arms: String = (0..12)
        .map(|step| format!("        {step} => n + {step},\n"))
        .collect();
    format!("pub fn {name}(n: i32) -> i32 {{\n    match n {{\n{arms}        _ => n,\n    }}\n}}\n")
}

fn based(config: &str, files: &[(&str, &str)]) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    for (name, text) in files {
        tree.write(name, text);
    }
    tree.base();
    tree
}

#[test]
fn changed_scopes_the_scoped_gates_to_the_working_tree_and_untracked_files() {
    let tree = based(EVERY_GATE, &[("src/old.rs", AN_ESCAPE)]);
    tree.write("src/new.rs", AN_ESCAPE);

    let scoped = tree.run(&["gate", "--changed"]);
    assert_eq!(scoped.code, 1, "{}", scoped.out);
    assert!(scoped.says("changed: 1 file(s)"), "{}", scoped.out);
    assert!(scoped.says("src/new.rs:2"), "{}", scoped.out);
    assert!(!scoped.says("src/old.rs"), "{}", scoped.out);

    let whole = tree.run(&["gate"]);
    assert_eq!(whole.code, 1, "{}", whole.out);
    assert!(whole.says("src/new.rs:2"), "{}", whole.out);
    assert!(!whole.says("src/old.rs"), "{}", whole.out);
}

#[test]
fn a_gate_that_is_not_scoped_still_runs_over_everything() {
    let tree = based(EVERY_GATE, &[]);
    tree.words("README.md", 30);
    tree.write("src/new.rs", CLEAN);

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("README.md is 30 words"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn an_accepted_entry_for_a_file_outside_the_changed_set_is_not_judged() {
    let tree = based(AN_UNMATCHED_ACCEPTED, &[]);
    tree.write("src/new.rs", CLEAN);

    let scoped = tree.run(&["gate", "--changed", "--strict"]);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
    assert!(!scoped.says("matched nothing this run"), "{}", scoped.out);

    let whole = tree.run(&["gate"]);
    assert_eq!(whole.code, 0, "{}", whole.out);
    assert!(whole.says("matched nothing this run"), "{}", whole.out);
}

#[test]
fn changed_diffs_against_the_pull_request_base_when_ci_names_one() {
    let tree = based(EVERY_GATE, &[]);
    tree.write("src/new.rs", AN_ESCAPE);
    tree.commit("work on the branch");
    tree.git(&["update-ref", "refs/remotes/origin/release", "HEAD"]);
    tree.write("src/newer.rs", AN_ESCAPE);

    let in_ci = tree.run_with(&[("GITHUB_BASE_REF", "release")], &["gate", "--changed"]);
    assert_eq!(in_ci.code, 1, "{}", in_ci.out);
    assert!(in_ci.says("src/newer.rs:2"), "{}", in_ci.out);
    assert!(!in_ci.says("src/new.rs:2"), "{}", in_ci.out);

    let locally = tree.run(&["gate", "--changed"]);
    assert_eq!(locally.code, 1, "{}", locally.out);
    assert!(locally.says("src/new.rs:2"), "{}", locally.out);
}

#[test]
fn a_run_outside_a_repository_is_a_tool_error_rather_than_an_empty_pass() {
    let tree = Tree::bare();
    tree.write("klin.json", EVERY_GATE);
    tree.words("README.md", 5);
    tree.write("src/new.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--changed"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no base commit"), "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn changed_restricts_complexity_as_well_as_escapes() {
    let tree = based(EVERY_GATE, &[("src/old.rs", &tangled("was_here"))]);
    tree.write("src/new.rs", &tangled("is_new"));

    let scoped = tree.run(&["gate", "--changed"]);
    assert_eq!(scoped.code, 1, "{}", scoped.out);
    assert!(scoped.says("FAIL  complexity"), "{}", scoped.out);
    assert!(scoped.says("src/new.rs:1"), "{}", scoped.out);
    assert!(!scoped.says("src/old.rs"), "{}", scoped.out);
}

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;

fn stop(tree: &Tree, event: &str) -> harness::Run {
    harness::feed(tree.root(), &["gate", "--hook"], event)
}

#[test]
fn hook_blocks_the_first_stop_and_hands_the_failures_back() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("a quality gate failed"), "{}", run.out);
    assert!(
        run.says("fix what each names, then stop again"),
        "{}",
        run.out
    );
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(
        run.says("README.md is 30 words, over its ceiling of 10"),
        "{}",
        run.out
    );
}

#[test]
fn hook_does_not_block_the_stop_after_that() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_SECOND_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("still, after one round of fixes"), "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("not blocking a second time"), "{}", run.out);
    assert!(!run.says("then stop again"), "{}", run.out);
    assert!(!run.says("CI will refuse"), "{}", run.out);
}

#[test]
fn hook_says_nothing_when_every_gate_passes() {
    let tree = tree(EVERY_GATE);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn hook_without_an_event_on_stdin_reports_but_does_not_block() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    for event in ["", "not json"] {
        let run = stop(&tree, event);
        assert_eq!(run.code, 1, "{event:?}: {}", run.out);
        assert!(run.says("FAIL  doc-size"), "{event:?}: {}", run.out);
        assert!(!run.says("stop again"), "{event:?}: {}", run.out);
    }
}

#[test]
fn hook_blocks_on_a_tool_error_too() {
    let tree = tree(A_BROKEN_GATE);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(run.says("could not run a quality gate"), "{}", run.out);
    assert!(!run.says("a quality gate failed"), "{}", run.out);
    assert!(
        run.says("fix what each names, then stop again"),
        "{}",
        run.out
    );
}

#[test]
fn hook_names_both_when_a_gate_failed_and_another_could_not_run() {
    let tree = tree(A_BROKEN_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("a quality gate failed, and another could not run"),
        "{}",
        run.out
    );
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("6 gate(s), 1 excluded, 1 failed, 1 tool error."),
        "{}",
        run.out
    );
}

#[test]
fn hook_says_a_gate_could_not_run_after_a_second_stop_too() {
    let tree = tree(A_BROKEN_GATE);

    let run = stop(&tree, A_SECOND_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("could not run a quality gate"), "{}", run.out);
    assert!(run.says("still, after one round of fixes"), "{}", run.out);
}

#[test]
fn hook_without_an_event_reports_a_tool_error_without_blocking_the_stop() {
    let tree = nothing_to_survey(r#"{ "project": "t" }"#);

    let run = stop(&tree, "");
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("configures no gate"), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn hook_says_nothing_in_a_tree_that_holds_no_config() {
    let tree = Tree::new();

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
    assert!(!tree.path(".git/klin").exists(), "wrote state");
}

#[test]
fn gate_by_hand_in_a_tree_that_holds_no_config_still_names_what_is_missing() {
    let tree = Tree::new();

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("klin.json does not exist"), "{}", run.out);
    assert!(run.says("nothing to gate"), "{}", run.out);
}

const BUILD_BLOCKED: &str = ".git/klin/build-blocked";
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;

fn settings() -> String {
    let at = concat!(env!("CARGO_MANIFEST_DIR"), "/.claude/settings.json");
    std::fs::read_to_string(at).unwrap_or_default()
}

#[test]
fn the_stop_hook_is_one_line_that_runs_the_binary() {
    let settings = settings();
    assert!(settings.contains("gate --hook --changed"), "{settings}");
    for wrapper in [BUILD_BLOCKED, "stop_hook_active", "cargo build"] {
        assert!(!settings.contains(wrapper), "{wrapper}: {settings}");
    }
}

#[test]
fn the_stamp_sits_beside_the_config_rather_than_the_working_directory() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = harness::feed(&tree.path("src"), &["gate", "--hook"], A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(tree.path(BUILD_BLOCKED).is_file(), "{}", run.out);
}

#[test]
fn the_gate_blocks_once_under_each_prompt_and_reports_on_the_stop_after() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    assert!(
        first.says("fix what each names, then stop again"),
        "{}",
        first.out
    );

    let again = stop(&tree, A_SECOND_STOP);
    assert_eq!(again.code, 0, "{}", again.out);
    assert!(again.says("not blocking a second time"), "{}", again.out);

    let prompt = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(prompt.code, 0, "{}", prompt.out);
    let after = stop(&tree, A_STOP);
    assert_eq!(after.code, 2, "{}", after.out);
    assert!(
        after.says("fix what each names, then stop again"),
        "{}",
        after.out
    );
}

#[test]
fn the_ladder_writes_nothing() {
    let tree = tree(EVERY_GATE);
    tree.write("src/lib.rs", AN_ESCAPE);
    tree.words("README.md", 30);

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let dirty = tree.status();
    assert_eq!(
        dirty, " M README.md\n M src/lib.rs\n?? src/work.rs\n",
        "the run touched the tree: {dirty}"
    );
}

fn object(text: &str, run: &harness::Run) -> Value {
    match serde_json::from_str(text) {
        Ok(report) => report,
        Err(why) => panic!("{why} — the run printed:\n{}", run.out),
    }
}

fn field<'a>(record: &'a Value, key: &str) -> &'a str {
    record.get(key).and_then(Value::as_str).unwrap_or("")
}

fn list<'a>(report: &'a Value, key: &str) -> &'a [Value] {
    match report.get(key).and_then(Value::as_array) {
        Some(records) => records.as_slice(),
        None => panic!("no {key} array in {report}"),
    }
}

fn outcomes(records: &[Value]) -> Vec<(&str, &str)> {
    records
        .iter()
        .map(|record| (field(record, "gate"), field(record, "outcome")))
        .collect()
}

#[test]
fn json_prints_one_object_holding_every_failing_finding() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write("src/lib.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "status"), "FAIL", "{}", run.out);
    assert_eq!(report.get("exit"), Some(&Value::from(1)), "{}", run.out);
    assert!(
        field(&report, "summary").contains("2 failed"),
        "{}",
        run.out
    );
    let findings = list(&report, "findings");
    assert_eq!(
        outcomes(findings),
        [("doc-size", "new"), ("escapes", "new")],
        "{}",
        run.out
    );
}

#[test]
fn a_json_finding_carries_the_site_the_values_and_the_advice() {
    let tree = tree(EVERY_GATE);
    tree.write("src/lib.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    let finding = &list(&report, "findings")[0];
    assert_eq!(field(finding, "file"), "src/lib.rs", "{}", run.out);
    assert_eq!(finding.get("line"), Some(&Value::from(2)), "{}", run.out);
    assert!(field(finding, "text").contains("unwrap"), "{}", run.out);
    assert_eq!(
        finding
            .get("values")
            .and_then(|values| values.get("escape")),
        Some(&Value::from("unwrap")),
        "{}",
        run.out
    );
    assert!(
        field(finding, "condition").contains("opts out"),
        "{}",
        run.out
    );
    assert!(
        field(finding, "fix_advice").contains("escape"),
        "{}",
        run.out
    );
}

#[test]
fn a_json_record_names_no_column_and_no_violation() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);
    tree.words("README.md", 30);
    tree.write("src/lib.rs", AN_ESCAPE);

    let run = tree.run(&["gate", "--json", "--strict"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    let banned = ["column", "violation", "invariant"];
    assert!(!list(&report, "findings").is_empty(), "{}", run.out);
    assert!(!list(&report, "notes").is_empty(), "{}", run.out);
    for record in list(&report, "findings")
        .iter()
        .chain(list(&report, "notes"))
    {
        for key in banned {
            assert_eq!(record.get(key), None, "{key} is in {record}");
        }
    }
}

#[test]
fn json_notes_say_why_a_strict_run_failed_with_nothing_over_the_gate() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);

    let run = tree.run(&["gate", "--json", "--strict"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "status"), "FAIL", "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
    assert_eq!(
        outcomes(list(&report, "notes")),
        [("escapes", "unmatched")],
        "{}",
        run.out
    );
}

#[test]
fn a_gate_that_could_not_run_is_a_json_finding_too() {
    let tree = tree(A_BROKEN_GATE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "status"), "ERROR", "{}", run.out);
    let finding = &list(&report, "findings")[0];
    assert_eq!(field(finding, "gate"), "escapes", "{}", run.out);
    assert_eq!(field(finding, "outcome"), "error", "{}", run.out);
    assert!(
        field(finding, "text").contains("no applicable file"),
        "{}",
        run.out
    );
}

#[test]
fn a_passing_json_run_holds_no_findings() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "status"), "PASS", "{}", run.out);
    assert_eq!(report.get("exit"), Some(&Value::from(0)), "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
    assert!(list(&report, "notes").is_empty(), "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_json_run_with_pinned_source_policy_invents_no_derived_entries() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let derived = list(&report, "derived");
    assert!(derived.is_empty(), "{}", run.out);
}

#[test]
fn a_config_the_run_cannot_read_is_a_json_object_too() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["gate", "--json", "--config", "absent.json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "status"), "ERROR", "{}", run.out);
    assert_eq!(report.get("exit"), Some(&Value::from(2)), "{}", run.out);
    let finding = &list(&report, "findings")[0];
    assert_eq!(field(finding, "outcome"), "error", "{}", run.out);
    assert!(
        field(finding, "text").contains("could not be read"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_the_grammar_rejected_is_a_json_finding_at_its_own_file() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.rs", "fn ( { ) unbalanced");

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert_eq!(
        outcomes(list(&report, "findings")),
        [("complexity", "unparsed"), ("dead-symbols", "unparsed")],
        "{}",
        run.out
    );
    assert_eq!(
        field(&list(&report, "findings")[0], "file"),
        "src/broken.rs",
        "{}",
        run.out
    );
}

const WORKFLOW: &str = include_str!("../.github/workflows/quality.yml");

fn ci_arguments() -> Vec<&'static str> {
    WORKFLOW
        .lines()
        .find(|line| line.contains("klin gate"))
        .unwrap_or_else(|| panic!("no klin gate line in:\n{WORKFLOW}"))
        .split_whitespace()
        .skip_while(|word| *word != "gate")
        .collect()
}

#[test]
fn deleting_a_section_leaves_the_gate_running_over_a_derived_section() {
    let tree = tree(EVERY_GATE);

    let whole = tree.run(&ci_arguments());
    assert_eq!(whole.code, 0, "{}", whole.out);

    tree.write(
        "klin.json",
        r#"{ "project": "t",
             "doc_size": [{"file": "README.md", "ceiling": 10}],
             "complexity": { "in": "src", "cc": 8, "lines": 60 } }"#,
    );
    let deleted = tree.run(&ci_arguments());
    assert_eq!(deleted.code, 0, "{}", deleted.out);
    assert!(!deleted.says("derived: escapes"), "{}", deleted.out);
}

const AN_EXCLUDED_GATE: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "doc_citations": [{"file": "README.md", "roots": ["."]}],
  "escapes": false,
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

const NOTHING_SAID_ABOUT_ESCAPES: &str = r#"{
  "project": "t",
  "doc_size": [{"file": "README.md", "ceiling": 10}],
  "doc_citations": [{"file": "README.md", "roots": ["."]}],
  "complexity": false
}"#;

#[test]
fn a_named_gate_runs_alone_when_the_command_line_names_it() {
    let tree = tree(AN_EXCLUDED_GATE);
    tree.write("src/big.rs", &tangled("big"));

    let run = tree.run(&["gate", "--gate", "complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  complexity"), "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
    assert!(run.says("1 gate(s), 1 excluded, 1 failed."), "{}", run.out);
}

/// The top-level `gates` list is gone: a gate's multiplicity belongs to its own section, as the
/// named entries of `sarif` and the named conventions have it. ADR 0038.
#[test]
fn a_gates_key_is_not_one_klin_reads() {
    let tree = tree(
        r#"{ "project": "t",
              "gates": [{"name": "n", "check": "complexity", "with": {}}] }"#,
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"gates\" is not a key klin reads"), "{}", run.out);
}

#[test]
fn two_gates_of_one_name_are_a_tool_error() {
    let tree = tree(
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "sarif": [{"name": "doc-size", "report": "out/lint.sarif"}] }"#,
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("two gates are named doc-size"), "{}", run.out);
}

#[test]
fn a_section_set_to_false_excludes_its_gate_and_the_summary_counts_it() {
    let tree = tree(AN_EXCLUDED_GATE);
    tree.write("src/risky.rs", AN_ESCAPE);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(
        run.says("6 gate(s), 1 excluded, all passed."),
        "{}",
        run.out
    );
}

#[test]
fn list_names_the_excluded_gates() {
    let tree = tree(AN_EXCLUDED_GATE);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — runs\ndoc-citations — runs\nstubs — runs\ncomplexity — runs\n\
         dead-symbols — runs\nreachability — runs\n\
         escapes — excluded\nlockfile — needs a section a person writes\n\
         inventory — needs a section a person writes\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n",
        "{:?}",
        run.out
    );
}

#[test]
fn list_names_an_available_gate_the_survey_cannot_supply_either() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("escapes — needs a section a person writes"),
        "{}",
        run.out
    );
}

#[test]
fn list_names_a_gate_the_survey_supplies_as_one_that_runs() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(rows(&run).contains("escapes — runs\n"), "{}", run.out);
    assert!(
        !rows(&run).contains("escapes — needs a section"),
        "{}",
        run.out
    );
}

#[test]
fn naming_an_excluded_gate_is_a_tool_error() {
    let tree = tree(AN_EXCLUDED_GATE);

    let run = tree.run(&["gate", "--gate", "escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("named escapes is excluded"), "{}", run.out);
}

#[test]
fn strict_accounts_for_a_gate_the_survey_supplies() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let strict = tree.run(&["gate", "--strict"]);
    assert_eq!(strict.code, 0, "{}", strict.out);
    assert!(strict.says("ok    escapes"), "{}", strict.out);
}

/// The retired `--strict` failure of ADR 0010: a config that leaves a derivable gate out is a
/// config klin derives that section for, so there is nothing left to account for. The hole it
/// closed is closed by the source root failure below instead. ADR 0016.
#[test]
fn strict_accepts_a_config_that_omits_a_derivable_gate() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let strict = tree.run(&["gate", "--strict"]);
    assert_eq!(strict.code, 0, "{}", strict.out);
    assert!(strict.says("ok    escapes"), "{}", strict.out);
    assert!(!strict.says("unaccounted"), "{}", strict.out);
}

#[test]
fn strict_refuses_a_tree_the_survey_finds_no_source_root_in() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let strict = tree.run(&["gate", "--strict"]);
    assert_eq!(strict.code, 2, "{}", strict.out);
    let named = tree
        .root()
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    assert!(strict.says("found no source root"), "{}", strict.out);
    assert!(strict.says(&named), "{}", strict.out);
    assert!(
        strict.says("a directory that holds nothing but source files"),
        "{}",
        strict.out
    );
}

/// The failure is about a gate the survey was left to supply, so a tree that reads no code
/// because a person excluded every gate that does is not this hole. ADR 0016.
#[test]
fn strict_accepts_a_tree_with_no_source_when_every_code_gate_is_excluded() {
    let tree = without_source(
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "doc_citations": [{"file": "README.md", "roots": ["."]}],
              "complexity": false,
              "escapes": false,
              "stubs": false,
              "dead_symbols": false,
              "reachability": false }"#,
    );

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("no source root"), "{}", run.out);
}

#[test]
fn list_says_a_compact_source_policy_runs() {
    let tree = tree(
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "escapes": {"in": "src"} }"#,
    );

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("escapes — runs"), "{}", run.out);
    assert!(!run.says("derived: escapes"), "{}", run.out);
}

/// A run that names its gates derives the values those gates read and no other's. The
/// document ceilings are the case: a `--gate escapes` run does not read the words of a
/// document only the working tree holds, and a run of every gate still does. ADR 0038.
#[test]
fn a_named_gate_derives_nothing_another_gate_would_need() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree.words("README.md", 5);

    let every = tree.run(&["gate"]);
    assert_eq!(every.code, 0, "{}", every.out);
    assert!(
        every.says("NOTE: doc_size README.md is 5 words and is not judged"),
        "{}",
        every.out
    );

    let one = tree.run(&["gate", "--gate", "escapes"]);
    assert_eq!(one.code, 0, "{}", one.out);
    assert!(!one.says("derived: escapes"), "{}", one.out);
    assert!(!one.says("doc_size"), "{}", one.out);
    assert!(!one.says("derived: complexity"), "{}", one.out);
}

#[test]
fn no_source_root_without_strict_is_a_note_and_the_gates_still_run() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE: the survey of"), "{}", run.out);
    assert!(run.says("found no source root"), "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
}

#[test]
fn no_source_root_in_the_hook_lets_the_turn_end() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_source_root_leaves_no_note() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("found no source root"), "{}", run.out);
}

#[test]
fn strict_passes_once_every_derivable_gate_is_set_to_false() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);
    tree.write(
        "klin.json",
        r#"{ "project": "t",
              "doc_size": [{"file": "README.md", "ceiling": 10}],
              "doc_citations": false,
              "complexity": false,
              "escapes": false,
              "stubs": false,
              "dead_symbols": false,
              "reachability": false }"#,
    );

    let run = tree.run(&["gate", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 gate(s), 6 excluded, all passed."),
        "{}",
        run.out
    );
}

#[test]
fn list_names_the_exclusions_when_every_gate_is_excluded() {
    let tree = tree(
        r#"{ "project": "t", "doc_size": false, "doc_citations": false, "escapes": false,
              "stubs": false, "complexity": false, "dead_symbols": false,
              "reachability": false }"#,
    );

    let run = tree.run(&["gate", "--list"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — excluded\ndoc-citations — excluded\nescapes — excluded\n\
         stubs — excluded\ncomplexity — excluded\n\
         dead-symbols — excluded\nreachability — excluded\n\
         lockfile — needs a section a person writes\n\
         inventory — needs a section a person writes\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n",
        "{:?}",
        run.out
    );

    let judged = tree.run(&["gate"]);
    assert_eq!(judged.code, 2, "{}", judged.out);
    assert!(
        judged.says(
            "excludes every gate it names: doc-size, doc-citations, escapes, stubs, complexity, \
             dead-symbols"
        ),
        "{}",
        judged.out
    );
}

#[test]
fn hook_notes_a_file_no_grammar_reads_and_does_not_block_the_stop() {
    let tree = tree(EVERY_GATE);
    tree.write("src/flow.rs", "%%% not rust %%%\n");

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(r#"{"systemMessage":"#), "{}", run.out);
    assert!(run.says("NOTE:"), "{}", run.out);
    assert!(run.says("src/flow.rs"), "{}", run.out);
    assert!(run.says("the Rust grammar rejected it"), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn hook_still_blocks_on_a_gate_failure_beside_the_note() {
    let tree = tree(EVERY_GATE);
    tree.write("src/flow.rs", "%%% not rust %%%\n");
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("a quality gate failed"), "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("src/flow.rs"), "{}", run.out);
    assert!(run.says("the Rust grammar rejected it"), "{}", run.out);
}

#[test]
fn hook_says_nothing_for_a_note_no_grammar_hole_left() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

#[test]
fn a_file_the_grammar_rejected_is_a_json_note_in_the_hook() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.rs", "fn ( { ) unbalanced");

    let run = harness::feed(tree.root(), &["gate", "--hook", "--json"], A_STOP);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = object(run.out.lines().last().unwrap_or_default(), &run);
    assert_eq!(
        outcomes(list(&report, "notes")),
        [("complexity", "unparsed"), ("dead-symbols", "unparsed")],
        "{}",
        run.out
    );
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
}

const ANOTHER_VERSION: &str = r#"{
  "project": "t",
  "version": "0.0.1",
  "doc_size": [{"file": "README.md", "ceiling": 10}]
}"#;

#[test]
fn a_version_the_binary_does_not_carry_is_a_note_and_nothing_else() {
    let tree = tree(ANOTHER_VERSION);

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("0.0.1"), "{}", run.out);
    assert!(run.says(env!("CARGO_PKG_VERSION")), "{}", run.out);
    assert!(run.says("7 gate(s), all passed."), "{}", run.out);
}

#[test]
fn the_running_version_and_no_version_both_print_no_note() {
    let matching = tree(&format!(
        r#"{{ "project": "t", "version": "{}",
              "doc_size": [{{"file": "README.md", "ceiling": 10}}] }}"#,
        env!("CARGO_PKG_VERSION")
    ));
    let absent = tree(r#"{ "project": "t", "doc_size": [{"file": "README.md", "ceiling": 10}] }"#);

    for tree in [matching, absent] {
        let run = tree.run(&["gate"]);
        assert_eq!(run.code, 0, "{}", run.out);
        assert!(!run.says("NOTE"), "{}", run.out);
    }
}

#[test]
fn a_version_that_is_not_a_string_is_a_tool_error() {
    let tree = tree(
        r#"{ "project": "t", "version": 1,
                         "doc_size": [{"file": "README.md", "ceiling": 10}] }"#,
    );

    let run = tree.run(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"version\""), "{}", run.out);
}

#[test]
fn the_version_note_reaches_the_json_notes() {
    let tree = tree(ANOTHER_VERSION);

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let notes = list(&report, "notes");
    let note = notes
        .iter()
        .find(|note| field(note, "outcome") == "version")
        .unwrap_or_else(|| panic!("no version note in:\n{}", run.out));
    assert!(field(note, "text").contains("0.0.1"), "{}", run.out);
}

#[test]
fn the_hook_hands_back_the_version_note_and_does_not_block_the_stop() {
    let tree = tree(ANOTHER_VERSION);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("0.0.1"), "{}", run.out);
}

/// A config that names a key no klin version reads, beside a gate that would otherwise pass.
const AN_UNKNOWN_KEY: &str = r#"{
  "project": "t",
  "nonsense": true,
  "doc_size": [{"file": "README.md", "ceiling": 10}]
}"#;

#[test]
fn hook_reports_a_config_error_and_does_not_block_the_stop() {
    let tree = tree(AN_UNKNOWN_KEY);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("\"nonsense\""), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn hook_with_strict_is_a_usage_error_and_runs_no_gate() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = harness::feed(tree.root(), &["gate", "--hook", "--strict"], A_STOP);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("--hook"), "{}", run.out);
    assert!(run.says("--strict"), "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
}

#[test]
fn no_ci_variable_changes_what_a_run_does() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let plain = tree.run(&["gate"]);
    let in_ci = tree.run_with(
        &[("CI", "true"), ("CONTINUOUS_INTEGRATION", "true")],
        &["gate"],
    );
    assert_eq!(plain.code, in_ci.code, "{}", in_ci.out);
    assert_eq!(plain.out, in_ci.out);
}

#[test]
fn hook_reports_a_schedule_with_no_step_due_and_does_not_block_the_stop() {
    let tree = tree(r#"{"doc_size": [{"file": "README.md", "ceiling": {"2999-01-01": 10}}]}"#);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("no step due"), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn hook_reports_a_section_naming_a_retired_key_and_does_not_block_the_stop() {
    let tree =
        tree(r#"{"escapes": {"roots": ["src"], "languages": ["rust"], "baseline": "old.json"}}"#);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("\"baseline\""), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

const A_LOST_FILE: &str = r#"{ "project": "t",
  "complexity": { "in": "src", "except": "src/gone.rs", "cc": 8, "lines": 60 } }"#;

fn lost_file() -> Tree {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 5);
    tree.write("src/gone.rs", "pub fn other() -> i32 { 2 }\n");
    tree.base();
    tree.write("klin.json", A_LOST_FILE);
    tree
}

#[test]
fn a_coverage_loss_is_a_note_under_the_gate_in_the_json() {
    let tree = lost_file();

    let run = tree.run(&["gate", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
    assert_eq!(
        outcomes(list(&report, "notes")),
        [("complexity", "lost")],
        "{}",
        run.out
    );
    let note = &list(&report, "notes")[0];
    assert_eq!(field(note, "file"), "src/gone.rs", "{}", run.out);
    let gates = list(&report, "gates");
    let gate = gates
        .iter()
        .find(|gate| field(gate, "name") == "complexity")
        .unwrap_or_else(|| panic!("no complexity row: {}", run.out));
    assert_eq!(gate["notes"], 1, "{}", run.out);
}

#[test]
fn a_coverage_loss_in_the_hook_is_a_note_and_the_turn_ends() {
    let tree = lost_file();

    let run = stop(&tree, A_STOP);
    assert_ne!(run.code, 2, "{}", run.out);
    assert!(
        run.says("NOTE: src/gone.rs was measured at the base"),
        "{}",
        run.out
    );
}
