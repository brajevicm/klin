use std::cmp::Reverse;
use std::fmt::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::{check, git::Repo, journal, turn};

/// What needs the person's attention, and what klin was worth. The report reads the journal of
/// 11.4 and nothing else: it re-runs no gate, reads no working tree and keeps no clock of its
/// own. It turns the lines into one regression per finding identity, decides the outcome and the
/// measurement confidence once, and hands that to one of three surfaces. The default says what
/// matters now, `--all` carries the evidence, `--json` carries the facts. The words a gate is
/// named by come from the catalogue and never from a table here. Spec 11.5.
#[derive(clap::Args)]
pub struct Args {
    /// The window to report, as a number of days, such as 30d. Seven days by default
    #[arg(long, value_name = "Nd", conflicts_with_all = ["turn", "session"])]
    since: Option<String>,
    /// Report the stops since the current turn stamp
    #[arg(long, conflicts_with = "session")]
    turn: bool,
    /// Report the lines of the newest session the journal holds
    #[arg(long)]
    session: bool,
    /// Print every regression of the window, the audit trail and the measurement evidence
    #[arg(long)]
    all: bool,
    /// Print the report as one JSON object
    #[arg(long)]
    json: bool,
}

/// Which lines one report reads.
#[derive(Clone, Copy)]
enum Scope {
    Turn,
    Session,
    Since(u64),
}

/// How one regression ended, read from the lines after the blocked stop that flagged it. The
/// writer never stores it, so a later reader can change the rule without rewriting history.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    FixedNext,
    FixedLater,
    ConfigChanged,
    SetAside,
    Open,
    AskedOnce,
}

impl Outcome {
    fn name(self) -> &'static str {
        match self {
            Outcome::FixedNext => "fixed-next",
            Outcome::FixedLater => "fixed-later",
            Outcome::ConfigChanged => "config-changed",
            Outcome::SetAside => "set-aside",
            Outcome::Open => "open",
            Outcome::AskedOnce => "asked-once",
        }
    }

    /// Whether the report counts this as a regression klin caught. A deleted test klin let
    /// through once the agent said why is a question the person answers, not a regression, so it
    /// stays out of the count and keeps its own audit entry.
    fn counted(self) -> bool {
        self != Outcome::AskedOnce
    }

    /// What the person reads about how it ended. The journal proves that a site was there and
    /// later absent from a measurement. It proves nothing about who edited the code, so no
    /// sentence here names an author.
    fn sentence(self, tries: usize) -> String {
        match self {
            Outcome::FixedNext => "Fixed after klin flagged it on the next measured try.".into(),
            Outcome::FixedLater => {
                format!("Fixed after klin flagged it {tries} measured tries later.")
            }
            Outcome::ConfigChanged => "Resolved after the config changed.".into(),
            Outcome::SetAside => "Set aside when you restarted.".into(),
            Outcome::Open => "Still open.".into(),
            Outcome::AskedOnce => "klin asked about it once and let it through.".into(),
        }
    }
}

/// What keys one regression across stops. A finding's `id` of 11.2 is the identity where the
/// record carries one. It hashes the gate, the path and the declaration text, so a renamed path
/// is a different id, and the reader stays conservative rather than merge two ids whose text
/// resembles each other. A record that carries no id — `doc-size` is one shape that does not —
/// falls back to the smallest key its recorded fields allow, so the gate is neither dropped nor
/// merged with the finding beside it. Spec 11.5, ADR 0034.
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
            gate: word(site, "gate").to_string(),
            file: word(site, "file").to_string(),
            line: site.get("line").and_then(Value::as_u64),
            text: word(site, "text").to_string(),
        },
    }
}

/// One regression: one finding site that a blocked stop put in front of the agent because it was
/// new or worse than the base, and everything the journal proves about what became of it. The
/// same site over four blocked stops is one of these, with its latest outcome.
struct Regression {
    identity: Identity,
    gate: String,
    id: Option<String>,
    file: String,
    line: Option<u64>,
    text: String,
    values: Value,
    remedy: String,
    /// The time of the blocked stop that first flagged the site.
    first: u64,
    /// The time of the last line that said anything about it.
    last: u64,
    /// The prompt counter that stop ran under, which groups one chapter of `--all`.
    chapter: u64,
    prompt: Option<String>,
    /// The config hash in force when the site was first flagged. Spec 11.4.
    config: String,
    outcome: Outcome,
    /// How many stops measured the gate after the site was first flagged.
    tries: usize,
    /// Where, in the lines the report read, the line that ended it sits.
    ended: Option<usize>,
}

impl Regression {
    fn opened(site: &Value, stop: &Value, chapter: u64, prompt: Option<String>) -> Regression {
        Regression {
            identity: identity(site),
            gate: word(site, "gate").to_string(),
            id: site
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string),
            file: word(site, "file").to_string(),
            line: site.get("line").and_then(Value::as_u64),
            text: word(site, "text").to_string(),
            values: site.get("values").cloned().unwrap_or(Value::Null),
            remedy: word(site, "fix_advice").to_string(),
            first: at(stop),
            last: at(stop),
            chapter,
            prompt,
            config: word(stop, "config_hash").to_string(),
            outcome: Outcome::Open,
            tries: 0,
            ended: None,
        }
    }

    fn close(&mut self, outcome: Outcome, time: u64, index: usize) {
        self.outcome = outcome;
        self.last = time;
        self.ended = Some(index);
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

    /// Where the site is, as one token a person can paste into an editor.
    fn site(&self) -> String {
        match self.line {
            Some(line) if line > 0 => format!("{}:{line}", self.file),
            _ => self.file.clone(),
        }
    }
}

const DAY: u64 = 86_400;

/// How many open sites the default report names before it points at `--all`. The default answers
/// what needs attention now, and a fourth line is a list. Spec 11.5.
const SITES: usize = 3;

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let scope = scope(args)?;
    let (lines, skipped) = journal::read(start);
    let now = clock();
    let held = scoped(&lines, scope, start, now);
    let mut report = Report::read(&held, scope, now, skipped, lines.is_empty() && skipped == 0);
    report.confidence.unscoped = unscoped(&lines, scope, start);
    match args.json {
        true => json(out, &report, &lines, scope, now),
        false => text(out, args, start, &report),
    }
    Ok(0)
}

fn scope(args: &Args) -> Result<Scope, Error> {
    if args.turn {
        return Ok(Scope::Turn);
    }
    if args.session {
        return Ok(Scope::Session);
    }
    window(args.since.as_deref()).map(Scope::Since)
}

fn window(since: Option<&str>) -> Result<u64, Error> {
    let Some(text) = since else {
        return Ok(7);
    };
    let digits = text.strip_suffix('d').unwrap_or(text);
    match digits.parse::<u64>() {
        Ok(days) if days > 0 => Ok(days),
        _ => Err(Error(format!(
            "--since takes a number of days, such as 30d, and not {text}"
        ))),
    }
}

fn clock() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

fn at(line: &Value) -> u64 {
    line.get("time").and_then(Value::as_u64).unwrap_or_default()
}

fn kind(line: &Value) -> &str {
    line.get("kind").and_then(Value::as_str).unwrap_or_default()
}

fn list<'a>(line: &'a Value, key: &str) -> &'a [Value] {
    match line.get(key).and_then(Value::as_array) {
        Some(entries) => entries,
        None => &[],
    }
}

fn word<'a>(record: &'a Value, key: &str) -> &'a str {
    record.get(key).and_then(Value::as_str).unwrap_or_default()
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

/// Whether klin could tell where the window the person asked for begins. `--turn` needs a turn
/// stamp it can read, and `--session` needs a session id somewhere in the journal. Without one
/// the scope holds no line at all, and a report that never found its window must not read as a
/// quiet one. A window of days always begins somewhere. Spec 6.2, 11.5.
fn unscoped(lines: &[Value], scope: Scope, root: &Path) -> Option<String> {
    match scope {
        Scope::Since(_) => None,
        Scope::Turn => turn::taken_at(root)
            .is_none()
            .then(|| "klin could not tell where this turn began".to_string()),
        Scope::Session => lines
            .iter()
            .all(|line| word(line, "session").is_empty())
            .then(|| "klin could not tell where this session began".to_string()),
    }
}

/// The lines a scope holds, oldest first. Spec 11.5.
fn scoped(lines: &[Value], scope: Scope, root: &Path, now: u64) -> Vec<Value> {
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
/// A journal time is a whole second, so a line from the second a reset moved the stamp in would
/// read as part of the turn, and the reset's own line bounds it instead. Spec 6.2, 11.5.
fn this_turn(lines: &[Value], since: Option<u64>) -> Vec<Value> {
    let Some(since) = since else {
        return Vec::new();
    };
    let reset = lines
        .iter()
        .rposition(|line| kind(line) == "reset")
        .map_or(0, |index| index + 1);
    lines[reset..]
        .iter()
        .filter(|line| at(line) >= since)
        .cloned()
        .collect()
}

/// The lines carrying the newest session id, and the resets among them, which a person runs
/// outside any session and which still set aside the regressions before them.
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
        .filter(|line| word(line, "session") == newest || kind(line) == "reset")
        .cloned()
        .collect()
}

/// One pass over the window's lines: the regressions it has opened, and the slots of those no
/// later line has closed yet.
struct Pass {
    held: Vec<Regression>,
    live: Vec<usize>,
}

/// The regressions the lines hold, newest first, with no I/O, no clock and no formatting. One
/// per finding identity, whichever blocked stops flagged it, carrying its latest outcome.
fn regressions(lines: &[Value]) -> Vec<Regression> {
    let mut pass = Pass {
        held: Vec::new(),
        live: Vec::new(),
    };
    for (index, line) in lines.iter().enumerate() {
        match kind(line) {
            "reset" => pass.set_aside(at(line), index),
            "stop" => {
                pass.settle(line, index);
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
    /// A person moved the turn stamp. That makes every regression still open unknown: not fixed,
    /// and not proven to be in the tree, because the report never looks at the tree.
    fn set_aside(&mut self, time: u64, index: usize) {
        for slot in self.live.drain(..) {
            self.held[slot].close(Outcome::SetAside, time, index);
        }
    }

    /// What this stop said about the regressions open before it. A gate the stop ran no row for
    /// did not run, and a row that says `ERR` measured nothing: neither ends a regression and
    /// neither counts as a try. A site this stop let through after klin asked is a question and
    /// never a fix. Spec 11.2, 11.5.
    fn settle(&mut self, line: &Value, index: usize) {
        let (time, hash) = (at(line), word(line, "config_hash"));
        let held = &mut self.held;
        self.live.retain(|slot| {
            let one = &mut held[*slot];
            if let_through(line, one) {
                one.close(Outcome::AskedOnce, time, index);
                return false;
            }
            if !measured(line, &one.gate) {
                return true;
            }
            one.tries += 1;
            if carries(line, &one.identity) {
                one.last = time;
                return true;
            }
            let outcome = one.resolved(hash);
            one.close(outcome, time, index);
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
            self.held[slot].ended = None;
            self.live.push(slot);
        }
    }
}

/// The failing findings of one stop. A record that names no gate or no file is a run that could
/// not measure, and it is no regression.
fn failures(line: &Value) -> impl Iterator<Item = &Value> {
    list(line, "findings")
        .iter()
        .filter(|site| !word(site, "gate").is_empty() && !word(site, "file").is_empty())
}

/// Whether this stop still recorded the site. A stop lists what it found, so a site missing from
/// a stop that measured its gate is a site that went, and two sites under one failing gate end
/// apart from each other.
fn carries(line: &Value, held: &Identity) -> bool {
    failures(line).any(|site| identity(site) == *held)
}

/// Whether this stop ran the gate and measured something. A gate the stop carries no row for did
/// not run, because a tree that does not build runs none, and a gate that errored measured
/// nothing. Neither says the site went. Spec 11.2.
fn measured(line: &Value, gate: &str) -> bool {
    list(line, "gates")
        .iter()
        .any(|row| word(row, "name") == gate && word(row, "status") != "ERR")
}

/// Whether this stop recorded the site as one it let through after an earlier stop asked about
/// it, which is a note and not a finding. Spec 8.2.
fn let_through(line: &Value, one: &Regression) -> bool {
    list(line, "notes").iter().any(|note| {
        word(note, "gate") == one.gate
            && word(note, "outcome") == "deleted"
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

/// What the window's own records say klin did not see. A positive claim about a window klin did
/// not measure whole is the one lie the report must not tell, so the reader decides this once and
/// the report puts it above the value it found. Spec 11.5.
#[derive(Default)]
struct Confidence {
    /// Source files a grammar refused.
    unparsed: Vec<String>,
    /// Files the base measured and the working tree did not.
    lost: Vec<String>,
    /// Known-language files no structural adapter reads.
    not_measured: Vec<String>,
    /// Dependency or public-surface forms a check supports and could not resolve.
    unresolved: usize,
    /// Gates a stop ran that measured nothing.
    errored: Vec<String>,
    /// Journal lines the reader could not take.
    skipped: u64,
    /// The window the person asked for, where klin could not tell where it begins.
    unscoped: Option<String>,
}

impl Confidence {
    fn read(lines: &[Value], skipped: u64) -> Confidence {
        let mut held = Confidence {
            skipped,
            ..Confidence::default()
        };
        for line in lines {
            for note in list(line, "notes") {
                held.note(note);
            }
            for row in list(line, "gates") {
                let name = word(row, "name");
                if word(row, "status") == "ERR" && !held.errored.iter().any(|held| held == name) {
                    held.errored.push(name.to_string());
                }
            }
        }
        held
    }

    fn note(&mut self, note: &Value) {
        let file = word(note, "file").to_string();
        let held = match word(note, "outcome") {
            check::UNPARSED => &mut self.unparsed,
            check::LOST => &mut self.lost,
            check::NOT_MEASURED => &mut self.not_measured,
            check::UNRESOLVED => {
                self.unresolved += 1;
                return;
            }
            _ => return,
        };
        if !file.is_empty() && !held.contains(&file) {
            held.push(file);
        }
    }

    fn files(&self) -> usize {
        self.unparsed.len() + self.lost.len() + self.not_measured.len()
    }

    /// The one sentence the report puts above its value claim, and `None` for a window klin
    /// measured whole. The narrowest claim the records support wins, and `--all` carries the rows
    /// behind it.
    fn gap(&self) -> Option<String> {
        if self.unscoped.is_some() {
            return self.unscoped.clone();
        }
        let files = self.files();
        if files > 0 {
            return Some(self.unmeasured(files));
        }
        if !self.errored.is_empty() {
            let count = self.errored.len();
            return Some(plural(
                count,
                "check couldn't finish",
                "checks couldn't finish",
            ));
        }
        if self.unresolved > 0 {
            return Some(plural(
                self.unresolved,
                "dependency couldn't be resolved",
                "dependencies couldn't be resolved",
            ));
        }
        match usize::try_from(self.skipped).unwrap_or(usize::MAX) {
            0 => None,
            count => Some(plural(
                count,
                "journal record couldn't be read",
                "journal records couldn't be read",
            )),
        }
    }

    /// The narrowest true claim about the files klin did not measure: where every one of them is
    /// a file a grammar refused, the reader can say so and not only that something went unread.
    fn unmeasured(&self, files: usize) -> String {
        match self.lost.is_empty() && self.not_measured.is_empty() {
            true => plural(
                files,
                "source file couldn't be parsed",
                "source files couldn't be parsed",
            ),
            false => plural(files, "file wasn't measured", "files weren't measured"),
        }
    }

    fn whole(&self) -> bool {
        self.gap().is_none()
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        _ => format!("{count} {many}"),
    }
}

/// What the window's regressions came to, counted once each.
#[derive(Default)]
struct Counts {
    caught: usize,
    open: usize,
    fixed: usize,
    fixed_next: usize,
    fixed_later: usize,
    config_changed: usize,
    set_aside: usize,
    asked_once: usize,
}

impl Counts {
    fn of(held: &[Regression]) -> Counts {
        let many = |outcome| held.iter().filter(|one| one.outcome == outcome).count();
        Counts {
            caught: held.iter().filter(|one| one.outcome.counted()).count(),
            open: many(Outcome::Open),
            fixed: many(Outcome::FixedNext) + many(Outcome::FixedLater),
            fixed_next: many(Outcome::FixedNext),
            fixed_later: many(Outcome::FixedLater),
            config_changed: many(Outcome::ConfigChanged),
            set_aside: many(Outcome::SetAside),
            asked_once: many(Outcome::AskedOnce),
        }
    }

    fn record(&self) -> Map<String, Value> {
        let mut out = Map::new();
        out.insert("caught".into(), self.caught.into());
        out.insert("open".into(), self.open.into());
        out.insert("fixed-next".into(), self.fixed_next.into());
        out.insert("fixed-later".into(), self.fixed_later.into());
        out.insert("config-changed".into(), self.config_changed.into());
        out.insert("set-aside".into(), self.set_aside.into());
        out.insert("asked-once".into(), self.asked_once.into());
        out
    }
}

/// One thing the person was asked about or did. None of these is a regression: a reset and a
/// guard refusal ask nothing, and a deleted test klin let through is a question the person
/// answers. They stay out of the count and keep their own trail. Spec 11.5.
enum Audit {
    Guard {
        time: u64,
        decision: String,
        reason: String,
    },
    Reset {
        time: u64,
        set_aside: usize,
    },
    Deleted {
        time: u64,
        file: String,
        line: Option<u64>,
        text: String,
    },
}

impl Audit {
    fn time(&self) -> u64 {
        match self {
            Audit::Guard { time, .. } | Audit::Reset { time, .. } | Audit::Deleted { time, .. } => {
                *time
            }
        }
    }

    fn sentence(&self) -> String {
        match self {
            Audit::Guard {
                decision, reason, ..
            } => guarded(decision, reason),
            Audit::Reset { set_aside: 0, .. } => "You told klin to start over.".into(),
            Audit::Reset { set_aside: 1, .. } => {
                "You restarted, and 1 regression was set aside.".into()
            }
            Audit::Reset { set_aside, .. } => {
                format!("You restarted, and {set_aside} regressions were set aside.")
            }
            Audit::Deleted {
                file, line, text, ..
            } => deleted(file, *line, text),
        }
    }

    fn record(&self) -> Value {
        let (kind, decision, reason, file, line) = match self {
            Audit::Guard {
                decision, reason, ..
            } => (
                "guard",
                Some(decision.clone()),
                Some(reason.clone()),
                None,
                None,
            ),
            Audit::Reset { .. } => ("reset", None, None, None, None),
            Audit::Deleted { file, line, .. } => {
                ("asked-once", None, None, Some(file.clone()), *line)
            }
        };
        serde_json::json!({
            "time": self.time(),
            "kind": kind,
            "decision": decision,
            "reason": reason,
            "file": file,
            "line": line,
        })
    }
}

/// What a guard line's reason names, in the person's words. A reason this binary does not know
/// reads as a tool call. Spec 11.4.
const GUARDED: [(&str, &str); 7] = [
    ("config-write", "an edit to klin.json"),
    ("state-write", "an edit to klin's own state"),
    ("config-mention", "a command that named klin.json"),
    ("state-mention", "a command that named klin's own state"),
    ("init", "klin init, which only you run"),
    ("install", "klin install, which only you run"),
    ("turn-reset", "klin turn reset, which only you run"),
];

fn guarded(decision: &str, reason: &str) -> String {
    let what = GUARDED
        .iter()
        .find(|(tag, _)| *tag == reason)
        .map_or("a tool call", |(_, what)| what);
    match decision {
        "ask" => format!("klin asked you before {what}"),
        _ => format!("klin refused {what}"),
    }
}

/// A deleted test klin let through, named by its declaration line, or by its file where the whole
/// file went.
fn deleted(file: &str, line: Option<u64>, text: &str) -> String {
    let clipped = clip(text);
    let site = clipped.trim_end_matches(['{', ':', ' ']);
    match (site.is_empty(), line.filter(|line| *line > 0)) {
        (true, _) => format!("{file} deleted. The agent said why."),
        (false, Some(line)) => {
            format!("a test deleted from {file}:{line}, {site}. The agent said why.")
        }
        (false, None) => format!("a test deleted from {file}, {site}. The agent said why."),
    }
}

/// Everything the person was asked about or did in the window, newest first. Spec 11.5.
fn audit_trail(lines: &[Value], held: &[Regression]) -> Vec<Audit> {
    let mut out: Vec<Audit> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| match kind(line) {
            "guard" => Some(Audit::Guard {
                time: at(line),
                decision: word(line, "decision").to_string(),
                reason: word(line, "reason").to_string(),
            }),
            "reset" => Some(Audit::Reset {
                time: at(line),
                set_aside: held
                    .iter()
                    .filter(|one| one.ended == Some(index) && one.outcome == Outcome::SetAside)
                    .count(),
            }),
            _ => None,
        })
        .collect();
    out.extend(
        held.iter()
            .filter(|one| one.outcome == Outcome::AskedOnce)
            .map(|one| Audit::Deleted {
                time: one.last,
                file: one.file.clone(),
                line: one.line,
                text: one.text.clone(),
            }),
    );
    out.sort_by_key(|one| Reverse(one.time()));
    out
}

/// One report, read once: the window's regressions, the audit trail beside them, what klin could
/// not measure, and the activity facts. Each surface formats this and works nothing out again.
struct Report {
    scope: Scope,
    regressions: Vec<Regression>,
    audit: Vec<Audit>,
    confidence: Confidence,
    counts: Counts,
    now: u64,
    stops: usize,
    ms: u64,
    /// Whether the journal held no line at all, which is a first run and not a quiet window.
    empty: bool,
}

impl Report {
    fn read(lines: &[Value], scope: Scope, now: u64, skipped: u64, empty: bool) -> Report {
        let regressions = regressions(lines);
        Report {
            counts: Counts::of(&regressions),
            audit: audit_trail(lines, &regressions),
            confidence: Confidence::read(lines, skipped),
            regressions,
            scope,
            now,
            stops: lines.iter().filter(|line| kind(line) == "stop").count(),
            ms: lines
                .iter()
                .filter_map(|line| line.get("timing")?.get("klin_ms")?.as_u64())
                .sum(),
            empty,
        }
    }

    fn open(&self) -> Vec<&Regression> {
        self.regressions
            .iter()
            .filter(|one| one.outcome == Outcome::Open)
            .collect()
    }
}

/// The default report: what needs attention, what klin was worth, and where to look. The opening
/// state is the strongest condition the window holds, so measurement doubt outranks open
/// regressions, which outrank the uncertainty a reset left, which outranks the good news.
/// Spec 11.5.
fn text(out: &mut String, args: &Args, start: &Path, report: &Report) {
    if report.empty {
        let _ = writeln!(
            out,
            "klin is on. Your first recap appears after the agent finishes a task."
        );
        return;
    }
    let attention = attention(report);
    let named = attention.iter().any(|line| line.contains("regression"));
    for line in &attention {
        let _ = writeln!(out, "{line}");
    }
    value(out, report, named);
    sites(out, report, args.all);
    if args.all {
        history(out, start, report);
    }
}

fn attention(report: &Report) -> Vec<String> {
    let mut said = Vec::new();
    if let Some(gap) = report.confidence.gap() {
        said.push(format!("Stats may be incomplete: {gap}."));
    }
    match report.counts.open {
        0 => {}
        1 => said.push("1 regression needs your attention.".into()),
        open => said.push(format!("{open} regressions need your attention.")),
    }
    if report.counts.set_aside > 0 {
        said.push(restarted(report.counts.set_aside, report.counts.open > 0));
    }
    if said.is_empty() {
        said.push("Nothing needs your attention.".into());
    }
    said
}

/// A reset makes the regressions it caught unknown: not fixed, and not proven to be in the tree,
/// because the report reads the journal and never the tree.
fn restarted(aside: usize, after_open: bool) -> String {
    match (after_open, aside) {
        (true, 1) => "1 more was set aside when you restarted.".into(),
        (true, _) => format!("{aside} more were set aside when you restarted."),
        (false, 1) => "1 regression was set aside when you restarted.".into(),
        (false, _) => format!("{aside} regressions were set aside when you restarted."),
    }
}

fn value(out: &mut String, report: &Report, named: bool) {
    let counts = &report.counts;
    if counts.caught == 0 {
        if report.confidence.unscoped.is_none() {
            let _ = writeln!(out, "No regressions were found {}.", when(report.scope));
        }
        return;
    }
    let caught = match named {
        true => format!("klin caught {} {}.", counts.caught, when(report.scope)),
        false => format!(
            "klin caught {} {}.",
            counted(counts.caught),
            when(report.scope)
        ),
    };
    let said: Vec<String> = [
        Some(caught),
        mended(counts.fixed, counts.caught, report.confidence.whole()),
        reconfigured(counts.config_changed),
    ]
    .into_iter()
    .flatten()
    .collect();
    let _ = writeln!(out, "\n{}", said.join(" "));
}

/// A regression that went from a measurement taken under another configuration. The journal
/// proves it is gone and not that anyone edited the code, so the sentence says only that.
fn reconfigured(count: usize) -> Option<String> {
    match count {
        0 => None,
        _ => Some(format!("{count} resolved after the config changed.")),
    }
}

/// What the journal proves about the regressions that went: each was flagged and later absent
/// from a measurement. It does not prove who edited the code, so no sentence here names an
/// author. Where klin did not measure the window whole, the claim is about what the report knows.
fn mended(fixed: usize, caught: usize, whole: bool) -> Option<String> {
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

/// The open sites, at most three of them, each on one scannable line, and the pointer at the
/// evidence for the rest. A person already reading `--all` has the rest below, so the pointer
/// stays out of that surface. Spec 11.5.
fn sites(out: &mut String, report: &Report, all: bool) {
    let open = report.open();
    if open.is_empty() {
        return;
    }
    let shown = open.len().min(SITES);
    let width = open[..shown]
        .iter()
        .map(|one| one.site().chars().count())
        .max()
        .unwrap_or_default();
    let _ = writeln!(out);
    for one in &open[..shown] {
        let _ = writeln!(out, "{:width$}  {}", one.site(), describe(one));
    }
    if open.len() > shown && !all {
        let _ = writeln!(out, "\nand {} more · klin stats --all", open.len() - shown);
    }
}

/// The site in the fewest words that still say what it is: the text the record carries, the
/// values it measured where it carries no text, and the catalogue's own noun where it carries
/// neither.
fn describe(one: &Regression) -> String {
    let text = clip(&one.text);
    if !text.is_empty() {
        return text;
    }
    match measures(&one.values) {
        Some(said) => said,
        None => label(&one.gate, 1).to_string(),
    }
}

fn measures(values: &Value) -> Option<String> {
    let held: Vec<String> = values
        .as_object()?
        .iter()
        .map(|(key, value)| format!("{key} {value}"))
        .collect();
    (!held.is_empty()).then(|| held.join(", "))
}

/// What a report calls one gate's findings: the catalogue's own words, and the recorded gate name
/// for a gate this binary holds no row for, which keeps a line for a gate klin no longer has
/// readable. Spec 11.5.
fn label(gate: &str, many: usize) -> &str {
    match check::labels(gate) {
        Some(labels) => labels.count(many),
        None => gate,
    }
}

/// The evidence `--all` exists for: every regression of the window, grouped into the turns they
/// were caught in and newest first, then the audit trail and what klin could not measure.
fn history(out: &mut String, start: &Path, report: &Report) {
    let offset = offset();
    let _ = writeln!(out, "\nklin, {} in {}", when(report.scope), place(start));
    for chapter in chapters(&report.regressions) {
        turn_chapter(out, report, &chapter, offset);
    }
    audit(out, report, offset);
    measurement(out, report);
}

/// The regressions of one turn, kept together by the prompt counter the stop that flagged them
/// ran under.
fn chapters(held: &[Regression]) -> Vec<Vec<&Regression>> {
    let mut out: Vec<Vec<&Regression>> = Vec::new();
    for one in held {
        match out.iter_mut().find(|group| group[0].chapter == one.chapter) {
            Some(group) => group.push(one),
            None => out.push(vec![one]),
        }
    }
    out
}

fn turn_chapter(out: &mut String, report: &Report, held: &[&Regression], offset: i64) {
    let _ = writeln!(out, "\n{}", day(held[0].first, report.now, offset));
    if let Some(prompt) = held[0].prompt.as_deref() {
        let _ = writeln!(out, "You asked: \"{prompt}\"");
    }
    let _ = writeln!(out);
    for gate in gates_of(held) {
        let sites: Vec<&&Regression> = held.iter().filter(|one| one.gate == gate).collect();
        let _ = writeln!(out, "  {} {}", sites.len(), label(&gate, sites.len()));
        for one in sites {
            let _ = writeln!(out, "    {}  {}", one.site(), describe(one));
            let _ = writeln!(out, "      {}", one.outcome.sentence(one.tries));
            if one.outcome == Outcome::Open && !one.remedy.is_empty() {
                let _ = writeln!(out, "      {}", one.remedy);
            }
        }
    }
}

fn gates_of(held: &[&Regression]) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    for one in held {
        if !named.contains(&one.gate) {
            named.push(one.gate.clone());
        }
    }
    named
}

fn audit(out: &mut String, report: &Report, offset: i64) {
    if report.audit.is_empty() {
        return;
    }
    let _ = writeln!(out, "\nAudit");
    for one in &report.audit {
        let when = day(one.time(), report.now, offset);
        let _ = writeln!(out, "  {when}  {}", one.sentence());
    }
}

fn measurement(out: &mut String, report: &Report) {
    let held = &report.confidence;
    if held.whole() {
        return;
    }
    let _ = writeln!(out, "\nMeasurement");
    named(out, "no grammar read", &held.unparsed);
    named(out, "the base measured and this tree did not", &held.lost);
    named(out, "no structural adapter reads", &held.not_measured);
    if held.unresolved > 0 {
        let _ = writeln!(
            out,
            "  {} dependency form(s) klin could not resolve.",
            held.unresolved
        );
    }
    if !held.errored.is_empty() {
        let _ = writeln!(
            out,
            "  {} gate(s) measured nothing: {}",
            held.errored.len(),
            held.errored.join(", ")
        );
    }
    if held.skipped > 0 {
        let _ = writeln!(
            out,
            "  {} journal line(s) klin does not understand.",
            held.skipped
        );
    }
}

fn named(out: &mut String, why: &str, files: &[String]) {
    if files.is_empty() {
        return;
    }
    let _ = writeln!(out, "  {} file(s) {why}: {}", files.len(), files.join(", "));
}

/// The facts, and none of the person's sentences: one entry per regression identity, the audit
/// trail, the measurement gaps and the activity the journal recorded. The human default prints
/// less than this, and nothing factual is dropped because it stopped printing. Spec 11.5.
fn json(out: &mut String, report: &Report, lines: &[Value], scope: Scope, now: u64) {
    let window = match scope {
        Scope::Turn => serde_json::json!({ "scope": "turn" }),
        Scope::Session => serde_json::json!({ "scope": "session" }),
        Scope::Since(days) => serde_json::json!({ "scope": "days", "days": days }),
    };
    let record = serde_json::json!({
        "window": window,
        "stops": report.stops,
        "skipped": report.confidence.skipped,
        "unreadable": report.confidence.unparsed.len() + report.confidence.lost.len(),
        "counts": report.counts.record(),
        "episodes": report.regressions.iter().map(episode).collect::<Vec<Value>>(),
        "audit": report.audit.iter().map(Audit::record).collect::<Vec<Value>>(),
        "confidence": confidence(&report.confidence),
        "activity": { "stops": report.stops, "klin_ms": report.ms },
        "earlier": earlier(lines, scope, now),
    });
    let _ = writeln!(out, "{record}");
}

fn episode(one: &Regression) -> Value {
    serde_json::json!({
        "gate": one.gate,
        "label": label(&one.gate, 1),
        "id": one.id,
        "key": {
            "gate": one.gate,
            "file": one.file,
            "line": one.line,
            "text": one.text,
        },
        "file": one.file,
        "line": one.line,
        "text": one.text,
        "values": one.values,
        "remedy": one.remedy,
        "time": one.first,
        "first": one.first,
        "last": one.last,
        "prompt": one.prompt,
        "outcome": one.outcome.name(),
        "tries": one.tries,
        "config_changed": one.outcome == Outcome::ConfigChanged,
    })
}

fn confidence(held: &Confidence) -> Value {
    serde_json::json!({
        "whole": held.whole(),
        "gap": held.gap(),
        "unscoped": held.unscoped,
        "unparsed": held.unparsed,
        "lost": held.lost,
        "not_measured": held.not_measured,
        "unresolved": held.unresolved,
        "errored": held.errored,
        "skipped": held.skipped,
    })
}

/// What the window before this one caught, for a harness that trends the two. Only a window of
/// days has one, and only where the journal reaches back over all of it. Spec 11.5.
fn earlier(lines: &[Value], scope: Scope, now: u64) -> Option<Value> {
    let Scope::Since(days) = scope else {
        return None;
    };
    let span = days.saturating_mul(DAY);
    let (from, until) = (
        now.saturating_sub(span.saturating_mul(2)),
        now.saturating_sub(span),
    );
    if lines.first().is_none_or(|first| at(first) > from) {
        return None;
    }
    let held: Vec<Value> = lines
        .iter()
        .filter(|line| (from..until).contains(&at(line)))
        .cloned()
        .collect();
    let counts = Counts::of(&regressions(&held));
    Some(serde_json::json!({ "caught": counts.caught, "open": counts.open }))
}

/// The journal a stop's telling needs and no more: back to the seven-day cutoff the week's
/// headline reads, or to the turn stamp where the turn reaches further back. The `klin radius`
/// run that appends a prompt line takes the stamp after it, and a journal time is a whole
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
        .all(|one| one.gate == check::PUBLIC_API);
    match (counts.open, counts.caught) {
        (0, 0) => None,
        (0, caught) => Some(caught_this("this turn", caught, counts.fixed)),
        (_, _) if public_api => Some(
            "Public API compatibility breaks still need your attention. `klin stats --turn` shows them."
                .into(),
        ),
        (1, _) => {
            Some("1 regression still needs your attention. `klin stats --turn` shows it.".into())
        }
        (open, _) => Some(format!(
            "{open} regressions still need your attention. `klin stats --turn` shows them."
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
        true => "`klin stats` shows them.",
        false => "`klin stats` shows the rest.",
    };
    format!(
        "{} {rest}",
        caught_this("in the last seven days", counts.caught, counts.fixed)
    )
}

/// The counted unit in the person's words. Spec 11.5, CONTEXT.md.
fn counted(count: usize) -> String {
    match count {
        1 => "1 regression".to_string(),
        _ => format!("{count} regressions"),
    }
}

fn clip(text: &str) -> String {
    let held: String = text.chars().take(70).collect();
    held.trim().to_string()
}

/// The window in the words a sentence takes it in.
fn when(scope: Scope) -> String {
    match scope {
        Scope::Turn => "this turn".to_string(),
        Scope::Session => "this session".to_string(),
        Scope::Since(1) => "today".to_string(),
        Scope::Since(2..=7) => "this week".to_string(),
        Scope::Since(8..=31) => "this month".to_string(),
        Scope::Since(days) => format!("in the last {days} days"),
    }
}

/// "this repository" where the repository has one worktree, and "this worktree" where a person
/// keeps more than one, because then the numbers are this tree's alone.
fn place(start: &Path) -> &'static str {
    let listed = Repo::at(start).worktrees().unwrap_or_default();
    match listed
        .lines()
        .filter(|line| line.starts_with("worktree "))
        .count()
    {
        0 | 1 => "this repository",
        _ => "this worktree",
    }
}

/// The local offset, read from the system once per report, with UTC as the fallback, because
/// `std` has no time zone and klin takes no date crate.
fn offset() -> i64 {
    let Ok(done) = Command::new("date").arg("+%z").output() else {
        return 0;
    };
    let said = String::from_utf8_lossy(&done.stdout);
    let text = said.trim();
    let (sign, digits) = match text.split_at_checked(1) {
        Some(("+", digits)) => (1, digits),
        Some(("-", digits)) => (-1, digits),
        _ => return 0,
    };
    let Some((hours, minutes)) = digits.split_at_checked(2) else {
        return 0;
    };
    match (hours.parse::<i64>(), minutes.parse::<i64>()) {
        (Ok(hours), Ok(minutes)) => sign * (hours * 3600 + minutes * 60),
        _ => 0,
    }
}

/// The day a line happened on, in the person's words: Today, Yesterday, the weekday name inside
/// seven days, and the date beyond that.
fn day(time: u64, now: u64, offset: i64) -> String {
    let days = |seconds: u64| (seconds as i64 + offset).div_euclid(DAY as i64);
    let then = days(time);
    match days(now) - then {
        i64::MIN..=0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        2..=6 => weekday(then).to_string(),
        _ => date(then),
    }
}

fn weekday(days: i64) -> &'static str {
    const NAMES: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    NAMES[((days + 3).rem_euclid(7)) as usize]
}

/// The civil date of a day count since the epoch, by Howard Hinnant's algorithm, which needs no
/// table and no crate.
fn date(days: i64) -> String {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let day_of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let slot = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * slot + 2) / 5 + 1;
    let month = match slot < 10 {
        true => slot + 3,
        false => slot - 9,
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
