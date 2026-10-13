//! App mode of `reachability`. In a TypeScript package whose manifest states no entry point (ADR
//! 0044) and depends on `next`, a file is reached when the module graph leads to it from a file
//! Next.js or a package script runs, and an export is used when a reached file imports it. The
//! walk follows imports, re-exports and literal `import()` calls, type-only ones included, so a
//! re-export reaches its target only when the file that writes it is reached (ADR 0061, amended).
//! A package with no entry point and no framework klin recognizes is a hole and judges nothing,
//! and so is an app package where klin cannot prove an alias configuration. Spec 8.4.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::rc::Rc;

use serde_json::Value;

use crate::config::scope::{Scope, under_or_at};
use crate::contract::coverage::Unresolved;
use crate::facts::survey;
use crate::modules::resolver::{Doubt, Topology, directory, joined};
use crate::modules::{self, Aliases, ModuleGraph};
use crate::surface;
use crate::syntax::LanguageId;
use crate::syntax::structural::facts::Visibility;
use crate::syntax::structural::{self, facts::DeclarationKind, facts::FileFacts};

const MANIFEST: &str = "package.json";
const KIT: &str = "components.json";
const FRAMEWORK: &str = "next";
const DEPENDENCIES: &[&str] = &["dependencies", "devDependencies", "peerDependencies"];
const EVERY: &str = "*";
const DEFAULT: &str = "default";
const SCRIPTED: &[&str] = &[".ts", ".tsx", ".mts", ".cts"];

/// The basenames, less their extension, of the files Next.js reads under `app/`.
const CONVENTIONS: &[&str] = &[
    "page",
    "layout",
    "template",
    "loading",
    "error",
    "global-error",
    "not-found",
    "global-not-found",
    "forbidden",
    "unauthorized",
    "default",
    "route",
    "opengraph-image",
    "twitter-image",
    "icon",
    "apple-icon",
    "sitemap",
    "robots",
    "manifest",
];

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const METADATA: &[&str] = &[
    "metadata",
    "generateMetadata",
    "viewport",
    "generateViewport",
];
const SEGMENT: &[&str] = &[
    "generateStaticParams",
    "dynamic",
    "dynamicParams",
    "revalidate",
    "runtime",
    "fetchCache",
    "preferredRegion",
    "maxDuration",
];
const IMAGE: &[&str] = &["alt", "size", "contentType", "generateImageMetadata"];
const PAGES: &[&str] = &[
    "getServerSideProps",
    "getStaticProps",
    "getStaticPaths",
    "config",
];

/// The exports Next.js reads from each convention file beside its default export.
const CONVENTION_EXPORTS: &[(&str, &[&[&str]])] = &[
    ("route", &[METHODS, SEGMENT]),
    ("page", &[METADATA, SEGMENT]),
    ("layout", &[METADATA, SEGMENT]),
    ("opengraph-image", &[IMAGE]),
    ("twitter-image", &[IMAGE]),
    ("icon", &[IMAGE]),
    ("apple-icon", &[IMAGE]),
    ("sitemap", &[&["generateSitemaps"]]),
];

/// What kind of entry Next.js or a script makes of a file, which decides the exports it reads.
#[derive(Clone, Copy)]
enum Kind {
    Convention(&'static str),
    Pages,
    Middleware,
    Instrumentation,
    InstrumentationClient,
    Program,
}

impl Kind {
    /// Whether the runtime reads this export from a file of this kind.
    fn reads(self, name: &str) -> bool {
        let read: &[&[&str]] = match self {
            Kind::Convention(stem) => CONVENTION_EXPORTS
                .iter()
                .find(|(held, _)| *held == stem)
                .map_or(&[], |(_, read)| read),
            Kind::Program => &[],
            Kind::Pages => &[PAGES],
            Kind::Middleware => &[&["config", "middleware", "proxy"]],
            Kind::Instrumentation => &[&["register", "onRequestError"]],
            Kind::InstrumentationClient => &[&["onRouterTransitionStart"]],
        };
        name == DEFAULT || read.iter().any(|names| names.contains(&name))
    }
}

/// One package manifest: where it is, whether it states no entry point, whether it depends on
/// Next.js, and the commands its scripts run.
struct Package {
    manifest: String,
    directory: String,
    app: bool,
    next: bool,
    scripts: Vec<String>,
}

impl Package {
    fn read(manifest: &str, bytes: Option<Vec<u8>>) -> Package {
        let parsed: Option<Value> = bytes.and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let fields = parsed.as_ref().and_then(Value::as_object);
        let depends = |field: &&str| {
            fields
                .and_then(|held| held.get(*field)?.as_object())
                .is_some_and(|held| held.contains_key(FRAMEWORK))
        };
        Package {
            manifest: manifest.to_string(),
            directory: directory(manifest).to_string(),
            app: fields.is_some_and(|held| !surface::declares_entry(held)),
            next: DEPENDENCIES.iter().any(depends),
            scripts: fields
                .and_then(|held| held.get("scripts")?.as_object())
                .into_iter()
                .flatten()
                .filter_map(|(_, command)| command.as_str().map(str::to_string))
                .collect(),
        }
    }

    fn judged(&self) -> bool {
        self.app && self.next
    }
}

fn packages(files: &[String], read: impl Fn(&str) -> Option<Vec<u8>>) -> Vec<Package> {
    files
        .iter()
        .filter(|file| basename(file) == MANIFEST && survey::surveyed(file))
        .map(|manifest| Package::read(manifest, read(manifest)))
        .collect()
}

/// Whether a tree holds a package app mode judges, read from its manifests alone.
pub(super) fn present(root: &Path, files: &[String]) -> bool {
    packages(files, |file| std::fs::read(root.join(file)).ok())
        .iter()
        .any(Package::judged)
}

/// One tree as app mode reads it: its root, the files it lists and their facts, and the base
/// path of every file a rename moved, as the module graph takes them.
pub(super) struct Side<'a> {
    pub root: &'a Path,
    pub files: &'a [String],
    pub facts: &'a [Rc<FileFacts>],
    pub renamed: &'a HashMap<String, String>,
}

/// One export of a reached file that no other reached file imports.
pub(super) struct Unused {
    pub file: String,
    pub line: u64,
    pub name: String,
}

/// What app mode found in one tree, under current paths: each judged file that is no entry with
/// whether no entry reaches it, each unused export, how many exports it judged, every file of an
/// app package, which no family judges, the files it judged, entries included, the files the
/// section's scope took out, and the holes that kept a package from being judged.
#[derive(Default)]
pub(super) struct Judged {
    pub files: Vec<(String, bool)>,
    pub unused: Vec<Unused>,
    pub exports: usize,
    pub owned: HashSet<String>,
    pub judged: HashSet<String>,
    pub excluded: HashSet<String>,
    pub holes: Vec<Unresolved>,
}

pub(super) fn judged(side: Side, scope: &Scope) -> Judged {
    let topology = modules::topology(side.root, side.files, side.facts, side.renamed);
    let current: HashMap<&str, &str> = side
        .renamed
        .iter()
        .map(|(now, was)| (was.as_str(), now.as_str()))
        .collect();
    let current = |file: &str| current.get(file).copied().unwrap_or(file).to_string();
    let packages = packages(topology.files(), |file| topology.read(file));
    let owned = owned(&topology, &packages);
    let mut out = Judged::default();
    for (at, files) in &owned {
        let package = &packages[*at];
        let selected = files.iter().any(|file| scope.selects(&current(file)));
        if package.app && !package.next && selected {
            out.holes.push(Unresolved {
                file: current(&package.manifest),
                line: 0,
                text: format!("{} TypeScript file(s)", files.len()),
                why: "a package with no entry point whose framework klin does not recognize"
                    .to_string(),
            });
        }
    }
    if !packages.iter().any(Package::judged) {
        return out;
    }
    let graph = modules::build(&topology);
    let walk = Walk::of(&topology, &graph);
    for (at, files) in &owned {
        let package = &packages[*at];
        if package.judged() {
            out.owned.extend(files.iter().map(|file| current(file)));
            let app = App {
                walk: &walk,
                package,
                files,
                scope,
                current: &current,
            };
            app.judge(&mut out);
        }
    }
    out
}

/// The TypeScript files each package owns, by the package's position: the deepest package
/// directory that holds a file owns it. A file under a hidden or skipped directory is no
/// package's, as no structural check measures it.
fn owned<'t>(topology: &'t Topology, packages: &[Package]) -> BTreeMap<usize, Vec<&'t str>> {
    let mut out: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for file in topology.files() {
        if structural::language_of(file) != Some(LanguageId::TypeScript) || !survey::surveyed(file)
        {
            continue;
        }
        let deepest = packages
            .iter()
            .enumerate()
            .filter(|(_, package)| within(&package.directory, file))
            .max_by_key(|(_, package)| package.directory.len());
        if let Some((at, _)) = deepest {
            out.entry(at).or_default().push(file);
        }
    }
    out
}

/// The module graph of one tree as the walk reads it: each TypeScript file's module, the modules
/// each module leads to, and the modules each import site leads to. An import that may name any
/// of several held files leads to each of them, and one that names no held file to none.
struct Walk<'g> {
    topology: &'g Topology<'g>,
    module_of: HashMap<&'g str, usize>,
    next: Vec<Vec<usize>>,
    sites: HashMap<(usize, u64), Vec<usize>>,
    unproved: Vec<&'g modules::resolver::Hole>,
}

impl<'g> Walk<'g> {
    fn of(topology: &'g Topology<'g>, graph: &'g ModuleGraph) -> Walk<'g> {
        let module_of: HashMap<&str, usize> = graph
            .modules
            .iter()
            .enumerate()
            .filter(|(_, module)| module.target.is_none() && module.nesting.is_empty())
            .flat_map(|(at, module)| module.sources.iter().map(move |file| (file.as_str(), at)))
            .collect();
        let mut walk = Walk {
            topology,
            module_of,
            next: vec![Vec::new(); graph.modules.len()],
            sites: HashMap::new(),
            unproved: Vec::new(),
        };
        for dependency in &graph.dependencies {
            walk.lead(dependency.from, dependency.start_byte, dependency.to);
        }
        for hole in &graph.holes {
            walk.doubted(hole);
        }
        walk
    }

    fn lead(&mut self, from: usize, start_byte: u64, to: usize) {
        self.next[from].push(to);
        self.sites.entry((from, start_byte)).or_default().push(to);
    }

    fn doubted(&mut self, hole: &'g modules::resolver::Hole) {
        let Some(from) = self.module_of.get(hole.file.as_str()).copied() else {
            return;
        };
        match &hole.doubt {
            Doubt::Several(files) => {
                for file in files {
                    if let Some(to) = self.module_of.get(file.as_str()).copied() {
                        self.lead(from, hole.start_byte, to);
                    }
                }
            }
            Doubt::Unproved if hole.local_alias => self.unproved.push(hole),
            Doubt::Unproved | Doubt::Missing => {}
        }
    }

    /// Which modules the entries lead to, the entries included.
    fn reached(&self, entries: impl Iterator<Item = usize>) -> Vec<bool> {
        let mut reached = vec![false; self.next.len()];
        let mut queue: VecDeque<usize> = VecDeque::new();
        for entry in entries {
            if !reached[entry] {
                reached[entry] = true;
                queue.push_back(entry);
            }
        }
        while let Some(at) = queue.pop_front() {
            for to in &self.next[at] {
                if !reached[*to] {
                    reached[*to] = true;
                    queue.push_back(*to);
                }
            }
        }
        reached
    }

    /// What the reached files import from other modules: every name of a module a namespace
    /// import, a star or an `import()` takes, and each name a named import or re-export takes.
    fn used(&self, reached: &[bool]) -> Used<'g> {
        let mut used = Used::default();
        for (file, from) in &self.module_of {
            let Some(facts) = self.topology.facts(file).filter(|_| reached[*from]) else {
                continue;
            };
            for import in &facts.imports {
                let targets = self.sites.get(&(*from, import.start_byte));
                for to in targets.into_iter().flatten().filter(|to| *to != from) {
                    used.take(*to, &import.paths);
                }
            }
        }
        used
    }
}

#[derive(Default)]
struct Used<'g> {
    every: HashSet<usize>,
    named: HashSet<(usize, &'g str)>,
}

impl<'g> Used<'g> {
    fn take(&mut self, module: usize, names: &'g [String]) {
        if names.iter().any(|name| name == EVERY) {
            self.every.insert(module);
        }
        self.named
            .extend(names.iter().map(|name| (module, name.as_str())));
    }

    fn holds(&self, module: usize, name: &str) -> bool {
        self.every.contains(&module) || self.named.contains(&(module, name))
    }
}

/// One app package judged: its entries, the files they reach, and the exports reached files take.
struct App<'a, 'g> {
    walk: &'a Walk<'g>,
    package: &'a Package,
    files: &'a [&'g str],
    scope: &'a Scope,
    current: &'a dyn Fn(&str) -> String,
}

impl App<'_, '_> {
    fn judge(&self, out: &mut Judged) {
        let unproved = self.unproved();
        if !unproved.is_empty() {
            out.holes.extend(unproved);
            return;
        }
        let kinds = self.kinds();
        let entries = kinds
            .keys()
            .filter_map(|file| self.walk.module_of.get(file).copied());
        let reached = self.walk.reached(entries);
        let used = self.walk.used(&reached);
        let kit = self.kit();
        for file in self.files.iter().copied().filter(|file| judgeable(file)) {
            let now = (self.current)(file);
            let Some(module) = self.walk.module_of.get(file).copied() else {
                continue;
            };
            if !self.scope.selects(&now) {
                out.excluded.insert(now);
                continue;
            }
            out.judged.insert(now.clone());
            let kind = kinds.get(file).copied();
            if kind.is_none() {
                out.files.push((now.clone(), !reached[module]));
            }
            let kit = kit.as_deref().is_some_and(|kit| under_or_at(file, kit));
            if reached[module] && !kit {
                self.exports(file, (module, kind), &used, now, out);
            }
        }
    }

    /// What keeps the package from being judged: an alias configuration klin cannot prove, and a
    /// file the grammar refused, whose imports are unknown.
    fn unproved(&self) -> Vec<Unresolved> {
        let aliases = self
            .walk
            .unproved
            .iter()
            .filter(|hole| self.files.contains(&hole.file.as_str()))
            .map(|hole| Unresolved {
                file: (self.current)(&hole.file),
                line: hole.line,
                text: hole.text.clone(),
                why: hole.why.clone(),
            });
        let unread = self
            .files
            .iter()
            .filter(|file| self.walk.topology.facts(file).is_none())
            .map(|file| Unresolved {
                file: (self.current)(file),
                line: 0,
                text: (self.current)(file),
                why: "a file klin could not parse, so what it imports is unknown".to_string(),
            });
        aliases.chain(unread).collect()
    }

    /// The package's entries, each with the kind of entry it is.
    fn kinds(&self) -> HashMap<&str, Kind> {
        let scripted = self.scripted();
        self.files
            .iter()
            .copied()
            .filter(|file| judgeable(file))
            .filter_map(|file| {
                let relative = relative(&self.package.directory, file);
                let kind =
                    kind(relative).or_else(|| scripted.contains(file).then_some(Kind::Program))?;
                Some((file, kind))
            })
            .collect()
    }

    /// The TypeScript files the package's scripts name, joined to its directory.
    fn scripted(&self) -> HashSet<String> {
        let separator = |c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '(' | ')');
        self.package
            .scripts
            .iter()
            .flat_map(|command| command.split(separator))
            .map(|word| word.trim_matches(['"', '\'']))
            .filter_map(|word| word.rsplit('=').next())
            .filter(|word| SCRIPTED.iter().any(|end| word.ends_with(end)))
            .filter_map(|word| joined(&self.package.directory, word))
            .filter(|file| self.walk.topology.holds(file))
            .collect()
    }

    /// The directory a `components.json` beside the manifest names as its `ui` alias, resolved
    /// through the package's own paths rules, and none where it names none klin can prove.
    fn kit(&self) -> Option<String> {
        let file = joined(&self.package.directory, KIT)?;
        let topology = self.walk.topology;
        let ui = topology
            .holds(&file)
            .then(|| topology.read(&file))
            .flatten()
            .and_then(|bytes| ui_alias(&bytes))?;
        let paths = Aliases::read(topology);
        self.files.iter().find_map(|from| paths.resolve(from, &ui))
    }

    fn exports(
        &self,
        file: &str,
        (module, kind): (usize, Option<Kind>),
        used: &Used,
        now: String,
        out: &mut Judged,
    ) {
        let Some(facts) = self.walk.topology.facts(file) else {
            return;
        };
        for (name, line) in exports(facts) {
            out.exports += 1;
            if !used.holds(module, &name) && !kind.is_some_and(|kind| kind.reads(&name)) {
                out.unused.push(Unused {
                    file: now.clone(),
                    line,
                    name,
                });
            }
        }
    }
}

/// The alias a `components.json` names for its kit: `aliases.ui`.
fn ui_alias(bytes: &[u8]) -> Option<String> {
    let parsed: Value = serde_json::from_slice(bytes).ok()?;
    Some(parsed.pointer("/aliases/ui")?.as_str()?.to_string())
}

/// Every name a file exports at its top, with the line that first exports it: its exported
/// declarations, by the name a consumer addresses each by, and every name an export clause, a
/// default export or a re-export names. A bare star names none.
fn exports(facts: &FileFacts) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for (name, line) in declared(facts) {
        out.entry(name.to_string()).or_insert(line);
    }
    for export in facts.exports.iter().filter(|export| export.supported) {
        for name in export.leaves.iter().filter_map(|leaf| leaf.name.as_deref()) {
            out.entry(name.to_string()).or_insert(export.line);
        }
    }
    out
}

/// Each exported declaration of a file by the name a consumer addresses it by, with its line.
fn declared(facts: &FileFacts) -> impl Iterator<Item = (&str, u64)> {
    facts
        .declarations
        .iter()
        .filter(|declaration| {
            declaration.visibility == Visibility::Public
                && declaration.kind != DeclarationKind::Method
                && !declaration.associated
        })
        .flat_map(|declaration| {
            let names: Vec<&str> = match declaration.exported_as.as_deref() {
                Some(name) => vec![name],
                None => declaration.names().collect(),
            };
            names.into_iter().map(|name| (name, declaration.line))
        })
}

/// The kind of entry Next.js makes of a file at this path under its package, and none for an
/// ordinary module.
fn kind(relative: &str) -> Option<Kind> {
    let (stem, _) = basename(relative).rsplit_once('.')?;
    if let Some(kind) = rooted(stem, directory(relative)) {
        return Some(kind);
    }
    if under_or_at(relative, "app") || under_or_at(relative, "src/app") {
        return CONVENTIONS
            .iter()
            .find(|held| **held == stem)
            .map(|held| Kind::Convention(held));
    }
    (under_or_at(relative, "pages") || under_or_at(relative, "src/pages")).then_some(Kind::Pages)
}

/// The kind of a file Next.js reads at the package root or under `src/`, by its name.
fn rooted(stem: &str, at: &str) -> Option<Kind> {
    let root = matches!(at, "" | "src");
    match stem {
        "middleware" | "proxy" if root => Some(Kind::Middleware),
        "instrumentation" if root => Some(Kind::Instrumentation),
        "instrumentation-client" if root => Some(Kind::InstrumentationClient),
        "next.config" if at.is_empty() => Some(Kind::Program),
        _ => None,
    }
}

/// Whether a file is one app mode judges: neither a test, by the test convention of 8.2, nor a
/// declaration file.
fn judgeable(file: &str) -> bool {
    let name = basename(file);
    !survey::marked(file)
        && !name.ends_with(".d.ts")
        && !name.ends_with(".d.mts")
        && !name.ends_with(".d.cts")
}

fn relative<'f>(directory: &str, file: &'f str) -> &'f str {
    match directory {
        "" => file,
        at => file
            .strip_prefix(at)
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or(file),
    }
}

/// Whether a package directory holds a file, the tree root holding every file.
fn within(directory: &str, file: &str) -> bool {
    directory.is_empty() || under_or_at(file, directory)
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}
