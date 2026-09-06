use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::{Config, Error};

pub type Values = Map<String, Value>;

pub fn baseline_path(
    config: &Config,
    section_name: &str,
    section: &Values,
) -> Result<PathBuf, Error> {
    let named = section
        .get("baseline")
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(section_name, "baseline"))?;
    let name = named.rsplit(['/', '\\']).next().unwrap_or(named);
    if !name.contains("baseline") || !name.ends_with(".json") {
        return Err(Error(format!(
            "{}: \"{section_name}\" names its baseline {named}, which the guard cannot recognise — \
             the file name must contain \"baseline\" and end with .json",
            config.file.display()
        )));
    }
    Ok(config.path(named))
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

#[derive(Default)]
pub struct Verdict {
    pub new: Vec<Finding>,
    pub worsened: Vec<(Finding, Values)>,
    pub improved: Vec<(Finding, Values)>,
    pub stale: Vec<Values>,
    pub drift: Option<String>,
}

impl Verdict {
    pub fn failed(&self) -> bool {
        !self.new.is_empty() || !self.worsened.is_empty()
    }

    pub fn loose(&self) -> bool {
        !self.stale.is_empty() || !self.improved.is_empty() || self.drift.is_some()
    }
}

pub fn read(path: &Path) -> Result<(Vec<Values>, Option<Values>), Error> {
    if !path.is_file() {
        return Ok((Vec::new(), None));
    }
    let text = std::fs::read_to_string(path).map_err(|why| Error::unreadable(path, why))?;
    let data: Value = serde_json::from_str(&text).map_err(|why| Error::unreadable(path, why))?;
    let shape = || {
        Error(format!(
            "{}: a baseline is {{\"provenance\", \"entries\"}} — this file holds another shape",
            path.display()
        ))
    };
    let Value::Object(mut data) = data else {
        return Err(shape());
    };
    let Some(Value::Array(list)) = data.remove("entries") else {
        return Err(shape());
    };
    let mut entries = Vec::new();
    for item in list {
        let Value::Object(entry) = item else {
            return Err(shape());
        };
        entries.push(entry);
    }
    let provenance = match data.remove("provenance") {
        Some(Value::Object(stored)) => Some(stored),
        _ => None,
    };
    Ok((entries, provenance))
}

pub fn write(path: &Path, findings: &[Finding], provenance: &Values) -> Result<(), Error> {
    let mut data = Values::new();
    data.insert("provenance".into(), Value::Object(provenance.clone()));
    data.insert(
        "entries".into(),
        findings
            .iter()
            .map(|finding| Value::Object(finding.entry()))
            .collect(),
    );
    let unwritable = |why: &dyn std::fmt::Display| {
        Error(format!("{} could not be written: {why}", path.display()))
    };
    let text =
        serde_json::to_string_pretty(&Value::Object(data)).map_err(|why| unwritable(&why))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|why| unwritable(&why))?;
    }
    std::fs::write(path, text + "\n").map_err(|why| unwritable(&why))
}

pub fn provenance(tool: &str, version: &str, config: &Value) -> Values {
    let mut out = Values::new();
    out.insert("tool".into(), tool.into());
    out.insert("version".into(), version.into());
    out.insert("config".into(), config_hash(config).into());
    out
}

fn config_hash(config: &Value) -> String {
    let canonical = config.to_string();
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in canonical.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn drift_between(stored: Option<&Values>, current: Option<&Values>) -> Option<String> {
    let stored = stored?;
    let current = current?;
    let text = |map: &Values, key: &str| map.get(key).and_then(Value::as_str).map(str::to_string);
    let tool = text(current, "tool").unwrap_or_default();
    for key in ["tool", "version", "config"] {
        let (Some(was), Some(now)) = (text(stored, key), text(current, key)) else {
            continue;
        };
        if was == now {
            continue;
        }
        return Some(match key {
            "tool" => format!("the baseline was measured by {was}, this run by {now}"),
            "version" => format!("the baseline was measured by {tool} {was}, this run by {now}"),
            _ => format!(
                "the baseline was written under a different gate configuration ({was}, now {now})"
            ),
        });
    }
    None
}

pub fn restrict(
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
    Worsened,
    Improved,
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
    if comparable.iter().any(|(now, was)| now > was) {
        return Outcome::Worsened;
    }
    if comparable.iter().any(|(now, was)| now < was) {
        return Outcome::Improved;
    }
    Outcome::Held
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
            let distance = entry
                .get("line")
                .and_then(Value::as_u64)
                .map_or(0, |line| line.abs_diff(finding.line));
            candidates.push((-shared, distance, at_finding, at_entry));
        }
    }
    candidates.sort();
    let mut findings: Vec<Option<Finding>> = findings.into_iter().map(Some).collect();
    let mut entries: Vec<Option<Values>> = entries.into_iter().map(Some).collect();
    let mut pairs = Vec::new();
    for (_, _, at_finding, at_entry) in candidates {
        if findings[at_finding].is_none() || entries[at_entry].is_none() {
            continue;
        }
        if let (Some(finding), Some(entry)) =
            (findings[at_finding].take(), entries[at_entry].take())
        {
            pairs.push((finding, entry));
        }
    }
    let unmatched = findings.into_iter().flatten().collect();
    let stale = entries.into_iter().flatten().collect();
    (pairs, unmatched, stale)
}

pub fn judge(
    findings: Vec<Finding>,
    entries: Vec<Values>,
    metrics: &[&str],
    stored: Option<&Values>,
    current: Option<&Values>,
) -> Verdict {
    let mut verdict = Verdict {
        drift: drift_between(stored, current),
        ..Verdict::default()
    };
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
        let (mut pairs, unmatched, stale) = match_group(group_findings, group_entries, metrics);
        pairs.sort_by_key(|(finding, _)| finding.line);
        for (finding, entry) in pairs {
            match compare(&finding, &entry, metrics) {
                Outcome::Worsened => verdict.worsened.push((finding, entry)),
                Outcome::Improved => verdict.improved.push((finding, entry)),
                Outcome::Held => {}
            }
        }
        verdict.new.extend(unmatched);
        verdict.stale.extend(stale);
    }
    verdict
        .new
        .sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    verdict
}

pub struct Gate<'a> {
    pub noun: &'a str,
    pub over: &'a str,
    pub fix: &'a str,
    pub remedy: &'a str,
    pub show: fn(&Values) -> String,
}

pub fn report(
    verdict: &Verdict,
    gate: &Gate,
    baseline_size: usize,
    ok_line: &str,
    quiet: bool,
    strict: bool,
    out: &mut String,
) -> u8 {
    if verdict.failed() {
        failures(verdict, gate, baseline_size, out);
        notes(verdict, gate, false, out);
        return 1;
    }
    if !quiet {
        let _ = writeln!(out, "{ok_line}");
    }
    notes(verdict, gate, true, out);
    if strict && verdict.loose() {
        let _ = writeln!(
            out,
            "FAIL: the baseline is looser than the code — under --strict it must match exactly. \
             Tighten it with the command above and commit the result."
        );
        return 1;
    }
    0
}

fn failures(verdict: &Verdict, gate: &Gate, baseline_size: usize, out: &mut String) {
    if !verdict.new.is_empty() {
        let _ = writeln!(
            out,
            "FAIL: {} new {} {}, beyond the {} the baseline holds:",
            verdict.new.len(),
            gate.noun,
            gate.over,
            baseline_size
        );
        for finding in &verdict.new {
            let _ = writeln!(
                out,
                "  {}:{}  {}  {}",
                finding.file,
                finding.line,
                (gate.show)(&finding.values),
                clip(&finding.text)
            );
        }
    }
    if !verdict.worsened.is_empty() {
        let _ = writeln!(
            out,
            "FAIL: {} baselined {} got worse — the ratchet only tightens:",
            verdict.worsened.len(),
            gate.noun
        );
        for (finding, entry) in &verdict.worsened {
            let _ = writeln!(
                out,
                "  {}:{}  {}, was {}  {}",
                finding.file,
                finding.line,
                (gate.show)(&finding.values),
                (gate.show)(entry),
                clip(&finding.text)
            );
        }
    }
    let _ = writeln!(out, "{}", gate.fix);
}

fn text(entry: &Values, key: &str) -> String {
    entry
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn notes(verdict: &Verdict, gate: &Gate, offer_remedy: bool, out: &mut String) {
    if !verdict.stale.is_empty() {
        let count = verdict.stale.len();
        let plural = if count == 1 { "y" } else { "ies" };
        listed(
            out,
            &format!(
                "NOTE: {count} baseline entr{plural} matched nothing this run — fixed, split, \
                 renamed or deleted:"
            ),
            verdict
                .stale
                .iter()
                .map(|entry| {
                    format!(
                        "{}  {}  {}",
                        text(entry, "file"),
                        (gate.show)(entry),
                        clip(&text(entry, "text"))
                    )
                })
                .collect(),
        );
    }
    if !verdict.improved.is_empty() {
        listed(
            out,
            &format!(
                "NOTE: {} baselined {} improved — the baseline still records the old value:",
                verdict.improved.len(),
                gate.noun
            ),
            verdict
                .improved
                .iter()
                .map(|(finding, entry)| {
                    format!(
                        "{}:{}  {}, baseline says {}  {}",
                        finding.file,
                        finding.line,
                        (gate.show)(&finding.values),
                        (gate.show)(entry),
                        clip(&finding.text)
                    )
                })
                .collect(),
        );
    }
    if let Some(drift) = &verdict.drift {
        let _ = writeln!(out, "NOTE: {drift} — its numbers may not be comparable.");
    }
    if verdict.loose() && offer_remedy {
        let _ = writeln!(
            out,
            "Tighten the baseline (this only ever lowers it): {}",
            gate.remedy
        );
    }
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
