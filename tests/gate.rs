mod harness;

use harness::Tree;
use serde_json::Value;

const CLEAN: &str = "pub fn simple(a: i32) -> i32 {\n    a + 1\n}\n";

const EVERY_GATE: &str = r#"{
  "doc_size": {"README.md": 10},
  "escapes": { "in": "src" },
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

/// A config whose escapes scope holds no applicable file, so that gate errors.
const A_BROKEN_GATE: &str = r#"{
  "doc_size": {"README.md": 10},
  "doc_citations": false,
  "escapes": { "in": "README.md" },
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

/// A config holding one accepted escape that no site in the tree matches.
const AN_UNMATCHED_ACCEPTED: &str = r#"{
  "accepted": [{"gate": "escapes", "file": "src/gone.rs", "text": "the line that held it",
                "count": 1}],
  "doc_size": {"README.md": 10},
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

    let run = tree.run(&["check"]);
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
    let tree = tree(r#"{ "doc_size": {"README.md": 10} }"#);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    stubs"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(!run.says("derived: escapes"), "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn a_status_row_per_gate_and_a_summary_line() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    doc-size"), "{}", run.out);
    assert!(run.says("ok    doc-citations"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn a_passing_gate_prints_a_row_and_the_one_ok_line_under_it() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(
        run.says("OK: 0 escape site(s) in the tree ("),
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

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(
        run.says("README.md is 30 words, over its ceiling of 10"),
        "{}",
        run.out
    );
    assert!(
        run.says("judgement: fail, measurement: complete, execution: ok, exit 1"),
        "{}",
        run.out
    );
}

#[test]
fn every_gate_runs_even_when_an_earlier_one_failed() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    let source = format!("pub fn f() {{\n    x.{};\n}}\n", "unwrap()");
    tree.write("src/work.rs", &source);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("FAIL  escapes"), "{}", run.out);
    assert!(run.says("ok    complexity"), "{}", run.out);
    assert!(
        run.says("judgement: fail, measurement: complete, execution: ok, exit 1"),
        "{}",
        run.out
    );
}

#[test]
fn a_tool_error_is_distinguishable_from_a_gate_failure() {
    let tree = tree(A_BROKEN_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("judgement: fail, measurement: complete, execution: error, exit 2"),
        "{}",
        run.out
    );
}

#[test]
fn a_tool_error_alone_exits_two() {
    let tree = tree(A_BROKEN_GATE);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: error, exit 2"),
        "{}",
        run.out
    );
}

/// The one row per gate `policy` prints, without the per-key lines under each and without the
/// build, accepted and state directory lines that follow them all.
fn rows(run: &harness::Run) -> String {
    run.out
        .lines()
        .filter(|line| !line.starts_with(' '))
        .filter(|line| {
            !["state: ", "build — ", "accepted — "]
                .iter()
                .any(|at| line.starts_with(at))
        })
        .map(|line| line.to_string() + "\n")
        .collect()
}

#[test]
fn list_prints_the_configured_gates_and_runs_none_of_them() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — runs\ndoc-citations — runs\nescapes — runs\nstubs — runs\n\
         complexity — runs\ndead-symbols — runs\nreachability — runs\n\
         public-api — runs\n\
         lockfile — not-applicable\n\
         inventory — not-applicable\n\
         layering — needs a section a person writes\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n\
         measurement-lost — built-in\n",
        "{:?}",
        run.out
    );
    assert!(!run.says("ok    doc-size"), "{}", run.out);
}

#[test]
fn list_puts_the_excluded_gates_before_the_ones_that_need_a_section() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — runs\ndoc-citations — runs\ncomplexity — excluded\n\
         lockfile — not-applicable\n\
         escapes — not-applicable\n\
         stubs — not-applicable\n\
         inventory — not-applicable\n\
         dead-symbols — not-applicable\n\
         reachability — not-applicable\n\
         layering — needs a section a person writes\n\
         public-api — not-applicable\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n\
         measurement-lost — built-in\n",
        "{:?}",
        run.out
    );
}

#[test]
fn list_ends_with_the_state_directory() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let last = run.out.trim_end().lines().last().unwrap_or_default();
    assert!(last.starts_with("state: "), "{}", run.out);
    assert!(last.contains("klin"), "{}", run.out);
}

#[test]
fn gate_by_name_runs_only_that_gate() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn gate_by_name_is_repeatable_and_keeps_ladder_order() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["check", "complexity", "doc-size"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(at(&run, "doc-size") < at(&run, "complexity"), "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn a_gate_name_the_config_does_not_configure_is_an_unsupported_hole() {
    let tree = without_source(r#"{ "doc_size": {"README.md": 10} }"#);

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("INCOMPLETE  escapes"), "{}", run.out);
    assert!(run.says("HOLE: unsupported"), "{}", run.out);
}

#[test]
fn a_config_that_configures_no_gate_is_a_nothing_measured_hole() {
    let tree = nothing_to_survey(r#"{}"#);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("HOLE: nothing-measured"), "{}", run.out);
    assert!(
        run.says("holds no language or document klin measures"),
        "{}",
        run.out
    );
}

#[test]
fn a_section_named_after_the_command_is_a_tool_error() {
    let tree = tree(r#"{ "doc-size": [{"file": "README.md", "ceiling": 1}] }"#);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"doc-size\" is what the command"), "{}", run.out);
    assert!(run.says("\"doc_size\""), "{}", run.out);
}

#[test]
fn an_accepted_entry_that_matched_nothing_is_a_review_item_of_its_gate() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("REVIEW  escapes"), "{}", run.out);
}

#[test]
fn the_config_flag_names_the_klin_json_every_gate_runs_under() {
    let tree = tree(EVERY_GATE);
    tree.write("elsewhere/klin.json", EVERY_GATE);
    tree.words("elsewhere/README.md", 30);
    tree.write("elsewhere/src/lib.rs", CLEAN);

    let run = tree.run(&["check", "--config", &tree.at("elsewhere/klin.json")]);
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

    let scoped = tree.run(&["check", "--changed"]);
    assert_eq!(scoped.code, 1, "{}", scoped.out);
    assert!(scoped.says("changed: 1 file(s)"), "{}", scoped.out);
    assert!(scoped.says("src/new.rs:2"), "{}", scoped.out);
    assert!(!scoped.says("src/old.rs"), "{}", scoped.out);

    let whole = tree.run(&["check"]);
    assert_eq!(whole.code, 1, "{}", whole.out);
    assert!(whole.says("src/new.rs:2"), "{}", whole.out);
    assert!(!whole.says("src/old.rs"), "{}", whole.out);
}

#[test]
fn a_gate_that_is_not_scoped_still_runs_over_everything() {
    let tree = based(EVERY_GATE, &[]);
    tree.words("README.md", 30);
    tree.write("src/new.rs", CLEAN);

    let run = tree.run(&["check", "--changed"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("README.md is 30 words"), "{}", run.out);
    assert!(run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn an_accepted_entry_for_a_file_outside_the_changed_set_is_not_judged() {
    let tree = based(AN_UNMATCHED_ACCEPTED, &[]);
    tree.write("src/new.rs", CLEAN);

    let scoped = tree.run(&["check", "--changed"]);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
    assert!(!scoped.says("matched nothing this run"), "{}", scoped.out);

    let whole = tree.run(&["check"]);
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

    let in_ci = tree.run_with(&[("GITHUB_BASE_REF", "release")], &["check", "--changed"]);
    assert_eq!(in_ci.code, 1, "{}", in_ci.out);
    assert!(in_ci.says("src/newer.rs:2"), "{}", in_ci.out);
    assert!(!in_ci.says("src/new.rs:2"), "{}", in_ci.out);

    let locally = tree.run(&["check", "--changed"]);
    assert_eq!(locally.code, 1, "{}", locally.out);
    assert!(locally.says("src/new.rs:2"), "{}", locally.out);
}

#[test]
fn a_run_outside_a_repository_is_a_tool_error_rather_than_an_empty_pass() {
    let tree = Tree::bare();
    tree.write("klin.json", EVERY_GATE);
    tree.words("README.md", 5);
    tree.write("src/new.rs", AN_ESCAPE);

    let run = tree.run(&["check", "--changed"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no base commit"), "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn changed_restricts_complexity_as_well_as_escapes() {
    let tree = based(EVERY_GATE, &[("src/old.rs", &tangled("was_here"))]);
    tree.write("src/new.rs", &tangled("is_new"));

    let scoped = tree.run(&["check", "--changed"]);
    assert_eq!(scoped.code, 1, "{}", scoped.out);
    assert!(scoped.says("FAIL  complexity"), "{}", scoped.out);
    assert!(scoped.says("src/new.rs:1"), "{}", scoped.out);
    assert!(!scoped.says("src/old.rs"), "{}", scoped.out);
}

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;

fn stop(tree: &Tree, event: &str) -> harness::Run {
    harness::feed(tree.root(), harness::AGENT, event)
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
fn a_host_flag_alone_does_not_spend_a_gate_block() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_SECOND_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("this stop is not blocked"), "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("not blocking again"), "{}", run.out);
    assert!(
        run.says("klin holds no record of the tree the last gate block saw"),
        "{}",
        run.out
    );
    assert!(!run.says("then stop again"), "{}", run.out);
    assert!(!run.says("CI will refuse"), "{}", run.out);
}

#[test]
fn hook_prints_only_the_gates_that_did_not_pass() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("pinned: doc_size README.md 10"), "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("gate(s)"), "{}", run.out);
    assert!(run.says("1 failed."), "{}", run.out);
    for passed in [
        "ok    escapes",
        "ok    complexity",
        "pinned: complexity cc 8",
        "OK:",
    ] {
        assert!(!run.says(passed), "{passed}: {}", run.out);
    }

    let by_hand = tree.run(&["check"]);
    assert_eq!(by_hand.code, 1, "{}", by_hand.out);
    assert!(by_hand.says("ok    escapes"), "{}", by_hand.out);
    assert!(by_hand.says("pinned: complexity cc 8"), "{}", by_hand.out);
}

#[test]
fn hook_keeps_the_note_a_passing_gate_left_beside_the_failure() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write("src/flow.rs", "%%% not rust %%%\n");

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(
        run.says("src/flow.rs is not measured (unreadable)"),
        "{}",
        run.out
    );
    assert!(!run.says("OK:"), "{}", run.out);
}

#[test]
fn the_structured_report_of_a_hook_stop_still_holds_every_gate() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let report = object(journal.lines().last().unwrap_or_default(), &run);
    let gates = harness::gate_rows(&report)
        .as_array()
        .cloned()
        .unwrap_or_default();
    for name in ["doc-size", "escapes", "complexity"] {
        assert!(
            gates.iter().any(|gate| field(gate, "name") == name),
            "{name}: {report}"
        );
    }
}

#[test]
fn hook_says_nothing_when_every_gate_passes() {
    let tree = tree(EVERY_GATE);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "{:?}", run.out);
}

/// An event the ingress cannot read names no kind, so it runs no gate and never exits 2.
/// Spec 10.10.
#[test]
fn a_hook_event_it_cannot_read_answers_nothing() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    for event in ["", "not json", "{}"] {
        let run = stop(&tree, event);
        assert_eq!(run.code, 0, "{event:?}: {}", run.out);
        assert_eq!(run.printed, "", "{event:?}: {}", run.out);
        assert!(!run.says("doc-size"), "{event:?}: {}", run.out);
    }
}

/// A capability-scope error blocks nothing: the Stop tells it to the person. Spec 7.3, 10.4.
#[test]
fn a_tool_error_alone_blocks_nothing_and_is_told() {
    let tree = tree(A_BROKEN_GATE);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(run.says("nothing blocks the stop"), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn hook_names_both_when_a_gate_failed_and_another_could_not_run() {
    let tree = tree(A_BROKEN_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("a quality gate failed — fix what each FAIL names"),
        "{}",
        run.out
    );
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("7 gate(s), 1 excluded, 1 failed, 1 tool error."),
        "{}",
        run.out
    );
}

/// A tool error spends no gate block, so the FAIL after it takes the first. Spec 10.4.
#[test]
fn a_tool_error_spends_no_gate_block_so_the_fail_after_it_takes_the_first() {
    let tree = tree(A_BROKEN_GATE);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 0, "{}", first.out);

    tree.words("README.md", 30);
    let second = stop(&tree, A_STOP);
    assert_eq!(second.code, 2, "{}", second.out);
    assert!(second.says("gate block 1 of 2"), "{}", second.out);
    assert!(second.says("FAIL  doc-size"), "{}", second.out);
    assert!(second.says("ERR   escapes"), "{}", second.out);
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

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 3, "{}", run.out);
    assert!(run.says("HOLE: nothing-measured"), "{}", run.out);
    assert!(run.says("naming one of: doc-size"), "{}", run.out);
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
    assert!(settings.contains("__agent event"), "{settings}");
    for wrapper in [BUILD_BLOCKED, "stop_hook_active", "cargo build"] {
        assert!(!settings.contains(wrapper), "{wrapper}: {settings}");
    }
}

#[test]
fn the_stamp_sits_beside_the_config_rather_than_the_working_directory() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let run = harness::feed(&tree.path("src"), harness::AGENT, A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(tree.path(BUILD_BLOCKED).is_file(), "{}", run.out);
}

#[test]
fn the_gate_blocks_twice_under_each_prompt_and_only_over_a_changed_tree() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    assert!(
        first.says("fix what each names, then stop again (gate block 1 of 2 in this turn)"),
        "{}",
        first.out
    );

    let unchanged = stop(&tree, A_SECOND_STOP);
    assert_eq!(unchanged.code, 0, "{}", unchanged.out);
    assert!(unchanged.says("FAIL  doc-size"), "{}", unchanged.out);
    assert!(!unchanged.says("ok    escapes"), "{}", unchanged.out);
    assert!(
        unchanged.says("not blocking again; the tree did not change since the last gate block"),
        "{}",
        unchanged.out
    );

    tree.words("README.md", 31);
    let changed = stop(&tree, A_SECOND_STOP);
    assert_eq!(changed.code, 2, "{}", changed.out);
    assert!(
        changed.says("fix what each names, then stop again (gate block 2 of 2 in this turn)"),
        "{}",
        changed.out
    );

    tree.words("README.md", 32);
    let capped = stop(&tree, A_SECOND_STOP);
    assert_eq!(capped.code, 0, "{}", capped.out);
    assert!(
        capped.says("not blocking again; the gate has blocked 2 stops under this prompt"),
        "{}",
        capped.out
    );

    let prompt = harness::feed(tree.root(), harness::AGENT, A_PROMPT);
    assert_eq!(prompt.code, 0, "{}", prompt.out);
    let after = stop(&tree, A_STOP);
    assert_eq!(after.code, 2, "{}", after.out);
    assert!(after.says("gate block 1 of 2"), "{}", after.out);
}

#[test]
fn a_second_gate_block_klin_cannot_record_is_reported_and_blocks_nothing() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let staging = tree.path(".git/klin/build-blocked.writing");
    assert!(
        std::fs::create_dir_all(&staging).is_ok(),
        "{}",
        staging.display()
    );

    tree.words("README.md", 31);
    let second = stop(&tree, A_SECOND_STOP);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(
        second.says("klin could not record a gate block"),
        "{}",
        second.out
    );
}

/// A record an older klin wrote names one spent gate block and no gate tree, so it can never
/// prove the tree changed. Spec 16.3.
#[test]
fn a_record_an_older_klin_wrote_proves_no_second_gate_block() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let first = stop(&tree, A_STOP);
    assert_eq!(first.code, 2, "{}", first.out);
    let text = std::fs::read_to_string(tree.path(BUILD_BLOCKED)).unwrap_or_default();
    let held: Value = object(&text, &first);
    let older = serde_json::json!({
        "prompt": held["prompt"],
        "builds": 0,
        "gate_spent": true,
        "tree": "a tree an older build block saw",
    });
    tree.write(BUILD_BLOCKED, &older.to_string());

    tree.words("README.md", 31);
    let second = stop(&tree, A_SECOND_STOP);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(
        second.says("klin holds no record of the tree the last gate block saw"),
        "{}",
        second.out
    );
}

#[test]
fn the_ladder_writes_nothing() {
    let tree = tree(EVERY_GATE);
    tree.write("src/lib.rs", AN_ESCAPE);
    tree.words("README.md", 30);

    let run = tree.run(&["check"]);
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

/// The check and the kind of each record: a finding's outcome, a note's kind, an error's reason.
fn outcomes(records: &[Value]) -> Vec<(&str, &str)> {
    records
        .iter()
        .map(|record| {
            let said = ["outcome", "reason", "kind"]
                .into_iter()
                .map(|key| field(record, key))
                .find(|said| !said.is_empty())
                .unwrap_or_default();
            let check = match field(record, "check") {
                "" => field(record, "gate"),
                check => check,
            };
            (check, said)
        })
        .collect()
}

#[test]
fn json_prints_one_object_holding_every_failing_finding() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);
    tree.write("src/work.rs", AN_ESCAPE);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "judgement"), "fail", "{}", run.out);
    assert_eq!(report.get("exit"), Some(&Value::from(1)), "{}", run.out);
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

    let run = tree.run(&["check", "--json"]);
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
    assert!(field(finding, "remedy").contains("escape"), "{}", run.out);
}

#[test]
fn hook_evidence_is_the_original_build_blocked_stop_not_a_second_gate_run() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"build":"exit 1","escapes":{"in":"src"}}"#);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree.write("src/work.rs", AN_ESCAPE);
    let evidence = tree.at("stop-report.json");

    let hook = harness::feed_with(
        tree.root(),
        &[("KLIN_HOOK_REPORT", evidence.as_str())],
        harness::AGENT,
        A_STOP,
    );
    assert_eq!(hook.code, 2, "{}", hook.out);
    let exact: Value = match std::fs::read_to_string(&evidence) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(report) => report,
            Err(why) => panic!("the hook evidence is not JSON: {why}"),
        },
        Err(why) => panic!("the original hook wrote no evidence: {why}"),
    };
    assert_eq!(exact["status"], "ERROR", "{exact}");
    assert_eq!(exact["exit"], 2, "{exact}");
    assert_eq!(exact["gates"], serde_json::json!([]), "{exact}");
    assert!(
        !exact["findings"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|finding| finding["gate"] == "escapes")
    );

    let standalone = tree.run(&["check", "--json"]);
    assert_eq!(standalone.code, 1, "{}", standalone.out);
    assert!(
        standalone.json()["findings"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|finding| finding["check"] == "escapes")
    );
}

#[test]
fn a_json_record_names_no_column_and_no_violation() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);
    tree.words("README.md", 30);
    tree.write("src/lib.rs", AN_ESCAPE);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 1, "{}", run.out);
    let report = run.json();
    let banned = ["column", "violation", "invariant"];
    assert!(!list(&report, "findings").is_empty(), "{}", run.out);
    assert!(!list(&report, "reviews").is_empty(), "{}", run.out);
    for record in list(&report, "findings")
        .iter()
        .chain(list(&report, "notes"))
        .chain(list(&report, "reviews"))
    {
        for key in banned {
            assert_eq!(record.get(key), None, "{key} is in {record}");
        }
    }
}

#[test]
fn json_reviews_say_why_a_run_with_nothing_over_the_gate_is_judged_review() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "judgement"), "review", "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
    let reviews: Vec<(&str, &str)> = list(&report, "reviews")
        .iter()
        .map(|review| (field(review, "check"), field(review, "kind")))
        .collect();
    assert_eq!(reviews, [("escapes", "unmatched-accepted")], "{}", run.out);
}

#[test]
fn a_gate_that_could_not_run_is_a_json_error() {
    let tree = tree(A_BROKEN_GATE);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "execution"), "error", "{}", run.out);
    let error = &list(&report, "errors")[0];
    assert_eq!(field(error, "check"), "escapes", "{}", run.out);
    assert_eq!(field(error, "kind"), "configuration", "{}", run.out);
    assert!(
        field(error, "message").contains("no applicable file"),
        "{}",
        run.out
    );
}

#[test]
fn a_passing_json_run_holds_no_findings() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "judgement"), "pass", "{}", run.out);
    assert_eq!(report.get("exit"), Some(&Value::from(0)), "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
    assert!(list(&report, "notes").is_empty(), "{}", run.out);
    assert!(!run.says("ok    escapes"), "{}", run.out);
}

#[test]
fn a_json_run_with_pinned_policy_invents_no_derived_entries() {
    let tree = tree(
        r#"{ "doc_size": {"README.md": 10}, "doc_citations": false,
             "escapes": { "in": "src" },
             "complexity": { "in": "src", "cc": 8, "lines": 60 } }"#,
    );

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    let derived = harness::derived(&report);
    assert!(derived.is_empty(), "{}", run.out);
}

#[test]
fn a_config_the_run_cannot_read_is_a_json_object_too() {
    let tree = tree(EVERY_GATE);

    let run = tree.run(&["check", "--json", "--config", "absent.json"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "execution"), "error", "{}", run.out);
    assert_eq!(report.get("exit"), Some(&Value::from(2)), "{}", run.out);
    let error = &list(&report, "errors")[0];
    assert_eq!(field(error, "kind"), "invocation", "{}", run.out);
    assert!(
        field(error, "message").contains("could not be read"),
        "{}",
        run.out
    );
}

#[test]
fn a_new_file_the_grammar_rejected_is_one_review_item_at_its_own_file() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.rs", "fn ( { ) unbalanced");

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert!(list(&report, "errors").is_empty(), "{}", run.out);
    assert_eq!(
        outcomes(list(&report, "reviews")),
        [("", "unreadable")],
        "{}",
        run.out
    );
    assert_eq!(
        field(&list(&report, "reviews")[0], "file"),
        "src/broken.rs",
        "{}",
        run.out
    );
}

#[test]
fn a_file_no_grammar_read_at_the_base_either_is_a_coverage_note() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.rs", "fn ( { ) unbalanced");
    tree.base();

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: src/broken.rs is not measured (unreadable)"),
        "{}",
        run.out
    );
    let run = tree.run(&["check", "--json"]);
    let report = run.json();
    assert_eq!(
        outcomes(list(&report, "notes")),
        [("", "unreadable")],
        "{}",
        run.out
    );
    assert_eq!(list(&report, "notes")[0]["coverage"], true, "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
}

#[test]
fn a_file_no_grammar_read_at_the_base_either_is_a_note_after_a_rename() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.rs", "fn ( { ) unbalanced");
    tree.base();
    tree.git(&["mv", "src/broken.rs", "src/moved.rs"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: src/moved.rs is not measured (unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn a_rename_one_grammar_reads_at_both_paths_keeps_the_note() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.js", "function ( { ) unbalanced");
    tree.base();
    tree.git(&["mv", "src/broken.js", "src/broken.mjs"]);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: src/broken.mjs is not measured (unreadable)"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_the_base_parsed_and_the_change_broke_is_a_measurement_lost_fail_by_hand() {
    let tree = tree(EVERY_GATE);
    tree.write("src/broken.rs", "fn fine() -> i32 { 1 }\n");
    tree.base();
    tree.write("src/broken.rs", "fn ( { ) unbalanced");

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  measurement-lost"), "{}", run.out);
    assert!(
        run.says("src/broken.rs was measured at the base"),
        "{}",
        run.out
    );
}

const WORKFLOW: &str = include_str!("../.github/workflows/quality.yml");

fn ci_arguments() -> Vec<&'static str> {
    WORKFLOW
        .lines()
        .find(|line| line.contains("klin check"))
        .unwrap_or_else(|| panic!("no klin check line in:\n{WORKFLOW}"))
        .split_whitespace()
        .skip_while(|word| *word != "check")
        .collect()
}

#[test]
fn deleting_a_section_leaves_the_gate_running_over_a_derived_section() {
    let tree = tree(EVERY_GATE);

    let whole = tree.run(&ci_arguments());
    assert_eq!(whole.code, 0, "{}", whole.out);

    tree.write(
        "klin.json",
        r#"{ "doc_size": {"README.md": 10},
             "complexity": { "in": "src", "cc": 8, "lines": 60 } }"#,
    );
    let deleted = tree.run(&ci_arguments());
    assert_eq!(deleted.code, 0, "{}", deleted.out);
    assert!(!deleted.says("derived: escapes"), "{}", deleted.out);
}

const AN_EXCLUDED_GATE: &str = r#"{
  "doc_size": {"README.md": 10},
  "escapes": false,
  "complexity": { "in": "src", "cc": 8, "lines": 60 }
}"#;

const NOTHING_SAID_ABOUT_ESCAPES: &str = r#"{
  "doc_size": {"README.md": 10},
  "complexity": false
}"#;

/// `policy` names each value a gate used with where it came from: the section pinned one, and
/// the run derived the other with its rule. Spec 11.6.
#[test]
fn policy_names_each_value_of_one_gate_with_its_provenance() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity": {"cc": 8}}"#);
    tree.write("src/lib.rs", "fn a() {}\nfn b() {}\n");
    tree.base();

    let run = tree.run(&["policy", "complexity"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("complexity — runs"), "{}", run.out);
    assert!(run.says("pinned: complexity cc 8"), "{}", run.out);
    assert!(
        run.says("derived: complexity lines 25 (the floor of 25"),
        "{}",
        run.out
    );
    assert!(!run.says("escapes"), "{}", run.out);
}

#[test]
fn a_named_gate_runs_alone_when_the_command_line_names_it() {
    let tree = tree(AN_EXCLUDED_GATE);
    tree.write("src/big.rs", &tangled("big"));

    let run = tree.run(&["check", "complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL  complexity"), "{}", run.out);
    assert!(!run.says("doc-size"), "{}", run.out);
    assert!(
        run.says("judgement: fail, measurement: complete, execution: ok, exit 1"),
        "{}",
        run.out
    );
}

/// The top-level `gates` list is gone: a gate's multiplicity belongs to its own section, as the
/// named entries of `sarif` and the named conventions have it. ADR 0038.
#[test]
fn a_gates_key_is_not_one_klin_reads() {
    let tree = tree(r#"{ "gates": [{"name": "n", "check": "complexity", "with": {}}] }"#);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"gates\" is not a key klin reads"), "{}", run.out);
}

#[test]
fn two_gates_of_one_name_are_a_tool_error() {
    let tree = tree(
        r#"{ "doc_size": {"README.md": 10},
              "sarif": [{"name": "doc-size", "report": "out/lint.sarif"}] }"#,
    );

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("two gates are named doc-size"), "{}", run.out);
}

#[test]
fn a_section_set_to_false_excludes_its_gate_and_the_summary_counts_it() {
    let tree = tree(AN_EXCLUDED_GATE);
    tree.write("src/risky.rs", AN_ESCAPE);

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("escapes"), "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn list_names_the_excluded_gates() {
    let tree = tree(AN_EXCLUDED_GATE);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — runs\ndoc-citations — runs\nstubs — runs\ncomplexity — runs\n\
         dead-symbols — runs\nreachability — runs\npublic-api — runs\n\
         escapes — excluded\nlockfile — not-applicable\n\
         inventory — not-applicable\n\
         layering — needs a section a person writes\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n\
         measurement-lost — built-in\n",
        "{:?}",
        run.out
    );
}

#[test]
fn list_names_an_available_gate_the_survey_cannot_supply_as_not_applicable() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("escapes — not-applicable"), "{}", run.out);
}

#[test]
fn list_names_a_gate_the_survey_supplies_as_one_that_runs() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["policy"]);
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

    let run = tree.run(&["check", "escapes"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("named escapes is excluded"), "{}", run.out);
}

#[test]
fn strict_accounts_for_a_gate_the_survey_supplies() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let strict = tree.run(&["check"]);
    assert_eq!(strict.code, 0, "{}", strict.out);
    assert!(strict.says("ok    escapes"), "{}", strict.out);
}

/// The retired `--strict` failure of ADR 0010: a config that leaves a derivable gate out is a
/// config klin derives that section for, so there is nothing left to account for. The hole it
/// closed is closed by the source root failure below instead. ADR 0016.
#[test]
fn strict_accepts_a_config_that_omits_a_derivable_gate() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);

    let strict = tree.run(&["check"]);
    assert_eq!(strict.code, 0, "{}", strict.out);
    assert!(strict.says("ok    escapes"), "{}", strict.out);
    assert!(!strict.says("unaccounted"), "{}", strict.out);
}

#[test]
fn a_tree_the_survey_finds_no_source_root_in_is_a_nothing_measured_hole() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("notes.txt", "nothing klin measures");
    tree.base();

    let strict = tree.run(&["check"]);
    assert_eq!(strict.code, 3, "{}", strict.out);
    assert!(strict.says("HOLE: nothing-measured"), "{}", strict.out);
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

/// Only the checks a run selects count toward the hole, so a run of a check that reads no code
/// judges a tree with no source root. Spec 11.3.
#[test]
fn a_selected_check_that_reads_no_code_runs_where_the_survey_finds_no_source_root() {
    let tree = without_source(NOTHING_SAID_ABOUT_ESCAPES);

    let run = tree.run(&["check", "doc-size"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("found no source root"), "{}", run.out);
}

/// The failure is about a gate the survey was left to supply, so a tree that reads no code
/// because a person excluded every gate that does is not this hole. ADR 0016.
#[test]
fn strict_accepts_a_tree_with_no_source_when_every_code_gate_is_excluded() {
    let tree = without_source(
        r#"{ "doc_size": {"README.md": 10},
              "complexity": false,
              "escapes": false,
              "stubs": false,
              "dead_symbols": false,
              "reachability": false,
              "public_api": false }"#,
    );

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("no source root"), "{}", run.out);
}

#[test]
fn list_says_a_compact_source_policy_runs() {
    let tree = tree(
        r#"{ "doc_size": {"README.md": 10},
              "escapes": {"in": "src"} }"#,
    );

    let run = tree.run(&["policy"]);
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
    tree.words("AGENTS.md", 5);

    let every = tree.run(&["check"]);
    assert_eq!(every.code, 0, "{}", every.out);
    assert!(
        every.says("derived: doc_size AGENTS.md 50, the 50-word default"),
        "{}",
        every.out
    );

    let one = tree.run(&["check", "escapes"]);
    assert_eq!(one.code, 0, "{}", one.out);
    assert!(!one.says("derived: escapes"), "{}", one.out);
    assert!(!one.says("doc_size"), "{}", one.out);
    assert!(!one.says("derived: complexity"), "{}", one.out);
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

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("found no source root"), "{}", run.out);
}

#[test]
fn strict_passes_once_every_derivable_gate_is_set_to_false() {
    let tree = tree(NOTHING_SAID_ABOUT_ESCAPES);
    tree.write(
        "klin.json",
        r#"{ "doc_size": {"README.md": 10},
              "doc_citations": false,
              "complexity": false,
              "escapes": false,
              "stubs": false,
              "dead_symbols": false,
              "reachability": false,
              "public_api": false }"#,
    );

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("judgement: pass, measurement: complete, execution: ok, exit 0"),
        "{}",
        run.out
    );
}

#[test]
fn list_names_the_exclusions_when_every_gate_is_excluded() {
    let tree = tree(
        r#"{ "doc_size": false, "doc_citations": false, "escapes": false,
              "stubs": false, "complexity": false, "dead_symbols": false,
              "reachability": false, "public_api": false }"#,
    );

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        rows(&run),
        "doc-size — excluded\ndoc-citations — excluded\nescapes — excluded\n\
         stubs — excluded\ncomplexity — excluded\n\
         dead-symbols — excluded\nreachability — excluded\n\
         public-api — excluded\n\
         lockfile — not-applicable\n\
         inventory — not-applicable\n\
         layering — needs a section a person writes\n\
         conventions — needs a section a person writes\n\
         sarif — needs a section a person writes\n\
         measurement-lost — built-in\n",
        "{:?}",
        run.out
    );

    let judged = tree.run(&["check"]);
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
    assert!(
        run.says("src/flow.rs is not measured (unreadable)"),
        "{}",
        run.out
    );
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
    assert!(
        run.says("src/flow.rs is not measured (unreadable)"),
        "{}",
        run.out
    );
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

    let (run, report) = harness::stop_report(tree.root(), A_STOP, &[]);
    assert_eq!(run.code, 0, "{}", run.out);
    let notes = list(&report, "notes");
    assert_eq!(notes.len(), 1, "{}", run.out);
    assert_eq!(field(&notes[0], "outcome"), "unparsed", "{}", run.out);
    assert_eq!(field(&notes[0], "file"), "src/broken.rs", "{}", run.out);
    assert!(list(&report, "findings").is_empty(), "{}", run.out);
}

const ANOTHER_VERSION: &str = r#"{
  "version": "0.0.1",
  "doc_size": {"README.md": 10}
}"#;

/// A configuration names no repository and no klin version: both are facts klin reads, so a
/// person who still writes either is told to delete it. ADR 0040.
#[test]
fn a_project_or_version_key_is_a_config_error_that_says_to_delete_it() {
    for (key, value) in [("project", "\"t\""), ("version", "\"0.0.1\"")] {
        let tree = tree(&format!(
            r#"{{ "{key}": {value}, "doc_size": {{"README.md": 10}} }}"#
        ));

        let run = tree.run(&["check"]);
        assert_eq!(run.code, 2, "{key}: {}", run.out);
        assert!(
            run.says(&format!("\"{key}\" is not a key klin reads")),
            "{}",
            run.out
        );
        assert!(run.says("delete the key"), "{}", run.out);
    }
}

#[test]
fn the_hook_reports_a_retired_version_key_and_does_not_block_the_stop() {
    let tree = tree(ANOTHER_VERSION);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(r#"\"version\""#), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

/// A config that names a key no klin version reads, beside a gate that would otherwise pass.
const AN_UNKNOWN_KEY: &str = r#"{
  "nonsense": true,
  "doc_size": {"README.md": 10}
}"#;

#[test]
fn hook_reports_a_config_error_and_does_not_block_the_stop() {
    let tree = tree(AN_UNKNOWN_KEY);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(r#"\"nonsense\""#), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn no_ci_variable_changes_what_a_run_does() {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 30);

    let plain = tree.run(&["check"]);
    let in_ci = tree.run_with(
        &[("CI", "true"), ("CONTINUOUS_INTEGRATION", "true")],
        &["check"],
    );
    assert_eq!(plain.code, in_ci.code, "{}", in_ci.out);
    assert_eq!(plain.out, in_ci.out);
}

#[test]
fn hook_reports_a_schedule_with_no_step_due_and_does_not_block_the_stop() {
    let tree = tree(r#"{"doc_size": {"README.md": {"2999-01-01": 10}}}"#);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("no step due"), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn hook_reports_a_section_naming_a_retired_key_and_does_not_block_the_stop() {
    let tree =
        tree(r#"{"escapes": {"roots": ["src"], "languages": ["rust"], "baseline": "old.json"}}"#);

    let run = stop(&tree, A_STOP);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says(r#"\"baseline\""#), "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

const A_LOST_FILE: &str =
    r#"{ "complexity": { "in": "src", "except": "src/gone.rs", "cc": 8, "lines": 60 } }"#;

fn lost_file() -> Tree {
    let tree = tree(EVERY_GATE);
    tree.words("README.md", 5);
    tree.write("src/gone.rs", "pub fn other() -> i32 { 2 }\n");
    tree.base();
    tree.write("klin.json", A_LOST_FILE);
    tree
}

#[test]
fn a_file_klin_json_drops_is_a_left_scope_coverage_note_in_the_json() {
    let tree = lost_file();

    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(
        outcomes(list(&report, "notes")),
        [("", "left-scope")],
        "{}",
        run.out
    );
    let note = &list(&report, "notes")[0];
    assert_eq!(field(note, "file"), "src/gone.rs", "{}", run.out);
    assert_eq!(note["coverage"], true, "{}", run.out);
    let complexity = list(&report, "capabilities")
        .iter()
        .find(|row| field(row, "name") == "complexity")
        .unwrap_or_else(|| panic!("no complexity row: {}", run.out));
    assert_eq!(complexity["coverage"]["limits"], 1, "{}", run.out);
}

#[test]
fn a_file_klin_json_drops_does_not_block_the_stop() {
    let tree = lost_file();

    let run = stop(&tree, A_STOP);
    assert_ne!(run.code, 2, "{}", run.out);
    assert!(!run.says("stop again"), "{}", run.out);
}

#[test]
fn a_file_klin_json_drops_passes_a_check_with_a_coverage_note() {
    let tree = lost_file();

    let run = tree.run(&["check"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: src/gone.rs is not measured (left-scope)"),
        "{}",
        run.out
    );
}

#[test]
fn a_minified_bundle_is_a_resource_limit_review_item_in_json() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("bundle.js", &"function bundled(){};".repeat(4_000));
    let run = tree.run(&["check", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    let report = run.json();
    assert_eq!(field(&report, "execution"), "ok", "{}", run.out);
    assert_eq!(
        outcomes(list(&report, "reviews")),
        [("", "resource-limit")],
        "{}",
        run.out
    );
    assert_eq!(
        field(&list(&report, "reviews")[0], "file"),
        "bundle.js",
        "{}",
        run.out
    );
}

#[test]
fn the_retired_commands_are_unrecognized_and_help_names_no_hidden_one() {
    let tree = Tree::new();
    for command in [
        &["gate"][..],
        &["gate", "--hook", "--changed"],
        &["radius"],
        &["guard"],
        &["complexity"],
        &["init"],
        &["install"],
        &["reference"],
        &["stats"],
    ] {
        let run = tree.run(command);
        let command = command.join(" ");
        assert_eq!(run.code, 2, "{command}: {}", run.out);
        assert!(
            run.says("unrecognized subcommand"),
            "{command}: {}",
            run.out
        );
    }

    let help = tree.run(&["--help"]);
    assert_eq!(help.code, 0, "{}", help.out);
    for hidden in ["radius", "guard", "turn", "cache", "gate", "__agent"] {
        assert!(!help.says(hidden), "{hidden}: {}", help.out);
    }
}
