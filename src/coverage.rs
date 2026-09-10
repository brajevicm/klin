use serde_json::{Map, Value};

use crate::config::Flags;

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
