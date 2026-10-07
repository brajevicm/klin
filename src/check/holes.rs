//! What a gate says about the holes in what it measured: files that left its scrutiny, files in a
//! language no adapter reads, forms no resolver proved, and files no grammar read. A file that
//! left scrutiny is a NOTE, and exit 2 only under `--strict`. Each other hole is a NOTE where the
//! change opened no hole or the agent cannot close it, and exit 2 elsewhere, so a green run never
//! implies a measurement klin did not make. Spec 8.6, 10.

use serde_json::Value;

use crate::check::contract::{Context, Hole, LOST, Located, Sink, Site, Unresolvable};
use crate::coverage::{Lost, Unresolved, held_at, in_scope};
use crate::syntax::structural::facts::Unsupported;
use crate::syntax::{self, Unparsed};

/// What a gate says about the files that left its scrutiny: a NOTE per file, which `--json`
/// records as a `lost` note. Under `--strict` the loss is exit 2, beside the
/// other strict failures of spec 10. In the hook and without either flag the code stands.
pub fn lost_said(lost: &[Lost], at: &Context, code: u8, out: &mut Sink) -> u8 {
    if lost.is_empty() {
        return code;
    }
    for file in lost {
        out.tell(Hole::Lost(Site {
            file: file.file.clone(),
            text: file.why.to_string(),
        }));
    }
    if !at.strict {
        return code;
    }
    out.tell(Hole::LeftScrutiny(lost.len()));
    out.error(format!("{} file(s) left scrutiny", lost.len()));
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
    out.tell(Hole::NotMeasured {
        fail: !at.hook(),
        files: files
            .iter()
            .map(|file| Site {
                file: file.file.clone(),
                text: file.language.to_string(),
            })
            .collect(),
    });
    if at.hook() { code } else { 2 }
}

/// What a gate says about the forms it supports and could not resolve: a NOTE in the hook, and
/// exit 2 elsewhere, because a green run must not imply a resolution klin did not make. A form
/// the base holds in the same file, with the same text and reason, is a NOTE in every run,
/// because the change opened no hole there. Each base form pairs with one form now, so a second
/// copy of a held form is new. `base` is built only outside the hook, where the answer decides
/// something. `kind` names what the forms are, which the renderer words. ADR 0021, spec 8.6.
pub fn unresolved_said(
    (now, base): (&[Unresolved], impl FnOnce() -> Vec<Unresolved>),
    kind: Unresolvable,
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
    for (fail, named) in [(false, &noted), (true, &refused)] {
        listed(fail, named, kind, out);
    }
    match refused.is_empty() {
        true => code,
        false => 2,
    }
}

/// One block of forms under one word, as the report prints it, and nothing for no form.
fn listed(fail: bool, named: &[(&Unresolved, bool)], kind: Unresolvable, out: &mut Sink) {
    if named.is_empty() {
        return;
    }
    out.tell(Hole::Unresolved {
        fail,
        kind,
        forms: named
            .iter()
            .map(|(hole, _)| {
                let form = Located {
                    file: hole.file.clone(),
                    line: hole.line,
                    text: hole.text.clone(),
                };
                (form, hole.why.to_string())
            })
            .collect(),
    });
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
    unparsed_said(false, &rejected.held, out);
    unparsed_said(true, &rejected.new, out);
    match rejected.new.is_empty() {
        true => code,
        false => 2,
    }
}

/// One block of unparsed files under one word.
fn unparsed_said(fail: bool, named: &[&Unparsed], out: &mut Sink) {
    if named.is_empty() {
        return;
    }
    let files = named
        .iter()
        .map(|file| Site {
            file: file.file.clone(),
            text: format!("the {} grammar rejected it", file.language),
        })
        .collect();
    out.tell(Hole::Unparsed { fail, files });
}
