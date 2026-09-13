//! The `dead-symbols` check: private Rust and TypeScript declarations with no reference outside
//! their own declaration. Identity is file plus declaration line/text, and the `dead` metric
//! ratchets from 0 (referenced) to 1 (unreferenced). Roots and structural languages derive from
//! the survey; `exclude`, `skip_dirs` and `ignore` are pinned policy. The structural index
//! owns parsing, declaration kinds and references; this module only chooses eligibility and
//! ratchets the result. ADR 0035, spec 8.4.

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base;
use crate::check::{self, Context, Sink};
use crate::config::{Config, Error};
use crate::coverage;
use crate::files;
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::{self, Key};
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

pub const KEYS: &[Key] = &[
    reference::ROOTS.required(),
    reference::LANGUAGES.defaulting("the structural languages this check supports"),
    reference::EXCLUDE,
    reference::SKIP_DIRS,
    IGNORE,
];

pub const DERIVED: &[&str] = &[reference::ROOTS.name, reference::LANGUAGES.name];

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
    skip_dirs: Vec<String>,
    exclude: Vec<String>,
}

struct Spec {
    roots: Vec<PathBuf>,
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
    evaluate(
        &context(args, start),
        args.report,
        &mut Sink::unrecorded(out),
    )
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    evaluate(at, false, out)
}

fn context<'a>(args: &'a Args, start: &'a Path) -> Context<'a> {
    Context {
        only: args.only.as_deref(),
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand("dead-symbols", start, args.config.as_deref())
    }
}

fn evaluate(at: &Context, report: bool, out: &mut Sink) -> Result<u8, Error> {
    let config = Config::load_with(at.config, at.start, at.with)?;
    let spec = spec(&config)?;
    at.say(&config, SECTION, out);
    let commit = base::commit(config.root(), at, out)?;
    let (before, after) = sweeps(at, &config, &spec, &commit)?;
    let before_states = states(&before.index, &spec.ignore);
    let after_states = states(&after.index, &spec.ignore);
    let held_before = held(&before_states, &config);
    let prior = held_before.iter().map(|state| finding(state)).collect();
    let now = dead_findings(&after_states, &before, &after, &held_before);
    let judged = after_states
        .iter()
        .filter(|state| in_scope(&state.file, at.only))
        .count();
    let dead = now.len();
    let said = after.files.coverage(at.only).said(out);
    let evaluator = evaluator();
    let code = evaluator.evaluate(
        now,
        prior,
        ratchet::accepted(&config, at.gate, evaluator.metrics)?,
        at,
        &format!(
            "OK: {judged} declaration(s) judged, {dead} dead symbol(s), all held at the base{said}"
        ),
        out,
    );
    let code = coverage_result(code, at, &config, &before, &after, out);
    reports(report, &after_states, &held_before, at.only, out);
    Ok(code)
}

fn sweeps(
    at: &Context,
    config: &Config,
    spec: &Spec,
    commit: &str,
) -> Result<(structural::Measurement, structural::Measurement), Error> {
    let after = measure(&spec.roots, &spec.selection, config.root())?;
    let before = before(at, config, spec, commit)?;
    Ok((before, after))
}

fn before(
    at: &Context,
    config: &Config,
    spec: &Spec,
    commit: &str,
) -> Result<structural::Measurement, Error> {
    let owned = base_tree(config, at, commit)?;
    let prior_root = match owned.as_ref() {
        Some(prior) => prior.root(),
        None => at
            .prior
            .ok_or_else(|| Error("a runner gives structural checks a base tree".into()))?,
    };
    let before_roots = base::roots(&spec.roots, config, prior_root)?;
    let before_selection = Selection {
        exclude: files::base_exclusions(config, SECTION, prior_root, &spec.selection.exclude),
        ..spec.selection.clone()
    };
    measure(&before_roots, &before_selection, prior_root)
}

fn base_tree(config: &Config, at: &Context, commit: &str) -> Result<Option<base::Prior>, Error> {
    let owned = (at.prior.is_none() || at.only.is_some())
        .then(|| base::materialize(config, commit, None))
        .transpose()?;
    Ok(owned)
}

fn held<'a>(states: &'a [State], config: &Config) -> Vec<&'a State> {
    states
        .iter()
        .filter(|state| config.was_held(&state.file))
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
    config: &Config,
    before: &structural::Measurement,
    after: &structural::Measurement,
    out: &mut Sink,
) -> u8 {
    let code = unsupported(&after.unsupported, at, code, out);
    let code = coverage::lost_said(
        &after.files.lost(&before.files, config, at.only),
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

fn spec(config: &Config) -> Result<Spec, Error> {
    let section = ratchet::section(config, SECTION)?;
    Ok(Spec {
        roots: files::roots(
            section.config,
            section.name,
            &section.values,
            reference::ROOTS,
        )?
        .ok_or_else(|| section.config.missing(section.name, reference::ROOTS.name))?,
        selection: selection(section.config, &section.values)?,
        ignore: files::strings(config, SECTION, &section.values, IGNORE)?,
    })
}

fn selection(config: &Config, values: &Values) -> Result<Selection, Error> {
    let named = files::strings(config, SECTION, values, reference::LANGUAGES)?;
    let extensions =
        structural::selected_extensions(&named).ok_or_else(|| unknown_language(config, &named))?;
    Ok(Selection {
        extensions,
        skip_dirs: files::skip_dirs(config, SECTION, values)?,
        exclude: files::strings(config, SECTION, values, reference::EXCLUDE)?,
    })
}

fn unknown_language(config: &Config, named: &[String]) -> Error {
    let name = named
        .iter()
        .find(|name| structural::selected_extensions(&[(*name).clone()]).is_none())
        .map_or("", String::as_str);
    Error(format!(
        "{}: \"{SECTION}\" measures no language called \"{name}\" — one of: {}",
        config.file.display(),
        structural::known_languages().join(", ")
    ))
}

pub fn language_extensions() -> Vec<(&'static str, String)> {
    structural::language_extensions()
}

fn measure(
    roots: &[PathBuf],
    selection: &Selection,
    repo_root: &Path,
) -> Result<structural::Measurement, Error> {
    let wanted = files::Wanted {
        extensions: &selection.extensions,
        skip_dirs: &selection.skip_dirs,
        exclude: &selection.exclude,
        exclude_except: &[],
        skip_hidden: true,
    };
    let found = files::found(roots, &wanted)?;
    structural::measure(found, repo_root)
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
    let dead = !index.references(&declaration.name).iter().any(|reference| {
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

fn lost_reference(
    state: &State,
    before: &structural::Measurement,
    after: &structural::Measurement,
    before_states: &[&State],
) -> Option<String> {
    let held = before_states.iter().find(|candidate| {
        candidate.file == state.file
            && candidate.line == state.line
            && candidate.name == state.name
            && candidate.text == state.text
    })?;
    if held.dead {
        return None;
    }
    let old = before.index.references(&state.name);
    let now: BTreeSet<(&str, u64)> = after
        .index
        .references(&state.name)
        .into_iter()
        .map(|reference| (reference.file, reference.line))
        .collect();
    old.into_iter()
        .filter(|reference| {
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

fn in_scope(file: &str, only: Option<&[String]>) -> bool {
    only.is_none_or(|only| only.iter().any(|wanted| wanted == file))
}

fn unsupported(files: &[structural::Unsupported], at: &Context, code: u8, out: &mut Sink) -> u8 {
    let files: Vec<&structural::Unsupported> = files
        .iter()
        .filter(|file| in_scope(&file.file, at.only))
        .collect();
    if files.is_empty() {
        return code;
    }
    let word = if at.hook() { "NOTE" } else { "FAIL" };
    let _ = writeln!(
        out.text,
        "{word}: {} file(s) in unsupported structural languages were not measured:",
        files.len()
    );
    for file in &files {
        let _ = writeln!(out.text, "  {}  {}", file.file, file.language);
    }
    let _ = writeln!(
        out.text,
        "Add a structural adapter for the language, or exclude the file and accept that nothing measures it."
    );
    out.record(|records| {
        for file in &files {
            let mut record = Map::new();
            record.insert("outcome".into(), check::NOT_MEASURED.into());
            record.insert("file".into(), file.file.clone().into());
            record.insert(
                "text".into(),
                format!("{} has no structural adapter", file.language).into(),
            );
            if at.hook() {
                records.notes.push(Value::Object(record));
            } else {
                records.findings.push(Value::Object(record));
            }
        }
    });
    if at.hook() { code } else { 2 }
}

fn report_dead(states: &[State], only: Option<&[String]>, out: &mut Sink) {
    let dead: Vec<&State> = states
        .iter()
        .filter(|state| state.dead && in_scope(&state.file, only))
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
        .filter(|state| state.dead && in_scope(&state.file, only))
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
