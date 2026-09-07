use std::path::Path;

use serde_json::Value;

use crate::changed::git;
use crate::config::Error;

const EMPTY: &str = "0000000000000000000000000000000000000000";

pub struct Base {
    pub commit: String,
    pub how: String,
}

impl Base {
    pub fn line(&self) -> String {
        format!(
            "base: {} — {}",
            &self.commit[..7.min(self.commit.len())],
            self.how
        )
    }
}

pub fn choose(root: &Path) -> Result<Base, Error> {
    let head = resolve(root, "HEAD");
    let mut tried: Vec<String> = Vec::new();
    for (reference, how) in candidates(root) {
        tried.push(
            reference
                .split("...")
                .next()
                .unwrap_or(&reference)
                .to_string(),
        );
        let Some(commit) = resolve(root, &reference) else {
            continue;
        };
        if head.as_deref() == Some(commit.as_str()) && !dirty(root) {
            return Err(Error(format!(
                "the base is HEAD ({how}) and the working tree matches it, so a run would \
                 measure nothing — work on a branch, or let CI name the base in its event data"
            )));
        }
        return Ok(Base { commit, how });
    }
    Err(Error(format!(
        "no base commit to compare against in {} — klin measures the working tree against a \
         base commit, and none of these resolved: {}. Fetch history, or work on a branch.",
        root.display(),
        match tried.is_empty() {
            true => "nothing".to_string(),
            false => tried.join(", "),
        }
    )))
}

fn dirty(root: &Path) -> bool {
    git(root, &["status", "--porcelain"]).is_some_and(|listed| !listed.trim().is_empty())
}

fn candidates(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(target) = environment("GITHUB_BASE_REF") {
        for reference in [format!("origin/{target}"), target.clone()] {
            out.push((
                reference,
                format!("the tip of {target}, the pull request target"),
            ));
        }
    }
    if let Some(before) = pushed_from() {
        out.push((before, "the commit this push started from".to_string()));
    }
    for branch in default_branches(root) {
        out.push((
            format!("{branch}...HEAD"),
            format!("the merge-base with {branch}"),
        ));
    }
    out
}

fn pushed_from() -> Option<String> {
    let path = environment("GITHUB_EVENT_PATH")?;
    let text = std::fs::read_to_string(path).ok()?;
    let event: Value = serde_json::from_str(&text).ok()?;
    let before = event.get("before")?.as_str()?;
    (before != EMPTY && !before.is_empty()).then(|| before.to_string())
}

fn environment(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
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

fn resolve(root: &Path, reference: &str) -> Option<String> {
    let found = match reference.split_once("...") {
        Some((branch, head)) => git(root, &["merge-base", branch, head])?,
        None => git(
            root,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{reference}^{{commit}}"),
            ],
        )?,
    };
    let found = found.trim().to_string();
    (!found.is_empty()).then_some(found)
}
