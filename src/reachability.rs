//! The `reachability` check: a file of a derived family that nothing else in the repository
//! references. A family is a root and a basename glob, such as `src/commands/*_command.rs`, and
//! a member is reached when another file references one of its eligible declarations by name.
//! Identity is the repository-relative path, and the `unreached` metric ratchets from 0 to 1.
//! Resolution is the structural index's name-only rule, so ambiguity makes a file look reached
//! and never unreached. With no section, a family is derived from the derivation commit alone,
//! and only where every member is proven reached without ambiguity. ADR 0035, spec 8.4.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use serde_json::{Map, Value};

use crate::base;
use crate::check::contract::{self, Context, HeldAtBase, Line, Listed, Measured, Sink};
use crate::check::holes;
use crate::config::Config;
use crate::coverage;
use crate::error::Error;
use crate::files;
use crate::key::{self, Key};
use crate::measurement::{self, Measurement};
use crate::project::Project;
use crate::ratchet::{self, Evaluator, Finding, Remedy};
use crate::record::Values;
use crate::scope::{self, Scope, under_or_at};
use crate::survey::{self, Survey};
use crate::syntax::structural::facts::{Declaration, DeclarationKind};
use crate::syntax::structural::{self, Declared, SourceIndex};
use crate::syntax::{self, LanguageId};
use crate::tree::Tree;
use crate::{cache, changed};

pub const SECTION: &str = "reachability";

const UNREACHED: &str = "unreached";
const SIBLING: &str = "sibling";
const LABEL: &str = "file";
/// The fewest members a family may be derived from. Spec 5.4.
const MEMBERS: usize = 3;
const REMEDY: &str = "Wire this file into the application through a real source reference, or \
                      delete it if the implementation is unused. If a public-api break names what \
                      an unreached file held, decide the two separately: restore the public \
                      contract where the task keeps it, or leave the break for a person to accept \
                      where the task removes it, and keep and wire the implementation if it is \
                      still needed, and delete it only if it is unused.";

pub const NAME: Key = Key {
    name: "name",
    holds: "what the run calls this family",
    required: true,
    rule: Some("the root and the pattern, as `src/commands/*_command.rs`"),
    default: "",
    shape: crate::key::Shape::String,
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
    shape: crate::key::Shape::String,
};

pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT];

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
    /// Whether the family's root and pattern select this path, before any exclusion. A file the
    /// test convention marks is never selected. Spec 5.4.
    fn selects(&self, path: &str) -> bool {
        !survey::marked(path)
            && self.roots.iter().any(|root| under_or_at(path, root))
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
                key::ROOTS.name.into(),
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

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let project = at.project;
    let config = &project.config;
    let families = families(project)?;
    said_families(&families, out);
    let commit = contract::base_commit(config.root(), at)?;
    let mut names = structural::NameCost::default();
    let mut layout = None;
    let (before, before_families, after) = sweeps(at, &families, &commit, &mut names, &mut layout)?;
    let (before_states, _) = judgement(&before, &mut names.before, &before_families);
    let (after_states, unjudged) = judgement(&after, &mut names.after, &families);
    out.record(|records| {
        records.facts = Some(before.cost + after.cost);
        records.names = Some(names);
        records.layout = layout;
    });
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
    let judged = after_states.len();
    let unreached = now.len();
    let said = out.covered(&covered(&after, &families).coverage(None));
    let evaluator = evaluator();
    let code = evaluator.evaluate(
        now,
        prior,
        ratchet::accepted(config, at.gate, evaluator.metrics)?,
        at,
        Line::new(
            Measured::Reachability {
                judged,
                unreached,
                unjudged,
            },
            said,
        ),
        out,
    );
    coverage_result(at, (&before, &before_families), (&after, &families), out);
    base_note(&held_before, out);
    Ok(code)
}

/// The two trees measured, over one base extraction in a changed run that is not strict, and the
/// families under the base's scope. Spec 8.4.
fn sweeps(
    at: &Context,
    families: &[Family],
    commit: &str,
    names: &mut structural::NameCost,
    layout: &mut Option<base::Layout>,
) -> Result<(Measurement, Vec<Family>, Measurement), Error> {
    let prior = structural::timed(&mut names.base, || contract::whole_base(at, commit))?;
    let unchanged = structural::timed(&mut names.base, || {
        contract::unchanged_base(at, prior, commit)
    })?;
    *layout = prior.layout();
    let mut after = structural::timed(&mut names.after.measure, || {
        measure(at.project.tree(), families, unchanged.as_ref())
    })?;
    let (before, before_families) =
        structural::timed(&mut names.before.measure, || before(at, families, prior))?;
    after.cost = after.cost
        + unchanged.map_or_else(
            structural::ExtractionCost::default,
            measurement::Unchanged::publish,
        );
    Ok((before, before_families, after))
}

fn judgement(
    measured: &Measurement,
    cost: &mut structural::TreeNameCost,
    families: &[Family],
) -> (Vec<State>, usize) {
    let index = measured.indexed(cost);
    structural::timed(&mut cost.query, || states(index, families))
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
    crate::config::no_retired_key(&config.file, SECTION, values)?;
    let name = values
        .get(NAME.name)
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, NAME.name))?;
    let roots = files::roots(config, SECTION, values, key::ROOTS)?
        .ok_or_else(|| config.missing(SECTION, key::ROOTS.name))?;
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
    let named = files::strings(config, SECTION, values, key::LANGUAGES)?;
    let extensions = structural::selected_extensions(&named)
        .ok_or_else(|| structural::unknown_language(config, SECTION, &named))?;
    Ok(Family {
        name,
        roots,
        pattern,
        extensions,
        exclude: files::strings(config, SECTION, values, key::EXCLUDE)?,
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
fn measure(
    tree: &Tree,
    families: &[Family],
    unchanged: Option<&measurement::Unchanged>,
) -> Result<Measurement, Error> {
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
    let found = files::found(
        tree.root(),
        || tree.files(),
        &[tree.root().to_path_buf()],
        &wanted,
    )?;
    measurement::measure(found, tree, unchanged)
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
    out.tell(contract::Derived::keyed(SECTION, None, value, names, rule));
}

/// The base tree measured, with the families under the scope the base commit recorded, so a
/// file this run's scope takes out shows as lost rather than vanishing. Spec 8.6.
fn before(
    at: &Context,
    families: &[Family],
    prior: &base::Prior,
) -> Result<(Measurement, Vec<Family>), Error> {
    let before_families = base_families(at.config(), prior.root(), families);
    Ok((
        measure(prior.tree(), &before_families, None)?,
        before_families,
    ))
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

/// What names a declaration from outside its file under the name-only rule: the structural
/// index's references and destructurings, and every named TypeScript re-export. A re-export
/// names a declaration by the name a consumer addresses it by, so `export { x as y } from "./m"`
/// names `x`, and `export { default as Profile } from "./m"` names the default export of `m`.
/// The index keeps no re-export as a reference, because `dead-symbols` judges private
/// declarations, which no re-export can name. A star re-export names nothing. Spec 8.4.
struct Naming<'a> {
    index: &'a SourceIndex,
    /// The files that hold an eligible declaration a named re-export in another file names.
    re_exported: HashSet<&'a str>,
    /// The files that hold an eligible declaration a named re-export in another file names by a
    /// name no other declaration under the index answers to.
    proven_by_re_export: HashSet<&'a str>,
}

impl<'a> Naming<'a> {
    /// Reads the re-exports once, from the few leaves that name something, so a member costs
    /// one lookup here and not one for each of its declarations.
    fn of(index: &'a SourceIndex) -> Naming<'a> {
        let mut naming = Naming {
            index,
            re_exported: HashSet::new(),
            proven_by_re_export: HashSet::new(),
        };
        let leaves = re_export_leaves(index);
        if leaves.is_empty() {
            return naming;
        }
        let aliased = aliased(index);
        for ((language, name), from) in &leaves {
            let targets: Vec<Declared> = index
                .declarations(*language, name)
                .filter(|held| {
                    !held.declaration.destructures() && held.declaration.exported_as.is_none()
                })
                .chain(
                    aliased
                        .get(&(*language, *name))
                        .into_iter()
                        .flatten()
                        .copied(),
                )
                .collect();
            let only = targets.len() == 1;
            for held in targets.iter().filter(|held| eligible(held.declaration)) {
                if from.iter().any(|at| *at != held.file) {
                    naming.re_exported.insert(held.file);
                    if only {
                        naming.proven_by_re_export.insert(held.file);
                    }
                }
            }
        }
        naming
    }

    /// Whether another file names one of this file's eligible declarations. A name reaches every
    /// declaration it names, so ambiguity reaches each of them.
    fn reached(&self, file: &structural::facts::FileFacts) -> bool {
        self.re_exported.contains(file.file.as_str())
            || file
                .declarations
                .iter()
                .filter(|declaration| eligible(declaration))
                .any(|declaration| self.referenced_elsewhere(file, declaration))
    }

    /// Whether another file names one eligible declaration of this file by a name no other
    /// declaration under the index answers to, which is evidence no ambiguity could have
    /// produced. A reference is judged by the declaration's name and a re-export by the name a
    /// consumer addresses it by. A destructuring that binds the name is no declaration of that
    /// name here. Spec 5.4.
    fn proven(&self, file: &structural::facts::FileFacts) -> bool {
        self.proven_by_re_export.contains(file.file.as_str())
            || file
                .declarations
                .iter()
                .filter(|declaration| eligible(declaration))
                .any(|declaration| {
                    self.index
                        .declarations(file.language, &declaration.name)
                        .filter(|held| !held.declaration.destructures())
                        .count()
                        == 1
                        && self.referenced_elsewhere(file, declaration)
                })
    }

    /// Whether a file other than the one that holds this declaration references its name or
    /// holds a destructuring declaration that binds it. Spec 5.4.
    fn referenced_elsewhere(
        &self,
        file: &structural::facts::FileFacts,
        declaration: &Declaration,
    ) -> bool {
        self.index
            .references(file.language, &declaration.name)
            .any(|site| site.file != file.file)
            || self
                .index
                .declarations(file.language, &declaration.name)
                .any(|held| held.file != file.file && held.declaration.destructures())
    }
}

/// Every name a named TypeScript re-export names, with the files that re-export it. A star
/// re-export names nothing.
fn re_export_leaves(index: &SourceIndex) -> HashMap<(LanguageId, &str), Vec<&str>> {
    let mut leaves: HashMap<(LanguageId, &str), Vec<&str>> = HashMap::new();
    for file in index.files() {
        let named = file
            .exports
            .iter()
            .filter(|export| export.source.is_some())
            .flat_map(|export| &export.leaves)
            .filter(|leaf| leaf.path != "*");
        for leaf in named {
            leaves
                .entry((file.language, leaf.path.as_str()))
                .or_default()
                .push(&file.file);
        }
    }
    leaves
}

/// Every declaration a consumer addresses by a name other than its own, such as `default` for
/// TypeScript's `export default function Profile`, under that name.
fn aliased(index: &SourceIndex) -> HashMap<(LanguageId, &str), Vec<Declared<'_>>> {
    let mut aliased: HashMap<(LanguageId, &str), Vec<Declared>> = HashMap::new();
    for file in index.files() {
        for declaration in file.declarations.iter().filter(|held| !held.destructures()) {
            if let Some(name) = declaration.exported_as.as_deref() {
                aliased
                    .entry((file.language, name))
                    .or_default()
                    .push(Declared {
                        file: &file.file,
                        language: file.language,
                        declaration,
                    });
            }
        }
    }
    aliased
}

/// Every member with an eligible declaration, judged, and the count of members measured with
/// none, which are not judged and not unreached.
fn states(index: &SourceIndex, families: &[Family]) -> (Vec<State>, usize) {
    let naming = Naming::of(index);
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
            unreached: !naming.reached(file),
            proven: naming.proven(file),
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
    at: &Context,
    (before, before_families): (&Measurement, &[Family]),
    (after, families): (&Measurement, &[Family]),
    out: &mut Sink,
) {
    let lost = covered(after, families).lost(&covered(before, before_families), at.project, None);
    holes::lost_said(&lost, out);
    let unparsed: Vec<syntax::Unparsed> = after
        .unparsed
        .iter()
        .filter(|file| family_of(families, &file.file).is_some())
        .map(|file| syntax::Unparsed {
            file: file.file.clone(),
            language: file.language,
        })
        .collect();
    holes::unread_said(&unparsed, at, out);
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[UNREACHED],
        unit: "unreached file(s)",
        condition: "where no other file references a declaration of the file",
        fix_advice: Remedy::Fixed(REMEDY),
        ceiling: None,
        format_metrics: show,
        nested: None,
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

fn base_note(states: &[&State], out: &mut Sink) {
    let unreached: Vec<&State> = states
        .iter()
        .filter(|state| state.unreached)
        .copied()
        .collect();
    if unreached.is_empty() {
        return;
    }
    out.tell(Listed::Held(HeldAtBase::Unreached(
        unreached.iter().map(|state| state.file.clone()).collect(),
    )));
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
/// those of them that are test roots. Spec 5.4, 8.2.
fn members_at(root: &Path, commit: &str, held: &Survey) -> Option<Vec<String>> {
    let extensions = structural::selected_extensions(&[]).unwrap_or_default();
    let source_test_roots: Vec<&String> = held
        .test_roots
        .iter()
        .filter(|at| held.roots.contains(at))
        .collect();
    Some(
        survey::listed(root, commit)?
            .into_iter()
            .filter(|path| extensions.iter().any(|end| path.ends_with(end)))
            .filter(|path| held.roots.iter().any(|at| under_or_at(path, at)))
            .filter(|path| !source_test_roots.iter().any(|at| under_or_at(path, at)))
            .collect(),
    )
}

/// The commit's files measured through one git process. A file that is not here was not
/// measured: no adapter, a grammar that refused it, or bytes git did not hand over.
fn evidence(root: &Path, commit: &str, paths: &[String]) -> BTreeMap<String, Member> {
    let names: Vec<&str> = paths.iter().map(String::as_str).collect();
    let mut facts = Vec::new();
    let mut name_pool = structural::facts::Names::default();
    changed::blobs(root, commit, &names, |path, bytes| {
        let Some(bytes) = bytes else {
            return;
        };
        if let Ok(structural::facts::Outcome::Facts(found)) =
            structural::of_with(path, &String::from_utf8_lossy(bytes), &mut name_pool)
        {
            facts.push(found);
        }
    });
    let index = SourceIndex::of(facts);
    let naming = Naming::of(&index);
    index
        .files()
        .iter()
        .map(|file| {
            let member = Member {
                eligible: file.declarations.iter().any(eligible),
                proven: naming.proven(file),
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
        key::ROOTS.name.into(),
        Value::Array(vec![candidate.root.clone().into()]),
    );
    out.insert(PATTERN.name.into(), candidate.pattern.clone().into());
    out.insert(key::LANGUAGES.name.into(), Value::Array(vec![name.into()]));
    Some(Value::Object(out))
}
