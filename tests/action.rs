mod harness;

use harness::Tree;
use std::process::Command;

const ACTION: &str = include_str!("../action.yml");

/// The script of the Action's run block `index`, counted from 1.
fn step(index: usize) -> String {
    ACTION
        .split("run: |\n")
        .nth(index)
        .unwrap_or_else(|| panic!("no run block {index} in:\n{ACTION}"))
        .lines()
        .take_while(|line| line.starts_with("        ") || line.is_empty())
        .map(|line| line.trim_start())
        .collect::<Vec<_>>()
        .join("\n")
}

fn installed_from(input: &str, action_ref: &str) -> String {
    let tree = Tree::bare();
    tree.write("klin.json", r#"{ "version": "9.9.9" }"#);
    tree.write(
        "bin/curl",
        "#!/bin/sh\nfor url; do :; done\necho \"$url\" > \"$RUNNER_TEMP/url\"\n\
         echo 'mkdir -p \"$KLIN_INSTALL_DIR/bin\" && touch \"$KLIN_INSTALL_DIR/bin/klin\"'\n",
    );
    let curl = tree.path("bin/curl");
    Command::new("chmod").arg("+x").arg(&curl).status().unwrap();
    let runner = tree.path("runner");
    std::fs::create_dir_all(&runner).unwrap();
    let path = format!(
        "{}:{}",
        tree.path("bin").display(),
        std::env::var("PATH").unwrap()
    );
    let run = Command::new("bash")
        .arg("-c")
        .arg(step(1))
        .current_dir(tree.root())
        .env("PATH", path)
        .env("INPUT_VERSION", input)
        .env("ACTION_REF", action_ref)
        .env("RUNNER_TEMP", &runner)
        .env("GITHUB_PATH", runner.join("path"))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(runner.join("url"))
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn the_action_installs_the_version_its_input_names() {
    assert_eq!(
        installed_from("1.0.0", "v0.3.0"),
        "https://github.com/brajevicm/klin/releases/download/v1.0.0/klin-installer.sh"
    );
}

#[test]
fn the_action_installs_the_tag_it_is_pinned_at_and_never_reads_klin_json() {
    assert_eq!(
        installed_from("", "v0.3.0"),
        "https://github.com/brajevicm/klin/releases/download/v0.3.0/klin-installer.sh"
    );
}

#[test]
fn the_action_installs_the_latest_release_when_it_is_pinned_at_a_branch() {
    assert_eq!(
        installed_from("", "main"),
        "https://github.com/brajevicm/klin/releases/latest/download/klin-installer.sh"
    );
}

const CLEAN: &str = "pub fn one(a: i32) -> i32 {\n    a + 1\n}\n";
const BROKEN: &str = "pub fn one(a: i32 -> i32 {\n    a + 1\n}\n";
const COMPLEX: &str = "(x: i32) -> i32 { if x > 1 { if x > 2 { if x > 3 { if x > 4 { if x > 5 { \
    if x > 6 { if x > 7 { if x > 8 { if x > 9 { if x > 10 { 1 } else { 2 } } else { 3 } } \
    else { 4 } } else { 5 } } else { 6 } } else { 7 } } else { 8 } } else { 9 } } else { 10 } \
    } else { 11 } }\n";

/// What the Action's gate step did in `tree` with `args`.
struct Gated {
    code: i32,
    printed: String,
    summary: String,
}

impl Gated {
    fn annotations(&self, level: &str) -> Vec<&str> {
        self.printed
            .lines()
            .filter(|line| line.starts_with(&format!("::{level} ")))
            .collect()
    }
}

fn gated(tree: &Tree, args: &str) -> Gated {
    let runner = tree.path(".runner");
    std::fs::create_dir_all(&runner).unwrap();
    let klin = std::path::PathBuf::from(harness::binary());
    let path = format!(
        "{}:{}",
        klin.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );
    let run = Command::new("bash")
        .arg("-c")
        .arg(step(2))
        .current_dir(tree.root())
        .env("PATH", path)
        .env("HOME", harness::empty_home())
        .env("ARGS", args)
        .env("RUNNER_TEMP", &runner)
        .env("GITHUB_ACTION_PATH", env!("CARGO_MANIFEST_DIR"))
        .env("GITHUB_STEP_SUMMARY", runner.join("summary"))
        .output()
        .unwrap();
    Gated {
        code: run.status.code().unwrap(),
        printed: format!(
            "{}{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        ),
        summary: std::fs::read_to_string(runner.join("summary")).unwrap_or_default(),
    }
}

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(".gitignore", ".runner/\n");
    tree.write("src/lib.rs", CLEAN);
    tree.base();
    tree
}

#[test]
fn the_action_fails_the_job_with_the_exit_of_klin_check() {
    let tree = tree();
    tree.write("src/lib.rs", BROKEN);

    let failed = gated(&tree, "");
    let refused = gated(&tree, "--strict");

    assert_eq!(failed.code, 1, "{}", failed.printed);
    assert_eq!(
        failed.annotations("error"),
        [
            "::error file=src/lib.rs,line=1,title=klin%3A measurement-lost::src/lib.rs was measured \
          at the base and klin cannot measure it now: the Rust grammar finds an error at line 1, \
          column 18, so nothing in it is judged. Make the file valid Rust again from line 1, \
          column 18."
        ],
        "{}",
        failed.printed
    );
    assert!(failed.summary.contains("exit 1"), "{}", failed.summary);
    assert_eq!(refused.code, 2, "{}", refused.printed);
    assert!(refused.printed.contains("--strict"), "{}", refused.printed);
}

#[test]
fn a_green_job_still_annotates_its_review_items() {
    let tree = tree();
    tree.write("src/new.rs", BROKEN);

    let gated = gated(&tree, "");

    assert_eq!(gated.code, 0, "{}", gated.printed);
    assert!(gated.annotations("error").is_empty(), "{}", gated.printed);
    assert_eq!(
        gated.annotations("warning"),
        [
            "::warning file=src/new.rs,title=klin review%3A unmeasured::the Rust grammar finds an \
          error at line 1, column 18 (unreadable)"
        ],
        "{}",
        gated.printed
    );
    assert!(
        gated.summary.contains("### Review items (1)"),
        "{}",
        gated.summary
    );
    assert!(
        gated.summary.contains("Files not measured: 1"),
        "{}",
        gated.summary
    );
}

#[test]
fn the_action_annotates_failing_findings_before_review_items_and_counts_the_rest() {
    let tree = tree();
    let complex: String = (0..12).map(|at| format!("pub fn f{at}{COMPLEX}")).collect();
    tree.write("src/complex.rs", &complex);
    for at in 0..12 {
        tree.write(&format!("src/broken{at}.rs"), BROKEN);
    }

    let gated = gated(&tree, "complexity");

    assert_eq!(gated.code, 1, "{}", gated.printed);
    let levels: Vec<&str> = gated
        .printed
        .lines()
        .filter_map(|line| line.strip_prefix("::"))
        .filter_map(|line| line.split(' ').next())
        .collect();
    assert_eq!(
        levels,
        [["error"; 10], ["warning"; 10]].concat(),
        "{}",
        gated.printed
    );
    assert!(
        gated.summary.contains("### Failing findings (12)"),
        "{}",
        gated.summary
    );
    assert!(
        gated.summary.contains("### Review items (12)"),
        "{}",
        gated.summary
    );
    assert!(
        gated
            .summary
            .contains("Not annotated: 2 failing finding(s) and 2 review item(s)"),
        "{}",
        gated.summary
    );
}
