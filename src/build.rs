use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::changed::Change;
use crate::check::{self, Said};
use crate::config::{self, Config, Error};
use crate::project::Project;
use crate::scope;
use crate::survey;

const BUILD: &str = config::BUILD.name;
/// The two keys one entry of the `build` list holds, which `config::BUILD` states.
pub const RUN: &str = "run";
pub const ROOT: &str = "root";

/// A manifest names a project klin can build, the file that must sit beside it, and the command
/// that builds it. A manifest with no command builds no project of its own. ADR 0012.
const MANIFESTS: &[(&str, &str, &str)] = &[
    ("Cargo.toml", "", "cargo build --all-targets"),
    ("go.mod", "", "go build ./..."),
    ("package.json", "tsconfig.json", "tsc --noEmit"),
    ("tsconfig.json", "", ""),
];

const RULE: &str = "one command per manifest";

/// One command that builds part of the tree. An entry with no root covers the whole tree.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub root: Option<String>,
    pub run: String,
}

/// Why a build did not pass. `Failed` is the output of the first entry that did not build.
/// `Missing` is an entry whose shell exited 127, which is a command it could not find: the tool
/// is absent, not the code broken, so the tree is unmeasured rather than failing. ADR 0048.
pub enum Failure {
    Failed(String),
    Missing { run: String, output: String },
}

const MISSING: i32 = 127;

/// What a hook run builds, and where the commands came from.
#[derive(Default)]
pub struct Plan {
    pub entries: Vec<Entry>,
    pub said: Vec<Said>,
}

/// The build a person chose, or the one the manifests derive when the config names none. A
/// `false` builds nothing. The derivation reads the tree's manifests and nothing any check
/// derives. Spec 5.2, 5.4, ADR 0012.
pub fn plan(project: &Project) -> Result<Plan, Error> {
    let config = &project.config;
    let shape = || {
        Error(format!(
            "{}: \"{BUILD}\" is a command, a list of {{\"root\", \"run\"}} entries, or false",
            config.file.display()
        ))
    };
    let entries = match config.pinned(BUILD) {
        None => return Ok(derived(project)),
        Some(Value::Bool(false)) => Vec::new(),
        Some(Value::String(run)) => vec![Entry {
            root: None,
            run: run.clone(),
        }],
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| entry(config, item.as_object().ok_or_else(shape)?))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(shape()),
    };
    Ok(Plan {
        entries,
        said: Vec::new(),
    })
}

fn entry(config: &Config, item: &serde_json::Map<String, Value>) -> Result<Entry, Error> {
    let run = item
        .get(RUN)
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(BUILD, RUN))?;
    let root = match item.get(ROOT) {
        None => None,
        Some(Value::String(at)) => Some(at.clone()),
        Some(_) => return Err(config.malformed(BUILD, ROOT, "a directory in the tree")),
    };
    Ok(Entry {
        root,
        run: run.to_string(),
    })
}

/// One entry per manifest the table names, at the directory that holds it, in path order, and
/// the one `derived:` line that says so. Spec 5.4.
fn derived(project: &Project) -> Plan {
    let manifests = &project.facts().found.manifests;
    let mut found: Vec<(Entry, String)> = manifests
        .iter()
        .filter_map(|path| command(project.root(), manifests, path))
        .collect();
    found.sort();
    let Some((first, _)) = found.first() else {
        return Plan::default();
    };
    let value = match (found.len(), &first.root) {
        (1, None) => Value::String(first.run.clone()),
        _ => Value::Array(found.iter().map(|(entry, _)| entry.value()).collect()),
    };
    let runs: Vec<String> = found
        .iter()
        .map(|(entry, from)| format!("{} from {from}", entry.run))
        .collect();
    let line = format!("derived: {BUILD} {}, {RULE}", runs.join(", "));
    let entries = found.into_iter().map(|(entry, _)| entry).collect();
    let said = vec![(line, Some(check::derived_entry(BUILD, None, value, RULE)))];
    Plan { entries, said }
}

impl Entry {
    fn value(&self) -> Value {
        let mut out = serde_json::Map::new();
        if let Some(root) = &self.root {
            out.insert(ROOT.into(), root.clone().into());
        }
        out.insert(RUN.into(), self.run.clone().into());
        Value::Object(out)
    }
}

/// The command a manifest the table names builds with, and the manifest it came from, or
/// nothing for one that builds no project of its own or whose companion file is not beside it.
fn command(root: &Path, manifests: &[String], path: &str) -> Option<(Entry, String)> {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    let (_, beside, run) = MANIFESTS.iter().find(|(held, _, _)| name == *held)?;
    let at = match survey::parent(path) {
        at if at == scope::ROOT => None,
        at => Some(at),
    };
    let companion = match &at {
        None => beside.to_string(),
        Some(at) => format!("{at}/{beside}"),
    };
    let whole = beside.is_empty() || manifests.contains(&companion);
    (!run.is_empty() && whole).then(|| {
        let run = installed(root, at.as_deref(), run);
        (Entry { root: at, run }, origin(path, beside))
    })
}

/// A derived command runs the tool the project installed when one sits in a `node_modules/.bin`
/// at the manifest's directory or above it, up to the root klin measures and never beyond it,
/// and the tool on `PATH` when the project installed none. The path is relative to the directory the
/// entry runs in, so no derived string carries where the tree sits. Nothing here resolves or
/// downloads a tool over the network. Spec 5.4, 9.3.
fn installed(root: &Path, at: Option<&str>, run: &str) -> String {
    let (tool, args) = run.split_once(' ').unwrap_or((run, ""));
    let mut up = String::new();
    let mut here = at.map_or_else(|| root.to_path_buf(), |at| root.join(at));
    loop {
        if runnable(&here.join("node_modules/.bin").join(tool)) {
            return format!("{up}node_modules/.bin/{tool} {args}")
                .trim_end()
                .to_string();
        }
        match here.parent() {
            Some(parent) if here != root => here = parent.to_path_buf(),
            _ => return run.to_string(),
        }
        up.push_str("../");
    }
}

/// Whether a path is a file this host can execute. A file without the bit is not the tool: a
/// shell that cannot run it exits 126, which ADR 0048 does not read as an absent tool, so the
/// `PATH` tool is the better command.
fn runnable(at: &Path) -> bool {
    let Ok(held) = at.metadata() else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        held.is_file() && held.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        held.is_file()
    }
}

/// The manifest a derived command came from, as the `derived:` line and a failing build name it.
fn origin(path: &str, beside: &str) -> String {
    match beside.is_empty() {
        true => path.to_string(),
        false => format!("{path} beside {beside}"),
    }
}

/// The entries a run builds. Without a changed set that is every entry. With one it is the
/// entries whose root holds a changed file, and every entry when a changed file is under none,
/// because klin cannot know what that file affects.
pub fn wanted<'a>(entries: &'a [Entry], changes: Option<&[Change]>) -> Vec<&'a Entry> {
    let every = || entries.iter().collect();
    let Some(changes) = changes else {
        return every();
    };
    let mut picked = vec![false; entries.len()];
    for name in changes.iter().flat_map(named) {
        let holders: Vec<usize> = (0..entries.len())
            .filter(|at| holds(&entries[*at], name))
            .collect();
        if holders.is_empty() {
            return every();
        }
        for at in holders {
            picked[at] = true;
        }
    }
    entries
        .iter()
        .enumerate()
        .filter(|(at, _)| picked[*at])
        .map(|(_, entry)| entry)
        .collect()
}

/// Both names a change carries. A rename out of a root leaves that root a source short, so the
/// root it left is built as well as the one it landed in.
fn named(change: &Change) -> impl Iterator<Item = &str> {
    std::iter::once(change.path.as_str()).chain(change.was.as_deref())
}

fn holds(entry: &Entry, path: &str) -> bool {
    match &entry.root {
        None => true,
        Some(root) => path.starts_with(&format!("{}/", root.trim_end_matches('/'))),
    }
}

/// The first entry that failed, or the first whose command the shell could not find when every
/// entry that ran passed, or None when every entry built. An absent tool skips its own entry
/// and no other, so a compile error behind it still blocks.
pub fn failure(root: &Path, wanted: &[&Entry]) -> Option<Failure> {
    let mut missing = None;
    for entry in wanted {
        let at = match &entry.root {
            Some(under) => root.join(under),
            None => root.to_path_buf(),
        };
        let done = Command::new("sh")
            .arg("-c")
            .arg(&entry.run)
            .current_dir(&at)
            .output();
        match done {
            Err(why) => return Some(Failure::Failed(format!("{}: {why}\n", entry.run))),
            Ok(done) if done.status.code() == Some(MISSING) => {
                missing.get_or_insert(Failure::Missing {
                    run: entry.run.clone(),
                    output: String::from_utf8_lossy(&done.stderr).trim().to_string(),
                });
            }
            Ok(done) if !done.status.success() => {
                let mut text = String::from_utf8_lossy(&done.stdout).into_owned();
                text.push_str(&String::from_utf8_lossy(&done.stderr));
                return Some(Failure::Failed(format!("$ {}\n{text}", entry.run)));
            }
            Ok(_) => (),
        }
    }
    missing
}
