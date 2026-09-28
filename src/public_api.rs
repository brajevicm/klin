//! The `public-api` check: a consumer-facing Rust or TypeScript contract the base exposed is
//! gone, or its declared contract changed, while the repository still compiles. It derives the
//! public surfaces of both trees from Cargo library targets and package entry points, through
//! the shared structural facts and module graph, and needs no configuration. Identity is the
//! surface a consumer addresses plus the exported path or name and the item's kind, never the
//! file that declares it, so a move behind an unchanged identity passes. Each break carries
//! `break` at 1, the base holds none, and an intentional break is an accepted entry. The
//! section is absent, or `false` to exclude the gate. Spec 8.2.1, ADR 0044.

use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::base;
use crate::changed;
use crate::check::{Context, Sink};
use crate::config::Error;
use crate::coverage::{self, Coverage};
use crate::modules::{self, ModuleGraph, Topology};
use crate::project::{Project, Tree};
use crate::ratchet::{self, Evaluator, Finding, Line, Remedy, Values};
use crate::reference::Key;
use crate::surface::{self, Contract, Derived, Item, MODULE, Surface};
use crate::syntax::{self, structural};

pub const SECTION: &str = "public_api";
pub const NAME: &str = "public-api";
pub const KEYS: &[Key] = &[];
pub const REFERENCE_TEXT: &str = "A Cargo library target and a TypeScript package entry point are surfaces whether or not the package can be published: `publish = false` and `\"private\": true` do not make a package not applicable (ADR 0050).";

const BREAK: &str = "break";
const KIND: &str = "kind";
const ORIGIN: &str = "origin";
const WAS: &str = "was";
const NOW: &str = "now";
const REMOVED_SURFACE: &str = "removed surface";
const REMOVED: &str = "removed";
const CHANGED: &str = "changed";
const SURFACE_TEXT: &str = "(surface)";
const REMEDY: &str = "Keep the surface, the item or the declared contract the base had where the task \
                      allows it. For a changed contract, a new item beside the unchanged one keeps \
                      the base's contract where that serves the task. Do not change what the task \
                      asked for only to satisfy this gate. If the break is intended, a person \
                      accepts it with an accepted entry in a reviewed commit, and until then CI \
                      refuses it.";
const HOOK_REMEDY: &str = "Keep the surface, the item or the declared contract the base had where \
                           the task allows it. For a changed contract, a new item beside the \
                           unchanged one keeps the base's contract where that serves the task. Do \
                           not change what the task asked for only to satisfy this gate. If this \
                           stop blocked on a break the task intends, say so in your reply and stop \
                           again, and the next stop may then end the turn. Your reply does not \
                           accept the break: a person accepts it with an accepted entry in a \
                           reviewed commit, and until then CI refuses it.";

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
    /// Print the derived public surfaces of the working tree, item by item, without judging them
    #[arg(long)]
    report: bool,
}

/// One tree as the gate judges it: its surfaces, its module graph, the files the grammar refused
/// under today's paths, and today's path of every file the base names by its path at the base.
struct Side {
    derived: Derived,
    graph: ModuleGraph,
    unparsed: Vec<syntax::Unparsed>,
    current: HashMap<String, String>,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let project = Project::load(args.config.as_deref(), start)?;
    let at = Context {
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand(NAME, &project)
    };
    let mut sink = Sink::unrecorded(out);
    match args.report {
        true => report(&at, &mut sink),
        false => gate(&at, &mut sink),
    }
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    at.config().policy(SECTION, KEYS)?;
    let commit = base::commit(at.project.root(), at, out)?;
    let (was, now) = sides(at, &commit, out)?;
    out.record(|records| {
        records.graph = Some(was.graph.cost() + now.graph.cost());
        records.surface = Some(was.derived.cost() + now.derived.cost());
    });
    let findings = breaks(&was.derived, &now.derived);
    let code = judged(at, &now, findings, out)?;
    let code = holes_said((&was, &now), at, code, out);
    inapplicable_note(&now.derived, out);
    let inside: Vec<syntax::Unparsed> = now
        .unparsed
        .iter()
        .filter(|file| {
            now.derived
                .surfaces
                .iter()
                .any(|surface| surface.files.binary_search(&file.file).is_ok())
        })
        .cloned()
        .collect();
    let unreadable: Vec<String> = was.unparsed.into_iter().map(|file| file.file).collect();
    let unread_at_base = base::whole(at, &commit)?.unread_either(&unreadable);
    Ok(syntax::unread(&inside, &unread_at_base, at, code, out))
}

/// The base and the working tree, each measured, resolved and derived. A changed run that is
/// not strict takes the base's facts for every file it did not change, as `layering` does, and
/// judges every file of both trees whatever its scope, because a manifest or a re-export can
/// change what an unchanged file means to a consumer.
fn sides(at: &Context, commit: &str, out: &mut Sink) -> Result<(Side, Side), Error> {
    let project = at.project;
    let prior = base::whole(at, commit)?;
    let unchanged = base::unchanged(at, prior, commit)?;
    let mut after = structural::measure_all(project.tree(), unchanged.as_ref())?;
    let before = structural::measure_all(prior.tree(), None)?;
    after.cost = after.cost
        + unchanged.map_or_else(
            structural::ExtractionCost::default,
            structural::Unchanged::publish,
        );
    out.record(|records| records.facts = Some(before.cost + after.cost));
    let renamed = changed::renamed(&project.changes(commit)?);
    Ok((
        side(prior.tree(), &before, &renamed)?,
        side(project.tree(), &after, &HashMap::new())?,
    ))
}

fn side(
    tree: &Tree,
    measured: &structural::Measurement,
    renamed: &HashMap<String, String>,
) -> Result<Side, Error> {
    let layout = Topology::new(tree.root(), tree.files()?, measured.facts(), renamed);
    let graph = modules::build(&layout);
    let derived = surface::derive(&layout, &graph);
    Ok(Side {
        derived,
        graph,
        unparsed: measured.unparsed.clone(),
        current: renamed
            .iter()
            .map(|(now, was)| (was.clone(), now.clone()))
            .collect(),
    })
}

pub fn language_extensions() -> Vec<(&'static str, String)> {
    structural::language_extensions()
}

/// Every break the working tree makes against the base: a base surface the working tree lacks,
/// once at the surface; and for every item of a surface both hold, an item gone, a measured
/// contract changed or no longer declared, or an opaque clause changed. An addition is never a
/// break, and an opaque item that became measured is not one either.
fn breaks(was: &Derived, now: &Derived) -> Vec<Finding> {
    let mut out = Vec::new();
    for surface in &was.surfaces {
        let Some(after) = now.surfaces.iter().find(|held| held.id == surface.id) else {
            out.push(finding(
                surface,
                None,
                SURFACE_TEXT,
                REMOVED_SURFACE,
                None,
                None,
            ));
            continue;
        };
        for item in &surface.items {
            let text = format!("{} ({})", item.path, item.kind);
            match after.item(&item.path, item.kind) {
                None => out.push(finding(surface, Some(item), &text, REMOVED, None, None)),
                Some(current) => {
                    if let Some((was, now)) = changed(&item.contract, &current.contract) {
                        out.push(finding(
                            surface,
                            Some(current),
                            &text,
                            CHANGED,
                            Some(was),
                            Some(now),
                        ));
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| (&a.file, &a.text).cmp(&(&b.file, &b.text)));
    out
}

/// The two contracts of one item where the working tree's breaks the base's, in the words a
/// failure prints, and `None` where it does not.
fn changed(was: &Contract, now: &Contract) -> Option<(String, String)> {
    match (was, now) {
        (Contract::Measured(before), Contract::Measured(after)) if before != after => {
            Some((before.clone(), after.clone()))
        }
        (Contract::Measured(before), Contract::Opaque(_)) => {
            Some((before.clone(), "no declared contract".to_string()))
        }
        (Contract::Opaque(Some(before)), Contract::Opaque(Some(after))) if before != after => {
            Some((before.clone(), after.clone()))
        }
        _ => None,
    }
}

fn finding(
    surface: &Surface,
    item: Option<&Item>,
    text: &str,
    kind: &str,
    was: Option<String>,
    now: Option<String>,
) -> Finding {
    let mut values = Values::new();
    values.insert(BREAK.into(), 1.into());
    values.insert(KIND.into(), kind.into());
    if let Some((file, line)) = item.and_then(|item| item.origin.as_ref()) {
        values.insert(ORIGIN.into(), format!("{file}:{line}").into());
    }
    if let (Some(was), Some(now)) = (was, now) {
        values.insert(WAS.into(), was.into());
        values.insert(NOW.into(), now.into());
    }
    Finding {
        file: surface.id.clone(),
        line: item
            .and_then(|item| item.origin.as_ref())
            .map_or(0, |(_, line)| *line),
        text: text.to_string(),
        values,
        body: None,
    }
}

fn judged(at: &Context, now: &Side, findings: Vec<Finding>, out: &mut Sink) -> Result<u8, Error> {
    let cost = now.derived.cost();
    let coverage = Coverage {
        found: cost.surfaces + now.derived.inapplicable.len(),
        measured: cost.surfaces,
        not_measured: now.derived.inapplicable.len(),
        excluded: 0,
        unreadable: 0,
    };
    let said = coverage.said(out);
    let state = format!(
        "{} external item(s) on {} surface(s) judged against the base, {} measured, {} opaque, no removal or contract change",
        cost.items, cost.surfaces, cost.measured, cost.opaque
    );
    let tail = format!("{said}; {}", discovered(&now.derived));
    let evaluator = evaluator(at.hook());
    Ok(evaluator.evaluate(
        findings,
        Vec::new(),
        ratchet::accepted(&at.project.config, at.gate, evaluator.metrics)?,
        at,
        Line {
            state: &state,
            tail: &tail,
        },
        out,
    ))
}

/// Where the working tree's surfaces came from, and what klin found and derived nothing from.
fn discovered(derived: &Derived) -> String {
    let rust = derived
        .surfaces
        .iter()
        .filter(|surface| surface.language == "Rust")
        .count();
    let typescript = derived.surfaces.len() - rust;
    format!(
        "{rust} Rust library target(s), {typescript} TypeScript entry point(s), {} package(s) or target(s) with no supported surface",
        derived.inapplicable.len()
    )
}

fn evaluator(hook: bool) -> Evaluator<'static> {
    Evaluator {
        metrics: &[BREAK],
        unit: "compatibility break(s)",
        condition: "where an external surface or item the base exposed is gone or its declared contract changed",
        fix_advice: Remedy::Fixed(if hook { HOOK_REMEDY } else { REMEDY }),
        ceiling: None,
        format_metrics: show,
        nested: Some(held_by_removed_modules),
    }
}

/// For each finding, the removed module it prints under: the outermost module under the same
/// surface id that the same change removed and whose path holds the finding's own path. Two
/// surfaces that share an id share every identity, so their modules group together.
fn held_by_removed_modules(found: &[Finding]) -> Vec<Option<usize>> {
    let removed =
        |finding: &Finding| finding.values.get(KIND).and_then(Value::as_str) == Some(REMOVED);
    let mut modules: HashMap<(&str, &str), usize> = HashMap::new();
    for (at, finding) in found.iter().enumerate().filter(|(_, held)| removed(held)) {
        let module = finding
            .text
            .strip_suffix(')')
            .and_then(|text| text.strip_suffix(MODULE))
            .and_then(|text| text.strip_suffix(" ("));
        if let Some(module) = module {
            modules.entry((finding.file.as_str(), module)).or_insert(at);
        }
    }
    found
        .iter()
        .map(|finding| {
            let (own, _) = finding
                .text
                .rsplit_once(" (")
                .filter(|_| removed(finding))?;
            own.match_indices("::")
                .find_map(|(end, _)| modules.get(&(finding.file.as_str(), &own[..end])).copied())
        })
        .collect()
}

fn show(values: &Values) -> String {
    let text = |key: &str| values.get(key).and_then(Value::as_str).unwrap_or("");
    let origin = match text(ORIGIN) {
        "" => String::new(),
        origin => format!(", declared at {origin}"),
    };
    match text(KIND) {
        CHANGED => format!("changed{origin}, was `{}`, now `{}`", text(WAS), text(NOW)),
        REMOVED_SURFACE => "the whole surface is gone".to_string(),
        REMOVED => format!("removed{origin}"),
        _ => BREAK.to_string(),
    }
}

/// The forms klin recognizes inside a supported surface and could not resolve, the module
/// resolution holes of #50 inside one, and the surfaces whose entry klin could not measure,
/// beside the ones the base held too, because a green run must not imply a surface was completely
/// measured. ADR 0021, spec 8.6.
fn holes_said((was, now): (&Side, &Side), at: &Context, code: u8, out: &mut Sink) -> u8 {
    let named = holes_of(now);
    coverage::unresolved_said(
        (&named, &holes_of(was)),
        (
            "form(s) inside a supported public surface could not be resolved, so the surface is not completely measured",
            "Write the export or re-export in a form klin lists, or make each path name exactly one module file the tree holds.",
        ),
        (at, code),
        out,
    )
}

/// Every hole inside a surface under today's paths, each once, in one order: the surface's own
/// holes and the module graph's holes in the files the surface reaches.
fn holes_of(side: &Side) -> Vec<coverage::Unresolved> {
    let mut named: Vec<coverage::Unresolved> = Vec::new();
    for surface in &side.derived.surfaces {
        let inside = side
            .graph
            .holes
            .iter()
            .filter(|hole| surface.files.binary_search(&hole.file).is_ok())
            .map(|hole| (&hole.file, hole.line, &hole.text, &hole.why));
        let own = surface
            .holes
            .iter()
            .map(|hole| (&hole.file, hole.line, &hole.text, &hole.why));
        for (file, line, text, why) in own.chain(inside) {
            named.push(coverage::Unresolved {
                file: side.current.get(file).unwrap_or(file).clone(),
                line,
                text: text.clone(),
                why: format!("{} — {}", surface.id, why),
            });
        }
    }
    named.sort();
    named.dedup();
    named
}

/// The packages and targets klin found and derived no surface from, said once, because a run
/// over them is not a compatibility success.
fn inapplicable_note(derived: &Derived, out: &mut Sink) {
    if derived.inapplicable.is_empty() {
        return;
    }
    let mut note = format!(
        "{} package(s) or target(s) with no supported public surface:",
        derived.inapplicable.len()
    );
    for held in derived.inapplicable.iter().take(20) {
        let _ = write!(note, "\n  {}: {}", held.what, held.why);
    }
    if derived.inapplicable.len() > 20 {
        let _ = write!(note, "\n  … and {} more", derived.inapplicable.len() - 20);
    }
    ratchet::noted(&[(String::new(), note)], out);
}

/// The derived contract of the working tree, item by item, so automatic derivation is
/// inspectable. Nothing is judged and no base is read.
fn report(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    at.config().policy(SECTION, KEYS)?;
    let tree = at.project.tree();
    let measured = structural::measure_all(tree, None)?;
    let layout = Topology::new(
        tree.root(),
        tree.files()?,
        measured.facts(),
        &HashMap::new(),
    );
    let graph = modules::build(&layout);
    let derived = surface::derive(&layout, &graph);
    let cost = derived.cost();
    let _ = writeln!(
        out.text,
        "REPORT: {} surface(s), {} item(s): {} measured, {} opaque, {} hole(s), {} package(s) or target(s) with no supported surface",
        cost.surfaces,
        cost.items,
        cost.measured,
        cost.opaque,
        cost.holes,
        derived.inapplicable.len()
    );
    for surface in &derived.surfaces {
        report_surface(surface, out.text);
    }
    for held in &derived.inapplicable {
        let _ = writeln!(out.text, "not applicable: {}: {}", held.what, held.why);
    }
    Ok(0)
}

fn report_surface(surface: &Surface, out: &mut String) {
    let _ = writeln!(
        out,
        "surface {}  {} — {}",
        surface.id, surface.language, surface.source
    );
    for item in &surface.items {
        let _ = writeln!(out, "  {}", report_item(surface, item));
    }
    for hole in &surface.holes {
        let _ = writeln!(
            out,
            "  hole {}:{}  {}  — {}",
            hole.file, hole.line, hole.text, hole.why
        );
    }
}

/// One item's report line: its identity, kind, measured or opaque status, origin, and its
/// signature on the line below where it is measured.
fn report_item(surface: &Surface, item: &Item) -> String {
    let origin = item
        .origin
        .as_ref()
        .map_or(String::new(), |(file, line)| format!("  {file}:{line}"));
    match &item.contract {
        Contract::Measured(signature) => format!(
            "{}  {}  measured{origin}\n      {signature}",
            identity(surface, item),
            item.kind
        ),
        Contract::Opaque(clause) => {
            let clause = clause
                .as_ref()
                .map_or(String::new(), |clause| format!(" ({clause})"));
            format!(
                "{}  {}  opaque{clause}{origin}",
                identity(surface, item),
                item.kind
            )
        }
    }
}

/// What a consumer writes for one item: the crate path for Rust, the package, subpath and name
/// for TypeScript.
fn identity(surface: &Surface, item: &Item) -> String {
    match surface.language {
        "Rust" => format!("{}::{}", surface.id, item.path),
        _ => format!("{} {}", surface.id, item.path),
    }
}
