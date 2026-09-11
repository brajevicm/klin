use std::fmt::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::journal;

/// What klin caught, in the person's words. The report reads the journal of 11.4 and nothing
/// else, turns its lines into episodes with no clock of its own, and prints them. No word of
/// the agent's glossary appears in the text, and nothing here names a check: a gate klin does
/// not have is read and printed like any other. Spec 11.5.
#[derive(clap::Args)]
pub struct Args {
    /// The window to report, as a number of days, such as 30d. Seven days by default
    #[arg(long, value_name = "Nd")]
    since: Option<String>,
    /// Print every item in each group, not the first five
    #[arg(long)]
    all: bool,
    /// Print the episodes as one JSON object
    #[arg(long)]
    json: bool,
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

    fn heading(self) -> &'static str {
        match self {
            Outcome::Open => "Still there",
            Outcome::FixedNext | Outcome::FixedLater => "Fixed after klin asked",
            Outcome::AskedOnce => "You were asked",
            Outcome::Reset => "You started the judgment over",
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
    /// How many more findings that gate left on that stop beside the one named here.
    pub more: usize,
    pub outcome: Outcome,
}

const GROUPS: [Outcome; 4] = [
    Outcome::Open,
    Outcome::FixedNext,
    Outcome::AskedOnce,
    Outcome::Reset,
];
const CAP: usize = 5;
const DAY: u64 = 86_400;

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let days = window(args.since.as_deref())?;
    let (lines, skipped) = journal::read(start);
    let empty = lines.is_empty() && skipped == 0;
    let now = clock();
    let since = now.saturating_sub(days.saturating_mul(DAY));
    let held: Vec<Value> = lines.into_iter().filter(|line| at(line) >= since).collect();
    let episodes = episodes(&held);
    let window = Reading {
        lines: &held,
        episodes: &episodes,
        skipped,
        now,
        days,
        empty,
    };
    match args.json {
        true => json(out, &window),
        false => text(out, args, start, &window),
    }
    Ok(0)
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
        for gate in gates_that_failed(line) {
            let sites: Vec<&Value> = failures(line, &gate).collect();
            let Some(first) = sites.first() else { continue };
            out.push(episode(first, &gate, at(line), sites.len() - 1, {
                resolve(lines, &stops, turn, &gate, word(first, "file"))
            }));
        }
    }
    out.sort_by_key(|one| std::cmp::Reverse(one.time));
    out
}

fn episode(site: &Value, gate: &str, time: u64, more: usize, outcome: Outcome) -> Episode {
    Episode {
        gate: gate.to_string(),
        file: word(site, "file").to_string(),
        line: site.get("line").and_then(Value::as_u64),
        text: word(site, "text").to_string(),
        remedy: word(site, "fix_advice").to_string(),
        time,
        more,
        outcome,
    }
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

/// How the episode ended, read from the lines after the stop that blocked. A site klin let
/// through after it asked is an ask, and never a fix: the code is as the agent left it.
fn resolve(lines: &[Value], stops: &[usize], turn: usize, gate: &str, file: &str) -> Outcome {
    let from = stops.get(turn).map_or(lines.len(), |index| index + 1);
    let mut seen = 0;
    for line in lines.iter().skip(from) {
        match kind(line) {
            "reset" => return Outcome::Reset,
            "stop" => {}
            _ => continue,
        }
        seen += 1;
        if let_through(line, gate, file) {
            return Outcome::AskedOnce;
        }
        if !clear(line, gate) {
            continue;
        }
        return match seen {
            1 => Outcome::FixedNext,
            _ => Outcome::FixedLater,
        };
    }
    Outcome::Open
}

/// Whether this stop ran the gate and the gate passed. A gate the stop carries no row for did
/// not run, because a tree that does not build runs none, and a gate that errored measured
/// nothing. Neither says the site went, so neither ends the episode. Spec 11.2.
fn clear(line: &Value, gate: &str) -> bool {
    list(line, "gates")
        .iter()
        .any(|row| word(row, "name") == gate && word(row, "status") == "ok")
}

/// Whether this stop recorded the site as one it let through after an earlier stop asked about
/// it, which is a note and not a finding. Spec 8.2.
fn let_through(line: &Value, gate: &str, file: &str) -> bool {
    list(line, "notes").iter().any(|note| {
        word(note, "gate") == gate
            && word(note, "file") == file
            && word(note, "outcome") == "deleted"
    })
}

/// One report's inputs: the window's lines, the episodes read from them, and what the reader
/// could not read. Nothing here is measured a second time.
struct Reading<'a> {
    lines: &'a [Value],
    episodes: &'a [Episode],
    skipped: u64,
    now: u64,
    days: u64,
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
            })
        })
        .collect();
    let report = serde_json::json!({
        "window": { "days": read.days },
        "stops": stops(read.lines),
        "skipped": read.skipped,
        "unreadable": unreadable(read.lines),
        "counts": counts(read.episodes),
        "episodes": episodes,
    });
    let _ = writeln!(out, "{report}");
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
    let _ = writeln!(out, "klin, {} in {}\n", scope(read.days), place(start));
    if read.empty {
        let _ = writeln!(
            out,
            "klin started watching today. Come back after a few turns."
        );
        return;
    }
    headline(out, read.lines, read.episodes);
    let offset = offset();
    for group in GROUPS {
        printed(out, args, read.episodes, group, read.now, offset);
    }
    if !read.episodes.is_empty() {
        let _ = writeln!(out, "\n{}", ran(read.lines));
    }
    measurement(out, read.lines, read.skipped);
}

fn headline(out: &mut String, lines: &[Value], episodes: &[Episode]) {
    if episodes.is_empty() {
        let _ = writeln!(
            out,
            "klin caught no shortcuts. klin ran {} and asked nothing.",
            times(stops(lines))
        );
        return;
    }
    let _ = write!(out, "klin caught {}.", shortcuts(episodes.len()));
    let fixed = many(episodes, Outcome::FixedNext) + many(episodes, Outcome::FixedLater);
    if fixed > 0 {
        let _ = write!(out, " The agent fixed {fixed} of them before you saw them.");
    }
    let _ = writeln!(out);
    let asked = many(episodes, Outcome::AskedOnce);
    if asked > 0 {
        let _ = writeln!(out, "klin asked you about {}.", shortcuts(asked));
    }
    let open = many(episodes, Outcome::Open);
    match open {
        0 => {}
        1 => {
            let _ = writeln!(out, "One is still there.");
        }
        _ => {
            let _ = writeln!(out, "{open} are still there.");
        }
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

/// The footer: what klin cost the person, in their units. A run under a second says so rather
/// than round to zero.
fn ran(lines: &[Value]) -> String {
    let ms: u64 = lines
        .iter()
        .filter_map(|line| line.get("timing").and_then(|timing| timing.get("total_ms")))
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

/// One group of the report. Open items carry their remedy, and everything else is grouped by
/// the day it happened on.
fn printed(
    out: &mut String,
    args: &Args,
    episodes: &[Episode],
    group: Outcome,
    now: u64,
    offset: i64,
) {
    let held: Vec<&Episode> = episodes
        .iter()
        .filter(|episode| episode.outcome.heading() == group.heading())
        .collect();
    if held.is_empty() {
        return;
    }
    let _ = writeln!(out, "\n{}", group.heading());
    let shown = match args.all {
        true => held.len(),
        false => held.len().min(CAP),
    };
    match group {
        Outcome::Open => open(out, &held[..shown], now, offset),
        _ => by_day(out, &held[..shown], now, offset),
    }
    if held.len() > shown {
        let _ = writeln!(out, "  and {} more. klin stats --all", held.len() - shown);
    }
}

fn open(out: &mut String, held: &[&Episode], now: u64, offset: i64) {
    for episode in held {
        let _ = writeln!(
            out,
            "  {}, left on {}",
            item(episode),
            day(episode.time, now, offset)
        );
        if !episode.remedy.is_empty() {
            let _ = writeln!(out, "    {}", episode.remedy);
        }
    }
}

fn by_day(out: &mut String, held: &[&Episode], now: u64, offset: i64) {
    let mut said = String::new();
    for episode in held {
        let when = day(episode.time, now, offset);
        if when != said {
            let _ = writeln!(out, "  {when}");
            said = when;
        }
        let _ = writeln!(out, "    {}", item(episode));
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

fn clip(text: &str) -> String {
    let held: String = text.chars().take(70).collect();
    held.trim().to_string()
}

fn scope(days: u64) -> String {
    match days {
        1 => "today".to_string(),
        2..=7 => "this week".to_string(),
        8..=31 => "this month".to_string(),
        _ => format!("the last {days} days"),
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
