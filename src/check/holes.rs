//! What klin could not measure, and what the change did to it. A gate names the files it could
//! not read and the files that left its scope, and nothing more. The runner sorts every file once
//! for the whole run against the base: a `measurement-lost` FAIL where the base measured the file
//! and the change made it unmeasurable, an opened gap where the change made it unmeasurable with
//! no clear agent cause, and a coverage note for klin's own limit the change did not open.
//! ADR 0003, ADR 0021, spec 7.2.

use std::collections::BTreeMap;
use std::path::Path;

use crate::changed::{self, Change};
use crate::check::contract::{Class, Context, Hole, Located, Sink, Site, Unresolvable};
use crate::coverage::{Lost, Unresolved, held_at, in_scope};
use crate::files::{self, Form};
use crate::project::Project;
use crate::syntax::{self, Refusal, Unparsed};

pub use crate::config::MEASUREMENT_LOST;

/// The kind of review item an opened gap is. Spec 7.2, 11.7.
pub const UNMEASURED: &str = "unmeasured";

/// The identity of the finding of a lost file: the hash of the built-in row's name and the file,
/// the same whatever gates a run selects. Spec 7.2.
pub fn lost_id(file: &str) -> String {
    crate::ratchet::site_id(MEASUREMENT_LOST, file, "")
}

/// The files the accepted list holds for every capability while klin cannot measure them: the
/// entries of gate `measurement-lost`, by file alone. Spec 7.2.
pub fn held_files(config: &crate::config::Config) -> Vec<String> {
    config
        .pinned(crate::config::ACCEPTED.name)
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|entry| {
            entry.get("gate").and_then(serde_json::Value::as_str) == Some(MEASUREMENT_LOST)
        })
        .filter_map(|entry| entry.get("file")?.as_str().map(str::to_string))
        .collect()
}

/// What a gate says about the files that left its scrutiny: measured at the base, still in the
/// tree, and not measured now. The runner decides what each one is. Spec 7.2.
pub fn lost_said(lost: &[Lost], out: &mut Sink) {
    for file in lost {
        out.tell(Hole::Lost(Site {
            file: file.file.clone(),
            text: file.why.to_string(),
        }));
    }
}

/// What a gate says about the forms it supports and could not resolve. A form the base holds in
/// the same file, with the same text and reason, is klin's own limit, because the change opened
/// no hole there. Each base form pairs with one form now, so a second copy of a held form is
/// opened. `base` is built only outside the hook, where the answer decides something; at the
/// Stop every form is a note. `kind` names what the forms are, which the renderer words. ADR
/// 0021, spec 7.2.
pub fn unresolved_said(
    (now, base): (&[Unresolved], impl FnOnce() -> Vec<Unresolved>),
    kind: Unresolvable,
    at: &Context,
    out: &mut Sink,
) {
    if now.is_empty() {
        return;
    }
    let held = match at.hook() {
        true => vec![true; now.len()],
        false => held_at(now, &base()),
    };
    let (noted, opened): (Vec<_>, Vec<_>) = now.iter().zip(held).partition(|(_, held)| *held);
    for (opened, named) in [(false, &noted), (true, &opened)] {
        listed(opened, named, kind, out);
    }
}

/// One block of forms under one word, as the report prints it, and nothing for no form.
fn listed(opened: bool, named: &[(&Unresolved, bool)], kind: Unresolvable, out: &mut Sink) {
    if named.is_empty() {
        return;
    }
    out.tell(Hole::Unresolved {
        opened,
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

/// The gap reason of a form a resolver could not resolve: `ambiguous` where two files, two
/// exports or two declarations answer it or a re-export cycles, and `unresolved` otherwise.
/// Spec 7.2.
pub fn form_reason(why: &str) -> &'static str {
    // ponytail: the resolvers word their holes in free text, so this reads the words; carry a
    // typed reason on each resolver hole if another wording appears.
    let ambiguous = [
        "names more than one",
        "and by the one at line",
        "and by the glob at",
        "lie across",
        "cyclic",
        "modules declare a type of that name",
    ];
    match ambiguous.iter().any(|said| why.contains(said)) {
        true => "ambiguous",
        false => "unresolved",
    }
}

/// What a gate says about the files in its scope that no grammar read. The runner decides what
/// each one is against the base. Spec 7.2.
pub fn unread_said(unparsed: &[Unparsed], at: &Context, out: &mut Sink) {
    let files: Vec<Site> = unparsed
        .iter()
        .filter(|file| in_scope(&file.file, at.only))
        .map(|file| Site {
            file: file.file.clone(),
            text: file.language.to_string(),
        })
        .collect();
    if !files.is_empty() {
        out.tell(Hole::Unparsed(files));
    }
}

/// One file a gate could not measure, as that gate saw it.
pub enum Seen {
    /// No grammar read it.
    Unread,
    /// It left the gate's scope, for the reason the gate gives.
    Left(String),
    /// A manifest no parser read, which the gate that reads manifests already classed.
    Manifest { class: Class, why: String },
    /// A file the working tree's `.gitattributes` make not text, which no gate read.
    Form(Form),
}

/// One file klin could not measure, sorted once for the run: its class, the reason its class
/// names it by, the line and column of a parse's first error node, the words a person reads, and
/// every gate that reported it. Spec 7.2.
pub struct Unmeasured {
    pub file: String,
    pub language: Option<&'static str>,
    pub class: Class,
    pub reason: &'static str,
    pub at: Option<(u64, u64)>,
    pub text: String,
    pub gates: Vec<String>,
}

/// Every file the gates of one run reported, each sorted once against the base, in path order.
/// `base` is the commit the run compares against, and `None` where it has none, which leaves
/// every file klin's own limit. Spec 7.2.
pub fn sorted(
    project: &Project,
    base: Option<&str>,
    reported: Vec<(String, String, Seen)>,
) -> Vec<Unmeasured> {
    let mut by_file: BTreeMap<String, Vec<(String, Seen)>> = BTreeMap::new();
    for (gate, file, seen) in reported {
        by_file.entry(file).or_default().push((gate, seen));
    }
    let changes = base.and_then(|base| project.changes(base).ok());
    let against = changes.as_deref().zip(base);
    by_file
        .into_iter()
        .map(|(file, seen)| {
            let mut gates: Vec<String> = seen.iter().map(|(gate, _)| gate.clone()).collect();
            gates.dedup();
            let (class, reason, at, text) = one(project, against, &file, &seen);
            Unmeasured {
                language: syntax::language_of(&file).map(|language| language.name),
                file,
                class,
                reason,
                at,
                text,
                gates,
            }
        })
        .collect()
}

type Sorted = (Class, &'static str, Option<(u64, u64)>, String);

/// One file, by the strongest thing a gate saw: a read that failed, then a manifest, then a
/// file that left a scope.
fn one(
    project: &Project,
    against: Option<(&[Change], &str)>,
    file: &str,
    seen: &[(String, Seen)],
) -> Sorted {
    let strongest = seen
        .iter()
        .map(|(_, seen)| seen)
        .min_by_key(|seen| seen.rank());
    match strongest {
        Some(Seen::Form(form)) => formed(project.root(), against, file, *form),
        Some(Seen::Unread) => unread(project, against, file),
        Some(Seen::Manifest { class, why }) => manifest(*class, why),
        Some(Seen::Left(why)) => left(project, against, file, why),
        None => left(project, against, file, ""),
    }
}

impl Seen {
    /// Which of two things gates saw of one file names it: its form, then a read that failed,
    /// then a manifest, then a scope it left.
    fn rank(&self) -> u8 {
        match self {
            Seen::Form(_) => 0,
            Seen::Unread => 1,
            Seen::Manifest { .. } => 2,
            Seen::Left(_) => 3,
        }
    }
}

/// A manifest or lockfile the gate that reads it already classed against the base.
fn manifest(class: Class, why: &str) -> Sorted {
    let reason = match class {
        Class::Lost => "manifest",
        Class::Opened | Class::Limit => "unreadable",
    };
    (class, reason, None, why.to_string())
}

/// A file the working tree's `.gitattributes` make not text: klin's own limit where the base's
/// gave its path the same form, lost where the base measured the path as text and the change
/// gave it `binary`, `-diff` or a `filter`, and opened for a new path or a new encoding klin
/// cannot decode. Spec 7.2.
fn formed(root: &Path, against: Option<(&[Change], &str)>, file: &str, form: Form) -> Sorted {
    let (reason, said) = match (form.binary, form.filter) {
        (true, _) => ("not-text", "its attributes make it binary"),
        (false, true) => ("filtered", "its attributes run a filter klin does not run"),
        (false, false) => (
            "filtered",
            "its attributes name an encoding klin does not decode",
        ),
    };
    let class = against.map_or(Class::Limit, |against| {
        form_against(root, against, file, form)
    });
    let reason = match class {
        Class::Lost => "form",
        Class::Opened | Class::Limit => reason,
    };
    (class, reason, None, said.to_string())
}

/// What the base's own `.gitattributes` gave the path the base held this file at.
fn form_against(root: &Path, (changes, base): (&[Change], &str), file: &str, form: Form) -> Class {
    let was = match changes.iter().find(|change| change.path == file) {
        Some(change) => change.was.as_deref(),
        None => Some(file),
    };
    match was.map(|was| files::form_at(root, base, was)) {
        None => Class::Opened,
        Some(held) if held == form => Class::Limit,
        Some(_) if form.binary || form.filter => Class::Lost,
        Some(_) => Class::Opened,
    }
}

/// A file no grammar read now, by what the base's own reader made of the base's bytes.
fn unread(project: &Project, against: Option<(&[Change], &str)>, file: &str) -> Sorted {
    let now = refusal_at(project.root(), file).unwrap_or(Refusal::Parse { line: 1, column: 1 });
    let class = match against {
        None => Class::Limit,
        Some((changes, base)) => against_base(project.root(), (changes, base), file),
    };
    let language = syntax::language_of(file).map_or("its", |language| language.name);
    let (reason, at, said) = match now {
        Refusal::Parse { line, column } => (
            "parse",
            Some((line, column)),
            format!("the {language} grammar finds an error at line {line}, column {column}"),
        ),
        Refusal::LineCeiling { line } => (
            "line-ceiling",
            None,
            format!("line {line} is over the source-line ceiling of 65536 bytes"),
        ),
        Refusal::NotText => (
            "form",
            None,
            "it holds a NUL byte, so it is not text".to_string(),
        ),
    };
    let reason = match class {
        Class::Lost => reason,
        Class::Opened | Class::Limit => opened_reason(now),
    };
    (class, reason, at, said)
}

/// The reason an opened gap or a coverage note names a read that failed by. Spec 7.2.
fn opened_reason(refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::Parse { .. } => "unreadable",
        Refusal::LineCeiling { .. } => "resource-limit",
        Refusal::NotText => "not-text",
    }
}

/// Whether the base measured this file: lost where the base's reader read the base's bytes,
/// opened where the base did not hold it or no reader read its base path, and klin's own limit
/// where the change left it alone or the base could not read it either. A rename is read at the
/// path the base held it at, with that path's reader. Spec 7.2.
fn against_base(root: &Path, (changes, base): (&[Change], &str), file: &str) -> Class {
    let Some(change) = changes.iter().find(|change| change.path == file) else {
        return Class::Limit;
    };
    let Some(was) = change.was.as_deref() else {
        return Class::Opened;
    };
    if syntax::language_of(was).is_none() {
        return Class::Opened;
    }
    let Some(bytes) = changed::blob(root, base, was) else {
        return Class::Opened;
    };
    match syntax::refusal(was, &String::from_utf8_lossy(&bytes)) {
        None => Class::Lost,
        Some(_) => Class::Limit,
    }
}

/// Why the strict read refuses the working tree's copy of this file.
fn refusal_at(root: &Path, file: &str) -> Option<Refusal> {
    let bytes = std::fs::read(root.join(file)).ok()?;
    syntax::refusal(file, &String::from_utf8_lossy(&bytes))
}

/// A file that left a gate's scope: lost where its working-tree path is now a symbolic link,
/// klin's own limit where a person changed `klin.json`, and an opened gap otherwise. Spec 7.2.
fn left(project: &Project, against: Option<(&[Change], &str)>, file: &str, why: &str) -> Sorted {
    let root = project.root();
    if linked(root, file) {
        return (
            Class::Lost,
            "form",
            None,
            "it is a symbolic link now, so it is not text".to_string(),
        );
    }
    let configured = project
        .config
        .file
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .is_some_and(|config| {
            against.is_some_and(|(changes, _)| changes.iter().any(|change| change.path == config))
        });
    let class = match (against, configured) {
        (None, _) | (_, true) => Class::Limit,
        (Some(_), false) => Class::Opened,
    };
    (
        class,
        "left-scope",
        None,
        format!("measured at the base and not now — {why}"),
    )
}

fn linked(root: &Path, file: &str) -> bool {
    std::fs::symlink_metadata(root.join(file)).is_ok_and(|held| held.file_type().is_symlink())
}
