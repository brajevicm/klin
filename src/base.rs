use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::changed::{self, Change, git};
use crate::config::{Config, Error, Flags};

const EMPTY: &str = "0000000000000000000000000000000000000000";

/// The `before` tree, laid out as a directory, so a gate measures it the way it measures the
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

pub fn materialize(
    config: &Config,
    base: &Window,
    scope: Option<&[Change]>,
) -> Result<Prior, Error> {
    let dir = tempfile::Builder::new()
        .prefix("klin-base-")
        .tempdir()
        .map_err(|why| Error(format!("a directory for the base could not be made: {why}")))?;
    match scope {
        Some(changes) => written(config, base, changes, dir),
        None => checked_out(config, base, dir),
    }
}

fn checked_out(config: &Config, base: &Window, dir: tempfile::TempDir) -> Result<Prior, Error> {
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
            &base.before,
        ],
    )
    .ok_or_else(|| {
        Error(format!(
            "the base commit {} could not be checked out to measure it — fetch history, or \
             give CI the full clone",
            base.short()
        ))
    })?;
    let prior = Prior {
        root: dir.path().join(inside),
        dir,
        from_worktree: Some(root.to_path_buf()),
    };
    for change in changed::files(root, &base.before)? {
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
    base: &Window,
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
        let bytes = changed::blob(root, &base.before, was).ok_or_else(|| {
            Error(format!(
                "the base commit {} holds no {was}, which git says it changed — the base and the \
                 working tree disagree, so klin cannot judge this run",
                base.short()
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

/// The base commit a gate judges against: the one the runner chose, or the one this gate
/// chooses for itself and names once in the report. Spec 6.1.
pub fn commit(root: &Path, flags: &Flags, out: &mut String) -> Result<String, Error> {
    match &flags.base {
        Some(commit) => Ok(commit.clone()),
        None => Ok(announced(root, flags, out)?.before),
    }
}

/// The base a gate the runner did not lay out chooses for itself, named once in the report.
pub fn announced(root: &Path, flags: &Flags, out: &mut String) -> Result<Window, Error> {
    let base = choose(root, flags.strict)?;
    if !flags.quiet {
        let _ = writeln!(out, "{}", base.line());
    }
    Ok(base)
}

/// The base tree for a gate the runner did not lay out, such as a gate run by its own command.
pub fn own(config: &Config, flags: &Flags, out: &mut String) -> Result<Prior, Error> {
    let base = announced(config.root(), flags, out)?;
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

/// The pair of trees a run compares: which of the three kinds of 4.2 it is, the base commit
/// `before` names, and how that commit was chosen. `after` is the working tree.
#[derive(Clone)]
pub struct Window {
    pub kind: Kind,
    pub before: String,
    pub how: String,
}

/// The three window kinds of section 4.2. The hook judges a turn, `klin gate` by hand and CI
/// on a pull request judge a branch, and CI on a push judges the push.
#[derive(Clone, Copy)]
pub enum Kind {
    Turn,
    Branch,
    Push,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Turn => "turn",
            Kind::Branch => "branch",
            Kind::Push => "push",
        }
    }
}

impl Window {
    pub fn short(&self) -> &str {
        &self.before[..7.min(self.before.len())]
    }

    pub fn line(&self) -> String {
        format!(
            "window: {} — base {}, {}",
            self.kind.name(),
            self.short(),
            self.how
        )
    }

    pub fn record(&self) -> Value {
        let mut out = Map::new();
        out.insert("kind".into(), self.kind.name().into());
        out.insert("before".into(), self.before.clone().into());
        out.insert("after".into(), "the working tree".into());
        out.insert("how".into(), self.how.clone().into());
        Value::Object(out)
    }
}

pub fn choose(root: &Path, strict: bool) -> Result<Window, Error> {
    let head = resolve(root, "HEAD");
    let mut tried: Vec<String> = Vec::new();
    for (reference, how, kind, source) in candidates(root) {
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
        let base = Window {
            kind,
            before: commit,
            how,
        };
        if head.as_deref() == Some(base.before.as_str()) && !dirty(root) {
            return equal_to_head(root, strict, source, base);
        }
        return Ok(base);
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

/// Where a base candidate came from, which decides whether a base equal to HEAD hides work.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// A remote reference or a push event's commit. The remote agrees with HEAD.
    Remote,
    /// A local reference, which says nothing about what the remote has.
    Local,
}

/// A clean working tree whose base is HEAD measures nothing. That is a refusal only when klin
/// can show that commits are hidden from the remote, or when it cannot tell and CI is asking.
fn equal_to_head(root: &Path, strict: bool, source: Source, base: Window) -> Result<Window, Error> {
    if source == Source::Remote {
        return Ok(base);
    }
    let Some((name, tip)) = remote_tip(root) else {
        return cannot_tell(strict, base, "no remote default branch resolves");
    };
    let Some(unpushed) = git(
        root,
        &[
            "log",
            "--oneline",
            "-n",
            &SHOWN.to_string(),
            &format!("{tip}..HEAD"),
        ],
    ) else {
        return cannot_tell(
            strict,
            base,
            &format!("git could not list what {name} is missing"),
        );
    };
    let unpushed: Vec<&str> = unpushed.lines().filter(|line| !line.is_empty()).collect();
    if unpushed.is_empty() {
        return Ok(base);
    }
    Err(Error(format!(
        "the base is HEAD ({}) and the working tree matches it, so a run would measure nothing, \
         and {} on this branch that {name} does not hold:\n{}\nPush this branch, or fetch the \
         remote reference the base names, so a run has a real diff to measure.",
        base.how,
        match unpushed.len() < SHOWN {
            true => format!("these {} commit(s) sit", unpushed.len()),
            false => format!("at least these {SHOWN} commits sit"),
        },
        unpushed
            .iter()
            .map(|line| format!("  {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    )))
}

/// How many unpushed commits an error names, so a long-lived branch is not a wall of text.
const SHOWN: usize = 10;

/// klin cannot see whether the work is pushed. CI asks for the refusal, a local run for its gates.
fn cannot_tell(strict: bool, base: Window, why: &str) -> Result<Window, Error> {
    if !strict {
        return Ok(base);
    }
    Err(Error(format!(
        "the base is HEAD ({}) and the working tree matches it, so a run would measure nothing, \
         and {why}, so klin cannot tell whether these commits are pushed — fetch the remote, or \
         drop --strict",
        base.how
    )))
}

/// The tip of the remote's default branch, which says what the remote already has.
fn remote_tip(root: &Path) -> Option<(String, String)> {
    default_branches(root).into_iter().find_map(|branch| {
        let commit = resolve(root, &remote_reference(&branch)?)?;
        Some((branch, commit))
    })
}

/// The remote-tracking reference that a name such as `origin/main` abbreviates. A local branch
/// may carry that literal name, and git resolves it first, so klin never calls a base remote
/// on the strength of the short name alone.
fn remote_reference(branch: &str) -> Option<String> {
    branch
        .strip_prefix("origin/")
        .map(|remote| format!("refs/remotes/origin/{remote}"))
}

fn dirty(root: &Path) -> bool {
    git(root, &["status", "--porcelain"]).is_some_and(|listed| !listed.trim().is_empty())
}

fn candidates(root: &Path) -> Vec<(String, String, Kind, Source)> {
    let mut out = Vec::new();
    if let Some(target) = environment("GITHUB_BASE_REF") {
        for (reference, source) in [
            (format!("refs/remotes/origin/{target}"), Source::Remote),
            (target.clone(), Source::Local),
        ] {
            out.push((
                reference,
                format!("the tip of {target}, the pull request target"),
                Kind::Branch,
                source,
            ));
        }
    }
    if let Some(before) = pushed_from() {
        out.push((
            before,
            "the commit this push started from".to_string(),
            Kind::Push,
            Source::Remote,
        ));
    }
    for branch in default_branches(root) {
        let (reference, source) = match remote_reference(&branch) {
            Some(remote) => (remote, Source::Remote),
            None => (branch.clone(), Source::Local),
        };
        out.push((
            format!("{reference}...HEAD"),
            format!("the merge-base with {branch}"),
            Kind::Branch,
            source,
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
