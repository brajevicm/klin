use std::ffi::OsStr;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::base::{self, Kind, Window};
use crate::config::{Config, Error};
use crate::git::Repo;
use crate::host;
use crate::journal;
use crate::radius;
use crate::state;
use crate::write::{AtomicWrite, atomic_write};

/// The stamp file in the state directory, and the name it is written under before the rename,
/// so a hook that dies mid-write leaves the previous stamp rather than a torn one. Spec 6.5.
const FILE: &str = "turn";
/// The index the stamp is built in, apart from the one a person's `git add` writes.
const INDEX: &str = "index";
/// Git shares `refs/` across the worktrees of one repository, and `refs/worktree/` is one of
/// the exceptions, so each worktree keeps its own stamp. The ref is never pushed. Spec 6.5.
const REFERENCE: &str = "refs/worktree/klin/turn";
/// The prompt mark, beside the stamp and under the same guarded namespace. The stamp waits
/// for a green stop, so a report keyed to it re-measures one widening window on every prompt.
/// The mark moves on every event, and it is what the report measures. ADR 0024.
const MARK: &str = "refs/worktree/klin/mark";

#[derive(clap::Args)]
pub struct Args {
    /// Print how far this turn has spread, moving no stamp and raising no counter
    #[arg(long)]
    report: bool,
}

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
    /// Where the last event left the prompt mark, which the spread report measures from.
    pub mark: Option<String>,
    pub time: u64,
    pub green: bool,
    pub prompts: u64,
    /// The findings a stop's block already put in front of the agent under this stamp, by the
    /// site id of spec 11.2. A fresh stamp holds none. Spec 8.2.
    pub asked: Vec<String>,
    /// Whether a stop under this stamp spent a gate block, so the turn holds an intervention for
    /// the turn end to tell. A fresh stamp holds none. Spec 6.5, 9.5.
    pub intervened: bool,
    /// The hash of the exact stop report a host will submit as its next prompt. It is consumed
    /// once, so protocol-generated text cannot open a fresh turn and another prompt clears it.
    pub followup: Option<u64>,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if args.report {
        return radius::asked(start, out);
    }
    let Some((event, named, at)) = opening(start, out) else {
        return Ok(0);
    };
    let start = named.as_path();
    let never = !at.join(INDEX).exists();
    let opened = mark(start, &at);
    let tree = tree(start, &at);
    let held = held(start, &at, &mut Vec::new(), out);
    let prompts = held.as_ref().map_or(0, |held| held.prompts) + 1;
    if event.as_ref().is_some_and(|event| event.prompted) {
        journaled_prompt(
            start,
            &at,
            prompts,
            opened.as_deref(),
            tree.as_deref(),
            event.as_ref(),
            out,
        );
    }
    let mark = tree.as_deref().and_then(|tree| marked(start, tree));
    if let Some(stamp) = next(start, tree.as_deref(), never, held, prompts, out) {
        let mark = mark.or(stamp.mark);
        write(&at, &Stamp { mark, ..stamp }, out);
    }
    Ok(0)
}

/// Read and place the hook event once, resolve its tree, and stop before a turn opens when the
/// event is the exact follow-up a prior stop recorded.
fn opening(start: &Path, out: &mut String) -> Option<(Option<host::Event>, PathBuf, PathBuf)> {
    let event = host::read(None);
    let root = event
        .as_ref()
        .and_then(|event| event.root.clone())
        .unwrap_or_else(|| start.to_path_buf());
    state::dir(&root)?;
    if event
        .as_ref()
        .is_some_and(|event| event.prompted && consumes_followup(&root, &event.prompt, out))
    {
        return None;
    }
    let at = match state::ready(&root) {
        Ok(at) => at,
        Err(why) => {
            note(out, &format!("{why}, so this turn has no stamp"));
            return None;
        }
    };
    Some((event, root, at))
}

/// The prompt line of spec 9.6 and 11.4: the counter, the session and excerpt from the event,
/// and the radius facts, behind one config load so a `UserPromptSubmit` event reads klin.json
/// once and not twice. A config klin cannot read carries no excerpt: the one case where klin
/// cannot see `journal.prompt` is the case where it must not record the text.
fn journaled_prompt(
    start: &Path,
    at: &Path,
    prompts: u64,
    opened: Option<&str>,
    tree: Option<&str>,
    event: Option<&host::Event>,
    out: &mut String,
) {
    let config = Config::load(None, start).ok();
    let (enabled, facts) = match &config {
        Some(config) => (
            journal::prompt_enabled(config),
            radius::spread(config, start, at, opened, tree, out),
        ),
        None => (false, None),
    };
    journal::prompt(start, prompts, event, enabled, facts);
}

/// Where this turn's window opened: the prompt mark the last event left, or the mark ref when
/// the turn file is gone. It writes nothing back, because the report judges nothing. ADR 0024.
pub fn mark(root: &Path, at: &Path) -> Option<String> {
    let held = read(at)
        .and_then(|held| held.mark)
        .filter(|mark| resolve(root, mark).is_some());
    held.or_else(|| resolve(root, MARK))
}

/// The commit every derived value comes from: the stamp's parent, which is the HEAD the stamp
/// was taken over, so a commit inside an open turn does not move it. HEAD when no stamp is
/// readable, and `None` outside a repository. Spec 6.6.
pub fn derivation(root: &Path, at: Option<&Path>) -> Option<String> {
    at.and_then(read)
        .and_then(|held| held.parent)
        .filter(|parent| resolve(root, parent).is_some())
        .or_else(|| resolve(root, "HEAD"))
}

/// The mark this event leaves for the next prompt to measure from. It moves on a session start
/// and on a prompt alike, whatever verdict the last stop left. ADR 0024.
fn marked(root: &Path, tree: &str) -> Option<String> {
    stamped(root, tree, MARK).map(|(commit, _)| commit)
}

/// One rule, on a session start and on a prompt alike: the stamp moves on a first session or
/// after a green stop, and otherwise stays. The counter rises either way. Spec 6.2. A stamp
/// klin never took is a first session, whatever else the state directory holds, because other
/// commands write there too.
fn next(
    root: &Path,
    tree: Option<&str>,
    never: bool,
    held: Option<Stamp>,
    prompts: u64,
    out: &mut String,
) -> Option<Stamp> {
    let Some(held) = held else {
        return restored(root, tree, never, prompts, out);
    };
    if held.green {
        return taken(root, tree, prompts, out).or(Some(Stamp { prompts, ..held }));
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
    let tree = tree(start, &at);
    let Some(stamp) = taken(start, tree.as_deref(), prompts, out) else {
        return Err(Error("git could not stamp this tree".to_string()));
    };
    let mark = tree.as_deref().and_then(|tree| marked(start, tree));
    write(&at, &Stamp { mark, ..stamp }, out);
    journal::reset(start, prompts);
    let _ = writeln!(
        out,
        "klin: a person moved the turn stamp to the working tree."
    );
    Ok(0)
}

/// The prompt counter the stamp holds, which the build stamp keys its count to. Only `klin
/// radius` raises it, on a session start and on a prompt submitted. Zero when no stamp is
/// readable, and zero for as long as it stays unreadable, so a worktree whose prompt hook
/// never runs holds one budget rather than one for each turn. Spec 9.3.
pub fn prompts(at: &Path) -> u64 {
    read(at).map_or(0, |held| held.prompts)
}

/// When the current turn stamp was taken, and `None` when no stamp is readable. Spec 11.5.
pub fn taken_at(root: &Path) -> Option<u64> {
    state::dir(root)
        .and_then(|at| read(&at))
        .map(|held| held.time)
}

/// The findings a stop's block already put in front of the agent under the current stamp. The
/// record lives beside the stamp and not in the build stamp, so a prompt event between two
/// stops keeps it, and it goes when the stamp moves. Empty when no stamp is readable. Spec 8.2.
pub fn asked(root: &Path) -> Vec<String> {
    state::dir(root)
        .and_then(|at| read(&at))
        .map(|held| held.asked)
        .unwrap_or_default()
}

/// Whether a stop under the current stamp spent a gate block. False when no stamp is readable.
/// Spec 9.5.
pub fn intervened(root: &Path) -> bool {
    state::dir(root)
        .and_then(|at| read(&at))
        .is_some_and(|held| held.intervened)
}

/// Remember the exact report a host will submit as its next prompt. The host adapter says
/// whether it has that delivery mode; the turn owns the state that keeps it from becoming a
/// person's next turn.
pub fn expect_followup(root: &Path, report: &str) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    let Some(mut held) = read(&at) else {
        return;
    };
    held.followup = Some(state::hash(report.as_bytes()));
    write(&at, &held, &mut String::new());
}

/// Consume one expected follow-up. A different prompt clears the expectation and remains a
/// person's prompt; an exact match is host-generated and opens no turn.
fn consumes_followup(root: &Path, prompt: &str, out: &mut String) -> bool {
    let Some(at) = state::dir(root) else {
        return false;
    };
    let Some(mut held) = read(&at) else {
        return false;
    };
    let expected = held.followup.take();
    if expected.is_some() {
        write(&at, &held, out);
    }
    expected == Some(state::hash(prompt.as_bytes()))
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
        mark: text("mark"),
        time: held.get("time").and_then(Value::as_u64).unwrap_or_default(),
        green: text("verdict").as_deref() == Some("green"),
        prompts: held
            .get("prompts")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        asked: held
            .get("asked")
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        intervened: held
            .get("intervened")
            .and_then(Value::as_bool)
            .unwrap_or_default(),
        followup: held.get("followup").and_then(Value::as_u64),
    })
}

/// A fresh stamp, or `None` when git could not take one, so the caller keeps the stamp it has
/// and the next prompt tries again.
fn taken(root: &Path, tree: Option<&str>, prompts: u64, out: &mut String) -> Option<Stamp> {
    let Some((commit, parent)) = tree.and_then(|tree| stamped(root, tree, REFERENCE)) else {
        note(
            out,
            "git could not stamp this tree, so the stamp klin already had still stands",
        );
        return None;
    };
    Some(Stamp {
        commit: Some(commit),
        parent,
        mark: None,
        time: now(),
        green: false,
        prompts,
        asked: Vec::new(),
        intervened: false,
        followup: None,
    })
}

/// The stamp a prompt and a stop read alike: the `turn` file, or the ref as the recovery copy
/// when that file is gone. A stamp restored from the ref is red, so the next stop judges
/// everything since it, and a NOTE says the restore happened. Spec 6.2, 16.1.
fn held(root: &Path, at: &Path, flags: &mut Vec<&'static str>, out: &mut String) -> Option<Stamp> {
    if let Some(stamp) = read(at).filter(|stamp| resolves(root, stamp)) {
        return Some(stamp);
    }
    let stamp = kept(root)?;
    flags.push("turn-restored");
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
        mark: resolve(root, MARK),
        commit: Some(commit),
        time: now(),
        green: false,
        prompts: 0,
        asked: Vec::new(),
        intervened: false,
        followup: None,
    })
}

/// A stamp neither the file nor the ref holds. With the ref gone the stamp was deleted, and a
/// fresh stamp would photograph whatever the deletion hid. Spec 6.2.
fn restored(
    root: &Path,
    tree: Option<&str>,
    never: bool,
    prompts: u64,
    out: &mut String,
) -> Option<Stamp> {
    if never {
        return taken(root, tree, prompts, out);
    }
    note(out, GONE);
    Some(Stamp {
        commit: None,
        parent: None,
        mark: None,
        time: now(),
        green: false,
        prompts,
        asked: Vec::new(),
        intervened: false,
        followup: None,
    })
}

/// The window a stop in the hook judges: the turn stamp, or the whole branch when the stamp
/// was deleted, which the stop then writes as the stamp so the window stops widening. A state
/// directory klin cannot keep costs the same widening and nothing else. Spec 6.2, 14, 16.1.
pub fn window(
    root: &Path,
    flags: &mut Vec<&'static str>,
    out: &mut String,
) -> Result<Window, Error> {
    let Ok(at) = state::ready(root) else {
        return match kept(root) {
            Some(stamp) if abandoned(root, &stamp) => {
                note(out, LEFT_BEHIND);
                branch(root, out)
            }
            Some(stamp) => Ok(turn(&stamp)),
            None => branch(root, out),
        };
    };
    let held = held(root, &at, flags, out);
    if let Some(stamp) = held.as_ref().filter(|held| held.commit.is_some()) {
        if !abandoned(root, stamp) {
            return Ok(turn(stamp));
        }
        note(out, LEFT_BEHIND);
        reanchored(root);
        let base = branch(root, out)?;
        return Ok(replaced(&at, held.as_ref(), None, base, out));
    }
    note(out, GONE_ON_A_STOP);
    let mark = held.as_ref().and_then(|held| held.mark.clone());
    let base = branch(root, out)?;
    Ok(replaced(&at, held.as_ref(), mark, base, out))
}

/// Whether the commit the stamp was taken over has left current HEAD history, which is what a
/// checkout of divergent history, a reset or a rebase does to a stamp. The stamp is a synthetic
/// sibling of that commit and never an ancestor of HEAD itself, so the parent is what carries
/// the lineage. Only git's proven no abandons a turn: a git that could not answer, and a stamp
/// that names no parent, leave the turn where it is. Spec 6.2.
fn abandoned(root: &Path, stamp: &Stamp) -> bool {
    stamp
        .parent
        .as_deref()
        .and_then(|parent| Repo::at(root).contains(parent))
        == Some(false)
}

/// The stamp a stop writes in place of one it could not use: the base it judged instead, red,
/// keeping the prompt counter and dropping every per-turn record of the turn it left, so the
/// next stop judges from here and not from the window that widened. Spec 6.2, 16.1.
fn replaced(
    at: &Path,
    held: Option<&Stamp>,
    mark: Option<String>,
    base: Window,
    out: &mut String,
) -> Window {
    write(
        at,
        &Stamp {
            commit: Some(base.before.clone()),
            parent: Some(base.before.clone()),
            mark,
            time: now(),
            green: false,
            prompts: held.map_or(0, |held| held.prompts),
            asked: Vec::new(),
            intervened: false,
            followup: None,
        },
        out,
    );
    base
}

/// The two refs that outlive an abandoned turn, both deleted. They are recovery copies of a
/// stamp this checkout left, and a copy of it is the one thing a later stop must not read. A
/// stop that loses the `turn` file after this widens to the branch window, which is the window
/// this stop already judged, so the deletion forgives nothing. The ref cannot instead move to
/// the base: `kept` reads a stamp's parent as `commit^`, which holds for a synthetic stamp and
/// not for an ordinary commit, so a moved ref would name a parent the stop never took and pass
/// the lineage test on history that holds no base at all. Spec 6.2, 6.2.1.
fn reanchored(root: &Path) {
    let _ = git(root, None, &["update-ref", "-d", REFERENCE]);
    let _ = git(root, None, &["update-ref", "-d", MARK]);
}

/// The window the stamp itself is, once a stop has one to read.
fn turn(stamp: &Stamp) -> Window {
    Window {
        kind: Kind::Turn,
        before: stamp.commit.clone().unwrap_or_default(),
        how: format!("the turn stamp, taken {}", ago(stamp.time)),
        derives: stamp.parent.clone(),
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
            derives: Some(head.clone()),
            before: head,
            how: "HEAD, because no base resolves".to_string(),
        })
    })
}

/// The verdict this stop leaves for the next prompt to read, and, where the stop spent a gate
/// block, the findings that block put in front of the agent. Green lets the stamp move, red keeps
/// it, so the debt stays new until a person fixes, accepts or resets it. Spec 6.2, 8.2, 9.5.
pub fn verdict(
    root: &Path,
    green: bool,
    asked: Option<&[String]>,
    out: &mut String,
) -> Result<(), &'static str> {
    let Ok(at) = state::ready(root) else {
        return Err("klin could not ready the state directory, so this stop wrote no verdict");
    };
    let Some(held) = read(&at) else {
        return Err(
            "the state directory holds no stamp klin could read, so this stop wrote no \
                    verdict",
        );
    };
    let mut all = held.asked.clone();
    all.extend(asked.unwrap_or_default().iter().cloned());
    all.sort();
    all.dedup();
    let wrote = write(
        &at,
        &Stamp {
            green,
            asked: all,
            intervened: held.intervened || asked.is_some(),
            ..held
        },
        out,
    );
    match wrote {
        true => Ok(()),
        false => Err("the turn stamp could not be written, so this stop wrote no verdict"),
    }
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
const LEFT_BEHIND: &str = "this turn started from a commit HEAD no longer holds, so the turn \
                           stamp cannot describe this turn and this stop judges the whole \
                           branch instead";

/// The stamp: a commit over a tree of everything `.gitignore` does not exclude, with HEAD as
/// its parent, held under a ref so `git gc` does not prune it. Spec 6.5. The index starts
/// empty on every stamp, so a file that became ignored leaves the tree, at the cost of
/// hashing the whole tree once per prompt. A reused index would keep the stat cache.
fn stamped(root: &Path, tree: &str, reference: &str) -> Option<(String, Option<String>)> {
    let head = resolve(root, "HEAD");
    let mut args = vec!["commit-tree", tree];
    if let Some(head) = &head {
        args.extend(["-p", head]);
    }
    args.extend(["-m", "klin: the turn stamp"]);
    let commit = git(root, None, &args)?;
    git(root, None, &["update-ref", reference, &commit])?;
    Some((commit, head))
}

/// A tree of the working directory, everything `.gitignore` does not exclude, written through
/// an index of klin's own. Both the stamp and the spread report read the turn from it.
pub fn tree(root: &Path, at: &Path) -> Option<String> {
    tree_through(root, &at.join(INDEX))
}

/// The same tree through an index the caller names, for a reader that must not leave the
/// stamp's own index behind, because `run` reads that file's absence as a first session.
pub fn tree_through(root: &Path, index: &Path) -> Option<String> {
    let _ = std::fs::remove_file(index);
    git(root, Some(index), &["add", "-A"])?;
    git(root, Some(index), &["write-tree"])
}

fn resolve(root: &Path, reference: &str) -> Option<String> {
    let refspec = format!("{reference}^{{commit}}");
    Repo::at(root).rev_parse(&["--verify", "--quiet", &refspec])
}

/// Every git call the stamp makes, with klin as the author of its own commit and an index of
/// its own, so nothing here touches what a person staged.
pub fn git(root: &Path, index: Option<&Path>, args: &[&str]) -> Option<String> {
    let mut command = vec![
        "-c",
        "user.name=klin",
        "-c",
        "user.email=klin@invalid",
        "-c",
        "commit.gpgsign=false",
    ];
    command.extend_from_slice(args);
    let text = match index {
        Some(index) => {
            let env = [(OsStr::new("GIT_INDEX_FILE"), index.as_os_str())];
            Repo::at(root).text_with_env(&command, &env)
        }
        None => Repo::at(root).text(&command),
    }?;
    Some(text.trim().to_string())
}

fn write(at: &Path, stamp: &Stamp, out: &mut String) -> bool {
    let text = recorded(stamp).to_string() + "\n";
    let target = at.join(FILE);
    if atomic_write(AtomicWrite {
        target: &target,
        bytes: text.as_bytes(),
        keep_mode_from: None,
    })
    .is_ok()
    {
        return true;
    }
    note(out, &format!("{} could not be written", target.display()));
    false
}

/// The stamp as the `turn` file holds it. A field a fresh stamp does not have is left out.
/// Spec 6.5.
fn recorded(stamp: &Stamp) -> Value {
    let mut fields = Map::new();
    let fields_of = [
        ("commit", &stamp.commit),
        ("parent", &stamp.parent),
        ("mark", &stamp.mark),
    ];
    for (key, value) in fields_of {
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
    if !stamp.asked.is_empty() {
        fields.insert("asked".into(), stamp.asked.clone().into());
    }
    if stamp.intervened {
        fields.insert("intervened".into(), true.into());
    }
    if let Some(followup) = &stamp.followup {
        fields.insert("followup".into(), (*followup).into());
    }
    Value::Object(fields)
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
