//! The one repository-path scope every section shares. `in` names the paths a rule applies to
//! and `except` takes paths out of it. A selector is one repository-relative path, which names
//! itself and everything below it. It is never a glob and never outside the repository, so a
//! path that could only match nothing is refused when it is read. `.` is the repository root.
//! Spec 8.4, ADR 0038.

use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;

use crate::changed::Change;
use crate::config::Config;
use crate::error::Error;
use crate::git::Repo;
use crate::key::Key;
use crate::record::Values;

pub const IN: Key = Key {
    name: "in",
    holds: "a repository-relative path, or a list of them, the section applies to, with everything below each",
    required: false,
    rule: None,
    default: "the whole repository",
    shape: crate::key::Shape::StringOrList,
};

pub const EXCEPT: Key = Key {
    name: "except",
    holds: "a repository-relative path, or a list of them, taken out of `in`, with everything below each",
    required: false,
    rule: None,
    default: "nothing is taken out",
    shape: crate::key::Shape::StringOrList,
};

/// A section's scope: what its `in` and `except` state, and the paths this run keeps in it
/// because the change moved them, which no section states. Two scopes are the same when they
/// state the same paths. Spec 7.3.
#[derive(Clone, Default)]
pub struct Scope {
    within: Vec<Selector>,
    except: Vec<Selector>,
    kept: Vec<String>,
    /// The `in` paths that selected files in the base and that the change moved or deleted.
    pinned: Vec<String>,
}

impl PartialEq for Scope {
    fn eq(&self, other: &Scope) -> bool {
        self.within == other.within && self.except == other.except
    }
}

impl Scope {
    /// The scope a section states, with the paths this run keeps in it because the change moved
    /// them. Spec 7.3.
    pub fn read(
        config: &Config,
        moves: &Moves,
        section: &str,
        fields: &Values,
    ) -> Result<Scope, Error> {
        let mut scope = Scope::from_fields(fields)
            .map_err(|why| Error(format!("{}: \"{section}\" {why}", config.file.display())))?;
        scope.kept = moves.kept(section);
        scope.pinned = moves.pinned(section);
        Ok(scope)
    }

    /// The scope a section states, in one form for every way of writing the same selection:
    /// sorted, with no path another path of its list holds, no `in` of the repository root, and
    /// no `except` outside every `in`.
    pub fn from_fields(fields: &Values) -> Result<Scope, String> {
        let mut within = outermost(selectors(fields, IN)?);
        within.retain(|selector| selector.as_str() != ROOT);
        let except = outermost(selectors(fields, EXCEPT)?)
            .into_iter()
            .filter(|out| {
                within.is_empty()
                    || within
                        .iter()
                        .any(|kept| kept.holds(out.as_str()) || out.holds(kept.as_str()))
            })
            .collect();
        Ok(Scope {
            within,
            except,
            ..Scope::default()
        })
    }

    /// The scope as a section states it, which `from_fields` reads back as the same scope.
    pub fn value(&self) -> Value {
        let mut fields = Map::new();
        for (key, selectors) in [(IN, &self.within), (EXCEPT, &self.except)] {
            if !selectors.is_empty() {
                fields.insert(
                    key.name.into(),
                    selectors.iter().map(Selector::as_str).collect(),
                );
            }
        }
        Value::Object(fields)
    }

    pub fn description(&self) -> String {
        let names = |selectors: &[Selector]| {
            selectors
                .iter()
                .map(Selector::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        };
        match (self.within.is_empty(), self.except.is_empty()) {
            (true, true) => "whole repository".into(),
            (false, true) => format!("in {}", names(&self.within)),
            (true, false) => format!("except {}", names(&self.except)),
            (false, false) => format!("in {}; except {}", names(&self.within), names(&self.except)),
        }
    }

    pub fn selects(&self, path: &str) -> bool {
        self.keeps(path)
            || ((self.within.is_empty() || any_holds(&self.within, path))
                && !any_holds(&self.except, path))
    }

    /// Whether the section states an `in` that must select an applicable file. An `in` whose
    /// every path is a pin the change moved need not, and any other path, such as one this
    /// change wrote, still must. Spec 7.3.
    pub fn has_in(&self) -> bool {
        !self.within.is_empty()
            && !self
                .within
                .iter()
                .all(|selector| self.pinned.iter().any(|pin| pin == selector.as_str()))
    }

    /// Whether the stated `in` holds the path, which is what an `in` must select. A path this
    /// run keeps because the change moved it is not stated, so it counts for no `in`.
    pub fn inside(&self, path: &str) -> bool {
        self.within.is_empty() || any_holds(&self.within, path)
    }

    /// Whether this run keeps the path in the scope because the change moved it. Spec 7.3.
    fn keeps(&self, path: &str) -> bool {
        self.kept.iter().any(|kept| kept == path)
    }

    /// The scope recorded by the base commit, or today's scope when that commit has no readable
    /// compact policy, with the paths today's run keeps. Checks decide when this historical
    /// scope matters to their comparison.
    pub fn at_base(config: &Config, section: &str, prior: &Path, today: &Scope) -> Scope {
        config
            .file
            .file_name()
            .and_then(|name| std::fs::read_to_string(prior.join(name)).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|data| match data.get(section) {
                None => Some(Scope::default()),
                Some(value) => Scope::from_fields(value.as_object()?).ok(),
            })
            .map(|stated| Scope {
                kept: today.kept.clone(),
                pinned: today.pinned.clone(),
                ..stated
            })
            .unwrap_or_else(|| today.clone())
    }
}

/// What the change did to the paths the policy names, by a deterministic test on both trees:
/// each pinned `in` path that selects no file of the working tree, and each selected file a
/// rename took out of a scope that still selects other files. A pinned path that selects
/// nothing in the base either counts only where the base's own `klin.json` pins it and no path
/// of the same `in` selects a file: a path this change wrote that names nothing stays a
/// configuration error, and one beside a path that selects files never was one. Spec 7.3.
pub fn moved(config: &Config, files: &[String], base: &str, changes: &[Change]) -> Moves {
    let renamed: Vec<(&str, &str)> = changes
        .iter()
        .filter_map(|change| {
            let was = change.was.as_deref().filter(|was| *was != change.path)?;
            Some((was, change.path.as_str()))
        })
        .collect();
    let mut at_base = AtBase::new(config, base);
    let mut out = Vec::new();
    for (section, fields) in config.objects() {
        if let Ok(scope) = Scope::from_fields(fields) {
            out.extend(section_moves(
                section,
                &scope,
                (files, &renamed),
                &mut at_base,
            ));
        }
    }
    Moves(out)
}

/// What the change did to one section's scope: a `Pin` per pinned path that selects no file
/// now and is not quiet, and an `Out` per file a rename took out of a scope that still selects
/// files.
fn section_moves(
    section: &str,
    scope: &Scope,
    (files, renamed): (&[String], &[(&str, &str)]),
    at_base: &mut AtBase,
) -> Vec<Moved> {
    let dead: Vec<&Selector> = scope
        .within
        .iter()
        .filter(|selector| !files.iter().any(|file| selector.holds(file)))
        .collect();
    let quiet = dead.len() < scope.within.len();
    let mut out = Vec::new();
    for selector in &dead {
        let held = at_base.held(selector);
        if held == 0 && (quiet || !at_base.pins(section, selector)) {
            continue;
        }
        out.push(pin(section, selector, held, renamed));
    }
    if files.iter().any(|file| scope.selects(file)) {
        out.extend(moved_out(section, scope, &dead, renamed));
    }
    out
}

/// Whether any section states an `in` or an `except`, so a path the change moved can matter.
pub fn states_a_scope(config: &Config) -> bool {
    config
        .objects()
        .any(|(_, fields)| fields.contains_key(IN.name) || fields.contains_key(EXCEPT.name))
}

/// What the base commit holds, read only when a pinned path selects nothing now: its file list
/// and its own `klin.json`, each read once for every path that asks.
struct AtBase<'a> {
    config: &'a Config,
    repo: Repo<'a>,
    base: &'a str,
    listed: Option<Vec<String>>,
    based: Option<Option<Value>>,
}

impl<'a> AtBase<'a> {
    fn new(config: &'a Config, base: &'a str) -> AtBase<'a> {
        AtBase {
            config,
            repo: Repo::at(config.root()),
            base,
            listed: None,
            based: None,
        }
    }

    /// How many files of the base the path selects.
    fn held(&mut self, selector: &Selector) -> usize {
        let (repo, base) = (&self.repo, self.base);
        let listed = self
            .listed
            .get_or_insert_with(|| repo.ls_tree_paths(base).unwrap_or_default());
        listed.iter().filter(|path| selector.holds(path)).count()
    }

    /// Whether the base's `klin.json` pins this path in the section's `in`.
    fn pins(&mut self, section: &str, selector: &Selector) -> bool {
        let (config, repo, base) = (self.config, &self.repo, self.base);
        self.based
            .get_or_insert_with(|| {
                let name = config.file.file_name()?.to_string_lossy();
                serde_json::from_slice(&repo.blob(base, &name)?).ok()
            })
            .as_ref()
            .and_then(|data| Scope::from_fields(data.get(section)?.as_object()?).ok())
            .is_some_and(|scope| scope.within.contains(selector))
    }
}

/// A pinned path that selects no file of the working tree, with the files git saw renamed out
/// of it and how many of the `held` files the base held there went with no rename.
fn pin(section: &str, selector: &Selector, held: usize, renamed: &[(&str, &str)]) -> Moved {
    let gone: Vec<(String, String)> = renamed
        .iter()
        .filter(|(was, _)| selector.holds(was))
        .map(|(was, path)| (was.to_string(), path.to_string()))
        .collect();
    Moved::Pin {
        section: section.to_string(),
        path: selector.as_str().to_string(),
        deleted: held.saturating_sub(gone.len()),
        renamed: gone,
    }
}

/// The selected files a rename took out of the scope, other than those of a pinned path that
/// selects nothing now, which `pin` follows.
fn moved_out(
    section: &str,
    scope: &Scope,
    dead: &[&Selector],
    renamed: &[(&str, &str)],
) -> Vec<Moved> {
    renamed
        .iter()
        .filter(|(was, path)| {
            scope.selects(was) && !scope.selects(path) && !any_holds_of(dead, was)
        })
        .map(|(_, path)| Moved::Out {
            section: section.to_string(),
            path: path.to_string(),
        })
        .collect()
}

/// What the change did to the paths the policy names, decided once per run against the base
/// and followed by every scope the run reads. Spec 7.3.
#[derive(Default)]
pub struct Moves(Vec<Moved>);

impl Moves {
    pub fn iter(&self) -> impl Iterator<Item = &Moved> {
        self.0.iter()
    }

    /// The paths this run keeps in a section's scope because the change moved them.
    fn kept(&self, section: &str) -> Vec<String> {
        let mut kept = Vec::new();
        for moved in self.iter().filter(|moved| moved.section() == Some(section)) {
            match moved {
                Moved::Pin { renamed, .. } => {
                    kept.extend(renamed.iter().map(|(_, path)| path.clone()))
                }
                Moved::Out { path, .. } => kept.push(path.clone()),
                Moved::Skipped { .. } => {}
            }
        }
        kept
    }

    /// The pinned `in` paths of the section the change moved, which may select nothing.
    fn pinned(&self, section: &str) -> Vec<String> {
        self.iter()
            .filter_map(|moved| match moved {
                Moved::Pin {
                    section: of, path, ..
                } if of == section => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// The paths of the files a rename took out of the section's scope.
    pub fn out_of(&self, section: &str) -> Vec<&str> {
        self.iter()
            .filter_map(|moved| match moved {
                Moved::Out { section: of, path } if of == section => Some(path.as_str()),
                _ => None,
            })
            .collect()
    }
}

impl Extend<Moved> for Moves {
    fn extend<T: IntoIterator<Item = Moved>>(&mut self, moves: T) {
        self.0.extend(moves);
    }
}

/// What a change did to a path the policy names, decided once per run against the base. Spec 7.3.
#[derive(Clone, Debug)]
pub enum Moved {
    /// A pinned `in` path that selects no file of the working tree: the files git saw renamed
    /// out of it, by the path the base held and the path they have now, and how many of the
    /// files it selected in the base went with no rename. Nothing renamed and nothing deleted
    /// is a pin that selected nothing in the base either.
    Pin {
        section: String,
        path: String,
        renamed: Vec<(String, String)>,
        deleted: usize,
    },
    /// A selected file the change renamed out of a scope that still selects other files.
    Out { section: String, path: String },
    /// A file the change renamed from a path a walk reaches to one under a directory every walk
    /// skips, which no check measures in either tree.
    Skipped { was: String, path: String },
}

impl Moved {
    /// The section whose scope the move touches, and none for a move no scope decides.
    pub fn section(&self) -> Option<&str> {
        match self {
            Moved::Pin { section, .. } | Moved::Out { section, .. } => Some(section),
            Moved::Skipped { .. } => None,
        }
    }

    /// Whether files of a moved pin went with no rename, or it selects nothing in either tree,
    /// or a file went under a skipped directory, which the Stop notes. Spec 7.3.
    pub fn gone(&self) -> bool {
        match self {
            Moved::Pin {
                renamed, deleted, ..
            } => renamed.is_empty() || *deleted > 0,
            Moved::Out { .. } => false,
            Moved::Skipped { .. } => true,
        }
    }

    /// What a moved pin or a file moved under a skipped directory says to a person, and nothing
    /// for a file moved out of a scope, whose findings carry `moved_out_of_scope`. Spec 7.3.
    pub fn said(&self) -> Option<String> {
        let (section, path, renamed, deleted) = match self {
            Moved::Pin {
                section,
                path,
                renamed,
                deleted,
            } => (section, path, renamed, deleted),
            Moved::Out { .. } => return None,
            Moved::Skipped { was, path } => {
                return Some(format!(
                    "{was} moved to {path}, under a directory every walk skips, so no check \
                     measures it — move it back, or review the move"
                ));
            }
        };
        let to = renamed
            .iter()
            .map(|(_, path)| path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let what = match (renamed.is_empty(), *deleted) {
            (false, 0) => format!("its files moved to {to}, and this run measures them there"),
            (false, deleted) => format!(
                "its files moved to {to}, and this run measures them there, and {deleted} \
                 file(s) went with no rename"
            ),
            (true, 0) => "it selects nothing in the base or the working tree".to_string(),
            (true, deleted) => format!(
                "its {deleted} file(s) went with no rename, so the gate measures nothing there"
            ),
        };
        Some(format!(
            "the pinned \"in\" path {path} of \"{section}\" selects no file of the working tree: \
             {what} — update the pin in klin.json"
        ))
    }

    /// The old and new path of each file a moved pin followed or a rename took under a skipped
    /// directory, and nothing where none was renamed. Spec 11.7.
    pub fn reason(&self) -> Option<String> {
        let renamed = match self {
            Moved::Pin { renamed, .. } => renamed,
            Moved::Out { .. } => return None,
            Moved::Skipped { was, path } => return Some(format!("{was} -> {path}")),
        };
        (!renamed.is_empty()).then(|| {
            renamed
                .iter()
                .map(|(was, now)| format!("{was} -> {now}"))
                .collect::<Vec<_>>()
                .join(", ")
        })
    }
}

/// Whether any of these selectors holds the path.
fn any_holds_of(selectors: &[&Selector], path: &str) -> bool {
    selectors.iter().any(|selector| selector.holds(path))
}

/// The repository root, which every path is relative to and which holds every path.
pub const ROOT: &str = ".";

/// One `in` or `except` path, normalized: no leading `./` and no trailing slash.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Selector(String);

impl Selector {
    /// One path as a person wrote it under `key`, or why it names nothing a scope can hold.
    pub fn parse(key: Key, written: &str) -> Result<Selector, String> {
        let path = written
            .strip_prefix("./")
            .unwrap_or(written)
            .trim_end_matches('/');
        let absolute = written.starts_with(['/', '~']) || written.as_bytes().get(1) == Some(&b':');
        let why = if absolute {
            "is absolute — write it from the repository root"
        } else if path.contains(['*', '?', '[', '\\']) {
            "is not a path — it names a path and everything below it, and is never a glob"
        } else if path != ROOT && path.split('/').any(|part| matches!(part, "" | "." | "..")) {
            "does not name a path inside the repository"
        } else {
            return Ok(Selector(path.to_string()));
        };
        Err(format!(
            "has an \"{}\" path \"{written}\" that {why}",
            key.name
        ))
    }

    /// The path as a report names it.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this selector names the path or a directory above it.
    pub fn holds(&self, path: &str) -> bool {
        under_or_at(path, &self.0)
    }
}

/// The selectors one key of a section states: a path, or a non-empty list of them, and none
/// when the key is absent.
pub fn selectors(fields: &Map<String, Value>, key: Key) -> Result<Vec<Selector>, String> {
    let malformed = || {
        format!(
            "has an \"{}\" that is not a repository-relative path or a non-empty list of them",
            key.name
        )
    };
    let listed: Vec<&Value> = match fields.get(key.name) {
        None => return Ok(Vec::new()),
        Some(Value::Array(items)) if !items.is_empty() => items.iter().collect(),
        Some(one @ Value::String(_)) => vec![one],
        Some(_) => return Err(malformed()),
    };
    listed
        .into_iter()
        .map(|item| Selector::parse(key, item.as_str().ok_or_else(malformed)?))
        .collect()
}

fn outermost(mut selectors: Vec<Selector>) -> Vec<Selector> {
    selectors.sort_unstable();
    let mut kept: Vec<Selector> = Vec::new();
    for selector in selectors {
        if !any_holds(&kept, selector.as_str()) {
            kept.push(selector);
        }
    }
    kept
}

/// Whether any of the selectors holds the path.
pub fn any_holds(selectors: &[Selector], path: &str) -> bool {
    selectors.iter().any(|selector| selector.holds(path))
}

/// Whether a repository-relative path is the directory or lies below it. `.` holds everything.
pub fn under_or_at(path: &str, directory: &str) -> bool {
    directory == ROOT
        || path == directory
        || path
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Spec 5.4.
#[derive(Clone)]
pub struct Roots(HashSet<String>);

impl FromIterator<String> for Roots {
    fn from_iter<I: IntoIterator<Item = String>>(paths: I) -> Roots {
        Roots(paths.into_iter().collect())
    }
}

impl Roots {
    pub fn new(directories: &[String]) -> Roots {
        Roots(directories.iter().cloned().collect())
    }

    pub fn holds(&self, path: &str) -> bool {
        std::iter::once(path)
            .chain(ancestors(path))
            .any(|at| self.0.contains(at))
    }
}

pub fn ancestors(path: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(path), |at| {
        (*at != ROOT).then(|| at.rsplit_once('/').map_or(ROOT, |(up, _)| up))
    })
    .skip(1)
}
