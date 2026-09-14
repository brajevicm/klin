//! The `dead-symbols` check: private Rust and TypeScript declarations with no reference outside
//! their own declaration. Identity is file plus declaration line/text, and the `dead` metric
//! ratchets from 0 (referenced) to 1 (unreferenced). The check discovers every structural
//! language and accepts only `in`, `except` and name `ignore` as policy. The structural index
//! owns parsing, declaration kinds and references; this module chooses eligibility and ratchets
//! the result. ADR 0035, spec 8.4.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::base;
use crate::check::{Context, Sink};
use crate::config::{Config, Error};
use crate::coverage;
use crate::files;
use crate::project::{Project, Tree};
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::Key;
use crate::scope::{self, Scope};
use crate::syntax::{self, structural};

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
};

pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT, IGNORE];

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
    /// Print the complete current list of dead symbols
    #[arg(long)]
    report: bool,
    /// Judge only these repo-relative files, against only their sites at the base
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
}

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

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let project = Project::load(args.config.as_deref(), start)?;
    evaluate(
        &context(args, &project),
        args.report,
        &mut Sink::unrecorded(out),
    )
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    evaluate(at, false, out)
}

fn context<'a>(args: &'a Args, project: &'a Project) -> Context<'a> {
    Context {
        only: args.only.as_deref(),
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand("dead-symbols", project)
    }
}

fn evaluate(at: &Context, report: bool, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let spec = spec(project)?;
    let commit = base::commit(project.root(), at, out)?;
    let (before, after) = sweeps(at, &spec, &commit)?;
    let before_states = states(&before.index, &spec.ignore);
    let after_states = states(&after.index, &spec.ignore);
    let held_before = held(&before_states, project);
    let prior = held_before.iter().map(|state| finding(state)).collect();
    let now = dead_findings(&after_states, &before, &after, &held_before);
    let judged = after_states
        .iter()
        .filter(|state| coverage::in_scope(&state.file, at.only))
        .count();
    let dead = now.len();
    let said = after.files.coverage(at.only).said(out);
    let evaluator = evaluator();
    let code = evaluator.evaluate(
        now,
        prior,
        ratchet::accepted(&project.config, at.gate, evaluator.metrics)?,
        at,
        &format!(
            "OK: {judged} declaration(s) judged, {dead} dead symbol(s), all held at the base{said}"
        ),
        out,
    );
    let code = coverage_result(code, at, &before, &after, out);
    reports(report, &after_states, &held_before, at.only, out);
    Ok(code)
}

fn sweeps(
    at: &Context,
    spec: &Spec,
    commit: &str,
) -> Result<(structural::Measurement, structural::Measurement), Error> {
    let project = at.project;
    let after = measure(project.tree(), &spec.selection, project.root())?;
    let before = before(at, spec, commit)?;
    Ok((before, after))
}

fn before(at: &Context, spec: &Spec, commit: &str) -> Result<structural::Measurement, Error> {
    let owned = base_tree(at, commit)?;
    let prior = match owned.as_ref() {
        Some(prior) => prior,
        None => at
            .prior
            .ok_or_else(|| Error("a runner gives structural checks a base tree".into()))?,
    };
    let selection = Selection {
        scope: Scope::at_base(
            &at.project.config,
            SECTION,
            prior.root(),
            &spec.selection.scope,
        ),
        ..spec.selection.clone()
    };
    measure(prior.tree(), &selection, prior.root())
}

fn base_tree(at: &Context, commit: &str) -> Result<Option<base::Prior>, Error> {
    let owned = (at.prior.is_none() || at.only.is_some())
        .then(|| base::materialize(at.project, commit, None))
        .transpose()?;
    Ok(owned)
}

fn held<'a>(states: &'a [State], project: &Project) -> Vec<&'a State> {
    states
        .iter()
        .filter(|state| project.was_held(&state.file))
        .collect()
}

fn dead_findings(
    states: &[State],
    before: &structural::Measurement,
    after: &structural::Measurement,
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
    before: &structural::Measurement,
    after: &structural::Measurement,
    out: &mut Sink,
) -> u8 {
    let code = coverage::not_measured_said(&after.unsupported, at, code, out);
    let code = coverage::lost_said(
        &after.files.lost(&before.files, at.project, at.only),
        at,
        code,
        out,
    );
    syntax::unread(&after.unparsed, at, code, out)
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
    repo_root: &Path,
) -> Result<structural::Measurement, Error> {
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &selection.extensions,
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let mut found = files::found(tree, &[repo_root.to_path_buf()], &wanted)?;
    let mut excluded = Vec::new();
    found.kept.retain(|file| {
        let keep = selection.scope.selects(&files::relative(file, repo_root));
        if !keep {
            excluded.push(file.clone());
        }
        keep
    });
    found.excluded.extend(excluded);
    structural::measure(found, repo_root)
}

fn applicable(tree: &Tree, selection: &Selection) -> Result<bool, Error> {
    Ok(tree.files()?.iter().any(|file| {
        selection.scope.inside(file) && selection.extensions.iter().any(|end| file.ends_with(end))
    }))
}

fn states(index: &structural::SourceIndex, ignore: &[String]) -> Vec<State> {
    let mut out = Vec::new();
    for file in index.files() {
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

fn eligible(declaration: &structural::Declaration, ignore: &[String]) -> bool {
    !declaration.externally_visible
        && !declaration.entry_point
        && !ignore
            .iter()
            .any(|glob| files::glob_matches(glob.as_bytes(), declaration.name.as_bytes()))
}

fn state(
    index: &structural::SourceIndex,
    file: &structural::FileFacts,
    declaration: &structural::Declaration,
) -> State {
    let dead = !index
        .references(file.language, &declaration.name)
        .any(|reference| {
            reference.file != file.file
                || reference.line < declaration.line
                || reference.line > declaration.end
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
    before: &structural::Measurement,
    after: &structural::Measurement,
    before_states: &[&State],
) -> Finding {
    let mut finding = finding(state);
    if let Some(file) = lost_reference(state, before, after, before_states) {
        finding.values.insert(LOST_REFERENCE.into(), file.into());
    }
    finding
}

fn same_site(held: &State, state: &State) -> bool {
    held.file == state.file
        && held.line == state.line
        && held.name == state.name
        && held.text == state.text
}

fn lost_reference(
    state: &State,
    before: &structural::Measurement,
    after: &structural::Measurement,
    before_states: &[&State],
) -> Option<String> {
    let held = before_states
        .iter()
        .find(|candidate| same_site(candidate, state))?;
    if held.dead {
        return None;
    }
    let language = before
        .index
        .files()
        .iter()
        .find(|file| file.file == state.file)?
        .language;
    let old = before.index.references(language, &state.name);
    let now: BTreeSet<(&str, u64)> = after
        .index
        .references(language, &state.name)
        .map(|reference| (reference.file, reference.line))
        .collect();
    old.filter(|reference| {
        reference.file != held.file || reference.line < held.line || reference.line > held.end
    })
    .filter(|reference| !now.contains(&(reference.file, reference.line)))
    .map(|reference| reference.file.to_string())
    .next()
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[DEAD],
        unit: "dead symbol(s)",
        condition: "where no reference named the declaration exists outside its own declaration",
        fix_advice: REMEDY,
        ceiling: None,
        format_metrics: show,
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
    let _ = writeln!(out.text, "REPORT: {} dead symbol(s):", dead.len());
    for state in dead {
        let _ = writeln!(
            out.text,
            "  {}:{}  {}  {}",
            state.file, state.line, state.name, state.text
        );
    }
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
