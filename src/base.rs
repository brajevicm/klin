use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::changed::{self, Change, git};
use crate::config::{Config, Error, Flags};

const EMPTY: &str = "0000000000000000000000000000000000000000";

/// The base commit, laid out as a directory, so a gate measures it the way it measures the
/// working tree. A whole run checks the base out in a detached worktree. A scoped run writes
/// only the changed files, each at the path it has today, so a rename is not a tree of new debt.
pub struct Prior {
    dir: tempfile::TempDir,
    /// Where the configuration's own directory sits inside that tree.
    root: PathBuf,
    from_worktree: Option<PathBuf>,
}

impl Prior {
    /// The base's copy of the directory the configuration sits in, which paths are relative to.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Prior {
    fn drop(&mut self) {
        let Some(repository) = &self.from_worktree else {
            return;
        };
        git(
            repository,
            &[
                "worktree",
                "remove",
                "--force",
                &self.dir.path().to_string_lossy(),
            ],
        );
        git(repository, &["worktree", "prune"]);
    }
}

pub fn materialize(config: &Config, base: &Base, scope: Option<&[Change]>) -> Result<Prior, Error> {
    let dir = tempfile::Builder::new()
        .prefix("klin-base-")
        .tempdir()
        .map_err(|why| Error(format!("a directory for the base could not be made: {why}")))?;
    match scope {
        Some(changes) => written(config, base, changes, dir),
        None => checked_out(config, base, dir),
    }
}

fn checked_out(config: &Config, base: &Base, dir: tempfile::TempDir) -> Result<Prior, Error> {
    let root = config.root();
    let inside = under_the_repository(root)?;
    git(
        root,
        &[
            "worktree",
            "add",
            "--detach",
            "--quiet",
            &dir.path().to_string_lossy(),
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
        root: dir.path().join(inside),
        dir,
        from_worktree: Some(root.to_path_buf()),
    };
    for change in changed::files(root, &base.commit)? {
        let Some(was) = change.was.filter(|was| *was != change.path) else {
            continue;
        };
        move_within(prior.root(), &was, &change.path)?;
    }
    Ok(prior)
}

/// Where the configuration sits inside the repository, since a worktree holds the whole tree.
fn under_the_repository(root: &Path) -> Result<PathBuf, Error> {
    let named = git(root, &["rev-parse", "--show-toplevel"])
        .map(|found| PathBuf::from(found.trim()))
        .ok_or_else(|| {
            Error(format!(
                "{} is not inside a git repository, and klin measures the working tree against a \
                 base commit",
                root.display()
            ))
        })?;
    let top = named.canonicalize().unwrap_or(named);
    let here = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    Ok(here
        .strip_prefix(&top)
        .unwrap_or(Path::new(""))
        .to_path_buf())
}

fn move_within(root: &Path, was: &str, now: &str) -> Result<(), Error> {
    let (from, to) = (root.join(was), root.join(now));
    if !from.is_file() {
        return Ok(());
    }
    let moved = || Error(format!("the base's {was} could not be read at {now}"));
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|_| moved())?;
    }
    std::fs::rename(from, to).map_err(|_| moved())
}

fn written(
    config: &Config,
    base: &Base,
    changes: &[Change],
    dir: tempfile::TempDir,
) -> Result<Prior, Error> {
    let root = config.root();
    let prior = Prior {
        root: dir.path().to_path_buf(),
        dir,
        from_worktree: None,
    };
    for change in changes {
        let Some(was) = &change.was else {
            continue;
        };
        let bytes = changed::blob(root, &base.commit, was).ok_or_else(|| {
            Error(format!(
                "the base commit {} holds no {was}, which git says it changed — the base and the \
                 working tree disagree, so klin cannot judge this run",
                &base.commit[..7.min(base.commit.len())]
            ))
        })?;
        let path = prior.root().join(&change.path);
        let unwritable = |why: &dyn std::fmt::Display| {
            Error(format!("{} could not be written: {why}", path.display()))
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|why| unwritable(&why))?;
        }
        std::fs::write(&path, bytes).map_err(|why| unwritable(&why))?;
    }
    Ok(prior)
}

/// The base tree for a gate the runner did not lay out, such as a gate run by its own command.
pub fn own(config: &Config, flags: &Flags, out: &mut String) -> Result<Prior, Error> {
    let base = choose(config.root())?;
    if !flags.quiet {
        let _ = writeln!(out, "{}", base.line());
    }
    materialize(config, &base, None)
}

/// Where a gate's roots are in the base tree. A root the base does not hold measures nothing.
pub fn roots(roots: &[PathBuf], config: &Config, prior: &Path) -> Result<Vec<PathBuf>, Error> {
    roots
        .iter()
        .map(|root| {
            let inside = root.strip_prefix(config.root()).map_err(|_| {
                Error(format!(
                    "{}: the root {} is outside the tree klin compares, so no base of it exists \
                     — name a root under {}",
                    config.file.display(),
                    root.display(),
                    config.root().display()
                ))
            })?;
            let at = prior.join(inside);
            let _ = std::fs::create_dir_all(&at);
            Ok(at)
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
