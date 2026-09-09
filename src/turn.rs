use std::fmt::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::state;

/// The stamp file in the state directory, and the name it is written under before the rename,
/// so a hook that dies mid-write leaves the previous stamp rather than a torn one. Spec 6.5.
const FILE: &str = "turn";
const WRITING: &str = "turn.writing";
/// The index the stamp is built in, apart from the one a person's `git add` writes.
const INDEX: &str = "index";
/// Git shares `refs/` across the worktrees of one repository, and `refs/worktree/` is one of
/// the exceptions, so each worktree keeps its own stamp. The ref is never pushed. Spec 6.5.
const REFERENCE: &str = "refs/worktree/klin/turn";

#[derive(clap::Args)]
pub struct Args {}

/// Where the turn's window opens: the stamped commit, the HEAD it was taken over, when it was
/// taken, the verdict of the last stop, and how many prompts this worktree has seen.
pub struct Stamp {
    pub commit: Option<String>,
    pub parent: Option<String>,
    pub time: u64,
    pub green: bool,
    pub prompts: u64,
}

pub fn run(_args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if state::dir(start).is_none() {
        return Ok(0);
    }
    let at = match state::ready(start) {
        Ok(at) => at,
        Err(why) => {
            note(out, &format!("{why}, so this turn has no stamp"));
            return Ok(0);
        }
    };
    let never = !at.join(INDEX).exists();
    let held = read(&at);
    let prompts = held.as_ref().map_or(0, |held| held.prompts) + 1;
    if let Some(stamp) = next(start, &at, never, held, prompts, out) {
        write(&at, &stamp, out);
    }
    Ok(0)
}

/// One rule, on a session start and on a prompt alike: the stamp moves on a first session or
/// after a green stop, and otherwise stays. The counter rises either way. Spec 6.2. A stamp
/// klin never took is a first session, whatever else the state directory holds, because other
/// commands write there too.
fn next(
    root: &Path,
    at: &Path,
    never: bool,
    held: Option<Stamp>,
    prompts: u64,
    out: &mut String,
) -> Option<Stamp> {
    let Some(held) = held else {
        return restored(root, at, never, prompts, out);
    };
    if held.green {
        return taken(root, at, prompts, out).or(Some(Stamp { prompts, ..held }));
    }
    if held.commit.is_none() {
        note(out, GONE);
    }
    Some(Stamp { prompts, ..held })
}

fn read(at: &Path) -> Option<Stamp> {
    let text = std::fs::read_to_string(at.join(FILE)).ok()?;
    let held: Value = serde_json::from_str(&text).ok()?;
    let text = |key: &str| {
        held.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|found| !found.is_empty())
    };
    Some(Stamp {
        commit: text("commit"),
        parent: text("parent"),
        time: held.get("time").and_then(Value::as_u64).unwrap_or_default(),
        green: text("verdict").as_deref() == Some("green"),
        prompts: held
            .get("prompts")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
    })
}

/// A fresh stamp, or `None` when git could not take one, so the caller keeps the stamp it has
/// and the next prompt tries again.
fn taken(root: &Path, at: &Path, prompts: u64, out: &mut String) -> Option<Stamp> {
    let Some((commit, parent)) = stamped(root, at) else {
        note(
            out,
            "git could not stamp this tree, so the stamp klin already had still stands",
        );
        return None;
    };
    Some(Stamp {
        commit: Some(commit),
        parent,
        time: now(),
        green: false,
        prompts,
    })
}

/// A `turn` file that is gone. The ref is the recovery copy, and a stamp restored from it is
/// red, so the next stop judges everything since it. With the ref gone too the stamp was
/// deleted, and a fresh stamp would photograph whatever the deletion hid. Spec 6.2.
fn restored(root: &Path, at: &Path, never: bool, prompts: u64, out: &mut String) -> Option<Stamp> {
    if let Some(commit) = resolve(root, REFERENCE) {
        note(
            out,
            "the turn file was gone and the ref still held the stamp, so klin restored it with \
             a red verdict",
        );
        return Some(Stamp {
            parent: resolve(root, &format!("{commit}^")),
            commit: Some(commit),
            time: now(),
            green: false,
            prompts,
        });
    }
    if never {
        return taken(root, at, prompts, out);
    }
    note(out, GONE);
    Some(Stamp {
        commit: None,
        parent: None,
        time: now(),
        green: false,
        prompts,
    })
}

const GONE: &str = "the turn stamp and its ref are both gone, so klin wrote no fresh stamp and \
                    the next stop judges the whole branch";

/// The stamp: a commit over a tree of everything `.gitignore` does not exclude, with HEAD as
/// its parent, held under a ref so `git gc` does not prune it. Spec 6.5. The index starts
/// empty on every stamp, so a file that became ignored leaves the tree, at the cost of
/// hashing the whole tree once per prompt. A reused index would keep the stat cache.
fn stamped(root: &Path, at: &Path) -> Option<(String, Option<String>)> {
    let index = at.join(INDEX);
    let _ = std::fs::remove_file(&index);
    let git = |args: &[&str]| git(root, Some(&index), args);
    git(&["add", "-A"])?;
    let tree = git(&["write-tree"])?;
    let head = resolve(root, "HEAD");
    let mut args = vec!["commit-tree", &tree];
    if let Some(head) = &head {
        args.extend(["-p", head]);
    }
    args.extend(["-m", "klin: the turn stamp"]);
    let commit = git(&args)?;
    git(&["update-ref", REFERENCE, &commit])?;
    Some((commit, head))
}

fn resolve(root: &Path, reference: &str) -> Option<String> {
    let found = git(
        root,
        None,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
    )?;
    (!found.is_empty()).then_some(found)
}

/// Every git call the stamp makes, with klin as the author of its own commit and an index of
/// its own, so nothing here touches what a person staged.
fn git(root: &Path, index: Option<&Path>, args: &[&str]) -> Option<String> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=klin",
            "-c",
            "user.email=klin@invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let done = command.output().ok()?;
    done.status
        .success()
        .then(|| String::from_utf8_lossy(&done.stdout).trim().to_string())
}

fn write(at: &Path, stamp: &Stamp, out: &mut String) {
    let mut fields = Map::new();
    for (key, value) in [("commit", &stamp.commit), ("parent", &stamp.parent)] {
        if let Some(found) = value {
            fields.insert(key.into(), found.clone().into());
        }
    }
    fields.insert("time".into(), stamp.time.into());
    let verdict = match stamp.green {
        true => "green",
        false => "red",
    };
    fields.insert("verdict".into(), verdict.into());
    fields.insert("prompts".into(), stamp.prompts.into());
    let text = Value::Object(fields).to_string() + "\n";
    let writing = at.join(WRITING);
    if std::fs::write(&writing, text).is_ok() && std::fs::rename(&writing, at.join(FILE)).is_ok() {
        return;
    }
    let _ = std::fs::remove_file(&writing);
    note(
        out,
        &format!("{} could not be written", at.join(FILE).display()),
    );
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

fn note(out: &mut String, why: &str) {
    let _ = writeln!(out, "klin: NOTE: {why}.");
}
