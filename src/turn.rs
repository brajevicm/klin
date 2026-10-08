use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::base::{self, Kind, Window};
use crate::config::Config;
use crate::error::Error;
use crate::git::Repo;
use crate::handoff;
use crate::host::adapter::{self, Event};
use crate::journal;
use crate::key::Section;
use crate::radius;
use crate::stamp::{self, Here, History, Stamp, Verdict};
use crate::state;
use crate::write::{AtomicWrite, atomic_write};

/// Git shares `refs/` across the worktrees of one repository, and `refs/worktree/` is one of
/// the exceptions, so each worktree keeps its own stamp. The ref is never pushed. Spec 6.5.
const REFERENCE: &str = "refs/worktree/klin/turn";
/// A `session` or `prompt` event of the agent ingress, over the tree the event names, after the
/// opt-in walk found the worktree root's `klin.json`. Spec 10.3.
pub fn opened(event: Event, start: &Path, sections: &[Section], out: &mut String) -> u8 {
    let Some((at, _claim)) = opening(&event, start, out) else {
        return 0;
    };
    let prompted = event.kind == Some(adapter::Kind::Prompt);
    let opened = stamp::mark(start, &at);
    let capture = stamp::capture(start, &at.join(stamp::INDEX));
    let tree = capture.as_ref().map(|capture| capture.tree.as_str());
    let facts = prompted.then(|| prompt_facts(start, &at, opened.as_deref(), tree, sections, out));
    let Some(_lock) = state::lock(&at, Duration::from_secs(30)) else {
        note(
            out,
            "the state directory could not be locked, so this prompt changed no turn state",
        );
        return 0;
    };
    let never = !at.join(stamp::INDEX).exists();
    let held = held(start, &at, &mut Vec::new(), out);
    let prompts = held.as_ref().map_or(0, |held| held.prompts) + 1;
    handoff::clear(start, &event.session);
    if let Some((enabled, facts)) = facts {
        journal::prompt(start, prompts, Some(&event), enabled, facts);
    }
    let mark = tree.and_then(|tree| marked(start, tree));
    if let Some(stamp) = next(start, &at, tree, never, held, prompts, out) {
        let mark = mark.or(stamp.mark);
        if write(&at, &Stamp { mark, ..stamp }, out)
            && let Some(capture) = capture
        {
            let _ = capture.retain(&at.join(stamp::INDEX));
        }
    }
    0
}

/// The state directory, and the claim on this event, or `None` where the turn does not open:
/// another copy of klin's hooks took the event, or the event is the exact follow-up a prior
/// stop recorded. The claim comes first, so the copy that yields consumes no follow-up.
fn opening(event: &Event, root: &Path, out: &mut String) -> Option<(PathBuf, state::Claim)> {
    state::dir(root)?;
    let at = match state::ready(root) {
        Ok(at) => at,
        Err(why) => {
            note(out, &format!("{why}, so this turn has no stamp"));
            return None;
        }
    };
    let claim = state::claim(&at, &event.identity)?;
    if event.kind == Some(adapter::Kind::Prompt)
        && handoff::consumes(root, &event.session, &event.prompt)
    {
        return None;
    }
    Some((at, claim))
}

/// What the prompt line of spec 9.6 and 11.4 reads from the configuration: whether it records
/// the excerpt, and the radius facts, behind one config load so a `UserPromptSubmit` event reads
/// klin.json once and not twice. A config klin cannot read carries no excerpt: the one case
/// where klin cannot see `journal.prompt` is the case where it must not record the text.
fn prompt_facts(
    start: &Path,
    at: &Path,
    opened: Option<&str>,
    tree: Option<&str>,
    sections: &[Section],
    out: &mut String,
) -> (bool, Option<Value>) {
    match Config::load(None, start, sections) {
        Ok(config) => (
            journal::prompt_enabled(&config),
            radius::spread(&config, start, at, opened, tree, out),
        ),
        Err(_) => (false, None),
    }
}

/// The mark this event leaves for the next prompt to measure from. It moves on a session start
/// and on a prompt alike, whatever verdict the last stop left. ADR 0024.
fn marked(root: &Path, tree: &str) -> Option<String> {
    stamped(root, tree, stamp::MARK).map(|(commit, _)| commit)
}

/// One rule, on a session start and on a prompt alike: the stamp moves on a first session or
/// after a green or unjudged stop, and otherwise stays. The counter rises either way. Spec 6.2,
/// 6.6. A stamp klin never took is a first session, whatever else the state directory holds,
/// because other commands write there too.
fn next(
    root: &Path,
    at: &Path,
    tree: Option<&str>,
    never: bool,
    held: Option<Stamp>,
    prompts: u64,
    out: &mut String,
) -> Option<Stamp> {
    let Some(held) = held else {
        return restored(root, at, tree, never, prompts, out);
    };
    if held.verdict.moves() {
        return taken(root, at, tree, prompts, out).or(Some(Stamp { prompts, ..held }));
    }
    if held.commit.is_none() {
        note(out, GONE);
    }
    Some(Stamp { prompts, ..held })
}

/// The prompt counter the stamp holds, which the build stamp keys its count to. Only `klin
/// radius` raises it, on a session start and on a prompt submitted. Zero when no stamp is
/// readable, and zero for as long as it stays unreadable, so a worktree whose prompt hook
/// never runs holds one budget rather than one for each turn. Spec 9.3.
pub fn prompts(at: &Path) -> u64 {
    stamp::read(at).map_or(0, |held| held.prompts)
}

/// When the current turn stamp was taken, and `None` when no stamp is readable. Spec 11.5.
pub fn taken_at(root: &Path) -> Option<u64> {
    state::dir(root)
        .and_then(|at| stamp::read(&at))
        .map(|held| held.time)
}

/// The findings a stop's block already put in front of the agent under the current stamp. The
/// record lives beside the stamp and not in the build stamp, so a prompt event between two
/// stops keeps it, and it goes when the stamp moves. Empty when no stamp is readable. Spec 8.2.
pub fn asked(root: &Path) -> Vec<String> {
    state::dir(root)
        .and_then(|at| stamp::read(&at))
        .map(|held| held.asked)
        .unwrap_or_default()
}

/// Whether a stop under the current stamp spent a gate block. False when no stamp is readable.
/// Spec 9.5.
pub fn intervened(root: &Path) -> bool {
    state::dir(root)
        .and_then(|at| stamp::read(&at))
        .is_some_and(|held| held.intervened)
}

/// A fresh stamp, or `None` when git could not take one, so the caller keeps the stamp it has
/// and the next prompt tries again.
fn taken(
    root: &Path,
    at: &Path,
    tree: Option<&str>,
    prompts: u64,
    out: &mut String,
) -> Option<Stamp> {
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
        time: now(),
        prompts,
        history: History::now(root, at, &Here::read(root)),
        ..Stamp::default()
    })
}

/// The stamp a prompt and a stop read alike: the `turn` file, or the ref as the recovery copy
/// when that file is gone. A stamp restored from the ref is red, so the next stop judges
/// everything since it, and a NOTE says the restore happened. Spec 6.2, 16.1.
fn held(root: &Path, at: &Path, flags: &mut Vec<&'static str>, out: &mut String) -> Option<Stamp> {
    if let Some(stamp) = stamp::read(at).filter(|stamp| resolves(root, stamp)) {
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
        Some(commit) => stamp::resolve(root, commit).is_some(),
        None => true,
    }
}

/// The stamp the ref holds, which is the recovery copy of the `turn` file. Red, because a stop
/// that reads it judges everything since the stamp. Spec 6.5.
fn kept(root: &Path) -> Option<Stamp> {
    let commit = stamp::resolve(root, REFERENCE)?;
    Some(Stamp {
        parent: stamp::resolve(root, &format!("{commit}^")),
        mark: stamp::resolve(root, stamp::MARK),
        commit: Some(commit),
        time: now(),
        verdict: Verdict::red(),
        ..Stamp::default()
    })
}

/// A stamp neither the file nor the ref holds. With the ref gone the stamp was deleted, and a
/// fresh stamp would photograph whatever the deletion hid. Spec 6.2.
fn restored(
    root: &Path,
    at: &Path,
    tree: Option<&str>,
    never: bool,
    prompts: u64,
    out: &mut String,
) -> Option<Stamp> {
    if never {
        return taken(root, at, tree, prompts, out);
    }
    note(out, GONE);
    Some(Stamp {
        time: now(),
        verdict: Verdict::red(),
        prompts,
        ..Stamp::default()
    })
}

/// The window a stop in the hook judges, and why the Stop is advisory when the history moved
/// under the turn. The window is the turn stamp, or the whole branch when the stamp was deleted.
/// A repository with no remote takes no advisory Stop: a deleted stamp or a branch change there
/// judges the branch and writes the base it judged as the stamp, so the window stops widening.
/// A state directory klin cannot keep costs the same widening and nothing else. A stop that
/// `lost` the state lock reads the same window and writes nothing, because the stop holding the
/// lock may be writing the stamp. Spec 6.2, 6.5, 6.6, 14, 16.1.
pub fn window(
    root: &Path,
    lost: bool,
    flags: &mut Vec<&'static str>,
    out: &mut String,
) -> Result<(Window, Option<Reason>), Error> {
    let at = state::ready(root).ok().filter(|_| !lost);
    let Some(at) = at else {
        return read_only(root, out);
    };
    let here = Here::read(root);
    let held = held(root, &at, flags, out);
    let Some(stamp) = held.as_ref().filter(|held| held.commit.is_some()) else {
        if here.remotes {
            return Ok((branch(root, out)?, Some(Reason::StampMissing)));
        }
        note(out, GONE_ON_A_STOP);
        let mark = held.as_ref().and_then(|held| held.mark.clone());
        let base = branch(root, out)?;
        return Ok((replaced(&at, held.as_ref(), mark, base, out), None));
    };
    if here.remotes {
        let reason = advisory(root, &at, stamp, &here);
        if reason.is_none() {
            settled(root, &at, stamp, &here, out);
        }
        return Ok((turn(stamp), reason));
    }
    if !switched(root, stamp, &here) {
        return Ok((turn(stamp), None));
    }
    note(out, LEFT_BEHIND);
    reanchored(root);
    let base = branch(root, out)?;
    Ok((replaced(&at, held.as_ref(), None, base, out), None))
}

/// Why a Stop is advisory: the history moved under the turn, so a precise local judgement is
/// no longer possible. Spec 6.6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    IncomingCommits,
    BranchChanged,
    HistoryLost,
    StampMissing,
}

impl Reason {
    pub fn name(self) -> &'static str {
        match self {
            Reason::IncomingCommits => "incoming-commits",
            Reason::BranchChanged => "branch-changed",
            Reason::HistoryLost => "history-lost",
            Reason::StampMissing => "stamp-missing",
        }
    }
}

/// The reflog entries that bring other people's commits in or take the turn's history away. A
/// merge that stopped on a conflict logs its commit as `commit (merge)`.
const INCOMING: [&str; 5] = ["merge", "pull", "rebase", "reset", "commit (merge)"];

/// Rules 1 to 3 of spec 6.6 over a stamp that resolves. The merge-base comes from the
/// cache, so a Stop where neither HEAD nor the default branch moved starts no git process for
/// it. HEAD with no merge-base at all, as after a reset onto unrelated history, can still have
/// lost the recorded one.
fn advisory(root: &Path, at: &Path, stamp: &Stamp, here: &Here) -> Option<Reason> {
    if switched(root, stamp, here) {
        return Some(Reason::BranchChanged);
    }
    let (head, default) = here.pair()?;
    let recorded = recorded(root, at, stamp, default)?;
    match stamp::merge_base(root, at, head, default) {
        Some(now) if now == recorded => None,
        Some(now) => moved(root, here, stamp, &recorded, &now),
        None => lost(root, &recorded),
    }
}

/// The default-branch merge-base the stamp recorded. A stamp that recorded none, as one
/// restored from its ref, has its parent's.
fn recorded(root: &Path, at: &Path, stamp: &Stamp, default: &str) -> Option<String> {
    match &stamp.history.merge_base {
        Some(recorded) => Some(recorded.clone()),
        None => stamp::merge_base(root, at, stamp.parent.as_deref()?, default),
    }
}

/// A merge-base that moved. Forward past the recorded one, it is incoming commits when the
/// reflog says so, and the agent's own push otherwise. Anywhere else, the turn lost its history
/// when neither HEAD nor the default branch holds the recorded merge-base any more. So the
/// agent's own rewrite of commits the default branch still holds, as a `reset --soft` or an
/// amend after a push of the default branch, keeps the turn, and a reset onto a default branch
/// someone rewound or rewrote does not.
fn moved(root: &Path, here: &Here, stamp: &Stamp, recorded: &str, now: &str) -> Option<Reason> {
    let repo = Repo::at(root);
    if repo.is_ancestor(recorded, now) == Some(true) {
        return incoming(here, stamp).then_some(Reason::IncomingCommits);
    }
    let (_, default) = here.pair()?;
    if repo.is_ancestor(recorded, default) == Some(true) {
        return None;
    }
    lost(root, recorded)
}

/// Rule 3: HEAD no longer holds the recorded merge-base, on git's proven answer alone.
fn lost(root: &Path, recorded: &str) -> Option<Reason> {
    (Repo::at(root).contains(recorded) == Some(false)).then_some(Reason::HistoryLost)
}

/// Whether HEAD's reflog since the stamp holds an entry that brings commits in. A reflog that
/// cannot be compared, because HEAD has none or the stamp recorded no position, counts as one.
fn incoming(here: &Here, stamp: &Stamp) -> bool {
    here.reflog_since(stamp.history.reflog_position)
        .is_none_or(|entries| {
            entries
                .iter()
                .any(|entry| INCOMING.iter().any(|kind| entry.starts_with(kind)))
        })
}

/// The history a Stop that is not advisory leaves on the stamp: where it stands now, so the
/// agent's own push or branch is the stamp's history from here on, and a stamp restored from
/// its ref records it at its first Stop. Only a merge-base that moved forward replaces the
/// recorded one, so a rewind of the default branch fetched before a reset onto it is still
/// lost history at the Stop after the reset. Nothing is written when nothing moved.
fn settled(root: &Path, at: &Path, stamp: &Stamp, here: &Here, out: &mut String) {
    let mut history = History::now(root, at, here);
    if let (Some(was), Some(now)) = (&stamp.history.merge_base, &history.merge_base)
        && was != now
        && Repo::at(root).is_ancestor(was, now) != Some(true)
    {
        history.merge_base = Some(was.clone());
    }
    if history == stamp.history {
        return;
    }
    if let Some(held) = stamp::read(at) {
        write(at, &Stamp { history, ..held }, out);
    }
}

/// Whether the branch changed under the turn: HEAD's symbolic ref differs from the one the
/// stamp recorded, or the stamp recorded none, and the stamp's parent left HEAD history. So
/// `git switch -c` at the same HEAD, and an amend or a rebase of the turn's own commits on the
/// same branch, keep the turn window. Spec 6.6.
fn switched(root: &Path, stamp: &Stamp, here: &Here) -> bool {
    let moved = match (&stamp.history.head_ref, &here.head_ref) {
        (Some(was), Some(now)) => was != now,
        _ => true,
    };
    moved && abandoned(root, stamp)
}

/// The fresh stamp an advisory Stop takes of the tree it measured, with the history as it now
/// stands, none of the window's records, the prompt counter carried on, and the prompt mark
/// moved with it. The caller holds the state lock, so the verdict and the stamp describe the
/// same tree. Spec 6.6.
pub fn refreshed(root: &Path, capture: stamp::Capture, out: &mut String) -> bool {
    let Ok(at) = state::ready(root) else {
        return false;
    };
    let tree = Some(capture.tree.as_str());
    let Some(stamp) = taken(root, &at, tree, prompts(&at), out) else {
        return false;
    };
    let mark = tree.and_then(|tree| marked(root, tree));
    let wrote = write(&at, &Stamp { mark, ..stamp }, out);
    if wrote {
        let _ = capture.retain(&at.join(stamp::INDEX));
    }
    wrote
}

/// The tree an advisory Stop measures, captured through the stamp's own index for the fresh
/// stamp it takes. Spec 6.6.
pub fn capture(root: &Path) -> Option<stamp::Capture> {
    let at = state::ready(root).ok()?;
    stamp::capture(root, &at.join(stamp::INDEX))
}

/// The window as the stamp on disk names it, from the `turn` file or else the ref, and why the
/// Stop is advisory, with no restore, no re-anchor, no replacement and no fresh stamp written.
/// So a Stop that lost the lock blocks nothing on a history move either. Spec 6.5, 6.6, 14.
fn read_only(root: &Path, out: &mut String) -> Result<(Window, Option<Reason>), Error> {
    let at = state::dir(root);
    let file = at
        .as_deref()
        .and_then(stamp::read)
        .filter(|stamp| stamp.commit.is_some() && resolves(root, stamp));
    let here = Here::read(root);
    let Some(stamp) = file.or_else(|| kept(root)) else {
        return Ok((
            branch(root, out)?,
            here.remotes.then_some(Reason::StampMissing),
        ));
    };
    if here.remotes {
        let reason = at.and_then(|at| advisory(root, &at, &stamp, &here));
        return Ok((turn(&stamp), reason));
    }
    if !switched(root, &stamp, &here) {
        return Ok((turn(&stamp), None));
    }
    note(out, LEFT_BEHIND);
    Ok((branch(root, out)?, None))
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
            verdict: Verdict::red(),
            prompts: held.map_or(0, |held| held.prompts),
            ..Stamp::default()
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
    let _ = stamp::git(root, None, &["update-ref", "-d", REFERENCE]);
    let _ = stamp::git(root, None, &["update-ref", "-d", stamp::MARK]);
}

/// The window the stamp itself is, once a stop has one to read.
fn turn(stamp: &Stamp) -> Window {
    Window::new(
        Kind::Turn,
        stamp.commit.clone().unwrap_or_default(),
        format!("the turn stamp, taken {}", ago(stamp.time)),
        stamp.parent.clone(),
    )
}

/// The base `klin check` would choose, and HEAD when none resolves. Spec 6.3.
fn branch(root: &Path, out: &mut String) -> Result<Window, Error> {
    base::choose(root).or_else(|problem| {
        let Some(head) = stamp::resolve(root, "HEAD") else {
            return Err(problem);
        };
        note(
            out,
            "no base resolves, so this stop judges the tree against HEAD",
        );
        Ok(Window::new(
            Kind::Branch,
            head.clone(),
            "HEAD, because no base resolves".to_string(),
            Some(head),
        ))
    })
}

/// The notes and errors a Stop already told under the current stamp. Empty when no stamp is
/// readable. Spec 2.3.
pub fn told(root: &Path) -> Vec<String> {
    state::dir(root)
        .and_then(|at| stamp::read(&at))
        .map(|held| held.told)
        .unwrap_or_default()
}

/// `aborted` over the stamp before the Stop measures, so a Stop that dies leaves no earlier
/// `green` behind, and the verdict it replaced, which the final verdict needs. An earlier
/// `aborted` keeps the time it was first written. `None` when no stamp could take it.
/// Spec 6.6.
pub fn aborting(root: &Path) -> Option<Verdict> {
    let at = state::ready(root).ok()?;
    let mut held = stamp::read(&at)?;
    let since = match held.verdict {
        Verdict::Aborted { since } => since,
        _ => now(),
    };
    let prior = std::mem::replace(&mut held.verdict, Verdict::Aborted { since });
    write(&at, &held, &mut String::new()).then_some(prior)
}

/// What one Stop leaves under the stamp: the verdict it reached over the one before it, or `None`
/// where it measured nothing and the `aborted` it wrote stays, and the findings its gate block put
/// in front of the agent. Spec 6.6, 8.2.
pub struct Left<'a> {
    pub prior: Verdict,
    pub verdict: Option<Verdict>,
    pub asked: Option<&'a [String]>,
}

/// The notes and errors a Stop delivered, added to the stamp's `told` record once the host took
/// them, so a notice klin could not deliver is told again later. Spec 2.3.
pub fn heard(root: &Path, told: &[String]) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    let Some(held) = stamp::read(&at) else {
        return;
    };
    let stamp = Stamp {
        told: merged(&held.told, told),
        ..held
    };
    write(&at, &stamp, &mut String::new());
}

/// The verdict this stop leaves for the next prompt to read, and the verdict it wrote. A green or
/// unjudged verdict lets the stamp move, any other keeps it, so the debt stays new until a person
/// fixes or accepts it. Spec 6.2, 6.6, 8.2, 9.5.
pub fn verdict(root: &Path, left: Left, out: &mut String) -> Result<&'static str, &'static str> {
    let Ok(at) = state::ready(root) else {
        return Err("klin could not ready the state directory, so this stop wrote no verdict");
    };
    let Some(mut held) = stamp::read(&at) else {
        return Err(
            "the state directory holds no stamp klin could read, so this stop wrote no \
                    verdict",
        );
    };
    let verdict = match left.verdict {
        Some(verdict) => verdict.over(left.prior),
        None => std::mem::take(&mut held.verdict),
    };
    let name = verdict.name();
    let wrote = write(
        &at,
        &Stamp {
            verdict,
            asked: merged(&held.asked, left.asked.unwrap_or_default()),
            intervened: held.intervened || left.asked.is_some(),
            ..held
        },
        out,
    );
    match wrote {
        true => Ok(name),
        false => Err("the turn stamp could not be written, so this stop wrote no verdict"),
    }
}

fn merged(held: &[String], more: &[String]) -> Vec<String> {
    let mut all = held.to_vec();
    all.extend(more.iter().cloned());
    all.sort();
    all.dedup();
    all
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
    let head = stamp::resolve(root, "HEAD");
    let mut args = vec!["commit-tree", tree];
    if let Some(head) = &head {
        args.extend(["-p", head]);
    }
    args.extend(["-m", "klin: the turn stamp"]);
    let commit = stamp::git(root, None, &args)?;
    stamp::git(root, None, &["update-ref", reference, &commit])?;
    Some((commit, head))
}

fn write(at: &Path, stamp: &Stamp, out: &mut String) -> bool {
    let text = stamp::recorded(stamp).to_string() + "\n";
    let target = at.join(stamp::FILE);
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

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

fn note(out: &mut String, why: &str) {
    let _ = writeln!(out, "klin: NOTE: {why}.");
}
