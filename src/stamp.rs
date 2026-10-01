use std::ffi::OsStr;
use std::path::Path;

use serde_json::{Map, Value};

use crate::git::Repo;
use crate::state;

/// The stamp file in the state directory, and the name it is written under before the rename,
/// so a hook that dies mid-write leaves the previous stamp rather than a torn one. Spec 6.5.
pub const FILE: &str = "turn";
/// The index the stamp is built in, apart from the one a person's `git add` writes.
pub const INDEX: &str = "index";
/// The prompt mark, beside the stamp and under the same guarded namespace. The stamp waits
/// for a green stop, so a report keyed to it re-measures one widening window on every prompt.
/// The mark moves on every event, and it is what the report measures. ADR 0024.
pub const MARK: &str = "refs/worktree/klin/mark";

/// Where the turn's window opens: the stamped commit, the HEAD it was taken over, when it was
/// taken, the verdict of the last stop, and how many prompts this worktree has seen.
pub struct Stamp {
    pub commit: Option<String>,
    pub parent: Option<String>,
    /// Where the last event left the prompt mark, which the spread report measures from.
    pub mark: Option<String>,
    pub time: u64,
    pub green: bool,
    pub prompts: u64,
    /// The findings a stop's block already put in front of the agent under this stamp, by the
    /// site id of spec 11.2. A fresh stamp holds none. Spec 8.2.
    pub asked: Vec<String>,
    /// Whether a stop under this stamp spent a gate block, so the turn holds an intervention for
    /// the turn end to tell. A fresh stamp holds none. Spec 6.5, 9.5.
    pub intervened: bool,
}

/// Where this turn's window opened: the prompt mark the last event left, or the mark ref when
/// the turn file is gone. It writes nothing back, because the report judges nothing. ADR 0024.
pub fn mark(root: &Path, at: &Path) -> Option<String> {
    let held = read(at)
        .and_then(|held| held.mark)
        .filter(|mark| resolve(root, mark).is_some());
    held.or_else(|| resolve(root, MARK))
}

/// The commit every derived value comes from: the stamp's parent, which is the HEAD the stamp
/// was taken over, so a commit inside an open turn does not move it. HEAD when no stamp is
/// readable, and `None` outside a repository. Spec 6.6.
pub fn derivation(root: &Path, at: Option<&Path>) -> Option<String> {
    at.and_then(read)
        .and_then(|held| held.parent)
        .filter(|parent| resolve(root, parent).is_some())
        .or_else(|| resolve(root, "HEAD"))
}

/// The derivation commit when no window was chosen: the stamp's parent under the state
/// directory, and HEAD without one. Spec 6.6.
pub fn unwindowed(root: &Path) -> Option<String> {
    derivation(root, state::ready(root).ok().as_deref())
}

/// The stamp the `turn` file holds, and `None` when the file is missing or unreadable.
/// Spec 6.5.
pub fn read(at: &Path) -> Option<Stamp> {
    let text = std::fs::read_to_string(at.join(FILE)).ok()?;
    let held: Value = serde_json::from_str(&text).ok()?;
    let text = |key: &str| {
        held.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|found| !found.is_empty())
    };
    Some(Stamp {
        commit: text("commit"),
        parent: text("parent"),
        mark: text("mark"),
        time: held.get("time").and_then(Value::as_u64).unwrap_or_default(),
        green: text("verdict").as_deref() == Some("green"),
        prompts: held
            .get("prompts")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        asked: held
            .get("asked")
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        intervened: held
            .get("intervened")
            .and_then(Value::as_bool)
            .unwrap_or_default(),
    })
}

/// The stamp as the `turn` file holds it. A field a fresh stamp does not have is left out.
/// Spec 6.5.
pub fn recorded(stamp: &Stamp) -> Value {
    let mut fields = Map::new();
    let fields_of = [
        ("commit", stamp.commit.clone().map(Value::from)),
        ("parent", stamp.parent.clone().map(Value::from)),
        ("mark", stamp.mark.clone().map(Value::from)),
    ];
    for (key, value) in fields_of {
        if let Some(found) = value {
            fields.insert(key.into(), found);
        }
    }
    fields.insert("time".into(), stamp.time.into());
    let verdict = match stamp.green {
        true => "green",
        false => "red",
    };
    fields.insert("verdict".into(), verdict.into());
    fields.insert("prompts".into(), stamp.prompts.into());
    if !stamp.asked.is_empty() {
        fields.insert("asked".into(), stamp.asked.clone().into());
    }
    if stamp.intervened {
        fields.insert("intervened".into(), true.into());
    }
    Value::Object(fields)
}

/// A tree of the working directory, everything `.gitignore` does not exclude, written through
/// an index of klin's own. Both the stamp and the spread report read the turn from it.
pub fn tree(root: &Path, at: &Path) -> Option<String> {
    tree_through(root, &at.join(INDEX))
}

/// The same tree through an index the caller names, for a reader that must not leave the
/// stamp's own index behind, because `run` reads that file's absence as a first session.
pub fn tree_through(root: &Path, index: &Path) -> Option<String> {
    let _ = std::fs::remove_file(index);
    git(root, Some(index), &["add", "-A"])?;
    git(root, Some(index), &["write-tree"])
}

/// The commit a ref or object name points at, and `None` when it names no commit.
pub fn resolve(root: &Path, reference: &str) -> Option<String> {
    let refspec = format!("{reference}^{{commit}}");
    Repo::at(root).rev_parse(&["--verify", "--quiet", &refspec])
}

/// A git call with klin as the author of any commit it makes and an optional index of its own,
/// so nothing here touches what a person staged.
pub fn git(root: &Path, index: Option<&Path>, args: &[&str]) -> Option<String> {
    let mut command = vec![
        "-c",
        "user.name=klin",
        "-c",
        "user.email=klin@invalid",
        "-c",
        "commit.gpgsign=false",
    ];
    command.extend_from_slice(args);
    let text = match index {
        Some(index) => {
            let env = [(OsStr::new("GIT_INDEX_FILE"), index.as_os_str())];
            Repo::at(root).text_with_env(&command, &env)
        }
        None => Repo::at(root).text(&command),
    }?;
    Some(text.trim().to_string())
}
