use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::state;

/// The cache under one state directory, as a run holds it: a run that keeps state reads and
/// writes it, and a read-only command such as `klin policy` only reads it. Spec 11.6.
#[derive(Clone, Copy)]
pub struct Cache<'a> {
    at: &'a Path,
    keeps: bool,
}

impl<'a> Cache<'a> {
    pub fn new(at: &'a Path, keeps: bool) -> Cache<'a> {
        Cache { at, keeps }
    }

    pub fn read(self, commit: &str, key: &str) -> Option<Value> {
        read(self.at, commit, key)
    }

    pub fn write(self, commit: &str, key: &str, value: Value) {
        if self.keeps {
            write(self.at, commit, key, value);
        }
    }
}

/// What one derivation commit's survey holds, under the state directory keyed by that commit.
/// Its contents are a pure function of the commit and the binary version, so a file another
/// version wrote reads as nothing. Spec 6.6.
pub fn read(at: &Path, commit: &str, key: &str) -> Option<Value> {
    ours(&file(at, commit))?.get(key).cloned()
}

/// One key of that survey, merged into what the file already holds, so two derivations under
/// one commit do not overwrite each other. A file another version wrote is replaced rather than
/// merged into, because its other keys are that version's and not this one's. A cache klin
/// cannot write costs the next run the same derivation and nothing else.
pub fn write(at: &Path, commit: &str, key: &str, value: Value) {
    let file = file(at, commit);
    let mut fields = ours(&file).unwrap_or_default();
    fields.insert(VERSION.to_string(), env!("CARGO_PKG_VERSION").into());
    fields.insert(key.to_string(), value);
    if let Some(under) = file.parent() {
        let _ = std::fs::create_dir_all(under);
    }
    let _ = std::fs::write(&file, Value::Object(fields).to_string() + "\n");
}

/// The file's keys when this binary wrote them, and `None` for a file that is gone, unreadable
/// or another version's.
fn ours(file: &Path) -> Option<Map<String, Value>> {
    let text = std::fs::read_to_string(file).ok()?;
    let held: Value = serde_json::from_str(&text).ok()?;
    let fields = held.as_object()?;
    match fields.get(VERSION).and_then(Value::as_str) {
        Some(env!("CARGO_PKG_VERSION")) => Some(fields.clone()),
        _ => None,
    }
}

const VERSION: &str = "version";

fn file(at: &Path, commit: &str) -> PathBuf {
    at.join(state::CACHE).join(format!("{commit}.json"))
}
