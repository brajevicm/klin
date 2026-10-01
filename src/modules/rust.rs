//! Rust modules. A target root comes from a Cargo manifest, read through `cargo_toml` over the
//! tree's own file list, and from a conventional root only where no manifest above the file is
//! usable. From each root the resolver follows `mod` declarations to files, `#[path]` included,
//! and makes every file it reaches a module of that target, so a file two targets reach is a
//! module of each. A dependency is a path a `use` tree or a qualified path writes from `crate`,
//! `self` or `super`, resolved to the deepest module it names. A path from any other name may be
//! another crate or a local item, so it is counted and never resolved. Each target's extern
//! prelude names the libraries of the tree its manifest takes by path, so a consumer that
//! follows a path into another crate reaches the one Cargo would build.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::iter::Peekable;
use std::path::{Path, PathBuf};

use cargo_toml::{AbstractFilesystem, Manifest, Value};

use super::{Attachment, Builder, TargetKind, Topology, directory, joined};
use crate::survey;
use crate::syntax::structural::{DeclarationKind, ModuleDecl};

const MANIFEST: &str = "Cargo.toml";

struct Target {
    root: String,
    attachment: Attachment,
    package: String,
    name: String,
    kind: TargetKind,
    manifest: Option<String>,
    dependencies: Vec<PathDependency>,
}

/// One normal dependency a manifest takes from the tree by path: the name a target writes for
/// it where the manifest renames it, and the manifest of the package it names.
#[derive(Clone)]
struct PathDependency {
    renamed: Option<String>,
    manifest: String,
}

/// One lib or bin product a manifest names: what kind it is, the name a consumer addresses it
/// by, and the root file relative to the manifest.
struct Product {
    kind: TargetKind,
    name: String,
    path: String,
}

/// One module of one target: where it sits in the graph, the module that holds it, the modules
/// it declares by name, the names it declares and no file answers, and the directory its child
/// module files sit in.
struct Node {
    index: usize,
    parent: Option<usize>,
    children: BTreeMap<String, usize>,
    unresolved: HashSet<String>,
    directory: String,
    file: String,
    nesting: Vec<String>,
}

enum Reached {
    Module(usize),
    External,
    Above,
    Reported,
}

pub(super) fn resolve(builder: &mut Builder) {
    let targets = targets(builder);
    let first = builder.graph.targets.len();
    for target in &targets {
        let mut krate = Crate {
            target,
            nodes: Vec::new(),
            nestings: BTreeMap::new(),
        };
        let root = krate.add(
            builder,
            &target.root,
            Vec::new(),
            directory(&target.root),
            None,
        );
        let mut pending = vec![root];
        while let Some(file) = pending.pop() {
            pending.extend(krate.declared(builder, file));
        }
        krate.depend(builder);
        krate.publish(builder, root);
    }
    let libraries: BTreeMap<String, (String, usize)> = builder.graph.targets[first..]
        .iter()
        .filter(|target| target.kind == TargetKind::Library)
        .filter_map(|target| {
            Some((
                target.manifest.clone()?,
                (target.name.clone(), target.module),
            ))
        })
        .collect();
    for (at, target) in targets.iter().enumerate() {
        builder.graph.targets[first + at].crates = prelude(builder.topology, target, &libraries);
    }
}

/// The names of one target's extern prelude that reach a library the tree holds, each to that
/// library's root module: every dependency its manifest takes by path, under its rename or else
/// the library's own name, and every alias an `extern crate` at the top of its root gives one of
/// those dependencies. An alias an `extern crate` gives any other crate names that crate, so it
/// reaches no library of the tree.
fn prelude(
    topology: &Topology,
    target: &Target,
    libraries: &BTreeMap<String, (String, usize)>,
) -> BTreeMap<String, usize> {
    let dependencies: BTreeMap<String, usize> = target
        .dependencies
        .iter()
        .filter_map(|dependency| {
            let (name, module) = libraries.get(&dependency.manifest)?;
            Some((
                dependency.renamed.clone().unwrap_or_else(|| name.clone()),
                *module,
            ))
        })
        .collect();
    let mut out = dependencies.clone();
    let crates = topology
        .facts(&target.root)
        .into_iter()
        .flat_map(|facts| &facts.crates);
    for held in crates.filter(|held| held.nesting.is_empty()) {
        match dependencies.get(&held.name) {
            Some(&module) => out.insert(held.alias.clone(), module),
            None => out.remove(&held.alias),
        };
    }
    out
}

/// Every target root: the lib and bin targets each usable manifest names, then a conventional
/// root for every file no usable manifest above it speaks for.
fn targets(builder: &mut Builder) -> Vec<Target> {
    let topology = builder.topology;
    let mut read: Vec<(&str, bool)> = Vec::new();
    let mut out = Vec::new();
    let manifests = topology
        .files
        .iter()
        .filter(|file| basename(file) == MANIFEST && survey::surveyed(file));
    for manifest in manifests {
        let products = products(topology, manifest);
        read.push((directory(manifest), products.is_some()));
        let Some((package, products, dependencies)) = products else {
            continue;
        };
        for product in products {
            match joined(directory(manifest), &product.path).filter(|root| topology.holds(root)) {
                Some(root) => out.push(Target {
                    root,
                    attachment: Attachment::Manifest,
                    package: package.clone(),
                    name: product.name,
                    kind: product.kind,
                    manifest: Some(manifest.clone()),
                    dependencies: dependencies.clone(),
                }),
                None => builder.hole(
                    manifest,
                    0,
                    &product.path,
                    format!(
                        "names the {} target root {}, which the tree does not hold",
                        word(product.kind, &product.name),
                        product.path
                    ),
                ),
            }
        }
    }
    out.extend(conventional(topology, &read));
    out
}

fn word(kind: TargetKind, name: &str) -> String {
    match kind {
        TargetKind::Library => "lib".to_string(),
        TargetKind::Binary => format!("bin {name}"),
    }
}

/// The package name, the lib and bin targets and the path dependencies of one manifest, and
/// `None` where the manifest is not usable.
fn products(
    topology: &Topology,
    manifest: &str,
) -> Option<(String, Vec<Product>, Vec<PathDependency>)> {
    let mut parsed = Manifest::from_slice(&topology.read(manifest)?).ok()?;
    let listing = Listing {
        topology,
        directory: directory(manifest),
    };
    parsed
        .complete_from_abstract_filesystem::<Value, _>(listing, None)
        .ok()?;
    let dependencies = dependencies(&parsed, manifest);
    let package = parsed
        .package
        .as_ref()
        .map(|package| package.name.clone())
        .unwrap_or_default();
    let mut out: Vec<Product> = parsed
        .lib
        .and_then(|lib| {
            Some(Product {
                kind: TargetKind::Library,
                name: lib.name.unwrap_or_else(|| crate_name(&package)),
                path: lib.path?,
            })
        })
        .into_iter()
        .collect();
    for bin in parsed.bin {
        if let Some(path) = bin.path {
            out.push(Product {
                kind: TargetKind::Binary,
                name: bin.name.unwrap_or_else(|| stem(&path).to_string()),
                path,
            });
        }
    }
    Some((package, out, dependencies))
}

/// The normal dependencies of a completed manifest that name a path, its target-specific ones
/// included. `cargo_toml` writes a path a member inherits from another manifest's workspace from
/// the tree root, and every other path from the manifest's own directory.
fn dependencies(parsed: &Manifest<Value>, manifest: &str) -> Vec<PathDependency> {
    parsed
        .dependencies
        .iter()
        .chain(parsed.target.values().flat_map(|held| &held.dependencies))
        .filter_map(|(name, dependency)| {
            let detail = dependency.detail()?;
            let from = match detail.inherited && parsed.workspace.is_none() {
                true => "",
                false => directory(manifest),
            };
            let package = joined(from, detail.path.as_deref()?)?;
            Some(PathDependency {
                renamed: detail.package.as_ref().map(|_| crate_name(name)),
                manifest: joined(&package, MANIFEST)?,
            })
        })
        .collect()
}

/// The name a consumer writes for a package's library: the package name with each `-` as `_`.
fn crate_name(package: &str) -> String {
    package.replace('-', "_")
}

fn stem(path: &str) -> &str {
    basename(path)
        .rsplit_once('.')
        .map_or(basename(path), |(stem, _)| stem)
}

/// The conventional roots — `src/lib.rs`, `src/main.rs` and a file directly in `src/bin` — of
/// every file whose nearest manifest is absent or not usable.
fn conventional(topology: &Topology, read: &[(&str, bool)]) -> Vec<Target> {
    topology
        .files
        .iter()
        .filter(|file| file.ends_with(".rs") && conventional_root(file))
        .filter(|file| {
            read.iter()
                .filter(|(at, _)| at.is_empty() || file.starts_with(&format!("{at}/")))
                .max_by_key(|(at, _)| at.len())
                .is_none_or(|(_, usable)| !usable)
        })
        .map(|file| {
            let package = match basename(directory(directory(file))) {
                "" => "crate".to_string(),
                above => above.to_string(),
            };
            let (kind, name) = match basename(file) {
                "lib.rs" => (TargetKind::Library, crate_name(&package)),
                "main.rs" => (TargetKind::Binary, crate_name(&package)),
                _ => (TargetKind::Binary, stem(file).to_string()),
            };
            Target {
                root: file.clone(),
                attachment: Attachment::Convention,
                package,
                name,
                kind,
                manifest: None,
                dependencies: Vec::new(),
            }
        })
        .collect()
}

fn conventional_root(file: &str) -> bool {
    let at = directory(file);
    let root = basename(at) == "src" && matches!(basename(file), "lib.rs" | "main.rs");
    root || at == "src/bin" || at.ends_with("/src/bin")
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

/// The tree's file list as the directory listing `cargo_toml` infers targets from, so a
/// manifest's implicit targets come from the files a run already listed and no disk walk.
struct Listing<'a> {
    topology: &'a Topology<'a>,
    directory: &'a str,
}

impl AbstractFilesystem for Listing<'_> {
    fn file_names_in(&self, rel_path: &str) -> io::Result<HashSet<Box<str>>> {
        let missing = || io::Error::from(io::ErrorKind::NotFound);
        let at = joined(self.directory, rel_path).ok_or_else(missing)?;
        let skip = if at.is_empty() { 0 } else { at.len() + 1 };
        let names: HashSet<Box<str>> = self
            .topology
            .under(&at)
            .filter_map(|file| file[skip..].split('/').next())
            .map(Box::from)
            .collect();
        if names.is_empty() {
            return Err(missing());
        }
        Ok(names)
    }

    fn parse_root_workspace(
        &self,
        hint: Option<&Path>,
    ) -> Result<(Manifest<Value>, PathBuf), cargo_toml::Error> {
        let candidates: Vec<String> = match hint {
            Some(hint) => joined(self.directory, &hint.to_string_lossy())
                .into_iter()
                .collect(),
            None => above(self.directory),
        };
        for at in candidates {
            let Some(bytes) = joined(&at, MANIFEST).and_then(|file| self.topology.read(&file))
            else {
                continue;
            };
            let parsed = Manifest::from_slice(&bytes)?;
            if parsed.workspace.is_some() {
                return Ok((parsed, PathBuf::from(at)));
            }
        }
        Err(cargo_toml::Error::Other(
            "no workspace manifest above the package",
        ))
    }
}

/// Every directory above this one, nearest first, ending at the tree root.
fn above(at: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut here = at;
    while !here.is_empty() {
        here = directory(here);
        out.push(here.to_string());
    }
    out
}

/// One target's module tree as the resolver builds it.
struct Crate<'t> {
    target: &'t Target,
    nodes: Vec<Node>,
    /// For each file module, the module each inline nesting in that file names.
    nestings: BTreeMap<usize, BTreeMap<Vec<String>, usize>>,
}

impl Crate<'_> {
    fn add(
        &mut self,
        builder: &mut Builder,
        file: &str,
        nesting: Vec<String>,
        children: &str,
        parent: Option<(usize, &str)>,
    ) -> usize {
        let name = match nesting.is_empty() {
            true => file.to_string(),
            false => format!("{file}::{}", nesting.join("::")),
        };
        let index = builder.module(name, &[file], self.target.attachment);
        let id = self.nodes.len();
        self.nodes.push(Node {
            index,
            parent: parent.map(|(at, _)| at),
            children: BTreeMap::new(),
            unresolved: HashSet::new(),
            directory: children.to_string(),
            file: file.to_string(),
            nesting,
        });
        if let Some((at, named)) = parent {
            self.nodes[at].children.insert(named.to_string(), id);
        }
        id
    }

    /// The modules one file module declares: each inline module in it, and each file its
    /// external declarations load, which the caller follows in turn.
    fn declared(&mut self, builder: &mut Builder, at: usize) -> Vec<usize> {
        let topology = builder.topology;
        let file = self.nodes[at].file.clone();
        let mut nestings = BTreeMap::from([(Vec::new(), at)]);
        let mut loaded = Vec::new();
        let Some(facts) = topology.facts(&file) else {
            self.nestings.insert(at, nestings);
            return loaded;
        };
        let mut declarations: Vec<&ModuleDecl> = facts.module_declarations.iter().collect();
        declarations.sort_by_key(|declaration| declaration.nesting.len());
        for declaration in declarations {
            let Some(&parent) = nestings.get(&declaration.nesting) else {
                continue;
            };
            let name = declaration.name.trim_start_matches("r#");
            if declaration.inline {
                let mut nesting = declaration.nesting.clone();
                nesting.push(name.to_string());
                let children = joined(&self.nodes[parent].directory, name).unwrap_or_default();
                let id = self.add(
                    builder,
                    &file,
                    nesting.clone(),
                    &children,
                    Some((parent, name)),
                );
                nestings.insert(nesting, id);
            } else {
                loaded.extend(self.external(builder, parent, declaration));
            }
        }
        self.nestings.insert(at, nestings);
        loaded
    }

    /// The file module one external declaration loads, where exactly one file answers it.
    fn external(
        &mut self,
        builder: &mut Builder,
        parent: usize,
        declaration: &ModuleDecl,
    ) -> Option<usize> {
        let topology = builder.topology;
        let name = declaration.name.trim_start_matches("r#");
        let candidates = self.candidates(parent, declaration, name);
        let held: Vec<&(String, String)> = candidates
            .iter()
            .filter(|(file, _)| topology.holds(file))
            .collect();
        let why = match held.as_slice() {
            [(file, children)] if !self.holds_above(parent, file) => {
                return Some(self.add(builder, file, Vec::new(), children, Some((parent, name))));
            }
            [_] => "declares a module whose file already holds it".to_string(),
            [] if candidates.iter().any(|(file, _)| topology.ignored(file)) => {
                builder.graph.external += 1;
                String::new()
            }
            [] => format!("names no file the tree holds: {}", listed(&candidates)),
            _ => format!("names more than one file: {}", listed(&candidates)),
        };
        self.nodes[parent].unresolved.insert(name.to_string());
        if !why.is_empty() {
            builder.hole(
                &self.nodes[parent].file.clone(),
                declaration.line,
                &declaration.text,
                why,
            );
        }
        None
    }

    /// The files an external declaration may load, each with the directory its own child module
    /// files sit in. A `#[path]` at the top of a file is read from that file's directory, and
    /// one inside an inline module from that module's directory. A file a `#[path]` loads holds
    /// its children ignored it, as a `mod.rs` does.
    fn candidates(
        &self,
        parent: usize,
        declaration: &ModuleDecl,
        name: &str,
    ) -> Vec<(String, String)> {
        let holder = &self.nodes[parent];
        if let Some(path) = &declaration.path {
            let from = match declaration.nesting.is_empty() {
                true => directory(&holder.file),
                false => holder.directory.as_str(),
            };
            return joined(from, path)
                .map(|file| {
                    let children = directory(&file).to_string();
                    (file, children)
                })
                .into_iter()
                .collect();
        }
        let children = joined(&holder.directory, name).unwrap_or_default();
        [format!("{name}.rs"), format!("{name}/mod.rs")]
            .iter()
            .filter_map(|file| joined(&holder.directory, file))
            .map(|file| (file, children.clone()))
            .collect()
    }

    /// Whether a file module this one sits inside is already this file.
    fn holds_above(&self, mut at: usize, file: &str) -> bool {
        loop {
            let node = &self.nodes[at];
            if node.nesting.is_empty() && node.file == file {
                return true;
            }
            match node.parent {
                Some(parent) => at = parent,
                None => return false,
            }
        }
    }

    /// The target and its module tree written into the graph, so a consumer reads a module's
    /// parent, children and target without the resolver's own nodes.
    fn publish(&self, builder: &mut Builder, root: usize) {
        let target = builder.graph.targets.len();
        builder.graph.targets.push(super::Target {
            package: self.target.package.clone(),
            name: self.target.name.clone(),
            kind: self.target.kind,
            root: self.target.root.clone(),
            manifest: self.target.manifest.clone(),
            module: self.nodes[root].index,
            crates: BTreeMap::new(),
        });
        for node in &self.nodes {
            let module = &mut builder.graph.modules[node.index];
            module.nesting = node.nesting.clone();
            module.target = Some(target);
            module.parent = node.parent.map(|parent| self.nodes[parent].index);
            module.children = node
                .children
                .iter()
                .map(|(name, child)| (name.clone(), self.nodes[*child].index))
                .collect();
            module.unresolved = node.unresolved.iter().cloned().collect();
            module.bound = bound(builder.topology, node);
        }
    }

    /// Every dependency each file module's imports and qualified paths write.
    fn depend(&self, builder: &mut Builder) {
        let topology = builder.topology;
        for (at, nestings) in &self.nestings {
            let file = &self.nodes[*at].file;
            let Some(facts) = topology.facts(file) else {
                continue;
            };
            for import in &facts.imports {
                for path in nestings
                    .get(&import.nesting)
                    .into_iter()
                    .flat_map(|from| import.paths.iter().map(move |path| (*from, path)))
                {
                    self.dependency(builder, path, (file, import.line), &import.text);
                }
            }
            for path in &facts.paths {
                if let Some(from) = nestings.get(&path.nesting) {
                    self.dependency(builder, (*from, &path.path), (file, path.line), &path.path);
                }
            }
        }
    }

    fn dependency(
        &self,
        builder: &mut Builder,
        (from, path): (usize, &String),
        (file, line): (&str, u64),
        text: &str,
    ) {
        match self.target_of(from, path) {
            Reached::Module(to) if to != from => {
                builder.depend(self.nodes[from].index, self.nodes[to].index, file, line);
            }
            Reached::External => builder.graph.external += 1,
            Reached::Above => builder.hole(
                &self.nodes[from].file,
                line,
                text,
                format!("{path} goes above the crate root"),
            ),
            Reached::Module(_) | Reached::Reported => {}
        }
    }

    /// The deepest module a path names from this module: `crate` starts at the target root,
    /// `self` and `super` at this module and the ones above it, a child module this module
    /// declares at this module, as if `self::` came first, and each name after them at the child
    /// module of that name, until a name is no module.
    fn target_of(&self, from: usize, path: &str) -> Reached {
        let mut segments = path.split("::").peekable();
        let start = match segments.peek() {
            Some(&"crate") => segments.next().map(|_| 0),
            Some(&"self" | &"super") => Some(from),
            Some(first) if self.declares(from, first.trim_start_matches("r#")) => Some(from),
            _ => return Reached::External,
        };
        match start.and_then(|at| self.ascended(at, &mut segments)) {
            Some(at) => self.descended(at, segments),
            None => Reached::Above,
        }
    }

    fn declares(&self, at: usize, name: &str) -> bool {
        let node = &self.nodes[at];
        node.children.contains_key(name) || node.unresolved.contains(name)
    }

    fn ascended<'p>(
        &self,
        mut at: usize,
        segments: &mut Peekable<impl Iterator<Item = &'p str>>,
    ) -> Option<usize> {
        while let Some(segment) = segments.next_if(|segment| matches!(*segment, "self" | "super")) {
            if segment == "super" {
                at = self.nodes[at].parent?;
            }
        }
        Some(at)
    }

    fn descended<'p>(&self, mut at: usize, segments: impl Iterator<Item = &'p str>) -> Reached {
        for segment in segments.map(|segment| segment.trim_start_matches("r#")) {
            let node = &self.nodes[at];
            match node.children.get(segment) {
                Some(child) => at = *child,
                None if node.unresolved.contains(segment) => return Reached::Reported,
                None => break,
            }
        }
        Reached::Module(at)
    }
}

/// The names one module binds in the type namespace, as its file's facts give them under its
/// nesting: each name a `use` outside a function binds, each type it declares as an item, and,
/// below the target root, each alias an `extern crate` binds. The root's own aliases are names of
/// the extern prelude.
fn bound(topology: &Topology, node: &Node) -> BTreeSet<String> {
    let Some(facts) = topology.facts(&node.file) else {
        return BTreeSet::new();
    };
    let imported = facts
        .imports
        .iter()
        .filter(|import| import.nesting == node.nesting && !import.in_function)
        .flat_map(|import| import.names.iter().cloned());
    let declared = facts
        .declarations
        .iter()
        .filter(|held| held.nesting == node.nesting && !held.associated)
        .filter(|held| held.kind == DeclarationKind::Type)
        .map(|held| held.name.clone());
    let aliased = facts
        .crates
        .iter()
        .filter(|held| node.parent.is_some() && held.nesting == node.nesting)
        .map(|held| held.alias.clone());
    imported.chain(declared).chain(aliased).collect()
}

fn listed(candidates: &[(String, String)]) -> String {
    candidates
        .iter()
        .map(|(file, _)| file.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
