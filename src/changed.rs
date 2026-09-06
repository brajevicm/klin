use std::path::Path;
use std::process::Command;

use crate::config::Error;

pub fn files(root: &Path) -> Result<Vec<String>, Error> {
    let base = base(root);
    let diffed = diff(root, Some(&base))
        .or_else(|| diff(root, None))
        .ok_or_else(|| {
            Error(format!(
                "--changed needs a git repository, and git could not read {}",
                root.display()
            ))
        })?;
    let untracked = git(root, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default();
    let mut names: Vec<String> = diffed
        .lines()
        .chain(untracked.lines())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

fn diff(root: &Path, base: Option<&str>) -> Option<String> {
    let mut args = vec!["diff", "--name-only", "--diff-filter=d", "--relative"];
    if let Some(base) = base {
        args.push(base);
    }
    args.push("--");
    git(root, &args)
}

fn base(root: &Path) -> String {
    if let Some(requested) = std::env::var("GITHUB_BASE_REF")
        .ok()
        .filter(|name| !name.is_empty())
    {
        let pull_request = format!("origin/{requested}");
        if exists(root, &pull_request) {
            return pull_request;
        }
    }
    for candidate in default_branches(root) {
        if !exists(root, &candidate) {
            continue;
        }
        let found = git(root, &["merge-base", &candidate, "HEAD"]).unwrap_or_default();
        let found = found.trim();
        if !found.is_empty() {
            return found.to_string();
        }
    }
    "HEAD".to_string()
}

fn default_branches(root: &Path) -> Vec<String> {
    let named = git(
        root,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .map(|found| found.trim().to_string())
    .filter(|found| !found.is_empty());
    named
        .into_iter()
        .chain(["origin/main", "origin/master", "main", "master"].map(String::from))
        .collect()
}

fn exists(root: &Path, reference: &str) -> bool {
    git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
    )
    .is_some()
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let done = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("-c")
        .arg("core.quotePath=false")
        .args(args)
        .output()
        .ok()?;
    done.status
        .success()
        .then(|| String::from_utf8_lossy(&done.stdout).into_owned())
}
