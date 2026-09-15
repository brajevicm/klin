//! The `layering` check: a dependency that crosses a declared architectural boundary, or that
//! closes a dependency cycle, instead of going through the interface the architecture intends.
//! It reads the module graph of both trees and judges resolved dependencies only; containment
//! is never a dependency. A forbidden edge is keyed by the file that writes it, the two layers
//! and the module it reaches, named by its file and inline modules, and a cyclic edge by the file
//! and the module. Each carries `edge`
//! at 1, so a base edge with the same key is held and any other is new. A renamed file is placed
//! in the base's layers under its base path and keyed under its current one. The section is a
//! person's policy and nothing derives it: with no section the gate does not run. Spec 8.2.1,
//! ADR 0043.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{Map, Value};

use crate::changed::Change;
use crate::check::{self, Context, Sink};
use crate::config::{self, Config, Error};
use crate::modules::{self, Attachment, Cycles, GraphCost, Hole, ModuleGraph, Topology};
use crate::project::{Project, Tree};
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::Key;
use crate::scope::{self, Scope, Selector};
use crate::syntax::{self, structural};
use crate::{base, coverage, files};

pub const SECTION: &str = "layering";

const ACYCLIC: Key = Key {
    name: "acyclic",
    holds: "`true` to fail a dependency that closes a module cycle the base did not hold",
    required: false,
    rule: None,
    default: "`false`",
};

const LAYERS: Key = Key {
    name: "layers",
    holds: "a map of layer name to a layer: `in`, a repository-relative path or list of them the layer holds, and `can_use`, the layers it may depend on, or `null` for every layer. A layer may always depend on itself, and a file belongs to at most one layer",
    required: true,
    rule: None,
    default: "",
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

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
    /// Fail when an accepted entry matches nothing — what CI runs
    #[arg(long)]
    strict: bool,
}

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

/// One judged edge: its kind, the lines that write it, and the first dependency that does.
struct Edge {
    kind: Kind,
    lines: BTreeSet<u64>,
    first: usize,
}

type Edges = BTreeMap<(String, String), Edge>;

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let project = Project::load(args.config.as_deref(), start)?;
    let at = Context {
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand(SECTION, &project)
    };
    gate(&at, &mut Sink::unrecorded(out))
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let policy = policy(at.config())?;
    let commit = base::commit(at.project.root(), at, out)?;
    let (was, now) = sides(at, &commit, out)?;
    policy.applies(at.config(), &was, &now)?;
    let started = Instant::now();
    let (was_cycles, now_cycles) = (policy.cycles(&was), policy.cycles(&now));
    let was_edges = edges(&policy, &was, was_cycles.as_ref());
    let now_edges = edges(&policy, &now, now_cycles.as_ref());
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
    let findings = findings(&now_edges, &now.graph, now_cycles.as_ref(), &was_edges);
    let code = judged(at, (&policy, &now), findings, &was_edges, out)?;
    let code = coverage::lost_said(
        &now.covered(&policy)
            .lost(&was.covered(&policy), at.project, None),
        at,
        code,
        out,
    );
    let code = holes_said(&now, &policy, at, code, out);
    let unparsed: Vec<syntax::Unparsed> = now
        .unparsed
        .iter()
        .filter(|file| policy.scope.selects(&file.file))
        .cloned()
        .collect();
    held_note(&now_edges, &was_edges, out);
    Ok(syntax::unread(&unparsed, at, code, out))
}

/// The base and the working tree, each measured and resolved. A changed run that is not strict
/// takes the base's facts for every file it did not change, as `dead-symbols` does.
fn sides(at: &Context, commit: &str, out: &mut Sink) -> Result<(Side, Side), Error> {
    let project = at.project;
    let prior = base::whole(at, commit)?;
    let unchanged = base::unchanged(at, prior, commit)?;
    let mut after = measure(project.tree(), unchanged.as_ref())?;
    let before = measure(prior.tree(), None)?;
    after.cost = after.cost
        + unchanged.map_or_else(
            structural::ExtractionCost::default,
            structural::Unchanged::publish,
        );
    out.record(|records| records.facts = Some(before.cost + after.cost));
    let renamed = renamed(&project.changes(commit)?);
    Ok((
        side(prior.tree(), &before, &renamed)?,
        side(project.tree(), &after, &HashMap::new())?,
    ))
}

/// Every file the change set renamed, by its current path, with the path it had at the base.
fn renamed(changes: &[Change]) -> HashMap<String, String> {
    changes
        .iter()
        .filter_map(|change| {
            let was = change.was.as_ref().filter(|was| **was != change.path)?;
            Some((change.path.clone(), was.clone()))
        })
        .collect()
}

fn side(
    tree: &Tree,
    measured: &structural::Measurement,
    renamed: &HashMap<String, String>,
) -> Result<Side, Error> {
    let topology = |file: &str| {
        renamed
            .get(file)
            .cloned()
            .unwrap_or_else(|| file.to_string())
    };
    let layout = Topology::new(tree.root(), tree.files()?, measured.index.files(), renamed);
    let mut files: Vec<String> = measured
        .index
        .files()
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

/// Every structural file of a tree, measured through the tree's one extraction, because a module
/// anywhere in the tree may be the one a dependency reaches.
fn measure(
    tree: &Tree,
    unchanged: Option<&structural::Unchanged>,
) -> Result<structural::Measurement, Error> {
    let extensions = structural::selected_extensions(&[]).unwrap_or_default();
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let found = files::found(tree, &[tree.root().to_path_buf()], &wanted)?;
    structural::measure(found, tree, unchanged)
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

impl Policy {
    /// The cycles among the side's modules the section's scope selects, where `acyclic` asks.
    fn cycles<'s>(&self, side: &'s Side) -> Option<Cycles<'s>> {
        self.acyclic
            .then(|| side.graph.cycles(|module| self.scope.selects(&module.file)))
    }

    fn layer(&self, file: &str) -> Option<&Layer> {
        self.layers
            .iter()
            .find(|layer| scope::any_holds(&layer.within, file))
    }

    /// The two layers a dependency between these files crosses, where the source layer may not
    /// use the target layer, and `None` where it may or where either file is in no layer.
    fn forbidden(&self, from: &str, to: &str) -> Option<String> {
        let (source, target) = (self.layer(from)?, self.layer(to)?);
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

/// Every forbidden and cyclic edge between two files the section's scope selects, keyed under
/// current paths.
fn edges(policy: &Policy, side: &Side, cycles: Option<&Cycles>) -> Edges {
    let mut out = Edges::new();
    let graph = &side.graph;
    for (at, dependency) in graph.dependencies.iter().enumerate() {
        let from = &graph.modules[dependency.from].file;
        let to = &graph.modules[dependency.to].file;
        if !policy.scope.selects(from) || !policy.scope.selects(to) {
            continue;
        }
        let target = graph.identity(dependency.to, |file| side.current(file));
        let mut add = |kind, text: String| {
            let edge = out
                .entry((side.current(from), text))
                .or_insert_with(|| Edge {
                    kind,
                    lines: BTreeSet::new(),
                    first: at,
                });
            edge.lines.insert(dependency.line);
        };
        if let Some(layers) = policy.forbidden(from, to) {
            add(Kind::Forbidden, format!("{layers}: {target}"));
        }
        if cycles.is_some_and(|cycles| cycles.closes(dependency)) {
            add(Kind::Cycle, format!("{CYCLE}: {target}"));
        }
    }
    out
}

/// The working tree's edges as findings, each new cyclic edge with one cycle that explains it.
fn findings(
    edges: &Edges,
    graph: &ModuleGraph,
    cycles: Option<&Cycles>,
    held: &Edges,
) -> Vec<Finding> {
    let mut out: Vec<Finding> = edges
        .iter()
        .map(|((file, text), edge)| {
            let mut values = Values::new();
            values.insert(EDGE.into(), 1.into());
            values.insert(KIND.into(), edge.kind.word().into());
            values.insert(SITES.into(), edge.lines.len().into());
            let fresh = !held.contains_key(&(file.clone(), text.clone()));
            if let (true, Some(cycles)) = (fresh && edge.kind == Kind::Cycle, cycles) {
                let path = cycles.path(&graph.dependencies[edge.first]).join(" → ");
                values.insert(PATH.into(), path.into());
            }
            Finding {
                file: file.clone(),
                line: edge.lines.first().copied().unwrap_or_default(),
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
    (policy, now): (&Policy, &Side),
    findings: Vec<Finding>,
    was_edges: &Edges,
    out: &mut Sink,
) -> Result<u8, Error> {
    let prior = was_edges
        .iter()
        .map(|((file, text), edge)| Finding {
            file: file.clone(),
            line: edge.lines.first().copied().unwrap_or_default(),
            text: text.clone(),
            values: Values::from_iter([(EDGE.to_string(), Value::from(1))]),
            body: None,
        })
        .collect();
    let kinds = |kind: Kind| {
        findings
            .iter()
            .filter(|finding| finding.values.get(KIND).and_then(Value::as_str) == Some(kind.word()))
            .count()
    };
    let (forbidden, cyclic) = (kinds(Kind::Forbidden), kinds(Kind::Cycle));
    let said = now.covered(policy).coverage(None).said(out);
    let ok = format!(
        "OK: {} dependency edge(s) judged, {forbidden} forbidden, {cyclic} cyclic, all held at the base{said}; {}",
        selected_dependencies(policy, &now.graph),
        attachment(&now.graph)
    );
    let evaluator = evaluator();
    Ok(evaluator.evaluate(
        findings,
        prior,
        ratchet::accepted(&at.project.config, at.gate, evaluator.metrics)?,
        at,
        &ok,
        out,
    ))
}

fn selected_dependencies(policy: &Policy, graph: &ModuleGraph) -> usize {
    graph
        .dependencies
        .iter()
        .filter(|dependency| {
            policy.scope.selects(&graph.modules[dependency.from].file)
                && policy.scope.selects(&graph.modules[dependency.to].file)
        })
        .count()
}

/// How the working tree's files came to be modules, and how many dependencies V1 left alone.
fn attachment(graph: &ModuleGraph) -> String {
    let count = |kind: Attachment| {
        graph
            .attached
            .values()
            .filter(|held| **held == kind)
            .count()
    };
    format!(
        "{} file(s) attached, {} by a Cargo manifest and {} by a conventional root, {} not attached, {} external or unsupported dependenc(ies)",
        graph.attached.len(),
        count(Attachment::Manifest),
        count(Attachment::Convention),
        graph.unattached.len(),
        graph.external
    )
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[EDGE],
        unit: "dependency edge(s)",
        condition: "where a dependency crosses a layer the policy forbids or closes a cycle",
        fix_advice: REMEDY,
        ceiling: None,
        format_metrics: show,
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

/// The dependency forms the working tree's resolvers support and could not resolve: a NOTE in
/// the hook, and exit 2 elsewhere, because a green run must not imply a resolution klin did not
/// make. ADR 0021, spec 8.6.
fn holes_said(now: &Side, policy: &Policy, at: &Context, code: u8, out: &mut Sink) -> u8 {
    let named = now.holes(policy);
    if named.is_empty() {
        return code;
    }
    let word = if at.hook() { "NOTE" } else { "FAIL" };
    let _ = writeln!(
        out.text,
        "{word}: {} dependency form(s) klin resolves could not be resolved, so what they reach was not judged:",
        named.len()
    );
    for hole in &named {
        let _ = writeln!(
            out.text,
            "  {}:{}  {}  — {}",
            hole.file, hole.line, hole.text, hole.why
        );
    }
    let _ = writeln!(
        out.text,
        "Make each one name exactly one module file the tree holds, or take its file out of the section's scope."
    );
    out.record(|records| {
        for hole in &named {
            let record = serde_json::json!({
                "outcome": check::UNRESOLVED,
                "file": hole.file,
                "line": hole.line,
                "text": format!("{} — {}", hole.text, hole.why),
            });
            match at.hook() {
                true => records.notes.push(record),
                false => records.findings.push(record),
            }
        }
    });
    if at.hook() { code } else { 2 }
}

fn held_note(now: &Edges, was: &Edges, out: &mut Sink) {
    let held: Vec<&(String, String)> = now.keys().filter(|key| was.contains_key(*key)).collect();
    if held.is_empty() {
        return;
    }
    let mut note = format!(
        "{} forbidden or cyclic edge(s) the base already held:",
        held.len()
    );
    for (file, text) in held.iter().take(20) {
        let _ = write!(note, "\n  {file}  {text}");
    }
    if held.len() > 20 {
        let _ = write!(note, "\n  … and {} more", held.len() - 20);
    }
    ratchet::noted(&[(String::new(), note)], out);
}
