use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::sys::git::Repo;
use crate::sys::state;

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
    /// Where the history stood when the stamp was taken, which tells a later Stop whether the
    /// history moved under the turn. Spec 6.6.
    pub history: History,
}

/// The default-branch merge-base, HEAD's symbolic ref (`HEAD` when detached) and the position
/// of HEAD's reflog. A field the stamp never recorded is `None`. Spec 6.6.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct History {
    pub merge_base: Option<String>,
    pub head_ref: Option<String>,
    pub reflog_position: Option<ReflogPosition>,
}

/// How many entries HEAD's reflog held, which is where a later Stop starts reading it. The
/// reflog is compared by entry position, never by time. Spec 6.6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReflogPosition(pub u64);

impl History {
    /// The history as this event finds it, through the merge-base cache.
    pub fn now(root: &Path, at: &Path, here: &Here) -> History {
        History {
            merge_base: here
                .pair()
                .and_then(|(head, default)| merge_base(root, at, head, default)),
            head_ref: here.head_ref.clone(),
            reflog_position: here
                .reflog
                .as_ref()
                .and_then(|entries| u64::try_from(entries.len()).ok())
                .map(ReflogPosition),
        }
    }
}

/// HEAD, its symbolic ref, the default branch and HEAD's reflog as this event finds them, read
/// from the git directory's files. The default branch is candidate 3 of 0.x 6.3 among
/// remote-tracking refs alone. Spec 6.6.
#[derive(Default)]
pub struct Here {
    pub head: Option<String>,
    /// HEAD's symbolic ref, the branch an in-progress rebase works on, or `HEAD` when detached.
    pub head_ref: Option<String>,
    /// The default branch's ref and the commit it names.
    pub default: Option<(String, String)>,
    /// Whether any `refs/remotes/*` ref exists.
    pub remotes: bool,
    /// The message of each entry of HEAD's reflog, oldest first, and `None` without a reflog.
    pub reflog: Option<Vec<String>>,
}

impl Here {
    pub fn read(root: &Path) -> Here {
        let Some(dir) = Repo::at(root).rev_parse_path("--absolute-git-dir") else {
            return Here::default();
        };
        let common = std::fs::read_to_string(dir.join("commondir"))
            .map_or_else(|_| dir.clone(), |common| dir.join(common.trim()));
        if common.join("reftable").is_dir() {
            return Here::through_git(root, &dir);
        }
        let refs = Refs {
            packed: std::fs::read_to_string(common.join("packed-refs")).unwrap_or_default(),
            dir,
            common,
        };
        let head_ref = rebasing(&refs.dir).or_else(|| refs.symbolic("HEAD"));
        Here {
            head: refs.resolve("HEAD"),
            head_ref: Some(head_ref.unwrap_or_else(|| "HEAD".to_string())),
            default: default_candidates(refs.symbolic("refs/remotes/origin/HEAD"))
                .find_map(|name| Some((name.clone(), refs.resolve(&name)?))),
            remotes: refs.packed.contains(" refs/remotes/")
                || any_file(&refs.common.join("refs/remotes")),
            reflog: std::fs::read_to_string(refs.dir.join("logs/HEAD"))
                .ok()
                .map(|log| {
                    log.lines()
                        .map(|entry| {
                            entry
                                .split_once('\t')
                                .map_or("", |(_, said)| said)
                                .to_string()
                        })
                        .collect()
                }),
        }
    }

    /// The same facts from git itself, for a repository that keeps its refs in a reftable.
    // ponytail: one git process per fact; one `for-each-ref` call if reftable Stops grow slow.
    fn through_git(root: &Path, dir: &Path) -> Here {
        let repo = Repo::at(root);
        let symbolic = |name: &str| {
            repo.text(&["symbolic-ref", "-q", name])
                .map(|found| found.trim().to_string())
                .filter(|found| !found.is_empty())
        };
        Here {
            head: resolve(root, "HEAD"),
            head_ref: Some(
                rebasing(dir)
                    .or_else(|| symbolic("HEAD"))
                    .unwrap_or_else(|| "HEAD".to_string()),
            ),
            default: default_candidates(symbolic("refs/remotes/origin/HEAD"))
                .find_map(|name| Some((name.clone(), resolve(root, &name)?))),
            remotes: repo
                .text(&["for-each-ref", "--count=1", "refs/remotes"])
                .is_some_and(|found| !found.trim().is_empty()),
            reflog: repo
                .text(&["reflog", "exists", "HEAD"])
                .and_then(|_| repo.text(&["reflog", "show", "--format=%gs", "HEAD"]))
                .map(|log| log.lines().rev().map(str::to_string).collect()),
        }
    }

    /// HEAD and the default branch's commit, the pair the merge-base cache is keyed by.
    pub fn pair(&self) -> Option<(&str, &str)> {
        Some((self.head.as_deref()?, self.default.as_ref()?.1.as_str()))
    }

    /// The reflog messages after the position a stamp recorded, and `None` where either side
    /// has no reflog or the reflog is shorter than the position, so nothing can be compared.
    pub fn reflog_since(&self, position: Option<ReflogPosition>) -> Option<&[String]> {
        let entries = self.reflog.as_deref()?;
        entries.get(usize::try_from(position?.0).ok()?..)
    }
}

/// The branch an in-progress rebase works on, which counts as HEAD's symbolic ref while HEAD
/// is detached under it. Git keeps the rebase state in the git directory whatever the ref
/// format. Spec 6.6.
fn rebasing(dir: &Path) -> Option<String> {
    ["rebase-merge", "rebase-apply"]
        .iter()
        .find_map(|rebase| std::fs::read_to_string(dir.join(rebase).join("head-name")).ok())
        .map(|name| name.trim().to_string())
        .filter(|name| name.starts_with("refs/"))
}

fn default_candidates(named: Option<String>) -> impl Iterator<Item = String> {
    named
        .into_iter()
        .chain(["refs/remotes/origin/main", "refs/remotes/origin/master"].map(String::from))
}

/// Loose refs under the git directory, and the packed refs beside them.
struct Refs {
    dir: PathBuf,
    common: PathBuf,
    packed: String,
}

impl Refs {
    fn file(&self, name: &str) -> Option<String> {
        let under = match name {
            "HEAD" => &self.dir,
            _ => &self.common,
        };
        std::fs::read_to_string(under.join(name)).ok()
    }

    fn symbolic(&self, name: &str) -> Option<String> {
        let text = self.file(name)?;
        Some(text.strip_prefix("ref: ")?.trim().to_string())
    }

    fn resolve(&self, name: &str) -> Option<String> {
        let mut name = name.to_string();
        for _ in 0..5 {
            match self.symbolic(&name) {
                Some(target) => name = target,
                None => break,
            }
        }
        let commit = self
            .file(&name)
            .map(|text| text.trim().to_string())
            .or_else(|| {
                self.packed.lines().find_map(|line| {
                    let (commit, named) = line.split_once(' ')?;
                    (named == name).then(|| commit.to_string())
                })
            })?;
        (!commit.is_empty() && commit.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then_some(commit)
    }
}

fn any_file(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let path = entry.path();
            path.is_file() || any_file(&path)
        })
}

/// The merge-base of HEAD with the default branch, kept under the pair of commits it was
/// computed for, so git computes it only when one of them moved. Spec 6.6.
pub fn merge_base(root: &Path, at: &Path, head: &str, default: &str) -> Option<String> {
    let file = at.join(MERGE_BASE);
    if let Some(cached) = Cached::read(&file).filter(|cached| cached.keys(head, default)) {
        return Some(cached.merge_base);
    }
    let found = Repo::at(root)
        .text(&["merge-base", head, default])
        .map(|found| found.trim().to_string())
        .filter(|found| !found.is_empty())?;
    let cached = Cached {
        head: head.to_string(),
        default: default.to_string(),
        merge_base: found,
    };
    cached.write(&file);
    Some(cached.merge_base)
}

/// Where the merge-base cache lives in the state directory.
const MERGE_BASE: &str = "merge-base";

/// The merge-base cache: one merge-base and the pair of commits it was computed for.
struct Cached {
    head: String,
    default: String,
    merge_base: String,
}

impl Cached {
    /// The cache the file holds, and `None` for a file that is gone, torn or of another shape,
    /// which costs one `git merge-base` and nothing else.
    fn read(file: &Path) -> Option<Cached> {
        let held: Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
        let text = |key: &str| Some(held.get(key)?.as_str()?.to_string());
        Some(Cached {
            head: text("head")?,
            default: text("default")?,
            merge_base: text("merge_base")?,
        })
    }

    fn keys(&self, head: &str, default: &str) -> bool {
        self.head == head && self.default == default
    }

    /// A cache klin cannot write costs the next event one `git merge-base`.
    fn write(&self, file: &Path) {
        let held = serde_json::json!({
            "head": self.head,
            "default": self.default,
            "merge_base": self.merge_base,
        });
        let _ = std::fs::write(file, held.to_string() + "\n");
    }
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

/// The findings a stop's block already put in front of the agent under the current stamp. The
/// record lives beside the stamp and not in the build stamp, so a prompt event between two
/// stops keeps it, and it goes when the stamp moves. Empty when no stamp is readable. Spec 8.2.
pub fn asked(root: &Path) -> Vec<String> {
    state::dir(root)
        .and_then(|at| read(&at))
        .map(|held| held.asked)
        .unwrap_or_default()
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
        history: History {
            merge_base: text("merge_base"),
            head_ref: text("head_ref"),
            reflog_position: held
                .get("reflog_position")
                .and_then(Value::as_u64)
                .map(ReflogPosition),
        },
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
        (
            "merge_base",
            stamp.history.merge_base.clone().map(Value::from),
        ),
        ("head_ref", stamp.history.head_ref.clone().map(Value::from)),
        (
            "reflog_position",
            stamp
                .history
                .reflog_position
                .map(|position| position.0.into()),
        ),
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

/// The tree a commit was taken over, and `None` when the name resolves to no commit.
pub fn tree_of(root: &Path, commit: &str) -> Option<String> {
    let refspec = format!("{commit}^{{tree}}");
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
