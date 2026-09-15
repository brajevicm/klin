//! TypeScript surfaces. A surface begins only at explicit package metadata: an `exports` target,
//! a `types` or `typings` entry, or a `main` or `module` that itself names a TypeScript source
//! file the tree holds. Generated JavaScript is never reverse-mapped and no `src/index.ts` is
//! guessed. From an entry file the derivation follows exported declarations, local export
//! clauses, default exports and relative re-exports through the module graph's own edges. A
//! re-export of another package is an opaque item whose clause is its contract, and a star that
//! klin cannot list is a hole.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::Value;

use super::{Derived, Hole, ITEM, Inapplicable, Item, NAMESPACE, Surface, TYPE, declared, opaque};
use crate::modules::{ModuleGraph, Topology, directory, joined};
use crate::survey;
use crate::syntax::structural::{DeclarationKind, Export, FileFacts, Visibility};

const LANGUAGE: &str = "TypeScript";
const MANIFEST: &str = "package.json";
const SOURCE: &[&str] = &[".ts", ".tsx", ".mts", ".cts"];

pub(super) fn derive(topology: &Topology, graph: &ModuleGraph, out: &mut Derived) {
    let modules: HashMap<&str, usize> = graph
        .modules
        .iter()
        .enumerate()
        .filter(|(_, module)| module.target.is_none() && module.nesting.is_empty())
        .map(|(at, module)| (module.file.as_str(), at))
        .collect();
    let mut derivation = Derivation {
        topology,
        graph,
        modules,
        items: HashMap::new(),
        holes: HashMap::new(),
        visiting: Vec::new(),
    };
    let manifests: Vec<&String> = topology
        .files()
        .iter()
        .filter(|file| basename(file) == MANIFEST && survey::surveyed(file))
        .collect();
    for manifest in manifests {
        derivation.package(manifest, out);
    }
}

/// One package's metadata as the derivation reads it.
struct Package {
    name: String,
    entries: Vec<Entry>,
}

/// One subpath of a package: where it leads, or why it leads nowhere klin supports.
struct Entry {
    subpath: String,
    field: &'static str,
    target: Result<String, String>,
}

struct Derivation<'a> {
    topology: &'a Topology<'a>,
    graph: &'a ModuleGraph,
    modules: HashMap<&'a str, usize>,
    /// The items each file exposes, derived once however many entries and stars reach it.
    items: HashMap<usize, Vec<Item>>,
    holes: HashMap<usize, Vec<Hole>>,
    visiting: Vec<usize>,
}

impl<'a> Derivation<'a> {
    fn package(&mut self, manifest: &str, out: &mut Derived) {
        let directory = directory(manifest);
        let Some(package) = self.read(manifest, directory) else {
            out.inapplicable.push(Inapplicable {
                what: format!("TypeScript package ({manifest})"),
                why: "is not a JSON object klin can read".to_string(),
            });
            return;
        };
        let supported: Vec<&Entry> = package
            .entries
            .iter()
            .filter(|entry| entry.target.is_ok())
            .collect();
        if supported.is_empty() {
            let why = match package.entries.first() {
                None => "names no TypeScript source entry point in exports, types, typings, main or module".to_string(),
                Some(entry) => format!(
                    "its {} names no TypeScript source klin supports: {}",
                    entry.field,
                    package
                        .entries
                        .iter()
                        .filter_map(|entry| entry.target.as_ref().err())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            };
            out.inapplicable.push(Inapplicable {
                what: format!("TypeScript package {} ({manifest})", package.name),
                why,
            });
            return;
        }
        let mut package_holes: Vec<Hole> = package
            .entries
            .iter()
            .filter_map(|entry| {
                Some(Hole {
                    file: manifest.to_string(),
                    line: 0,
                    text: format!("{} {:?}", entry.field, entry.subpath),
                    why: entry.target.as_ref().err()?.clone(),
                })
            })
            .collect();
        for entry in supported {
            let Ok(file) = &entry.target else {
                continue;
            };
            let mut surface = Surface {
                id: format!("{} {:?}", package.name, entry.subpath),
                language: LANGUAGE,
                source: format!("{} {} {:?} -> {file}", manifest, entry.field, entry.subpath),
                items: Vec::new(),
                files: Vec::new(),
                holes: std::mem::take(&mut package_holes),
            };
            match self.modules.get(file.as_str()).copied() {
                Some(module) => {
                    surface.items = self.items_of(module).to_vec();
                    let mut reached = Vec::new();
                    self.reached(module, &mut reached);
                    for at in reached {
                        surface.files.push(self.graph.modules[at].file.clone());
                        surface
                            .holes
                            .extend(self.holes.get(&at).into_iter().flatten().cloned());
                    }
                }
                None => surface.holes.push(Hole {
                    file: file.clone(),
                    line: 0,
                    text: entry.subpath.clone(),
                    why: "the entry point is not a module the tree measured".to_string(),
                }),
            }
            out.surfaces.push(surface);
        }
    }

    /// Every module a star or a named re-export of this one reaches, this one included.
    fn reached(&self, at: usize, out: &mut Vec<usize>) {
        if out.contains(&at) {
            return;
        }
        out.push(at);
        let Some(facts) = self.facts(at) else {
            return;
        };
        for export in &facts.exports {
            if export.source.is_some() {
                for to in self.graph.reached_at(at, export.line) {
                    self.reached(to, out);
                }
            }
        }
    }

    fn facts(&self, at: usize) -> Option<&'a FileFacts> {
        self.topology.facts(&self.graph.modules[at].file)
    }

    /// The package name and every entry its metadata names, and `None` for a manifest that is
    /// not a JSON object.
    fn read(&self, manifest: &str, directory: &str) -> Option<Package> {
        let bytes = self.topology.read(manifest)?;
        let parsed: Value = serde_json::from_slice(&bytes).ok()?;
        let fields = parsed.as_object()?;
        let name = fields
            .get("name")
            .and_then(Value::as_str)
            .map_or_else(|| basename(directory).to_string(), str::to_string);
        let mut entries = Vec::new();
        match fields.get("exports") {
            Some(exports) => self.exports(exports, directory, &mut entries),
            None => entries.extend(self.direct(fields, directory)),
        }
        Some(Package { name, entries })
    }

    /// The one root entry a package without `exports` names: the first of `types`, `typings`,
    /// `main` and `module` that names a TypeScript source file, or the first one written, with
    /// why it is not one.
    fn direct(&self, fields: &serde_json::Map<String, Value>, directory: &str) -> Option<Entry> {
        let written: Vec<(&'static str, &str)> = ["types", "typings", "main", "module"]
            .into_iter()
            .filter_map(|field| Some((field, fields.get(field)?.as_str()?)))
            .collect();
        let mut tried: Vec<Entry> = written
            .iter()
            .map(|(field, named)| Entry {
                subpath: ".".to_string(),
                field,
                target: self.source(directory, named),
            })
            .collect();
        let chosen = tried
            .iter()
            .position(|entry| entry.target.is_ok())
            .unwrap_or(0);
        (chosen < tried.len()).then(|| tried.swap_remove(chosen))
    }

    /// The entries of an `exports` field: one for a string, one per subpath of a map whose keys
    /// start with `.`, and one root entry for a bare condition map.
    fn exports(&self, exports: &Value, directory: &str, out: &mut Vec<Entry>) {
        match exports {
            Value::Object(map) if map.keys().all(|key| key.starts_with('.')) => {
                for (subpath, target) in map {
                    let resolved = match subpath.contains('*') {
                        true => Err("a subpath pattern, which klin does not expand".to_string()),
                        false => self.reduced(target, directory),
                    };
                    out.push(Entry {
                        subpath: subpath.clone(),
                        field: "exports",
                        target: resolved,
                    });
                }
            }
            other => out.push(Entry {
                subpath: ".".to_string(),
                field: "exports",
                target: self.reduced(other, directory),
            }),
        }
    }

    /// One exports target reduced to one TypeScript source file: a string as written, or the
    /// one source file every condition of a map or entry of an array leads to.
    fn reduced(&self, target: &Value, directory: &str) -> Result<String, String> {
        let mut leaves = Vec::new();
        strings(target, &mut leaves);
        if leaves.is_empty() {
            return Err("names no target".to_string());
        }
        let mut found: Vec<String> = Vec::new();
        let mut refused: Vec<String> = Vec::new();
        for leaf in &leaves {
            match self.source(directory, leaf) {
                Ok(file) if !found.contains(&file) => found.push(file),
                Ok(_) => {}
                Err(why) => refused.push(why),
            }
        }
        match found.as_slice() {
            [one] => Ok(one.clone()),
            [] => Err(refused.join("; ")),
            many => Err(format!(
                "leads to more than one TypeScript source file: {}",
                many.join(", ")
            )),
        }
    }

    /// The TypeScript source file a target names, and why not: a path outside the package, a
    /// file of another kind such as generated JavaScript, or a file the tree does not hold.
    fn source(&self, directory: &str, named: &str) -> Result<String, String> {
        let Some(file) = joined(directory, named) else {
            return Err(format!("{named} leaves the tree"));
        };
        if !SOURCE.iter().any(|end| file.ends_with(end)) {
            return Err(format!(
                "{named} is not a TypeScript source or declaration file, and klin does not map generated output back to source"
            ));
        }
        match self.topology.holds(&file) {
            true => Ok(file),
            false => Err(format!("{named} names no file the tree holds")),
        }
    }

    /// The items one module exposes, derived once: its exported declarations, its export
    /// clauses and defaults, and what it re-exports through the module graph's edges.
    fn items_of(&mut self, at: usize) -> &[Item] {
        if !self.items.contains_key(&at) {
            if self.visiting.contains(&at) {
                return &[];
            }
            self.visiting.push(at);
            let (items, holes) = self.derive_module(at);
            self.visiting.pop();
            self.items.insert(at, items);
            self.holes.insert(at, holes);
        }
        &self.items[&at]
    }

    fn derive_module(&mut self, at: usize) -> (Vec<Item>, Vec<Hole>) {
        let file = self.graph.modules[at].file.clone();
        let Some(facts) = self.facts(at) else {
            return (
                Vec::new(),
                vec![Hole {
                    file,
                    line: 0,
                    text: String::new(),
                    why: "the module was not measured, so what it exports is unknown".to_string(),
                }],
            );
        };
        let mut items = Vec::new();
        let mut holes = Vec::new();
        for declaration in &facts.declarations {
            if declaration.visibility == Visibility::Public
                && declaration.kind != DeclarationKind::Method
            {
                let name = declaration
                    .exported_as
                    .clone()
                    .unwrap_or_else(|| declaration.name.clone());
                items.push(declared(name, &file, declaration));
            }
        }
        let own: HashSet<String> = items.iter().map(|item| item.path.clone()).collect();
        let mut stars: BTreeMap<String, (u64, Item)> = BTreeMap::new();
        let mut ambiguous: HashSet<String> = HashSet::new();
        for export in &facts.exports {
            if !export.supported {
                holes.push(Hole {
                    file: file.clone(),
                    line: export.line,
                    text: export.text.clone(),
                    why: "an export form klin does not list: `export =`, a namespace, or an ambient module".to_string(),
                });
                continue;
            }
            match &export.source {
                None => self.local(facts, &file, export, &mut items),
                Some(specifier) => {
                    self.re_export(
                        at,
                        facts,
                        export,
                        specifier,
                        &own,
                        &mut items,
                        &mut stars,
                        &mut ambiguous,
                        &mut holes,
                    );
                }
            }
        }
        for (name, (_, item)) in stars {
            if !ambiguous.contains(&name) {
                items.push(item);
            }
        }
        (items, holes)
    }

    /// A clause with no source: `export { a as b }` over a local declaration, and
    /// `export default x`.
    fn local(&self, facts: &FileFacts, file: &str, export: &Export, items: &mut Vec<Item>) {
        for leaf in &export.leaves {
            let Some(name) = &leaf.name else {
                continue;
            };
            let kind = type_only(export);
            let declarations: Vec<_> = facts
                .declarations
                .iter()
                .filter(|held| held.name == leaf.path && held.kind != DeclarationKind::Method)
                .collect();
            if leaf.path.is_empty() || declarations.is_empty() {
                items.push(opaque(
                    name.clone(),
                    kind.unwrap_or(ITEM),
                    file,
                    export.line,
                    clause(&leaf.path, name, None),
                ));
                continue;
            }
            for declaration in declarations {
                let mut item = declared(name.clone(), file, declaration);
                if let Some(kind) = kind {
                    item.kind = kind;
                }
                items.push(item);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn re_export(
        &mut self,
        at: usize,
        facts: &FileFacts,
        export: &Export,
        specifier: &str,
        own: &HashSet<String>,
        items: &mut Vec<Item>,
        stars: &mut BTreeMap<String, (u64, Item)>,
        ambiguous: &mut HashSet<String>,
        holes: &mut Vec<Hole>,
    ) {
        let file = facts.file.clone();
        let targets = self.graph.reached_at(at, export.line);
        let Some(target) = targets.first().copied() else {
            match relative(specifier) {
                true => holes.push(Hole {
                    file,
                    line: export.line,
                    text: export.text.clone(),
                    why: format!("{specifier} resolves to no TypeScript module the tree holds, or to more than one"),
                }),
                false => self.external(&file, export, specifier, items, holes),
            }
            return;
        };
        let reached: Vec<Item> = self.items_of(target).to_vec();
        for leaf in &export.leaves {
            let kind = type_only(export);
            match (&leaf.name, leaf.path.as_str()) {
                (None, _) => {
                    for item in reached.iter().filter(|item| item.path != "default") {
                        if own.contains(&item.path) {
                            continue;
                        }
                        match stars.get(&item.path) {
                            Some((line, _)) if *line != export.line => {
                                ambiguous.insert(item.path.clone());
                                holes.push(Hole {
                                    file: file.clone(),
                                    line: export.line,
                                    text: export.text.clone(),
                                    why: format!(
                                        "{} is provided by this star export and by the one at line {line}",
                                        item.path
                                    ),
                                });
                            }
                            _ => {
                                stars.insert(item.path.clone(), (export.line, item.clone()));
                            }
                        }
                    }
                }
                (Some(name), "*") => items.push(opaque(
                    name.clone(),
                    NAMESPACE,
                    &file,
                    export.line,
                    format!("* as {name} from {specifier}"),
                )),
                (Some(name), path) => {
                    let found: Vec<&Item> =
                        reached.iter().filter(|item| item.path == path).collect();
                    if found.is_empty() {
                        items.push(opaque(
                            name.clone(),
                            kind.unwrap_or(ITEM),
                            &file,
                            export.line,
                            clause(path, name, Some(specifier)),
                        ));
                        continue;
                    }
                    for item in found {
                        let mut item = item.clone();
                        item.path = name.clone();
                        if let Some(kind) = kind {
                            item.kind = kind;
                        }
                        items.push(item);
                    }
                }
            }
        }
    }

    /// A re-export from another package: each named clause is an opaque item whose clause is
    /// its contract, and a star over the package is a hole, because its names are unknown.
    fn external(
        &self,
        file: &str,
        export: &Export,
        specifier: &str,
        items: &mut Vec<Item>,
        holes: &mut Vec<Hole>,
    ) {
        for leaf in &export.leaves {
            match (&leaf.name, leaf.path.as_str()) {
                (None, _) => holes.push(Hole {
                    file: file.to_string(),
                    line: export.line,
                    text: export.text.clone(),
                    why: format!("a star export of another package, {specifier}, whose names klin does not inspect"),
                }),
                (Some(name), "*") => items.push(opaque(
                    name.clone(),
                    NAMESPACE,
                    file,
                    export.line,
                    format!("* as {name} from {specifier}"),
                )),
                (Some(name), path) => items.push(opaque(
                    name.clone(),
                    type_only(export).unwrap_or(ITEM),
                    file,
                    export.line,
                    clause(path, name, Some(specifier)),
                )),
            }
        }
    }
}

/// The kind a type-only export gives every name it exposes, and `None` for a value export.
fn type_only(export: &Export) -> Option<&'static str> {
    export.type_only.then_some(TYPE)
}

/// The normalized clause that exposes one name, which is the contract klin can compare for an
/// item it cannot classify.
fn clause(path: &str, name: &str, specifier: Option<&str>) -> String {
    let mut out = match path == name {
        true => name.to_string(),
        false => format!("{path} as {name}"),
    };
    if let Some(specifier) = specifier {
        out.push_str(&format!(" from {specifier}"));
    }
    out
}

/// Every string leaf of an exports target: the string itself, each value of a condition map,
/// and each entry of an array, in order.
fn strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.clone()),
        Value::Object(map) => map.values().for_each(|held| strings(held, out)),
        Value::Array(items) => items.iter().for_each(|held| strings(held, out)),
        _ => {}
    }
}

fn relative(specifier: &str) -> bool {
    matches!(specifier, "." | "..") || specifier.starts_with("./") || specifier.starts_with("../")
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}
