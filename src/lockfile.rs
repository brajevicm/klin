//! `lockfile` proves that every dependency a manifest names has an entry in the lockfile beside
//! it, that no pin the base held is gone, and that the lockfile records each exact pin. The
//! manifests are the ones the survey finds that klin has a reader for; a person narrows them only
//! with `in` and `except`. A site is the manifest's path and the dependency's name. Every
//! manifest and lockfile is read once per tree, and a lockfile several manifests share is parsed
//! once. Spec 5.4, 8.2.1, ADR 0040.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::Value;

use crate::check::{self, Context, Sink};
use crate::config::Error;
use crate::coverage::Coverage;
use crate::project::Project;
use crate::ratchet::{self, Evaluator, Finding, Line, Remedy, Values};
use crate::reference::Key;
use crate::scope::{self, Scope};
use crate::{base, changed};

pub const SECTION: &str = "lockfile";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT];

const RULE: &str = "the manifests the survey found that klin can read a lockfile for";
const UNLOCKED: &str = "unlocked";
const UNPINNED: &str = "unpinned";
const STALE: &str = "stale";
const METRICS: &[&str] = &[UNLOCKED, UNPINNED, STALE];
const REMEDIES: &[(&str, &str)] = &[
    (
        UNLOCKED,
        "Run the project's own install, so the lockfile records the dependency. A dependency the \
         lockfile does not know is one no install has ever resolved.",
    ),
    (
        UNPINNED,
        "Restore the exact version the base pinned, or pin an exact version for a new \
         dependency. A pin the base held is a version a person chose.",
    ),
    (
        STALE,
        "Install again, so the lockfile records the pinned version. A lockfile that records \
         another version installs that one, whatever the manifest pins.",
    ),
];

/// One dependency a manifest names, before the lockfile beside it is read. `name` is what the
/// manifest calls it, which is the site's identity, and `package` is what the lockfile records
/// it under, which a Cargo rename makes different. A dependency with no registry behind it — a
/// path, a git or a workspace dependency — has no lockfile entry to want and no version to pin,
/// so it carries no value. `pins` holds every exact version a statement gave it, which is judged
/// for staleness even when another statement is a range. A statement only ever takes a value
/// away, so two tables that name one dependency give the same reading in either order.
struct Dependency {
    name: String,
    package: String,
    registry: bool,
    exact: bool,
    pins: Vec<String>,
}

impl Dependency {
    fn named(name: &str) -> Dependency {
        Dependency {
            name: name.to_string(),
            package: name.to_string(),
            registry: true,
            exact: true,
            pins: Vec::new(),
        }
    }

    fn record_requirement(&mut self, requirement: &str) {
        match requirement.strip_prefix('=') {
            Some(pin) => self.pins.push(pin.trim().to_string()),
            None => self.exact = false,
        }
    }
}

/// One ecosystem: the manifest klin reads, the lockfiles it can read beside it, and the two
/// readers. A lockfile reader gives each name it holds, with the version it records for that
/// name where it records one, and returns `None` when its shape is not one it recognizes. An
/// entry with an `importer` is what the manifest in that directory, relative to the lockfile,
/// resolves the name to, and one without is a package in the lockfile's pool. Spec 8.2.1.
type DependencyReader = fn(&str, &[u8]) -> Result<Vec<Dependency>, Error>;
type LockfileReader = fn(&str, &[u8]) -> Result<Option<Vec<Entry>>, Error>;

struct Entry {
    name: String,
    version: Option<String>,
    importer: Option<String>,
}

impl Entry {
    fn pooled(name: &str, version: Option<String>) -> Entry {
        Entry {
            name: name.to_string(),
            version,
            importer: None,
        }
    }

    fn resolved(name: &str, version: Option<String>, importer: &str) -> Entry {
        Entry {
            importer: Some(importer.to_string()),
            ..Entry::pooled(name, version)
        }
    }
}

#[derive(Default)]
struct Locked {
    names: HashSet<String>,
    pool: HashMap<String, HashSet<String>>,
    importers: HashMap<(String, String), HashSet<String>>,
}

impl Locked {
    fn read(entries: Vec<Entry>) -> Locked {
        let mut out = Locked::default();
        for entry in entries {
            out.names.insert(entry.name.clone());
            let versions = match entry.importer {
                Some(importer) => out.importers.entry((importer, entry.name)).or_default(),
                None => out.pool.entry(entry.name).or_default(),
            };
            versions.extend(entry.version);
        }
        out
    }

    fn versions(&self, importer: &str, name: &str) -> Option<&HashSet<String>> {
        if self.importers.is_empty() {
            return self.pool.get(name);
        }
        let at = |importer: &str| {
            self.importers
                .get(&(importer.to_string(), name.to_string()))
        };
        at(importer).or_else(|| at(""))
    }
}

struct Format {
    manifest: &'static str,
    lockfiles: &'static [&'static str],
    dependencies: DependencyReader,
    locked: LockfileReader,
}

const FORMATS: &[Format] = &[
    Format {
        manifest: "Cargo.toml",
        lockfiles: &["Cargo.lock"],
        dependencies: cargo_dependencies,
        locked: cargo_locked,
    },
    Format {
        manifest: "package.json",
        lockfiles: &["package-lock.json", "pnpm-lock.yaml", "yarn.lock"],
        dependencies: npm_dependencies,
        locked: npm_locked,
    },
    Format {
        manifest: "go.mod",
        lockfiles: &["go.sum"],
        dependencies: go_dependencies,
        locked: go_locked,
    },
];

/// Every manifest the survey found that klin has a reader for, with that reader. Spec 5.4.
fn manifests(project: &Project) -> Vec<(&str, &'static Format)> {
    project
        .facts()
        .found
        .manifests
        .iter()
        .filter_map(|path| {
            let format = FORMATS
                .iter()
                .find(|format| format.manifest == basename(path))?;
            Some((path.as_str(), format))
        })
        .collect()
}

/// Whether the tree holds a manifest this check reads, which is when it runs with no section.
pub fn applies(project: &Project) -> bool {
    !manifests(project).is_empty()
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let sites = surveyed(at, out)?;
    let mut accepted = ratchet::accepted(at.config(), at.gate, &[UNLOCKED, UNPINNED])?;
    for entry in &mut accepted {
        if !entry.get(STALE).is_some_and(Value::is_number) {
            entry.insert(STALE.into(), 0.into());
        }
    }
    let state = format!(
        "{} dependenc{} in {} manifest(s), each locked and pinned",
        sites.judged,
        plural(sites.judged),
        sites.manifests
    );
    let tail = sites.coverage().said(out);
    let code = evaluator().evaluate(
        sites.findings,
        sites.prior,
        accepted,
        at,
        Line {
            state: &state,
            tail: &tail,
        },
        out,
    );
    ratchet::noted(&sites.notes, out);
    Ok(code)
}

/// Every manifest the survey found, read in both trees, less the ones the scope takes out.
fn surveyed(at: &Context, out: &mut Sink) -> Result<Sites, Error> {
    let config = at.config();
    let found = manifests(at.project);
    let scope = scope(at, &found)?;
    said(&found, out);
    let (judged, dropped): (Vec<_>, Vec<_>) =
        found.iter().partition(|(path, _)| scope.selects(path));
    let commit = base::commit(config.root(), at, out)?;
    let changes = at.project.changes(&commit)?;
    let renamed = renamed(&changes, &judged);
    let judged: Vec<(&str, &str, &Format)> = judged
        .into_iter()
        .map(|&(manifest, format)| {
            let was = renamed.get(manifest).copied().unwrap_or(manifest);
            (manifest, was, format)
        })
        .collect();
    let mut now = Side::working(
        config.root(),
        &candidates(judged.iter().map(|(now, _, format)| (*now, *format))),
    );
    let mut before = Side::at(
        config.root(),
        &commit,
        &candidates(judged.iter().map(|(_, was, format)| (*was, *format))),
    )?;
    let mut sites = Sites {
        listed: found.len(),
        excluded: dropped.len(),
        ..Sites::default()
    };
    for (manifest, was, format) in &judged {
        sites.add((&mut now, &mut before), (manifest, was), format, at.hook())?;
    }
    Ok(sites)
}

/// The path the base holds each judged manifest at that the window renamed and that kept its
/// format, read off the change set's rows for judged manifests only. A manifest renamed from
/// another format has no comparable base, because its base bytes were written for another
/// reader, so it is judged as one the base did not hold.
fn renamed<'a>(
    changes: &'a [changed::Change],
    judged: &[&(&str, &Format)],
) -> HashMap<&'a str, &'a str> {
    let judged: HashSet<&str> = judged.iter().map(|(path, _)| *path).collect();
    changes
        .iter()
        .filter(|change| judged.contains(change.path.as_str()))
        .filter_map(|change| {
            let was = change
                .was
                .as_deref()
                .filter(|was| basename(was) == basename(&change.path))?;
            Some((change.path.as_str(), was))
        })
        .collect()
}

/// The section's scope, refused when an `in` selects no manifest the survey found.
fn scope(at: &Context, found: &[(&str, &Format)]) -> Result<Scope, Error> {
    let config = at.config();
    let values = config.policy(SECTION, KEYS)?;
    let scope = Scope::read(config, SECTION, &values)?;
    if scope.has_in() && !found.iter().any(|(path, _)| scope.selects(path)) {
        return Err(Error(format!(
            "{}: \"{SECTION}\" has an \"in\" scope with no applicable manifest",
            config.file.display()
        )));
    }
    Ok(scope)
}

/// The manifests this run judges, as the one `derived:` line and its JSON entry.
fn said(found: &[(&str, &Format)], out: &mut Sink) {
    let names: Vec<&str> = found.iter().map(|(path, _)| *path).collect();
    out.provenance(
        format!("derived: {SECTION} manifests {}, {RULE}", names.join(", ")),
        Some(check::derived_entry(
            SECTION,
            Some("manifests"),
            names.into(),
            RULE,
        )),
    );
}

/// Every path a judged manifest could read in one tree: the manifest, and each lockfile name its
/// format knows at its own directory and at every directory above it.
fn candidates<'a>(judged: impl Iterator<Item = (&'a str, &'a Format)>) -> Vec<String> {
    let mut out = BTreeSet::new();
    for (manifest, format) in judged {
        out.insert(manifest.to_string());
        for name in format.lockfiles {
            out.extend(nearest(manifest, name));
        }
    }
    out.into_iter().collect()
}

/// One tree's bytes for every path a judged manifest could read, each read once, and the names
/// and versions every lockfile holds, parsed once however many manifests share it.
struct Side {
    bytes: HashMap<String, Vec<u8>>,
    locked: HashMap<String, Result<Option<Locked>, String>>,
}

impl Side {
    fn working(root: &std::path::Path, paths: &[String]) -> Side {
        let bytes = paths
            .iter()
            .filter_map(|path| Some((path.clone(), std::fs::read(root.join(path)).ok()?)))
            .collect();
        Side {
            bytes,
            locked: HashMap::new(),
        }
    }

    /// The same kind of paths at the base commit, through one git process. A read git refuses is
    /// an error: an empty base would read every dependency as new.
    fn at(root: &std::path::Path, commit: &str, paths: &[String]) -> Result<Side, Error> {
        let mut bytes = HashMap::new();
        let names: Vec<&str> = paths.iter().map(String::as_str).collect();
        changed::blobs(root, commit, &names, |path, held| {
            if let Some(held) = held {
                bytes.insert(path.to_string(), held.to_vec());
            }
        })
        .ok_or_else(|| {
            Error(format!(
                "the base commit {} could not be read — fetch history, or give CI the full clone",
                &commit[..7.min(commit.len())]
            ))
        })?;
        Ok(Side {
            bytes,
            locked: HashMap::new(),
        })
    }

    /// What this tree says about one manifest, and `None` when it holds no such manifest.
    fn state(&mut self, manifest: &str, format: &Format) -> Result<Option<State>, Error> {
        let Some(text) = self.bytes.get(manifest) else {
            return Ok(None);
        };
        let found = beside(&self.bytes, manifest, format.lockfiles);
        let none = Locked::default();
        let mut unreadable = None;
        let names = match &found {
            Some(at) => match locked(&mut self.locked, &self.bytes, at, format)? {
                Some(names) => names,
                None => {
                    unreadable = Some(at.clone());
                    &none
                }
            },
            None => &none,
        };
        let importer = found
            .as_deref()
            .map_or(String::new(), |at| relative(&parent(manifest), &parent(at)));
        let (deps, unparsed) = match (format.dependencies)(manifest, text) {
            Ok(deps) => (taken(deps, names, &importer), None),
            Err(Error(why)) => (BTreeMap::new(), Some(why)),
        };
        Ok(Some(State {
            deps,
            unparsed,
            unreadable,
            lockfile: found,
        }))
    }
}

/// The names one lockfile holds, each with the versions it records, parsed on the first
/// manifest that reads it and borrowed by every manifest after, and the error a malformed one
/// gives each of them.
fn locked<'a>(
    parsed: &'a mut HashMap<String, Result<Option<Locked>, String>>,
    bytes: &HashMap<String, Vec<u8>>,
    at: &str,
    format: &Format,
) -> Result<Option<&'a Locked>, Error> {
    let names = parsed.entry(at.to_string()).or_insert_with(|| {
        (format.locked)(at, &bytes[at])
            .map(|entries| entries.map(Locked::read))
            .map_err(|Error(why)| why)
    });
    names
        .as_ref()
        .map(Option::as_ref)
        .map_err(|why| Error(why.clone()))
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: METRICS,
        unit: "dependenc(ies)",
        condition: "that the lockfile beside the manifest does not lock at an exact pin",
        fix_advice: Remedy::ByValues(remedy),
        ceiling: None,
        format_metrics: show,
        nested: None,
    }
}

fn remedy(failing: &[&Values]) -> String {
    REMEDIES
        .iter()
        .filter(|(metric, _)| failing.iter().any(|values| number(values, metric) > 0))
        .map(|(_, text)| *text)
        .collect::<Vec<_>>()
        .join(" ")
}

fn show(values: &Values) -> String {
    format!(
        "{UNLOCKED} {}, {UNPINNED} {}, {STALE} {}",
        number(values, UNLOCKED),
        number(values, UNPINNED),
        number(values, STALE)
    )
}

fn number(values: &Values, key: &str) -> u64 {
    values.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn plural(count: usize) -> &'static str {
    match count {
        1 => "y",
        _ => "ies",
    }
}

/// What every judged manifest contributed: the working tree's sites, the base's sites beside
/// them, and one NOTE per manifest klin did not judge.
#[derive(Default)]
struct Sites {
    findings: Vec<Finding>,
    prior: Vec<Finding>,
    notes: Vec<(String, String)>,
    judged: usize,
    manifests: usize,
    /// What the gate's coverage counts beside `manifests`, the manifests it judged: how many
    /// the survey found, how many of those the scope took out, and how many the working tree
    /// no longer holds, which it discovers nothing of. A manifest it reached and did not read
    /// left a NOTE instead, and the coverage calls that one unreadable. Spec 8.6.
    listed: usize,
    excluded: usize,
    absent: usize,
}

impl Sites {
    fn coverage(&self) -> Coverage {
        Coverage {
            found: self.listed - self.absent,
            measured: self.manifests,
            not_measured: 0,
            excluded: self.excluded,
            unreadable: self.notes.len(),
        }
    }

    fn add(
        &mut self,
        (now, before): (&mut Side, &mut Side),
        (manifest, was): (&str, &str),
        format: &Format,
        hook: bool,
    ) -> Result<(), Error> {
        let (now, before) = match reading((now, before), (manifest, was), format, hook)? {
            Reading::Judged(now, before) => (now, before),
            Reading::Noted(at, why) => {
                self.notes.push((at, why));
                return Ok(());
            }
            Reading::Absent => {
                self.absent += 1;
                return Ok(());
            }
        };
        self.manifests += 1;
        self.judged += now.deps.len();
        for (name, values) in &now.deps {
            if before.deps.contains_key(name) || fails_when_new(values) {
                self.findings.push(finding(manifest, name, values.clone()));
            }
        }
        self.prior.extend(
            before
                .deps
                .iter()
                .map(|(name, values)| finding(manifest, name, values.clone())),
        );
        Ok(())
    }
}

fn fails_when_new(values: &Values) -> bool {
    number(values, UNLOCKED) > 0 || number(values, STALE) > 0
}

fn finding(manifest: &str, name: &str, values: Values) -> Finding {
    Finding {
        file: manifest.to_string(),
        line: 0,
        text: name.to_string(),
        values,
        body: None,
    }
}

/// What one tree says about one manifest: its dependencies with every value already taken, the
/// lockfile klin read them against, a lockfile klin found and cannot read, and why klin could
/// not parse the manifest itself.
#[derive(Default)]
struct State {
    deps: BTreeMap<String, Values>,
    lockfile: Option<String>,
    unreadable: Option<String>,
    unparsed: Option<String>,
}

/// Whether klin judges this manifest, and the NOTE that says why not. A manifest the working
/// tree no longer holds has no dependencies to judge and says nothing.
enum Reading {
    Judged(State, State),
    Noted(String, String),
    Absent,
}

/// `was` is the path the base holds the manifest at, which the window's rename may have changed,
/// so the base judges it beside the lockfile it had there.
fn reading(
    (now, before): (&mut Side, &mut Side),
    (manifest, was): (&str, &str),
    format: &Format,
    hook: bool,
) -> Result<Reading, Error> {
    let Some(now) = now.state(manifest, format)? else {
        return Ok(Reading::Absent);
    };
    let before = before.state(was, format)?;
    if let Some(noted) = unparseable(&now, before.as_ref(), manifest, hook)? {
        return Ok(noted);
    }
    let before = before.unwrap_or_default();
    if let Some(noted) = unreadable(&now, &before, manifest) {
        return Ok(noted);
    }
    if now.lockfile.is_none() && before.lockfile.is_none() {
        return Ok(Reading::Noted(
            manifest.to_string(),
            format!(
                "{manifest} has no lockfile beside it in either tree, so its dependencies are \
                 not judged"
            ),
        ));
    }
    Ok(Reading::Judged(now, before))
}

/// The NOTE for a lockfile klin found and cannot read, filed under the manifest it leaves unjudged.
/// Today's lockfile is named when it is the one klin cannot read, and otherwise the base's, at the
/// path the base held it at, which today's tree may no longer hold.
fn unreadable(now: &State, before: &State, manifest: &str) -> Option<Reading> {
    let (at, tree) = match (&now.unreadable, &before.unreadable) {
        (Some(at), _) => (at, ""),
        (None, Some(at)) => (at, " at the base"),
        (None, None) => return None,
    };
    Some(Reading::Noted(
        manifest.to_string(),
        format!(
            "{at}{tree} is a lockfile format klin cannot read yet, so the dependencies of \
             {manifest} are not judged"
        ),
    ))
}

/// A manifest klin cannot parse now is a tool error when it parsed at the base, because the work
/// broke it. One the base did not hold is a tool error outside the hook, because the work added
/// the hole, and a NOTE in the hook, where the agent cannot edit `except`. One that did not parse
/// at the base either is a fixture, and a NOTE. Spec 8.2.1, 8.6.
fn unparseable(
    now: &State,
    before: Option<&State>,
    manifest: &str,
    hook: bool,
) -> Result<Option<Reading>, Error> {
    let Some(why) = &now.unparsed else {
        return Ok(None);
    };
    match before.map(|before| before.unparsed.is_none()) {
        Some(true) => Err(Error(why.clone())),
        None if !hook => Err(Error(format!(
            "{why}, and the base did not hold {manifest}. Make it parse, or add it to the \
             section's `except` if it is invalid on purpose"
        ))),
        _ => Ok(Some(Reading::Noted(
            manifest.to_string(),
            format!("{why}, so the dependencies of {manifest} are not judged"),
        ))),
    }
}

/// Every value of every dependency, taken against the names and versions the lockfile holds.
fn taken(deps: Vec<Dependency>, locked: &Locked, importer: &str) -> BTreeMap<String, Values> {
    deps.into_iter()
        .map(|dep| {
            let unlocked = dep.registry && !locked.names.contains(&dep.package);
            let stale = dep.registry
                && !unlocked
                && !dep.pins.is_empty()
                && !locked
                    .versions(importer, &dep.package)
                    .is_some_and(|versions| pinned(&dep.pins, versions));
            (
                dep.name,
                values(unlocked, dep.registry && !dep.exact, stale),
            )
        })
        .collect()
}

fn pinned(pins: &[String], versions: &HashSet<String>) -> bool {
    pins.iter()
        .all(|pin| versions.iter().any(|version| holds(pin, version)))
}

fn holds(pin: &str, version: &str) -> bool {
    let partial =
        pin.split('.').count() < 3 && pin.chars().all(|at| at.is_ascii_digit() || at == '.');
    version == pin
        || version.starts_with("link:")
        || version.starts_with("file:")
        || (partial
            && !version.contains('-')
            && version
                .strip_prefix(pin)
                .is_some_and(|rest| rest.starts_with('.')))
}

fn values(unlocked: bool, unpinned: bool, stale: bool) -> Values {
    let mut out = Values::new();
    out.insert(UNLOCKED.into(), u64::from(unlocked).into());
    out.insert(UNPINNED.into(), u64::from(unpinned).into());
    out.insert(STALE.into(), u64::from(stale).into());
    out
}

/// The nearest lockfile of these names at or above the manifest's own directory, which is where
/// a workspace keeps the one lockfile its members share.
fn beside(bytes: &HashMap<String, Vec<u8>>, manifest: &str, names: &[&str]) -> Option<String> {
    let mut at = parent(manifest);
    loop {
        if let Some(found) = names
            .iter()
            .map(|name| joined(&at, name))
            .find(|path| bytes.contains_key(path))
        {
            return Some(found);
        }
        if at.is_empty() {
            return None;
        }
        at = parent(&at);
    }
}

/// Every path a lockfile of one name could sit at for this manifest, nearest first.
fn nearest(manifest: &str, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = parent(manifest);
    loop {
        out.push(joined(&at, name));
        if at.is_empty() {
            return out;
        }
        at = parent(&at);
    }
}

fn relative(dir: &str, root: &str) -> String {
    match root.is_empty() {
        true => dir.to_string(),
        false => dir
            .strip_prefix(root)
            .map_or(dir, |rest| rest.trim_start_matches('/'))
            .to_string(),
    }
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or(String::new(), |(up, _)| up.to_string())
}

fn joined(at: &str, name: &str) -> String {
    match at.is_empty() {
        true => name.to_string(),
        false => format!("{at}/{name}"),
    }
}

const CARGO_TABLES: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

/// Where a line of a `Cargo.toml` sits: in a dependency table, in the sub-table of one named
/// dependency, or somewhere this check reads nothing from.
#[derive(Default)]
enum At {
    Table,
    Named(String),
    #[default]
    Elsewhere,
}

/// A line scan of one `Cargo.toml`: where it is, how deep in an inline table or an array, and
/// the dependency whose inline table is still open, whose fields the lines below it state.
#[derive(Default)]
struct Cargo {
    found: BTreeMap<String, Dependency>,
    at: At,
    depth: i32,
    inline: Option<String>,
}

impl Cargo {
    fn line(&mut self, line: &str) {
        let line = uncommented(line).trim();
        if line.is_empty() {
            return;
        }
        self.read(line);
        self.depth = (self.depth + nesting(line)).max(0);
        self.inline = match self.depth > 0 {
            true => self.inline.take().or_else(|| opened(&self.at, line)),
            false => None,
        };
    }

    fn read(&mut self, line: &str) {
        if let Some(name) = self.inline.clone() {
            field_line(&name, line, &mut self.found);
        } else if line.starts_with('[') {
            self.at = cargo_header(line);
        } else {
            cargo_entry(&self.at, line, &mut self.found);
        }
    }
}

/// Every dependency the four tables of spec 8.2.1 name, read by a line scan.
fn cargo_dependencies(_at: &str, bytes: &[u8]) -> Result<Vec<Dependency>, Error> {
    let text = String::from_utf8_lossy(bytes);
    let mut scan = Cargo::default();
    for line in text.lines() {
        scan.line(line);
    }
    Ok(scan.found.into_values().collect())
}

/// A line with its comment cut off, so a brace a person wrote in a comment opens no inline
/// table and hides no dependency below it.
fn uncommented(line: &str) -> &str {
    let mut quoted = false;
    for (at, character) in line.char_indices() {
        match character {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..at],
            _ => (),
        }
    }
    line
}

fn nesting(line: &str) -> i32 {
    let opened = line.chars().filter(|at| *at == '{' || *at == '[').count() as i32;
    let closed = line.chars().filter(|at| *at == '}' || *at == ']').count() as i32;
    opened - closed
}

/// The dependency whose inline table this line opened and did not close. The lines below it
/// state its fields, so none of them is a dependency of its own.
fn opened(at: &At, line: &str) -> Option<String> {
    match at {
        At::Table => Some(unquoted(key_of(line)?).to_string()),
        At::Named(name) => Some(name.clone()),
        At::Elsewhere => None,
    }
}

fn key_of(line: &str) -> Option<&str> {
    let (key, _) = line.split_once('=')?;
    key.trim().split('.').next()
}

/// A table header, keyed by its last segment, so `target.'cfg(unix)'.dependencies` is a
/// dependency table and `dependencies.serde` is one dependency's own sub-table.
fn cargo_header(line: &str) -> At {
    let inner = line.trim_matches(['[', ']']);
    let mut segments = inner.rsplit('.');
    let last = segments.next().unwrap_or_default();
    let above = segments.next().unwrap_or_default();
    if CARGO_TABLES.contains(&last) {
        return At::Table;
    }
    if CARGO_TABLES.contains(&above) {
        return At::Named(unquoted(last).to_string());
    }
    At::Elsewhere
}

fn cargo_entry(at: &At, line: &str, found: &mut BTreeMap<String, Dependency>) {
    let Some((key, value)) = line.split_once('=') else {
        return;
    };
    let (key, value) = (key.trim(), value.trim());
    match at {
        At::Table => cargo_dependency(key, value, found),
        At::Named(name) => cargo_field(name, key, value, found),
        At::Elsewhere => (),
    }
}

/// One line of a dependency table, in either of the two shapes it takes: a dotted key, which
/// states one field, and a whole entry, which states the requirement or an inline table.
fn cargo_dependency(key: &str, value: &str, found: &mut BTreeMap<String, Dependency>) {
    match key.split_once('.') {
        Some((name, field)) => cargo_field(unquoted(name), field, value, found),
        None => cargo_whole(unquoted(key), value, found),
    }
}

fn cargo_whole(name: &str, value: &str, found: &mut BTreeMap<String, Dependency>) {
    if let Some(fields) = value.strip_prefix('{') {
        for field in fields.trim_end_matches('}').split(',') {
            field_line(name, field, found);
        }
        return;
    }
    if let Some(entry) = entry_of(name, found) {
        entry.record_requirement(unquoted(value));
    }
}

/// One field of a dependency, wherever it was written: a sub-table, a dotted key or an inline
/// table on one line or several.
fn cargo_field(name: &str, key: &str, value: &str, found: &mut BTreeMap<String, Dependency>) {
    let Some(entry) = entry_of(name, found) else {
        return;
    };
    match key {
        "version" => entry.record_requirement(unquoted(value)),
        "package" => entry.package = unquoted(value).to_string(),
        "path" | "git" | "workspace" => entry.registry = false,
        _ => (),
    }
}

fn field_line(name: &str, line: &str, found: &mut BTreeMap<String, Dependency>) {
    if let Some((key, value)) = line.split_once('=') {
        cargo_field(name, key.trim(), value.trim(), found);
    }
}

fn entry_of<'a>(
    name: &str,
    found: &'a mut BTreeMap<String, Dependency>,
) -> Option<&'a mut Dependency> {
    if name.is_empty() {
        return None;
    }
    Some(
        found
            .entry(name.to_string())
            .or_insert_with(|| Dependency::named(name)),
    )
}

fn unquoted(text: &str) -> &str {
    text.trim()
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap_or_default()
}

/// Every package a `Cargo.lock` holds: the `name` and `version` of each `[[package]]` block, in
/// whichever order the block states them.
fn cargo_locked(_at: &str, bytes: &[u8]) -> Result<Option<Vec<Entry>>, Error> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    let mut package: Option<(Option<&str>, Option<&str>)> = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            out.extend(package.take().and_then(cargo_package));
            package = (line == "[[package]]").then_some((None, None));
        } else if let Some((name, version)) = &mut package {
            *name = toml_value(line, "name").or(*name);
            *version = toml_value(line, "version").or(*version);
        }
    }
    out.extend(package.and_then(cargo_package));
    Ok(Some(out))
}

fn cargo_package((name, version): (Option<&str>, Option<&str>)) -> Option<Entry> {
    Some(Entry::pooled(name?, version.map(str::to_string)))
}

fn toml_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let value = line.strip_prefix(key)?.trim().strip_prefix('=')?;
    Some(unquoted(value))
}

const PNPM_TABLES: &[&str] = &["dependencies", "devDependencies", "optionalDependencies"];

const NPM_TABLES: &[&str] = &[
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

fn npm_dependencies(at: &str, bytes: &[u8]) -> Result<Vec<Dependency>, Error> {
    let data = json(at, bytes)?;
    let mut found: BTreeMap<String, Dependency> = BTreeMap::new();
    for table in NPM_TABLES {
        let Some(named) = data.get(table).and_then(Value::as_object) else {
            continue;
        };
        for (name, held) in named {
            let spec = held.as_str().unwrap_or_default();
            found.entry(name.clone()).or_insert_with(|| Dependency {
                registry: npm_registry(spec),
                exact: npm_exact(spec),
                pins: npm_exact(spec)
                    .then(|| spec.to_string())
                    .into_iter()
                    .collect(),
                ..Dependency::named(name)
            });
        }
    }
    Ok(found.into_values().collect())
}

/// Whether an npm specifier names a registry version at all. A `file:`, `link:`, `workspace:`,
/// git or GitHub specifier resolves to no registry entry.
fn npm_registry(spec: &str) -> bool {
    !(spec.contains(':') || spec.contains('/') || spec.starts_with('.'))
}

/// Whether an npm specifier is one version and not a range: a bare version, with no operator
/// and no wildcard segment.
fn npm_exact(spec: &str) -> bool {
    spec.starts_with(|first: char| first.is_ascii_digit())
        && !spec.contains(['^', '~', '>', '<', '=', '*', '|', ' '])
        && !spec
            .split(['.', '-'])
            .any(|part| part == "x" || part == "X")
}

/// Read the npm-family lockfile named by `at`; `None` means its line-scanned shape is unknown.
fn npm_locked(at: &str, bytes: &[u8]) -> Result<Option<Vec<Entry>>, Error> {
    match basename(at) {
        "package-lock.json" => package_locked(at, bytes).map(Some),
        "pnpm-lock.yaml" => Ok(pnpm_locked(bytes)),
        "yarn.lock" => Ok(yarn_locked(bytes)),
        _ => unreachable!("unsupported npm lockfile: {at}"),
    }
}

/// Every package name a `package-lock.json` holds, in both lockfile shapes: the keys of
/// `packages` with everything up to the last `node_modules/` stripped, and the keys of the
/// nested `dependencies` tree that version 1 writes. An entry that sits in no other package's
/// `node_modules` is what the manifest in the directory above it resolves the name to, and a
/// link takes the version of the package it links.
fn package_locked(at: &str, bytes: &[u8]) -> Result<Vec<Entry>, Error> {
    let data = json(at, bytes)?;
    let mut out = Vec::new();
    if let Some(packages) = data.get("packages").and_then(Value::as_object) {
        out.extend(packages.iter().filter_map(|(key, entry)| {
            let (above, name) = key.rsplit_once("node_modules/")?;
            Some(match above.contains("node_modules/") {
                true => Entry::pooled(name, None),
                false => Entry::resolved(
                    name,
                    npm_version(entry, packages),
                    above.trim_end_matches('/'),
                ),
            })
        }));
    }
    npm_nested(data.get("dependencies"), Some(""), &mut out);
    Ok(out)
}

fn npm_version(entry: &Value, packages: &serde_json::Map<String, Value>) -> Option<String> {
    let version = |entry: &Value| {
        entry
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    if entry.get("link").and_then(Value::as_bool) != Some(true) {
        return version(entry);
    }
    let target = entry.get("resolved").and_then(Value::as_str)?;
    Some(
        packages
            .get(target)
            .and_then(version)
            .unwrap_or(format!("link:{target}")),
    )
}

/// Package keys in the `packages` mapping of the pnpm lockfile versions klin recognizes, and
/// what each importer resolves its dependencies to.
fn pnpm_locked(bytes: &[u8]) -> Option<Vec<Entry>> {
    let text = String::from_utf8_lossy(bytes);
    let version = pnpm_version(&text)?;
    let mut packages = pnpm_packages(&text)?;
    packages.extend(pnpm_importers(&text));
    version.then_some(packages)
}

fn pnpm_importers(text: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut path: Vec<(usize, String)> = Vec::new();
    for line in text.lines().filter(|line| !yarn_blank(line.trim())) {
        let indent = line.len() - line.trim_start().len();
        while path.last().is_some_and(|(at, _)| *at >= indent) {
            path.pop();
        }
        let (key, value) = yaml_pair(line.trim());
        let keys: Vec<&str> = path.iter().map(|(_, key)| key.as_str()).collect();
        if let Some(importer) = pnpm_table(&keys)
            && !value.is_empty()
        {
            out.push(Entry::resolved(key, Some(pnpm_resolved(value)), &importer));
        } else if key == "version"
            && let Some((name, above)) = keys.split_last()
            && let Some(importer) = pnpm_table(above)
        {
            out.push(Entry::resolved(name, Some(pnpm_resolved(value)), &importer));
        }
        path.push((indent, key.to_string()));
    }
    out
}

fn pnpm_table(keys: &[&str]) -> Option<String> {
    match keys {
        [table] if PNPM_TABLES.contains(table) => Some(String::new()),
        ["importers", importer, table] if PNPM_TABLES.contains(table) => Some(
            importer
                .trim_start_matches('.')
                .trim_start_matches('/')
                .to_string(),
        ),
        _ => None,
    }
}

fn pnpm_resolved(value: &str) -> String {
    if value.starts_with("link:") || value.starts_with("file:") {
        return value.to_string();
    }
    let version = value.split(['(', '_']).next().unwrap_or(value);
    let version = version.rsplit_once('@').map_or(version, |(_, at)| at);
    version
        .rsplit_once('/')
        .map_or(version, |(_, at)| at)
        .to_string()
}

fn yaml_pair(line: &str) -> (&str, &str) {
    match line.split_once(": ") {
        Some((key, value)) => (yaml_text(key), yaml_text(value)),
        None => (yaml_text(line.trim_end_matches(':')), ""),
    }
}

fn pnpm_version(text: &str) -> Option<bool> {
    text.lines().find_map(pnpm_version_line)
}

fn pnpm_version_line(line: &str) -> Option<bool> {
    line.strip_prefix("lockfileVersion:")
        .map(pnpm_version_value)
}

fn pnpm_version_value(value: &str) -> bool {
    yaml_text(value)
        .split('.')
        .next()
        .and_then(|major| major.parse::<u64>().ok())
        .is_some_and(|major| major >= 4)
}

fn pnpm_packages(text: &str) -> Option<Vec<Entry>> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if pnpm_packages_header(line) {
            return pnpm_package_lines(lines);
        }
    }
    None
}

fn pnpm_packages_header(line: &str) -> bool {
    let Some(value) = line.strip_prefix("packages:") else {
        return false;
    };
    matches!(value.trim(), "" | "{}")
}

fn pnpm_package_lines(lines: std::str::Lines<'_>) -> Option<Vec<Entry>> {
    let mut package_indent = None;
    let mut out = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            break;
        }
        let at = package_indent.get_or_insert(indent);
        if indent == *at {
            out.push(pnpm_entry(yaml_key(trimmed)?)?);
        }
    }
    Some(out)
}

fn pnpm_entry(key: &str) -> Option<Entry> {
    let key = yaml_text(key)
        .split_once('(')
        .map_or(yaml_text(key), |(name, _)| name)
        .trim_start_matches('/');
    let (name, version) = pnpm_path(key)
        .or_else(|| {
            key.rsplit_once('@')
                .filter(|(name, version)| !name.is_empty() && !version.is_empty())
        })
        .or_else(|| {
            key.rsplit_once('/')
                .filter(|(_, version)| !version.is_empty())
        })?;
    let name = name.strip_prefix("registry.npmjs.org/").unwrap_or(name);
    Some(Entry::pooled(name, Some(version.to_string())))
}

fn pnpm_path(key: &str) -> Option<(&str, &str)> {
    let (name, rest) = key.rsplit_once('/')?;
    let version = rest.split('_').next()?;
    (version.starts_with(|at: char| at.is_ascii_digit()) && !version.contains('@'))
        .then_some((name, version))
}

/// Yarn classic has top-level selectors after its v1 marker; Yarn Berry has top-level locators
/// after a `__metadata` mapping whose version is at least 4. Each gives its package names from
/// those keys and its version from the `version` line below them.
fn yarn_locked(bytes: &[u8]) -> Option<Vec<Entry>> {
    let text = String::from_utf8_lossy(bytes);
    if text.lines().any(|line| line.trim() == "# yarn lockfile v1") {
        yarn_entries(text.lines(), false)
    } else {
        yarn_berry(&text)
    }
}

fn yarn_entries(lines: std::str::Lines<'_>, strict: bool) -> Option<Vec<Entry>> {
    let mut out = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for line in lines.filter(|line| !yarn_blank(line.trim())) {
        let trimmed = line.trim();
        if !line.starts_with(char::is_whitespace) {
            names = yarn_top(trimmed, strict)?;
            out.extend(names.iter().map(|name| Entry::pooled(name, None)));
        } else if let Some(version) = yarn_recorded(trimmed) {
            out.extend(
                names
                    .iter()
                    .map(|name| Entry::pooled(name, Some(version.clone()))),
            );
        }
    }
    Some(out)
}

fn yarn_blank(trimmed: &str) -> bool {
    trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---"
}

fn yarn_top(trimmed: &str, strict: bool) -> Option<Vec<String>> {
    match yaml_key(trimmed).and_then(yarn_names) {
        Some(found) => Some(found),
        None if strict => None,
        None => Some(Vec::new()),
    }
}

fn yarn_recorded(line: &str) -> Option<String> {
    let rest = line.strip_prefix("version")?;
    let rest = rest.strip_prefix(':').unwrap_or(rest);
    rest.starts_with(' ').then(|| yaml_text(rest).to_string())
}

fn yarn_berry(text: &str) -> Option<Vec<Entry>> {
    let version = yarn_metadata_version(text)?;
    let locators = yarn_locators(text)?;
    version.then_some(locators)
}

fn yarn_metadata_version(text: &str) -> Option<bool> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if yarn_metadata_header(line) {
            return yarn_version(lines);
        }
    }
    None
}

fn yarn_version(lines: std::str::Lines<'_>) -> Option<bool> {
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.len() == line.trim_start().len() {
            return None;
        }
        if let Some(value) = trimmed.strip_prefix("version:") {
            return Some(
                yaml_text(value)
                    .parse::<u64>()
                    .ok()
                    .is_some_and(|at| at >= 4),
            );
        }
    }
    None
}

fn yarn_metadata_header(line: &str) -> bool {
    line.len() == line.trim_start().len() && yaml_key(line.trim()) == Some("__metadata")
}

fn yarn_locators(text: &str) -> Option<Vec<Entry>> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if yarn_metadata_header(line) {
            return yarn_entries(lines, true);
        }
    }
    None
}

fn yarn_names(key: &str) -> Option<Vec<String>> {
    let names = yaml_text(key)
        .split(',')
        .map(yarn_name)
        .collect::<Option<Vec<_>>>()?;
    (!names.is_empty()).then_some(names)
}

fn yarn_name(selector: &str) -> Option<String> {
    let selector = yaml_text(selector);
    let at = if selector.starts_with('@') {
        let slash = selector.find('/')?;
        slash + 1 + selector[slash + 1..].find('@')?
    } else {
        selector.find('@')?
    };
    let name = &selector[..at];
    (!name.is_empty() && !selector[at + 1..].is_empty()).then(|| name.to_string())
}

fn yaml_key(line: &str) -> Option<&str> {
    line.rsplit_once(':')
        .map(|(key, _)| key.trim())
        .filter(|key| !key.is_empty())
}

fn yaml_text(text: &str) -> &str {
    text.trim().trim_matches(['\'', '"'])
}

fn npm_nested(held: Option<&Value>, importer: Option<&str>, out: &mut Vec<Entry>) {
    let Some(named) = held.and_then(Value::as_object) else {
        return;
    };
    for (name, entry) in named {
        out.push(match importer {
            Some(importer) => Entry::resolved(
                name,
                entry
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                importer,
            ),
            None => Entry::pooled(name, None),
        });
        npm_nested(entry.get("dependencies"), None, out);
    }
}

fn json(at: &str, bytes: &[u8]) -> Result<Value, Error> {
    serde_json::from_slice(bytes).map_err(|why| Error(format!("{at} is not valid JSON: {why}")))
}

/// Every module a `go.mod` requires, in both forms of the directive. A `require` states one
/// version and never a range, so a Go dependency is always pinned. A module a `replace`
/// directive points at a local path has no registry behind it.
fn go_dependencies(_at: &str, bytes: &[u8]) -> Result<Vec<Dependency>, Error> {
    let text = String::from_utf8_lossy(bytes);
    let replaced = go_replaced(&text);
    let mut found: BTreeMap<String, Dependency> = BTreeMap::new();
    let mut inside = false;
    for line in text.lines() {
        let line = bare(line);
        if inside {
            inside = !line.starts_with(')');
            go_require(line, &replaced, &mut found);
        } else if let Some(rest) = line.strip_prefix("require").map(str::trim) {
            inside = rest == "(";
            go_require(rest, &replaced, &mut found);
        }
    }
    Ok(found.into_values().collect())
}

fn go_require(line: &str, replaced: &[Replace], found: &mut BTreeMap<String, Dependency>) {
    let mut fields = line.split_whitespace();
    let Some(name) = fields.next() else {
        return;
    };
    if name == "(" || name == ")" {
        return;
    }
    let version = fields.next();
    let dependency = match replaced.iter().find(|held| held.applies(name, version)) {
        None => Dependency {
            pins: version.map(str::to_string).into_iter().collect(),
            ..Dependency::named(name)
        },
        Some(held) if held.local() => Dependency {
            registry: false,
            ..Dependency::named(name)
        },
        Some(held) => Dependency {
            package: held.target.clone(),
            pins: held.target_version.clone().into_iter().collect(),
            ..Dependency::named(name)
        },
    };
    found.insert(name.to_string(), dependency);
}

struct Replace {
    module: String,
    version: Option<String>,
    target: String,
    target_version: Option<String>,
}

impl Replace {
    fn applies(&self, module: &str, version: Option<&str>) -> bool {
        self.module == module
            && self
                .version
                .as_deref()
                .is_none_or(|held| Some(held) == version)
    }

    fn local(&self) -> bool {
        self.target.starts_with('.') || self.target.starts_with('/')
    }
}

/// Every `replace` directive, in both forms: the module and the version it replaces, and the
/// module or local path and the version it sends them to.
fn go_replaced(text: &str) -> Vec<Replace> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = bare(line).trim_start_matches("replace").trim();
        let Some((from, to)) = line.split_once("=>") else {
            continue;
        };
        let (mut from, mut to) = (from.split_whitespace(), to.split_whitespace());
        if let (Some(module), Some(target)) = (from.next(), to.next()) {
            out.push(Replace {
                module: module.to_string(),
                version: from.next().map(str::to_string),
                target: target.to_string(),
                target_version: to.next().map(str::to_string),
            });
        }
    }
    out
}

/// Every module a `go.sum` holds, with each version it has a line for: the first two fields of
/// each line, the `/go.mod` suffix cut off the version.
fn go_locked(_at: &str, bytes: &[u8]) -> Result<Option<Vec<Entry>>, Error> {
    let text = String::from_utf8_lossy(bytes);
    Ok(Some(
        text.lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let name = fields.next()?;
                let version = fields
                    .next()
                    .map(|at| at.trim_end_matches("/go.mod").to_string());
                Some(Entry::pooled(name, version))
            })
            .collect(),
    ))
}

fn bare(line: &str) -> &str {
    match line.find("//") {
        Some(at) => line[..at].trim(),
        None => line.trim(),
    }
}
