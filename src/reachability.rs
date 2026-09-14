//! The `reachability` check: a file of a derived family that nothing else in the repository
//! references. A family is a root and a basename glob, such as `src/commands/*_command.rs`, and
//! a member is reached when another file references one of its eligible declarations by name.
//! Identity is the repository-relative path, and the `unreached` metric ratchets from 0 to 1.
//! Resolution is the structural index's name-only rule, so ambiguity makes a file look reached
//! and never unreached. With no section, a family is derived from the derivation commit alone,
//! and only where every member is proven reached without ambiguity. ADR 0035, spec 8.4.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base;
use crate::check::{Context, Sink};
use crate::config::{Config, Error};
use crate::coverage;
use crate::files;
use crate::project::{Project, Tree};
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::{self, Key};
use crate::scope::{self, Scope, under_or_at};
use crate::survey::{self, Survey};
use crate::syntax::structural::{self, Declaration, DeclarationKind, Measurement, SourceIndex};
use crate::syntax::{self, LanguageId};
use crate::{cache, changed};

pub const SECTION: &str = "reachability";

const UNREACHED: &str = "unreached";
const SIBLING: &str = "sibling";
const LABEL: &str = "file";
/// The fewest members a family may be derived from. Spec 5.4.
const MEMBERS: usize = 3;
const REMEDY: &str = "Wire this file into the application through a real source reference, or \
                      delete it if the implementation is unused.";

pub const NAME: Key = Key {
    name: "name",
    holds: "what the run calls this family",
    required: true,
    rule: Some("the root and the pattern, as `src/commands/*_command.rs`"),
    default: "",
};

pub const PATTERN: Key = Key {
    name: "pattern",
    holds: "a glob on the basename that selects the family's files",
    required: true,
    rule: Some(
        "a name prefix or suffix at a token boundary with the concrete extension, shared by at \
         least three files of one directory that the derivation commit proves reached",
    ),
    default: "",
};

pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT];

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
    /// Judge only these repo-relative files, against only their sites at the base
    #[arg(long, num_args = 0.., value_name = "FILE")]
    only: Option<Vec<String>>,
}

/// One family the derivation commit proves: where its files are and what they are called.
#[derive(Clone)]
struct Family {
    name: String,
    roots: Vec<String>,
    pattern: String,
    extensions: Vec<&'static str>,
    exclude: Vec<String>,
    skip_dirs: Vec<String>,
    scope: Scope,
}

impl Family {
    /// Whether the family's root and pattern select this path, before any exclusion.
    fn selects(&self, path: &str) -> bool {
        self.roots.iter().any(|root| under_or_at(path, root))
            && self.extensions.iter().any(|end| path.ends_with(end))
            && files::glob_matches(self.pattern.as_bytes(), basename(path).as_bytes())
    }

    /// Whether this path is a member: selected, and dropped by no exclusion.
    fn holds(&self, path: &str) -> bool {
        self.selects(path)
            && self.scope.selects(path)
            && !path
                .split('/')
                .any(|segment| self.skip_dirs.iter().any(|dir| dir == segment))
            && !self.exclude.iter().any(|glob| {
                files::glob_matches(glob.as_bytes(), basename(path).as_bytes())
                    || files::glob_matches(glob.as_bytes(), path.as_bytes())
            })
    }

    fn record(&self) -> Value {
        Value::Object(Map::from_iter([
            (NAME.name.into(), self.name.clone().into()),
            (
                reference::ROOTS.name.into(),
                Value::Array(self.roots.iter().cloned().map(Value::from).collect()),
            ),
            (PATTERN.name.into(), self.pattern.clone().into()),
        ]))
    }
}

/// One judged member: which family explains it, whether anything reaches it, and whether one
/// of its declarations proves that without ambiguity.
struct State {
    file: String,
    family: usize,
    unreached: bool,
    proven: bool,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let project = Project::load(args.config.as_deref(), start)?;
    let at = Context {
        only: args.only.as_deref(),
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand(SECTION, &project)
    };
    gate(&at, &mut Sink::unrecorded(out))
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let config = &project.config;
    let families = families(project)?;
    said_families(&families, out);
    let commit = base::commit(config.root(), at, out)?;
    let after = measure(project.tree(), &families)?;
    let (before, before_families) = before(at, &families, &commit)?;
    out.record(|records| records.facts = Some(before.cost + after.cost));
    let (before_states, _) = states(&before.index, &before_families);
    let (after_states, unjudged) = states(&after.index, &families);
    let held_before: Vec<&State> = before_states
        .iter()
        .filter(|state| project.was_held(&state.file))
        .collect();
    let prior = held_before
        .iter()
        .map(|state| finding(state, None))
        .collect();
    let now: Vec<Finding> = after_states
        .iter()
        .filter(|state| state.unreached)
        .map(|state| finding(state, sibling(&after_states, state)))
        .collect();
    let judged = after_states
        .iter()
        .filter(|state| coverage::in_scope(&state.file, at.only))
        .count();
    let unreached = now.len();
    let covered_after = covered(&after, &families);
    let said = covered_after.coverage(at.only).said(out);
    let evaluator = evaluator();
    let code = evaluator.evaluate(
        now,
        prior,
        ratchet::accepted(config, at.gate, evaluator.metrics)?,
        at,
        &format!(
            "OK: {judged} file(s) judged, {unreached} unreached, {unjudged} measured with no \
             eligible declaration, all held at the base{said}"
        ),
        out,
    );
    let code = coverage_result(
        code,
        at,
        (&before, &before_families),
        (&after, &families),
        out,
    );
    base_note(&held_before, at.only, out);
    Ok(code)
}

fn families(project: &Project) -> Result<Vec<Family>, Error> {
    let config = &project.config;
    let values = config.policy(SECTION, KEYS)?;
    let scope = Scope::read(config, SECTION, &values)?;
    if scope.has_in() && !applicable(project.tree(), &scope)? {
        return Err(Error(format!(
            "{}: \"{SECTION}\" has an \"in\" scope with no applicable file",
            config.file.display()
        )));
    }
    let held = project
        .source_derivation()
        .and_then(|(facts, commit, at)| derived(project.root(), at, commit, facts))
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let listed = held.as_array().ok_or_else(|| {
        Error(format!(
            "{}: \"{SECTION}\" is a list of families, each a \"name\", \"roots\" and \"pattern\"",
            config.file.display()
        ))
    })?;
    listed
        .iter()
        .map(|entry| family(config, entry, &scope))
        .collect()
}

fn family(config: &Config, entry: &Value, scope: &Scope) -> Result<Family, Error> {
    let values = entry
        .as_object()
        .ok_or_else(|| config.malformed(SECTION, "entry", "an object"))?;
    ratchet::no_retired_key(&config.file, SECTION, values)?;
    let name = values
        .get(NAME.name)
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, NAME.name))?;
    let roots = files::roots(config, SECTION, values, reference::ROOTS)?
        .ok_or_else(|| config.missing(SECTION, reference::ROOTS.name))?;
    let pattern = values
        .get(PATTERN.name)
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, PATTERN.name))?;
    let roots = roots
        .iter()
        .map(|root| files::relative(root, config.root()))
        .collect();
    selected_by(
        config,
        values,
        name.to_string(),
        roots,
        pattern.to_string(),
        scope,
    )
}

/// The family with its file selection read: the languages that choose its extensions, and the
/// exclusions and skipped directories a person adds.
fn selected_by(
    config: &Config,
    values: &Map<String, Value>,
    name: String,
    roots: Vec<String>,
    pattern: String,
    scope: &Scope,
) -> Result<Family, Error> {
    let named = files::strings(config, SECTION, values, reference::LANGUAGES)?;
    let extensions = structural::selected_extensions(&named)
        .ok_or_else(|| structural::unknown_language(config, SECTION, &named))?;
    Ok(Family {
        name,
        roots,
        pattern,
        extensions,
        exclude: files::strings(config, SECTION, values, reference::EXCLUDE)?,
        skip_dirs: files::skip_dirs(config, SECTION, values)?,
        scope: scope.clone(),
    })
}

fn applicable(tree: &Tree, scope: &Scope) -> Result<bool, Error> {
    let extensions = structural::selected_extensions(&[]).unwrap_or_default();
    Ok(tree
        .files()?
        .iter()
        .any(|file| scope.inside(file) && extensions.iter().any(|end| file.ends_with(end))))
}

pub fn language_extensions() -> Vec<(&'static str, String)> {
    structural::language_extensions()
}

/// The whole tree under the root, in the families' languages, because a caller may sit
/// anywhere in the repository and a member is reached by any of them. Spec 8.4.
fn measure(tree: &Tree, families: &[Family]) -> Result<Measurement, Error> {
    let mut extensions: Vec<&str> = families
        .iter()
        .flat_map(|family| family.extensions.iter().copied())
        .collect();
    extensions.sort_unstable();
    extensions.dedup();
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &extensions,
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let found = files::found(tree, &[tree.root().to_path_buf()], &wanted)?;
    structural::measure(found, tree)
}

/// The derived families as one provenance line and its JSON entry, and nothing when none is.
fn said_families(families: &[Family], out: &mut Sink) {
    if families.is_empty() {
        return;
    }
    let names = families
        .iter()
        .map(|family| family.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let value = Value::Array(families.iter().map(Family::record).collect());
    let rule = "the file families the derivation commit proves reached";
    out.provenance(
        format!("derived: {SECTION} {names}, {rule}"),
        Some(Value::Object(Map::from_iter([
            ("section".into(), SECTION.into()),
            ("key".into(), Value::Null),
            ("value".into(), value),
            ("rule".into(), rule.into()),
        ]))),
    );
}

/// The base tree measured, with the families under the scope the base commit recorded, so a
/// file this run's scope takes out shows as lost rather than vanishing. Spec 8.6.
fn before(
    at: &Context,
    families: &[Family],
    commit: &str,
) -> Result<(Measurement, Vec<Family>), Error> {
    let prior = base::whole(at, commit)?;
    let before_families = base_families(at.config(), prior.root(), families);
    Ok((measure(prior.tree(), &before_families)?, before_families))
}

/// Each derived family under the compact scope recorded by the base commit. Spec 8.6.
fn base_families(config: &Config, prior: &Path, families: &[Family]) -> Vec<Family> {
    let today = families
        .first()
        .map(|family| family.scope.clone())
        .unwrap_or_default();
    let scope = Scope::at_base(config, SECTION, prior, &today);
    families
        .iter()
        .map(|family| Family {
            scope: scope.clone(),
            ..family.clone()
        })
        .collect()
}

/// A declaration that counts as reachability evidence: not a method, whose name is shared
/// across unrelated types too often for a name-only rule, and not an entry point, which a
/// runtime reaches without a source reference. Spec 8.4.
fn eligible(declaration: &Declaration) -> bool {
    declaration.kind != DeclarationKind::Method && !declaration.entry_point
}

fn family_of(families: &[Family], path: &str) -> Option<usize> {
    families.iter().position(|family| family.holds(path))
}

/// Every file some other file reaches, found from the references: each reference names every
/// declaration of its name, and a declaration in another file marks that file reached.
fn reached(index: &SourceIndex) -> BTreeSet<&str> {
    let mut out = BTreeSet::new();
    for file in index.files() {
        for reference in &file.references {
            for held in index.declarations(file.language, &reference.name) {
                if held.file != file.file && eligible(held.declaration) {
                    out.insert(held.file);
                }
            }
        }
    }
    out
}

/// Whether one eligible declaration of this file is the only one of its name under the index
/// and another file references it, which is evidence no ambiguity could have produced.
fn proven(index: &SourceIndex, file: &structural::FileFacts) -> bool {
    file.declarations
        .iter()
        .filter(|declaration| eligible(declaration))
        .any(|declaration| {
            index.declarations(file.language, &declaration.name).count() == 1
                && index
                    .references(file.language, &declaration.name)
                    .any(|site| site.file != file.file)
        })
}

/// Every member with an eligible declaration, judged, and the count of members measured with
/// none, which are not judged and not unreached.
fn states(index: &SourceIndex, families: &[Family]) -> (Vec<State>, usize) {
    let reached = reached(index);
    let mut out = Vec::new();
    let mut unjudged = 0;
    for file in index.files() {
        let Some(family) = family_of(families, &file.file) else {
            continue;
        };
        if !file.declarations.iter().any(eligible) {
            unjudged += 1;
            continue;
        }
        out.push(State {
            file: file.file.clone(),
            family,
            unreached: !reached.contains(file.file.as_str()),
            proven: proven(index, file),
        });
    }
    (out, unjudged)
}

/// The first proven sibling under the same family, in path order, and none when every sibling
/// is unreached or reached only through a name several files declare.
fn sibling<'a>(states: &'a [State], state: &State) -> Option<&'a str> {
    states
        .iter()
        .find(|other| other.family == state.family && other.proven && other.file != state.file)
        .map(|other| other.file.as_str())
}

fn finding(state: &State, sibling: Option<&str>) -> Finding {
    let mut values = Values::new();
    values.insert(UNREACHED.into(), u64::from(state.unreached).into());
    if let Some(sibling) = sibling {
        values.insert(SIBLING.into(), sibling.into());
    }
    Finding {
        file: state.file.clone(),
        line: 0,
        text: LABEL.to_string(),
        values,
        body: None,
    }
}

/// The measured tree's files kept to the families' members: judged, no adapter, dropped by an
/// exclusion, and refused by the grammar. Spec 8.6.
fn covered(measured: &Measurement, families: &[Family]) -> coverage::Files {
    let member = |files: &[String]| -> Vec<String> {
        files
            .iter()
            .filter(|file| family_of(families, file).is_some())
            .cloned()
            .collect()
    };
    let all = measured
        .files
        .measured
        .iter()
        .chain(&measured.files.not_measured)
        .chain(&measured.files.unreadable);
    coverage::Files {
        measured: member(&measured.files.measured),
        not_measured: member(&measured.files.not_measured),
        unreadable: member(&measured.files.unreadable),
        excluded: all
            .filter(|file| {
                family_of(families, file).is_none()
                    && families.iter().any(|family| family.selects(file))
            })
            .cloned()
            .collect(),
    }
}

fn coverage_result(
    code: u8,
    at: &Context,
    (before, before_families): (&Measurement, &[Family]),
    (after, families): (&Measurement, &[Family]),
    out: &mut Sink,
) -> u8 {
    let unsupported: Vec<structural::Unsupported> = after
        .unsupported
        .iter()
        .filter(|file| family_of(families, &file.file).is_some())
        .map(|file| structural::Unsupported {
            file: file.file.clone(),
            language: file.language,
        })
        .collect();
    let code = coverage::not_measured_said(&unsupported, at, code, out);
    let lost =
        covered(after, families).lost(&covered(before, before_families), at.project, at.only);
    let code = coverage::lost_said(&lost, at, code, out);
    let unparsed: Vec<syntax::Unparsed> = after
        .unparsed
        .iter()
        .filter(|file| family_of(families, &file.file).is_some())
        .map(|file| syntax::Unparsed {
            file: file.file.clone(),
            language: file.language,
        })
        .collect();
    syntax::unread(&unparsed, at, code, out)
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[UNREACHED],
        unit: "unreached file(s)",
        condition: "where no other file references a declaration of the file",
        fix_advice: REMEDY,
        ceiling: None,
        format_metrics: show,
    }
}

fn show(values: &Values) -> String {
    let state = match values.get(UNREACHED).and_then(Value::as_u64) {
        Some(0) => "reached",
        Some(1) => "unreached",
        _ => "unknown",
    };
    match values.get(SIBLING).and_then(Value::as_str) {
        Some(file) => format!("{state}, a reached sibling is {file}"),
        None => state.to_string(),
    }
}

fn base_note(states: &[&State], only: Option<&[String]>, out: &mut Sink) {
    let unreached: Vec<&State> = states
        .iter()
        .filter(|state| state.unreached && coverage::in_scope(&state.file, only))
        .copied()
        .collect();
    if unreached.is_empty() {
        return;
    }
    let mut note = format!(
        "{} unreached file(s) the base already held:",
        unreached.len()
    );
    for state in unreached.iter().take(20) {
        let _ = write!(note, "\n  {}", state.file);
    }
    if unreached.len() > 20 {
        let _ = write!(note, "\n  … and {} more", unreached.len() - 20);
    }
    ratchet::noted(&[(String::new(), note)], out);
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

/// What the derivation commit says about one structural file: whether it declares anything
/// eligible, whether another file provably reaches it, and its language.
struct Member {
    eligible: bool,
    proven: bool,
    language: LanguageId,
}

/// One family the derivation commit might prove: a directory, a basename glob, and every file
/// under that directory the glob selects.
struct Candidate {
    root: String,
    pattern: String,
    cohort: Vec<String>,
}

/// The families the derivation commit proves, as the section the survey supplies when the
/// config names none, and `None` when it proves none, so no empty list is pinned. Cached under
/// the commit, and nothing of the working tree reaches it. Spec 4.3, 5.4, 6.6.
pub fn derived(root: &Path, at: Option<&Path>, commit: &str, held: &Survey) -> Option<Value> {
    if held.roots.is_empty() {
        return None;
    }
    let families = match at.and_then(|at| cache::read(at, commit, SECTION)) {
        Some(Value::Array(cached)) => cached,
        _ => {
            let paths = members_at(root, commit, held)?;
            let evidence = evidence(root, commit, &paths);
            let families = families_of(&paths, &evidence);
            if let Some(at) = at {
                cache::write(at, commit, SECTION, Value::Array(families.clone()));
            }
            families
        }
    };
    (!families.is_empty()).then_some(Value::Array(families))
}

/// Every structural file the derivation commit holds under its own source roots and outside
/// its test roots. A test root is never a family. Spec 8.2.
fn members_at(root: &Path, commit: &str, held: &Survey) -> Option<Vec<String>> {
    let extensions = structural::selected_extensions(&[]).unwrap_or_default();
    Some(
        survey::listed(root, commit)?
            .into_iter()
            .filter(|path| extensions.iter().any(|end| path.ends_with(end)))
            .filter(|path| held.roots.iter().any(|at| under_or_at(path, at)))
            .filter(|path| !held.test_roots.iter().any(|at| under_or_at(path, at)))
            .collect(),
    )
}

/// The commit's files measured through one git process. A file that is not here was not
/// measured: no adapter, a grammar that refused it, or bytes git did not hand over.
fn evidence(root: &Path, commit: &str, paths: &[String]) -> BTreeMap<String, Member> {
    let names: Vec<&str> = paths.iter().map(String::as_str).collect();
    let mut facts = Vec::new();
    changed::blobs(root, commit, &names, |path, bytes| {
        let Some(bytes) = bytes else {
            return;
        };
        if let Ok(structural::Outcome::Facts(found)) =
            structural::of(path, &String::from_utf8_lossy(bytes))
        {
            facts.push(found);
        }
    });
    let index = SourceIndex::of(facts);
    index
        .files()
        .iter()
        .map(|file| {
            let member = Member {
                eligible: file.declarations.iter().any(eligible),
                proven: proven(&index, file),
                language: file.language,
            };
            (file.file.clone(), member)
        })
        .collect()
}

fn families_of(paths: &[String], evidence: &BTreeMap<String, Member>) -> Vec<Value> {
    let safe: Vec<Candidate> = candidates(paths)
        .into_iter()
        .filter(|candidate| {
            candidate.cohort.len() >= MEMBERS
                && candidate.cohort.iter().all(|path| {
                    evidence
                        .get(path)
                        .is_some_and(|member| member.eligible && member.proven)
                })
        })
        .collect();
    selected(safe)
        .iter()
        .filter_map(|candidate| entry(candidate, evidence))
        .collect()
}

/// Every prefix and suffix family at least three files of one directory and one extension
/// share, each with the complete cohort its root and pattern select. Spec 5.4.
fn candidates(paths: &[String]) -> Vec<Candidate> {
    let mut out = Vec::new();
    for ((directory, extension), names) in grouped(paths) {
        for pattern in shared(&names, extension) {
            out.push(candidate(paths, &directory, pattern));
        }
    }
    out
}

/// The basenames of one directory and one concrete extension, which is where a family is
/// seeded. A file at the tree root has no directory and seeds nothing, and a test directory
/// seeds nothing, though its files stay in the cohort of a family rooted above it. Spec 8.2.
fn grouped(paths: &[String]) -> BTreeMap<(String, &'static str), Vec<&str>> {
    let extensions = structural::selected_extensions(&[]).unwrap_or_default();
    let mut groups: BTreeMap<(String, &'static str), Vec<&str>> = BTreeMap::new();
    for path in paths {
        let Some((directory, name)) = path.rsplit_once('/') else {
            continue;
        };
        if directory
            .split('/')
            .any(|segment| survey::TEST_DIRS.contains(&segment))
        {
            continue;
        }
        if let Some(extension) = extension_of(name, &extensions) {
            groups
                .entry((directory.to_string(), extension))
                .or_default()
                .push(name);
        }
    }
    groups
}

/// The globs at least three of these basenames share.
fn shared(names: &[&str], extension: &str) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for name in names {
        let stem = &name[..name.len() - extension.len()];
        for pattern in patterns(stem, extension) {
            *counts.entry(pattern).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count >= MEMBERS)
        .map(|(pattern, _)| pattern)
        .collect()
}

fn extension_of(name: &str, extensions: &[&'static str]) -> Option<&'static str> {
    extensions
        .iter()
        .copied()
        .filter(|end| name.ends_with(end) && name.len() > end.len())
        .max_by_key(|end| end.len())
}

/// The basename globs one stem suggests: a prefix ending at a token boundary with a star after
/// it, and a suffix starting at one with a star before it. A boundary is a `_`, `-` or `.`, or
/// a capital after a lower-case letter or a digit. Nothing interior, and never the bare
/// extension.
fn patterns(stem: &str, extension: &str) -> Vec<String> {
    let bytes = stem.as_bytes();
    let mut out = Vec::new();
    for at in 1..bytes.len() {
        let (prefix_end, suffix_start) = match bytes[at] {
            b'_' | b'-' | b'.' => (at + 1, at),
            upper
                if upper.is_ascii_uppercase()
                    && (bytes[at - 1].is_ascii_lowercase() || bytes[at - 1].is_ascii_digit()) =>
            {
                (at, at)
            }
            _ => continue,
        };
        if prefix_end < bytes.len() {
            out.push(format!("{}*{extension}", &stem[..prefix_end]));
        }
        out.push(format!("*{}{extension}", &stem[suffix_start..]));
    }
    out
}

fn candidate(paths: &[String], directory: &str, pattern: String) -> Candidate {
    let cohort = paths
        .iter()
        .filter(|path| under_or_at(path, directory))
        .filter(|path| files::glob_matches(pattern.as_bytes(), basename(path).as_bytes()))
        .cloned()
        .collect();
    Candidate {
        root: directory.to_string(),
        pattern,
        cohort,
    }
}

/// The broadest safe candidates first, each kept unless one already kept selects its whole
/// cohort, then in name order so the section reads the same on every run.
fn selected(mut safe: Vec<Candidate>) -> Vec<Candidate> {
    let by_name = |a: &Candidate, b: &Candidate| (&a.root, &a.pattern).cmp(&(&b.root, &b.pattern));
    safe.sort_by(|a, b| {
        b.cohort
            .len()
            .cmp(&a.cohort.len())
            .then_with(|| by_name(a, b))
    });
    let mut out: Vec<Candidate> = Vec::new();
    for candidate in safe {
        let covered = out.iter().any(|held| {
            candidate
                .cohort
                .iter()
                .all(|path| held.cohort.contains(path))
        });
        if !covered {
            out.push(candidate);
        }
    }
    out.sort_by(by_name);
    out
}

fn entry(candidate: &Candidate, evidence: &BTreeMap<String, Member>) -> Option<Value> {
    let language = evidence.get(candidate.cohort.first()?)?.language;
    let (name, _) = structural::languages()
        .into_iter()
        .find(|(_, id)| *id == language)?;
    let mut out = Map::new();
    out.insert(
        NAME.name.into(),
        format!("{}/{}", candidate.root, candidate.pattern).into(),
    );
    out.insert(
        reference::ROOTS.name.into(),
        Value::Array(vec![candidate.root.clone().into()]),
    );
    out.insert(PATTERN.name.into(), candidate.pattern.clone().into());
    out.insert(
        reference::LANGUAGES.name.into(),
        Value::Array(vec![name.into()]),
    );
    Some(Value::Object(out))
}
