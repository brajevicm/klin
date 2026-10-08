use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use serde_json::{Map, Value};

use crate::git::Repo;
use crate::state;

/// The stamp file in the state directory, and the name it is written under before the rename,
/// so a hook that dies mid-write leaves the previous stamp rather than a torn one. Spec 6.5.
pub const FILE: &str = "turn";
/// The completed index retained after stamp publication, apart from a person's staged index.
pub const INDEX: &str = "index";
/// The prompt mark, beside the stamp and under the same guarded namespace. The stamp waits
/// for a green stop, so a report keyed to it re-measures one widening window on every prompt.
/// The mark moves on every event, and it is what the report measures. ADR 0024.
pub const MARK: &str = "refs/worktree/klin/mark";

/// What the last Stop under a stamp wrote, and `Pending` before one did. The first matching row
/// of the spec 6.6 table wins, so a Stop writes exactly one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Verdict {
    #[default]
    Pending,
    /// klin failed during the Stop, which wrote this before it measured.
    Aborted {
        since: u64,
    },
    /// The open findings by site id, and the deleted tests klin has not asked about yet.
    Red {
        open: Vec<String>,
        unasked: Vec<String>,
    },
    /// A run-scope configuration error stopped the Stop before it measured. `kept` says the
    /// window holds work no Stop judged, so the next prompt keeps the stamp.
    Unjudged {
        error: String,
        kept: bool,
    },
    Green,
}

impl Verdict {
    pub fn name(&self) -> &'static str {
        match self {
            Verdict::Pending => "pending",
            Verdict::Aborted { .. } => "aborted",
            Verdict::Red { .. } => "red",
            Verdict::Unjudged { .. } => "unjudged",
            Verdict::Green => "green",
        }
    }

    /// A red verdict no Stop judged, which keeps the window until a Stop does. Spec 6.2.
    pub fn red() -> Verdict {
        Verdict::Red {
            open: Vec::new(),
            unasked: Vec::new(),
        }
    }

    /// The verdict of a Stop that a run-scope configuration error stopped before it measured.
    pub fn unjudged(error: String) -> Verdict {
        Verdict::Unjudged { error, kept: false }
    }

    /// What a Stop leaves over the verdict before it: an `unjudged` Stop never hides a window it
    /// did not judge, so `red` and `aborted` stay, and over `pending` the next prompt keeps the
    /// stamp. Only a window a Stop judged green moves past an `unjudged` Stop. Spec 6.6.
    pub fn over(self, prior: Verdict) -> Verdict {
        match (self, prior) {
            (Verdict::Unjudged { .. }, kept @ (Verdict::Red { .. } | Verdict::Aborted { .. })) => {
                kept
            }
            (Verdict::Unjudged { error, .. }, Verdict::Pending) => {
                Verdict::Unjudged { error, kept: true }
            }
            (Verdict::Unjudged { error, .. }, Verdict::Unjudged { kept, .. }) => {
                Verdict::Unjudged { error, kept }
            }
            (verdict, _) => verdict,
        }
    }

    /// Whether the next session or prompt moves the stamp. An `unjudged` window moves, so the
    /// first Stop after a fix never judges what came before it. Spec 6.6.
    pub fn moves(&self) -> bool {
        matches!(self, Verdict::Green | Verdict::Unjudged { kept: false, .. })
    }

    fn read(held: &Value) -> Verdict {
        let text = |key: &str| held.get(key).and_then(Value::as_str).unwrap_or_default();
        match text("verdict") {
            "aborted" => Verdict::Aborted {
                since: held
                    .get("since")
                    .and_then(Value::as_u64)
                    .unwrap_or_default(),
            },
            "red" => Verdict::Red {
                open: strings(held, "open"),
                unasked: strings(held, "unasked"),
            },
            "unjudged" => Verdict::Unjudged {
                error: text("error").to_string(),
                kept: held.get("kept").and_then(Value::as_bool) == Some(true),
            },
            "green" => Verdict::Green,
            _ => Verdict::Pending,
        }
    }

    fn record(&self, fields: &mut Map<String, Value>) {
        fields.insert("verdict".into(), self.name().into());
        match self {
            Verdict::Aborted { since } => {
                fields.insert("since".into(), (*since).into());
            }
            Verdict::Red { open, unasked } => {
                listed(fields, "open", open);
                listed(fields, "unasked", unasked);
            }
            Verdict::Unjudged { error, kept } => {
                fields.insert("error".into(), error.clone().into());
                if *kept {
                    fields.insert("kept".into(), true.into());
                }
            }
            Verdict::Pending | Verdict::Green => {}
        }
    }
}

/// Where the turn's window opens: the stamped commit, the HEAD it was taken over, when it was
/// taken, the verdict of the last stop, and how many prompts this worktree has seen.
#[derive(Default)]
pub struct Stamp {
    pub commit: Option<String>,
    pub parent: Option<String>,
    /// Where the last event left the prompt mark, which the spread report measures from.
    pub mark: Option<String>,
    pub time: u64,
    pub verdict: Verdict,
    pub prompts: u64,
    /// The findings a stop's block already put in front of the agent under this stamp, by the
    /// site id of spec 11.2. A fresh stamp holds none. Spec 8.2.
    pub asked: Vec<String>,
    /// Whether a stop under this stamp spent a gate block, so the turn holds an intervention for
    /// the turn end to tell. A fresh stamp holds none. Spec 6.5, 9.5.
    pub intervened: bool,
    /// The notes and errors a Stop already told under this stamp, by record, so a later Stop
    /// does not repeat them. A fresh stamp holds none. Spec 2.3.
    pub told: Vec<String>,
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
        verdict: Verdict::read(&held),
        prompts: held
            .get("prompts")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        asked: strings(&held, "asked"),
        intervened: held
            .get("intervened")
            .and_then(Value::as_bool)
            .unwrap_or_default(),
        told: strings(&held, "told"),
    })
}

fn strings(held: &Value, key: &str) -> Vec<String> {
    held.get(key)
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn listed(fields: &mut Map<String, Value>, key: &str, ids: &[String]) {
    if !ids.is_empty() {
        fields.insert(key.into(), ids.to_vec().into());
    }
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
    stamp.verdict.record(&mut fields);
    fields.insert("prompts".into(), stamp.prompts.into());
    listed(&mut fields, "asked", &stamp.asked);
    if stamp.intervened {
        fields.insert("intervened".into(), true.into());
    }
    listed(&mut fields, "told", &stamp.told);
    Value::Object(fields)
}

/// A completed private index, retained only after the caller publishes the stamp it describes.
pub struct Capture {
    pub tree: String,
    temporary: tempfile::TempDir,
    _lock: state::Lock,
}

impl Capture {
    pub fn retain(self, index: &Path) -> Option<String> {
        std::fs::rename(self.temporary.path().join(INDEX), index).ok()?;
        Some(self.tree)
    }
}

/// The same tree through an index the caller names, for a reader that must not leave the
/// stamp's own index behind, because `run` reads that file's absence as a first session.
pub fn tree_through(root: &Path, index: &Path) -> Option<String> {
    capture(root, index)?.retain(index)
}

pub fn capture(root: &Path, index: &Path) -> Option<Capture> {
    let scratch = index.with_extension("captures");
    std::fs::create_dir_all(&scratch).ok()?;
    let lock = state::lock(&scratch, Duration::from_secs(30))?;
    remove_orphans(&scratch)?;
    let temporary = tempfile::Builder::new()
        .prefix("capture-")
        .tempdir_in(scratch)
        .ok()?;
    let fresh = temporary.path().join(INDEX);
    git(root, Some(&fresh), &["add", "-A"])?;
    let tree = git(root, Some(&fresh), &["write-tree"])?;
    Some(Capture {
        tree,
        temporary,
        _lock: lock,
    })
}

/// Called only while holding the capture lock, so no live capture is swept.
fn remove_orphans(scratch: &Path) -> Option<()> {
    // The lock makes every prior capture directory an orphan, including after SIGKILL.
    for entry in std::fs::read_dir(scratch).ok()? {
        let entry = entry.ok()?;
        if entry.file_name().to_str()?.starts_with("capture-") {
            std::fs::remove_dir_all(entry.path()).ok()?;
        }
    }
    Some(())
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
