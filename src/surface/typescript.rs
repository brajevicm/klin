//! TypeScript surfaces. A surface begins only at explicit package metadata: an `exports` target,
//! a `types` or `typings` entry, or a `main` or `module` that itself names a TypeScript source
//! file the tree holds. Generated JavaScript is never reverse-mapped and no `src/index.ts` is
//! guessed. From an entry file the derivation follows exported declarations, local export
//! clauses, default exports and local re-exports through the module graph's own edges. A
//! re-export of another package and an exported namespace are opaque items whose clause is their
//! contract, and a star that klin cannot list is a hole.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::Value;

use super::item::{
    Found, Hole, ITEM, Inapplicable, Item, NAMESPACE, Surface, TYPE, declared, opaque,
};
use crate::modules::ModuleGraph;
use crate::modules::resolver::{Topology, directory, joined};
use crate::survey;
use crate::syntax::structural::facts::{DeclarationKind, Export, FileFacts, Visibility};

pub(super) const LANGUAGE: &str = "TypeScript";
const MANIFEST: &str = "package.json";
const SOURCE: &[&str] = &[".ts", ".tsx", ".mts", ".cts"];

pub(super) fn derive(topology: &Topology, graph: &ModuleGraph, out: &mut Found) {
    let modules: HashMap<&str, usize> = graph
        .modules
        .iter()
        .enumerate()
        .filter(|(_, module)| module.target.is_none() && module.nesting.is_empty())
        .flat_map(|(at, module)| module.sources.iter().map(move |file| (file.as_str(), at)))
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
    fn package(&mut self, manifest: &str, out: &mut Found) {
        let Some(package) = self.read(manifest, directory(manifest)) else {
            out.inapplicable.push(Inapplicable {
                what: format!("TypeScript package ({manifest})"),
                why: "is not a JSON object klin can read".to_string(),
            });
            return;
        };
        if package.entries.iter().all(|entry| entry.target.is_err()) {
            out.inapplicable.push(Inapplicable {
                what: format!("TypeScript package {} ({manifest})", package.name),
                why: not_applicable(&package),
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
        for entry in &package.entries {
            if let Ok(file) = &entry.target {
                let holes = std::mem::take(&mut package_holes);
                out.surfaces
                    .push(self.surface(manifest, &package.name, entry, file, holes));
            }
        }
    }

    /// One entry as a surface: the items of its entry module, the files that module reaches,
    /// and the holes found on the way.
    fn surface(
        &mut self,
        manifest: &str,
        package: &str,
        entry: &Entry,
        file: &str,
        holes: Vec<Hole>,
    ) -> Surface {
        let mut surface = Surface {
            id: format!("{package} {:?}", entry.subpath),
            manifest: None,
            language: LANGUAGE,
            source: format!("{manifest} {} {:?} -> {file}", entry.field, entry.subpath),
            items: Vec::new(),
            files: Vec::new(),
            holes,
        };
        let Some(module) = self.modules.get(file).copied() else {
            surface.holes.push(Hole {
                file: file.to_string(),
                line: 0,
                text: entry.subpath.clone(),
                why: "the entry point is not a module the tree measured".to_string(),
            });
            return surface;
        };
        surface.items = self.items_of(module).to_vec();
        let mut reached = Vec::new();
        self.reached(module, &mut reached);
        for at in reached {
            surface.files.push(self.file(at).to_string());
            surface
                .holes
                .extend(self.holes.get(&at).into_iter().flatten().cloned());
        }
        surface
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
                for to in self.graph.reached_at(at, self.file(at), export.line) {
                    self.reached(to, out);
                }
            }
        }
    }

    fn facts(&self, at: usize) -> Option<&'a FileFacts> {
        self.topology.facts(self.file(at))
    }

    /// The file a TypeScript module is: its one source.
    fn file(&self, at: usize) -> &'a str {
        &self.graph.modules[at].sources[0]
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
        let (mut found, mut refused): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
        for leaf in &leaves {
            match self.source(directory, leaf) {
                Ok(file) if !found.contains(&file) => found.push(file),
                Ok(_) => {}
                Err(why) => refused.push(why),
            }
        }
        one_of(found, refused)
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
        let file = self.file(at).to_string();
        let Some(facts) = self.facts(at) else {
            let hole = Hole {
                file,
                line: 0,
                text: String::new(),
                why: "the module was not measured, so what it exports is unknown".to_string(),
            };
            return (Vec::new(), vec![hole]);
        };
        let mut exposing = Exposing {
            file,
            items: own_declarations(facts),
            own: HashSet::new(),
            stars: BTreeMap::new(),
            ambiguous: HashSet::new(),
            holes: Vec::new(),
        };
        exposing.own = exposing
            .items
            .iter()
            .map(|item| item.path.clone())
            .collect();
        for export in facts.exports.iter().filter(|held| held.contract.is_none()) {
            match (&export.supported, &export.source) {
                (false, _) => exposing.unsupported(export),
                (true, None) => {
                    self.local(facts, &exposing.file.clone(), export, &mut exposing.items)
                }
                (true, Some(specifier)) => self.re_export(at, export, specifier, &mut exposing),
            }
        }
        exposing.finish()
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

    /// A clause with a source: through the module graph's edge for its line, or as an external
    /// package where the graph has no local-resolution hole at that site.
    fn re_export(&mut self, at: usize, export: &Export, specifier: &str, into: &mut Exposing) {
        let Some(target) = self
            .graph
            .reached_at(at, self.file(at), export.line)
            .first()
            .copied()
        else {
            match self.unresolved(at, export, specifier) {
                true => into.holes.push(Hole {
                    file: into.file.clone(),
                    line: export.line,
                    text: export.text.clone(),
                    why: format!(
                        "{specifier} resolves to no TypeScript module the tree holds, or to more than one"
                    ),
                }),
                false => {
                    let file = into.file.clone();
                    self.external(&file, export, specifier, &mut into.items, &mut into.holes);
                }
            }
            return;
        };
        let reached: Vec<Item> = self.items_of(target).to_vec();
        for leaf in &export.leaves {
            match (&leaf.name, leaf.path.as_str()) {
                (None, _) => into.star(export, &reached),
                (Some(name), "*") => into.items.push(opaque(
                    name.clone(),
                    NAMESPACE,
                    &into.file,
                    export.line,
                    format!("* as {name} from {specifier}"),
                )),
                (Some(name), path) => into.named(export, specifier, &reached, path, name),
            }
        }
    }

    /// Only a graph hole at this re-export's own site affects its contract.
    fn unresolved(&self, at: usize, export: &Export, specifier: &str) -> bool {
        relative(specifier)
            || self.graph.holes.iter().any(|hole| {
                hole.file == self.file(at) && hole.line == export.line && hole.text == export.text
            })
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

/// One module's items as they are gathered: the names it declares itself, what its stars
/// provide and which of those two stars fight over, and the holes on the way.
struct Exposing {
    file: String,
    items: Vec<Item>,
    own: HashSet<String>,
    stars: BTreeMap<String, (u64, Item)>,
    ambiguous: HashSet<String>,
    holes: Vec<Hole>,
}

impl Exposing {
    fn unsupported(&mut self, export: &Export) {
        self.holes.push(Hole {
            file: self.file.clone(),
            line: export.line,
            text: export.text.clone(),
            why: "an export form klin does not list, such as `export =` or an ambient module"
                .to_string(),
        });
    }

    /// Every item a star export provides, less the names this module declares itself. A name
    /// two stars provide is ambiguous, and a hole.
    fn star(&mut self, export: &Export, reached: &[Item]) {
        for item in reached.iter().filter(|item| item.path != "default") {
            if self.own.contains(&item.path) {
                continue;
            }
            match self.stars.get(&item.path) {
                Some((line, _)) if *line != export.line => {
                    self.ambiguous.insert(item.path.clone());
                    self.holes.push(Hole {
                        file: self.file.clone(),
                        line: export.line,
                        text: export.text.clone(),
                        why: format!(
                            "{} is provided by this star export and by the one at line {line}",
                            item.path
                        ),
                    });
                }
                _ => {
                    self.stars
                        .insert(item.path.clone(), (export.line, item.clone()));
                }
            }
        }
    }

    /// One named clause of a re-export: the reached module's items of that name under the
    /// external name, or an opaque item where the module has none.
    fn named(
        &mut self,
        export: &Export,
        specifier: &str,
        reached: &[Item],
        path: &str,
        name: &str,
    ) {
        let kind = type_only(export);
        let found: Vec<&Item> = reached.iter().filter(|item| item.path == path).collect();
        if found.is_empty() {
            self.items.push(opaque(
                name.to_string(),
                kind.unwrap_or(ITEM),
                &self.file,
                export.line,
                clause(path, name, Some(specifier)),
            ));
            return;
        }
        for item in found {
            let mut item = item.clone();
            item.path = name.to_string();
            if let Some(kind) = kind {
                item.kind = kind;
            }
            self.items.push(item);
        }
    }

    fn finish(mut self) -> (Vec<Item>, Vec<Hole>) {
        for (name, (_, item)) in self.stars {
            if !self.ambiguous.contains(&name) {
                self.items.push(item);
            }
        }
        (self.items, self.holes)
    }
}

/// The items a module declares and exports itself, under their external names, and each
/// namespace an `export` statement declares, whose contract is its clause.
fn own_declarations(facts: &FileFacts) -> Vec<Item> {
    let namespaces = facts.exports.iter().filter_map(|export| {
        let contract = export.contract.clone()?;
        let leaf = export.leaves.first()?;
        Some(opaque(
            leaf.path.clone(),
            NAMESPACE,
            &facts.file,
            export.line,
            contract,
        ))
    });
    facts
        .declarations
        .iter()
        .filter(|held| held.visibility == Visibility::Public)
        .filter(|held| held.kind != DeclarationKind::Method)
        .map(|held| {
            let name = held
                .exported_as
                .clone()
                .unwrap_or_else(|| held.name.clone());
            declared(name, &facts.file, held)
        })
        .chain(namespaces)
        .collect()
}

/// Why a package has no supported surface.
fn not_applicable(package: &Package) -> String {
    match package.entries.first() {
        None => "names no TypeScript source entry point in exports, types, typings, main or module"
            .to_string(),
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
    }
}

/// The one source file a target's leaves lead to, or why there is not exactly one.
fn one_of(found: Vec<String>, refused: Vec<String>) -> Result<String, String> {
    match found.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(refused.join("; ")),
        many => Err(format!(
            "leads to more than one TypeScript source file: {}",
            many.join(", ")
        )),
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
