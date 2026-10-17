//! App mode of `reachability`. In a TypeScript package whose manifest states no entry point (ADR
//! 0044) and depends on `next`, or on `vite` beside an `index.html`, a file is reached when the
//! module graph leads to it from a file Next.js, the `index.html` or a package script runs, and an
//! export is used when a reached file imports it. The
//! walk follows imports, re-exports and literal `import()` calls, type-only ones included, so a
//! re-export reaches its target only when the file that writes it is reached (ADR 0061, amended).
//! A package with no entry point and no framework klin recognizes is a hole and judges nothing,
//! and so is an app package where klin cannot prove an alias configuration. Spec 8.4.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::rc::Rc;

use serde_json::Value;

use super::page::{self, Unread};
use crate::config::scope::{Scope, under_or_at};
use crate::contract::coverage::Unresolved;
use crate::facts::{files, survey};
use crate::modules::resolver::{Doubt, Topology, directory, joined};
use crate::modules::{self, Aliases, GraphCost, ModuleGraph};
use crate::surface;
use crate::syntax::LanguageId;
use crate::syntax::structural::facts::{Declaration, Visibility};
use crate::syntax::structural::{self, facts::DeclarationKind, facts::FileFacts};

const MANIFEST: &str = "package.json";
const KIT: &str = "components.json";
const FRAMEWORK: &str = "next";
const VITE: &str = "vite";
const PAGE: &str = "index.html";
/// The single-file components a Vite app may hold whose imports klin does not read.
const UNREAD: &[&str] = &[".vue", ".svelte", ".astro"];
const DEPENDENCIES: &[&str] = &["dependencies", "devDependencies", "peerDependencies"];
/// The base tree, which every exemption app mode grants is read from: its `.gitattributes`
/// marks, its `components.json` and the paths rules that resolve it, its package scripts, and the
/// convention files it held that a leading `_` turns off. A change cannot widen its own
/// exemptions: what it writes there counts once a person makes it part of the base. The marks
/// and the kit apply where the base holds them, to each file by the path the base holds it at, so
/// a rename moves no exemption onto other files.
pub(super) struct Base<'a> {
    root: &'a Path,
    files: &'a [String],
    held: HashSet<&'a str>,
    renamed: &'a HashMap<String, String>,
}

impl<'a> Base<'a> {
    pub(super) fn of(
        root: &'a Path,
        files: &'a [String],
        renamed: &'a HashMap<String, String>,
    ) -> Base<'a> {
        Base {
            root,
            files,
            held: files.iter().map(String::as_str).collect(),
            renamed,
        }
    }

    /// The path the base holds a file at, by the path it has today.
    fn was<'f>(&'f self, now: &'f str) -> &'f str {
        self.renamed.get(now).map_or(now, String::as_str)
    }

    /// The base's TypeScript paths rules.
    fn aliases(&self) -> Aliases {
        Aliases::read(&modules::topology(self.root, self.files, &[], self.renamed))
    }

    fn holds(&self, file: &str) -> bool {
        self.held.contains(file)
    }

    fn read(&self, file: &str) -> Option<Vec<u8>> {
        self.holds(file)
            .then(|| std::fs::read(self.root.join(file)).ok())
            .flatten()
    }

    /// The in-tree `.gitattributes` texts, shallowest first, whose `linguist-vendored` and
    /// `linguist-generated` marks leave files out of both trees' judgement.
    fn marks(&self) -> Vec<(String, String)> {
        let mut held: Vec<&str> = self
            .held
            .iter()
            .copied()
            .filter(|file| basename(file) == ".gitattributes")
            .collect();
        held.sort_by_key(|file| (file.matches('/').count(), *file));
        held.into_iter()
            .filter_map(|file| {
                let text = files::attribute_text(&self.read(file)?);
                Some((self.was(file).to_string(), text))
            })
            .collect()
    }

    /// The commands the scripts of the base's copy of this manifest run.
    fn scripts(&self, manifest: &str) -> Vec<String> {
        let parsed: Option<Value> = self
            .read(manifest)
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        parsed
            .as_ref()
            .and_then(|held| held.get("scripts")?.as_object())
            .into_iter()
            .flatten()
            .filter_map(|(_, command)| command.as_str().map(str::to_string))
            .collect()
    }
}

/// The UI frameworks whose packages are apps: a package with no entry point that depends on one
/// of these and not on `next` is an app klin does not judge, and says so.
const UI_FRAMEWORKS: &[&str] = &[
    "react",
    "vue",
    "svelte",
    "solid-js",
    "preact",
    "react-router",
    "@remix-run/react",
    "astro",
    "nuxt",
    "@sveltejs/kit",
    "expo",
    "react-native",
];
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

/// What a package manifest makes of its package: a Next.js app, a Vite app with the files its
/// `index.html` module scripts name, an app klin does not judge with why, or anything else, such
/// as a library that states an entry point, a command a `bin` names, a workspace root, a package
/// of tools, or a manifest klin cannot read. A package with an entry point is no app, nor, unless
/// it is a Next or Vite app, one with a `bin` or `workspaces`. ADR 0044.
#[derive(PartialEq, Eq)]
enum Role {
    Library,
    NextApp,
    ViteApp(Vec<String>),
    Unjudged(&'static str),
}

const UNRECOGNIZED: &str = "a package with no entry point whose framework klin does not recognize";
const VITE_UNREAD: &str = "a Vite app that holds .vue, .svelte or .astro files klin does not read";
const VITE_INLINE: &str =
    "a Vite app whose index.html holds an inline module script klin does not read";
const VITE_PLACEHOLDER: &str =
    "a Vite app whose index.html names a module through a %...% placeholder klin does not expand";
const VITE_UNNAMED: &str =
    "a Vite app whose index.html names no TypeScript module the package holds";

/// One package manifest: where it is, and what it makes of its package.
struct Package {
    manifest: String,
    directory: String,
    role: Role,
}

impl Package {
    /// A package by its manifest alone, a Vite app's entries still unread.
    fn read(manifest: &str, bytes: Option<Vec<u8>>, paged: bool) -> Package {
        let parsed: Option<Value> = bytes.and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let fields = parsed.as_ref().and_then(Value::as_object);
        let depends = |names: &[&str]| {
            DEPENDENCIES.iter().any(|field| {
                fields
                    .and_then(|held| held.get(*field)?.as_object())
                    .is_some_and(|held| names.iter().any(|name| held.contains_key(*name)))
            })
        };
        let entryless = fields.is_some_and(|held| !surface::declares_entry(held));
        let tool =
            fields.is_some_and(|held| held.contains_key("bin") || held.contains_key("workspaces"));
        let role = match entryless {
            true if depends(&[FRAMEWORK]) => Role::NextApp,
            true if paged && depends(&[VITE]) => Role::ViteApp(Vec::new()),
            true if !tool && depends(UI_FRAMEWORKS) => Role::Unjudged(UNRECOGNIZED),
            _ => Role::Library,
        };
        Package {
            manifest: manifest.to_string(),
            directory: directory(manifest).to_string(),
            role,
        }
    }

    fn judged(&self) -> bool {
        matches!(self.role, Role::NextApp | Role::ViteApp(_))
    }

    fn page(&self) -> Option<String> {
        joined(&self.directory, PAGE)
    }

    /// The role of a Vite app once its `index.html` and the files it owns are read: the
    /// TypeScript files its module scripts name, or the first reason klin cannot judge it.
    fn vite(&self, html: &str, owned: &[&str]) -> Role {
        if owned
            .iter()
            .any(|file| UNREAD.iter().any(|end| file.ends_with(end)))
        {
            return Role::Unjudged(VITE_UNREAD);
        }
        let at = self.page().map(|page| directory(&page).to_string());
        let named = match page::named(html, &self.directory, at.as_deref().unwrap_or_default()) {
            Ok(named) => named,
            Err(Unread::Inline) => return Role::Unjudged(VITE_INLINE),
            Err(Unread::Placeholder) => return Role::Unjudged(VITE_PLACEHOLDER),
        };
        let named: Vec<String> = named
            .into_iter()
            .filter(|file| owned.contains(&file.as_str()) && typescript(file))
            .collect();
        match named.is_empty() {
            true => Role::Unjudged(VITE_UNNAMED),
            false => Role::ViteApp(named),
        }
    }
}

/// Every package manifest of a tree with its role. A Vite app's role is read from the
/// `index.html` this tree holds, as a module script ships its file and grants no exemption.
fn packages(files: &[String], read: impl Fn(&str) -> Option<Vec<u8>>) -> Vec<Package> {
    let held: HashSet<&str> = files.iter().map(String::as_str).collect();
    let mut packages: Vec<Package> = files
        .iter()
        .filter(|file| basename(file) == MANIFEST && survey::surveyed(file))
        .map(|manifest| {
            let page = joined(directory(manifest), PAGE);
            let paged = page.is_some_and(|page| held.contains(page.as_str()));
            Package::read(manifest, read(manifest), paged)
        })
        .collect();
    if !packages
        .iter()
        .any(|package| matches!(package.role, Role::ViteApp(_)))
    {
        return packages;
    }
    let mut owned: HashMap<usize, Vec<&str>> = HashMap::new();
    for file in files.iter().filter(|file| survey::surveyed(file)) {
        if let Some(at) = owner(&packages, file) {
            owned.entry(at).or_default().push(file);
        }
    }
    for (at, package) in packages.iter_mut().enumerate() {
        if !matches!(package.role, Role::ViteApp(_)) {
            continue;
        }
        let html = package
            .page()
            .and_then(|page| read(&page))
            .unwrap_or_default();
        let owned = owned.get(&at).map_or(&[][..], Vec::as_slice);
        package.role = package.vite(&String::from_utf8_lossy(&html), owned);
    }
    packages
}

/// Whether a tree holds a package app mode judges, read from its manifests alone.
pub(super) fn present(root: &Path, files: &[String]) -> bool {
    packages(files, |file| std::fs::read(root.join(file)).ok())
        .iter()
        .any(Package::judged)
}

/// One tree as app mode reads it: its root, the files it lists and the facts of every structural
/// file, the base path of every file a rename moved, as the module graph takes them, and the
/// tree's module graph, which every gate that reads it shares.
pub(super) struct Side<'a> {
    pub root: &'a Path,
    pub files: &'a [String],
    pub facts: &'a [Rc<FileFacts>],
    pub renamed: &'a HashMap<String, String>,
    pub graph: &'a modules::Shared,
    /// The base tree, which the exemptions are read from.
    pub base: &'a Base<'a>,
}

/// One file app mode judged that is no entry, and whether no entry reaches it.
pub(super) struct JudgedFile {
    pub file: String,
    pub unreached: bool,
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
    pub files: Vec<JudgedFile>,
    pub unused: Vec<Unused>,
    pub dormant: Vec<Unused>,
    pub exports: usize,
    pub owned: HashSet<String>,
    pub judged: HashSet<String>,
    pub excluded: HashSet<String>,
    pub holes: Vec<Unresolved>,
    /// What reading the tree's module graph cost, and none where no app package asked for it.
    pub graph: Option<GraphCost>,
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
        if let Role::Unjudged(why) = package.role
            && selected
        {
            out.holes.push(Unresolved {
                file: current(&package.manifest),
                line: 0,
                text: format!("{} TypeScript file(s)", files.len()),
                why: why.to_string(),
            });
        }
    }
    if !packages.iter().any(Package::judged) {
        return out;
    }
    let (graph, cost) = side.graph.get(&topology);
    out.graph = Some(cost);
    let marks = side.base.marks();
    let walk = Walk::of(&topology, &graph, &marks);
    let used = walk.used();
    for (at, files) in &owned {
        let package = &packages[*at];
        if package.judged() {
            out.owned.extend(files.iter().map(|file| current(file)));
            let app = App {
                base: side.base,
                walk: &walk,
                used: &used,
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
        if !typescript(file) || !survey::surveyed(file) {
            continue;
        }
        if let Some(at) = owner(packages, file) {
            out.entry(at).or_default().push(file);
        }
    }
    out
}

/// The position of the package that owns a file: the deepest package directory that holds it.
fn owner(packages: &[Package], file: &str) -> Option<usize> {
    packages
        .iter()
        .enumerate()
        .filter(|(_, package)| within(&package.directory, file))
        .max_by_key(|(_, package)| package.directory.len())
        .map(|(at, _)| at)
}

fn typescript(file: &str) -> bool {
    structural::language_of(file) == Some(LanguageId::TypeScript)
}

/// The module graph of one tree as the walk reads it: each TypeScript file's module, the modules
/// each module leads to, and the modules each import site leads to. An import that may name any
/// of several held files leads to each of them, and one that names no held file to none.
struct Walk<'g> {
    topology: &'g Topology<'g>,
    /// The base's `.gitattributes` texts, which mark vendored files.
    attributes: &'g [(String, String)],
    module_of: HashMap<&'g str, usize>,
    next: Vec<Vec<usize>>,
    sites: HashMap<(usize, u64), Vec<usize>>,
    unproved: Vec<&'g modules::resolver::Hole>,
}

impl<'g> Walk<'g> {
    fn of(
        topology: &'g Topology<'g>,
        graph: &'g ModuleGraph,
        attributes: &'g [(String, String)],
    ) -> Walk<'g> {
        let module_of: HashMap<&str, usize> = graph
            .modules
            .iter()
            .enumerate()
            .filter(|(_, module)| module.target.is_none() && module.nesting.is_empty())
            .flat_map(|(at, module)| module.sources.iter().map(move |file| (file.as_str(), at)))
            .collect();
        let mut walk = Walk {
            attributes,
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

    /// Whether the base's `.gitattributes` mark a file `linguist-vendored` or
    /// `linguist-generated`, so its code is not the repository's own.
    fn vendored(&self, file: &str) -> bool {
        !self.attributes.is_empty() && files::vendored(file, self.attributes)
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

    /// What the files of the tree import from other modules: every name of a module a
    /// namespace import, a star or an `import()` takes, and each name a named import or
    /// re-export takes. An unreached file's imports count, so an export only dead code uses is
    /// no finding of its own beside that file's; a test's imports do not, as tests are no
    /// entries.
    fn used(&self) -> Used<'g> {
        let mut used = Used::default();
        for (file, from) in &self.module_of {
            let Some(facts) = self.topology.facts(file).filter(|_| judgeable(file)) else {
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
    base: &'a Base<'a>,
    walk: &'a Walk<'g>,
    used: &'a Used<'g>,
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
        let kit = self.kit();
        for file in self.files.iter().copied().filter(|file| judgeable(file)) {
            let now = (self.current)(file);
            let Some(module) = self.walk.module_of.get(file).copied() else {
                continue;
            };
            if self.left_out(file, &now, kit.as_deref()) || !self.scope.selects(&now) {
                out.excluded.insert(now);
                continue;
            }
            let kind = kinds.get(file).copied();
            match reached[module] {
                true => self.exports(file, (module, kind), &now, out),
                false => self.dormant(file, &now, out),
            }
            if kind.is_none() {
                out.files.push(JudgedFile {
                    file: now.clone(),
                    unreached: !reached[module],
                });
            }
            out.judged.insert(now);
        }
    }

    /// Whether a file is code the app holds and did not write, or a route a person turned off:
    /// a file of the kit, a file the base's `.gitattributes` mark vendored or generated, or a
    /// convention file under `app/` the base held that a leading `_` turns off. Neither rule
    /// judges it. The kit and the marks lie where the base holds them, and leave a file out by
    /// the path it has or the one the base held it at, so a renamed kit stays left out.
    fn left_out(&self, file: &str, now: &str, kit: Option<&str>) -> bool {
        let paths = [now, self.base.was(now)];
        paths
            .iter()
            .any(|path| kit.is_some_and(|kit| under_or_at(path, kit)) || self.walk.vendored(path))
            || self.package.role == Role::NextApp
                && private(relative(&self.package.directory, file))
                && self.base.holds(file)
    }

    /// What keeps the package from being judged: an alias configuration klin cannot prove, and a
    /// file the grammar refused, whose imports are unknown. Only a file app mode judges counts:
    /// a test's or a declaration file's imports reach nothing, so its holes hide nothing.
    fn unproved(&self) -> Vec<Unresolved> {
        let aliases = self
            .walk
            .unproved
            .iter()
            .filter(|hole| judgeable(&hole.file))
            .filter(|hole| self.files.binary_search(&hole.file.as_str()).is_ok())
            .map(|hole| Unresolved {
                file: (self.current)(&hole.file),
                line: hole.line,
                text: hole.text.clone(),
                why: hole.why.clone(),
            });
        let unread = self
            .files
            .iter()
            .filter(|file| judgeable(file))
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
            .filter_map(|file| Some((file, self.kind(file, &scripted)?)))
            .collect()
    }

    /// The kind of entry the package's runtime makes of a file, by the package's role: a Next
    /// app routes its convention files, a Vite app runs what its `index.html` names, and every
    /// app runs its root `*.config` files and the files its scripts name.
    fn kind(&self, file: &str, scripted: &HashSet<String>) -> Option<Kind> {
        let relative = relative(&self.package.directory, file);
        let routed = match &self.package.role {
            Role::NextApp => routed(relative),
            Role::ViteApp(named) => named
                .iter()
                .any(|held| held == file)
                .then_some(Kind::Program),
            Role::Library | Role::Unjudged(_) => None,
        };
        routed
            .or_else(|| configured(relative))
            .or_else(|| scripted.contains(file).then_some(Kind::Program))
    }

    /// The TypeScript files the scripts of the base's manifest name, joined to its directory.
    fn scripted(&self) -> HashSet<String> {
        let separator = |c: char| c.is_whitespace() || matches!(c, ';' | '&' | '|' | '(' | ')');
        self.base
            .scripts(&self.package.manifest)
            .iter()
            .flat_map(|command| command.split(separator))
            .map(|word| word.trim_matches(['"', '\'']))
            .filter_map(|word| word.rsplit('=').next())
            .filter(|word| SCRIPTED.iter().any(|end| word.ends_with(end)))
            .filter_map(|word| joined(&self.package.directory, word))
            .filter(|file| self.walk.topology.holds(file))
            .collect()
    }

    /// The directory of the package's vendored kit: the `ui` alias the base's `components.json`
    /// beside the manifest names, resolved through the base's paths rules, and none where it
    /// names none klin can prove. With no `components.json`, the `components/ui` directory at
    /// the package root or under `src/`, where a kit is vendored by hand.
    fn kit(&self) -> Option<String> {
        let file = joined(&self.package.directory, KIT)?;
        if !self.base.holds(&file) {
            return self.vendored_by_hand();
        }
        let ui = self.base.read(&file).and_then(|bytes| ui_alias(&bytes))?;
        let paths = self.base.aliases();
        self.files
            .iter()
            .find_map(|from| paths.resolve(self.base.was(&(self.current)(from)), &ui))
    }

    fn vendored_by_hand(&self) -> Option<String> {
        ["components/ui", "src/components/ui"]
            .into_iter()
            .filter_map(|at| joined(&self.package.directory, at))
            .find(|at| self.files.iter().any(|file| under_or_at(file, at)))
    }

    fn exports(
        &self,
        file: &str,
        (module, kind): (usize, Option<Kind>),
        now: &str,
        out: &mut Judged,
    ) {
        let Some(facts) = self.walk.topology.facts(file) else {
            return;
        };
        let signed = signed(facts);
        for (name, line) in exports(facts) {
            out.exports += 1;
            let read = kind.is_some_and(|kind| kind.reads(&name));
            if !self.used.holds(module, &name) && !read && !signed.contains(name.as_str()) {
                out.unused.push(Unused {
                    file: now.to_string(),
                    line,
                    name,
                });
            }
        }
    }
}

impl App<'_, '_> {
    /// The exports of an unreached file, which its `unreached` finding holds: at the base they
    /// are held debt, so the file becoming reached opens no new finding for an export it
    /// already had.
    fn dormant(&self, file: &str, now: &str, out: &mut Judged) {
        let Some(facts) = self.walk.topology.facts(file) else {
            return;
        };
        out.dormant
            .extend(exports(facts).into_iter().map(|(name, line)| Unused {
                file: now.to_string(),
                line,
                name,
            }));
    }
}

/// The alias a `components.json` names for its kit: `aliases.ui`.
fn ui_alias(bytes: &[u8]) -> Option<String> {
    let parsed: Value = serde_json::from_slice(bytes).ok()?;
    Some(parsed.pointer("/aliases/ui")?.as_str()?.to_string())
}

/// Every value a file exports at its top, with the line that first exports it: its exported
/// declarations, by the name a consumer addresses each by, and every name an export clause, a
/// default export or a re-export names. A bare star names none, and a type alias, an interface
/// or an `export type` clause exports no value.
fn exports(facts: &FileFacts) -> BTreeMap<String, u64> {
    let mut out = BTreeMap::new();
    for (name, line) in declared(facts) {
        out.entry(name.to_string()).or_insert(line);
    }
    let values = facts
        .exports
        .iter()
        .filter(|export| export.supported && !export.type_only);
    for export in values {
        for name in export.leaves.iter().filter_map(|leaf| leaf.name.as_deref()) {
            out.entry(name.to_string()).or_insert(export.line);
        }
    }
    out
}

/// Every value the declared contract of an exported declaration reads through `typeof`, as
/// `export type Size = (typeof sizes)[number]` reads `sizes`. A declaration a local export
/// clause exposes, such as `type Size` beside `export type { Size }`, is exported too. Any other
/// word of a contract, such as a property name, reads no value.
fn signed(facts: &FileFacts) -> HashSet<&str> {
    let clauses: HashSet<&str> = facts
        .exports
        .iter()
        .filter(|export| export.source.is_none())
        .flat_map(|export| &export.leaves)
        .map(|leaf| leaf.path.as_str())
        .collect();
    let mut out = HashSet::new();
    let declared = facts
        .declarations
        .iter()
        .filter(|held| exported(held) || clauses.contains(held.name.as_str()));
    for declaration in declared {
        let Some(signature) = declaration.signature.as_deref() else {
            continue;
        };
        let words: Vec<&str> = signature
            .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .filter(|word| !word.is_empty())
            .collect();
        let read = words
            .windows(2)
            .filter(|pair| pair[0] == "typeof")
            .map(|pair| pair[1]);
        out.extend(read);
    }
    out
}

fn exported(declaration: &Declaration) -> bool {
    declaration.visibility == Visibility::Public
        && declaration.kind != DeclarationKind::Method
        && !declaration.associated
}

/// Whether a declaration is a type alias or an interface, which exists only for the compiler.
fn type_only(declaration: &Declaration) -> bool {
    let contract = declaration.signature.as_deref().unwrap_or_default();
    let contract = contract.strip_prefix("declare ").unwrap_or(contract);
    declaration.kind == DeclarationKind::Type
        && (contract.starts_with("type ") || contract.starts_with("interface "))
}

/// Each exported declaration of a file by the name a consumer addresses it by, with its line.
fn declared(facts: &FileFacts) -> impl Iterator<Item = (&str, u64)> {
    facts
        .declarations
        .iter()
        .filter(|declaration| exported(declaration) && !type_only(declaration))
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
fn routed(relative: &str) -> Option<Kind> {
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
    if !matches!(at, "" | "src") {
        return None;
    }
    match stem {
        "middleware" | "proxy" => Some(Kind::Middleware),
        "instrumentation" => Some(Kind::Instrumentation),
        "instrumentation-client" => Some(Kind::InstrumentationClient),
        _ => None,
    }
}

/// A `*.config` file at the package root, such as `next.config.ts` or `vite.config.ts`, is a
/// program its tool runs.
fn configured(relative: &str) -> Option<Kind> {
    let (stem, _) = relative.rsplit_once('.')?;
    (!relative.contains('/') && stem.ends_with(".config")).then_some(Kind::Program)
}

/// Whether a file under `app/` or `src/app/` is a convention file a person turned off by
/// renaming it with a leading `_`, such as `_route.ts`, which Next.js no longer routes. A folder
/// or any other file named so is judged as usual, so a name alone hides no new code.
fn private(relative: &str) -> bool {
    let routed = under_or_at(relative, "app") || under_or_at(relative, "src/app");
    let disabled = basename(relative)
        .rsplit_once('.')
        .and_then(|(stem, _)| stem.strip_prefix('_'))
        .is_some_and(|name| CONVENTIONS.contains(&name));
    routed && disabled
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
