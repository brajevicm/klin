//! The module graph of one tree: which module each Rust and TypeScript file is, which module
//! holds which, and which module each one depends on. It is built from the structural facts a
//! run already extracted and from the tree's own file list, and it parses no source. Resolution
//! is conservative: a dependency is an edge only where a resolver proves its target, a form a
//! resolver supports and cannot prove is a hole, and a form outside V1 is counted and never
//! guessed. Containment builds the module tree and is never an edge. A check reads modules,
//! dependencies, holes and cycles, and never a graph library's types. ADR 0043, spec 8.4.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::iter::Peekable;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::syntax::structural::{self, FileFacts, LanguageId};

mod rust;
mod typescript;

type Resolve = fn(&mut Builder);

/// One resolver per structural language, run only over a tree that lists a path of that
/// language. A new language is a resolver file and one entry here.
const RESOLVERS: &[(LanguageId, Resolve)] = &[
    (LanguageId::Rust, rust::resolve),
    (LanguageId::TypeScript, typescript::resolve),
];

/// The manifests that make a language present in a tree with no source of it, because its
/// resolver or surface reads them: a Cargo manifest names Rust targets, a package manifest
/// names TypeScript entry points.
const MANIFESTS: &[(&str, LanguageId)] = &[
    ("Cargo.toml", LanguageId::Rust),
    ("package.json", LanguageId::TypeScript),
];

/// One tree as the graph reads it: every file under the path its topology gives it, the facts of
/// its structural files, and where each path's bytes sit. The base tree names a renamed file by
/// the path it had at the base, so the base's topology is the base's own. Spec 8.4.
pub struct Topology<'a> {
    root: &'a Path,
    files: Vec<String>,
    physical: HashMap<String, String>,
    facts: HashMap<String, Rc<FileFacts>>,
    present: Vec<LanguageId>,
}

impl<'a> Topology<'a> {
    /// The tree at `root`, whose file list and facts name files as the tree holds them, with
    /// `named` giving the topology path of every file the topology names differently.
    pub fn new(
        root: &'a Path,
        files: &[String],
        facts: &[Rc<FileFacts>],
        named: &HashMap<String, String>,
    ) -> Topology<'a> {
        let topology = |file: &str| named.get(file).cloned().unwrap_or_else(|| file.to_string());
        let mut listed: Vec<String> = files.iter().map(|file| topology(file)).collect();
        listed.sort_unstable();
        listed.dedup();
        Topology {
            root,
            present: present(&listed),
            files: listed,
            physical: named
                .iter()
                .map(|(held, path)| (path.clone(), held.clone()))
                .collect(),
            facts: facts
                .iter()
                .map(|held| (topology(&held.file), Rc::clone(held)))
                .collect(),
        }
    }

    /// Whether the tree lists a source or manifest path of this language, which is what decides
    /// whether a resolver or a surface derivation of it runs. A source whose grammar refused it
    /// still counts, because a resolver still places it.
    pub fn present(&self, language: LanguageId) -> bool {
        self.present.contains(&language)
    }

    /// Whether the tree lists a file at this path.
    pub fn holds(&self, path: &str) -> bool {
        self.files
            .binary_search_by(|held| held.as_str().cmp(path))
            .is_ok()
    }

    /// Every file the tree lists, under its topology path, in path order.
    pub fn files(&self) -> &[String] {
        &self.files
    }

    /// Whether a file sits at this path on disk and the file list leaves it out, as it leaves out
    /// a file git ignores, such as generated source.
    fn ignored(&self, path: &str) -> bool {
        !self.holds(path) && self.root.join(self.held_path(path)).is_file()
    }

    fn held_path<'p>(&'p self, path: &'p str) -> &'p str {
        self.physical.get(path).map_or(path, String::as_str)
    }

    /// The bytes of one file the tree holds, read from disk. A manifest is read this way; a
    /// source file never is, because its facts are already extracted.
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join(self.held_path(path))).ok()
    }

    /// The structural facts of one file, and `None` for a file no adapter measured.
    pub fn facts(&self, path: &str) -> Option<&FileFacts> {
        self.facts.get(path).map(Rc::as_ref)
    }

    /// Every file below a directory, in path order, and every file for the root's empty name.
    fn under<'s>(&'s self, directory: &str) -> impl Iterator<Item = &'s str> {
        let prefix = match directory {
            "" => String::new(),
            named => format!("{named}/"),
        };
        let from = self.files.partition_point(|held| *held < prefix);
        self.files[from..]
            .iter()
            .take_while(move |held| held.starts_with(&prefix))
            .map(String::as_str)
    }
}

/// The languages of the resolvers a tree lists a path for, from the file list it already holds.
/// The scan stops once every registered language is found.
fn present(files: &[String]) -> Vec<LanguageId> {
    let mut out: Vec<LanguageId> = Vec::new();
    for file in files {
        if out.len() == RESOLVERS.len() {
            break;
        }
        let name = file.rsplit('/').next().unwrap_or(file);
        let language = structural::language_of(file).or_else(|| {
            MANIFESTS
                .iter()
                .find(|(held, _)| *held == name)
                .map(|(_, id)| *id)
        });
        if let Some(id) = language.filter(|id| RESOLVERS.iter().any(|(held, _)| held == id))
            && !out.contains(&id)
        {
            out.push(id);
        }
    }
    out
}

/// What building the graphs of a gate's trees cost: their modules, the physical sources those
/// modules hold, their dependency sites, the distinct module pairs those sites join, the
/// resolvers each language dispatched, and the time resolution and cycle finding took. Spec
/// 11.2, 13.
#[derive(Default, Clone, Copy)]
pub struct GraphCost {
    pub modules: usize,
    pub sources: usize,
    pub dependencies: usize,
    pub edges: usize,
    pub dispatches: [usize; RESOLVERS.len()],
    pub time: Duration,
}

impl GraphCost {
    /// Each registered resolver's language with how many times it ran.
    pub fn dispatched(&self) -> impl Iterator<Item = (LanguageId, usize)> + '_ {
        RESOLVERS
            .iter()
            .zip(self.dispatches)
            .map(|((language, _), count)| (*language, count))
    }
}

impl std::ops::Add for GraphCost {
    type Output = GraphCost;

    fn add(self, other: GraphCost) -> GraphCost {
        let mut dispatches = self.dispatches;
        for (held, more) in dispatches.iter_mut().zip(other.dispatches) {
            *held += more;
        }
        GraphCost {
            modules: self.modules + other.modules,
            sources: self.sources + other.sources,
            dependencies: self.dependencies + other.dependencies,
            edges: self.edges + other.edges,
            dispatches,
            time: self.time + other.time,
        }
    }
}

/// How a file came to be a module: under a target a Cargo manifest names, under a conventional
/// root where no manifest says, or as a file that is its own module.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Attachment {
    Manifest,
    Convention,
    File,
}

/// What a target is built as. A library is what another crate consumes, so only a library has
/// a public surface.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetKind {
    Library,
    Binary,
}

/// One Cargo target, or one conventional root standing in for it: the package that owns it, the
/// crate name a consumer addresses it by, its kind, its root file, the manifest that named it,
/// and its root module in the graph.
pub struct Target {
    pub package: String,
    pub name: String,
    pub kind: TargetKind,
    pub root: String,
    pub manifest: Option<String>,
    pub module: usize,
}

/// One module: the name a report prints, which is its resolver's identity for it, and the
/// physical files that hold it. A module holds at least one file, in path order, each once, and
/// the order means nothing. One file may hold several modules, and one file under two targets is
/// a module under each. A Rust module is one file or inline in one, and a TypeScript module is
/// one file; a resolver for another language may group several files into one module. A Rust
/// module also knows its place in its target's tree; a TypeScript module stands alone. ADR 0047.
pub struct Module {
    pub name: String,
    pub sources: Vec<String>,
    /// The inline modules between the file and this module, outermost first.
    pub nesting: Vec<String>,
    /// The target this module belongs to, and `None` for a module no target owns.
    pub target: Option<usize>,
    /// The module that declares this one, and `None` for a target root or a file module.
    pub parent: Option<usize>,
    /// Each module this one declares, by the name it declares it under.
    pub children: BTreeMap<String, usize>,
    /// The names this module declares as modules that no file answers, so a path through one is
    /// unresolved and never external.
    pub unresolved: BTreeSet<String>,
}

/// Where a path from one module ends up. The module graph resolves the module part and hands
/// back the segments after it, which name an item of that module or nothing.
pub enum Resolved {
    Module {
        module: usize,
        rest: Vec<String>,
    },
    /// The path starts at a name that is no module here: another crate, or a local item.
    External,
    /// The path goes above the crate root or through a module no file answers.
    Unresolved,
}

/// One resolved dependency of one module on another, at the physical site that writes it: the
/// position of that file among the writing module's sources, and the line. `ModuleGraph::source`
/// names the file.
pub struct Dependency {
    pub from: usize,
    pub to: usize,
    pub source: u32,
    pub line: u64,
}

/// One form a resolver supports and could not prove a target for.
pub struct Hole {
    pub file: String,
    pub line: u64,
    pub text: String,
    pub why: String,
}

#[derive(Default)]
pub struct ModuleGraph {
    pub modules: Vec<Module>,
    pub targets: Vec<Target>,
    pub dependencies: Vec<Dependency>,
    pub holes: Vec<Hole>,
    /// The dependencies written on something outside V1: another crate, a package, an alias.
    pub external: usize,
    pub attached: BTreeMap<String, Attachment>,
    /// The structural files no resolver made a module of, such as a Rust file no target reaches.
    pub unattached: Vec<String>,
    pub dispatches: [usize; RESOLVERS.len()],
    pub time: Duration,
}

/// What a resolver adds to the graph it builds.
pub(crate) struct Builder<'a> {
    topology: &'a Topology<'a>,
    graph: ModuleGraph,
}

impl Builder<'_> {
    /// A module of these files, each of which counts as attached on its own.
    fn module(&mut self, name: String, files: &[&str], attachment: Attachment) -> usize {
        let mut sources: Vec<String> = files.iter().map(|file| file.to_string()).collect();
        sources.sort_unstable();
        sources.dedup();
        assert!(!sources.is_empty(), "a module holds at least one file");
        for file in &sources {
            let held = self
                .graph
                .attached
                .entry(file.clone())
                .or_insert(attachment);
            *held = (*held).min(attachment);
        }
        self.graph.modules.push(Module {
            name,
            sources,
            nesting: Vec::new(),
            target: None,
            parent: None,
            children: BTreeMap::new(),
            unresolved: BTreeSet::new(),
        });
        self.graph.modules.len() - 1
    }

    /// A dependency written in `file`, which is one of the writing module's own sources.
    fn depend(&mut self, from: usize, to: usize, file: &str, line: u64) {
        let written = self.graph.modules[from]
            .sources
            .binary_search_by(|held| held.as_str().cmp(file));
        match written {
            Ok(source) => self.graph.dependencies.push(Dependency {
                from,
                to,
                source: source as u32,
                line,
            }),
            Err(_) => self.hole(
                file,
                line,
                &self.graph.modules[to].name.clone(),
                "a dependency from a file that is not one of its module's files".to_string(),
            ),
        }
    }

    fn hole(&mut self, file: &str, line: u64, text: &str, why: String) {
        self.graph.holes.push(Hole {
            file: file.to_string(),
            line,
            text: text.to_string(),
            why,
        });
    }
}

/// The graph of one tree, each resolver whose language the tree holds run over it once.
pub fn build(topology: &Topology) -> ModuleGraph {
    let started = Instant::now();
    let mut builder = Builder {
        topology,
        graph: ModuleGraph::default(),
    };
    for (at, (language, resolve)) in RESOLVERS.iter().enumerate() {
        if topology.present(*language) {
            builder.graph.dispatches[at] += 1;
            resolve(&mut builder);
        }
    }
    let mut graph = builder.graph;
    let site = |held: &Dependency| (held.from, held.to, held.source, held.line);
    graph.dependencies.sort_unstable_by_key(site);
    graph.dependencies.dedup_by_key(|held| site(held));
    graph.holes.sort_by(|a, b| {
        (&a.file, a.line, &a.text, &a.why).cmp(&(&b.file, b.line, &b.text, &b.why))
    });
    graph
        .holes
        .dedup_by(|a, b| (&a.file, a.line, &a.why) == (&b.file, b.line, &b.why));
    let mut unattached: Vec<String> = topology
        .facts
        .keys()
        .filter(|file| !graph.attached.contains_key(*file))
        .cloned()
        .collect();
    unattached.sort_unstable();
    graph.unattached = unattached;
    graph.time = started.elapsed();
    graph
}

/// The cycles among the dependencies one selection keeps: which dependencies close one, one
/// path that shows it, and how many distinct module pairs the components were found over.
pub struct Cycles<'g> {
    graph: &'g ModuleGraph,
    component: Vec<Option<usize>>,
    next: Vec<Vec<usize>>,
    edges: usize,
}

impl ModuleGraph {
    /// What building this graph cost.
    pub fn cost(&self) -> GraphCost {
        let edges = self
            .dependencies
            .windows(2)
            .filter(|pair| (pair[0].from, pair[0].to) != (pair[1].from, pair[1].to))
            .count()
            + usize::from(!self.dependencies.is_empty());
        GraphCost {
            modules: self.modules.len(),
            sources: self.modules.iter().map(|module| module.sources.len()).sum(),
            dependencies: self.dependencies.len(),
            edges,
            dispatches: self.dispatches,
            time: self.time,
        }
    }

    /// The physical file that writes a dependency.
    pub fn source(&self, dependency: &Dependency) -> &str {
        &self.modules[dependency.from].sources[dependency.source as usize]
    }

    /// The name that identifies a module under current paths: its resolver's name, with the
    /// source path it starts with, if any, under `current`. A renamed file keeps the identity of
    /// the modules it holds.
    pub fn identity(&self, module: usize, current: impl Fn(&str) -> String) -> String {
        let held = &self.modules[module];
        held.sources
            .iter()
            .find_map(|file| {
                let rest = held.name.strip_prefix(file.as_str())?;
                (rest.is_empty() || rest.starts_with("::")).then_some((file, rest))
            })
            .map_or_else(
                || held.name.clone(),
                |(file, rest)| format!("{}{rest}", current(file)),
            )
    }

    /// The identity that tells one module apart from every other across the base and the working
    /// tree: the kind and root of the target that owns it under current paths, then its identity.
    /// A file two targets reach is a different module under each. A module no target owns is
    /// its identity alone. ADR 0047.
    pub fn semantic(&self, module: usize, current: impl Fn(&str) -> String) -> String {
        let identity = self.identity(module, &current);
        match self.modules[module].target {
            Some(at) => {
                let target = &self.targets[at];
                format!("{:?} {} {identity}", target.kind, current(&target.root))
            }
            None => identity,
        }
    }

    /// The module a path names from this module, and the segments left after it. `crate` starts
    /// at the module's target root, `self` and `super` at the module and the ones above it, a
    /// name this module declares as a child at that child, and any other first name is external.
    /// Each further name descends into a child of that name until one is no module.
    pub fn resolve(&self, from: usize, path: &str) -> Resolved {
        let mut segments = path
            .split("::")
            .map(|segment| segment.trim_start_matches("r#"))
            .peekable();
        let Some(start) = self.start(from, &mut segments) else {
            return Resolved::External;
        };
        match self.ascended(start, &mut segments) {
            Some(at) => self.descended(at, segments),
            None => Resolved::Unresolved,
        }
    }

    /// The module a path's first segment starts from, and `None` for a name that is no module
    /// here.
    fn start<'p>(
        &self,
        from: usize,
        segments: &mut Peekable<impl Iterator<Item = &'p str>>,
    ) -> Option<usize> {
        match segments.peek().copied()? {
            "crate" => {
                segments.next();
                Some(self.targets[self.modules[from].target?].module)
            }
            "self" | "super" => Some(from),
            first if self.modules[from].children.contains_key(first) => Some(from),
            _ => None,
        }
    }

    /// The module the leading `self` and `super` segments climb to, and `None` above the root.
    fn ascended<'p>(
        &self,
        mut at: usize,
        segments: &mut Peekable<impl Iterator<Item = &'p str>>,
    ) -> Option<usize> {
        while let Some(segment) = segments.next_if(|segment| matches!(*segment, "self" | "super")) {
            if segment == "super" {
                at = self.modules[at].parent?;
            }
        }
        Some(at)
    }

    /// The deepest child module the remaining segments name, with the segments after it.
    fn descended<'p>(&self, mut at: usize, segments: impl Iterator<Item = &'p str>) -> Resolved {
        let mut rest = Vec::new();
        for segment in segments {
            let module = &self.modules[at];
            match module.children.get(segment) {
                Some(child) if rest.is_empty() => at = *child,
                None if rest.is_empty() && module.unresolved.contains(segment) => {
                    return Resolved::Unresolved;
                }
                _ => rest.push(segment.to_string()),
            }
        }
        Resolved::Module { module: at, rest }
    }

    /// Every module the dependencies one line of one file of a module writes resolve to.
    pub fn reached_at(&self, from: usize, file: &str, line: u64) -> Vec<usize> {
        self.dependencies
            .iter()
            .filter(|dependency| dependency.from == from && dependency.line == line)
            .filter(|dependency| self.source(dependency) == file)
            .map(|dependency| dependency.to)
            .collect()
    }

    /// The strongly connected components of the modules the dependency sites `keep` selects
    /// join. The sites are physical, so a scope judges each file that writes one; the components
    /// are over modules, and every selected pair of modules is one edge however many sites
    /// write it.
    pub fn cycles(&self, keep: impl Fn(&Dependency) -> bool) -> Cycles<'_> {
        let mut pairs: Vec<(usize, usize)> = self
            .dependencies
            .iter()
            .filter(|dependency| keep(dependency))
            .map(|dependency| (dependency.from, dependency.to))
            .collect();
        pairs.dedup();
        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        let mut nodes: Vec<Option<NodeIndex>> = vec![None; self.modules.len()];
        let mut next = vec![Vec::new(); self.modules.len()];
        for (from, to) in &pairs {
            let mut node = |at: usize| *nodes[at].get_or_insert_with(|| graph.add_node(at));
            let (a, b) = (node(*from), node(*to));
            graph.add_edge(a, b, ());
            next[*from].push(*to);
        }
        for targets in &mut next {
            targets.sort_by(|a, b| self.modules[*a].name.cmp(&self.modules[*b].name));
        }
        let mut component = vec![None; self.modules.len()];
        for (id, members) in tarjan_scc(&graph).into_iter().enumerate() {
            for node in members {
                component[graph[node]] = Some(id);
            }
        }
        Cycles {
            graph: self,
            component,
            next,
            edges: pairs.len(),
        }
    }
}

impl Cycles<'_> {
    /// How many distinct module pairs the components were found over.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn edges(&self) -> usize {
        self.edges
    }

    /// Whether this dependency joins two modules of one component, which is a cycle, and a
    /// dependency of a module on itself is one.
    pub fn closes(&self, dependency: &Dependency) -> bool {
        self.component[dependency.from].is_some()
            && self.component[dependency.from] == self.component[dependency.to]
    }

    /// The module names of one shortest cycle through this dependency, starting and ending at
    /// the module that writes it. The path explains a finding and never names it.
    pub fn path(&self, dependency: &Dependency) -> Vec<&str> {
        let mut before: HashMap<usize, usize> = HashMap::new();
        let mut queue = VecDeque::from([dependency.to]);
        while let Some(at) = queue.pop_front().filter(|at| *at != dependency.from) {
            for next in &self.next[at] {
                let fresh = *next != dependency.to && !before.contains_key(next);
                if fresh && self.component[*next] == self.component[at] {
                    before.insert(*next, at);
                    queue.push_back(*next);
                }
            }
        }
        let mut trail = vec![dependency.from];
        let mut at = dependency.from;
        while let Some(previous) = before.get(&at).filter(|_| at != dependency.to) {
            at = *previous;
            trail.push(at);
        }
        trail.push(dependency.from);
        trail.reverse();
        trail
            .into_iter()
            .map(|module| self.graph.modules[module].name.as_str())
            .collect()
    }
}

/// A path joined to a directory, with `.` and `..` read, and `None` where it leaves the tree.
pub(crate) fn joined(directory: &str, relative: &str) -> Option<String> {
    let mut parts: Vec<&str> = directory
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    for part in relative.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(parts.join("/"))
}

/// The directory a path sits in, and the empty name for the tree root.
pub(crate) fn directory(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(at, _)| at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_of_many_files_attaches_each_file_and_names_each_site() {
        let files = ["b.go".to_string(), "a.go".to_string()];
        let topology = Topology::new(Path::new("."), &files, &[], &HashMap::new());
        let mut builder = Builder {
            topology: &topology,
            graph: ModuleGraph::default(),
        };
        let from = builder.module("p".into(), &["b.go", "a.go", "b.go"], Attachment::File);
        let to = builder.module("q".into(), &["c.go"], Attachment::File);
        builder.depend(from, to, "b.go", 3);
        builder.depend(from, to, "a.go", 3);
        let graph = builder.graph;
        assert_eq!(graph.modules[from].sources, ["a.go", "b.go"]);
        let attached: Vec<&str> = graph.attached.keys().map(String::as_str).collect();
        assert_eq!(attached, ["a.go", "b.go", "c.go"]);
        assert_eq!(graph.source(&graph.dependencies[0]), "b.go");
        assert_eq!(graph.reached_at(from, "a.go", 3), [to]);
        assert_eq!(graph.cost().sources, 3);
    }
}
