use std::fmt::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::base::{self, Kind, Window};
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

#[derive(clap::Args)]
pub struct Moved {
    #[command(subcommand)]
    which: Which,
}

#[derive(clap::Subcommand)]
enum Which {
    /// Move the stamp to the working tree, whatever verdict the last stop left
    Reset,
}

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
    let held = held(start, &at, out);
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

/// The third route out of a red window: a person moves the stamp to the working tree, so the
/// debt behind it stops reading as new. The fresh stamp is red like any other, so the next
/// prompt leaves it where the reset put it until a stop judges the tree, and the counter
/// carries over, because the turn did not end. Spec 6.2.
pub fn moved(args: &Moved, start: &Path, out: &mut String) -> Result<u8, Error> {
    let Which::Reset = args.which;
    let at = state::ready(start).map_err(Error)?;
    let prompts = read(&at).map_or(0, |held| held.prompts);
    let Some(stamp) = taken(start, &at, prompts, out) else {
        return Err(Error("git could not stamp this tree".to_string()));
    };
    write(&at, &stamp, out);
    let _ = writeln!(
        out,
        "klin: a person moved the turn stamp to the working tree."
    );
    Ok(0)
}

/// The prompt counter the stamp holds, which the build stamp keys its count to. Zero when no
/// stamp is readable, so a stop with no stamp still has one budget of its own. Spec 9.3.
pub fn prompts(at: &Path) -> u64 {
    read(at).map_or(0, |held| held.prompts)
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

/// The stamp a prompt and a stop read alike: the `turn` file, or the ref as the recovery copy
/// when that file is gone. A stamp restored from the ref is red, so the next stop judges
/// everything since it, and a NOTE says the restore happened. Spec 6.2, 16.1.
fn held(root: &Path, at: &Path, out: &mut String) -> Option<Stamp> {
    if let Some(stamp) = read(at).filter(|stamp| resolves(root, stamp)) {
        return Some(stamp);
    }
    let stamp = kept(root)?;
    note(
        out,
        "the turn file was gone and the ref still held the stamp, so klin restored it with \
         a red verdict",
    );
    write(at, &stamp, out);
    Some(stamp)
}

/// A stamp whose commit git still holds. A commit reachable only from a per-worktree ref can
/// be pruned by a `git gc` in a sibling worktree, and a base klin cannot check out ends a stop
/// at exit 2 with no route to green, so a stamp that does not resolve counts as deleted. A
/// stamp that names no commit is the record of a deletion, and it has no commit to lose.
fn resolves(root: &Path, stamp: &Stamp) -> bool {
    match stamp.commit.as_deref() {
        Some(commit) => resolve(root, commit).is_some(),
        None => true,
    }
}

/// The stamp the ref holds, which is the recovery copy of the `turn` file. Red, because a stop
/// that reads it judges everything since the stamp. Spec 6.5.
fn kept(root: &Path) -> Option<Stamp> {
    let commit = resolve(root, REFERENCE)?;
    Some(Stamp {
        parent: resolve(root, &format!("{commit}^")),
        commit: Some(commit),
        time: now(),
        green: false,
        prompts: 0,
    })
}

/// A stamp neither the file nor the ref holds. With the ref gone the stamp was deleted, and a
/// fresh stamp would photograph whatever the deletion hid. Spec 6.2.
fn restored(root: &Path, at: &Path, never: bool, prompts: u64, out: &mut String) -> Option<Stamp> {
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

/// The window a stop in the hook judges: the turn stamp, or the whole branch when the stamp
/// was deleted, which the stop then writes as the stamp so the window stops widening. A state
/// directory klin cannot keep costs the same widening and nothing else. Spec 6.2, 14, 16.1.
pub fn window(root: &Path, out: &mut String) -> Result<Window, Error> {
    let Ok(at) = state::ready(root) else {
        return match kept(root) {
            Some(stamp) => Ok(turn(&stamp)),
            None => branch(root, out),
        };
    };
    let held = held(root, &at, out);
    if let Some(stamp) = held.as_ref().filter(|held| held.commit.is_some()) {
        return Ok(turn(stamp));
    }
    note(out, GONE_ON_A_STOP);
    let base = branch(root, out)?;
    write(
        &at,
        &Stamp {
            commit: Some(base.before.clone()),
            parent: Some(base.before.clone()),
            time: now(),
            green: false,
            prompts: held.map_or(0, |held| held.prompts),
        },
        out,
    );
    Ok(base)
}

/// The window the stamp itself is, once a stop has one to read.
fn turn(stamp: &Stamp) -> Window {
    Window {
        kind: Kind::Turn,
        before: stamp.commit.clone().unwrap_or_default(),
        how: format!("the turn stamp, taken {}", ago(stamp.time)),
    }
}

/// The base `klin gate` would choose by hand, and HEAD when none resolves. Spec 6.3.
fn branch(root: &Path, out: &mut String) -> Result<Window, Error> {
    base::choose(root, false).or_else(|problem| {
        let Some(head) = resolve(root, "HEAD") else {
            return Err(problem);
        };
        note(
            out,
            "no base resolves, so this stop judges the tree against HEAD",
        );
        Ok(Window {
            kind: Kind::Branch,
            before: head,
            how: "HEAD, because no base resolves".to_string(),
        })
    })
}

/// The verdict this stop leaves for the next prompt to read. Green lets the stamp move, red
/// keeps it, so the debt stays new until a person fixes, accepts or resets it. Spec 6.2.
pub fn verdict(root: &Path, green: bool, out: &mut String) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    let Some(held) = read(&at) else {
        return;
    };
    write(&at, &Stamp { green, ..held }, out);
}

/// How long ago the stamp was taken, from the time the stamp holds. An age rather than a date,
/// because it answers the only question a person reading a stop's report asks of it.
fn ago(time: u64) -> String {
    let seconds = now().saturating_sub(time);
    match seconds {
        0..60 => "just now".to_string(),
        60..3600 => format!("{} minute(s) ago", seconds / 60),
        _ => format!("{} hour(s) ago", seconds / 3600),
    }
}

const GONE: &str = "the turn stamp and its ref are both gone, so klin wrote no fresh stamp and \
                    the next stop judges the whole branch";
const GONE_ON_A_STOP: &str = "no turn stamp resolves, in the turn file or in the ref, so this \
                              stop judges the whole branch and writes the base it judged \
                              against as the stamp";

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
