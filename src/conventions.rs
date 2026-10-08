//! The `conventions` check: the project's own rules. A convention names one thing the project
//! forbids — a literal `text`, a `code` pattern, or a `files` glob over paths — where it applies,
//! through `in` and `except`, and the `remedy` to take instead. Each convention is judged as its
//! own gate, `conventions/<name>`, so two conventions on one line are two findings and two debts.
//! A `text` or `code` site is a file and the text of the line a match starts on, and `count`
//! ratchets the matches that site holds. A `files` site is the path, at a count of 1. A site that
//! moves to another file is new, and a file git renamed keeps its sites, as spec 4.4 has it. The
//! scope and a `files` glob read the path each tree holds, so a file renamed out of `except`, or
//! into a glob, is new. The tree is walked once and each file read and parsed once, whatever the
//! number of conventions. The section is a person's policy, and the survey derives none of it.
//! Spec 8.4, ADR 0037.

use serde_json::Value;

use crate::check::contract::{Context, Counted, Line, Plain, Sink, Told};
use crate::check::holes;
use crate::config::Config;
use crate::error::Error;
use crate::key::{self};
use crate::ratchet::{self, Evaluator, Finding, Remedy};
use crate::record::Values;

pub mod report;
pub mod rules;

use rules::{
    CONVENTION, COUNT, Convention, Hole, IN, LANGUAGE, LOGICAL, MATCHER, METRICS, Measured, Place,
    REMEDY, Rule, at_the_base, conventions, holes, measure, resolved, walked,
};

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let config = at.config();
    let conventions = conventions(config)?;
    let places = walked(config, at.project.tree())?;
    let rules = every_rule(config, &conventions, &places)?;
    let mut after = measure(&rules, &places)?;
    let mut before = at_the_base(&rules, at)?;
    let code = every_convention(config, &rules, (&mut after, &mut before), at, out)?;
    let code = holes_said(&holes(&conventions, &places), at, code, out);
    holes::lost_said(&after.files.lost(&before.files, at.project, at.only), out);
    holes::unread_said(&after.unparsed, at, out);
    Ok(code)
}

/// Every convention resolved, or the first one whose language or pattern a person must settle.
fn every_rule<'a>(
    config: &Config,
    conventions: &'a [Convention],
    places: &[Place],
) -> Result<Vec<Rule<'a>>, Error> {
    conventions
        .iter()
        .map(|convention| {
            resolved(convention, places).map_err(|why| {
                Error(format!(
                    "{}: {}",
                    config.file.display(),
                    why.said(&convention.name)
                ))
            })
        })
        .collect()
}

/// Each convention judged in name order, and the one `OK:` line with the coverage of the gate.
fn every_convention(
    config: &Config,
    rules: &[Rule],
    (after, before): (&mut Measured, &mut Measured),
    at: &Context,
    out: &mut Sink,
) -> Result<u8, Error> {
    let said = out.covered(&after.files.coverage(at.only));
    let mut code = 0;
    for rule in rules {
        code = code.max(judged(config, rule, (&mut *after, &mut *before), at, out)?);
    }
    if code == 0 {
        out.tell(Told::judged(Line::new(
            Counted::Conventions(rules.len()),
            said,
        )));
    }
    Ok(code)
}

/// One convention judged as its own gate, against its own sites at the base and its own
/// accepted entries.
fn judged(
    config: &Config,
    rule: &Rule,
    (after, before): (&mut Measured, &mut Measured),
    at: &Context,
    out: &mut Sink,
) -> Result<u8, Error> {
    let name = &rule.convention.name;
    let gate = key::entry_gate(at.gate, name);
    let now = described(rule, &gate, after.take(name));
    let sites = ratchet::scoped(&now, at.only);
    let condition = format!("the convention {name} forbids");
    let evaluator = Evaluator {
        metrics: METRICS,
        unit: "site(s)",
        condition: &condition,
        fix_advice: Remedy::Fixed(&rule.convention.remedy),
        ceiling: None,
        format_metrics: show,
        nested: None,
    };
    Ok(evaluator.evaluate(
        now,
        described(rule, &gate, before.take(name)),
        ratchet::accepted(config, &gate, METRICS)?,
        &Context { gate: &gate, ..*at },
        Line::new(
            Counted::Convention {
                name: name.clone(),
                sites,
            },
            None,
        ),
        out,
    ))
}

fn show(values: &Values) -> String {
    let count = values
        .get(COUNT)
        .and_then(Value::as_u64)
        .unwrap_or_default();
    match values.get(CONVENTION).and_then(Value::as_str) {
        Some(name) => format!("{name} x{count}"),
        None => format!("x{count}"),
    }
}

/// The findings with what a report and the JSON say about the rule behind them. None of these
/// values is ratcheted.
fn described(rule: &Rule, gate: &str, mut found: Vec<Finding>) -> Vec<Finding> {
    for finding in &mut found {
        let values = &mut finding.values;
        values.insert(CONVENTION.into(), rule.convention.name.clone().into());
        values.insert(LOGICAL.into(), gate.into());
        values.insert(MATCHER.into(), rule.convention.kind().into());
        if let Some(code) = &rule.code {
            values.insert(LANGUAGE.name.into(), code.language.into());
        }
        values.insert(REMEDY.name.into(), rule.convention.remedy.clone().into());
    }
    found
}

/// What a gate does about its holes. An `except` path that names nothing takes nothing out, so it
/// is a NOTE. An `in` path that names nothing leaves its convention measuring nothing there, so it
/// is exit 2, and a NOTE in the hook, where the agent cannot edit the configuration. Spec 8.4.
fn holes_said(holes: &[Hole], at: &Context, code: u8, out: &mut Sink) -> u8 {
    let (refused, noted): (Vec<&Hole>, Vec<&Hole>) = holes
        .iter()
        .partition(|hole| hole.key == IN.name && !at.hook());
    let noted: Vec<(String, String)> = noted
        .iter()
        .map(|hole| (String::new(), hole.named()))
        .collect();
    ratchet::noted(&noted, out);
    for hole in &refused {
        out.tell(Plain::PathMissing(hole.named()));
        out.error(hole.named());
    }
    match refused.is_empty() {
        true => code,
        false => 2,
    }
}
