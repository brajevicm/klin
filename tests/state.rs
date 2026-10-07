mod harness;

use std::path::Path;
use std::process::Command;

use harness::{Run, Tree};

const GATES: &str = r#""doc_size": {"README.md": 10}"#;
const CLEAN: &str = "fn simple(a: i32) -> i32 {\n    a + 1\n}\n";
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;

fn tree(build: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", &format!("{{\n  {build}\n  {GATES}\n}}"));
    tree.words("README.md", 5);
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

fn state_line(run: &Run) -> String {
    run.out
        .lines()
        .find(|line| line.starts_with("state: "))
        .unwrap_or_default()
        .to_string()
}

fn git(cwd: &Path, args: &[&str]) {
    let done = Command::new("git").arg("-C").arg(cwd).args(args).output();
    match done {
        Ok(done) => assert!(
            done.status.success(),
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&done.stderr)
        ),
        Err(why) => panic!("git {} could not run: {why}", args.join(" ")),
    }
}

#[test]
fn list_prints_the_state_directory_under_the_git_directory() {
    let tree = tree("");

    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(state_line(&run).ends_with("/.git/klin"), "{}", run.out);
}

#[test]
fn the_build_stamp_lands_under_the_git_directory_where_git_never_sees_it() {
    let tree = tree("\"build\": \"false\",");

    let run = harness::feed(tree.root(), &["gate", "--hook"], A_STOP);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(tree.state("build-blocked").is_file(), "{}", run.out);
    assert_eq!(tree.status(), "", "the run touched the working tree");

    tree.git(&["clean", "-fdx"]);
    assert!(tree.state("build-blocked").is_file());
}

#[test]
fn two_clones_of_one_repository_get_two_state_directories() {
    let tree = tree("");
    let clone = Tree::bare();
    let cache = Tree::bare();
    git(tree.root(), &["clone", "-q", ".", &clone.at("")]);
    let under = cache.root().display().to_string();
    let environment = [("KLIN_STATE_DIR", under.as_str())];

    let mine = state_line(&tree.run_with(&environment, &["policy"]));
    let theirs = state_line(&harness::run_from_with(
        clone.root(),
        &environment,
        &["policy"],
    ));
    assert!(mine.contains(&under), "{mine}");
    assert!(theirs.contains(&under), "{theirs}");
    assert_ne!(mine, theirs);
}

#[test]
fn a_git_worktree_gets_its_own_state_directory() {
    let tree = tree("");
    let beside = Tree::bare();
    let at = beside.path("side");
    git(
        tree.root(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            &at.display().to_string(),
        ],
    );

    let mine = state_line(&tree.run(&["policy"]));
    let theirs = state_line(&harness::run_from(&at, &["policy"]));
    assert!(theirs.ends_with("/klin"), "{theirs}");
    assert_ne!(mine, theirs);
}

#[test]
fn cache_clean_removes_the_survey_cache_and_leaves_the_stamps() {
    let tree = tree("");
    tree.write(".git/klin/cache/a-tree", "measured");
    tree.write(".git/klin/build-blocked", "");

    let run = tree.run(&["cache", "clean"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!tree.state("cache").exists(), "{}", run.out);
    assert!(tree.state("build-blocked").is_file(), "{}", run.out);
}

#[test]
fn cache_clean_all_removes_only_the_cache_of_a_repository_that_is_gone() {
    let tree = tree("");
    let cache = Tree::bare();
    let under = cache.root().display().to_string();
    let environment = [("KLIN_STATE_DIR", under.as_str())];
    let stamped = tree.run_with(&environment, &["gate", "--hook"]);
    assert_eq!(stamped.code, 0, "{}", stamped.out);
    let mine = state_line(&tree.run_with(&environment, &["policy"]));
    let mine = Path::new(mine.trim_start_matches("state: ")).to_path_buf();
    let stamp = mine.join("build-blocked");
    assert!(std::fs::write(&stamp, "").is_ok(), "a stamp of my own");
    let held = cache.write("0000000000000000/cache/a-tree", "measured");
    cache.write("0000000000000000/repository", &cache.at("gone"));
    let orphan = cache.write("0000000000000000/build-blocked", "");

    let run = tree.run_with(&environment, &["cache", "clean", "--all"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("removed 1 cache"), "{}", run.out);
    assert!(!held.exists(), "{}", run.out);
    assert!(
        orphan.is_file(),
        "the stamp of a tree klin cannot see went too"
    );
    assert!(stamp.is_file(), "{}", run.out);
}

#[test]
fn an_unwritable_state_directory_says_why_and_blocks_nothing() {
    let tree = tree("");
    let file = tree.at("a-file");
    tree.write("a-file", "");

    let run = tree.run_with(&[("KLIN_STATE_DIR", file.as_str())], &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("wrote no verdict"), "{}", run.out);
}

#[test]
fn init_says_an_older_ignore_line_is_inert_and_leaves_it_alone() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    tree.words("README.md", 5);
    tree.write(".gitignore", ".klin/\n");

    let run = tree.run(&["setup"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("the .klin line is inert"), "{}", run.out);
    let held = std::fs::read_to_string(tree.path(".gitignore")).unwrap_or_default();
    assert_eq!(held, ".klin/\n");
}

#[test]
fn a_build_failure_under_an_unwritable_state_directory_says_why_and_blocks_nothing() {
    let tree = tree("\"build\": \"false\",");
    tree.write("a-file", "");
    let file = tree.at("a-file");

    let run = tree.run_with(&[("KLIN_STATE_DIR", file.as_str())], &["gate", "--hook"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("does not build"), "{}", run.out);
    assert!(run.says("NOTE"), "{}", run.out);
    assert!(run.says("blocks nothing"), "{}", run.out);
}

#[test]
fn a_tree_that_is_no_repository_has_no_state_even_under_the_override() {
    let tree = Tree::bare();
    let cache = Tree::bare();
    let under = cache.root().display().to_string();

    let run = tree.run_with(&[("KLIN_STATE_DIR", under.as_str())], &["cache", "clean"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("no git repository here"), "{}", run.out);
}

#[test]
fn the_override_keeps_the_build_stamp_in_the_directory_it_names() {
    let tree = tree("\"build\": \"false\",");
    let cache = Tree::bare();
    let under = cache.root().display().to_string();
    let environment = [("KLIN_STATE_DIR", under.as_str())];

    let run = tree.run_with(&environment, &["gate", "--hook"]);
    assert_eq!(run.code, 2, "{}", run.out);
    let mine = state_line(&tree.run_with(&environment, &["policy"]));
    let mine = Path::new(mine.trim_start_matches("state: "));
    assert!(mine.starts_with(&under), "{}", mine.display());
    assert!(mine.join("build-blocked").is_file(), "{}", run.out);
    assert!(!tree.state("build-blocked").exists(), "{}", run.out);
}

#[test]
fn a_stop_that_cannot_take_the_state_directory_says_so_and_writes_no_verdict() {
    let tree = tree("");
    let first = harness::feed(tree.root(), &["gate", "--hook"], A_STOP);
    assert_eq!(first.code, 0, "{}", first.out);
    assert_eq!(tree.field("verdict"), "green", "{}", first.out);

    let _lock = held_lock(&tree);
    tree.words("README.md", 30);

    let held = harness::feed(tree.root(), &["gate", "--hook"], A_STOP);
    assert_eq!(held.code, 0, "{}", held.out);
    assert!(held.says("held the state directory"), "{}", held.out);
    assert!(held.says("FAIL  doc-size"), "{}", held.out);
    assert!(!held.says("gate block 1 of 2"), "{}", held.out);
    assert_eq!(tree.field("verdict"), "green", "{}", held.out);
    let record = std::fs::read_to_string(tree.state("build-blocked")).unwrap_or_default();
    assert!(!record.contains(r#""gate_blocks":1"#), "{record}");
}

#[test]
fn a_build_failure_at_a_stop_that_cannot_take_the_state_directory_spends_no_block() {
    let tree = tree("\"build\": \"test ! -f fails\",");
    let first = harness::feed(tree.root(), &["gate", "--hook"], A_STOP);
    assert_eq!(first.code, 0, "{}", first.out);

    let _lock = held_lock(&tree);
    tree.write("fails", "");
    let held = harness::feed(tree.root(), &["gate", "--hook"], A_STOP);
    assert_eq!(held.code, 0, "{}", held.out);
    assert!(held.says("does not build"), "{}", held.out);
    assert!(held.says("held the state directory"), "{}", held.out);
    let record = std::fs::read_to_string(tree.state("build-blocked")).unwrap_or_default();
    assert!(!record.contains(r#""builds":1"#), "{record}");
}

/// The advisory lock a stop takes over the state directory, held by the test past the hook's
/// budget, so the stop under test runs as one that lost it.
fn held_lock(tree: &Tree) -> std::fs::File {
    let opened = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(tree.state("lock"));
    let Ok(lock) = opened else {
        panic!("the lock file could not be opened");
    };
    assert!(lock.lock().is_ok(), "another holder has the lock");
    lock
}
