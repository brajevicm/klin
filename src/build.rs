use std::path::{Path, PathBuf};
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
fn runs(root: &Path, at: &Path, entry: &Entry) -> (String, How) {
    let (tool, args) = entry.run.split_once(' ').unwrap_or((&entry.run, ""));
    let Some((found, how)) = entry.node.then(|| installed(root, at, tool)).flatten() else {
        return (entry.run.clone(), How::Wrote);
    };
    (format!("{found} {args}").trim_end().to_string(), how)
}

/// Where the command one entry runs came from: as the table or a person wrote it, as the binary
/// the project installed, or through the package manager of a Plug'n'Play checkout. Spec 9.3.
#[derive(PartialEq)]
enum How {
    Wrote,
    Installed,
    Manager,
}

/// A JavaScript package manager whose command line klin knows, and how it runs a tool of the
/// project it installed. Yarn's `-B` is its binaries-only form, so a script a person named after
/// the tool cannot stand in for the tool. pnpm's `exec` runs a command with the project's own
/// binaries before any other, which is as near as pnpm states. Spec 9.3.
#[derive(Clone, Copy, PartialEq)]
enum Manager {
    Yarn,
    Pnpm,
}

impl Manager {
    fn runs(self, tool: &str) -> String {
        match self {
            Manager::Yarn => format!("yarn run -B {tool}"),
            Manager::Pnpm => format!("pnpm exec {tool}"),
        }
    }

    /// Whether this checkout holds the tool, asked of the manager itself and answered by its
    /// exit code alone. Yarn names a binary without running it, and answers that it holds none
    /// with the 1 of its usage error. Any other code is a Yarn that could not answer, so the
    /// build runs through it and its own exit decides. pnpm states no such question, so its
    /// checkout is taken at its word. Spec 9.3.
    fn holds(self, at: &Path, tool: &str) -> bool {
        match self {
            Manager::Pnpm => true,
            Manager::Yarn => shell(at, &format!("yarn bin {tool}"))
                .map(|done| done.status.code() != Some(1))
                .unwrap_or(true),
        }
    }

    /// The file this manager writes its own settings in, and the key that names the model.
    fn settings(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Manager::Yarn => &[(".yarnrc.yml", "nodeLinker")],
            Manager::Pnpm => &[
                ("pnpm-workspace.yaml", "nodeLinker"),
                (".npmrc", "node-linker"),
            ],
        }
    }
}

/// What a checkout says about the model it installs a project's tools in. The manager that owns
/// the checkout is the one authority on that, so its settings decide against any file another
/// model left behind. Spec 9.3.
enum Model {
    Play,
    Modules,
}

/// How the checkout runs one tool of a JavaScript project, and `None` when no model klin reads
/// holds it, which leaves the tool on `PATH`. The manager that owns the checkout is read first,
/// then that manager's own settings, both from the directory that holds a Plug'n'Play marker
/// and upward, because a marker says how a project installs its tools and a file below it
/// belongs to another project. Where its settings name no model, the whole chain is
/// searched for an installed binary before a Plug'n'Play marker is read, so a marker an old
/// model left behind cannot take the run from a binary installed above it, and a binary an old
/// model left behind wins the same way: no file tells those two states apart, and a binary is
/// evidence that the tool is there to run. A Plug'n'Play checkout that holds no such tool is a
/// checkout with no project compiler, so the tool on `PATH` runs. Spec 9.3.
fn installed(root: &Path, at: &Path, tool: &str) -> Option<(String, How)> {
    let chain = chain(root, at);
    let marked = chain.iter().position(|(here, _)| marker(here));
    let owns = &chain[marked.unwrap_or_default()..];
    let manager = owns.iter().find_map(|(here, _)| manager(here)).flatten();
    let model = manager.and_then(|manager| owns.iter().find_map(|(here, _)| model(here, manager)));
    match model {
        Some(Model::Play) => through(at, manager?, tool),
        Some(Model::Modules) => bin(&chain, tool),
        None => bin(&chain, tool).or_else(|| marked.and_then(|_| through(at, manager?, tool))),
    }
}

/// The nearest `node_modules/.bin` of the chain that holds the tool, named from the directory
/// the entry runs in.
fn bin(chain: &[(PathBuf, String)], tool: &str) -> Option<(String, How)> {
    let (_, up) = chain
        .iter()
        .find(|(here, _)| here.join(BIN).join(tool).symlink_metadata().is_ok())?;
    Some((format!("{up}{BIN}/{tool}"), How::Installed))
}

/// How one manager runs the tool of its own Plug'n'Play checkout, and `None` when the checkout
/// holds no such tool, which is a checkout with no project compiler. Spec 9.3.
fn through(at: &Path, manager: Manager, tool: &str) -> Option<(String, How)> {
    manager
        .holds(at, tool)
        .then(|| (manager.runs(tool), How::Manager))
}

/// Each directory from the one an entry runs in up to the root klin measures, and the prefix
/// that names it from that directory. The root is searched and nothing above it is.
fn chain(root: &Path, at: &Path) -> Vec<(PathBuf, String)> {
    let mut here = at.to_path_buf();
    let mut up = String::new();
    let mut all = Vec::new();
    loop {
        all.push((here.clone(), up.clone()));
        match here.parent() {
            Some(parent) if here != root => here = parent.to_path_buf(),
            _ => return all,
        }
        up.push_str("../");
    }
}

/// The model one directory's settings name, read from the files that manager writes and no
/// other. A value the manager does not define names no model, which leaves the checkout to say
/// what it installed. Spec 9.3.
fn model(here: &Path, manager: Manager) -> Option<Model> {
    let named = manager
        .settings()
        .iter()
        .find_map(|(file, key)| value(here, file, key))?;
    match named.as_str() {
        "pnp" => Some(Model::Play),
        "node-modules" | "isolated" | "hoisted" => Some(Model::Modules),
        _ => None,
    }
}

/// What one key of a configuration file holds, without the comment or the quotation marks a
/// person may write around it. The key is read where the file begins a line, and the last one
/// the file holds is the one it means.
fn value(here: &Path, file: &str, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(here.join(file)).ok()?;
    let held = text
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
        .filter_map(|line| line.split_once([':', '=']))
        .rfind(|(named, _)| named.trim_end() == key)?
        .1;
    let held = held.split('#').next().unwrap_or_default();
    Some(held.trim().trim_matches(['"', '\'']).to_string())
}

/// Whether one directory holds the file a Plug'n'Play install writes in place of a
/// `node_modules` directory. Yarn 2.4 and later, and pnpm, write `.pnp.cjs`. Yarn 2.0 to 2.3
/// wrote `.pnp.js`, which Yarn Classic writes as well, and only the later Yarn keeps a
/// `.yarnrc.yml` beside it. A Classic checkout is no model klin resolves, because Yarn Classic
/// has no binaries-only form of `yarn run`. Spec 9.3.
fn marker(here: &Path) -> bool {
    here.join(".pnp.cjs").is_file()
        || (here.join(".pnp.js").is_file() && here.join(".yarnrc.yml").is_file())
}

/// Whose checkout one directory is, by the manager it pins, the lockfile it holds, or the
/// configuration file only one manager writes. A pin that names a manager klin has no
/// Plug'n'Play form for answers as well: that directory says the checkout is neither manager's,
/// and nothing above it is asked. `None` is a directory that says nothing. Spec 9.3.
fn manager(here: &Path) -> Option<Option<Manager>> {
    let held = std::fs::read_to_string(here.join(NODE)).unwrap_or_default();
    let held: Value = serde_json::from_str(&held).unwrap_or_default();
    let holds = |name: &str| here.join(name).is_file();
    match held.get("packageManager").and_then(Value::as_str) {
        Some(pinned) if pinned.starts_with("yarn") => Some(Some(Manager::Yarn)),
        Some(pinned) if pinned.starts_with("pnpm") => Some(Some(Manager::Pnpm)),
        Some(_) => Some(None),
        None if holds("pnpm-lock.yaml") => Some(Some(Manager::Pnpm)),
        None if holds("yarn.lock") || holds(".yarnrc.yml") => Some(Some(Manager::Yarn)),
        None => None,
    }
}

/// The directory a JavaScript package manager installs the tools of one project into.
const BIN: &str = "node_modules/.bin";

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
/// entry that ran passed, or None when every entry built, and the line each entry klin resolved
/// against the checkout says it ran. An absent tool skips its own entry and no other, so a
/// compile error behind it still blocks. Spec 9.3.
pub fn failure(root: &Path, wanted: &[&Entry]) -> (Option<Failure>, Vec<Said>) {
    let mut missing = None;
    let mut said = Vec::new();
    let mut failed = None;
    for entry in wanted {
        let (why, line) = built(root, entry);
        said.extend(line.map(|line| (line, None)));
        match why {
            None => (),
            Some(found @ Failure::Missing { .. }) => {
                missing.get_or_insert(found);
            }
            Some(found) => {
                failed = Some(found);
                break;
            }
        }
    }
    (failed.or(missing), said)
}

/// Why one entry did not build, or None when it built, and the line that says what it ran when
/// klin resolved the command against the checkout. A tool the project installed is not an absent
/// tool: a broken install fails its own build rather than leaving the tree unmeasured. Spec 9.3.
fn built(root: &Path, entry: &Entry) -> (Option<Failure>, Option<String>) {
    let at = match &entry.root {
        Some(under) => root.join(under),
        None => root.to_path_buf(),
    };
    let (run, how) = runs(root, &at, entry);
    let line = (run != entry.run).then(|| match &entry.root {
        Some(under) => format!("resolved: {BUILD} {run} in {under}"),
        None => format!("resolved: {BUILD} {run}"),
    });
    (ran(&at, &run, how), line)
}

/// One command in the shell, at the directory it runs in, with Corepack's network disabled so
/// that no package manager klin starts can fetch a version this host does not already hold.
/// Spec 9.3.
fn shell(at: &Path, run: &str) -> std::io::Result<std::process::Output> {
    Command::new("sh")
        .arg("-c")
        .arg(run)
        .current_dir(at)
        .env("COREPACK_ENABLE_NETWORK", "0")
        .output()
}

/// What the shell made of one command, which is the whole test klin applies: it reads no shell
/// message and guesses no tool name. ADR 0048.
fn ran(at: &Path, run: &str, how: How) -> Option<Failure> {
    let done = match shell(at, run) {
        Err(why) => return Some(Failure::Failed(format!("{run}: {why}\n"))),
        Ok(done) => done,
    };
    match done.status.code() {
        Some(0) => None,
        Some(MISSING) if how != How::Installed => Some(Failure::Missing {
            run: run.to_string(),
            output: String::from_utf8_lossy(&done.stderr).trim().to_string(),
        }),
        _ => {
            let mut text = String::from_utf8_lossy(&done.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&done.stderr));
            Some(Failure::Failed(format!("$ {run}\n{text}")))
        }
    }
}
