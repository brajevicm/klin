use std::collections::{BTreeSet, HashMap};

use crate::contract::project::Project;

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
            .filter(|file| still_held(project, file))
            .map(|file| Lost {
                file: file.clone(),
                why: if self.unreadable.contains(file) {
                    Left::Refused
                } else if self.excluded.contains(file) {
                    Left::Excluded
                } else if self.not_measured.contains(file) {
                    Left::Unattached
                } else {
                    Left::Undiscovered
                },
            })
            .collect()
    }
}

/// Whether the working tree still holds this exact path: in its file list, or as a symbolic
/// link. The list names each file as the directory holds it,
/// so a path a case-only rename left behind is not held on a file system that ignores case,
/// where asking for the path itself finds the renamed file. Spec 7.2.
fn still_held(project: &Project, file: &str) -> bool {
    let tree = project.tree();
    tree.files().is_ok_and(|files| {
        files
            .binary_search_by(|held| held.as_str().cmp(file))
            .is_ok()
    }) || std::fs::symlink_metadata(project.root().join(file))
        .is_ok_and(|held| held.file_type().is_symlink())
}

pub struct Lost {
    pub file: String,
    pub why: Left,
}

/// Why a file the base measured is not measured now. Only `Excluded` is a person's decision:
/// the gates measure the base under the base's own `klin.json` scope and the working tree under
/// today's, so a file today's `in` or `except` drops is one a reviewed commit dropped. Every
/// other reason comes from the facts, such as a manifest. Spec 7.2.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Left {
    Refused,
    Excluded,
    Unattached,
    Undiscovered,
}

impl Left {
    pub fn text(self) -> &'static str {
        match self {
            Left::Refused => "the grammar refused it",
            Left::Excluded => "an exclusion drops it now",
            Left::Unattached => "no structural adapter or module reads it now",
            Left::Undiscovered => "no discovery rule places it under a root now",
        }
    }
}

/// Whether a scoped run judges this file, which is every file outside a scoped run.
pub fn in_scope(file: &str, only: Option<&[String]>) -> bool {
    only.is_none_or(|only| only.iter().any(|wanted| wanted == file))
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

/// Whether each form now pairs with a form the base holds in the same file with the same text and
/// reason, at any line. Each base form pairs once, so a second copy of a held form is new.
pub fn held_at(now: &[Unresolved], base: &[Unresolved]) -> Vec<bool> {
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
