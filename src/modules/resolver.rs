use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::rc::Rc;

use crate::syntax::structural::LanguageId;
use crate::syntax::structural::facts::FileFacts;

/// One tree as the graph reads it: every file under the path its topology gives it, the facts of
/// its structural files, and where each path's bytes sit. The base tree names a renamed file by
/// the path it had at the base, so the base's topology is the base's own. Spec 8.4.
pub struct Topology<'a> {
    pub(super) root: &'a Path,
    pub(super) files: Vec<String>,
    pub(super) physical: HashMap<String, String>,
    pub(super) facts: HashMap<String, Rc<FileFacts>>,
    pub(super) present: Vec<LanguageId>,
}

impl<'a> Topology<'a> {
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
    pub(super) fn ignored(&self, path: &str) -> bool {
        !self.holds(path) && self.root.join(self.held_path(path)).is_file()
    }

    pub(super) fn held_path<'p>(&'p self, path: &'p str) -> &'p str {
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
    pub(super) fn under<'s>(&'s self, directory: &str) -> impl Iterator<Item = &'s str> {
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

impl TargetKind {
    pub fn word(self) -> &'static str {
        match self {
            TargetKind::Library => "lib",
            TargetKind::Binary => "bin",
        }
    }
}

/// One Cargo target, or one conventional root standing in for it: the package that owns it, the
/// crate name a consumer addresses it by, its kind, its root file, the manifest that named it,
/// its root module in the graph, and the names of its extern prelude that reach a library the
/// tree holds, each to that library's root module: a dependency its manifest names by path, and
/// an alias an `extern crate` at the top of its root gives one, such as
/// `pub extern crate wgpu_types as wgt;`.
pub struct Target {
    pub package: String,
    pub name: String,
    pub kind: TargetKind,
    pub root: String,
    pub manifest: Option<String>,
    pub module: usize,
    pub crates: BTreeMap<String, usize>,
}

/// One module: the name a report prints, which is its resolver's identity for it, and the
/// physical files that hold it. A module holds at least one file, in path order, each once, and
/// the order means nothing. One file may hold several modules, and one file under two targets is
/// a module under each. A Rust module is one file or inline in one, and a TypeScript module is
/// one file; a resolver for another language may group several files into one module. A Rust
/// module also knows its place in its target's tree; a TypeScript module stands alone. ADR 0058.
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
    /// The names this module binds in the type namespace other than its child modules: each
    /// name a `use` binds and each type it declares. A path from one is a local item and never
    /// reaches a crate of the same name.
    pub bound: BTreeSet<String>,
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
    /// Whether an explicit TypeScript paths rule recognized this dependency as local.
    pub local_alias: bool,
    pub file: String,
    pub line: u64,
    pub text: String,
    pub why: String,
}

/// What a resolver adds to the graph it builds.
pub(crate) struct Builder<'a> {
    pub(super) topology: &'a Topology<'a>,
    pub(super) modules: Vec<Module>,
    pub(super) targets: Vec<Target>,
    pub(super) dependencies: Vec<Dependency>,
    pub(super) holes: Vec<Hole>,
    pub(super) external: usize,
    pub(super) attached: BTreeMap<String, Attachment>,
}

impl<'a> Builder<'a> {
    pub(super) fn new(topology: &'a Topology<'a>) -> Builder<'a> {
        Builder {
            topology,
            modules: Vec::new(),
            targets: Vec::new(),
            dependencies: Vec::new(),
            holes: Vec::new(),
            external: 0,
            attached: BTreeMap::new(),
        }
    }

    /// A module of these files, each of which counts as attached on its own.
    pub(super) fn module(&mut self, name: String, files: &[&str], attachment: Attachment) -> usize {
        let mut sources: Vec<String> = files.iter().map(|file| file.to_string()).collect();
        sources.sort_unstable();
        sources.dedup();
        assert!(!sources.is_empty(), "a module holds at least one file");
        for file in &sources {
            let held = self.attached.entry(file.clone()).or_insert(attachment);
            *held = (*held).min(attachment);
        }
        self.modules.push(Module {
            name,
            sources,
            nesting: Vec::new(),
            target: None,
            parent: None,
            children: BTreeMap::new(),
            unresolved: BTreeSet::new(),
            bound: BTreeSet::new(),
        });
        self.modules.len() - 1
    }

    /// A dependency written in `file`, which is one of the writing module's own sources.
    pub(super) fn depend(&mut self, from: usize, to: usize, file: &str, line: u64) {
        let written = self.modules[from]
            .sources
            .binary_search_by(|held| held.as_str().cmp(file));
        match written {
            Ok(source) => self.dependencies.push(Dependency {
                from,
                to,
                source: source as u32,
                line,
            }),
            Err(_) => self.hole(
                file,
                line,
                &self.modules[to].name.clone(),
                "a dependency from a file that is not one of its module's files".to_string(),
            ),
        }
    }

    pub(super) fn hole(&mut self, file: &str, line: u64, text: &str, why: String) {
        self.holes.push(Hole {
            local_alias: false,
            file: file.to_string(),
            line,
            text: text.to_string(),
            why,
        });
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
