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

/// The manifest whose tool a JavaScript package manager installs into the tree, which is the one
/// derived command klin resolves against the checkout before it runs. Spec 9.3.
const NODE: &str = "package.json";

const RULE: &str = "one command per manifest";

/// One command that builds part of the tree. An entry with no root covers the whole tree.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub root: Option<String>,
    pub run: String,
    /// Whether a JavaScript package manager installs this entry's tool into the tree, so the run
    /// resolves the installed tool before it runs. A command a person wrote never does. Spec 9.3.
    pub node: bool,
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
            node: false,
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
        node: false,
    })
}

/// One entry per manifest the table names, at the directory that holds it, in path order, and
/// the one `derived:` line that says so. Spec 5.4.
fn derived(project: &Project) -> Plan {
    let manifests = &project.facts().found.manifests;
    let mut found: Vec<(Entry, String)> = manifests
        .iter()
        .filter_map(|path| command(manifests, path))
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
fn command(manifests: &[String], path: &str) -> Option<(Entry, String)> {
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
        (
            Entry {
                root: at,
                run: run.to_string(),
                node: name == NODE,
            },
            origin(path, beside),
        )
    })
}

/// The command one entry runs in this checkout, and whether it is the project's own tool. A
/// derived JavaScript entry runs the tool the package manager installed: from the directory the
/// entry runs in, and then each directory above it up to the root klin measures and never above
/// it, the nearest `node_modules/.bin` that holds the tool names it, and a Yarn Plug'n'Play
/// checkout, which installs no `node_modules`, runs it through Yarn's own binary. Nothing here
/// downloads a tool. The derived value is the command of 5.4 and this resolution is no part of
/// it, so nothing a checkout installs reaches the journal or the build order. Spec 5.4, 9.3.
fn runs(root: &Path, at: &Path, entry: &Entry) -> (String, bool) {
    let (tool, args) = entry.run.split_once(' ').unwrap_or((&entry.run, ""));
    let Some((found, own)) = entry.node.then(|| installed(root, at, tool)).flatten() else {
        return (entry.run.clone(), false);
    };
    (format!("{found} {args}").trim_end().to_string(), own)
}

/// How the checkout runs one tool of a JavaScript project, and whether that command is the
/// project's own tool, which a broken install leaves unable to run. `None` is a checkout that
/// installed none, which leaves the tool on `PATH`.
fn installed(root: &Path, at: &Path, tool: &str) -> Option<(String, bool)> {
    let mut here = at.to_path_buf();
    let mut up = String::new();
    loop {
        if here
            .join("node_modules/.bin")
            .join(tool)
            .symlink_metadata()
            .is_ok()
        {
            return Some((format!("{up}node_modules/.bin/{tool}"), true));
        }
        if PNP.iter().any(|name| here.join(name).is_file()) {
            return Some((format!("yarn exec {tool}"), false));
        }
        match here.parent() {
            Some(parent) if here != root => here = parent.to_path_buf(),
            _ => return None,
        }
        up.push_str("../");
    }
}

/// The files a Yarn Plug'n'Play checkout holds in place of a `node_modules` directory. Yarn 2.0
/// to 2.3 wrote the first name and every later Yarn writes the second.
const PNP: &[&str] = &[".pnp.js", ".pnp.cjs"];

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
/// and no other, so a compile error behind it still blocks. Spec 9.3.
pub fn failure(root: &Path, wanted: &[&Entry]) -> Option<Failure> {
    let mut missing = None;
    for entry in wanted {
        match built(root, entry) {
            None => (),
            Some(found @ Failure::Missing { .. }) => {
                missing.get_or_insert(found);
            }
            Some(failed) => return Some(failed),
        }
    }
    missing
}

/// Why one entry did not build, or None when it built. A tool the project installed is not an
/// absent tool: a broken install fails its own build rather than leaving the tree unmeasured.
/// Spec 9.3.
fn built(root: &Path, entry: &Entry) -> Option<Failure> {
    let at = match &entry.root {
        Some(under) => root.join(under),
        None => root.to_path_buf(),
    };
    let (run, own) = runs(root, &at, entry);
    let done = match Command::new("sh")
        .arg("-c")
        .arg(&run)
        .current_dir(&at)
        .output()
    {
        Err(why) => return Some(Failure::Failed(format!("{run}: {why}\n"))),
        Ok(done) => done,
    };
    match done.status.code() {
        Some(0) => None,
        Some(MISSING) if !own => Some(Failure::Missing {
            run,
            output: String::from_utf8_lossy(&done.stderr).trim().to_string(),
        }),
        _ => {
            let mut text = String::from_utf8_lossy(&done.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&done.stderr));
            Some(Failure::Failed(format!("$ {run}\n{text}")))
        }
    }
}
