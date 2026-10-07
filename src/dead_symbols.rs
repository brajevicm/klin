//! The `dead-symbols` check: private Rust and TypeScript declarations with no reference outside
//! their own declaration. Identity is file plus declaration line/text, and the `dead` metric
//! ratchets from 0 (referenced) to 1 (unreferenced). The check discovers every structural
//! language and accepts only `in`, `except` and name `ignore` as policy. The structural index
//! owns parsing, declaration kinds and references; this module chooses eligibility and ratchets
//! the result. ADR 0035, spec 8.4.

use std::collections::BTreeSet;
use std::fmt::Write;

use serde_json::Value;

use crate::base::{self, Prior};
use crate::check::contract::{self, Context, Line, Listed, Sink, Site};
use crate::check::holes;
use crate::config::Config;
use crate::coverage;
use crate::error::Error;
use crate::files;
use crate::key::Key;
use crate::measurement;
use crate::project::Project;
use crate::ratchet::{self, Evaluator, Finding, Remedy};
use crate::record::Values;
use crate::scope::{self, Scope};
use crate::syntax::{self, structural};
use crate::tree::Tree;

pub const SECTION: &str = "dead_symbols";

const IGNORE_NAME: &str = "ignore";
const DEAD: &str = "dead";
const LOST_REFERENCE: &str = "lost_reference";
const REMEDY: &str =
    "Delete the declaration if the refactor made it obsolete, or restore a real reference to it.";

pub const IGNORE: Key = Key {
    name: IGNORE_NAME,
    holds: "name globs for declarations the check leaves out",
    required: false,
    rule: None,
    default: "Rust `main`, test functions and declarations marked externally visible",
    shape: crate::key::Shape::Strings,
};

pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT, IGNORE];

#[derive(Clone)]
struct Selection {
    extensions: Vec<&'static str>,
    scope: Scope,
}

struct Spec {
    selection: Selection,
    ignore: Vec<String>,
}

struct State {
    file: String,
    name: String,
    line: u64,
    end: u64,
    text: String,
    dead: bool,
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    evaluate(at, false, out)
}

fn evaluate(at: &Context, report: bool, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let spec = spec(project)?;
    let commit = contract::base_commit(project.root(), at)?;
    let mut names = structural::NameCost::default();
    let mut layout = None;
    let (before, after) = sweeps(at, &spec, &commit, &mut names, &mut layout)?;
    let affected = affected_scope(at, &before, &after, &mut names);
    let widened = at.scoped(affected.as_deref().or(at.only));
    let at = &widened;
    let judged_scope = at.only.filter(|_| at.changes.is_some() && !at.strict);
    let before_states = judgement(&before, &mut names.before, &spec.ignore, judged_scope);
    let after_states = judgement(&after, &mut names.after, &spec.ignore, judged_scope);
    let built = (before_states.len() + after_states.len()) as u64;
    let held_before = held(&before_states, project);
    let prior = held_before.iter().map(|state| finding(state)).collect();
    let now = structural::timed(names.lost.get_or_insert_default(), || {
        dead_findings(&after_states, &before, &after, &held_before)
    });
    out.record(|records| {
        records.facts = Some(before.cost + after.cost);
        records.states = Some(built);
        records.names = Some(names);
        records.layout = layout;
        records.footprint = Some(structural::footprint::of([before.facts(), after.facts()]));
    });
    let judged = after_states
        .iter()
        .filter(|state| coverage::in_scope(&state.file, at.only))
        .count();
    let dead = now.len();
    let said = out.covered(&after.files.coverage(at.only));
    let evaluator = evaluator();
    let code = evaluator.evaluate(
        now,
        prior,
        ratchet::accepted(&project.config, at.gate, evaluator.metrics)?,
        at,
        Line {
            state: format!("{judged} declaration(s) judged, {dead} dead symbol(s)"),
            coverage: said,
            ..Line::default()
        },
        out,
    );
    let prior = contract::whole_base(at, &commit)?;
    let unread_at_base = || prior.unread_either(&before.files.unreadable);
    let code = coverage_result(code, at, (&before, unread_at_base), &after, out);
    reports(report, &after_states, &held_before, at.only, out);
    Ok(code)
}

/// The two trees measured. A changed run that is not strict measures them over one base
/// extraction: the working tree takes the base's facts for every file its `Change` set leaves
/// out, and extracts only the files it changed. Strict and whole runs extract both trees.
fn sweeps(
    at: &Context,
    spec: &Spec,
    commit: &str,
    names: &mut structural::NameCost,
    layout: &mut Option<base::Layout>,
) -> Result<(measurement::Measurement, measurement::Measurement), Error> {
    let prior = structural::timed(&mut names.base, || contract::whole_base(at, commit))?;
    let unchanged = structural::timed(&mut names.base, || {
        contract::unchanged_base(at, prior, commit)
    })?;
    *layout = prior.layout();
    let mut after = structural::timed(&mut names.after.measure, || {
        measure(at.project.tree(), &spec.selection, unchanged.as_ref())
    })?;
    let before = structural::timed(&mut names.before.measure, || before(at, spec, prior))?;
    after.cost = after.cost
        + unchanged.map_or_else(
            structural::ExtractionCost::default,
            measurement::Unchanged::publish,
        );
    Ok((before, after))
}

/// The effective judgement scope of a changed, non-strict run: the physical scope the runner
/// gave, plus every file declaring a name whose reference evidence this turn changed. A
/// declaration that did not move can still change from referenced to dead when its last caller
/// changed, so the physical scope alone is not the semantic impact scope. A name a changed file
/// references on both sides cannot flip one, so only the names one side holds alone widen
/// anything: no type, import or receiver resolution enters here, and a name with several
/// declarations widens to all of them, which fails less. Issue #237, spec 8.4.
fn affected_scope(
    at: &Context,
    before: &measurement::Measurement,
    after: &measurement::Measurement,
    names: &mut structural::NameCost,
) -> Option<Vec<String>> {
    let only = at.only.filter(|_| at.changes.is_some() && !at.strict)?;
    structural::timed(&mut names.before.index, || before.index());
    structural::timed(&mut names.after.index, || after.index());
    let mut affected = BTreeSet::new();
    for change in at.changes? {
        if !measured_after(after, &change.path) {
            continue;
        }
        let now = reference_names(after.index().file(&change.path));
        let was = change
            .was
            .as_ref()
            .map(|was| reference_names(before.index().file(was)))
            .unwrap_or_default();
        affected.extend(now.symmetric_difference(&was).cloned());
    }
    let mut scope: BTreeSet<String> = only.iter().cloned().collect();
    scope.extend(declaring_files(&affected, before, after));
    Some(scope.into_iter().collect())
}

/// Every file that declares one of these names in either tree.
fn declaring_files(
    names: &BTreeSet<(syntax::LanguageId, structural::facts::Name)>,
    before: &measurement::Measurement,
    after: &measurement::Measurement,
) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    for (language, name) in names {
        for index in [before.index(), after.index()] {
            for declared in index.declarations(*language, name.as_str()) {
                files.insert(declared.file.to_string());
            }
        }
    }
    files
}

fn reference_names(
    file: Option<&structural::facts::FileFacts>,
) -> BTreeSet<(syntax::LanguageId, structural::facts::Name)> {
    let Some(file) = file else {
        return BTreeSet::new();
    };
    file.references
        .iter()
        .map(|reference| (file.language, reference.name.clone()))
        .collect()
}

/// Whether the working tree's structural evidence for this path is a measurement. A changed
/// file the analyzer could not read is a coverage hole the run already reports, and its old
/// reference names are not proof that the references went away, so nothing widens from it.
fn measured_after(after: &measurement::Measurement, path: &str) -> bool {
    !after.unparsed.iter().any(|held| held.file == path)
        && !after.unsupported.iter().any(|held| held.file == path)
}

fn judgement(
    measured: &measurement::Measurement,
    cost: &mut structural::TreeNameCost,
    ignore: &[String],
    only: Option<&[String]>,
) -> Vec<State> {
    let index = measured.indexed(cost);
    structural::timed(&mut cost.query, || states(index, ignore, only))
}

fn before(at: &Context, spec: &Spec, prior: &Prior) -> Result<measurement::Measurement, Error> {
    let selection = Selection {
        scope: Scope::at_base(
            &at.project.config,
            SECTION,
            prior.root(),
            &spec.selection.scope,
        ),
        ..spec.selection.clone()
    };
    measure(prior.tree(), &selection, None)
}

fn held<'a>(states: &'a [State], project: &Project) -> Vec<&'a State> {
    states
        .iter()
        .filter(|state| project.was_held(&state.file))
        .collect()
}

fn dead_findings(
    states: &[State],
    before: &measurement::Measurement,
    after: &measurement::Measurement,
    held_before: &[&State],
) -> Vec<Finding> {
    states
        .iter()
        .filter(|state| state.dead)
        .map(|state| finding_with_lost_reference(state, before, after, held_before))
        .collect()
}

fn coverage_result(
    code: u8,
    at: &Context,
    (before, unread_at_base): (&measurement::Measurement, impl FnOnce() -> Vec<String>),
    after: &measurement::Measurement,
    out: &mut Sink,
) -> u8 {
    let code = holes::not_measured_said(&after.unsupported, at, code, out);
    let code = holes::lost_said(
        &after.files.lost(&before.files, at.project, at.only),
        at,
        code,
        out,
    );
    holes::unread_said(&after.unparsed, unread_at_base, at, code, out)
}

fn reports(
    report: bool,
    after_states: &[State],
    held_before: &[&State],
    only: Option<&[String]>,
    out: &mut Sink,
) {
    if report {
        report_dead(after_states, only, out);
    }
    base_note(held_before, only, out);
}

fn spec(project: &Project) -> Result<Spec, Error> {
    let config = &project.config;
    let values = config.policy(SECTION, KEYS)?;
    let selection = selection(config, &values)?;
    if selection.scope.has_in() && !applicable(project.tree(), &selection)? {
        return Err(Error(format!(
            "{}: \"{SECTION}\" has an \"in\" scope with no applicable file",
            config.file.display()
        )));
    }
    Ok(Spec {
        selection,
        ignore: files::strings(config, SECTION, &values, IGNORE)?,
    })
}

fn selection(config: &Config, values: &Values) -> Result<Selection, Error> {
    Ok(Selection {
        extensions: structural::selected_extensions(&[]).unwrap_or_default(),
        scope: Scope::read(config, SECTION, values)?,
    })
}

pub fn language_extensions() -> Vec<(&'static str, String)> {
    structural::language_extensions()
}

fn measure(
    tree: &Tree,
    selection: &Selection,
    unchanged: Option<&measurement::Unchanged>,
) -> Result<measurement::Measurement, Error> {
    let repo_root = tree.root();
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &selection.extensions,
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let mut found = files::found(
        tree.root(),
        || tree.files(),
        &[repo_root.to_path_buf()],
        &wanted,
    )?;
    let mut excluded = Vec::new();
    found.kept.retain(|file| {
        let keep = selection.scope.selects(&files::relative(file, repo_root));
        if !keep {
            excluded.push(file.clone());
        }
        keep
    });
    found.excluded.extend(excluded);
    measurement::measure(found, tree, unchanged)
}

fn applicable(tree: &Tree, selection: &Selection) -> Result<bool, Error> {
    Ok(tree.files()?.iter().any(|file| {
        selection.scope.inside(file) && selection.extensions.iter().any(|end| file.ends_with(end))
    }))
}

/// The declaration state of one tree, built only for the files the run judges. The index stays
/// complete over both trees, so a declaration in scope is judged against every reference the
/// repository holds, and only the states nothing can report are left unbuilt. Spec 8.4.
fn states(
    index: &structural::SourceIndex,
    ignore: &[String],
    only: Option<&[String]>,
) -> Vec<State> {
    let mut out = Vec::new();
    for file in index.files() {
        if !coverage::in_scope(&file.file, only) {
            continue;
        }
        for declaration in &file.declarations {
            if !eligible(declaration, ignore) {
                continue;
            }
            out.push(state(index, file, declaration));
        }
    }
    out.sort_by(|a, b| (&a.file, a.line, &a.name).cmp(&(&b.file, b.line, &b.name)));
    out
}

fn eligible(declaration: &structural::facts::Declaration, ignore: &[String]) -> bool {
    !declaration.externally_visible
        && !declaration.entry_point
        && !declaration.names().all(|name| {
            ignore
                .iter()
                .any(|glob| files::glob_matches(glob.as_bytes(), name.as_bytes()))
        })
}

fn state(
    index: &structural::SourceIndex,
    file: &structural::facts::FileFacts,
    declaration: &structural::facts::Declaration,
) -> State {
    let dead = !declaration.names().any(|name| {
        index.references(file.language, name).any(|reference| {
            reference.file != file.file
                || reference.line < declaration.line
                || reference.line > declaration.end
        })
    });
    State {
        file: file.file.clone(),
        name: declaration.name.clone(),
        line: declaration.line,
        end: declaration.end,
        text: declaration.text.clone(),
        dead,
    }
}

fn finding(state: &State) -> Finding {
    let mut values = Values::new();
    values.insert(DEAD.into(), u64::from(state.dead).into());
    Finding {
        file: state.file.clone(),
        line: state.line,
        text: state.text.clone(),
        values,
        body: None,
    }
}

fn finding_with_lost_reference(
    state: &State,
    before: &measurement::Measurement,
    after: &measurement::Measurement,
    before_states: &[&State],
) -> Finding {
    let mut finding = finding(state);
    if let Some(file) = lost_reference(state, before, after, before_states) {
        finding.values.insert(LOST_REFERENCE.into(), file.into());
    }
    finding
}

/// The first base state at this site. The states are in file, line and name order, so the
/// site is found by halving them.
fn held_at<'a>(states: &[&'a State], state: &State) -> Option<&'a State> {
    let from = states.partition_point(|held| site(held) < site(state));
    states[from..]
        .iter()
        .take_while(|held| site(held) == site(state))
        .find(|held| held.text == state.text)
        .copied()
}

fn site(state: &State) -> (&str, u64, &str) {
    (&state.file, state.line, &state.name)
}

fn lost_reference(
    state: &State,
    before: &measurement::Measurement,
    after: &measurement::Measurement,
    before_states: &[&State],
) -> Option<String> {
    let held = held_at(before_states, state)?;
    if held.dead {
        return None;
    }
    let file = before.index().file(&state.file)?;
    let declaration = file.declarations.iter().find(|declaration| {
        (declaration.line, &declaration.name, &declaration.text)
            == (held.line, &held.name, &held.text)
    })?;
    declaration
        .names()
        .filter_map(|name| {
            let now: BTreeSet<(&str, u64)> = after
                .index()
                .references(file.language, name)
                .map(|reference| (reference.file, reference.line))
                .collect();
            before
                .index()
                .references(file.language, name)
                .filter(|reference| {
                    reference.file != held.file
                        || reference.line < held.line
                        || reference.line > held.end
                })
                .find(|reference| !now.contains(&(reference.file, reference.line)))
                .map(|reference| reference.file)
        })
        .min()
        .map(str::to_string)
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[DEAD],
        unit: "dead symbol(s)",
        condition: "where no reference named the declaration exists outside its own declaration",
        fix_advice: Remedy::Fixed(REMEDY),
        ceiling: None,
        format_metrics: show,
        nested: None,
    }
}

fn show(values: &Values) -> String {
    let state = match values.get(DEAD).and_then(Value::as_u64) {
        Some(0) => "referenced",
        Some(1) => "dead",
        _ => "unknown",
    };
    match values.get(LOST_REFERENCE).and_then(Value::as_str) {
        Some(file) => format!("{state}, lost reference in {file}"),
        None => state.to_string(),
    }
}

fn report_dead(states: &[State], only: Option<&[String]>, out: &mut Sink) {
    let dead: Vec<&State> = states
        .iter()
        .filter(|state| state.dead && coverage::in_scope(&state.file, only))
        .collect();
    out.tell(Listed::DeadSymbols(
        dead.into_iter()
            .map(|state| {
                let site = Site {
                    file: state.file.clone(),
                    line: Some(state.line),
                    text: state.text.clone(),
                };
                (site, state.name.clone())
            })
            .collect(),
    ));
}

fn base_note(states: &[&State], only: Option<&[String]>, out: &mut Sink) {
    let dead: Vec<&State> = states
        .iter()
        .filter(|state| state.dead && coverage::in_scope(&state.file, only))
        .copied()
        .collect();
    if dead.is_empty() {
        return;
    }
    let mut note = format!("{} dead symbol(s) the base already held:", dead.len());
    for state in dead.iter().take(20) {
        let _ = write!(note, "\n  {}:{}  {}", state.file, state.line, state.text);
    }
    if dead.len() > 20 {
        let _ = write!(note, "\n  … and {} more", dead.len() - 20);
    }
    ratchet::noted(&[(String::new(), note)], out);
}
