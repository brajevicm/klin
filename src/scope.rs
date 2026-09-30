//! The one repository-path scope every section shares. `in` names the paths a rule applies to
//! and `except` takes paths out of it. A selector is one repository-relative path, which names
//! itself and everything below it. It is never a glob and never outside the repository, so a
//! path that could only match nothing is refused when it is read. `.` is the repository root.
//! Spec 8.4, ADR 0038.

use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;

use crate::config::{Config, Error};
use crate::ratchet::Values;
use crate::reference::Key;

pub const IN: Key = Key {
    name: "in",
    holds: "a repository-relative path, or a list of them, the section applies to, with everything below each",
    required: false,
    rule: None,
    default: "the whole repository",
    shape: crate::reference::Shape::StringOrList,
};

pub const EXCEPT: Key = Key {
    name: "except",
    holds: "a repository-relative path, or a list of them, taken out of `in`, with everything below each",
    required: false,
    rule: None,
    default: "nothing is taken out",
    shape: crate::reference::Shape::StringOrList,
};

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Scope {
    within: Vec<Selector>,
    except: Vec<Selector>,
}

impl Scope {
    pub fn read(config: &Config, section: &str, fields: &Values) -> Result<Scope, Error> {
        Scope::from_fields(fields)
            .map_err(|why| Error(format!("{}: \"{section}\" {why}", config.file.display())))
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
        Ok(Scope { within, except })
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
        (self.within.is_empty() || any_holds(&self.within, path)) && !any_holds(&self.except, path)
    }

    pub fn has_in(&self) -> bool {
        !self.within.is_empty()
    }

    pub fn inside(&self, path: &str) -> bool {
        self.within.is_empty() || any_holds(&self.within, path)
    }

    /// The scope recorded by the base commit, or today's scope when that commit has no readable
    /// compact policy. Checks decide when this historical scope matters to their comparison.
    pub fn at_base(config: &Config, section: &str, prior: &Path, today: &Scope) -> Scope {
        config
            .file
            .file_name()
            .and_then(|name| std::fs::read_to_string(prior.join(name)).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .and_then(|data| match data.get(section) {
                None => Some(Scope::default()),
                Some(value) => Scope::read(config, section, value.as_object()?).ok(),
            })
            .unwrap_or_else(|| today.clone())
    }
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
pub struct Roots(HashSet<String>);

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
