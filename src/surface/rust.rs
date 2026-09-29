//! Rust surfaces. A Cargo library target is one surface, named by its crate name. From its root
//! module the derivation follows every `pub mod` and every `pub use`, so an item's path is the
//! path a consumer writes and never the file that declares it. A plain `pub` item inside a
//! private module is external only where a `pub use` exposes it. A restricted visibility is
//! never external. A re-export that reaches a library target the tree holds is followed into
//! its source. A re-export of any other crate, of an enum variant, or of a name klin cannot find
//! is an opaque item whose clause is its contract; a glob klin cannot list is a hole.

use std::collections::{HashMap, HashSet};

use super::{Derived, Hole, ITEM, Inapplicable, Item, MODULE, Surface, declared, opaque};
use crate::modules::{Module, ModuleGraph, Resolved, TargetKind, Topology};
use crate::syntax::structural::{Declaration, DeclarationKind, Export, ExportLeaf, Visibility};

const LANGUAGE: &str = "Rust";

pub(super) fn derive(topology: &Topology, graph: &ModuleGraph, out: &mut Derived) {
    let mut packages: Vec<(String, bool)> = Vec::new();
    for (at, target) in graph.targets.iter().enumerate() {
        let package = match &target.manifest {
            Some(manifest) => format!("Rust package {} ({manifest})", target.package),
            None => format!("Rust root {}", target.root),
        };
        let library = target.kind == TargetKind::Library;
        match packages.iter_mut().find(|(held, _)| *held == package) {
            Some((_, held)) => *held |= library,
            None => packages.push((package, library)),
        }
        if !library {
            continue;
        }
        let mut derivation = Derivation::new(topology, graph, at);
        derivation.walk(target.module, "");
        out.surfaces.push(derivation.surface);
    }
    for (what, _) in packages.into_iter().filter(|(_, library)| !library) {
        out.inapplicable.push(Inapplicable {
            what,
            why: "has no library target, and a binary has no public surface".to_string(),
        });
    }
}

/// One library target being derived: its surface so far, the inherent methods and type
/// declarations of each target it reaches by that target and name, the targets indexed so far,
/// and what has been walked or is being looked up through globs, so a cycle of re-exports ends.
struct Derivation<'a> {
    topology: &'a Topology<'a>,
    graph: &'a ModuleGraph,
    surface: Surface,
    methods: HashMap<(usize, &'a str), Vec<(usize, &'a Declaration)>>,
    types: HashMap<(usize, &'a str), Vec<usize>>,
    indexed: HashSet<usize>,
    walked: HashSet<(usize, String)>,
    sought: HashSet<(usize, String)>,
    globbed: HashSet<(usize, String, String)>,
    glob_names: HashMap<String, (String, u64)>,
}

impl<'a> Derivation<'a> {
    fn new(topology: &'a Topology<'a>, graph: &'a ModuleGraph, target: usize) -> Derivation<'a> {
        let named = &graph.targets[target];
        Derivation {
            topology,
            graph,
            surface: Surface {
                id: named.name.clone(),
                language: LANGUAGE,
                source: match &named.manifest {
                    Some(manifest) => format!("library target {} of {manifest}", named.root),
                    None => format!("conventional library root {}", named.root),
                },
                items: Vec::new(),
                files: Vec::new(),
                holes: Vec::new(),
            },
            methods: HashMap::new(),
            types: HashMap::new(),
            indexed: HashSet::new(),
            walked: HashSet::new(),
            sought: HashSet::new(),
            globbed: HashSet::new(),
            glob_names: HashMap::new(),
        }
    }

    /// Every public inherent method of one target by the type it is added to, and every type
    /// declaration by name, so a method finds its type wherever its `impl` sits. A target is
    /// indexed once, when a type of it is first exposed.
    fn index(&mut self, target: usize) {
        if !self.indexed.insert(target) {
            return;
        }
        for (at, module) in self.graph.modules.iter().enumerate() {
            if module.target != Some(target) {
                continue;
            }
            for declaration in self.declarations(at) {
                match (declaration.kind, &declaration.owner) {
                    (DeclarationKind::Method, Some(owner))
                        if declaration.visibility == Visibility::Public =>
                    {
                        self.methods
                            .entry((target, owner))
                            .or_default()
                            .push((at, declaration));
                    }
                    (DeclarationKind::Type, _) => {
                        self.types
                            .entry((target, &declaration.name))
                            .or_default()
                            .push(at);
                    }
                    _ => {}
                }
            }
        }
    }

    fn module(&self, at: usize) -> &'a Module {
        &self.graph.modules[at]
    }

    /// The file a Rust module is, or is inline in: its one source.
    fn file(&self, at: usize) -> &'a str {
        &self.module(at).sources[0]
    }

    /// The declarations written directly in one module: those of its file under its nesting.
    /// The iterator borrows the tree and the graph alone, never the derivation.
    fn declarations(&self, at: usize) -> impl Iterator<Item = &'a Declaration> + use<'a> {
        let module = self.module(at);
        let topology: &'a Topology<'a> = self.topology;
        topology
            .facts(self.file(at))
            .into_iter()
            .flat_map(|facts| &facts.declarations)
            .filter(move |declaration| declaration.nesting == module.nesting)
    }

    fn exports(&self, at: usize) -> impl Iterator<Item = &'a Export> + use<'a> {
        let module = self.module(at);
        let topology: &'a Topology<'a> = self.topology;
        topology
            .facts(self.file(at))
            .into_iter()
            .flat_map(|facts| &facts.exports)
            .filter(move |export| export.nesting == module.nesting)
    }

    /// The child modules one module declares as `pub mod`, by name.
    fn public_children(&self, at: usize) -> Vec<(String, usize)> {
        let module = self.module(at);
        self.topology
            .facts(self.file(at))
            .into_iter()
            .flat_map(|facts| &facts.module_declarations)
            .filter(|held| held.nesting == module.nesting && held.visibility == Visibility::Public)
            .filter_map(|held| {
                let name = held.name.trim_start_matches("r#");
                Some((name.to_string(), *module.children.get(name)?))
            })
            .collect()
    }

    /// Every name a module exposes on its own: its public declarations, its public child
    /// modules and the names its explicit re-exports bind. A glob never shadows one of these.
    fn own_names(&self, at: usize) -> HashSet<String> {
        let mut names: HashSet<String> = self
            .declarations(at)
            .filter(|held| held.visibility == Visibility::Public)
            .filter(|held| held.kind != DeclarationKind::Method)
            .map(|held| held.name.clone())
            .collect();
        names.extend(self.public_children(at).into_iter().map(|(name, _)| name));
        names.extend(
            self.exports(at)
                .flat_map(|export| &export.leaves)
                .filter_map(|leaf| leaf.name.clone()),
        );
        names
    }

    /// Every item reachable under `prefix` from one module: what it declares public, the public
    /// modules below it, and what its `pub use` trees expose.
    fn walk(&mut self, at: usize, prefix: &str) {
        if !self.walked.insert((at, prefix.to_string())) {
            return;
        }
        self.surface.files.push(self.file(at).to_string());
        for declaration in self.public_declarations(at) {
            self.expose(at, declaration, join(prefix, &declaration.name));
        }
        for (name, child) in self.public_children(at) {
            let path = join(prefix, &name);
            self.module_item(child, &path);
        }
        let own = self.own_names(at);
        for export in self.exports(at) {
            for leaf in &export.leaves {
                self.leaf(at, export, leaf, prefix, &own);
            }
        }
    }

    /// The declarations one module exposes as items of its own: public, and not a method.
    fn public_declarations(&self, at: usize) -> Vec<&'a Declaration> {
        self.declarations(at)
            .filter(|held| held.visibility == Visibility::Public)
            .filter(|held| held.kind != DeclarationKind::Method)
            .collect()
    }

    /// One leaf of a `pub use`: under its own name, or every name of the module it globs.
    fn leaf(
        &mut self,
        at: usize,
        export: &'a Export,
        leaf: &'a ExportLeaf,
        prefix: &str,
        own: &HashSet<String>,
    ) {
        match &leaf.name {
            Some(name) => self.named(at, export, leaf, join(prefix, name)),
            None => self.glob(at, export, leaf, prefix, own),
        }
    }

    /// One public module as an item, and everything under it.
    fn module_item(&mut self, at: usize, path: &str) {
        self.surface.items.push(Item {
            path: path.to_string(),
            kind: MODULE,
            origin: Some((self.file(at).to_string(), 1)),
            contract: super::Contract::Opaque(None),
        });
        self.walk(at, path);
    }

    /// One declaration as an item under `path`, with the public inherent methods of a type
    /// under it. A method whose `impl` sits in another module attaches where the declaring
    /// target declares exactly one type of that name, and is a hole otherwise.
    fn expose(&mut self, at: usize, declaration: &'a Declaration, path: String) {
        let file = self.file(at);
        self.surface
            .items
            .push(declared(path.clone(), file, declaration));
        let Some(target) = self
            .module(at)
            .target
            .filter(|_| declaration.kind == DeclarationKind::Type)
        else {
            return;
        };
        self.index(target);
        let key = (target, declaration.name.as_str());
        let methods = self.methods.get(&key).cloned().unwrap_or_default();
        let declared_in = self.types.get(&key).map_or(0, |modules| modules.len());
        for (holder, method) in methods {
            let holder_file = self.file(holder);
            if holder == at || declared_in == 1 {
                self.surface
                    .items
                    .push(declared(join(&path, &method.name), holder_file, method));
            } else {
                self.surface.holes.push(Hole {
                    file: holder_file.to_string(),
                    line: method.line,
                    text: method.text.clone(),
                    why: format!(
                        "an inherent impl of {} outside the module that declares it, and {declared_in} modules declare a type of that name",
                        declaration.name
                    ),
                });
            }
        }
    }

    /// One named leaf of a `pub use` or a `pub extern crate`, exposed under `path`: a module and
    /// everything under it, a declaration of the module the path reaches, a re-export that
    /// module makes under the name, or an opaque item where klin proves the name is exposed and
    /// no more. A crate root is opaque, because its items are judged under its own surface.
    fn named(&mut self, from: usize, export: &'a Export, leaf: &'a ExportLeaf, path: String) {
        let file = self.file(from);
        match self.graph.resolve(from, &leaf.path) {
            Resolved::Module { module, rest }
                if rest.is_empty() && self.module(module).parent.is_some() =>
            {
                self.module_item(module, &path)
            }
            Resolved::Module { module, rest } if rest.len() == 1 => {
                if !self.named_in(module, &rest[0], path.clone()) {
                    self.surface.items.push(opaque(
                        path,
                        ITEM,
                        file,
                        export.line,
                        leaf.path.clone(),
                    ));
                }
            }
            Resolved::Module { .. } | Resolved::External => {
                self.surface
                    .items
                    .push(opaque(path, ITEM, file, export.line, leaf.path.clone()));
            }
            Resolved::Unresolved => self.surface.holes.push(Hole {
                file: file.to_string(),
                line: export.line,
                text: export.text.clone(),
                why: unresolved(&leaf.path),
            }),
        }
    }

    /// One name looked up in the module a path reached: its public declarations of that name,
    /// the re-exports it makes under that name, or else what its globs provide under it. False
    /// where none gives the name.
    fn named_in(&mut self, module: usize, name: &str, path: String) -> bool {
        let declarations: Vec<&Declaration> = self
            .public_declarations(module)
            .into_iter()
            .filter(|held| held.name == name)
            .collect();
        for declaration in &declarations {
            self.expose(module, declaration, path.clone());
        }
        let re_exported: Vec<(&Export, &ExportLeaf)> = self
            .exports(module)
            .flat_map(|held| held.leaves.iter().map(move |leaf| (held, leaf)))
            .filter(|(_, held)| held.name.as_deref() == Some(name))
            .collect();
        for (held, inner) in &re_exported {
            self.named(module, held, inner, path.clone());
        }
        !declarations.is_empty() || !re_exported.is_empty() || self.globbed_in(module, name, path)
    }

    /// One name looked up through the globs of a module in source order, until a module one of
    /// them reaches gives it. A glob that leads back into the same lookup gives nothing.
    fn globbed_in(&mut self, module: usize, name: &str, path: String) -> bool {
        let key = (module, path);
        if !self.sought.insert(key.clone()) {
            return false;
        }
        let reached: Vec<usize> = self
            .exports(module)
            .flat_map(|held| &held.leaves)
            .filter(|leaf| leaf.name.is_none())
            .filter_map(|leaf| {
                let target = leaf.path.trim_end_matches("::*");
                match self.graph.resolve(module, target) {
                    Resolved::Module { module, rest } if rest.is_empty() => Some(module),
                    _ => None,
                }
            })
            .collect();
        let found = reached
            .into_iter()
            .any(|at| self.named_in(at, name, key.1.clone()));
        self.sought.remove(&key);
        found
    }

    /// One glob of a `pub use`: every name the module it reaches exposes, under `prefix`, less
    /// the names the globbing module exposes itself. A name two globs provide is a hole, and so
    /// is a glob over anything klin cannot list.
    fn glob(
        &mut self,
        from: usize,
        export: &'a Export,
        leaf: &'a ExportLeaf,
        prefix: &str,
        shadow: &HashSet<String>,
    ) {
        let key = (from, leaf.path.clone(), prefix.to_string());
        if !self.globbed.insert(key) {
            return;
        }
        let Some(reached) = self.globbed_module(from, export, leaf) else {
            return;
        };
        let mut inner_shadow = shadow.clone();
        inner_shadow.extend(self.own_names(reached));
        for held in self.exports(reached) {
            for inner in held.leaves.iter().filter(|inner| inner.name.is_none()) {
                self.glob(reached, held, inner, prefix, &inner_shadow);
            }
        }
        let site = (self.file(from).to_string(), export.line);
        for (name, exposure) in self.provided(reached) {
            if !shadow.contains(&name) {
                self.provide(reached, export, &site, join(prefix, &name), exposure);
            }
        }
    }

    /// One name a glob provides under `path`: a hole where another glob already provides it,
    /// and the exposure itself otherwise.
    fn provide(
        &mut self,
        reached: usize,
        export: &'a Export,
        site: &(String, u64),
        path: String,
        exposure: Exposure<'a>,
    ) {
        if let Some(other) = self.glob_names.get(&path).filter(|other| *other != site) {
            self.surface.holes.push(Hole {
                file: site.0.clone(),
                line: export.line,
                text: export.text.clone(),
                why: format!(
                    "{} is provided by this glob and by the glob at {}:{}",
                    path.rsplit("::").next().unwrap_or(&path),
                    other.0,
                    other.1
                ),
            });
            return;
        }
        self.glob_names.insert(path.clone(), site.clone());
        match exposure {
            Exposure::Declaration(declaration) => self.expose(reached, declaration, path),
            Exposure::Module(child) => self.module_item(child, &path),
            Exposure::Leaf(held, inner) => self.named(reached, held, inner, path),
        }
    }

    /// The module a glob reaches, or the hole that says why klin cannot list its names.
    fn globbed_module(&mut self, from: usize, export: &Export, leaf: &ExportLeaf) -> Option<usize> {
        let target = leaf.path.trim_end_matches("::*");
        let why = match self.graph.resolve(from, target) {
            Resolved::Module { module, rest } if rest.is_empty() => return Some(module),
            Resolved::Module { .. } => format!(
                "{} globs an item, not a module, so klin cannot list what it exposes",
                leaf.path
            ),
            Resolved::External => format!(
                "{} globs another crate, whose names klin cannot list",
                leaf.path
            ),
            Resolved::Unresolved => unresolved(&leaf.path),
        };
        self.surface.holes.push(Hole {
            file: self.file(from).to_string(),
            line: export.line,
            text: export.text.clone(),
            why,
        });
        None
    }

    /// Every name a module hands to a glob of it: its public declarations, its public child
    /// modules and its own named re-exports.
    fn provided(&self, reached: usize) -> Vec<(String, Exposure<'a>)> {
        let mut out: Vec<(String, Exposure<'a>)> = self
            .public_declarations(reached)
            .into_iter()
            .map(|declaration| (declaration.name.clone(), Exposure::Declaration(declaration)))
            .collect();
        out.extend(
            self.public_children(reached)
                .into_iter()
                .map(|(name, child)| (name, Exposure::Module(child))),
        );
        for held in self.exports(reached) {
            for inner in &held.leaves {
                if let Some(name) = &inner.name {
                    out.push((name.clone(), Exposure::Leaf(held, inner)));
                }
            }
        }
        out
    }
}

fn unresolved(path: &str) -> String {
    format!("{path} names a module no file answers or goes above the crate root")
}

/// One way a module exposes a name a glob picks up.
enum Exposure<'a> {
    Declaration(&'a Declaration),
    Module(usize),
    Leaf(&'a Export, &'a ExportLeaf),
}

fn join(prefix: &str, name: &str) -> String {
    match prefix.is_empty() {
        true => name.to_string(),
        false => format!("{prefix}::{name}"),
    }
}
