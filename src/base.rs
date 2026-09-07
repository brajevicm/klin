use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::changed::{self, Change, git};
use crate::config::{Config, Error, Flags};

const EMPTY: &str = "0000000000000000000000000000000000000000";

/// The base commit, laid out as a directory, so a gate measures it the way it measures the
/// working tree. A whole run checks the base out in a detached worktree. A scoped run writes
/// only the changed files, each at the path it has today, so a rename is not a tree of new debt.
pub struct Prior {
    dir: PathBuf,
    from_worktree: Option<PathBuf>,
}

impl Prior {
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Prior {
    fn drop(&mut self) {
        match &self.from_worktree {
            Some(root) => {
                git(
                    root,
                    &["worktree", "remove", "--force", &self.dir.to_string_lossy()],
                );
            }
            None => {
                let _ = std::fs::remove_dir_all(&self.dir);
            }
        }
    }
}

pub fn materialize(root: &Path, base: &Base, scope: Option<&[Change]>) -> Result<Prior, Error> {
    let dir = std::env::temp_dir().join(format!(
        "klin-base-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or_default()
    ));
    match scope {
        Some(changes) => written(root, base, changes, dir),
        None => checked_out(root, base, dir),
    }
}

fn checked_out(root: &Path, base: &Base, dir: PathBuf) -> Result<Prior, Error> {
    git(
        root,
        &[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            &dir.to_string_lossy(),
            &base.commit,
        ],
    )
    .ok_or_else(|| {
        Error(format!(
            "the base commit {} could not be checked out to measure it — fetch history, or \
             give CI the full clone",
            &base.commit[..7.min(base.commit.len())]
        ))
    })?;
    let prior = Prior {
        dir,
        from_worktree: Some(root.to_path_buf()),
    };
    for change in changed::files(root, &base.commit)? {
        let Some(was) = change.was.filter(|was| *was != change.path) else {
            continue;
        };
        move_within(prior.dir(), &was, &change.path);
    }
    Ok(prior)
}

fn move_within(dir: &Path, was: &str, now: &str) {
    let (from, to) = (dir.join(was), dir.join(now));
    if let Some(parent) = to.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::rename(from, to);
}

fn written(root: &Path, base: &Base, changes: &[Change], dir: PathBuf) -> Result<Prior, Error> {
    let prior = Prior {
        dir,
        from_worktree: None,
    };
    std::fs::create_dir_all(prior.dir()).map_err(|why| {
        Error(format!(
            "{} could not be written: {why}",
            prior.dir().display()
        ))
    })?;
    for change in changes {
        let Some(was) = &change.was else {
            continue;
        };
        let Some(bytes) = changed::blob(root, &base.commit, was) else {
            continue;
        };
        let path = prior.dir().join(&change.path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, bytes);
    }
    Ok(prior)
}

/// The base tree for a gate the runner did not lay out, such as a gate run by its own command.
pub fn own(config: &Config, flags: &Flags, out: &mut String) -> Result<Prior, Error> {
    let base = choose(config.root())?;
    if !flags.quiet {
        let _ = writeln!(out, "{}", base.line());
    }
    materialize(config.root(), &base, None)
}

/// Where a gate's roots are in the base tree. A root the base does not hold measures nothing.
pub fn roots(roots: &[PathBuf], config: &Config, prior: &Path) -> Vec<PathBuf> {
    roots
        .iter()
        .map(|root| match root.strip_prefix(config.root()) {
            Ok(inside) => prior.join(inside),
            Err(_) => root.to_path_buf(),
        })
        .inspect(|root| {
            let _ = std::fs::create_dir_all(root);
        })
        .collect()
}

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
