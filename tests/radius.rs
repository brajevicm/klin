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

/// Either value alone is enough, so a turn that spread over more directories than usual
/// reports even though its line count is ordinary.
#[test]
fn the_directories_alone_carry_the_report() {
    let tree = tree();
    stamped(&tree);
    for at in 0..3 {
        tree.write(&format!("d{at}/f.rs"), &lines(2, "fn f"));
    }

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("wider than"), "{}", run.out);
    assert!(
        run.says("6 lines in 3 files, under d0/, d1/, d2/."),
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

/// The stamp waits for a green stop, and the mark does not, so a second prompt after a red
/// stop measures its own turn rather than the window the first one left. ADR 0024.
#[test]
fn a_red_stop_does_not_widen_the_next_turn() {
    let tree = tree();
    stamped(&tree);
    a_wide_turn(&tree);
    let first = radius(&tree, A_PROMPT);
    assert!(first.says("wider than"), "{}", first.out);
    let stamp = tree.field("commit");

    tree.write("src/a.rs", &lines(10, "// touched "));
    let second = radius(&tree, A_PROMPT);
    assert_eq!(second.code, 0, "{}", second.out);
    assert!(!second.says("wider than"), "{}", second.out);
    assert_eq!(tree.field("commit"), stamp, "a red stop moved the stamp");
}

/// A session start ends no turn, and it still opens one, so the work before it belongs to no
/// turn the report names.
#[test]
fn a_session_start_moves_the_mark() {
    let tree = tree();
    stamped(&tree);
    a_wide_turn(&tree);
    assert_eq!(radius(&tree, A_SESSION).code, 0);

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("wider than"), "{}", run.out);
}

/// The mark is a commit under its own per-worktree ref, so a `git gc` cannot prune it.
#[test]
fn the_mark_is_a_ref_of_its_own() {
    let tree = tree();
    stamped(&tree);
    assert_eq!(tree.field("mark"), tree.revision("refs/worktree/klin/mark"));
    assert_ne!(tree.field("mark"), "", "no mark was written");

    a_wide_turn(&tree);
    let first = tree.field("mark");
    assert_eq!(radius(&tree, A_PROMPT).code, 0);
    assert_ne!(tree.field("mark"), first, "a prompt left the mark behind");
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
    let mark = tree.field("mark");
    let prompts = tree.field("prompts");

    let run = tree.run(&["radius", "--report"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("measured against the prompt mark"), "{}", run.out);
    assert!(run.says("110 lines in 3 files"), "{}", run.out);
    assert_eq!(tree.field("commit"), commit, "--report moved the stamp");
    assert_eq!(tree.field("mark"), mark, "--report moved the mark");
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

/// The same history, with a configuration this project's checks can load. It is not committed,
/// so it is not one of the commits the percentile reads.
fn history(small: usize, big: usize) -> Tree {
    let tree = harness::history(small, big);
    tree.write("klin.json", "{\n  \"project\": \"t\"\n}\n");
    tree
}

fn reported(tree: &Tree) -> Run {
    stamped(tree);
    let run = tree.run(&["radius", "--report"]);
    assert_eq!(run.code, 0, "{}", run.out);
    run
}

/// Forty-three small commits, the config commit and six big ones is fifty, so the value at
/// `ceil(0.9 * 50)` is the first of the big ones.
#[test]
fn the_values_are_the_ninetieth_percentile_of_the_last_commits() {
    let run = reported(&history(43, 6));

    assert!(
        run.says("derived: radius lines 30, the 90th percentile of the last 50 non-merge commits"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: radius directories 3, the 90th percentile"),
        "{}",
        run.out
    );
    assert!(
        run.says("usually change about 30 lines under 3 directories."),
        "{}",
        run.out
    );
}

/// The rank is `ceil(0.9 * n)`, so one commit either way moves it past the big commits. Fifty
/// commits with five big ones stops short of them, and fifty-one with six reaches them, which
/// rounding down or interpolating would not.
#[test]
fn the_percentile_is_the_nearest_rank() {
    let short = reported(&history(44, 5));
    assert!(short.says("derived: radius lines 3,"), "{}", short.out);
    assert!(
        short.says("derived: radius directories 1,"),
        "{}",
        short.out
    );

    let long = reported(&history(44, 6));
    assert!(long.says("derived: radius lines 30,"), "{}", long.out);
    assert!(long.says("derived: radius directories 3,"), "{}", long.out);
}

/// A merge commit restates a branch the sample already holds, so it is not one of the fifty.
/// Forty-eight commits, one on a side branch and the merge that brings it back is forty-nine.
#[test]
fn a_merge_commit_is_not_in_the_sample() {
    let tree = history(41, 6);
    tree.git(&["checkout", "-q", "-b", "side"]);
    tree.write("side/f.txt", &lines(100, "line "));
    tree.commit("a side commit");
    tree.git(&["checkout", "-q", "-"]);
    tree.git(&[
        "-c",
        "user.name=klin",
        "-c",
        "user.email=klin@example.com",
        "-c",
        "commit.gpgsign=false",
        "merge",
        "-q",
        "--no-ff",
        "-m",
        "a merge",
        "side",
    ]);

    stamped(&tree);
    let run = tree.run(&["radius", "--report"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("49 non-merge commit(s) reach"), "{}", run.out);
}

#[test]
fn below_fifty_commits_the_report_says_why() {
    let tree = history(42, 6);

    stamped(&tree);
    let run = tree.run(&["radius", "--report"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"radius\" section"), "{}", run.out);
    assert!(run.says("49 non-merge commit(s) reach"), "{}", run.out);
    assert!(run.says("fewer than the 50"), "{}", run.out);
}

#[test]
fn below_fifty_commits_the_hook_says_nothing() {
    let tree = history(42, 6);
    stamped(&tree);
    a_wide_turn(&tree);

    let run = radius(&tree, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("wider than"), "{}", run.out);
    assert!(!run.says("derived:"), "{}", run.out);
}

/// A section the config pins is used as written, and history is never read for it.
#[test]
fn a_pinned_section_names_no_derived_value() {
    let tree = history(43, 6);
    tree.write("klin.json", CONFIG);

    let run = reported(&tree);
    assert!(
        run.says("usually change about 50 lines under 2 directories."),
        "{}",
        run.out
    );
    assert!(run.says("pinned: radius lines 50"), "{}", run.out);
    assert!(run.says("pinned: radius directories 2"), "{}", run.out);
    assert!(!run.says("derived:"), "{}", run.out);
}

/// A key the config pins is used as written beside a key history supplies.
#[test]
fn a_half_pinned_section_derives_the_other_key() {
    let tree = history(43, 6);
    tree.write(
        "klin.json",
        "{\n  \"project\": \"t\",\n  \"radius\": { \"lines\": 7 }\n}\n",
    );

    let run = reported(&tree);
    assert!(
        run.says("usually change about 7 lines under 3 directories."),
        "{}",
        run.out
    );
    assert!(run.says("derived: radius directories 3,"), "{}", run.out);
    assert!(run.says("pinned: radius lines 7"), "{}", run.out);
    assert!(!run.says("derived: radius lines"), "{}", run.out);
}

/// A section klin cannot read two numbers out of says so, rather than deriving both in silence
/// and leaving a person to wonder why the file it names changed nothing. Spec 5.2.
#[test]
fn a_radius_section_that_is_not_an_object_is_a_config_error() {
    let tree = history(43, 6);
    tree.write(
        "klin.json",
        "{\n  \"project\": \"t\",\n  \"radius\": 5\n}\n",
    );

    stamped(&tree);
    let run = tree.run(&["radius", "--report"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"radius\""), "{}", run.out);
    assert!(!run.says("derived:"), "{}", run.out);
}

fn written(file: &std::path::Path, text: &str) {
    assert!(
        std::fs::write(file, text).is_ok(),
        "{} could not be written",
        file.display()
    );
}

fn cache(tree: &Tree) -> std::path::PathBuf {
    let commit = tree.field("parent");
    assert_ne!(commit, "", "the stamp names no parent");
    tree.state(&format!("cache/{commit}.json"))
}

/// The sample is the last two hundred commits, so older ones do not hold the value down or
/// prop it up. The six big commits here are the oldest and fall outside it.
#[test]
fn the_sample_stops_at_two_hundred_commits() {
    let tree = harness::history_from(6, 200);
    tree.write("klin.json", "{\n  \"project\": \"t\"\n}\n");

    let run = reported(&tree);
    assert!(
        run.says("derived: radius lines 1, the 90th percentile of the last 200 non-merge commits"),
        "{}",
        run.out
    );
    assert!(run.says("derived: radius directories 1,"), "{}", run.out);
}

#[test]
fn the_values_are_cached_under_the_derivation_commit() {
    let tree = history(43, 6);
    assert!(reported(&tree).says("derived: radius lines 30,"));
    let file = cache(&tree);
    assert!(file.is_file(), "{} was not written", file.display());

    written(
        &file,
        &format!(
            "{{\"version\":\"{}\",\"radius\":{{\"lines\":4,\"directories\":9,\"commits\":50}}}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    );
    let run = tree.run(&["radius", "--report"]);
    assert!(run.says("derived: radius lines 4,"), "{}", run.out);
}

#[test]
fn a_cache_another_version_wrote_is_derived_again() {
    let tree = history(43, 6);
    assert!(reported(&tree).says("derived: radius lines 30,"));
    let file = cache(&tree);

    written(
        &file,
        "{\"version\":\"0.0.0\",\"radius\":{\"lines\":4,\"directories\":9,\"commits\":50}}\n",
    );
    let run = tree.run(&["radius", "--report"]);
    assert!(run.says("derived: radius lines 30,"), "{}", run.out);
}

/// A commit inside an open turn does not move the derivation commit, so the values hold from
/// the start of a turn to its end. Spec 6.6.
#[test]
fn a_commit_inside_the_turn_does_not_move_the_derivation_commit() {
    let tree = history(43, 6);
    assert!(reported(&tree).says("derived: radius lines 30,"));
    let first = cache(&tree);

    for at in 0..3 {
        tree.write(&format!("d{at}/f.txt"), &lines(200, "line "));
    }
    tree.commit("a commit inside the turn");
    assert_eq!(radius(&tree, A_PROMPT).code, 0);

    assert_eq!(cache(&tree), first, "the derivation commit moved");
}

/// A person moving the stamp takes a fresh one over today's HEAD, so the derivation commit
/// moves with it and the values are read again under the new one. Spec 6.6.
#[test]
fn a_moved_stamp_derives_under_the_commit_it_was_taken_over() {
    let tree = history(43, 6);
    assert!(reported(&tree).says("derived: radius lines 30,"));
    let first = cache(&tree);

    for at in 0..3 {
        tree.write(&format!("d{at}/f.txt"), &lines(200, "line "));
    }
    tree.commit("a commit a person keeps");
    assert_eq!(tree.run(&["turn", "reset"]).code, 0);
    assert_eq!(tree.run(&["radius", "--report"]).code, 0);

    let second = cache(&tree);
    assert_ne!(second, first, "the derivation commit stayed behind");
    assert!(second.is_file(), "{} was not written", second.display());
}
