use std::cmp::Reverse;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::config::file::MEASUREMENT_LOST;
use crate::engine::catalogue;
use crate::hook::journal;
use crate::hook::turn;

/// Which lines one report reads.
#[derive(Clone, Copy)]
pub enum Scope {
    Turn,
    Session,
    Since(u64),
}

/// How one regression ended, read from the lines after the blocked stop that flagged it. The
/// writer never stores it, so a later reader can change the rule without rewriting history.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    FixedNext,
    FixedLater,
    ConfigChanged,
    SetAside,
    NotCompared,
    Open,
    AskedOnce,
}

impl Outcome {
    /// Whether the report counts this as a regression klin caught. A deleted test klin let
    /// through once the agent said why is a question the person answers, not a regression, so it
    /// stays out of the count and keeps its own audit entry.
    pub fn counted(self) -> bool {
        self != Outcome::AskedOnce
    }
}

/// What keys one regression across stops. A finding's `id` of 11.7 is the identity where the
/// record carries one. It hashes the check, the path and the declaration text, so a renamed path
/// is a different id, and the reader stays conservative rather than merge two ids whose text
/// resembles each other. A record that carries no id falls back to the smallest key its
/// recorded fields allow, so the check is neither dropped nor merged with the finding beside it.
/// Spec 13.2, ADR 0034.
#[derive(Clone, PartialEq, Eq)]
enum Identity {
    Id(String),
    Site {
        gate: String,
        file: String,
        line: Option<u64>,
        text: String,
    },
}

fn identity(site: &Value) -> Identity {
    match site.get("id").and_then(Value::as_str) {
        Some(id) if !id.is_empty() => Identity::Id(id.to_string()),
        _ => Identity::Site {
            gate: check_of(site).to_string(),
            file: word(site, "file").to_string(),
            line: site.get("line").and_then(Value::as_u64),
            text: word(site, "text").to_string(),
        },
    }
}

/// One regression: one finding site that a blocked stop put in front of the agent because it was
/// new or worse than the base, and everything the journal proves about what became of it. The
/// same site over four blocked stops is one of these, with its latest outcome.
pub struct Regression {
    identity: Identity,
    pub check: String,
    pub id: Option<String>,
    pub file: String,
    pub line: Option<u64>,
    pub text: String,
    pub values: Value,
    pub remedy: String,
    /// The time of the blocked stop that first flagged the site.
    pub first: u64,
    /// The time of the last line that said anything about it.
    pub last: u64,
    /// The prompt counter that stop ran under, which groups one chapter of `--all`.
    pub chapter: u64,
    pub prompt: Option<String>,
    /// The config hash in force when the site was first flagged. Spec 11.4.
    config: String,
    /// The semantics version of the capability that flagged it, which a later measurement must
    /// share to prove a fix. Spec 8.3.
    semantics: Option<u64>,
    pub outcome: Outcome,
    /// How many stops measured the gate after the site was first flagged.
    pub tries: usize,
}

impl Regression {
    fn opened(site: &Value, stop: &Value, chapter: u64, prompt: Option<String>) -> Regression {
        Regression {
            identity: identity(site),
            check: check_of(site).to_string(),
            id: site
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string),
            file: word(site, "file").to_string(),
            line: site.get("line").and_then(Value::as_u64),
            text: word(site, "text").to_string(),
            values: site.get("values").cloned().unwrap_or(Value::Null),
            remedy: word(site, "remedy").to_string(),
            first: at(stop),
            last: at(stop),
            chapter,
            prompt,
            config: word(stop, "config_hash").to_string(),
            semantics: semantics(stop, check_of(site)),
            outcome: Outcome::Open,
            tries: 0,
        }
    }

    fn close(&mut self, outcome: Outcome, time: u64) {
        self.outcome = outcome;
        self.last = time;
    }

    /// How a site that went from a measurement reads. The `config_hash` of 11.4 is what tells a
    /// code fix from a policy change: where it moved before the site went, the journal proves the
    /// regression is gone and not that anyone fixed anything.
    fn resolved(&self, hash: &str) -> Outcome {
        match (hash == self.config, self.tries) {
            (false, _) => Outcome::ConfigChanged,
            (true, 1) => Outcome::FixedNext,
            (true, _) => Outcome::FixedLater,
        }
    }
}

pub const DAY: u64 = 86_400;

pub fn clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

pub fn at(line: &Value) -> u64 {
    line.get("time").and_then(Value::as_u64).unwrap_or_default()
}

pub fn kind(line: &Value) -> &str {
    line.get("kind").and_then(Value::as_str).unwrap_or_default()
}

pub fn list<'a>(line: &'a Value, key: &str) -> &'a [Value] {
    match line.get(key).and_then(Value::as_array) {
        Some(entries) => entries,
        None => &[],
    }
}

pub fn word<'a>(record: &'a Value, key: &str) -> &'a str {
    record.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// One list of the check document a Stop line holds under `result`. Spec 13.1.
pub fn held<'a>(line: &'a Value, key: &str) -> &'a [Value] {
    line.get("result").map_or(&[], |result| list(result, key))
}

/// The check a record names, and `measurement-lost` for the built-in row's records, which name
/// none. Spec 11.7.
fn check_of(record: &Value) -> &str {
    match record.get("check").and_then(Value::as_str) {
        Some(check) => check,
        None => MEASUREMENT_LOST,
    }
}

/// The semantics version under which this Stop measured the check, and `None` for the built-in
/// row, which has no producer of its own. Spec 8.2.
fn semantics(line: &Value, check: &str) -> Option<u64> {
    held(line, "measurements")
        .iter()
        .find(|record| word(record, "check") == check)
        .and_then(|record| record["basis"]["producer"]["semantics_version"].as_u64())
}

fn blocked(line: &Value) -> bool {
    line.get("hook")
        .and_then(|hook| hook.get("blocked"))
        .and_then(Value::as_bool)
        .unwrap_or_default()
}

/// Whether this stop itself spent a gate block, which is where a regression opens: a build block
/// puts no gate finding in front of the agent. A line an older klin wrote names no `gate_block`,
/// and its `blocked` stands in. ADR 0034, ADR 0052.
fn gate_blocked(line: &Value) -> bool {
    match line.get("hook").and_then(|hook| hook.get("gate_block")) {
        Some(block) => !block.is_null(),
        None => blocked(line),
    }
}

/// The lines a scope holds, oldest first. Spec 11.5.
pub fn scoped(lines: &[Value], scope: Scope, root: &Path, now: u64) -> Vec<Value> {
    match scope {
        Scope::Since(days) => {
            let since = now.saturating_sub(days.saturating_mul(DAY));
            lines
                .iter()
                .filter(|line| at(line) >= since)
                .cloned()
                .collect()
        }
        Scope::Turn => this_turn(lines, turn::taken_at(root)),
        Scope::Session => this_session(lines),
    }
}

/// The lines at or after the time the turn stamp was taken, and none where no stamp is readable.
/// Spec 6.2, 13.2.
fn this_turn(lines: &[Value], since: Option<u64>) -> Vec<Value> {
    let Some(since) = since else {
        return Vec::new();
    };
    lines
        .iter()
        .filter(|line| at(line) >= since)
        .cloned()
        .collect()
}

/// The lines carrying the newest session id. Spec 13.2.
fn this_session(lines: &[Value]) -> Vec<Value> {
    let Some(newest) = lines
        .iter()
        .rev()
        .map(|line| word(line, "session"))
        .find(|session| !session.is_empty())
    else {
        return Vec::new();
    };
    let first = lines
        .iter()
        .position(|line| word(line, "session") == newest)
        .unwrap_or_default();
    lines[first..]
        .iter()
        .filter(|line| word(line, "session") == newest)
        .cloned()
        .collect()
}

/// One pass over the window's lines: the regressions it has opened, the slots of those no later
/// line has closed yet, and whether the last Stop wrote `unjudged`, whose stamp the next prompt
/// moves.
struct Pass {
    held: Vec<Regression>,
    live: Vec<usize>,
    unjudged: bool,
}

/// The regressions the lines hold, newest first, with no I/O, no clock and no formatting. One
/// per finding identity, whichever blocked stops flagged it, carrying its latest outcome.
pub fn regressions(lines: &[Value]) -> Vec<Regression> {
    let mut pass = Pass {
        held: Vec::new(),
        live: Vec::new(),
        unjudged: false,
    };
    for (index, line) in lines.iter().enumerate() {
        match kind(line) {
            "prompt" if pass.unjudged => {
                pass.set_aside(at(line));
                pass.unjudged = false;
            }
            "stop" => {
                match line["advisory"].is_string() {
                    true => pass.set_aside(at(line)),
                    false => pass.settle(line),
                }
                pass.unjudged = word(line, "verdict") == "unjudged";
                if gate_blocked(line) {
                    pass.flag(lines, index);
                }
            }
            _ => {}
        }
    }
    pass.held.sort_by_key(|one| Reverse(one.first));
    pass.held
}

impl Pass {
    /// The stamp moved with no Stop judging the window: an advisory Stop took a fresh one, or a
    /// prompt moved an `unjudged` one. That makes every regression still open unknown: not fixed,
    /// and not proven to be in the tree, because the report never looks at the tree. Spec 13.2.
    fn set_aside(&mut self, time: u64) {
        for slot in self.live.drain(..) {
            self.held[slot].close(Outcome::SetAside, time);
        }
    }

    /// What this stop said about the regressions open before it. A check the stop ran no row for
    /// did not run, and a row whose execution erred measured nothing: neither ends a regression
    /// and neither counts as a try. A site this stop let through after klin asked is a question
    /// and never a fix. A site gone from a measurement under another semantics version is not
    /// compared, and never a fix. Spec 8.3, 13.2.
    fn settle(&mut self, line: &Value) {
        let (time, hash) = (at(line), word(line, "config_hash"));
        let held = &mut self.held;
        self.live.retain(|slot| {
            let one = &mut held[*slot];
            if let_through(line, one) {
                one.close(Outcome::AskedOnce, time);
                return false;
            }
            if !measured(line, &one.check) {
                return true;
            }
            one.tries += 1;
            let measured_under = semantics(line, &one.check);
            if carries(line, &one.identity) {
                one.last = time;
                one.semantics = measured_under;
                return true;
            }
            let outcome = match measured_under == one.semantics {
                true => one.resolved(hash),
                false => Outcome::NotCompared,
            };
            one.close(outcome, time);
            false
        });
    }

    /// The sites this blocked stop put in front of the agent. One the window already holds keeps
    /// its regression, so a site that blocks four stops running counts once, and one flagged
    /// again after a reset leaves the set-aside state for its latest one.
    fn flag(&mut self, lines: &[Value], index: usize) {
        let line = &lines[index];
        let chapter = line
            .get("prompt")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let prompt = excerpt(lines, index);
        for site in failures(line) {
            match self.slot(&identity(site)) {
                Some(slot) => self.reopen(slot, at(line)),
                None => {
                    self.live.push(self.held.len());
                    self.held
                        .push(Regression::opened(site, line, chapter, prompt.clone()));
                }
            }
        }
    }

    fn slot(&self, identity: &Identity) -> Option<usize> {
        self.held.iter().position(|one| one.identity == *identity)
    }

    fn reopen(&mut self, slot: usize, time: u64) {
        self.held[slot].last = time;
        if !self.live.contains(&slot) {
            self.held[slot].outcome = Outcome::Open;
            self.live.push(slot);
        }
    }
}

/// The failing findings of one stop: new or worse than the base, at a file. Spec 11.7.
fn failures(line: &Value) -> impl Iterator<Item = &Value> {
    held(line, "findings").iter().filter(|site| {
        matches!(word(site, "outcome"), "new" | "worsened") && !word(site, "file").is_empty()
    })
}

/// Whether this stop still recorded the site. A stop lists what it found, so a site missing from
/// a stop that measured its gate is a site that went, and two sites under one failing gate end
/// apart from each other.
fn carries(line: &Value, held: &Identity) -> bool {
    failures(line).any(|site| identity(site) == *held)
}

/// Whether this stop ran the check and measured something. A check the stop carries no active
/// row for did not run, because a tree that does not build runs none, and a check whose
/// execution erred measured nothing. Neither says the site went. The built-in row of lost files
/// measured wherever the run did. Spec 11.7.
fn measured(line: &Value, check: &str) -> bool {
    if check == MEASUREMENT_LOST {
        return line["result"]["measurement"].is_string();
    }
    held(line, "capabilities").iter().any(|row| {
        word(row, "name") == check
            && word(row, "state") == "active"
            && word(row, "execution") != "error"
    })
}

/// Whether this stop recorded the site as one it let through after an earlier stop asked about
/// it, or as a test function that went with its file, which is a note and not a finding.
/// Spec 8.2.
fn let_through(line: &Value, one: &Regression) -> bool {
    held(line, "notes").iter().any(|note| {
        word(note, "check") == one.check
            && match word(note, "kind") {
                "deleted" => true,
                "note" => note.get("line").is_some(),
                _ => false,
            }
            && word(note, "file") == one.file
            && note.get("line").and_then(Value::as_u64) == one.line
    })
}

/// The excerpt of the prompt the stop at `index` ran under: the last prompt line before it that
/// carries the stop's counter. None where the journal recorded no excerpt. Spec 11.4.
fn excerpt(lines: &[Value], index: usize) -> Option<String> {
    let counter = lines[index].get("prompt")?;
    lines[..index]
        .iter()
        .rev()
        .find(|line| kind(line) == "prompt" && line.get("prompt") == Some(counter))
        .map(|line| word(line, "text"))
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// What the window's regressions came to, counted once each.
#[derive(Default)]
pub struct Counts {
    pub caught: usize,
    pub open: usize,
    pub fixed: usize,
    pub config_changed: usize,
    pub set_aside: usize,
    pub not_compared: usize,
}

impl Counts {
    pub fn of(held: &[Regression]) -> Counts {
        let many = |outcome| held.iter().filter(|one| one.outcome == outcome).count();
        Counts {
            caught: held.iter().filter(|one| one.outcome.counted()).count(),
            open: many(Outcome::Open),
            fixed: many(Outcome::FixedNext) + many(Outcome::FixedLater),
            config_changed: many(Outcome::ConfigChanged),
            set_aside: many(Outcome::SetAside),
            not_compared: many(Outcome::NotCompared),
        }
    }
}

/// What the journal proves about the regressions that went: each was flagged and later absent
/// from a measurement. It does not prove who edited the code, so no sentence here names an
/// author. Where klin did not measure the window whole, the claim is about what the report knows.
pub fn mended(fixed: usize, caught: usize, whole: bool) -> Option<String> {
    match (fixed, whole) {
        (0, _) => None,
        (_, true) => Some(proven(fixed, fixed == caught)),
        (_, false) => Some(partial(fixed, fixed == caught)),
    }
}

fn proven(fixed: usize, all: bool) -> String {
    match (all, fixed) {
        (true, 1) => "It was fixed after klin flagged it.".into(),
        (true, _) => format!("All {fixed} were fixed after klin flagged them."),
        (false, 1) => "1 was fixed after klin flagged it.".into(),
        (false, _) => format!("{fixed} were fixed after klin flagged them."),
    }
}

fn partial(fixed: usize, all: bool) -> String {
    match (all, fixed) {
        (true, 1) => "The one known regression was fixed.".into(),
        (true, _) => format!("All {fixed} known regressions were fixed."),
        (false, 1) => "1 known regression was fixed.".into(),
        (false, _) => format!("{fixed} known regressions were fixed."),
    }
}

/// The journal a stop's telling needs and no more: back to the seven-day cutoff the week's
/// headline reads, or to the turn stamp where the turn reaches further back. The prompt event
/// that appends a prompt line takes the stamp after it, and a journal time is a whole
/// second, so the stamp's own second is not the bound and the second before it is. A worktree
/// holding no readable stamp is the one case nothing bounds, and it reads the whole file.
/// Spec 9.5, 11.4.
pub fn stop_tail(root: &Path) -> journal::Tail {
    let week = clock().saturating_sub(7 * DAY);
    let cutoff = turn::taken_at(root).map_or(0, |taken| taken.saturating_sub(1).min(week));
    journal::tail(root, cutoff)
}

/// What the turn end tells the person on a stop nothing blocks: the turn line, and beside it at
/// most once every seven days the week's line, each with the name the journal records it under.
/// `this` is the stop's own line, which the journal does not hold yet. It reads the bounded tail
/// the stop already took and never the whole journal. Spec 9.5.
pub fn turn_end(root: &Path, tail: journal::Tail, this: Value) -> Vec<(&'static str, String)> {
    let mut lines = tail.lines;
    lines.push(this);
    let now = clock();
    let turn = regressions(&scoped(&lines, Scope::Turn, root, now));
    let Some(said) = turn_line(&turn) else {
        return Vec::new();
    };
    let mut parts = vec![("turn", said)];
    if weekly(&lines, tail.older, now) {
        parts.push((
            "weekly",
            weekly_line(&scoped(&lines, Scope::Since(7), root, now)),
        ));
    }
    parts
}

/// The turn in one sentence: what klin caught and what became of it, or what still needs the
/// person. Nothing for a turn that caught none, and never a claim about who wrote the fix. Where
/// every open regression is a public-api break, it names the problem and not the count, because
/// one intended removal can be several breaks. ADR 0054.
fn turn_line(held: &[Regression]) -> Option<String> {
    let counts = Counts::of(held);
    let public_api = held
        .iter()
        .filter(|one| one.outcome == Outcome::Open)
        .all(|one| one.check == catalogue::PUBLIC_API);
    match (counts.open, counts.caught) {
        (0, 0) => None,
        (0, caught) => Some(caught_this("this turn", caught, counts.fixed)),
        (_, _) if public_api => Some(
            "Public API compatibility breaks still need your attention. `klin report` shows them."
                .into(),
        ),
        (1, _) => Some("1 regression still needs your attention. `klin report` shows it.".into()),
        (open, _) => Some(format!(
            "{open} regressions still need your attention. `klin report` shows them."
        )),
    }
}

fn caught_this(when: &str, caught: usize, fixed: usize) -> String {
    let said = format!("klin caught {} {when}.", counted(caught));
    match mended(fixed, caught, true) {
        Some(mended) => format!("{said} {mended}"),
        None => said,
    }
}

/// Whether the week's line is due: no line of the last seven days carried it, and the journal
/// reaches back a week, so there is a week to tell. `older` is the bounded reader's word that the
/// journal holds a line before the lines it returned.
fn weekly(lines: &[Value], older: bool, now: u64) -> bool {
    let week = now.saturating_sub(7 * DAY);
    let told = |line: &Value| list(line, "told").iter().any(|part| part == "weekly");
    (older || lines.first().is_some_and(|first| at(first) <= week))
        && !lines.iter().any(|line| at(line) > week && told(line))
}

/// The week in one sentence, with the command that lists it, so a person who never types the
/// command learns it exists. Spec 9.5.
fn weekly_line(week: &[Value]) -> String {
    let held = regressions(week);
    let counts = Counts::of(&held);
    let rest = match counts.fixed == counts.caught {
        true => "`klin report --since 7d` shows them.",
        false => "`klin report --since 7d` shows the rest.",
    };
    format!(
        "{} {rest}",
        caught_this("in the last seven days", counts.caught, counts.fixed)
    )
}

/// The counted unit in the person's words. Spec 11.5, CONTEXT.md.
pub fn counted(count: usize) -> String {
    match count {
        1 => "1 regression".to_string(),
        _ => format!("{count} regressions"),
    }
}
