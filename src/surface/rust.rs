//! Rust surfaces. A Cargo library target is one surface, named by its crate name. From its root
//! module the derivation follows every `pub mod` and every `pub use`, so an item's path is the
//! path a consumer writes and never the file that declares it. A plain `pub` item inside a
//! private module is external only where a `pub use` exposes it. A restricted visibility is
//! never external. A re-export that reaches a library target the tree holds is followed into
//! its source. A re-export of any other crate, of an enum variant, or of a name klin cannot find
//! is an opaque item whose clause is its contract; a glob klin cannot list is a hole.

use std::collections::{HashMap, HashSet};

use super::item::{Found, Hole, ITEM, Inapplicable, Item, MODULE, Surface, declared, opaque};
use crate::modules::resolver::{Module, TargetKind, Topology};
use crate::modules::{ModuleGraph, Resolved};
use crate::syntax::structural::facts::{
    Declaration, DeclarationKind, Export, ExportLeaf, Visibility,
};

const LANGUAGE: &str = "Rust";

pub(super) fn derive(topology: &Topology, graph: &ModuleGraph, out: &mut Found) {
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
        derivation.walk(target.module, "", None);
        out.surfaces.push(derivation.surface);
    }
    for (what, _) in packages.into_iter().filter(|(_, library)| !library) {
        out.inapplicable.push(Inapplicable {
            what,
            why: "has no library target, and a binary has no public surface".to_string(),
        });
    }
}

/// One library target being derived: its surface so far, the public inherent items and type
/// declarations of each target it reaches by that target and name, the targets indexed so far,
/// and what has been walked or is being looked up through globs, so a cycle of re-exports ends.
struct Derivation<'a> {
    topology: &'a Topology<'a>,
    graph: &'a ModuleGraph,
    surface: Surface,
    members: HashMap<(usize, &'a str), Vec<(usize, &'a Declaration)>>,
    types: HashMap<(usize, &'a str), Vec<usize>>,
    indexed: HashSet<usize>,
    walked: HashSet<(usize, String)>,
    walking: HashSet<usize>,
    naming: HashSet<(usize, &'a str)>,
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
                manifest: named.manifest.clone(),
                language: LANGUAGE,
                source: match &named.manifest {
                    Some(manifest) => format!("library target {} of {manifest}", named.root),
                    None => format!("conventional library root {}", named.root),
                },
                items: Vec::new(),
                files: Vec::new(),
                holes: Vec::new(),
            },
            members: HashMap::new(),
            types: HashMap::new(),
            indexed: HashSet::new(),
            walked: HashSet::new(),
            walking: HashSet::new(),
            naming: HashSet::new(),
            sought: HashSet::new(),
            globbed: HashSet::new(),
            glob_names: HashMap::new(),
        }
    }

    /// Every public inherent item of one target, a method or an associated constant or type, by
    /// the type it is added to, and every type declared as a module item by name, so an inherent
    /// item finds its type wherever its `impl` sits. A target is indexed once, when a type of it
    /// is first exposed.
    fn index(&mut self, target: usize) {
        if !self.indexed.insert(target) {
            return;
        }
        for (at, module) in self.graph.modules.iter().enumerate() {
            if module.target != Some(target) {
                continue;
            }
            for declaration in self.declarations(at) {
                match (declaration.associated, &declaration.owner) {
                    (true, Some(owner)) if declaration.visibility == Visibility::Public => {
                        self.members
                            .entry((target, owner))
                            .or_default()
                            .push((at, declaration));
                    }
                    (false, _) if declaration.kind == DeclarationKind::Type => {
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

    /// The declarations that are items of one module, less the associated items a type body
    /// holds.
    fn items(&self, at: usize) -> impl Iterator<Item = &'a Declaration> + use<'a> {
        self.declarations(at).filter(|held| !held.associated)
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
            .items(at)
            .filter(|held| held.visibility == Visibility::Public)
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

    /// Every name a module binds itself at any visibility, in its namespace: each item it
    /// declares, each child module and `extern crate` alias as a type, and each name a `use`
    /// outside a function binds in both. A glob of the module never provides a name in a
    /// namespace the module binds it in.
    fn shadowing(&self, at: usize) -> Shadow {
        let module = self.module(at);
        let facts = self.topology.facts(self.file(at));
        let mut out: Shadow = self
            .items(at)
            .flat_map(|held| {
                namespaces(held.kind)
                    .iter()
                    .map(|space| (held.name.clone(), *space))
            })
            .collect();
        out.extend(
            module
                .children
                .keys()
                .map(|name| (name.clone(), Namespace::Type)),
        );
        let crates = facts.iter().flat_map(|facts| &facts.crates);
        out.extend(
            crates
                .filter(|held| held.nesting == module.nesting)
                .map(|held| (held.alias.clone(), Namespace::Type)),
        );
        let imports = facts.iter().flat_map(|facts| &facts.imports);
        for import in imports.filter(|held| held.nesting == module.nesting && !held.in_function) {
            for name in &import.names {
                out.extend(BOTH.iter().map(|space| (name.clone(), *space)));
            }
        }
        out
    }

    /// Every item reachable under `prefix` from one module: what it declares public, the public
    /// modules below it, and what its `pub use` trees expose. `via` stays with public children
    /// until another re-export replaces it; containment alone cannot close a cycle.
    fn walk(&mut self, at: usize, prefix: &str, via: Option<(&str, &Export)>) {
        if !self.walking.insert(at) {
            if let Some((file, export)) = via {
                self.surface.holes.push(Hole {
                    file: file.to_string(),
                    line: export.line,
                    text: export.text.clone(),
                    why: "cyclic module re-export gives unbounded public paths".to_string(),
                });
            }
            return;
        }
        if !self.walked.insert((at, prefix.to_string())) {
            self.walking.remove(&at);
            return;
        }
        self.surface.files.push(self.file(at).to_string());
        for declaration in self.public_declarations(at) {
            self.expose(at, declaration, join(prefix, &declaration.name));
        }
        for (name, child) in self.public_children(at) {
            let path = join(prefix, &name);
            self.module_item(child, &path, via);
        }
        let own = self.shadowing(at);
        for export in self.exports(at) {
            for leaf in &export.leaves {
                self.leaf(at, export, leaf, prefix, &own);
            }
        }
        self.walking.remove(&at);
    }

    /// The declarations one module exposes as items of its own: public, and no associated item.
    fn public_declarations(&self, at: usize) -> Vec<&'a Declaration> {
        self.items(at)
            .filter(|held| held.visibility == Visibility::Public)
            .collect()
    }

    /// One leaf of a `pub use`: under its own name, or every name of the module it globs.
    fn leaf(
        &mut self,
        at: usize,
        export: &'a Export,
        leaf: &'a ExportLeaf,
        prefix: &str,
        own: &Shadow,
    ) {
        match &leaf.name {
            Some(name) => self.named(at, export, leaf, join(prefix, name)),
            None => self.glob(at, export, leaf, prefix, own),
        }
    }

    /// One public module as an item, and everything under it.
    fn module_item(&mut self, at: usize, path: &str, via: Option<(&str, &Export)>) {
        self.surface.items.push(Item {
            path: path.to_string(),
            kind: MODULE,
            origin: Some((self.file(at).to_string(), 1)),
            contract: super::item::Contract::Opaque(None),
        });
        self.walk(at, path, via);
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
        let methods = self.members.get(&key).cloned().unwrap_or_default();
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
        let key = (from, leaf.path.as_str());
        if !self.naming.insert(key) {
            self.surface.holes.push(Hole {
                file: file.to_string(),
                line: export.line,
                text: export.text.clone(),
                why: "cyclic named re-export cannot be resolved".to_string(),
            });
            return;
        }
        match self.graph.resolve(from, &leaf.path) {
            Resolved::Module { module, rest }
                if rest.is_empty() && self.module(module).parent.is_some() =>
            {
                self.module_item(module, &path, Some((file, export)))
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
        self.naming.remove(&key);
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

    /// One name looked up through the globs of a module: what the one glob that provides it
    /// gives, and a hole where two do, as a glob walk reports them. A glob that leads back into
    /// the same lookup gives nothing.
    fn globbed_in(&mut self, module: usize, name: &str, path: String) -> bool {
        let key = (module, path);
        if hides(&self.shadowing(module), name, BOTH) || !self.sought.insert(key.clone()) {
            return false;
        }
        let providers: Vec<(&Export, usize)> = self
            .globs(module)
            .into_iter()
            .filter(|(_, at)| self.provides(*at, name, &mut HashSet::new()))
            .collect();
        let found = match providers.as_slice() {
            [] => false,
            [(_, at)] => self.named_in(*at, name, key.1.clone()),
            [(first, _), (second, _), ..] => {
                let site = (self.file(module).to_string(), first.line);
                self.surface
                    .holes
                    .push(twice_globbed(&site.0, second, name, &site));
                true
            }
        };
        self.sought.remove(&key);
        found
    }

    /// Each glob of a module whose path reaches a module, with that module.
    fn globs(&self, module: usize) -> Vec<(&'a Export, usize)> {
        self.exports(module)
            .flat_map(|held| held.leaves.iter().map(move |leaf| (held, leaf)))
            .filter(|(_, leaf)| leaf.name.is_none())
            .filter_map(|(held, leaf)| {
                match self
                    .graph
                    .resolve(module, leaf.path.trim_end_matches("::*"))
                {
                    Resolved::Module { module, rest } if rest.is_empty() => Some((held, module)),
                    _ => None,
                }
            })
            .collect()
    }

    /// Whether a module gives a name: as one of its own, or through a glob of it where the
    /// module binds no name of its own under it.
    fn provides(&self, module: usize, name: &str, seen: &mut HashSet<usize>) -> bool {
        seen.insert(module)
            && (self.own_names(module).contains(name)
                || !hides(&self.shadowing(module), name, BOTH)
                    && self
                        .globs(module)
                        .into_iter()
                        .any(|(_, at)| self.provides(at, name, seen)))
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
        shadow: &Shadow,
    ) {
        let key = (from, leaf.path.clone(), prefix.to_string());
        if !self.globbed.insert(key) {
            return;
        }
        let Some(reached) = self.globbed_module(from, export, leaf) else {
            return;
        };
        let mut inner_shadow = shadow.clone();
        inner_shadow.extend(self.shadowing(reached));
        for held in self.exports(reached) {
            for inner in held.leaves.iter().filter(|inner| inner.name.is_none()) {
                self.glob(reached, held, inner, prefix, &inner_shadow);
            }
        }
        let site = (self.file(from).to_string(), export.line);
        for (name, exposure) in self.provided(reached) {
            if !hides(shadow, &name, exposure.namespaces()) {
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
            let name = path.rsplit("::").next().unwrap_or(&path);
            let hole = twice_globbed(&site.0, export, name, other);
            self.surface.holes.push(hole);
            return;
        }
        self.glob_names.insert(path.clone(), site.clone());
        match exposure {
            Exposure::Declaration(declaration) => self.expose(reached, declaration, path),
            Exposure::Module(child) => self.module_item(child, &path, Some((&site.0, export))),
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

/// The hole a name is where the glob `export` in `file` provides it and so does the glob at
/// `other`.
fn twice_globbed(file: &str, export: &Export, name: &str, other: &(String, u64)) -> Hole {
    Hole {
        file: file.to_string(),
        line: export.line,
        text: export.text.clone(),
        why: format!(
            "{name} is provided by this glob and by the glob at {}:{}",
            other.0, other.1
        ),
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

impl Exposure<'_> {
    /// The namespaces the exposed name lives in, and both for a re-export klin has not followed.
    fn namespaces(&self) -> &'static [Namespace] {
        match self {
            Exposure::Declaration(declaration) => namespaces(declaration.kind),
            Exposure::Module(_) => &[Namespace::Type],
            Exposure::Leaf(..) => BOTH,
        }
    }
}

/// Where a Rust name lives: types, traits and modules apart from functions and values, so a
/// binding hides a name a glob provides only in its own namespace.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Namespace {
    Type,
    Value,
}

const BOTH: &[Namespace] = &[Namespace::Type, Namespace::Value];

/// Each name a module binds, once for every namespace it binds the name in.
type Shadow = HashSet<(String, Namespace)>;

fn namespaces(kind: DeclarationKind) -> &'static [Namespace] {
    match kind {
        DeclarationKind::Type => &[Namespace::Type],
        DeclarationKind::Function | DeclarationKind::Constant | DeclarationKind::Variable => {
            &[Namespace::Value]
        }
        DeclarationKind::Method => &[],
    }
}

/// Whether a module's bindings hide a name in any of the namespaces it lives in.
fn hides(shadow: &Shadow, name: &str, spaces: &[Namespace]) -> bool {
    spaces
        .iter()
        .any(|space| shadow.contains(&(name.to_string(), *space)))
}

fn join(prefix: &str, name: &str) -> String {
    match prefix.is_empty() {
        true => name.to_string(),
        false => format!("{prefix}::{name}"),
    }
}
