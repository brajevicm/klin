//! The one repository-path scope every section shares. `in` names the paths a rule applies to
//! and `except` takes paths out of it. A selector is one repository-relative path, which names
//! itself and everything below it. It is never a glob and never outside the repository, so a
//! path that could only match nothing is refused when it is read. `.` is the repository root.
//! Spec 8.4, ADR 0038.

use serde_json::{Map, Value};

use crate::reference::Key;

/// The repository root, which every path is relative to and which holds every path.
pub const ROOT: &str = ".";

/// One `in` or `except` path, normalized: no leading `./` and no trailing slash.
#[derive(Clone, Debug, PartialEq, Eq)]
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
