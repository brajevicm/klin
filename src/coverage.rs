use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

use serde_json::{Map, Value};

use crate::check::{self, Context, Sink};
use crate::project::Project;
use crate::syntax::structural::Unsupported;

/// The scope one gate measured, said on its `OK:` line and carried in the JSON under
/// `coverage`. The boundary is the same for every check, and this is where it is written down:
/// `found` counts every file the check's own discovery rule reached under its roots, before
/// anything dropped one; `excluded` counts the ones an exclusion dropped; `unreadable` counts
/// the ones it reached and could not read or parse; `not_measured` counts known-language files
/// with no structural adapter; `measured` counts the ones it judged. A scoped run counts only
/// the files in its scope. So a green run over a scope smaller than a reader expected says so on
/// that one line. Spec 8.6, 11.1, 11.2.
///
/// A check whose scope is not a set of files counts the thing it discovers: a document, a
/// manifest, a test file the base holds. Every count is of the same thing in one gate, and the
/// check's module docstring names it.
#[derive(Default, Clone, Copy)]
pub struct Coverage {
    pub found: usize,
    pub measured: usize,
    pub not_measured: usize,
    pub excluded: usize,
    pub unreadable: usize,
}

/// How many of these files a scoped run counts, which is every one of them outside a scoped
/// run. The same rule the findings of one gate are restricted by. Spec 8.6.
pub fn scoped(files: &[String], only: Option<&[String]>) -> usize {
    match only {
        Some(only) => files.iter().filter(|file| only.contains(file)).count(),
        None => files.len(),
    }
}

impl Coverage {
    /// What the whole scope was measured, which is the common case: nothing excluded and
    /// nothing unread.
    pub fn whole(measured: usize) -> Coverage {
        Coverage {
            found: measured,
            measured,
            not_measured: 0,
            excluded: 0,
            unreadable: 0,
        }
    }

    /// What every `OK:` line adds after what the gate judged, recorded for `--json` on the way
    /// past so one call per check carries both. Spec 8.6.
    pub fn said(&self, out: &mut Sink) -> String {
        out.record(|records| records.coverage = Some(self.record()));
        match self.not_measured {
            0 => format!(
                " ({} file(s) found, {} measured, {} excluded, {} unreadable)",
                self.found, self.measured, self.excluded, self.unreadable
            ),
            not_measured => format!(
                " ({} file(s) found, {} measured, {} not measured, {} excluded, {} unreadable)",
                self.found, self.measured, not_measured, self.excluded, self.unreadable
            ),
        }
    }

    fn record(&self) -> Value {
        let mut out = Map::new();
        out.insert("found".into(), self.found.into());
        out.insert("measured".into(), self.measured.into());
        out.insert("not_measured".into(), self.not_measured.into());
        out.insert("excluded".into(), self.excluded.into());
        out.insert("unreadable".into(), self.unreadable.into());
        Value::Object(out)
    }
}

/// The files one walk reached, sorted the way the coverage counts them: the ones the check
/// judged, the ones with no structural adapter, the ones an exclusion dropped, and the ones it
/// reached and could not read.
#[derive(Default)]
pub struct Files {
    pub measured: Vec<String>,
    pub not_measured: Vec<String>,
    pub excluded: Vec<String>,
    pub unreadable: Vec<String>,
}

impl Files {
    pub fn coverage(&self, only: Option<&[String]>) -> Coverage {
        let measured = scoped(&self.measured, only);
        let not_measured = scoped(&self.not_measured, only);
        let excluded = scoped(&self.excluded, only);
        let unreadable = scoped(&self.unreadable, only);
        Coverage {
            found: measured + not_measured + excluded + unreadable,
            measured,
            not_measured,
            excluded,
            unreadable,
        }
    }

    /// Every file `before` measured that this tree still holds and did not measure, with the
    /// reason this tree gives: an exclusion, a grammar that refused it, no structural adapter,
    /// or no discovery rule left that reaches it. Roots are the union over both trees, so a check
    /// can only discover
    /// more, and a file that left scrutiny this way left through one of those three. The run
    /// reads one configuration, so only `after` can say why. A file under a root the
    /// derivation commit's survey did not hold matches nothing in `before` (7.1), so it is not
    /// lost either. Spec 8.6.
    pub fn lost(&self, before: &Files, project: &Project, only: Option<&[String]>) -> Vec<Lost> {
        let measured: BTreeSet<&String> = self.measured.iter().collect();
        before
            .measured
            .iter()
            .filter(|file| only.is_none_or(|only| only.contains(file)))
            .filter(|file| !measured.contains(file) && project.was_held(file))
            .filter(|file| project.root().join(file).is_file())
            .map(|file| Lost {
                file: file.clone(),
                why: if self.unreadable.contains(file) {
                    "the grammar refused it"
                } else if self.excluded.contains(file) {
                    "an exclusion drops it now"
                } else if self.not_measured.contains(file) {
                    "no structural adapter measures its language"
                } else {
                    "no discovery rule places it under a root now"
                },
            })
            .collect()
    }
}

pub struct Lost {
    pub file: String,
    pub why: &'static str,
}

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
            record.insert("outcome".into(), check::LOST.into());
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
        "FAIL: {} file(s) left scrutiny — under --strict a file klin measured at the base and \
         does not measure now, though it is still in the tree, is a failure. {LOST_REMEDY}",
        lost.len()
    );
    2
}

/// Whether a scoped run judges this file, which is every file outside a scoped run.
pub fn in_scope(file: &str, only: Option<&[String]>) -> bool {
    only.is_none_or(|only| only.iter().any(|wanted| wanted == file))
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
            record.insert("outcome".into(), check::NOT_MEASURED.into());
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

/// One form a gate supports and could not resolve, and why.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Unresolved {
    pub file: String,
    pub line: u64,
    pub text: String,
    pub why: String,
}

impl Unresolved {
    /// What pairs a form with one the base holds: its file, text and reason, at any line.
    fn key(&self) -> (&str, &str, &str) {
        (&self.file, &self.text, &self.why)
    }
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
                    "outcome": check::UNRESOLVED,
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

/// Whether each form now pairs with a form the base holds in the same file with the same text and
/// reason, at any line. Each base form pairs once, so a second copy of a held form is new.
fn held_at(now: &[Unresolved], base: &[Unresolved]) -> Vec<bool> {
    let mut left: HashMap<(&str, &str, &str), usize> = HashMap::new();
    for was in base {
        *left.entry(was.key()).or_default() += 1;
    }
    now.iter()
        .map(|hole| match left.get_mut(&hole.key()) {
            Some(count) if *count > 0 => {
                *count -= 1;
                true
            }
            _ => false,
        })
        .collect()
}

/// Whether a note records a file the run could not read or stopped measuring, which the hook
/// prints even when nothing blocks the stop.
pub fn is_lost(note: &Value) -> bool {
    note.get("outcome").and_then(Value::as_str) == Some(check::LOST)
}
