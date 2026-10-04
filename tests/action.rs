use crate::harness::Tree;
use std::process::Command;

const ACTION: &str = include_str!("../action.yml");

fn install_step() -> String {
    ACTION
        .split("run: |\n")
        .nth(1)
        .unwrap_or_else(|| panic!("no run block in:\n{ACTION}"))
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
        .arg(install_step())
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
