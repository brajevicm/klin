//! The module graph of one tree: which module each Rust and TypeScript file is, which module
//! holds which, and which module each one depends on. It is built from the structural facts a
//! run already extracted and from the tree's own file list, and it parses no source. Resolution
//! is conservative: a dependency is an edge only where a resolver proves its target, a form a
//! resolver supports and cannot prove is a hole, and a form outside V1 is counted and never
//! guessed. Containment builds the module tree and is never an edge. A check reads modules,
//! dependencies, holes and cycles, and never a graph library's types. ADR 0043, spec 8.4.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::syntax::structural::FileFacts;

mod rust;
mod typescript;

/// One resolver per structural language. A new language is a resolver file and one entry here.
const RESOLVERS: &[fn(&mut Builder)] = &[rust::resolve, typescript::resolve];

/// One tree as the graph reads it: every file under the path its topology gives it, the facts of
/// its structural files, and where each path's bytes sit. The base tree names a renamed file by
/// the path it had at the base, so the base's topology is the base's own. Spec 8.4.
pub struct Topology<'a> {
    root: &'a Path,
    files: Vec<String>,
    physical: HashMap<String, String>,
    facts: HashMap<String, Rc<FileFacts>>,
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

    fn holds(&self, path: &str) -> bool {
        self.files
            .binary_search_by(|held| held.as_str().cmp(path))
            .is_ok()
    }

    /// Whether a file sits at this path on disk and the file list leaves it out, as it leaves out
    /// a file git ignores, such as generated source.
    fn ignored(&self, path: &str) -> bool {
        !self.holds(path) && self.root.join(self.held_path(path)).is_file()
    }

    fn held_path<'p>(&'p self, path: &'p str) -> &'p str {
        self.physical.get(path).map_or(path, String::as_str)
    }

    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join(self.held_path(path))).ok()
    }

    fn facts(&self, path: &str) -> Option<&FileFacts> {
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

/// What building the graphs of a gate's trees cost: their modules, their dependencies, and the
/// time resolution and cycle finding took. Spec 11.2, 13.
#[derive(Default, Clone, Copy)]
pub struct GraphCost {
    pub modules: usize,
    pub dependencies: usize,
    pub time: Duration,
}

impl std::ops::Add for GraphCost {
    type Output = GraphCost;

    fn add(self, other: GraphCost) -> GraphCost {
        GraphCost {
            modules: self.modules + other.modules,
            dependencies: self.dependencies + other.dependencies,
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

/// One module: the name a report prints and the file that holds it. One file may hold several
/// modules, and one file under two targets is a module under each.
pub struct Module {
    pub name: String,
    pub file: String,
}

/// One resolved dependency of one module on another, at the line that writes it.
pub struct Dependency {
    pub from: usize,
    pub to: usize,
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
    pub dependencies: Vec<Dependency>,
    pub holes: Vec<Hole>,
    /// The dependencies written on something outside V1: another crate, a package, an alias.
    pub external: usize,
    pub attached: BTreeMap<String, Attachment>,
    /// The structural files no resolver made a module of, such as a Rust file no target reaches.
    pub unattached: Vec<String>,
    pub time: Duration,
}

/// What a resolver adds to the graph it builds.
pub(crate) struct Builder<'a> {
    topology: &'a Topology<'a>,
    graph: ModuleGraph,
}

impl Builder<'_> {
    fn module(&mut self, name: String, file: &str, attachment: Attachment) -> usize {
        let held = self
            .graph
            .attached
            .entry(file.to_string())
            .or_insert(attachment);
        *held = (*held).min(attachment);
        self.graph.modules.push(Module {
            name,
            file: file.to_string(),
        });
        self.graph.modules.len() - 1
    }

    fn depend(&mut self, from: usize, to: usize, line: u64) {
        self.graph.dependencies.push(Dependency { from, to, line });
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

/// The graph of one tree, every resolver run over it once.
pub fn build(topology: &Topology) -> ModuleGraph {
    let started = Instant::now();
    let mut builder = Builder {
        topology,
        graph: ModuleGraph::default(),
    };
    for resolve in RESOLVERS {
        resolve(&mut builder);
    }
    let mut graph = builder.graph;
    graph
        .dependencies
        .sort_unstable_by_key(|held| (held.from, held.to, held.line));
    graph
        .dependencies
        .dedup_by_key(|held| (held.from, held.to, held.line));
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

/// The cycles among the modules one selection keeps: which dependencies close one, and one
/// path that shows it.
pub struct Cycles<'g> {
    graph: &'g ModuleGraph,
    component: Vec<Option<usize>>,
    next: Vec<Vec<usize>>,
}

impl ModuleGraph {
    /// What building this graph cost.
    pub fn cost(&self) -> GraphCost {
        GraphCost {
            modules: self.modules.len(),
            dependencies: self.dependencies.len(),
            time: self.time,
        }
    }

    /// The name that identifies a module under current paths: its file under `current`, with the
    /// inline modules its name adds after that file.
    pub fn identity(&self, module: usize, current: impl Fn(&str) -> String) -> String {
        let held = &self.modules[module];
        let inline = held
            .name
            .strip_prefix(held.file.as_str())
            .unwrap_or_default();
        format!("{}{inline}", current(&held.file))
    }

    /// The strongly connected components of the dependencies between the modules `keep` selects.
    pub fn cycles(&self, keep: impl Fn(&Module) -> bool) -> Cycles<'_> {
        let mut graph: DiGraph<usize, ()> = DiGraph::new();
        let nodes: Vec<Option<NodeIndex>> = self
            .modules
            .iter()
            .enumerate()
            .map(|(at, module)| keep(module).then(|| graph.add_node(at)))
            .collect();
        let mut next = vec![Vec::new(); self.modules.len()];
        for dependency in &self.dependencies {
            if let (Some(from), Some(to)) = (nodes[dependency.from], nodes[dependency.to]) {
                graph.add_edge(from, to, ());
                next[dependency.from].push(dependency.to);
            }
        }
        for targets in &mut next {
            targets.sort_by(|a, b| self.modules[*a].name.cmp(&self.modules[*b].name));
            targets.dedup();
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
        }
    }
}

impl Cycles<'_> {
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
fn joined(directory: &str, relative: &str) -> Option<String> {
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
fn directory(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(at, _)| at)
}
