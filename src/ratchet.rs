use std::collections::BTreeMap;
use std::fmt::Write;

use serde_json::{Map, Value};

use crate::config::{Config, Error, Flags, Records};

/// The engine every ratcheting gate judges through. It exposes six items: `Values`, `Section`
/// and `section`, `Finding`, `accepted`, and `Evaluator` with its `evaluate` call. Everything
/// else here, the matcher and the reporter included, is private.
pub type Values = Map<String, Value>;

const ACCEPTED: &str = "accepted";

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

pub struct Section<'a> {
    pub config: &'a Config,
    pub name: &'a str,
    pub values: Values,
}

pub fn section<'a>(config: &'a Config, name: &'a str) -> Result<Section<'a>, Error> {
    let Some(values) = config.section(name)?.as_object() else {
        return Err(Error(format!(
            "{}: \"{name}\" must be an object",
            config.file.display()
        )));
    };
    no_retired_key(&config.file, name, values)?;
    Ok(Section {
        config,
        name,
        values: values.clone(),
    })
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

pub struct Finding {
    pub file: String,
    pub line: u64,
    pub text: String,
    pub values: Values,
}

impl Finding {
    fn entry(&self) -> Values {
        let mut out = Values::new();
        out.insert("file".into(), self.file.clone().into());
        out.insert("text".into(), self.text.clone().into());
        out.insert("line".into(), self.line.into());
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
    let Ok(listed) = config.section(ACCEPTED) else {
        return Ok(Vec::new());
    };
    let shape = || {
        Error(format!(
            "{}: \"{ACCEPTED}\" is a list of {{\"gate\", \"file\", \"text\"}} entries, each with \
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
        names_every_value(config, gate, &entry, metrics)?;
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
) -> Result<(), Error> {
    let missing: Vec<&str> = metrics
        .iter()
        .copied()
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
}

impl Comparison {
    fn failed(&self) -> bool {
        !self.unmatched_findings.is_empty() || !self.rose.is_empty()
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

fn compare(finding: &Finding, entry: &Values, metrics: &[&str]) -> Outcome {
    let comparable: Vec<(f64, f64)> = metrics
        .iter()
        .filter_map(|metric| {
            Some((
                finding.values.get(*metric)?.as_f64()?,
                entry.get(*metric)?.as_f64()?,
            ))
        })
        .collect();
    match comparable.iter().any(|(now, was)| now > was) {
        true => Outcome::Rose,
        false => Outcome::Held,
    }
}

fn distance(finding: &Finding, entry: &Values) -> u64 {
    if is_accepted(entry) {
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
                distance(finding, entry),
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
    for (group_findings, group_entries) in groups.into_values() {
        let (mut pairs, findings_left, entries_left) =
            match_group(group_findings, group_entries, metrics);
        pairs.sort_by_key(|(finding, _)| finding.line);
        for (finding, entry) in pairs {
            match compare(&finding, &entry, metrics) {
                Outcome::Rose => comparison.rose.push((finding, entry)),
                Outcome::Held => {}
            }
        }
        comparison.unmatched_findings.extend(findings_left);
        comparison
            .unmatched_accepted
            .extend(entries_left.into_iter().filter(is_accepted));
    }
    comparison
        .unmatched_findings
        .sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    comparison
}

pub struct Evaluator<'a> {
    pub metrics: &'a [&'a str],
    pub unit: &'a str,
    pub condition: &'a str,
    pub fix_advice: &'a str,
    pub format_metrics: fn(&Values) -> String,
}

impl Evaluator<'_> {
    /// Judge today's findings against the ones the base commit holds, plus the accepted list.
    pub fn evaluate(
        &self,
        findings: Vec<Finding>,
        prior: Vec<Finding>,
        accepted: Vec<Values>,
        flags: &Flags,
        ok_line: &str,
        out: &mut String,
    ) -> u8 {
        let entries: Vec<Values> = accepted
            .into_iter()
            .chain(prior.iter().map(Finding::entry))
            .collect();
        let (findings, entries) = restrict(findings, entries, flags.only.as_deref());
        let held = entries.len();
        let comparison = judge(findings, entries, self.metrics);
        report(&comparison, self, held, ok_line, flags, out)
    }
}

fn report(
    comparison: &Comparison,
    evaluator: &Evaluator,
    held: usize,
    ok_line: &str,
    flags: &Flags,
    out: &mut String,
) -> u8 {
    flags.record(|records| collect(comparison, evaluator, records));
    if comparison.failed() {
        failures(comparison, evaluator, held, out);
        notes(comparison, evaluator, out);
        return 1;
    }
    if !flags.quiet {
        let _ = writeln!(out, "{ok_line}");
    }
    notes(comparison, evaluator, out);
    if flags.strict && !comparison.unmatched_accepted.is_empty() {
        let _ = writeln!(
            out,
            "FAIL: the accepted list holds {} entr{} that matched nothing — under --strict an \
             entry that no longer describes the code is a failure. Delete the line.",
            comparison.unmatched_accepted.len(),
            match comparison.unmatched_accepted.len() {
                1 => "y",
                _ => "ies",
            }
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
        for finding in &comparison.unmatched_findings {
            let _ = writeln!(
                out,
                "  {}:{}  {}  {}",
                finding.file,
                finding.line,
                (evaluator.format_metrics)(&finding.values),
                clip(&finding.text)
            );
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
                "  {}:{}  {}, was {}  {}",
                finding.file,
                finding.line,
                (evaluator.format_metrics)(&finding.values),
                (evaluator.format_metrics)(entry),
                clip(&finding.text)
            );
        }
    }
    let _ = writeln!(out, "{}", evaluator.fix_advice);
}

fn text(entry: &Values, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn notes(comparison: &Comparison, evaluator: &Evaluator, out: &mut String) {
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
                    "{}  {}  {}",
                    text(entry, "file"),
                    (evaluator.format_metrics)(entry),
                    clip(&text(entry, "text"))
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

fn collect(comparison: &Comparison, evaluator: &Evaluator, records: &mut Records) {
    let failing = |outcome, finding: &Finding| {
        let mut out = site(outcome, &finding.file, Some(finding.line), &finding.text);
        out.insert("values".into(), Value::Object(finding.values.clone()));
        out.insert("condition".into(), evaluator.condition.into());
        out.insert("fix_advice".into(), evaluator.fix_advice.into());
        Value::Object(out)
    };
    for finding in &comparison.unmatched_findings {
        records.findings.push(failing("new", finding));
    }
    for (finding, _) in &comparison.rose {
        records.findings.push(failing("worsened", finding));
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

fn unmatched_record(entry: &Values) -> Values {
    let mut values = entry.clone();
    for key in ["file", "text", "line", ACCEPTED, "gate"] {
        values.remove(key);
    }
    let mut out = site(
        "unmatched",
        &text(entry, "file"),
        entry.get("line").and_then(Value::as_u64),
        &text(entry, "text"),
    );
    out.insert("values".into(), Value::Object(values));
    out
}
