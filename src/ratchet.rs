use std::collections::BTreeMap;
use std::fmt::Write;
use std::hash::{DefaultHasher, Hash, Hasher};

use serde_json::{Map, Value};

use crate::check::{Context, Records, Sink};
use crate::config::{self, Config, Error};

/// The engine every ratcheting gate judges through. It exposes `Values`, `Section` and
/// `section`, `no_retired_key`, `Finding` with the `body_hash` its site is keyed by, `accepted`,
/// `noted`, `scoped`, `identity`, `Line`, and `Evaluator` with its `evaluate` call. Everything
/// else here, the matcher and the reporter included, is private.
pub type Values = Map<String, Value>;

/// The key a finding carries when it matched an accepted entry, which is a record field of spec
/// 11.2 and not the config key `config::ACCEPTED` of the same spelling.
const ACCEPTED: &str = "accepted";

const BODY: &str = "body_hash";

const RETIRED: &[(&str, &str)] = &[
    (
        "baseline",
        "which is not a key klin reads — a run compares the working tree against the base \
         commit, and a person accepts debt in the \"accepted\" list. Delete the key and the \
         file it names.",
    ),
    (
        "sources",
        "which klin now spells \"roots\", the name every section uses for the same thing. \
         Rename the key, so nothing measures a different set in silence.",
    ),
];

/// A pattern row klin retired from a built-in table, as the gate that held it, the values key
/// its name is recorded under, the name, the file suffix the row read, and where the row went.
/// An accepted entry naming one matches nothing for a reason the entry cannot show, so the note
/// and the `--strict` failure say which row it names and what replaced it. The suffix is part of
/// the key, because a table retires one language's row and keeps the same name for another's. A
/// row a project deleted from its own `patterns` is not one of these. Section 14.
const RETIRED_ROWS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "escapes",
        "escape",
        "todo",
        ".rs",
        "is a row klin retired, which moved to the \"stubs\" check, so one site is never \
         reported by two checks. Accept the site as {\"gate\": \"stubs\", \"stub\": \"not \
         implemented\"}, or delete the entry.",
    ),
    (
        "escapes",
        "escape",
        "skipped test",
        ".py",
        "is a row klin narrowed, whose Python pattern now ends in a word boundary, so \
         `pytest.mark.skipif` no longer matches it: a conditional skip states which platforms a \
         test supports. Delete the entry.",
    ),
];

/// What one unmatched accepted entry names, when it names a row klin retired.
fn retired_row(gate: &str, entry: &Values) -> Option<String> {
    RETIRED_ROWS
        .iter()
        .find(|(named, key, row, suffix, _)| {
            *named == gate && text(entry, key) == *row && text(entry, "file").ends_with(suffix)
        })
        .map(|(_, _, row, _, went)| format!("\"{row}\" {went}"))
}

/// A section naming a key klin retired, refused before any gate runs. Section 14.
pub fn no_retired_key(
    file: &std::path::Path,
    name: &str,
    values: &Map<String, Value>,
) -> Result<(), Error> {
    for (retired, why) in RETIRED {
        if values.contains_key(*retired) {
            return Err(Error(format!(
                "{}: \"{name}\" names a \"{retired}\", {why}",
                file.display()
            )));
        }
    }
    Ok(())
}

#[derive(Clone)]
pub struct Finding {
    pub file: String,
    pub line: u64,
    pub text: String,
    pub values: Values,
    pub body: Option<u64>,
}

/// The key the cross-file pass of spec 4.4 matches on: everything below the first line, with
/// every run of whitespace collapsed. Dropping that line drops the name, so a rename matches
/// too, and collapsing whitespace means a move that reindents matches. A declaration that fits
/// on one line has nothing below it, so the whole line is its key. A declaration that wraps
/// over several lines keeps its later lines, like the declaration line of ADR 0008 does.
pub fn body_hash(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    let body = source.split_once('\n').map_or(source, |(_, body)| body);
    for word in body.split_whitespace() {
        word.hash(&mut hasher);
    }
    hasher.finish()
}

impl Finding {
    fn entry(&self) -> Values {
        let mut out = Values::new();
        out.insert("file".into(), self.file.clone().into());
        out.insert("text".into(), self.text.clone().into());
        out.insert("line".into(), self.line.into());
        if let Some(body) = self.body {
            out.insert(BODY.into(), body.into());
        }
        out.extend(
            self.values
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        out
    }
}

/// The debt a person accepted in the config, as prior entries for one gate. An entry must carry
/// every value the gate ratchets, or it would hold a site at any value it grows to.
pub fn accepted(config: &Config, gate: &str, metrics: &[&str]) -> Result<Vec<Values>, Error> {
    accepted_leaving_out(config, gate, metrics, &[])
}

pub fn accepted_leaving_out(
    config: &Config,
    gate: &str,
    metrics: &[&str],
    optional: &[&str],
) -> Result<Vec<Values>, Error> {
    let section = config::ACCEPTED.name;
    let Some(listed) = config.pinned(section) else {
        return Ok(Vec::new());
    };
    let shape = || {
        Error(format!(
            "{}: \"{section}\" is a list of {{\"gate\", \"file\", \"text\"}} entries, each with \
             the value that gate allows",
            config.file.display()
        ))
    };
    let mut out = Vec::new();
    for item in listed.as_array().ok_or_else(shape)? {
        let mut entry = item.as_object().ok_or_else(shape)?.clone();
        let named = entry
            .get("gate")
            .and_then(Value::as_str)
            .ok_or_else(shape)?;
        if named != gate {
            continue;
        }
        names_every_value(config, gate, &entry, metrics, optional)?;
        entry.remove(BODY);
        entry.insert(ACCEPTED.into(), true.into());
        out.push(entry);
    }
    Ok(out)
}

fn names_every_value(
    config: &Config,
    gate: &str,
    entry: &Values,
    metrics: &[&str],
    optional: &[&str],
) -> Result<(), Error> {
    let missing: Vec<&str> = metrics
        .iter()
        .copied()
        .filter(|metric| entry.contains_key(*metric) || !optional.contains(metric))
        .filter(|metric| entry.get(*metric).and_then(Value::as_f64).is_none())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(Error(format!(
        "{}: the accepted entry for {} in {} does not give a number for {}, which {gate} \
         ratchets — a value it leaves out would grow unjudged at that site",
        config.file.display(),
        text(entry, "text"),
        text(entry, "file"),
        missing.join(", ")
    )))
}

/// The NOTE lines a gate leaves about what it did not judge, printed for a person and kept for
/// `--json`. A note carries no ceiling, so none of these fails anything. Spec 8.6, 11.
pub fn noted(notes: &[(String, String)], out: &mut Sink) {
    noted_as("note", notes, out);
}

pub fn noted_as(outcome: &str, notes: &[(String, String)], out: &mut Sink) {
    for (_, why) in notes {
        let _ = writeln!(out.text, "NOTE: {why}");
    }
    out.record(|records| {
        for (at, why) in notes {
            let mut record = Map::new();
            record.insert("outcome".into(), outcome.into());
            record.insert("file".into(), at.clone().into());
            record.insert("text".into(), why.clone().into());
            records.notes.push(Value::Object(record));
        }
    });
}

fn is_accepted(entry: &Values) -> bool {
    entry.get(ACCEPTED).is_some()
}

/// How many of a gate's findings a scoped run judges, which is what its OK line counts.
pub fn scoped(found: &[Finding], only: Option<&[String]>) -> usize {
    match only {
        Some(only) => found
            .iter()
            .filter(|site| only.contains(&site.file))
            .count(),
        None => found.len(),
    }
}

#[derive(Default)]
struct Comparison {
    unmatched_findings: Vec<Finding>,
    rose: Vec<(Finding, Values)>,
    unmatched_accepted: Vec<Values>,
    /// The findings a base site or an accepted entry held, with the entry that held each, which
    /// is every finding on a passing run and none of the new or risen ones on a failing one.
    /// Spec 11.2.
    held: Vec<(Finding, Values)>,
}

impl Comparison {
    fn failed(&self) -> bool {
        !self.unmatched_findings.is_empty() || !self.rose.is_empty()
    }

    /// The held findings an accepted entry holds, and the ones a base site holds.
    fn reasons(&self) -> (usize, usize) {
        let accepted = self
            .held
            .iter()
            .filter(|(_, entry)| is_accepted(entry))
            .count();
        (accepted, self.held.len() - accepted)
    }
}

/// What a check says of the state it measured, and what follows the qualifier the ratchet adds.
/// A check owns its own vocabulary and its counts. It never writes why its findings pass,
/// because only the comparison knows that, so no check can claim a base comparison that did not
/// happen. Spec 8.6.
pub struct Line<'a> {
    pub state: &'a str,
    pub tail: &'a str,
}

/// Why every current finding passed, in the words the comparison proved. A run that judged no
/// finding claims nothing: there was nothing for the base or the accepted list to hold.
/// Spec 8.6.
fn qualifier(comparison: &Comparison) -> String {
    match comparison.reasons() {
        (0, 0) => String::new(),
        (0, _) => ", all held at the base".to_string(),
        (_, 0) => ", all on the accepted list".to_string(),
        (accepted, base) => {
            format!(", {base} held at the base and {accepted} on the accepted list")
        }
    }
}

fn restrict(
    findings: Vec<Finding>,
    entries: Vec<Values>,
    only: Option<&[String]>,
) -> (Vec<Finding>, Vec<Values>) {
    let Some(only) = only else {
        return (findings, entries);
    };
    let wanted = |file: &str| only.iter().any(|name| name == file);
    (
        findings
            .into_iter()
            .filter(|finding| wanted(&finding.file))
            .collect(),
        entries
            .into_iter()
            .filter(|entry| {
                entry
                    .get("file")
                    .and_then(Value::as_str)
                    .is_some_and(wanted)
            })
            .collect(),
    )
}

enum Outcome {
    Rose,
    Held,
}

fn rose(finding: &Finding, entry: &Values, metric: &str) -> bool {
    let Some(now) = finding.values.get(metric).and_then(Value::as_f64) else {
        return false;
    };
    entry
        .get(metric)
        .and_then(Value::as_f64)
        .is_none_or(|was| now > was)
}

fn compare(finding: &Finding, entry: &Values, metrics: &[&str]) -> Outcome {
    match metrics.iter().any(|metric| rose(finding, entry, metric)) {
        true => Outcome::Rose,
        false => Outcome::Held,
    }
}

fn distance(finding: &Finding, entry: &Values, by_line: bool) -> u64 {
    if !by_line || is_accepted(entry) {
        return 0;
    }
    entry
        .get("line")
        .and_then(Value::as_u64)
        .map_or(0, |line| line.abs_diff(finding.line))
}

fn match_group(
    findings: Vec<Finding>,
    entries: Vec<Values>,
    metrics: &[&str],
    by_line: bool,
) -> (Vec<(Finding, Values)>, Vec<Finding>, Vec<Values>) {
    let mut candidates = Vec::new();
    for (at_finding, finding) in findings.iter().enumerate() {
        for (at_entry, entry) in entries.iter().enumerate() {
            let shared = metrics
                .iter()
                .filter(|metric| {
                    entry
                        .get(**metric)
                        .is_some_and(|value| finding.values.get(**metric) == Some(value))
                })
                .count() as i64;
            let rose = matches!(compare(finding, entry, metrics), Outcome::Rose);
            candidates.push((
                rose,
                -shared,
                distance(finding, entry, by_line),
                at_finding,
                at_entry,
            ));
        }
    }
    candidates.sort();
    let mut findings: Vec<Option<Finding>> = findings.into_iter().map(Some).collect();
    let mut entries: Vec<Option<Values>> = entries.into_iter().map(Some).collect();
    let mut pairs = Vec::new();
    for (_, _, _, at_finding, at_entry) in candidates {
        if findings[at_finding].is_none() || entries[at_entry].is_none() {
            continue;
        }
        if let (Some(finding), Some(entry)) =
            (findings[at_finding].take(), entries[at_entry].take())
        {
            pairs.push((finding, entry));
        }
    }
    let unmatched_findings = findings.into_iter().flatten().collect();
    let unmatched_entries = entries.into_iter().flatten().collect();
    (pairs, unmatched_findings, unmatched_entries)
}

/// The cross-file pass of spec 4.4. What the primary match left untaken pairs again by body
/// hash, across files, so a function moved with its body unchanged keeps its site. A line
/// distance between two files means nothing, so it does not rank here. Only a `before` site
/// carries a hash, so an accepted entry is never matched this way.
fn moved(
    findings: Vec<Finding>,
    entries: Vec<Values>,
    metrics: &[&str],
) -> (Vec<(Finding, Values)>, Vec<Finding>, Vec<Values>) {
    let mut groups: BTreeMap<u64, (Vec<Finding>, Vec<Values>)> = BTreeMap::new();
    let mut findings_left = Vec::new();
    let mut entries_left = Vec::new();
    for entry in entries {
        match entry.get(BODY).and_then(Value::as_u64) {
            Some(hash) => groups.entry(hash).or_default().1.push(entry),
            None => entries_left.push(entry),
        }
    }
    for finding in findings {
        match finding.body {
            Some(hash) => groups.entry(hash).or_default().0.push(finding),
            None => findings_left.push(finding),
        }
    }
    let mut pairs = Vec::new();
    for (mut group_findings, mut group_entries) in groups.into_values() {
        group_findings.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
        group_entries.sort_by_key(entry_site);
        let (found, findings_over, entries_over) =
            match_group(group_findings, group_entries, metrics, false);
        pairs.extend(found);
        findings_left.extend(findings_over);
        entries_left.extend(entries_over);
    }
    (pairs, findings_left, entries_left)
}

fn entry_site(entry: &Values) -> (String, u64) {
    (
        text(entry, "file"),
        entry.get("line").and_then(Value::as_u64).unwrap_or(0),
    )
}

/// The untaken entries of one site that carry forward. An untaken `before` entry has no outcome
/// (16.5), but only a site the `after` tree no longer holds can be a move target: an entry that
/// an accepted one outranked is still in place, so a body-identical function elsewhere is a copy
/// and not a move. Spec 4.4. An accepted entry carries forward for its NOTE.
fn untaken(entries: Vec<Values>, mut lost: usize) -> Vec<Values> {
    let mut out = Vec::new();
    for entry in entries {
        if is_accepted(&entry) {
            out.push(entry);
        } else if lost > 0 {
            lost -= 1;
            out.push(entry);
        }
    }
    out
}

fn take(comparison: &mut Comparison, pairs: Vec<(Finding, Values)>, metrics: &[&str]) {
    for (finding, entry) in pairs {
        match compare(&finding, &entry, metrics) {
            Outcome::Rose => comparison.rose.push((finding, entry)),
            Outcome::Held => comparison.held.push((finding, entry)),
        }
    }
}

fn judge(findings: Vec<Finding>, entries: Vec<Values>, metrics: &[&str]) -> Comparison {
    let mut comparison = Comparison::default();
    let mut groups: BTreeMap<(String, String), (Vec<Finding>, Vec<Values>)> = BTreeMap::new();
    for entry in entries {
        let key = (text(&entry, "file"), text(&entry, "text"));
        groups.entry(key).or_default().1.push(entry);
    }
    for finding in findings {
        let key = (finding.file.clone(), finding.text.clone());
        groups.entry(key).or_default().0.push(finding);
    }
    let mut findings_left = Vec::new();
    let mut entries_left = Vec::new();
    for (group_findings, group_entries) in groups.into_values() {
        let lost = group_entries
            .iter()
            .filter(|entry| !is_accepted(entry))
            .count()
            .saturating_sub(group_findings.len());
        let (pairs, findings_over, entries_over) =
            match_group(group_findings, group_entries, metrics, true);
        take(&mut comparison, pairs, metrics);
        findings_left.extend(findings_over);
        entries_left.extend(untaken(entries_over, lost));
    }
    let (pairs, findings_left, entries_left) = moved(findings_left, entries_left, metrics);
    take(&mut comparison, pairs, metrics);
    comparison.unmatched_findings = findings_left;
    comparison.unmatched_accepted = entries_left.into_iter().filter(is_accepted).collect();
    comparison
        .unmatched_findings
        .sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    comparison
        .rose
        .sort_by(|(a, _), (b, _)| (&a.file, a.line).cmp(&(&b.file, b.line)));
    comparison
}

/// For each new finding, the one whose group it prints inside, or `None` where it leads.
pub type Nesting = fn(&[Finding]) -> Vec<Option<usize>>;

#[derive(Clone, Copy)]
pub enum Remedy<'a> {
    Fixed(&'a str),
    ByValues(fn(&[&Values]) -> String),
}

impl Remedy<'_> {
    fn text(self, values: &[&Values]) -> String {
        match self {
            Remedy::Fixed(text) => text.to_string(),
            Remedy::ByValues(built) => built(values),
        }
    }
}

pub struct Evaluator<'a> {
    pub metrics: &'a [&'a str],
    pub unit: &'a str,
    pub condition: &'a str,
    pub fix_advice: Remedy<'a>,
    /// The ceiling in force, printed beside every failure and carried in the JSON. `None` for a
    /// gate whose only ceiling is the value the base holds, which each failure already names.
    /// Spec 4.7, 8.6.
    pub ceiling: Option<&'a str>,
    pub format_metrics: fn(&Values) -> String,
    /// For each new finding, the one whose group it prints inside, for a gate whose text report
    /// groups what one change took away. A lead is a finding that prints inside no group.
    /// Presentation only: every finding keeps its own identity, record and count.
    pub nested: Option<Nesting>,
}

impl Evaluator<'_> {
    /// Judge today's findings against the ones the base commit holds, plus the accepted list.
    pub fn evaluate(
        &self,
        findings: Vec<Finding>,
        prior: Vec<Finding>,
        accepted: Vec<Values>,
        at: &Context,
        line: Line,
        out: &mut Sink,
    ) -> u8 {
        let entries: Vec<Values> = accepted
            .into_iter()
            .chain(prior.iter().map(Finding::entry))
            .collect();
        let (findings, entries) = restrict(findings, entries, at.only);
        let held = entries.len();
        let comparison = judge(findings, entries, self.metrics);
        report(&comparison, self, held, line, at, out)
    }
}

/// What the ratchet makes of each finding, without printing or recording anything: `new`,
/// `worsened`, `held` by a base site, or `accepted` by an entry a person wrote. For a report that
/// explains a gate rather than judging it. Spec 8.4.
pub fn outcomes(
    findings: Vec<Finding>,
    prior: Vec<Finding>,
    accepted: Vec<Values>,
    metrics: &[&str],
) -> Vec<(Finding, &'static str)> {
    let entries = accepted
        .into_iter()
        .chain(prior.iter().map(Finding::entry))
        .collect();
    let comparison = judge(findings, entries, metrics);
    let held = comparison.held.into_iter().map(|(finding, entry)| {
        let outcome = if is_accepted(&entry) {
            "accepted"
        } else {
            "held"
        };
        (finding, outcome)
    });
    let mut out: Vec<(Finding, &'static str)> = comparison
        .unmatched_findings
        .into_iter()
        .map(|finding| (finding, "new"))
        .chain(
            comparison
                .rose
                .into_iter()
                .map(|(finding, _)| (finding, "worsened")),
        )
        .chain(held)
        .collect();
    out.sort_by(|(a, _), (b, _)| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

fn report(
    comparison: &Comparison,
    evaluator: &Evaluator,
    held: usize,
    line: Line,
    at: &Context,
    out: &mut Sink,
) -> u8 {
    let (accepted, _) = comparison.reasons();
    out.record(|records| {
        records.held = Some(records.held.unwrap_or(0) + comparison.held.len() as u64);
        records.accepted = Some(records.accepted.unwrap_or(0) + accepted as u64);
        collect(comparison, evaluator, at.gate, records);
    });
    if comparison.failed() {
        failures(comparison, evaluator, held, out.text);
        notes(comparison, evaluator, at.gate, out.text);
        return 1;
    }
    if !at.quiet {
        let _ = writeln!(
            out.text,
            "OK: {}{}{}",
            line.state,
            qualifier(comparison),
            line.tail
        );
    }
    notes(comparison, evaluator, at.gate, out.text);
    if at.strict && !comparison.unmatched_accepted.is_empty() {
        let heading = format!(
            "FAIL: the accepted list holds {} entr{} that matched nothing — under --strict an \
             entry that no longer describes the code is a failure. Delete the line.",
            comparison.unmatched_accepted.len(),
            match comparison.unmatched_accepted.len() {
                1 => "y",
                _ => "ies",
            }
        );
        listed(
            out.text,
            &heading,
            comparison
                .unmatched_accepted
                .iter()
                .filter_map(|entry| {
                    retired_row(at.gate, entry)
                        .map(|went| format!("{}: {went}", text(entry, "file")))
                })
                .collect(),
        );
        return 1;
    }
    0
}

fn failures(comparison: &Comparison, evaluator: &Evaluator, held: usize, out: &mut String) {
    if !comparison.unmatched_findings.is_empty() {
        let _ = writeln!(
            out,
            "FAIL: {} new {} {}, beyond the {} the base holds:",
            comparison.unmatched_findings.len(),
            evaluator.unit,
            evaluator.condition,
            held
        );
        let found = &comparison.unmatched_findings;
        for (lead, inside) in grouped(found, evaluator.nested) {
            let members = inside.into_iter().map(|at| ("  ", &found[at]));
            for (indent, finding) in std::iter::once(("", &found[lead])).chain(members) {
                let _ = writeln!(
                    out,
                    "  {indent}{}:{}  {}  {}{}",
                    finding.file,
                    finding.line,
                    (evaluator.format_metrics)(&finding.values),
                    clip(&finding.text),
                    against(None, evaluator)
                );
            }
        }
    }
    if !comparison.rose.is_empty() {
        let _ = writeln!(
            out,
            "FAIL: {} {} got worse — the ratchet only tightens:",
            comparison.rose.len(),
            evaluator.unit
        );
        for (finding, entry) in &comparison.rose {
            let _ = writeln!(
                out,
                "  {}:{}  {}, was {}{}  {}{}",
                finding.file,
                finding.line,
                (evaluator.format_metrics)(&finding.values),
                (evaluator.format_metrics)(entry),
                came_from(finding, entry),
                clip(&finding.text),
                against(Some(entry), evaluator)
            );
        }
    }
    let _ = writeln!(out, "{}", remedy(comparison, evaluator));
}

/// Every new finding once, as the leads in their order, each with the findings that print inside
/// its group. A finding whose named lead is itself inside a group, or out of range, leads.
fn grouped(found: &[Finding], nested: Option<Nesting>) -> Vec<(usize, Vec<usize>)> {
    let under = nested.map_or_else(|| vec![None; found.len()], |nested| nested(found));
    let lead = |at: usize| {
        under
            .get(at)
            .copied()
            .flatten()
            .filter(|held| *held != at && under.get(*held).is_some_and(Option::is_none))
    };
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut slot: Vec<Option<usize>> = vec![None; found.len()];
    for at in (0..found.len()).filter(|at| lead(*at).is_none()) {
        slot[at] = Some(groups.len());
        groups.push((at, Vec::new()));
    }
    for at in 0..found.len() {
        if let Some(group) = lead(at).and_then(|held| slot[held]) {
            groups[group].1.push(at);
        }
    }
    groups
}

/// What one failure was judged against: the `before` site it matched, or the accepted entry, or
/// nothing at all, and the ceiling in force beside it. A person disputes a wrong match from
/// this, and an agent fixes the site the ratchet actually compared. Spec 8.6.
fn against(entry: Option<&Values>, evaluator: &Evaluator) -> String {
    let matched = match entry {
        None => "nothing matched".to_string(),
        Some(entry) if is_accepted(entry) => {
            format!("matched the accepted entry for {}", text(entry, "file"))
        }
        Some(entry) => {
            let (file, line) = entry_site(entry);
            format!("matched the base site at {file}:{line}")
        }
    };
    match evaluator.ceiling {
        Some(ceiling) => format!("  — {matched}, ceiling {ceiling}"),
        None => format!("  — {matched}"),
    }
}

/// The file a failure's own line does not name, which is the one the second pass of 4.4 matched
/// it to. Without it a function that moved and grew reads as a regression where it now sits.
fn came_from(finding: &Finding, entry: &Values) -> String {
    match text(entry, "file") {
        was if was.is_empty() || was == finding.file => String::new(),
        was => format!(" at {was}"),
    }
}

fn remedy(comparison: &Comparison, evaluator: &Evaluator) -> String {
    let failing: Vec<Values> = comparison
        .unmatched_findings
        .iter()
        .map(|finding| finding.values.clone())
        .chain(
            comparison
                .rose
                .iter()
                .map(|(finding, entry)| risen(finding, entry, evaluator.metrics)),
        )
        .collect();
    let failing: Vec<&Values> = failing.iter().collect();
    evaluator.fix_advice.text(&failing)
}

fn risen(finding: &Finding, entry: &Values, metrics: &[&str]) -> Values {
    metrics
        .iter()
        .filter(|metric| rose(finding, entry, metric))
        .filter_map(|metric| Some((metric.to_string(), finding.values.get(*metric)?.clone())))
        .collect()
}

fn text(entry: &Values, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn notes(comparison: &Comparison, evaluator: &Evaluator, gate: &str, out: &mut String) {
    if comparison.unmatched_accepted.is_empty() {
        return;
    }
    let count = comparison.unmatched_accepted.len();
    let plural = if count == 1 { "y" } else { "ies" };
    listed(
        out,
        &format!("NOTE: {count} accepted entr{plural} matched nothing this run:"),
        comparison
            .unmatched_accepted
            .iter()
            .map(|entry| {
                format!(
                    "{}  {}  {}{}",
                    text(entry, "file"),
                    (evaluator.format_metrics)(entry),
                    clip(&text(entry, "text")),
                    match retired_row(gate, entry) {
                        Some(went) => format!("  — {went}"),
                        None => String::new(),
                    }
                )
            })
            .collect(),
    );
}

fn listed(out: &mut String, heading: &str, rows: Vec<String>) {
    let _ = writeln!(out, "{heading}");
    for row in rows.iter().take(20) {
        let _ = writeln!(out, "  {row}");
    }
    if rows.len() > 20 {
        let _ = writeln!(out, "  … and {} more", rows.len() - 20);
    }
}

fn clip(text: &str) -> String {
    text.chars().take(70).collect()
}

fn collect(comparison: &Comparison, evaluator: &Evaluator, gate: &str, records: &mut Records) {
    let failing = |outcome, finding: &Finding, entry: Option<&Values>| {
        let mut out = site(outcome, &finding.file, Some(finding.line), &finding.text);
        out.insert("id".into(), identity(gate, finding).into());
        out.insert("values".into(), Value::Object(finding.values.clone()));
        out.insert("condition".into(), evaluator.condition.into());
        let values = entry.map_or_else(
            || finding.values.clone(),
            |entry| risen(finding, entry, evaluator.metrics),
        );
        out.insert(
            "fix_advice".into(),
            evaluator.fix_advice.text(&[&values]).into(),
        );
        out.insert(
            "ceiling".into(),
            evaluator.ceiling.map_or(Value::Null, Into::into),
        );
        out.insert("matched".into(), entry.map_or(Value::Null, matched_record));
        Value::Object(out)
    };
    for finding in &comparison.unmatched_findings {
        records.findings.push(failing("new", finding, None));
    }
    for (finding, entry) in &comparison.rose {
        records
            .findings
            .push(failing("worsened", finding, Some(entry)));
    }
    for entry in &comparison.unmatched_accepted {
        records.notes.push(Value::Object(unmatched_record(entry)));
    }
}

fn site(outcome: &str, file: &str, line: Option<u64>, text: &str) -> Values {
    let mut out = Values::new();
    out.insert("outcome".into(), outcome.into());
    out.insert("file".into(), file.into());
    if let Some(line) = line {
        out.insert("line".into(), line.into());
    }
    out.insert("text".into(), text.into());
    out
}

/// The site identity of 4.4 in one token: a hash of the gate, the file and the declaration
/// text, so a harness follows one site across stops without parsing the rest. It names the
/// site and not the finding, so two findings 4.4 keys the same way share it. The path is
/// hashed too, so a rename changes the id while the site of 4.4 survives. FNV-1a, written out
/// here, so one site keeps one id across builds of klin. Spec 11.2.
pub fn identity(gate: &str, finding: &Finding) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let parts = [
        gate.as_bytes(),
        finding.file.as_bytes(),
        finding.text.as_bytes(),
    ];
    for byte in parts.join(&0u8) {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The `before` site or accepted entry a failure was matched to, with the values it held there,
/// so a harness sees both sides of the comparison. Spec 11.2.
fn matched_record(entry: &Values) -> Value {
    let mut out = Values::new();
    out.insert("file".into(), text(entry, "file").into());
    out.insert("text".into(), text(entry, "text").into());
    if let Some(line) = entry.get("line").and_then(Value::as_u64) {
        out.insert("line".into(), line.into());
    }
    out.insert(ACCEPTED.into(), is_accepted(entry).into());
    out.insert("values".into(), Value::Object(entry_values(entry)));
    Value::Object(out)
}

/// The values one entry holds, without the keys that name the site it sits at.
fn entry_values(entry: &Values) -> Values {
    let mut values = entry.clone();
    for key in ["file", "text", "line", ACCEPTED, BODY, "gate"] {
        values.remove(key);
    }
    values
}

fn unmatched_record(entry: &Values) -> Values {
    let mut out = site(
        "unmatched",
        &text(entry, "file"),
        entry.get("line").and_then(Value::as_u64),
        &text(entry, "text"),
    );
    out.insert("values".into(), Value::Object(entry_values(entry)));
    out
}
