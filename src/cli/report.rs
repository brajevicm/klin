use std::cmp::Reverse;
use std::fmt::Write;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::config::file::MEASUREMENT_LOST;
use crate::engine::catalogue;
use crate::hook::journal::{self, Dated};
use crate::hook::stats::{
    Counts, DAY, Outcome, Regression, Scope, at, clock, counted, held, kind, list, mended,
    regressions, scoped, word,
};
use crate::sys::error::Error;
use crate::{hook::turn, sys::git::Repo};

/// What needs the person's attention, and what klin was worth. The report reads the journal of
/// 13.1 and the stamp's time, and nothing else: it re-runs no check, reads no working tree,
/// writes nothing and keeps no clock of its own. It turns the lines into one regression per
/// finding identity, decides the outcome and the measurement confidence once, and hands that to
/// one of three surfaces. The default says what matters now, `--details` carries the evidence,
/// `--json` carries the facts. The words a check is named by come from the catalogue and never
/// from a table here. Spec 13.2.
#[derive(clap::Args)]
pub struct Args {
    /// The window to report, as a number of days, such as 7d. The newest session by default
    #[arg(long, value_name = "Nd")]
    since: Option<String>,
    /// Print every regression of the window, the audit trail and the measurement evidence
    #[arg(long)]
    details: bool,
    /// Print the report as one JSON object
    #[arg(long)]
    json: bool,
}

/// How many open sites the default report names before it points at `--all`. The default answers
/// what needs attention now, and a fourth line is a list. Spec 11.5.
const SITES: usize = 3;

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let scope = scope(args)?;
    let (lines, skipped) = journal::read(start);
    let now = clock();
    let held = scoped(&lines, scope, start, now);
    let notices = journal::open_notices(&lines, turn::stamp_commit(start).as_deref());
    let empty = lines.is_empty() && skipped == 0;
    let mut report = Report::read(&held, scope, now, skipped, notices, empty);
    report.confidence.unscoped = unscoped(&lines, scope, start);
    if !args.json && report.confidence.unscoped.is_some() && matches!(scope, Scope::Session) {
        let _ = writeln!(
            out,
            "The journal holds no session yet, so there is nothing to report for one. \
             `klin report --since 7d` reads the last seven days."
        );
        return Ok(0);
    }
    match args.json {
        true => json(out, &report),
        false => text(out, args, start, &report),
    }
    Ok(0)
}

fn scope(args: &Args) -> Result<Scope, Error> {
    match &args.since {
        Some(text) => days(text).map(Scope::Since),
        None => Ok(Scope::Session),
    }
}

fn days(text: &str) -> Result<u64, Error> {
    let digits = text.strip_suffix('d').unwrap_or(text);
    match digits.parse::<u64>() {
        Ok(days) if days > 0 => Ok(days),
        _ => Err(Error(format!(
            "--since takes a number of days, such as 30d, and not {text}"
        ))),
    }
}

/// Whether klin could tell where the window the person asked for begins. A turn needs a turn
/// stamp it can read, and a session needs a session id somewhere in the journal. Without one
/// the scope holds no line at all, and a report that never found its window must not read as a
/// quiet one. A window of days always begins somewhere. Spec 6.2, 13.2.
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

/// What the window's own records say klin did not see. A positive claim about a window klin did
/// not measure whole is the one lie the report must not tell, so the reader decides this once and
/// the report puts it above the value it found. Spec 13.2.
#[derive(Default)]
struct Confidence {
    /// The files a capability did not measure, from the scope's records.
    not_measured: Vec<String>,
    /// The holes the scope's measurements left.
    holes: usize,
    /// Checks a stop ran whose execution erred.
    errored: Vec<String>,
    /// Journal lines the reader could not take.
    skipped: u64,
    /// The window the person asked for, where klin could not tell where it begins.
    unscoped: Option<String>,
}

impl Confidence {
    fn read(evidence: &Evidence, lines: &[Value], skipped: u64) -> Confidence {
        let mut held = Confidence {
            skipped,
            holes: evidence.holes.len(),
            ..Confidence::default()
        };
        for item in &evidence.not_measured {
            if !held.not_measured.contains(&item.file) {
                held.not_measured.push(item.file.clone());
            }
        }
        for row in lines
            .iter()
            .flat_map(|line| self::held(line, "capabilities"))
        {
            let name = word(row, "name");
            if word(row, "execution") == "error" && !held.errored.iter().any(|held| held == name) {
                held.errored.push(name.to_string());
            }
        }
        held
    }

    /// The one sentence the report puts above its value claim, and `None` for a window klin
    /// measured whole. The narrowest claim the records support wins, and `--details` carries the
    /// rows behind it.
    fn gap(&self) -> Option<String> {
        if self.unscoped.is_some() {
            return self.unscoped.clone();
        }
        if !self.not_measured.is_empty() {
            return Some(plural(
                self.not_measured.len(),
                "file wasn't measured",
                "files weren't measured",
            ));
        }
        if !self.errored.is_empty() {
            let count = self.errored.len();
            return Some(plural(
                count,
                "check couldn't finish",
                "checks couldn't finish",
            ));
        }
        if self.holes > 0 {
            return Some(plural(
                self.holes,
                "measurement was incomplete",
                "measurements were incomplete",
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

    fn whole(&self) -> bool {
        self.gap().is_none()
    }
}

/// One review item of the scope, keyed by its check, kind, file and text. Spec 13.2.
struct Review {
    check: Value,
    kind: String,
    file: Value,
    text: Value,
    reason: Value,
    last_seen: u64,
}

/// One hole of the scope, keyed by its check and reason.
struct Holed {
    check: Value,
    reason: String,
    last_seen: u64,
}

/// One file a capability did not measure, keyed by its check, reason and file.
struct Unmeasured {
    check: Value,
    reason: String,
    file: String,
    last_seen: u64,
}

/// The evidence beside the regressions: review items, holes, files not measured, advisory Stops,
/// and the notices of the open window that only the journal holds. Spec 13.2, 13.3.
#[derive(Default)]
struct Evidence {
    reviews: Vec<Review>,
    holes: Vec<Holed>,
    not_measured: Vec<Unmeasured>,
    advisory: Vec<Dated>,
    notices: Vec<Dated>,
}

impl Evidence {
    /// The evidence of the scope's Stop lines, beside the open window's notices. Spec 13.2.
    fn read(lines: &[Value], notices: Vec<Dated>) -> Evidence {
        let mut out = Evidence {
            notices,
            ..Evidence::default()
        };
        for line in lines.iter().filter(|line| kind(line) == "stop") {
            let time = at(line);
            if let Some(reason) = line["advisory"].as_str() {
                out.advisory.push(Dated {
                    time,
                    text: reason.to_string(),
                });
            }
            out.reviewed(line, time);
            out.holed(line, time);
            out.unmeasured(line, time);
        }
        out
    }

    /// The review items a Stop reported, each kept once with the last time a Stop saw it.
    fn reviewed(&mut self, line: &Value, time: u64) {
        for item in held(line, "reviews") {
            let same = |one: &Review| {
                one.check == item["check"]
                    && one.kind == word(item, "kind")
                    && one.file == item["file"]
                    && one.text == item["text"]
            };
            let one = upsert(&mut self.reviews, same, || Review {
                check: item["check"].clone(),
                kind: word(item, "kind").to_string(),
                file: item["file"].clone(),
                text: item["text"].clone(),
                reason: Value::Null,
                last_seen: time,
            });
            one.last_seen = time;
            one.reason = item["reason"].clone();
        }
    }

    /// The holes a Stop's measurements left, each kept once by check and reason. Spec 8.1.
    fn holed(&mut self, line: &Value, time: u64) {
        for record in held(line, "measurements") {
            for hole in list(record, "holes") {
                let (check, reason) = (&record["check"], word(hole, "reason"));
                let same = |one: &Holed| one.check == *check && one.reason == reason;
                upsert(&mut self.holes, same, || Holed {
                    check: check.clone(),
                    reason: reason.to_string(),
                    last_seen: time,
                })
                .last_seen = time;
            }
        }
    }

    /// The files a Stop did not measure: an opened gap, a coverage note, and a lost file.
    /// Spec 7.2.
    fn unmeasured(&mut self, line: &Value, time: u64) {
        let gaps = held(line, "reviews")
            .iter()
            .filter(|item| word(item, "kind") == "unmeasured")
            .map(|item| (&item["check"], word(item, "reason"), word(item, "file")));
        let limits = held(line, "notes")
            .iter()
            .filter(|note| note["coverage"] == true)
            .map(|note| (&note["check"], word(note, "kind"), word(note, "file")));
        let lost = held(line, "findings")
            .iter()
            .filter(|finding| word(finding, "kind") == MEASUREMENT_LOST)
            .map(|finding| {
                let reason = word(&finding["values"], "reason");
                (&finding["check"], reason, word(finding, "file"))
            });
        for (check, reason, file) in gaps.chain(limits).chain(lost) {
            let same =
                |one: &Unmeasured| one.check == *check && one.reason == reason && one.file == file;
            upsert(&mut self.not_measured, same, || Unmeasured {
                check: check.clone(),
                reason: reason.to_string(),
                file: file.to_string(),
                last_seen: time,
            })
            .last_seen = time;
        }
    }
}

/// The entry of `list` that `same` finds, or a new one `made` pushes, so each key is kept once.
fn upsert<T>(list: &mut Vec<T>, same: impl Fn(&T) -> bool, made: impl FnOnce() -> T) -> &mut T {
    let at = match list.iter().position(same) {
        Some(at) => at,
        None => {
            list.push(made());
            list.len() - 1
        }
    };
    &mut list[at]
}

fn plural(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        _ => format!("{count} {many}"),
    }
}

/// One thing the person was asked about. Neither is a regression: a guard refusal asks nothing,
/// and a deleted test klin let through is a question the person answers. They stay out of the
/// count and keep their own trail. Spec 13.2.
enum Audit {
    Guard {
        time: u64,
        decision: String,
        reason: String,
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
            Audit::Guard { time, .. } | Audit::Deleted { time, .. } => *time,
        }
    }

    fn sentence(&self) -> String {
        match self {
            Audit::Guard {
                decision, reason, ..
            } => guarded(decision, reason),
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
const GUARDED: [(&str, &str); 6] = [
    ("config-write", "an edit to klin.json"),
    ("state-write", "an edit to klin's own state"),
    ("config-mention", "a command that named klin.json"),
    ("state-mention", "a command that named klin's own state"),
    ("setup", "klin setup, which only you run"),
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

/// Everything the person was asked about in the window, newest first. Spec 13.2.
fn audit_trail(lines: &[Value], held: &[Regression]) -> Vec<Audit> {
    let mut out: Vec<Audit> = lines
        .iter()
        .filter(|line| kind(line) == "guard")
        .map(|line| Audit::Guard {
            time: at(line),
            decision: word(line, "decision").to_string(),
            reason: word(line, "reason").to_string(),
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

/// One report, read once: the window's regressions, the audit trail and the evidence beside
/// them, and what klin could not measure. Each surface formats this and works nothing out again.
struct Report {
    scope: Scope,
    regressions: Vec<Regression>,
    audit: Vec<Audit>,
    evidence: Evidence,
    confidence: Confidence,
    counts: Counts,
    now: u64,
    /// The newest session id the scope's lines carry.
    session: Option<String>,
    stops: usize,
    ms: u64,
    /// Whether the journal held no line at all, which is a first run and not a quiet window.
    empty: bool,
}

impl Report {
    fn read(
        lines: &[Value],
        scope: Scope,
        now: u64,
        skipped: u64,
        notices: Vec<Dated>,
        empty: bool,
    ) -> Report {
        let regressions = regressions(lines);
        let evidence = Evidence::read(lines, notices);
        Report {
            counts: Counts::of(&regressions),
            audit: audit_trail(lines, &regressions),
            confidence: Confidence::read(&evidence, lines, skipped),
            evidence,
            regressions,
            scope,
            now,
            session: lines
                .iter()
                .rev()
                .map(|line| word(line, "session"))
                .find(|session| !session.is_empty())
                .map(str::to_string),
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
    sites(out, report, args.details);
    if args.details {
        history(out, start, report);
    }
}

fn attention(report: &Report) -> Vec<String> {
    let mut said: Vec<String> = report
        .confidence
        .gap()
        .map(|gap| format!("Stats may be incomplete: {gap}."))
        .into_iter()
        .collect();
    said.extend(unsettled(&report.counts));
    said.extend(evidence_lines(&report.evidence));
    if said.is_empty() {
        said.push("Nothing needs your attention.".into());
    }
    said
}

/// The regressions no later measurement settled: open, set aside, or not compared.
fn unsettled(counts: &Counts) -> Vec<String> {
    let mut said = Vec::new();
    match counts.open {
        0 => {}
        1 => said.push("1 regression needs your attention.".into()),
        open => said.push(format!("{open} regressions need your attention.")),
    }
    if counts.set_aside > 0 {
        said.push(set_aside(counts.set_aside, counts.open > 0));
    }
    if counts.not_compared > 0 {
        let them = match counts.not_compared {
            1 => "it",
            _ => "them",
        };
        said.push(format!(
            "{} went under a changed measurement, so klin did not compare {them}.",
            counted(counts.not_compared)
        ));
    }
    said
}

/// The open review items, and each notice of the open window only the journal holds.
fn evidence_lines(evidence: &Evidence) -> Vec<String> {
    let mut said = Vec::new();
    match evidence.reviews.len() {
        0 => {}
        1 => said.push("1 review item is open.".into()),
        many => said.push(format!("{many} review items are open.")),
    }
    match evidence.advisory.len() {
        0 => {}
        1 => said.push("1 Stop was advisory: the history moved, so klin blocked nothing.".into()),
        many => said.push(format!(
            "{many} Stops were advisory: the history moved, so klin blocked nothing."
        )),
    }
    for notice in &evidence.notices {
        said.push(format!(
            "klin left you a notice in this window:\n{}",
            notice.text
        ));
    }
    said
}

/// A stamp that moved with no Stop judging the window makes the regressions it held unknown: not
/// fixed, and not proven to be in the tree, because the report reads the journal and never the
/// tree. Spec 13.2.
fn set_aside(aside: usize, after_open: bool) -> String {
    let why = "because the window moved before klin judged it again";
    match (after_open, aside) {
        (true, 1) => format!("1 more was set aside, {why}."),
        (true, _) => format!("{aside} more were set aside, {why}."),
        (false, 1) => format!("1 regression was set aside, {why}."),
        (false, _) => format!("{aside} regressions were set aside, {why}."),
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
        let _ = writeln!(
            out,
            "\nand {} more · klin report --details",
            open.len() - shown
        );
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
        None => label(&one.check, 1).to_string(),
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
    match catalogue::labels(gate) {
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
    evidence(out, report, offset);
    measurement(out, report);
}

/// The review items, the advisory Stops and the files not measured, which `--details` lists
/// whole. Spec 13.2.
fn evidence(out: &mut String, report: &Report, offset: i64) {
    let held = &report.evidence;
    if !held.reviews.is_empty() {
        let _ = writeln!(out, "\nReview");
        for one in &held.reviews {
            let _ = writeln!(
                out,
                "  {}  {}",
                one.file.as_str().unwrap_or_default(),
                one.text.as_str().unwrap_or(&one.kind)
            );
        }
    }
    if !held.advisory.is_empty() {
        let _ = writeln!(out, "\nAdvisory");
        for Dated { time, text: reason } in &held.advisory {
            let when = day(*time, report.now, offset);
            let _ = writeln!(
                out,
                "  {when}  the history moved ({reason}), so klin blocked nothing"
            );
        }
    }
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
        let sites: Vec<&&Regression> = held.iter().filter(|one| one.check == gate).collect();
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
        if !named.contains(&one.check) {
            named.push(one.check.clone());
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
    named(out, "no capability measured", &held.not_measured);
    if held.holes > 0 {
        let _ = writeln!(out, "  {} measurement(s) left a hole.", held.holes);
    }
    if !held.errored.is_empty() {
        let _ = writeln!(
            out,
            "  {} check(s) measured nothing: {}",
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

/// The report document of spec 13.3: the facts, and none of the person's sentences. Beside it,
/// `episodes`, `audit` and `activity` keep the trail the benchmark reads. Spec 11.7, 13.3.
fn json(out: &mut String, report: &Report) {
    let (kind, value) = match report.scope {
        Scope::Turn => ("turn", Value::Null),
        Scope::Session => ("session", report.session.clone().into()),
        Scope::Since(days) => ("since", format!("{days}d").into()),
    };
    let held = &report.evidence;
    let counts = &report.counts;
    let record = serde_json::json!({
        "schema_version": 1,
        "command": "report",
        "scope": { "kind": kind, "value": value, "known": report.confidence.unscoped.is_none() },
        "counts": {
            "caught": counts.caught,
            "fixed": counts.fixed + counts.config_changed,
            "open": counts.open,
            "not_compared": counts.not_compared,
            "set_aside": counts.set_aside,
            "reviews": held.reviews.len(),
            "holes": held.holes.len(),
            "not_measured": held.not_measured.len(),
            "notices": held.notices.len(),
            "advisory": held.advisory.len(),
        },
        "regressions": report
            .regressions
            .iter()
            .filter(|one| one.outcome.counted())
            .map(regression)
            .collect::<Vec<Value>>(),
        "reviews": held.reviews.iter().map(|one| serde_json::json!({
            "check": one.check, "kind": one.kind, "file": one.file, "text": one.text,
            "reason": one.reason, "last_seen": one.last_seen,
        })).collect::<Vec<Value>>(),
        "holes": held.holes.iter().map(|one| serde_json::json!({
            "check": one.check, "reason": one.reason, "last_seen": one.last_seen,
        })).collect::<Vec<Value>>(),
        "not_measured": held.not_measured.iter().map(|one| serde_json::json!({
            "check": one.check, "reason": one.reason, "file": one.file,
            "last_seen": one.last_seen,
        })).collect::<Vec<Value>>(),
        "notices": held.notices.iter().map(|notice| serde_json::json!({
            "time": notice.time, "message": notice.text, "delivered": false,
        })).collect::<Vec<Value>>(),
        "advisory": held.advisory.iter().map(|advisory| serde_json::json!({
            "time": advisory.time, "reason": advisory.text,
        })).collect::<Vec<Value>>(),
        "skipped_lines": report.confidence.skipped,
        "episodes": report.regressions.iter().map(episode).collect::<Vec<Value>>(),
        "audit": report.audit.iter().map(Audit::record).collect::<Vec<Value>>(),
        "activity": { "stops": report.stops, "klin_ms": report.ms },
    });
    let _ = writeln!(out, "{record}");
}

/// One regression as the report document names it. Spec 13.3.
fn regression(one: &Regression) -> Value {
    serde_json::json!({
        "id": one.id,
        "check": one.check,
        "file": one.file,
        "text": one.text,
        "state": one.outcome.state(),
        "config_changed": one.outcome == Outcome::ConfigChanged,
        "first_seen": one.first,
        "last_seen": one.last,
    })
}

fn episode(one: &Regression) -> Value {
    serde_json::json!({
        "gate": one.check,
        "label": label(&one.check, 1),
        "id": one.id,
        "key": {
            "gate": one.check,
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
