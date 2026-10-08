//! The `layering` check: a dependency that crosses a declared architectural boundary, or that
//! closes a dependency cycle, instead of going through the interface the architecture intends.
//! It reads the module graph of both trees and judges resolved dependencies only; containment
//! is never a dependency. Judged sites group into semantic edges: the module that writes one
//! and the module it reaches, each by its semantic identity, and the edge's text, which names
//! its kind, its layers and the module it reaches by its file and inline modules. The working
//! tree's semantic edges pair with the base's first; each is then reported as one finding per
//! file that writes it and text, carrying `edge` at 1, held where the base holds every semantic
//! edge it merges and new otherwise. A renamed file is placed in the base's layers under its
//! base path and keyed under its current one. The section is a person's policy and nothing
//! derives it: with no section the gate does not run. Spec 8.2.1, ADR 0043, ADR 0058.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Instant;

use serde_json::{Map, Value};

use crate::check::contract::{
    self, Context, HeldAtBase, Line, Listed, Measured, Sink, Site, Unresolvable,
};
use crate::check::holes;
use crate::config::{self, Config};
use crate::coverage;
use crate::error::Error;
use crate::key::Key;
use crate::measurement;
use crate::modules::resolver::{Attachment, Dependency, Hole};
use crate::modules::{self, Cycles, GraphCost, ModuleGraph};
use crate::ratchet::{self, Evaluator, Finding, Remedy};
use crate::record::Values;
use crate::scope::{self, Scope, Selector};
use crate::syntax::{self, structural};
use crate::tree::Tree;

pub const SECTION: &str = "layering";

const ACYCLIC: Key = Key {
    name: "acyclic",
    holds: "`true` to fail a dependency that closes a module cycle the base did not hold",
    required: false,
    rule: None,
    default: "`false`",
    shape: crate::key::Shape::Boolean,
};

const LAYERS: Key = Key {
    name: "layers",
    holds: "a map of layer name to a layer: `in`, a repository-relative path or list of them the layer holds, and `can_use`, the layers it may depend on, or `null` for every layer. A layer may always depend on itself, and a file belongs to at most one layer",
    required: true,
    rule: None,
    default: "",
    shape: crate::key::Shape::Layers,
};

pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT, ACYCLIC, LAYERS];

const CAN_USE: &str = "can_use";
const EDGE: &str = "edge";
const KIND: &str = "kind";
const SITES: &str = "sites";
const PATH: &str = "path";
const FORBIDDEN: &str = "forbidden";
const CYCLE: &str = "cycle";
const REMEDY: &str = "Depend on a layer this layer's `can_use` names, through that layer's interface, or \
                      move the code to the layer it belongs to. Break a new cycle by moving what \
                      both modules need into a module neither depends back on.";

struct Layer {
    name: String,
    within: Vec<Selector>,
    can_use: Option<Vec<String>>,
}

struct Policy {
    scope: Scope,
    acyclic: bool,
    layers: Vec<Layer>,
}

/// One tree as the gate judges it: its structural files and unparsed files under their topology
/// paths, its module graph, and the current path of every file the topology names otherwise.
struct Side {
    files: Vec<String>,
    unparsed: Vec<syntax::Unparsed>,
    graph: ModuleGraph,
    current: HashMap<String, String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Forbidden,
    Cycle,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Forbidden => FORBIDDEN,
            Kind::Cycle => CYCLE,
        }
    }
}

/// One judged semantic edge: its kind, every physical site that writes it as lines by file
/// under current paths, and the first dependency that does.
struct Edge {
    kind: Kind,
    sites: BTreeMap<String, BTreeSet<u64>>,
    first: usize,
}

/// The semantic edges of one side, keyed by the semantic identity of the module that writes
/// each, that of the module it reaches, and its text, which names its kind, its layers and the
/// module it reaches. ADR 0058.
type Edges = BTreeMap<(String, String, String), Edge>;

/// One finding as the working tree reports it: the semantic edges one file writes under one
/// text, merged, and whether the base held every one of them.
struct Physical {
    kind: Kind,
    lines: BTreeSet<u64>,
    first: usize,
    held: bool,
}

/// The working tree's findings, keyed by the file and the text each reports.
type Physicals = BTreeMap<(String, String), Physical>;

/// Where path policy puts one physical file: whether the section's scope selects it, and the
/// layer that holds it.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Place {
    selected: bool,
    layer: Option<usize>,
}

/// One policy fact folded over a module's files: the same for every file, or not. ADR 0058.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Folded<T> {
    All(T),
    Mixed,
}

impl<T: Copy + PartialEq> Folded<T> {
    fn and(self, other: Folded<T>) -> Folded<T> {
        if self == other { self } else { Folded::Mixed }
    }
}

/// Path policy over one side's graph: each physical source placed once, and each module's
/// scope and layer folded once over its sources, so a dependency is judged by lookup.
struct Placed<'g> {
    files: HashMap<&'g str, Place>,
    modules: Vec<(Folded<bool>, Folded<Option<usize>>)>,
}

impl Placed<'_> {
    /// Whether the section judges a dependency: the file that writes it is selected, and every
    /// file of the module it reaches is.
    fn judges(&self, graph: &ModuleGraph, dependency: &Dependency) -> bool {
        self.files[graph.source(dependency)].selected
            && self.modules[dependency.to].0 == Folded::All(true)
    }
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let policy = policy(at.config())?;
    let commit = contract::base_commit(at.project.root(), at)?;
    let (was, now, base_scoped) = sides(at, &commit, out)?;
    policy.applies(at.config(), &was, &now)?;
    let started = Instant::now();
    let was_placed = policy.placed(&was.graph);
    let now_placed = policy.placed(&now.graph);
    let was_cycles = policy.cycles(&was, &was_placed);
    let now_cycles = policy.cycles(&now, &now_placed);
    let (was_edges, was_ambiguous) = edges(&policy, (&was, &was_placed), was_cycles.as_ref());
    let (now_edges, ambiguous) = edges(&policy, (&now, &now_placed), now_cycles.as_ref());
    let time = started.elapsed();
    out.record(|records| {
        records.graph = Some(
            was.graph.cost()
                + now.graph.cost()
                + GraphCost {
                    time,
                    ..GraphCost::default()
                },
        );
    });
    let was_files = was.covered(&policy);
    let now_files = now.covered(&policy);
    let left = now_files.lost(&was_files, at.project, None);
    let (left_edges, left_physicals, left_findings) =
        under_the_base(&policy, base_scoped, &left, &was_edges)?;
    let mut physicals = physicals(&now_edges, &was_edges);
    let mut findings = findings(&physicals, &now.graph, now_cycles.as_ref());
    findings.extend(left_findings);
    physicals.extend(left_physicals);
    let mut now_edges = now_edges;
    now_edges.extend(left_edges);
    let code = judged(
        at,
        (&policy, &now, &now_placed),
        findings,
        (&physicals, &was_edges, &now_edges),
        out,
    )?;
    holes::lost_said(&left, out);
    holes::formed_said(&now_files, at, out);
    holes_said((&was, &now), &policy, (&was_ambiguous, &ambiguous), at, out);
    let unparsed: Vec<syntax::Unparsed> = now
        .unparsed
        .iter()
        .filter(|file| policy.scope.selects(&file.file))
        .cloned()
        .collect();
    held_note(&physicals, out);
    holes::unread_said(&unparsed, at, out);
    Ok(code)
}

/// The edges, findings and physical sites that the files which left the section's scope write
/// under the base's scope: the base's manifests over the working tree's facts. A manifest that
/// narrows the scope then hides no new edge, and a file that left clean is only the review
/// item. Spec 7.2.
fn under_the_base(
    policy: &Policy,
    base_scoped: impl FnOnce() -> Result<Side, Error>,
    left: &[coverage::Lost],
    was_edges: &Edges,
) -> Result<(Edges, Physicals, Vec<Finding>), Error> {
    if left.is_empty() {
        return Ok(Default::default());
    }
    let base_scoped = &base_scoped()?;
    let files: BTreeSet<&str> = left.iter().map(|file| file.file.as_str()).collect();
    let placed = policy.placed(&base_scoped.graph);
    let cycles = policy.cycles(base_scoped, &placed);
    let (mut edges, _) = edges(policy, (base_scoped, &placed), cycles.as_ref());
    for edge in edges.values_mut() {
        edge.sites.retain(|file, _| files.contains(file.as_str()));
    }
    edges.retain(|_, edge| !edge.sites.is_empty());
    let physicals = physicals(&edges, was_edges);
    let findings = findings(&physicals, &base_scoped.graph, cycles.as_ref());
    Ok((edges, physicals, findings))
}

/// The base and the working tree, each measured and resolved, and the working tree resolved
/// under the base's manifests on demand. A changed run that is not strict takes the base's facts
/// for every file it did not change, as `dead-symbols` does.
fn sides<'a>(
    at: &Context<'a>,
    commit: &str,
    out: &mut Sink,
) -> Result<(Side, Side, impl FnOnce() -> Result<Side, Error> + 'a), Error> {
    let project = at.project;
    let prior = contract::whole_base(at, commit)?;
    let unchanged = contract::unchanged_base(at, prior, commit)?;
    let mut after = measurement::measure_all(project.tree(), unchanged.as_ref())?;
    let before = measurement::measure_all(prior.tree(), None)?;
    after.cost = after.cost
        + unchanged.map_or_else(
            structural::ExtractionCost::default,
            measurement::Unchanged::publish,
        );
    out.record(|records| records.facts = Some(before.cost + after.cost));
    let was = side(prior.tree(), &before, prior.renamed())?;
    let now = side(project.tree(), &after, &HashMap::new())?;
    let base_scoped = move || {
        let files = project.tree().files()?;
        side_in((prior.root(), files), &after, &HashMap::new())
    };
    Ok((was, now, base_scoped))
}

fn side(
    tree: &Tree,
    measured: &measurement::Measurement,
    renamed: &HashMap<String, String>,
) -> Result<Side, Error> {
    side_in((tree.root(), tree.files()?), measured, renamed)
}

/// One side resolved from the manifests under `root` over these files and facts.
fn side_in(
    (root, listed): (&std::path::Path, &[String]),
    measured: &measurement::Measurement,
    renamed: &HashMap<String, String>,
) -> Result<Side, Error> {
    let topology = |file: &str| {
        renamed
            .get(file)
            .cloned()
            .unwrap_or_else(|| file.to_string())
    };
    let facts = measured.facts();
    let layout = modules::topology(root, listed, facts, renamed);
    let mut files: Vec<String> = facts
        .iter()
        .map(|facts| topology(&facts.file))
        .chain(measured.unparsed.iter().map(|file| topology(&file.file)))
        .collect();
    files.sort_unstable();
    Ok(Side {
        files,
        unparsed: measured
            .unparsed
            .iter()
            .map(|file| syntax::Unparsed {
                file: topology(&file.file),
                language: file.language,
            })
            .collect(),
        graph: modules::build(&layout),
        current: renamed
            .iter()
            .map(|(now, was)| (was.clone(), now.clone()))
            .collect(),
    })
}

pub fn language_extensions() -> Vec<(&'static str, String)> {
    structural::language_extensions()
}

impl Side {
    fn current(&self, file: &str) -> String {
        self.current
            .get(file)
            .cloned()
            .unwrap_or_else(|| file.to_string())
    }

    /// The side's files as coverage counts them, under their current paths: attached, not
    /// attached, outside the section's scope, and refused by the grammar.
    fn covered(&self, policy: &Policy) -> coverage::Files {
        let mut out = coverage::Files::default();
        let unparsed: BTreeSet<&str> = self
            .unparsed
            .iter()
            .map(|file| file.file.as_str())
            .collect();
        let unattached: BTreeSet<&str> = self.graph.unattached.iter().map(String::as_str).collect();
        for file in &self.files {
            let into = if !policy.scope.selects(file) {
                &mut out.excluded
            } else if unparsed.contains(file.as_str()) {
                &mut out.unreadable
            } else if unattached.contains(file.as_str()) {
                &mut out.not_measured
            } else {
                &mut out.measured
            };
            into.push(self.current(file));
        }
        out
    }
}

fn policy(config: &Config) -> Result<Policy, Error> {
    let fields = config
        .required(SECTION)?
        .as_object()
        .ok_or_else(|| refused(config, "must be an object with \"layers\", or false"))?;
    let names: Vec<&str> = KEYS.iter().map(|key| key.name).collect();
    config::known_fields(&config.file, SECTION, fields, &names)?;
    let layers = layers(config, fields)?;
    names_layers(config, &layers)?;
    Ok(Policy {
        scope: Scope::read(config, SECTION, fields)?,
        acyclic: acyclic(config, fields)?,
        layers,
    })
}

fn acyclic(config: &Config, fields: &Map<String, Value>) -> Result<bool, Error> {
    match fields.get(ACYCLIC.name) {
        None => Ok(false),
        Some(Value::Bool(acyclic)) => Ok(*acyclic),
        Some(_) => Err(refused(
            config,
            "has an \"acyclic\" that is not true or false",
        )),
    }
}

fn layers(config: &Config, fields: &Map<String, Value>) -> Result<Vec<Layer>, Error> {
    let listed = fields
        .get(LAYERS.name)
        .and_then(Value::as_object)
        .filter(|layers| !layers.is_empty())
        .ok_or_else(|| {
            refused(
                config,
                "needs \"layers\", a map of layer name to the layer's \"in\" and \"can_use\"",
            )
        })?;
    listed
        .iter()
        .map(|(name, fields)| layer(config, name, fields))
        .collect()
}

fn layer(config: &Config, name: &str, value: &Value) -> Result<Layer, Error> {
    let fields = value
        .as_object()
        .ok_or_else(|| refused(config, &format!("layer \"{name}\" must be an object")))?;
    config::known_fields(
        &config.file,
        &format!("{SECTION} layer {name}"),
        fields,
        &[scope::IN.name, CAN_USE],
    )?;
    let within = scope::selectors(fields, scope::IN)
        .map_err(|why| refused(config, &format!("layer \"{name}\" {why}")))?;
    if within.is_empty() {
        return Err(refused(
            config,
            &format!("layer \"{name}\" needs an \"in\""),
        ));
    }
    Ok(Layer {
        name: name.to_string(),
        within,
        can_use: can_use(config, name, fields)?,
    })
}

/// The layers one layer may use, and `None` for every layer. Absent, a layer uses only itself.
fn can_use(
    config: &Config,
    name: &str,
    fields: &Map<String, Value>,
) -> Result<Option<Vec<String>>, Error> {
    let malformed = || {
        refused(
            config,
            &format!(
                "layer \"{name}\" has a \"can_use\" that is not a list of layer names or null"
            ),
        )
    };
    match fields.get(CAN_USE) {
        None => Ok(Some(Vec::new())),
        Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect::<Option<Vec<String>>>()
            .map(Some)
            .ok_or_else(malformed),
        Some(_) => Err(malformed()),
    }
}

fn names_layers(config: &Config, layers: &[Layer]) -> Result<(), Error> {
    for layer in layers {
        let unknown = layer
            .can_use
            .iter()
            .flatten()
            .find(|used| !layers.iter().any(|held| held.name == **used));
        if let Some(used) = unknown {
            return Err(refused(
                config,
                &format!(
                    "layer \"{}\" can use \"{used}\", which names no layer",
                    layer.name
                ),
            ));
        }
    }
    Ok(())
}

fn refused(config: &Config, why: &str) -> Error {
    Error(format!("{}: \"{SECTION}\" {why}", config.file.display()))
}

#[cfg(test)]
thread_local! {
    /// The work one side's judgement does, counted where a test asks: files placed, modules
    /// folded, dependency sites judged, and modules named. #220.
    static WORK: std::cell::Cell<[usize; 4]> = const { std::cell::Cell::new([0; 4]) };
}

/// The counters of `WORK`: files placed, modules folded, dependency sites judged, modules named.
const PLACED: usize = 0;
const FOLDED: usize = 1;
const JUDGED: usize = 2;
const NAMED: usize = 3;

/// Each module's report name and semantic identity on one side, each made at most once, so
/// naming costs one pass over a module's files and never one per site that reaches it.
struct Names<'s> {
    side: &'s Side,
    held: HashMap<usize, (String, String)>,
}

impl<'s> Names<'s> {
    fn new(side: &'s Side) -> Names<'s> {
        Names {
            side,
            held: HashMap::new(),
        }
    }

    fn of(&mut self, module: usize) -> &(String, String) {
        let side = self.side;
        self.held.entry(module).or_insert_with(|| {
            worked(NAMED);
            let current = |file: &str| side.current(file);
            let identity = side.graph.identity(module, current);
            let semantic = side.graph.semantic(module, &identity, current);
            (identity, semantic)
        })
    }
}

/// One unit of the work `WORK` counts, and nothing outside a test.
#[cfg_attr(not(test), allow(unused_variables))]
fn worked(what: usize) {
    #[cfg(test)]
    WORK.with(|work| {
        let mut counts = work.get();
        counts[what] += 1;
        work.set(counts);
    });
}

impl Policy {
    /// Every physical source of a graph placed once, and every module folded once over its
    /// sources.
    fn placed<'g>(&self, graph: &'g ModuleGraph) -> Placed<'g> {
        let mut files: HashMap<&str, Place> = HashMap::new();
        let modules = graph
            .modules
            .iter()
            .map(|module| {
                worked(FOLDED);
                module
                    .sources
                    .iter()
                    .map(|file| {
                        *files
                            .entry(file.as_str())
                            .or_insert_with(|| self.place(file))
                    })
                    .map(|place| (Folded::All(place.selected), Folded::All(place.layer)))
                    .reduce(|(scope, layer), (more, other)| (scope.and(more), layer.and(other)))
                    .unwrap_or((Folded::Mixed, Folded::Mixed))
            })
            .collect();
        Placed { files, modules }
    }

    fn place(&self, file: &str) -> Place {
        worked(PLACED);
        Place {
            selected: self.scope.selects(file),
            layer: self
                .layers
                .iter()
                .position(|layer| scope::any_holds(&layer.within, file)),
        }
    }

    /// The cycles among the dependencies the section judges on one side, where `acyclic` asks.
    fn cycles<'s>(&self, side: &'s Side, placed: &Placed) -> Option<Cycles<'s>> {
        self.acyclic.then(|| {
            side.graph
                .cycles(|dependency| placed.judges(&side.graph, dependency))
        })
    }

    /// The two layers a dependency crosses, where the source layer may not use the target
    /// layer, and `None` where it may or where either side is in no layer.
    fn forbidden(&self, from: Option<usize>, to: Option<usize>) -> Option<String> {
        let (source, target) = (&self.layers[from?], &self.layers[to?]);
        let allowed = source.name == target.name
            || source
                .can_use
                .as_ref()
                .is_none_or(|used| used.contains(&target.name));
        (!allowed).then(|| format!("{} → {}", source.name, target.name))
    }

    /// The configuration errors the trees reveal: an `in` that holds no structural file in either
    /// tree, so a layer the work emptied is still judged, and a file two layers hold.
    fn applies(&self, config: &Config, was: &Side, now: &Side) -> Result<(), Error> {
        let files = || was.files.iter().chain(&now.files);
        if self.scope.has_in() && !files().any(|file| self.scope.inside(file)) {
            return Err(refused(
                config,
                "has an \"in\" scope with no applicable file",
            ));
        }
        for layer in &self.layers {
            if !files().any(|file| scope::any_holds(&layer.within, file)) {
                return Err(refused(
                    config,
                    &format!(
                        "layer \"{}\" has an \"in\" with no applicable file",
                        layer.name
                    ),
                ));
            }
        }
        was.files
            .iter()
            .chain(&now.files)
            .try_for_each(|file| self.one_layer(config, file))
    }

    fn one_layer(&self, config: &Config, file: &str) -> Result<(), Error> {
        let holding: Vec<String> = self
            .layers
            .iter()
            .filter(|layer| scope::any_holds(&layer.within, file))
            .map(|layer| format!("\"{}\"", layer.name))
            .collect();
        match holding.len() {
            0 | 1 => Ok(()),
            _ => Err(refused(
                config,
                &format!(
                    "puts {file} in layers {} — a file belongs to at most one layer",
                    holding.join(" and ")
                ),
            )),
        }
    }
}

/// Every forbidden and cyclic semantic edge the section judges, with the physical sites that
/// write it, and every dependency the section cannot judge because the files of the module it
/// reaches straddle the scope or the layers. A straddled module is never placed by one of its
/// files. The sites are grouped by module pair first, and each pair's semantic identities are
/// named once. ADR 0058.
fn edges(
    policy: &Policy,
    (side, placed): (&Side, &Placed),
    cycles: Option<&Cycles>,
) -> (Edges, Vec<Hole>) {
    let mut pairs: BTreeMap<(usize, usize, String), Edge> = BTreeMap::new();
    let mut ambiguous = Vec::new();
    let mut names = Names::new(side);
    let graph = &side.graph;
    for (at, dependency) in graph.dependencies.iter().enumerate() {
        let Some(verdict) = verdict(policy, placed, cycles, graph, dependency).filter(|verdict| {
            verdict.forbidden.is_some() || verdict.cyclic || verdict.straddled.is_some()
        }) else {
            continue;
        };
        let file = graph.source(dependency);
        let target = names.of(dependency.to).0.clone();
        if let Some(what) = verdict.straddled {
            ambiguous.push(Hole {
                local_alias: false,
                file: side.current(file),
                line: dependency.line,
                start_byte: dependency.start_byte,
                text: target.clone(),
                why: format!("reaches a module whose files lie across {what}"),
            });
        }
        let mut add = |kind, text: String| {
            let edge = pairs
                .entry((dependency.from, dependency.to, text))
                .or_insert_with(|| Edge {
                    kind,
                    sites: BTreeMap::new(),
                    first: at,
                });
            edge.sites
                .entry(side.current(file))
                .or_default()
                .insert(dependency.line);
        };
        if let Some(layers) = verdict.forbidden {
            add(Kind::Forbidden, format!("{layers}: {target}"));
        }
        if verdict.cyclic {
            add(Kind::Cycle, format!("{CYCLE}: {target}"));
        }
    }
    (semantic(&mut names, pairs), ambiguous)
}

/// Module-pair edges keyed by the semantic identity of each module, merging the sites of any
/// two pairs whose modules name alike. ADR 0058.
fn semantic(names: &mut Names, pairs: BTreeMap<(usize, usize, String), Edge>) -> Edges {
    let mut out = Edges::new();
    for ((from, to, text), edge) in pairs {
        let key = (names.of(from).1.clone(), names.of(to).1.clone(), text);
        let merged = out.entry(key).or_insert_with(|| Edge {
            kind: edge.kind,
            sites: BTreeMap::new(),
            first: edge.first,
        });
        for (file, lines) in edge.sites {
            merged.sites.entry(file).or_default().extend(lines);
        }
    }
    out
}

/// What the section says of one dependency site: the layers it crosses where the policy
/// forbids that, whether it closes a cycle, and what it cannot judge because the files of the
/// module it reaches straddle the scope or the layers.
struct Verdict {
    forbidden: Option<String>,
    cyclic: bool,
    straddled: Option<&'static str>,
}

/// The verdict on one site, and `None` where the scope leaves out the file that writes it or
/// every file of the module it reaches. A module that straddles the scope gets no verdict; one
/// that straddles the layers gets no layer verdict and is still judged for cycles.
fn verdict(
    policy: &Policy,
    placed: &Placed,
    cycles: Option<&Cycles>,
    graph: &ModuleGraph,
    dependency: &Dependency,
) -> Option<Verdict> {
    worked(JUDGED);
    let from = placed.files[graph.source(dependency)];
    let (scope, layer) = placed.modules[dependency.to];
    match scope {
        _ if !from.selected => None,
        Folded::All(false) => None,
        Folded::Mixed => Some(Verdict {
            forbidden: None,
            cyclic: false,
            straddled: Some("the section's scope"),
        }),
        Folded::All(true) => Some(Verdict {
            forbidden: match layer {
                Folded::All(to) => policy.forbidden(from.layer, to),
                Folded::Mixed => None,
            },
            cyclic: cycles.is_some_and(|cycles| cycles.closes(dependency)),
            straddled: (layer == Folded::Mixed && from.layer.is_some())
                .then_some("more than one layer, or a layer and none"),
        }),
    }
}

/// The working tree's semantic edges paired with the base's before any finding exists, then
/// reported where they are written: one finding per file and text, held where the base holds
/// every semantic edge it merges. Evidence that moves between the files of one module, splits
/// across them or joins in one is held; a new semantic edge is new wherever it is written.
/// ADR 0058.
fn physicals(now: &Edges, was: &Edges) -> Physicals {
    let mut out = Physicals::new();
    for (key, edge) in now {
        let held = was.contains_key(key);
        for (file, lines) in &edge.sites {
            let physical = out
                .entry((file.clone(), key.2.clone()))
                .or_insert_with(|| Physical {
                    kind: edge.kind,
                    lines: BTreeSet::new(),
                    first: edge.first,
                    held: true,
                });
            physical.lines.extend(lines);
            physical.held &= held;
        }
    }
    out
}

/// The working tree's findings, each new cyclic one with one cycle that explains it.
fn findings(physicals: &Physicals, graph: &ModuleGraph, cycles: Option<&Cycles>) -> Vec<Finding> {
    let mut out: Vec<Finding> = physicals
        .iter()
        .map(|((file, text), physical)| {
            let mut values = Values::new();
            values.insert(EDGE.into(), 1.into());
            values.insert(KIND.into(), physical.kind.word().into());
            values.insert(SITES.into(), physical.lines.len().into());
            if let (false, Kind::Cycle, Some(cycles)) = (physical.held, physical.kind, cycles) {
                let path = cycles.path(&graph.dependencies[physical.first]).join(" → ");
                values.insert(PATH.into(), path.into());
            }
            Finding {
                file: file.clone(),
                line: physical.lines.first().copied().unwrap_or_default(),
                text: text.clone(),
                values,
                body: None,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

fn judged(
    at: &Context,
    (policy, now, placed): (&Policy, &Side, &Placed),
    findings: Vec<Finding>,
    (physicals, was_edges, now_edges): (&Physicals, &Edges, &Edges),
    out: &mut Sink,
) -> Result<u8, Error> {
    let prior = prior(physicals, (was_edges, now_edges));
    let kinds = |kind: Kind| {
        findings
            .iter()
            .filter(|finding| finding.values.get(KIND).and_then(Value::as_str) == Some(kind.word()))
            .count()
    };
    let (forbidden, cyclic) = (kinds(Kind::Forbidden), kinds(Kind::Cycle));
    let said = out.covered(&now.covered(policy).coverage(None));
    let sites = now
        .graph
        .dependencies
        .iter()
        .filter(|dependency| placed.judges(&now.graph, dependency))
        .count();
    let state = Measured::Layering(contract::Layering {
        sites,
        forbidden,
        cyclic,
        ..attachment(&now.graph)
    });
    let evaluator = evaluator();
    Ok(evaluator.evaluate(
        findings,
        prior,
        ratchet::accepted(&at.project.config, at.gate, evaluator.metrics)?,
        at,
        Line::new(state, said),
        out,
    ))
}

/// The base's sites for the ratchet: each working-tree finding the semantic pairing held, at
/// that finding's own site, and each base finding the working tree no longer reports at all,
/// so the count of what the base holds still names debt since fixed. A base finding whose site
/// the working tree reports without holding it is left out, or it would hold a new semantic
/// edge by its site. An accepted entry is still matched by the site a person wrote, and never
/// follows a move.
fn prior(physicals: &Physicals, (was_edges, now_edges): (&Edges, &Edges)) -> Vec<Finding> {
    let site = |(file, text): &(String, String), lines: &BTreeSet<u64>| Finding {
        file: file.clone(),
        line: lines.first().copied().unwrap_or_default(),
        text: text.clone(),
        values: Values::from_iter([(EDGE.to_string(), Value::from(1))]),
        body: None,
    };
    let mut out: Vec<Finding> = physicals
        .iter()
        .filter(|(_, physical)| physical.held)
        .map(|(key, physical)| site(key, &physical.lines))
        .collect();
    let mut retired: BTreeMap<(String, String), BTreeSet<u64>> = BTreeMap::new();
    for (key, edge) in was_edges
        .iter()
        .filter(|(key, _)| !now_edges.contains_key(*key))
    {
        for (file, lines) in &edge.sites {
            retired
                .entry((file.clone(), key.2.clone()))
                .or_default()
                .extend(lines);
        }
    }
    out.extend(
        retired
            .iter()
            .filter(|(key, _)| !physicals.contains_key(*key))
            .map(|(key, lines)| site(key, lines)),
    );
    out
}

/// How the working tree's files came to be modules, and how many dependencies V1 left alone.
/// How the module graph attached the files, as the `OK:` line counts them. The caller sets the
/// counts of what it judged.
fn attachment(graph: &ModuleGraph) -> contract::Layering {
    let count = |kind: Attachment| {
        graph
            .attached
            .values()
            .filter(|held| **held == kind)
            .count()
    };
    contract::Layering {
        attached: graph.attached.len(),
        by_manifest: count(Attachment::Manifest),
        by_convention: count(Attachment::Convention),
        unattached: graph.unattached.len(),
        external: graph.external,
        ..contract::Layering::default()
    }
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[EDGE],
        unit: "dependency edge(s)",
        condition: "where a dependency crosses a layer the policy forbids or closes a cycle",
        fix_advice: Remedy::Fixed(REMEDY),
        ceiling: None,
        format_metrics: show,
        nested: None,
    }
}

fn show(values: &Values) -> String {
    let sites = values.get(SITES).and_then(Value::as_u64).unwrap_or(1);
    match (
        values.get(KIND).and_then(Value::as_str),
        values.get(PATH).and_then(Value::as_str),
    ) {
        (Some(FORBIDDEN), _) => format!("forbidden, {sites} site(s)"),
        (Some(CYCLE), Some(path)) => format!("cyclic, {sites} site(s), {path}"),
        (Some(CYCLE), None) => format!("cyclic, {sites} site(s)"),
        _ => EDGE.to_string(),
    }
}

impl Side {
    /// The holes where the section's scope selects the source file that writes one, and every
    /// hole a manifest writes, because a manifest's target decides modules in every scope.
    fn holes(&self, policy: &Policy) -> Vec<&Hole> {
        self.graph
            .holes
            .iter()
            .filter(|hole| {
                policy.scope.selects(&hole.file) || self.files.binary_search(&hole.file).is_err()
            })
            .collect()
    }
}

/// The dependency forms the working tree's resolvers support and could not resolve, beside the
/// ones the base could not resolve either, under today's paths. ADR 0021, spec 8.6.
fn holes_said(
    (was, now): (&Side, &Side),
    policy: &Policy,
    (was_ambiguous, ambiguous): (&[Hole], &[Hole]),
    at: &Context,
    out: &mut Sink,
) {
    let row = |file: String, hole: &Hole| coverage::Unresolved {
        file,
        line: hole.line,
        text: hole.text.clone(),
        why: hole.why.clone(),
    };
    let named: Vec<coverage::Unresolved> = now
        .holes(policy)
        .into_iter()
        .chain(ambiguous)
        .map(|hole| row(hole.file.clone(), hole))
        .collect();
    let base = || {
        was.holes(policy)
            .into_iter()
            .map(|hole| row(was.current(&hole.file), hole))
            .chain(
                was_ambiguous
                    .iter()
                    .map(|hole| row(hole.file.clone(), hole)),
            )
            .collect()
    };
    holes::unresolved_said((&named, base), Unresolvable::Dependency, at, out);
}

fn held_note(physicals: &Physicals, out: &mut Sink) {
    let held: Vec<&(String, String)> = physicals
        .iter()
        .filter(|(_, physical)| physical.held)
        .map(|(key, _)| key)
        .collect();
    if held.is_empty() {
        return;
    }
    out.tell(Listed::Held(HeldAtBase::Edges(
        held.into_iter()
            .map(|(file, text)| Site {
                file: file.clone(),
                text: text.clone(),
            })
            .collect(),
    )));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::resolver::Module;
    use crate::scope::Selector;

    fn module(name: &str, sources: &[&str]) -> Module {
        Module {
            name: name.to_string(),
            sources: sources.iter().map(|file| file.to_string()).collect(),
            nesting: Vec::new(),
            target: None,
            parent: None,
            children: BTreeMap::new(),
            unresolved: BTreeSet::new(),
            bound: BTreeSet::new(),
        }
    }

    fn site(from: usize, to: usize, source: u32, line: u64) -> Dependency {
        Dependency {
            from,
            to,
            source,
            line,
            start_byte: line,
        }
    }

    fn policy(except: &[&str]) -> Policy {
        let layer = |name: &str| Layer {
            name: name.to_string(),
            within: vec![Selector::parse(scope::IN, name).unwrap()],
            can_use: Some(Vec::new()),
        };
        let fields = match except {
            [] => serde_json::json!({}),
            _ => serde_json::json!({ "except": except }),
        };
        Policy {
            scope: Scope::from_fields(fields.as_object().unwrap()).unwrap(),
            acyclic: true,
            layers: vec![layer("a"), layer("b"), layer("c")],
        }
    }

    fn side(modules: Vec<Module>, dependencies: Vec<Dependency>) -> Side {
        Side {
            files: Vec::new(),
            unparsed: Vec::new(),
            graph: ModuleGraph {
                modules,
                dependencies,
                ..ModuleGraph::default()
            },
            current: HashMap::new(),
        }
    }

    fn work() -> [usize; 4] {
        WORK.with(|work| work.replace([0; 4]))
    }

    /// Many modules of many files, each file writing repeated sites to the same two modules:
    /// each file is placed once, each module folded once and named once however many sites
    /// reach it, each site judged once, and the components are found over each module pair
    /// once. No module's name starts with one of its files, so naming one scans them all.
    #[test]
    fn multi_source_work_is_linear_in_files_modules_sites_and_unique_edges() {
        let (count, files, lines) = (100, 10, 5);
        let names: Vec<Vec<String>> = (0..count)
            .map(|at| {
                let layer = if at < count / 2 { "a" } else { "b" };
                (0..files)
                    .map(|file| format!("{layer}/m{at:03}/f{file:02}.go"))
                    .collect()
            })
            .collect();
        let modules = names
            .iter()
            .enumerate()
            .map(|(at, sources)| {
                let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
                module(&format!("m{at:03}"), &sources)
            })
            .collect();
        let mut dependencies = Vec::new();
        for from in 0..count {
            let mut targets = [(from + 1) % count, (from + 2) % count];
            targets.sort_unstable();
            for to in targets {
                for source in 0..files as u32 {
                    for line in 1..=lines {
                        dependencies.push(site(from, to, source, line));
                    }
                }
            }
        }
        let side = side(modules, dependencies);
        let sites = count * 2 * files * lines as usize;
        let policy = policy(&[]);
        work();
        let placed = policy.placed(&side.graph);
        let cycles = policy.cycles(&side, &placed).unwrap();
        let (edges, ambiguous) = edges(&policy, (&side, &placed), Some(&cycles));
        assert_eq!(work(), [count * files, count, sites, count]);
        assert_eq!(side.graph.dependencies.len(), sites);
        assert_eq!(cycles.edges(), count * 2);
        assert_eq!(side.graph.cost().edges, count * 2);
        let forbidden = edges.values().filter(|edge| edge.kind == Kind::Forbidden);
        assert_eq!(forbidden.count(), 6);
        assert_eq!(edges.len(), 6 + count * 2);
        assert!(ambiguous.is_empty());
    }

    /// A site the section allows and no cycle closes names no module, so a passing graph pays
    /// for no report name. #221.
    #[test]
    fn an_allowed_site_names_no_module() {
        let modules = vec![module("a/x.rs", &["a/x.rs"]), module("a/y.rs", &["a/y.rs"])];
        let side = side(modules, vec![site(0, 1, 0, 1)]);
        let policy = policy(&[]);
        let placed = policy.placed(&side.graph);
        let cycles = policy.cycles(&side, &placed).unwrap();
        work();
        let (edges, ambiguous) = edges(&policy, (&side, &placed), Some(&cycles));
        assert_eq!(work()[NAMED], 0);
        assert!(edges.is_empty() && ambiguous.is_empty());
    }

    /// A module whose files straddle two layers, or the scope, is never placed by one of them:
    /// a dependency on it is a visible ambiguity, with no layer verdict, and a dependency on a
    /// module that straddles the scope never enters a cycle.
    #[test]
    fn a_straddled_destination_is_ambiguous_and_never_judged() {
        let modules = vec![
            module("x", &["a/x.go", "b/x.go"]),
            module("y", &["a/y1.go", "b/y2.go"]),
        ];
        let dependencies = vec![site(0, 1, 0, 3), site(1, 0, 0, 3), site(1, 0, 1, 3)];
        let side = side(modules, dependencies);
        let across_layers = policy(&[]);
        let placed_layers = across_layers.placed(&side.graph);
        let (edges, ambiguous) = edges(&across_layers, (&side, &placed_layers), None);
        assert!(edges.is_empty());
        let why: Vec<&str> = ambiguous.iter().map(|hole| hole.why.as_str()).collect();
        assert_eq!(why.len(), 3);
        assert!(why.iter().all(|why| why.contains("more than one layer")));
        let across_scope = policy(&["b"]);
        let placed_scope = across_scope.placed(&side.graph);
        let cycles = across_scope.cycles(&side, &placed_scope).unwrap();
        assert_eq!(cycles.edges(), 0);
        let (edges, ambiguous) = super::edges(&across_scope, (&side, &placed_scope), Some(&cycles));
        assert!(edges.is_empty());
        let sites: Vec<(&str, u64)> = ambiguous
            .iter()
            .map(|hole| (hole.file.as_str(), hole.line))
            .collect();
        assert_eq!(sites, [("a/x.go", 3), ("a/y1.go", 3)]);
        assert!(ambiguous[0].why.contains("the section's scope"));
    }

    /// Two files of one module that write a dependency on the same line are two sites.
    #[test]
    fn a_site_is_its_file_and_offset() {
        let modules = vec![
            module("x", &["a/x1.go", "a/x2.go"]),
            module("y", &["a/y.go"]),
            module("z", &["a/z.go"]),
        ];
        let side = side(modules, vec![site(0, 1, 0, 7), site(0, 2, 1, 7)]);
        assert_eq!(side.graph.reached_at(0, "a/x1.go", 7), [1]);
        assert_eq!(side.graph.reached_at(0, "a/x2.go", 7), [2]);
    }

    /// What the ratchet makes of the working tree's findings where module `p` of files `p1`
    /// and `p2` depends on module `q`, as `sites` of the base and of the working tree give it.
    fn outcomes(
        p: [&str; 2],
        was: &[(u32, u64)],
        now: &[(u32, u64)],
        accepted: &[&str],
    ) -> Vec<(String, &'static str)> {
        let policy = policy(&[]);
        let judged = |sites: &[(u32, u64)]| {
            let modules = vec![module("p", &p), module("q", &["c/q.go"])];
            let dependencies = sites
                .iter()
                .map(|(file, line)| site(0, 1, *file, *line))
                .collect();
            let side = side(modules, dependencies);
            let placed = policy.placed(&side.graph);
            (edges(&policy, (&side, &placed), None).0, side)
        };
        let ((was, _), (now, now_side)) = (judged(was), judged(now));
        let physicals = physicals(&now, &was);
        let found = findings(&physicals, &now_side.graph, None);
        let entries = accepted
            .iter()
            .map(|file| {
                let entry = serde_json::json!({
                    "gate": SECTION, "file": file, "text": "b → c: c/q.go", "edge": 1, "accepted": true,
                });
                entry.as_object().unwrap().clone()
            })
            .collect();
        ratchet::outcomes(found, prior(&physicals, (&was, &now)), entries, &[EDGE])
            .into_iter()
            .map(|(finding, outcome)| (finding.file, outcome))
            .collect()
    }

    /// An edge that moves to another file of its module is one base site at its new place, and
    /// the file it left is not retired debt as well.
    #[test]
    fn a_moved_semantic_edge_is_held_once() {
        let policy = policy(&[]);
        let judged = |source: u32| {
            let modules = vec![
                module("p", &["b/p1.go", "b/p2.go"]),
                module("q", &["c/q.go"]),
            ];
            let side = side(modules, vec![site(0, 1, source, 5)]);
            let placed = policy.placed(&side.graph);
            edges(&policy, (&side, &placed), None).0
        };
        let (was, now) = (judged(0), judged(1));
        let prior = prior(&physicals(&now, &was), (&was, &now));
        let sites: Vec<&str> = prior.iter().map(|finding| finding.file.as_str()).collect();
        assert_eq!(sites, ["b/p2.go"]);
    }

    /// Evidence of one semantic edge that moves between the files of its module, splits across
    /// them, joins in one of them, or gains a site in another is held; moved to a file of
    /// another layer it is another edge, and new.
    #[test]
    fn a_semantic_edge_is_paired_before_its_findings_are_made() {
        let files = ["b/p1.go", "b/p2.go"];
        let held = |file: &str| (file.to_string(), "held");
        assert_eq!(
            outcomes(files, &[(0, 5)], &[(1, 9)], &[]),
            [held("b/p2.go")]
        );
        assert_eq!(
            outcomes(files, &[(0, 5), (0, 20)], &[(0, 5), (1, 9)], &[]),
            [held("b/p1.go"), held("b/p2.go")]
        );
        assert_eq!(
            outcomes(files, &[(0, 5), (1, 9)], &[(0, 5), (0, 20)], &[]),
            [held("b/p1.go")]
        );
        assert_eq!(
            outcomes(files, &[(0, 5)], &[(0, 5), (1, 9)], &[]),
            [held("b/p1.go"), held("b/p2.go")]
        );
        assert_eq!(
            outcomes(["a/p2.go", "b/p1.go"], &[(1, 5)], &[(0, 9)], &[]),
            [("a/p2.go".to_string(), "new")]
        );
        assert_eq!(
            outcomes(files, &[(0, 5)], &[(1, 9)], &["b/p1.go"]),
            [held("b/p2.go")]
        );
        assert_eq!(
            outcomes(files, &[], &[(1, 9)], &["b/p1.go"]),
            [("b/p2.go".to_string(), "new")]
        );
    }
}
