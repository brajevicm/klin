use std::collections::BTreeSet;
use std::fmt::Write;

use serde_json::{Map, Value};

use crate::config::{Config, Flags};

/// The scope one gate measured, said on its `OK:` line and carried in the JSON under
/// `coverage`. The boundary is the same for every check, and this is where it is written down:
/// `found` counts every file the check's own discovery rule reached under its roots, before
/// anything dropped one; `excluded` counts the ones an exclusion dropped; `unreadable` counts
/// the ones it reached and could not read or parse; `measured` counts the ones it judged. A
/// scoped run counts only the files in its scope. So a green run over a scope smaller than a
/// reader expected says so on that one line. Spec 8.6, 11.1, 11.2.
///
/// A check whose scope is not a set of files counts the thing it discovers: a document, a
/// manifest, a test file the base holds. Every count is of the same thing in one gate, and the
/// check's module docstring names it.
#[derive(Default, Clone, Copy)]
pub struct Coverage {
    pub found: usize,
    pub measured: usize,
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
            excluded: 0,
            unreadable: 0,
        }
    }

    /// What every `OK:` line adds after what the gate judged, recorded for `--json` on the way
    /// past so one call per check carries both. Spec 8.6.
    pub fn said(&self, flags: &Flags) -> String {
        flags.record(|records| records.coverage = Some(self.record()));
        format!(
            " ({} file(s) found, {} measured, {} excluded, {} unreadable)",
            self.found, self.measured, self.excluded, self.unreadable
        )
    }

    fn record(&self) -> Value {
        let mut out = Map::new();
        out.insert("found".into(), self.found.into());
        out.insert("measured".into(), self.measured.into());
        out.insert("excluded".into(), self.excluded.into());
        out.insert("unreadable".into(), self.unreadable.into());
        Value::Object(out)
    }
}

/// The files one walk reached, sorted the way the coverage counts them: the ones the check
/// judged, the ones an exclusion dropped, and the ones it reached and could not read.
#[derive(Default)]
pub struct Files {
    pub measured: Vec<String>,
    pub excluded: Vec<String>,
    pub unreadable: Vec<String>,
}

impl Files {
    pub fn coverage(&self, only: Option<&[String]>) -> Coverage {
        let measured = scoped(&self.measured, only);
        let excluded = scoped(&self.excluded, only);
        let unreadable = scoped(&self.unreadable, only);
        Coverage {
            found: measured + excluded + unreadable,
            measured,
            excluded,
            unreadable,
        }
    }

    /// Every file `before` measured that this tree still holds and did not measure, with the
    /// reason this tree gives: an exclusion, a grammar that refused it, or no discovery rule
    /// left that reaches it. Roots are the union over both trees, so a check can only discover
    /// more, and a file that left scrutiny this way left through one of those three. The run
    /// reads one configuration, so only `after` can say why. A file under a root the
    /// derivation commit's survey did not hold matches nothing in `before` (7.1), so it is not
    /// lost either. Spec 8.6.
    pub fn lost(&self, before: &Files, config: &Config, only: Option<&[String]>) -> Vec<Lost> {
        let measured: BTreeSet<&String> = self.measured.iter().collect();
        before
            .measured
            .iter()
            .filter(|file| only.is_none_or(|only| only.contains(file)))
            .filter(|file| !measured.contains(file) && config.was_held(file))
            .filter(|file| config.root().join(file).is_file())
            .map(|file| Lost {
                file: file.clone(),
                why: if self.unreadable.contains(file) {
                    "the grammar refused it"
                } else if self.excluded.contains(file) {
                    "an exclusion drops it now"
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

const LOST: &str = "lost";

const LOST_REMEDY: &str = "Drop the exclusion or restore the rule that reached it, or exclude it \
                           on purpose and accept that nothing measures it.";

/// What a gate says about the files that left its scrutiny: a NOTE per file for a person and a
/// `lost` record under its notes for `--json`. Under `--strict` the loss is exit 2, beside the
/// other strict failures of spec 10. In the hook and without either flag the code stands.
pub fn lost_said(lost: &[Lost], flags: &Flags, code: u8, out: &mut String) -> u8 {
    if lost.is_empty() {
        return code;
    }
    for file in lost {
        let _ = writeln!(
            out,
            "NOTE: {} was measured at the base and is not measured now — {}",
            file.file, file.why
        );
    }
    flags.record(|records| {
        for file in lost {
            let mut record = Map::new();
            record.insert("outcome".into(), LOST.into());
            record.insert("file".into(), file.file.clone().into());
            record.insert("text".into(), file.why.into());
            records.notes.push(Value::Object(record));
        }
    });
    if !flags.strict {
        return code;
    }
    let _ = writeln!(
        out,
        "FAIL: {} file(s) left scrutiny — under --strict a file klin measured at the base and \
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
