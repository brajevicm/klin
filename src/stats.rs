use std::cmp::{Ordering, Reverse};
use std::fmt::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::{journal, turn};

/// What klin caught, in the person's words. The report reads the journal of 11.4 and nothing
/// else, turns its lines into episodes with no clock of its own, and prints them. No word of
/// the agent's glossary appears in the text, and nothing here names a check: a gate klin does
/// not have is read and printed like any other. Spec 11.5.
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
    /// Print every item in each group, not the first five
    #[arg(long)]
    all: bool,
    /// Print the episodes as one JSON object
    #[arg(long)]
    json: bool,
}

/// Which lines one report reads. Only a window of days has a window before it to compare with.
#[derive(Clone, Copy)]
enum Scope {
    Turn,
    Session,
    Since(u64),
}

/// How the episode ended, read from the lines that follow the one that blocked. The writer
/// never stores it, so a later reader can change the rule without rewriting history.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    FixedNext,
    FixedLater,
    Reset,
    Open,
    AskedOnce,
}

impl Outcome {
    fn name(self) -> &'static str {
        match self {
            Outcome::FixedNext => "fixed-next",
            Outcome::FixedLater => "fixed-later",
            Outcome::Reset => "reset",
            Outcome::Open => "open",
            Outcome::AskedOnce => "asked-once",
        }
    }

    /// The group an episode prints under. A reset's episodes print under the reset itself.
    fn heading(self) -> &'static str {
        match self {
            Outcome::Open => "Still there",
            Outcome::FixedNext | Outcome::FixedLater => "Fixed after klin asked",
            Outcome::AskedOnce | Outcome::Reset => "You were asked",
        }
    }
}

/// One gate failure on one stop that spent the prompt's gate block, and how it ended.
pub struct Episode {
    pub gate: String,
    pub file: String,
    pub line: Option<u64>,
    pub text: String,
    pub remedy: String,
    pub time: u64,
    /// How many more findings that gate left on that stop that ended the same way as this one.
    pub more: usize,
    pub outcome: Outcome,
    /// Where, in the lines the episode was read from, the line that ended it sits.
    pub ended: Option<usize>,
    /// The excerpt of the prompt the stop ran under, when the journal recorded one.
    pub prompt: Option<String>,
}

/// One thing the person was asked about or did, newest first in its group. Spec 11.5.
enum Asked<'a> {
    Guard {
        time: u64,
        decision: &'a str,
        reason: &'a str,
    },
    /// A reset, and the episodes it set aside.
    Reset {
        time: u64,
        set_aside: Vec<&'a Episode>,
    },
    Deleted(&'a Episode),
}

impl Asked<'_> {
    fn time(&self) -> u64 {
        match self {
            Asked::Guard { time, .. } | Asked::Reset { time, .. } => *time,
            Asked::Deleted(episode) => episode.time,
        }
    }
}

/// The report's groups, in the order they print. Spec 11.5.
const GROUPS: [Outcome; 3] = [Outcome::Open, Outcome::FixedNext, Outcome::AskedOnce];
const CAP: usize = 5;
const DAY: u64 = 86_400;

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let scope = scope(args)?;
    let (lines, skipped) = journal::read(start);
    let now = clock();
    let held = scoped(&lines, scope, start, now);
    let episodes = episodes(&held);
    let asked = asked(&held, &episodes);
    let earlier = earlier(&lines, scope, now);
    let read = Reading {
        lines: &held,
        episodes: &episodes,
        asked: &asked,
        earlier: earlier.as_deref(),
        skipped,
        now,
        scope,
        empty: lines.is_empty() && skipped == 0,
    };
    match args.json {
        true => json(out, &read),
        false => text(out, args, start, &read),
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
/// outside any session and which still end the episodes before them.
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

/// The episodes of the window before this one, when the journal reaches back over all of it.
/// Only a window of days has one. Spec 11.5.
fn earlier(lines: &[Value], scope: Scope, now: u64) -> Option<Vec<Episode>> {
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
    Some(episodes(&held))
}

/// The episodes the lines hold, with no I/O, no clock and no formatting. One episode per gate
/// that failed on a stop that blocked, which is the intervention of ADR 0004 and 0022.
pub fn episodes(lines: &[Value]) -> Vec<Episode> {
    let stops: Vec<usize> = (0..lines.len())
        .filter(|index| kind(&lines[*index]) == "stop")
        .collect();
    let mut out = Vec::new();
    for (turn, index) in stops.iter().enumerate() {
        let line = &lines[*index];
        if !blocked(line) {
            continue;
        }
        let prompt = excerpt(lines, *index);
        for gate in gates_that_failed(line) {
            let sites: Vec<&Value> = failures(line, &gate).collect();
            for (outcome, ended, held) in resolve(lines, &stops, turn, &gate, sites) {
                let more = held.len() - 1;
                out.push(episode(
                    held[0],
                    &gate,
                    line,
                    more,
                    (outcome, ended),
                    &prompt,
                ));
            }
        }
    }
    out.sort_by_key(|one| Reverse(one.time));
    out
}

fn episode(
    site: &Value,
    gate: &str,
    stop: &Value,
    more: usize,
    (outcome, ended): (Outcome, Option<usize>),
    prompt: &Option<String>,
) -> Episode {
    Episode {
        gate: gate.to_string(),
        file: word(site, "file").to_string(),
        line: site.get("line").and_then(Value::as_u64),
        text: word(site, "text").to_string(),
        remedy: word(site, "fix_advice").to_string(),
        time: at(stop),
        more,
        outcome,
        ended,
        prompt: prompt.clone(),
    }
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

/// The failing findings of one gate on one stop. A record that names no file is a run that
/// could not measure, and it is no intervention.
fn failures<'a>(line: &'a Value, gate: &'a str) -> impl Iterator<Item = &'a Value> {
    list(line, "findings")
        .iter()
        .filter(move |site| word(site, "gate") == gate && !word(site, "file").is_empty())
}

fn gates_that_failed(line: &Value) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    for site in list(line, "findings") {
        let gate = word(site, "gate");
        if !gate.is_empty()
            && !word(site, "file").is_empty()
            && !named.iter().any(|held| held == gate)
        {
            named.push(gate.to_string());
        }
    }
    named
}

/// How the episode ended, and where the line that ended it sits, read from the lines after the
/// stop that blocked. A site klin let through after it asked is an ask, and never a fix: the code
/// is as the agent left it. The sites it let through end apart from the ones beside them.
fn resolve<'a>(
    lines: &[Value],
    stops: &[usize],
    turn: usize,
    gate: &str,
    mut left: Vec<&'a Value>,
) -> Vec<(Outcome, Option<usize>, Vec<&'a Value>)> {
    let from = stops.get(turn).map_or(lines.len(), |index| index + 1);
    let mut ended = Vec::new();
    let mut seen = 0;
    for (index, line) in lines.iter().enumerate().skip(from) {
        match kind(line) {
            "reset" => {
                ended.push((Outcome::Reset, Some(index), left));
                return ended;
            }
            "stop" if measured(line, gate) => {}
            _ => continue,
        }
        seen += 1;
        let (through, rest) = left
            .into_iter()
            .partition::<Vec<_>, _>(|site| let_through(line, gate, site));
        if !through.is_empty() {
            ended.push((Outcome::AskedOnce, Some(index), through));
        }
        left = rest;
        if left.is_empty() || clear(line, gate) {
            return cleared(ended, seen, index, left);
        }
    }
    ended.push((Outcome::Open, None, left));
    ended
}

fn cleared<'a>(
    mut ended: Vec<(Outcome, Option<usize>, Vec<&'a Value>)>,
    seen: usize,
    index: usize,
    left: Vec<&'a Value>,
) -> Vec<(Outcome, Option<usize>, Vec<&'a Value>)> {
    if left.is_empty() {
        return ended;
    }
    let outcome = match seen {
        1 => Outcome::FixedNext,
        _ => Outcome::FixedLater,
    };
    ended.push((outcome, Some(index), left));
    ended
}

/// Whether this stop ran the gate and the gate passed. A gate the stop carries no row for did
/// not run, because a tree that does not build runs none, and a gate that errored measured
/// nothing. Neither says the site went, so neither ends the episode. Spec 11.2.
fn clear(line: &Value, gate: &str) -> bool {
    list(line, "gates")
        .iter()
        .any(|row| word(row, "name") == gate && word(row, "status") == "ok")
}

fn measured(line: &Value, gate: &str) -> bool {
    list(line, "gates")
        .iter()
        .any(|row| word(row, "name") == gate && word(row, "status") != "ERR")
}

/// Whether this stop recorded the site as one it let through after an earlier stop asked about
/// it, which is a note and not a finding. Spec 8.2.
fn let_through(line: &Value, gate: &str, site: &Value) -> bool {
    list(line, "notes").iter().any(|note| {
        word(note, "gate") == gate
            && word(note, "outcome") == "deleted"
            && word(note, "file") == word(site, "file")
            && note.get("line") == site.get("line")
    })
}

/// What a guard line's reason names, in the person's words. A reason this binary does not know
/// reads as a tool call. Spec 11.4.
const GUARDED: [(&str, &str); 6] = [
    ("config-write", "an edit to klin.json"),
    ("state-write", "an edit to klin's own state"),
    ("config-mention", "a command that named klin.json"),
    ("state-mention", "a command that named klin's own state"),
    ("init", "klin init, which only you run"),
    ("turn-reset", "klin turn reset, which only you run"),
];

/// Everything the person was asked about or did in the window, newest first: every guard
/// answer, every reset with the episodes it set aside, and every deleted test klin let through
/// once the agent said why. Spec 11.5.
fn asked<'a>(lines: &'a [Value], episodes: &'a [Episode]) -> Vec<Asked<'a>> {
    let mut out: Vec<Asked> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| match kind(line) {
            "guard" => Some(Asked::Guard {
                time: at(line),
                decision: word(line, "decision"),
                reason: word(line, "reason"),
            }),
            "reset" => Some(Asked::Reset {
                time: at(line),
                set_aside: episodes
                    .iter()
                    .filter(|episode| episode.ended == Some(index))
                    .collect(),
            }),
            _ => None,
        })
        .collect();
    out.extend(
        episodes
            .iter()
            .filter(|episode| episode.outcome == Outcome::AskedOnce)
            .map(Asked::Deleted),
    );
    out.sort_by_key(|asked| Reverse(asked.time()));
    out
}

/// The item's words: one sentence, or a sentence with the shortcuts under it.
fn sentence(asked: &Asked) -> String {
    match asked {
        Asked::Guard {
            decision, reason, ..
        } => guarded(decision, reason),
        Asked::Reset { set_aside, .. } => set_aside_by(set_aside),
        Asked::Deleted(episode) => deleted(episode),
    }
}

fn guarded(decision: &str, reason: &str) -> String {
    let what = GUARDED
        .iter()
        .find(|(tag, _)| *tag == reason)
        .map_or("a tool call", |(_, what)| what);
    match decision {
        "ask" => format!("klin asked before {what}"),
        _ => format!("klin refused {what}"),
    }
}

/// A reset in the person's words. The report cannot see whether the code is still in the tree,
/// so the action says "if", and CI still judges the branch.
fn set_aside_by(episodes: &[&Episode]) -> String {
    let (it, still) = match episodes.len() {
        0 => return "You told klin to start over.".to_string(),
        1 => ("it", "it's"),
        _ => ("them", "they're"),
    };
    let mut said = format!("You set aside {}.", shortcuts(episodes.len()));
    for episode in episodes {
        let _ = write!(said, "\n  {}{}", item(episode), asking(episode));
    }
    let _ = write!(
        said,
        "\nIf {still} still there, fix {it}, or accept {it} in `klin.json`, before you push."
    );
    said
}

/// A deleted test klin let through, named by its declaration line, or by its file where the
/// whole file went.
fn deleted(episode: &Episode) -> String {
    let clipped = clip(&episode.text);
    let site = clipped.trim_end_matches(['{', ':', ' ']);
    match (site.is_empty(), episode.line) {
        (true, _) => format!("{} deleted. The agent said why.", episode.file),
        (false, Some(line)) => format!(
            "a test deleted from {}:{line}, {site}. The agent said why.",
            episode.file
        ),
        (false, None) => format!(
            "a test deleted from {}, {site}. The agent said why.",
            episode.file
        ),
    }
}

/// What the turn end tells the person on a stop nothing blocks: the turn line, and beside it at
/// most once every seven days the week's headline, each with the name the journal records it
/// under. `this` is the stop's own line, which the journal does not hold yet. Spec 9.5.
///
/// ponytail: reads the whole journal on each stop in a turn with an intervention; read only its
/// tail if a long-lived journal pushes the hook past the budget of spec 13.
pub fn turn_end(root: &Path, this: Value) -> Vec<(&'static str, String)> {
    let (mut lines, _) = journal::read(root);
    lines.push(this);
    let now = clock();
    let turn = episodes(&scoped(&lines, Scope::Turn, root, now));
    let Some(said) = turn_line(&turn) else {
        return Vec::new();
    };
    let mut parts = vec![("turn", said)];
    if weekly(&lines, now) {
        parts.push((
            "weekly",
            weekly_line(&scoped(&lines, Scope::Since(7), root, now)),
        ));
    }
    parts
}

/// The count fixed on a turn that ends with nothing left, and the count still there on one that
/// ends red. Nothing for a turn with no intervention, or with only questions the note names.
fn turn_line(episodes: &[Episode]) -> Option<String> {
    let caught = episodes.len();
    match (many(episodes, Outcome::Open), fixed(episodes)) {
        (0, 0) => None,
        (0, fixed) => Some(format!(
            "klin: the agent took {} this turn and {} after klin asked.",
            shortcuts(caught),
            mended(fixed, caught)
        )),
        (1, _) => {
            Some("klin: one shortcut is still there. `klin stats --turn` names it.".to_string())
        }
        (open, _) => Some(format!(
            "klin: {open} shortcuts are still there. `klin stats --turn` names them."
        )),
    }
}

/// Whether the week's headline is due: no line of the last seven days carried it, and the
/// journal reaches back a week, so there is a week to tell.
fn weekly(lines: &[Value], now: u64) -> bool {
    let week = now.saturating_sub(7 * DAY);
    let told = |line: &Value| list(line, "told").iter().any(|part| part == "weekly");
    lines.first().is_some_and(|first| at(first) <= week)
        && !lines.iter().any(|line| at(line) > week && told(line))
}

/// The week in one sentence, good news first, and the command that lists it, so a person who
/// never types the command learns it exists. Spec 9.5.
fn weekly_line(week: &[Value]) -> String {
    let episodes = episodes(week);
    let caught = episodes.len();
    let agent = match fixed(&episodes) {
        0 => String::new(),
        fixed => format!(" and the agent {} on its own", mended(fixed, caught)),
    };
    format!(
        "In the last seven days, klin caught {}{agent}. `klin stats` lists them.",
        shortcuts(caught)
    )
}

/// The report's headline: klin is the subject of the first sentence and the agent of the
/// second, and what is still there is its own sentence. Spec 11.5.
fn headline(episodes: &[Episode], asked: usize) -> (String, Option<String>) {
    let caught = episodes.len();
    let agent = match (fixed(episodes), asked) {
        (0, 0) => String::new(),
        (0, asked) => format!(" The agent asked you {}.", times(asked)),
        (fixed, 0) => format!(" The agent {} on its own.", mended(fixed, caught)),
        (fixed, asked) => format!(
            " The agent {} on its own and asked you {}.",
            mended(fixed, caught),
            times(asked)
        ),
    };
    let head = format!("klin caught {}.{agent}", shortcuts(caught));
    (head, still_there(many(episodes, Outcome::Open)))
}

fn still_there(open: usize) -> Option<String> {
    match open {
        0 => None,
        1 => Some("One is still there.".to_string()),
        _ => Some(format!("{open} are still there.")),
    }
}

fn fixed(episodes: &[Episode]) -> usize {
    many(episodes, Outcome::FixedNext) + many(episodes, Outcome::FixedLater)
}

/// What the agent fixed, in the words the headline, the turn line and the weekly line share.
fn mended(fixed: usize, caught: usize) -> String {
    match (fixed, caught) {
        (1, 1) => "fixed it".to_string(),
        (2, 2) => "fixed both".to_string(),
        _ if fixed == caught => format!("fixed all {fixed}"),
        _ => format!("fixed {fixed} of them"),
    }
}

/// One report's inputs: the scope's lines, the episodes and questions read from them, the
/// episodes of the window before, and what the reader could not read. Nothing here is measured
/// a second time.
struct Reading<'a> {
    lines: &'a [Value],
    episodes: &'a [Episode],
    asked: &'a [Asked<'a>],
    earlier: Option<&'a [Episode]>,
    skipped: u64,
    now: u64,
    scope: Scope,
    /// Whether the journal held no line at all, which is a first run and not a quiet window.
    empty: bool,
}

fn json(out: &mut String, read: &Reading) {
    let episodes: Vec<Value> = read
        .episodes
        .iter()
        .map(|episode| {
            serde_json::json!({
                "gate": episode.gate,
                "file": episode.file,
                "line": episode.line,
                "text": episode.text,
                "remedy": episode.remedy,
                "time": episode.time,
                "more": episode.more,
                "outcome": episode.outcome.name(),
                "prompt": episode.prompt,
            })
        })
        .collect();
    let window = match read.scope {
        Scope::Turn => serde_json::json!({ "scope": "turn" }),
        Scope::Session => serde_json::json!({ "scope": "session" }),
        Scope::Since(days) => serde_json::json!({ "scope": "days", "days": days }),
    };
    let earlier = read.earlier.map(|earlier| {
        serde_json::json!({ "caught": earlier.len(), "open": many(earlier, Outcome::Open) })
    });
    let report = serde_json::json!({
        "window": window,
        "stops": stops(read.lines),
        "skipped": read.skipped,
        "unreadable": unreadable(read.lines),
        "counts": counts(read.episodes),
        "episodes": episodes,
        "asked": read.asked.iter().map(record).collect::<Vec<Value>>(),
        "earlier": earlier,
    });
    let _ = writeln!(out, "{report}");
}

/// One thing the person was asked about or did, as facts and not as the person's words.
fn record(asked: &Asked) -> Value {
    let (kind, decision, reason, file, line) = match asked {
        Asked::Guard {
            decision, reason, ..
        } => ("guard", Some(*decision), Some(*reason), None, None),
        Asked::Reset { .. } => ("reset", None, None, None, None),
        Asked::Deleted(episode) => (
            "asked-once",
            None,
            None,
            Some(episode.file.as_str()),
            episode.line,
        ),
    };
    serde_json::json!({
        "time": asked.time(),
        "kind": kind,
        "decision": decision,
        "reason": reason,
        "file": file,
        "line": line,
    })
}

fn counts(episodes: &[Episode]) -> Map<String, Value> {
    let mut out = Map::new();
    out.insert("caught".into(), episodes.len().into());
    for outcome in [
        Outcome::FixedNext,
        Outcome::FixedLater,
        Outcome::Reset,
        Outcome::Open,
        Outcome::AskedOnce,
    ] {
        out.insert(outcome.name().into(), many(episodes, outcome).into());
    }
    out
}

fn many(episodes: &[Episode], outcome: Outcome) -> usize {
    episodes
        .iter()
        .filter(|episode| episode.outcome == outcome)
        .count()
}

fn stops(lines: &[Value]) -> usize {
    lines.iter().filter(|line| kind(line) == "stop").count()
}

/// How many files the stops in this window reached and could not read or measure. Green with
/// half the tree unparsed is the one lie the report must not tell.
fn unreadable(lines: &[Value]) -> usize {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for note in lines.iter().flat_map(|line| list(line, "notes")) {
        let at = (word(note, "gate"), word(note, "file"));
        if matches!(word(note, "outcome"), "unparsed" | "lost") && !seen.contains(&at) {
            seen.push(at);
        }
    }
    seen.len()
}

fn text(out: &mut String, args: &Args, start: &Path, read: &Reading) {
    let _ = writeln!(out, "klin, {} in {}\n", title(read.scope), place(start));
    if read.empty {
        let _ = writeln!(
            out,
            "klin started watching today. Come back after a few turns."
        );
        return;
    }
    lead(out, read);
    let offset = offset();
    for outcome in GROUPS {
        let heading = outcome.heading();
        match outcome {
            Outcome::Open => open(out, args, read, offset),
            Outcome::FixedNext | Outcome::FixedLater => {
                let fixed: Vec<(u64, String)> = read
                    .episodes
                    .iter()
                    .filter(|episode| episode.outcome.heading() == heading)
                    .map(|episode| (episode.time, item(episode) + &asking(episode)))
                    .collect();
                group(out, args, heading, &fixed, read, offset);
            }
            Outcome::AskedOnce | Outcome::Reset => {
                let asked: Vec<(u64, String)> = read
                    .asked
                    .iter()
                    .map(|asked| (asked.time(), sentence(asked)))
                    .collect();
                group(out, args, heading, &asked, read, offset);
            }
        }
    }
    let footer: Vec<String> = compared(read)
        .into_iter()
        .chain((!read.episodes.is_empty()).then(|| ran(read.lines)))
        .collect();
    if !footer.is_empty() {
        let _ = writeln!(out, "\n{}", footer.join("\n"));
    }
    measurement(out, read.lines, read.skipped);
}

fn questions(asked: &[Asked]) -> usize {
    asked
        .iter()
        .filter(|asked| match asked {
            Asked::Guard { decision, .. } => *decision == "ask",
            Asked::Reset { .. } => false,
            Asked::Deleted(_) => true,
        })
        .count()
}

fn lead(out: &mut String, read: &Reading) {
    if read.episodes.is_empty() {
        let asked = match questions(read.asked) {
            0 => "asked nothing".to_string(),
            count => format!("asked you {}", times(count)),
        };
        let _ = writeln!(
            out,
            "klin caught no shortcuts. klin ran {} and {asked}.",
            times(stops(read.lines))
        );
        return;
    }
    let (head, still) = headline(read.episodes, questions(read.asked));
    let _ = writeln!(out, "{head}");
    if let Some(still) = still {
        let _ = writeln!(out, "{still}");
    }
}

fn shortcuts(count: usize) -> String {
    match count {
        1 => "1 shortcut".to_string(),
        _ => format!("{count} shortcuts"),
    }
}

fn times(count: usize) -> String {
    match count {
        1 => "once".to_string(),
        _ => format!("{count} times"),
    }
}

/// The line that compares this window with the one before it, judged on what is still there
/// first and on the count after. It reads this worktree's own journal and nothing else.
fn compared(read: &Reading) -> Option<String> {
    let (Scope::Since(days), Some(earlier)) = (read.scope, read.earlier) else {
        return None;
    };
    let open = |episodes: &[Episode]| many(episodes, Outcome::Open);
    let judged = open(read.episodes)
        .cmp(&open(earlier))
        .then(read.episodes.len().cmp(&earlier.len()));
    let word = match judged {
        Ordering::Less => "better",
        Ordering::Greater => "worse",
        Ordering::Equal => "the same",
    };
    let (then, this) = periods(days);
    Some(format!(
        "{then}: {}, {} left open. {this} {word}.",
        shortcuts(earlier.len()),
        open(earlier)
    ))
}

/// The window before and this one, in the words of the title.
fn periods(days: u64) -> (String, String) {
    let named = |then: &str, this: &str| (then.to_string(), this.to_string());
    match days {
        1 => named("Yesterday", "Today is"),
        2..=7 => named("Last week", "This week is"),
        8..=31 => named("Last month", "This month is"),
        _ => (
            format!("The {days} days before"),
            format!("These {days} days are"),
        ),
    }
}

/// The footer: what klin cost the person, in their units. A run under a second says so rather
/// than round to zero.
fn ran(lines: &[Value]) -> String {
    let ms: u64 = lines
        .iter()
        .filter_map(|line| line.get("timing").and_then(|timing| timing.get("klin_ms")))
        .filter_map(Value::as_u64)
        .sum();
    let spent = match ms {
        0..=999 => "less than a second".to_string(),
        _ => format!("{} seconds", ms / 1000),
    };
    format!(
        "klin ran {} and took {spent} in total.",
        times(stops(lines))
    )
}

fn measurement(out: &mut String, lines: &[Value], skipped: u64) {
    let lost = unreadable(lines);
    if skipped == 0 && lost == 0 {
        return;
    }
    let _ = writeln!(out, "\nMeasurement");
    if skipped > 0 {
        let _ = writeln!(
            out,
            "  klin skipped {skipped} journal line(s) it does not understand."
        );
    }
    if lost > 0 {
        let _ = writeln!(out, "  klin could not read {lost} file(s) in this window.");
    }
}

/// The open group, the one group with an action: each item carries its remedy.
fn open(out: &mut String, args: &Args, read: &Reading, offset: i64) {
    let held: Vec<&Episode> = read
        .episodes
        .iter()
        .filter(|episode| episode.outcome == Outcome::Open)
        .collect();
    if held.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n{}", Outcome::Open.heading());
    let shown = shown(args, held.len());
    for episode in &held[..shown] {
        let _ = writeln!(
            out,
            "  {}, left on {}{}",
            item(episode),
            day(episode.time, read.now, offset),
            asking(episode)
        );
        if !episode.remedy.is_empty() {
            let _ = writeln!(out, "    {}", episode.remedy);
        }
    }
    more(out, held.len(), shown);
}

/// Every other group, headed by the day each item happened on. An item of more than one line
/// keeps its own indentation under the first.
fn group(
    out: &mut String,
    args: &Args,
    heading: &str,
    items: &[(u64, String)],
    read: &Reading,
    offset: i64,
) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n{heading}");
    let shown = shown(args, items.len());
    let mut said = String::new();
    for (time, item) in &items[..shown] {
        let when = day(*time, read.now, offset);
        if when != said {
            let _ = writeln!(out, "  {when}");
            said = when;
        }
        for line in item.lines() {
            let _ = writeln!(out, "    {line}");
        }
    }
    more(out, items.len(), shown);
}

fn shown(args: &Args, count: usize) -> usize {
    match args.all {
        true => count,
        false => count.min(CAP),
    }
}

fn more(out: &mut String, count: usize, shown: usize) {
    if count > shown {
        let _ = writeln!(out, "  and {} more. klin stats --all", count - shown);
    }
}

fn item(episode: &Episode) -> String {
    let at = match episode.line {
        Some(line) => format!("{}:{line}", episode.file),
        None => episode.file.clone(),
    };
    let more = match episode.more {
        0 => String::new(),
        count => format!(", and {count} more in the same check"),
    };
    format!("{} in {at}{more}", clip(&episode.text))
}

/// The request the shortcut came in, quoted as the person wrote it, which turns a list of sites
/// into the person's own story. Empty where the journal recorded no excerpt.
fn asking(episode: &Episode) -> String {
    match &episode.prompt {
        Some(prompt) => format!(", while you asked for \"{prompt}\""),
        None => String::new(),
    }
}

fn clip(text: &str) -> String {
    let held: String = text.chars().take(70).collect();
    held.trim().to_string()
}

fn title(scope: Scope) -> String {
    match scope {
        Scope::Turn => "this turn".to_string(),
        Scope::Session => "this session".to_string(),
        Scope::Since(1) => "today".to_string(),
        Scope::Since(2..=7) => "this week".to_string(),
        Scope::Since(8..=31) => "this month".to_string(),
        Scope::Since(days) => format!("the last {days} days"),
    }
}

/// "this repository" where the repository has one worktree, and "this worktree" where a person
/// keeps more than one, because then the numbers are this tree's alone.
fn place(start: &Path) -> &'static str {
    let done = Command::new("git")
        .arg("-C")
        .arg(start)
        .args(["worktree", "list", "--porcelain"])
        .output();
    let listed = match &done {
        Ok(done) => String::from_utf8_lossy(&done.stdout),
        Err(_) => return "this repository",
    };
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
