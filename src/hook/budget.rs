use std::path::Path;

use serde_json::{Value, json};

use crate::sys::write::{AtomicWrite, atomic_write};
use crate::{hook::turn, sys::state, window::stamp};

/// Where klin records what one prompt already spent, so the stop that follows knows how many
/// build blocks and gate blocks are left. In the state directory, which an agent does not
/// empty. ADR 0019, ADR 0022, ADR 0052.
const BUILD_BLOCKED: &str = "build-blocked";
/// The index the build stamp hashes the tree through, apart from the turn stamp's own.
const BUILD_INDEX: &str = "build-index";
/// How many stops one prompt's build failures may block. klin bounds this itself, because the
/// host documents no cap of its own. ADR 0022, spec 9.3.
const BLOCKS: u64 = 8;
/// How many stops one prompt's gate failures may block. The second needs a tree that changed
/// since the first. ADR 0052, spec 9.3.
const GATE_BLOCKS: u64 = 2;
const UNWRITABLE: &str = "count-unwritable";

/// Why a stop after a gate block spends none: klin has no tree of its own to compare against.
const UNPROVEN: &str = "klin holds no record of the tree the last gate block saw, so it cannot \
    tell whether this stop changed it";
/// Why a stop spends no gate block when its record would not write. ADR 0052.
const UNRECORDED: &str = "klin could not record a gate block, so nothing would bound it";
/// Why a stop that lost the state lock spends no gate block. Spec 6.5.
const LOCKED: &str = "another klin event held the state directory, so this stop could not count a \
    gate block";
/// Why a stop over the tree the last gate block saw spends none. ADR 0052.
const UNCHANGED_SINCE_GATE: &str = "the tree did not change since the last gate block";

/// One stop's per-prompt block budget: the host session that asks, whether the stop
/// `continued` a chain of messages its host submitted by itself, and whether it `lost` the state
/// lock, so another stop may be writing the count. Every block is recorded before it is
/// delivered, and a block klin cannot record is not spent. Spec 6.5, 9.3, 16.3, ADR 0052.
pub struct Budget<'a> {
    pub root: &'a Path,
    pub session: Option<&'a str>,
    pub continued: bool,
    pub lost: bool,
}

/// What this prompt already spent, as the journal line records it. Spec 13.1.
pub struct Spent {
    pub prompt: u64,
    pub builds: u64,
    pub gate_blocks: u64,
}

/// What a build failure at this stop spends: the block it took and its number in this turn, no
/// block because the tree did not change since the last one, or no block because klin could
/// not record one.
pub enum BuildBlock {
    Spent(u64),
    Unchanged,
    Unbounded,
}

/// What a gate failure at this stop spends: the gate block it took, with its number under this
/// prompt, or no block and the reason the report gives.
pub enum GateBlock {
    Spent(u64),
    Passed(String),
}

impl Budget<'_> {
    /// A stop that no automatic message came before opens its prompt's budget, even where it
    /// spends none, so a later stop of the chain it opens inherits that budget and never one this
    /// session left under an earlier prompt. A record another session left stays: no chain of
    /// this session inherits it, and its own chain still may. Spec 9.3, ADR 0052.
    pub fn open(&self, flags: &mut Vec<&'static str>) {
        let Ok(at) = state::ready(self.root) else {
            return;
        };
        let prompt = turn::prompts(&at);
        let stale = Record::read(&at)
            .is_some_and(|held| held.taken_by(self.session) && !held.taken_under(prompt));
        if self.lost || self.continued || !stale {
            return;
        }
        if !counted(&at, &self.count(&at)) {
            flags.push(UNWRITABLE);
        }
    }

    /// What this prompt spent so far, and nothing without a state directory to read it from.
    pub fn spent(&self) -> Option<Spent> {
        let at = state::ready(self.root).ok()?;
        let count = self.count(&at);
        Some(Spent {
            prompt: count.prompt,
            builds: count.builds,
            gate_blocks: count.gate_blocks,
        })
    }

    /// The build block a failing build may spend. None at a stop that lost the state lock,
    /// because another stop may be writing the count; the lock's own NOTE tells it. Spec 6.5.
    pub fn build_block(&self, flags: &mut Vec<&'static str>) -> BuildBlock {
        match self.lost {
            true => BuildBlock::Unbounded,
            false => self.raised(flags),
        }
    }

    /// The block this build failure spends. `Unchanged` when the working tree is the one the last
    /// block was taken over, because blocking again on a tree the agent did not touch teaches it
    /// nothing. `Unbounded` when klin could not record the block, either because the state
    /// directory is gone or because the record itself would not write: neither count could bound
    /// the blocks, so the NOTE names the write that failed and the stop is not blocked. Spec 14.
    fn raised(&self, flags: &mut Vec<&'static str>) -> BuildBlock {
        let at = match state::ready(self.root) {
            Ok(at) => at,
            Err(why) => return unbounded(&why, flags),
        };
        let held = self.count(&at);
        let tree = working_tree(self.root, &at);
        if held.builds > 0 && tree.is_some() && tree == held.build_tree {
            return BuildBlock::Unchanged;
        }
        let count = Count {
            builds: held.builds + 1,
            build_tree: tree,
            ..held
        };
        match counted(&at, &count) {
            true => BuildBlock::Spent(count.builds),
            false => unbounded(
                &format!("{} could not be written", at.join(BUILD_BLOCKED).display()),
                flags,
            ),
        }
    }

    /// The gate block this failure may take, recorded before it is delivered. The first is
    /// free. The second needs a tree that differs from the one klin recorded for the first, so a
    /// stop over the tree the agent left alone reports and lets the turn end. None comes after
    /// the second, none at a stop that lost the state lock, and none at all without a state
    /// directory to record it in. A block klin cannot record could not be bounded, because the
    /// next stop would read it as never spent and take it again, so it becomes a report.
    ///
    /// The host's flag says a block happened, never which tree it saw: where klin's record holds
    /// no gate block, the flag counts as one klin never recorded, so the stop spends none, and
    /// it never proves a second. After a build block that flag is true while no gate block is
    /// spent, so it counts only where no build block was spent either. Spec 16.3, ADR 0052.
    pub fn gate_block(&self, blocked_before: bool, flags: &mut Vec<&'static str>) -> GateBlock {
        if self.lost {
            return GateBlock::Passed(LOCKED.to_string());
        }
        let Ok(at) = state::ready(self.root) else {
            return GateBlock::Passed(UNRECORDED.to_string());
        };
        let count = self.count(&at);
        let (number, tree) = match next(self.root, &count, &at, blocked_before) {
            Ok(next) => next,
            Err(why) => return GateBlock::Passed(why),
        };
        let recorded = Count {
            gate_blocks: number,
            gate_tree: tree,
            ..count
        };
        if counted(&at, &recorded) {
            return GateBlock::Spent(number);
        }
        flags.push(UNWRITABLE);
        GateBlock::Passed(UNRECORDED.to_string())
    }

    /// The record as this prompt left it. A stop that `continued` a chain of messages its host
    /// submitted by itself keeps its own session's record whatever prompt it was taken under:
    /// another hook's message may have won the host's merge, and klin read it as a person's
    /// prompt, but the chain belongs to the prompt that opened it, and the stop that opened it
    /// wrote the record (`open`). The record is written back under the current counter, so every
    /// later stop of the chain reads it too. Spec 9.3, ADR 0052.
    fn count(&self, at: &Path) -> Count {
        let prompt = turn::prompts(at);
        let held = Record::read(at)
            .filter(|held| {
                held.taken_under(prompt) || self.continued && held.taken_by(self.session)
            })
            .unwrap_or_default();
        Count {
            prompt,
            session: self.session.map(str::to_string),
            builds: held.builds.unwrap_or_default(),
            build_tree: held.build_tree.or(held.tree),
            gate_blocks: held.gate_blocks.unwrap_or(u64::from(held.gate_spent)),
            gate_tree: held.gate_tree,
        }
    }
}

impl BuildBlock {
    /// Whether the stop blocks, what the hook says about a tree that does not build, and the note
    /// that says why the stop does not block. The words name the bound from `BLOCKS`, so the cap
    /// and the words for it cannot drift apart, and the block this stop spends, so the agent
    /// reads how many are left. Spec 9.3.
    pub fn outcome(&self) -> (bool, String, Option<String>) {
        const SAID: &str = "the tree does not build, so no gate ran";
        match self {
            BuildBlock::Spent(builds) if *builds <= BLOCKS => (
                true,
                format!(
                    "{SAID} (a stop that changed the tree blocks until it does, block {builds} \
                     of {BLOCKS} in this turn)"
                ),
                None,
            ),
            BuildBlock::Spent(_) => (
                false,
                SAID.to_string(),
                Some(format!(
                    "the build has blocked {BLOCKS} stops under this prompt, so klin stops \
                     blocking; the failure stands."
                )),
            ),
            BuildBlock::Unchanged => (
                false,
                SAID.to_string(),
                Some(
                    "the tree did not change since the stop klin last blocked, so klin does not \
                     block again; the failure stands."
                        .to_string(),
                ),
            ),
            BuildBlock::Unbounded => (
                false,
                SAID.to_string(),
                Some(
                    "klin could not safely spend a build block, so this build failure blocks \
                     nothing."
                        .to_string(),
                ),
            ),
        }
    }
}

/// The gate block's place in this turn's cap, as the lead of a blocked stop names it, so the
/// cap and the words for it cannot drift apart.
pub fn gate_numbered(number: u64) -> String {
    format!("gate block {number} of {GATE_BLOCKS} in this turn")
}

/// The next gate block and the tree it is taken over, or why the stop spends none.
fn next(
    root: &Path,
    count: &Count,
    at: &Path,
    blocked_before: bool,
) -> Result<(u64, Option<String>), String> {
    if count.gate_blocks >= GATE_BLOCKS {
        return Err(format!(
            "the gate has blocked {GATE_BLOCKS} stops under this prompt, which is the most it \
             blocks"
        ));
    }
    let flagged = count.builds == 0 && blocked_before;
    if count.gate_blocks == 0 && !flagged {
        return Ok((1, working_tree(root, at)));
    }
    let Some(before) = count.gate_tree.as_deref() else {
        return Err(UNPROVEN.to_string());
    };
    match working_tree(root, at) {
        Some(tree) if tree == before => Err(UNCHANGED_SINCE_GATE.to_string()),
        Some(tree) => Ok((count.gate_blocks + 1, Some(tree))),
        None => Err(UNPROVEN.to_string()),
    }
}

fn unbounded(why: &str, flags: &mut Vec<&'static str>) -> BuildBlock {
    flags.push(UNWRITABLE);
    eprintln!(
        "klin: NOTE: {why} — so no count could bound the build blocks, and this build failure \
         blocks nothing."
    );
    BuildBlock::Unbounded
}

/// The working tree as the build stamp records it, hashed through the build stamp's own index.
fn working_tree(root: &Path, at: &Path) -> Option<String> {
    stamp::tree_through(root, &at.join(BUILD_INDEX))
}

/// The build stamp: one record per prompt. The prompt counter of the turn file it was taken
/// under, the host session that took it, how many stops a build failure and a gate failure
/// already blocked, and the tree each kind of block last saw. The two kinds never share a count
/// or a tree. A record taken under an earlier prompt reads as zero, so every prompt gets the
/// whole budget. Spec 16.3, ADR 0052.
struct Count {
    prompt: u64,
    session: Option<String>,
    builds: u64,
    /// The working tree the last build block was taken over, so a stop that changed nothing
    /// since is reported and not blocked again. ADR 0048.
    build_tree: Option<String>,
    gate_blocks: u64,
    /// The working tree the last gate block was taken over. Only a tree klin recorded here can
    /// prove that a later stop changed it. ADR 0052.
    gate_tree: Option<String>,
}

/// The record as it stands on disk. A field that is missing or holds another type reads as
/// absent. A record an older klin wrote names its build tree `tree` and its one gate block
/// `gate_spent`, and names no gate tree or session, so it can never prove a second gate block or
/// carry into a chain. Spec 16.3.
#[derive(Default)]
struct Record {
    prompt: Option<u64>,
    session: Option<String>,
    builds: Option<u64>,
    build_tree: Option<String>,
    gate_blocks: Option<u64>,
    gate_tree: Option<String>,
    tree: Option<String>,
    gate_spent: bool,
}

impl Record {
    fn read(at: &Path) -> Option<Record> {
        let text = std::fs::read_to_string(at.join(BUILD_BLOCKED)).ok()?;
        let held = serde_json::from_str::<Value>(&text).ok()?;
        let number = |key: &str| held.get(key).and_then(Value::as_u64);
        let text = |key: &str| held.get(key)?.as_str().map(str::to_string);
        Some(Record {
            prompt: number("prompt"),
            session: text("session"),
            builds: number("builds"),
            build_tree: text("build_tree"),
            gate_blocks: number("gate_blocks"),
            gate_tree: text("gate_tree"),
            tree: text("tree"),
            gate_spent: held.get("gate_spent").and_then(Value::as_bool) == Some(true),
        })
    }

    fn taken_under(&self, prompt: u64) -> bool {
        self.prompt == Some(prompt)
    }

    fn taken_by(&self, session: Option<&str>) -> bool {
        session.is_some_and(|session| self.session.as_deref() == Some(session))
    }
}

/// Whether the record reached the disk. A count klin cannot write bounds nothing, so the
/// caller reports the failure and does not block on it. Spec 14.
fn counted(at: &Path, count: &Count) -> bool {
    let text = json!({
        "prompt": count.prompt,
        "session": count.session,
        "builds": count.builds,
        "build_tree": count.build_tree,
        "gate_blocks": count.gate_blocks,
        "gate_tree": count.gate_tree,
    })
    .to_string()
        + "\n";
    atomic_write(AtomicWrite {
        target: &at.join(BUILD_BLOCKED),
        bytes: text.as_bytes(),
        keep_mode_from: None,
    })
    .is_ok()
}
