mod harness;

use harness::{Run, Tree};

const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const A_SESSION: &str = r#"{"hook_event_name": "SessionStart"}"#;
const CONFIG: &str = r#"{
  "project": "t",
  "radius": { "lines": 50, "directories": 2 }
}
"#;

fn lines(count: usize, text: &str) -> String {
    (0..count)
        .map(|at| format!("{text}{at}\n"))
        .collect::<String>()
}

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree.write("src/a.rs", &lines(10, "// held "));
    tree.base();
    tree
}

fn radius(tree: &Tree, event: &str) -> Run {
    harness::feed(tree.root(), &["radius"], event)
}

/// The tree as the stamp sees it, so what follows is one turn's work.
fn stamped(tree: &Tree) -> Run {
    let run = radius(tree, A_SESSION);
    assert_eq!(run.code, 0, "{}", run.out);
    run
}

fn a_wide_turn(tree: &Tree) {
    tree.write("src/wide.rs", &lines(40, "fn f"));
    tree.write("docs/wide.md", &lines(50, "line "));
    tree.write("src/a.rs", &lines(10, "    // held "));
}

#[test]
fn a_prompt_after_a_wide_turn_reports_the_spread() {
    let tree = tree();
    stamped(&tree);
    a_wide_turn(&tree);

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("wider than this project's usual change"),
        "{}",
        run.out
    );
    assert!(
        run.says("110 lines in 3 files, under docs/, src/."),
        "{}",
        run.out
    );
    assert!(
        run.says("20 of those lines are formatting only, and 0 are code that moved."),
        "{}",
        run.out
    );
    assert!(
        run.says("Largest single file: docs/wide.md, 50 lines."),
        "{}",
        run.out
    );
    assert!(
        run.says("usually change about 50 lines under 2 directories."),
        "{}",
        run.out
    );
}

#[test]
fn a_turn_within_both_values_reports_nothing() {
    let tree = tree();
    stamped(&tree);
    tree.write("src/a.rs", &lines(10, "// touched "));

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("wider than"), "{}", run.out);
}

#[test]
fn a_config_with_no_radius_section_reports_nothing() {
    let tree = Tree::new();
    tree.write("klin.json", "{\n  \"project\": \"t\"\n}\n");
    tree.write("src/a.rs", &lines(10, "// held "));
    tree.base();
    stamped(&tree);
    a_wide_turn(&tree);

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("wider than"), "{}", run.out);
}

#[test]
fn a_user_diff_setting_does_not_change_the_numbers() {
    let tree = tree();
    tree.write("src/moved.rs", &lines(60, "fn f"));
    tree.commit("a file to move");
    stamped(&tree);
    tree.git(&["config", "diff.renames", "false"]);
    tree.git(&["config", "diff.algorithm", "myers"]);
    tree.git(&["mv", "src/moved.rs", "src/elsewhere.rs"]);

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("120 lines in 2 files, under src/."), "{}", run.out);
    assert!(
        run.says("0 of those lines are formatting only, and 120 are code that moved."),
        "{}",
        run.out
    );
}

#[test]
fn a_diff_setting_and_a_subdirectory_do_not_change_the_numbers() {
    let tree = tree();
    stamped(&tree);
    tree.git(&["config", "diff.relative", "true"]);
    a_wide_turn(&tree);

    let run = harness::feed(&tree.path("src"), &["radius"], A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("110 lines in 3 files, under docs/, src/."),
        "{}",
        run.out
    );
}

#[test]
fn the_report_names_directories_up_to_a_point() {
    let tree = tree();
    stamped(&tree);
    for at in 0..8 {
        tree.write(&format!("d{at}/f.rs"), &lines(10, "fn f"));
    }

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("80 lines in 8 files, under d0/, d1/, d2/, d3/, d4/, d5/, and 2 more."),
        "{}",
        run.out
    );
}

#[test]
fn a_session_start_reports_no_spread() {
    let tree = tree();
    stamped(&tree);
    a_wide_turn(&tree);

    let run = radius(&tree, A_SESSION);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("wider than"), "{}", run.out);
}

#[test]
fn report_measures_without_moving_the_stamp_or_the_counter() {
    let tree = tree();
    stamped(&tree);
    a_wide_turn(&tree);
    let commit = tree.field("commit");
    let prompts = tree.field("prompts");

    let run = tree.run(&["radius", "--report"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("measured against the turn stamp"), "{}", run.out);
    assert!(run.says("110 lines in 3 files"), "{}", run.out);
    assert_eq!(tree.field("commit"), commit, "--report moved the stamp");
    assert_eq!(
        tree.field("prompts"),
        prompts,
        "--report raised the counter"
    );
}

#[test]
fn report_names_a_missing_radius_section() {
    let tree = Tree::new();
    tree.write("klin.json", "{\n  \"project\": \"t\"\n}\n");
    tree.base();
    stamped(&tree);

    let run = tree.run(&["radius", "--report"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"radius\" section"), "{}", run.out);
}

#[test]
fn outside_a_repository_the_hook_says_nothing() {
    let tree = Tree::bare();
    tree.write("src/a.rs", &lines(10, "// held "));

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(run.out, "", "the hook spoke outside a repository");
}
