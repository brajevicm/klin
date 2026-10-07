//! What a gate says about the holes in what it measured: files that left its scrutiny, files in a
//! language no adapter reads, forms no resolver proved, and files no grammar read. A file that
//! left scrutiny is a NOTE, and exit 2 only under `--strict`. Each other hole is a NOTE where the
//! change opened no hole or the agent cannot close it, and exit 2 elsewhere, so a green run never
//! implies a measurement klin did not make. Spec 8.6, 10.

use std::fmt::Write;

use serde_json::{Map, Value};

use crate::check::contract::{Context, LOST, NOT_MEASURED, Records, Sink, UNPARSED, UNRESOLVED};
use crate::coverage::{Lost, Unresolved, held_at, in_scope};
use crate::syntax::structural::facts::Unsupported;
use crate::syntax::{self, Unparsed};

const LOST_REMEDY: &str = "Drop the exclusion or restore the rule that reached it, or exclude it \
                           on purpose and accept that nothing measures it.";

/// What a gate says about the files that left its scrutiny: a NOTE per file for a person and a
/// `lost` record under its notes for `--json`. Under `--strict` the loss is exit 2, beside the
/// other strict failures of spec 10. In the hook and without either flag the code stands.
pub fn lost_said(lost: &[Lost], at: &Context, code: u8, out: &mut Sink) -> u8 {
    if lost.is_empty() {
        return code;
    }
    for file in lost {
        let _ = writeln!(
            out.text,
            "NOTE: {} was measured at the base and is not measured now — {}",
            file.file, file.why
        );
    }
    out.record(|records| {
        for file in lost {
            let mut record = Map::new();
            record.insert("outcome".into(), LOST.into());
            record.insert("file".into(), file.file.clone().into());
            record.insert("text".into(), file.why.into());
            records.notes.push(Value::Object(record));
        }
    });
    if !at.strict {
        return code;
    }
    let _ = writeln!(
        out.text,
        "FAIL: {} file(s) left scrutiny — a file klin measured at the base and \
         does not measure now, though it is still in the tree, is a failure. {LOST_REMEDY}",
        lost.len()
    );
    2
}

/// Whether a note records a file the run could not read or stopped measuring, which the hook
/// prints even when nothing blocks the stop.
pub fn is_lost(note: &Value) -> bool {
    note.get("outcome").and_then(Value::as_str) == Some(LOST)
}

/// What a structural gate says about the files in a language no adapter measures: a FAIL
/// outside the hook, because a green run must not imply they were analyzed, and a NOTE in it,
/// because the agent cannot add an adapter. Spec 8.4, 8.6.
pub fn not_measured_said(files: &[Unsupported], at: &Context, code: u8, out: &mut Sink) -> u8 {
    let files: Vec<&Unsupported> = files
        .iter()
        .filter(|file| in_scope(&file.file, at.only))
        .collect();
    if files.is_empty() {
        return code;
    }
    let word = if at.hook() { "NOTE" } else { "FAIL" };
    let _ = writeln!(
        out.text,
        "{word}: {} file(s) in unsupported structural languages were not measured:",
        files.len()
    );
    for file in &files {
        let _ = writeln!(out.text, "  {}  {}", file.file, file.language);
    }
    let _ = writeln!(
        out.text,
        "Add a structural adapter for the language, or exclude the file and accept that nothing measures it."
    );
    out.record(|records| {
        for file in &files {
            let mut record = Map::new();
            record.insert("outcome".into(), NOT_MEASURED.into());
            record.insert("file".into(), file.file.clone().into());
            record.insert(
                "text".into(),
                format!("{} has no structural adapter", file.language).into(),
            );
            if at.hook() {
                records.notes.push(Value::Object(record));
            } else {
                records.findings.push(Value::Object(record));
            }
        }
    });
    if at.hook() { code } else { 2 }
}

/// What a gate says about the forms it supports and could not resolve: a NOTE in the hook, and
/// exit 2 elsewhere, because a green run must not imply a resolution klin did not make. A form
/// the base holds in the same file, with the same text and reason, is a NOTE in every run,
/// because the change opened no hole there. Each base form pairs with one form now, so a second
/// copy of a held form is new. `base` is built only outside the hook, where the answer decides
/// something. `what` follows the count on the first line, and
/// `remedy` closes each block. ADR 0021, spec 8.6.
pub fn unresolved_said(
    (now, base): (&[Unresolved], impl FnOnce() -> Vec<Unresolved>),
    (what, remedy): (&str, &str),
    (at, code): (&Context, u8),
    out: &mut Sink,
) -> u8 {
    if now.is_empty() {
        return code;
    }
    let held = match at.hook() {
        true => vec![true; now.len()],
        false => held_at(now, &base()),
    };
    let (noted, refused): (Vec<_>, Vec<_>) = now.iter().zip(held).partition(|(_, held)| *held);
    for (word, named) in [("NOTE", &noted), ("FAIL", &refused)] {
        listed(word, named, (what, remedy), out);
    }
    out.record(|records| {
        for (into, named) in [
            (&mut records.notes, &noted),
            (&mut records.findings, &refused),
        ] {
            into.extend(named.iter().map(|(hole, _)| {
                serde_json::json!({
                    "outcome": UNRESOLVED,
                    "file": hole.file,
                    "line": hole.line,
                    "text": format!("{} — {}", hole.text, hole.why),
                })
            }));
        }
    });
    match refused.is_empty() {
        true => code,
        false => 2,
    }
}

/// One block of forms under one word, as the report prints it, and nothing for no form.
fn listed(word: &str, named: &[(&Unresolved, bool)], (what, remedy): (&str, &str), out: &mut Sink) {
    if named.is_empty() {
        return;
    }
    let _ = writeln!(out.text, "{word}: {} {what}:", named.len());
    for (hole, _) in named {
        let _ = writeln!(
            out.text,
            "  {}:{}  {}  — {}",
            hole.file, hole.line, hole.text, hole.why
        );
    }
    let _ = writeln!(out.text, "{remedy}");
}

/// What a gate does about the files no grammar read: a NOTE in the hook, and exit 2 outside
/// it, because an agent cannot fix a grammar and a file klin cannot read is a hole in the
/// ratchet. A file the base held and could not read either is a NOTE in every run, because the
/// change opened no hole there. `base` names those files under today's paths, and is asked only
/// outside the hook, when a file in scope needs it. ADR 0003, ADR 0021, spec 8.6, 14.
pub fn unread_said(
    unparsed: &[Unparsed],
    base: impl FnOnce() -> Vec<String>,
    at: &Context,
    code: u8,
    out: &mut Sink,
) -> u8 {
    let rejected = syntax::rejected(unparsed, at.only, (!at.hook()).then_some(base));
    unparsed_said("NOTE", &rejected.held, out, |records| &mut records.notes);
    unparsed_said("FAIL", &rejected.new, out, |records| &mut records.findings);
    match rejected.new.is_empty() {
        true => code,
        false => 2,
    }
}

/// One block of unparsed files under one word, each recorded where `into` puts it.
fn unparsed_said(
    word: &str,
    named: &[&Unparsed],
    out: &mut Sink,
    into: fn(&mut Records) -> &mut Vec<Value>,
) {
    if named.is_empty() {
        return;
    }
    let _ = writeln!(
        out.text,
        "{word}: {} file(s) the grammar could not parse, so nothing in them was measured:",
        named.len()
    );
    for file in named {
        let rejected = format!("the {} grammar rejected it", file.language);
        let _ = writeln!(out.text, "  {}  {rejected}", file.file);
        out.record(|records| into(records).push(unparsed_site(file, &rejected)));
    }
    let _ = writeln!(out.text, "{UNPARSED_REMEDY}");
}

const UNPARSED_REMEDY: &str = "A file klin cannot read is a hole in the ratchet. Update the \
                               grammar, or exclude the file and accept that nothing measures it.";

fn unparsed_site(file: &Unparsed, rejected: &str) -> Value {
    let mut out = Map::new();
    out.insert("outcome".into(), UNPARSED.into());
    out.insert("file".into(), file.file.clone().into());
    out.insert("text".into(), rejected.into());
    Value::Object(out)
}
