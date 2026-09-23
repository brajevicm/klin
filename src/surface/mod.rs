//! The public surfaces of one tree: what a consumer of a Rust library crate or a TypeScript
//! package can address, derived from the structural facts and the module graph a run already
//! holds. A surface is a Cargo library target or one subpath of a package's entry points; an
//! item is what a consumer names under it, by the path or name it is exposed under and never by
//! the file that declares it. An item is measured where its declared contract is canonical, and
//! opaque where klin can prove it exists and nothing more. A form klin recognizes and cannot
//! list is a hole, and a package or target with no supported surface is named as such. This
//! layer sits above `syntax::structural` and `modules` and parses nothing. Spec 8.2.1, ADR 0044.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::modules::{ModuleGraph, Topology};
use crate::syntax::structural::{Declaration, DeclarationKind, LanguageId};

mod rust;
mod typescript;

type Derive = fn(&Topology, &ModuleGraph, &mut Derived);

/// One surface derivation per structural language, run only over a tree that lists a path of
/// that language, as its resolver is.
const DERIVATIONS: &[(LanguageId, Derive)] = &[
    (LanguageId::Rust, rust::derive),
    (LanguageId::TypeScript, typescript::derive),
];

/// The words a report prints for what an item is.
pub const FUNCTION: &str = "function";
pub const METHOD: &str = "method";
pub const TYPE: &str = "type";
pub const CONSTANT: &str = "constant";
pub const VARIABLE: &str = "variable";
pub const MODULE: &str = "module";
pub const NAMESPACE: &str = "namespace";
/// An item klin proves exposed and cannot classify: a re-export of another crate or package,
/// an enum variant re-exported by path, or an anonymous default export.
pub const ITEM: &str = "item";

/// How much of an item's contract klin proves.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Contract {
    /// A stable identity and a supported canonical declared signature.
    Measured(String),
    /// The item exists. Where the clause that exposes it is itself the contract klin can prove,
    /// that clause is here to be compared; otherwise only presence is judged.
    Opaque(Option<String>),
}

/// One thing a consumer addresses under a surface.
#[derive(Clone, Debug)]
pub struct Item {
    /// The path or name the consumer writes, without the surface's own name.
    pub path: String,
    pub kind: &'static str,
    /// Where the declaration behind the item sits, as explanation only.
    pub origin: Option<(String, u64)>,
    pub contract: Contract,
}

/// One consumer-facing surface: a library crate, or one export subpath of a package.
pub struct Surface {
    /// What a consumer names the surface by: the crate name, or the package name and subpath.
    pub id: String,
    pub language: &'static str,
    /// Where the surface was discovered, for a report.
    pub source: String,
    pub items: Vec<Item>,
    /// The files whose facts the surface is derived from, so a gate can say which unparsed
    /// files sit inside it.
    pub files: Vec<String>,
    pub holes: Vec<Hole>,
}

/// One form klin recognizes inside a supported surface and cannot resolve, so the surface is not
/// completely measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hole {
    pub file: String,
    pub line: u64,
    pub text: String,
    pub why: String,
}

/// A package or target klin found and derived no surface from, and why.
pub struct Inapplicable {
    pub what: String,
    pub why: String,
}

/// Everything the tree's surfaces came to, and what deriving them cost.
#[derive(Default)]
pub struct Derived {
    pub surfaces: Vec<Surface>,
    pub inapplicable: Vec<Inapplicable>,
    pub dispatches: [usize; DERIVATIONS.len()],
    pub time: Duration,
}

/// What deriving a gate's surfaces over its two trees cost. Spec 11.2.
#[derive(Default, Clone, Copy)]
pub struct SurfaceCost {
    pub surfaces: usize,
    pub items: usize,
    pub measured: usize,
    pub opaque: usize,
    pub holes: usize,
    pub dispatches: [usize; DERIVATIONS.len()],
    pub time: Duration,
}

impl SurfaceCost {
    /// Each registered derivation's language with how many times it ran.
    pub fn dispatched(&self) -> impl Iterator<Item = (LanguageId, usize)> + '_ {
        DERIVATIONS
            .iter()
            .zip(self.dispatches)
            .map(|((language, _), count)| (*language, count))
    }
}

impl std::ops::Add for SurfaceCost {
    type Output = SurfaceCost;

    fn add(self, other: SurfaceCost) -> SurfaceCost {
        let mut dispatches = self.dispatches;
        for (held, more) in dispatches.iter_mut().zip(other.dispatches) {
            *held += more;
        }
        SurfaceCost {
            dispatches,
            surfaces: self.surfaces + other.surfaces,
            items: self.items + other.items,
            measured: self.measured + other.measured,
            opaque: self.opaque + other.opaque,
            holes: self.holes + other.holes,
            time: self.time + other.time,
        }
    }
}

/// The surfaces of one tree, each derivation whose language the tree holds run over it once.
pub fn derive(topology: &Topology, graph: &ModuleGraph) -> Derived {
    let started = Instant::now();
    let mut out = Derived::default();
    for (at, (language, derive)) in DERIVATIONS.iter().enumerate() {
        if topology.present(*language) {
            out.dispatches[at] += 1;
            derive(topology, graph, &mut out);
        }
    }
    for surface in &mut out.surfaces {
        let ordered = surface.language == typescript::LANGUAGE;
        surface.items = merged(std::mem::take(&mut surface.items), ordered);
        surface.holes.sort_by(|a, b| {
            (&a.file, a.line, &a.text, &a.why).cmp(&(&b.file, b.line, &b.text, &b.why))
        });
        surface.holes.dedup();
        surface.files.sort_unstable();
        surface.files.dedup();
    }
    out.surfaces.sort_by(|a, b| a.id.cmp(&b.id));
    out.inapplicable.sort_by(|a, b| a.what.cmp(&b.what));
    out.time = started.elapsed();
    out
}

impl Derived {
    pub fn cost(&self) -> SurfaceCost {
        let items = self.surfaces.iter().flat_map(|surface| &surface.items);
        SurfaceCost {
            surfaces: self.surfaces.len(),
            items: items.clone().count(),
            measured: items
                .clone()
                .filter(|item| matches!(item.contract, Contract::Measured(_)))
                .count(),
            opaque: items
                .filter(|item| matches!(item.contract, Contract::Opaque(_)))
                .count(),
            holes: self
                .surfaces
                .iter()
                .map(|surface| surface.holes.len())
                .sum(),
            dispatches: self.dispatches,
            time: self.time,
        }
    }
}

impl Surface {
    pub fn item(&self, path: &str, kind: &str) -> Option<&Item> {
        self.items
            .iter()
            .find(|item| item.path == path && item.kind == kind)
    }
}

/// Items of one surface under one identity, folded into one: a Rust item declared twice under
/// `cfg`, or a TypeScript overload set, is one item whose contract lists each declared
/// signature once. Where `ordered`, as for a TypeScript overload set, the signatures of one file
/// keep their source order, because overload resolution follows it, and the groups of different
/// files are ordered by their text, so a renamed file never changes a contract. Otherwise, as
/// for Rust's `cfg` twins, every signature is ordered by its text. An empty signature, an
/// implementation that follows its overloads, adds nothing. An identity that any declaration
/// leaves opaque is opaque.
fn merged(items: Vec<Item>, ordered: bool) -> Vec<Item> {
    let mut by_identity: BTreeMap<(String, &'static str), Vec<Item>> = BTreeMap::new();
    for item in items {
        by_identity
            .entry((item.path.clone(), item.kind))
            .or_default()
            .push(item);
    }
    by_identity
        .into_values()
        .map(|same| folded(same, ordered))
        .collect()
}

/// One identity's items as one item: opaque where any of them is, with every clause once, and
/// measured by its signatures otherwise. The first declaration names the origin.
fn folded(mut same: Vec<Item>, ordered: bool) -> Item {
    same.sort_by_key(|item| item.origin.clone());
    let opaque = same
        .iter()
        .any(|item| matches!(item.contract, Contract::Opaque(_)));
    let mut clauses: Vec<String> = same
        .iter()
        .filter_map(|item| match &item.contract {
            Contract::Opaque(clause) => clause.clone(),
            Contract::Measured(_) => None,
        })
        .collect();
    clauses.sort();
    clauses.dedup();
    let contract = match opaque {
        true => Contract::Opaque((!clauses.is_empty()).then(|| clauses.join("; "))),
        false => Contract::Measured(signatures(&same, ordered)),
    };
    Item {
        contract,
        ..same.swap_remove(0)
    }
}

/// The measured signatures of one identity's items, which come sorted by origin: where
/// `ordered`, each file's in source order and once each, and the groups of different files
/// ordered by their text; otherwise each signature on its own, ordered by its text.
fn signatures(same: &[Item], ordered: bool) -> String {
    let mut files: Vec<(Option<&String>, Vec<&str>)> = Vec::new();
    for item in same {
        let Contract::Measured(signature) = &item.contract else {
            continue;
        };
        if signature.is_empty() {
            continue;
        }
        let file = item.origin.as_ref().map(|(file, _)| file);
        match files
            .last_mut()
            .filter(|(held, _)| ordered && *held == file)
        {
            Some((_, held)) if held.contains(&signature.as_str()) => {}
            Some((_, held)) => held.push(signature),
            None => files.push((file, vec![signature])),
        }
    }
    let mut groups: Vec<String> = files
        .into_iter()
        .map(|(_, signatures)| signatures.join("; "))
        .collect();
    groups.sort();
    groups.dedup();
    groups.join("; ")
}

/// The word for a declaration's kind.
pub fn kind_word(kind: DeclarationKind) -> &'static str {
    match kind {
        DeclarationKind::Function => FUNCTION,
        DeclarationKind::Method => METHOD,
        DeclarationKind::Type => TYPE,
        DeclarationKind::Constant => CONSTANT,
        DeclarationKind::Variable => VARIABLE,
    }
}

/// One item from one declaration, exposed under `path`.
fn declared(path: String, file: &str, declaration: &Declaration) -> Item {
    Item {
        path,
        kind: kind_word(declaration.kind),
        origin: Some((file.to_string(), declaration.line)),
        contract: match &declaration.signature {
            Some(signature) => Contract::Measured(signature.clone()),
            None => Contract::Opaque(None),
        },
    }
}

/// One item klin proves exposed by a clause and cannot classify further.
fn opaque(path: String, kind: &'static str, file: &str, line: u64, clause: String) -> Item {
    Item {
        path,
        kind,
        origin: Some((file.to_string(), line)),
        contract: Contract::Opaque(Some(clause)),
    }
}
