//! `lockfile` proves that every dependency a manifest names has an entry in the lockfile beside
//! it, and that no pin the base held is gone. The manifests are the ones the survey finds that
//! klin has a reader for; a person narrows them only with `in` and `except`. A site is the
//! manifest's path and the dependency's name. Every manifest and lockfile is read once per tree,
//! and a lockfile several manifests share is parsed once. Spec 5.4, 8.2.1, ADR 0040.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::Value;

use crate::check::{self, Context, Sink};
use crate::config::Error;
use crate::coverage::Coverage;
use crate::project::Project;
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::reference::Key;
use crate::scope::{self, Scope};
use crate::{base, changed};

pub const SECTION: &str = "lockfile";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[scope::IN, scope::EXCEPT];

const RULE: &str = "the manifests the survey found that klin can read a lockfile for";
const UNLOCKED: &str = "unlocked";
const UNPINNED: &str = "unpinned";
const METRICS: &[&str] = &[UNLOCKED, UNPINNED];
const REMEDY: &str = "Run the project's own install so the lockfile records the dependency, and \
    give the specifier the base's exact version back. A dependency the lockfile does not know is \
    one no install has ever resolved, and a pin the base held is a version a person chose.";

/// One dependency a manifest names, before the lockfile beside it is read. `name` is what the
/// manifest calls it, which is the site's identity, and `package` is what the lockfile records
/// it under, which a Cargo rename makes different. A dependency with no registry behind it — a
/// path, a git or a workspace dependency — has no lockfile entry to want and no version to pin,
/// so it carries neither value. A statement only ever takes a value away, so two tables that
/// name one dependency give the same reading in either order.
struct Dependency {
    name: String,
    package: String,
    registry: bool,
    exact: bool,
}

impl Dependency {
    fn named(name: &str) -> Dependency {
        Dependency {
            name: name.to_string(),
            package: name.to_string(),
            registry: true,
            exact: true,
        }
    }
}

/// One ecosystem: the manifest klin reads, the lockfiles it can read beside it, and the two
/// readers. A lockfile reader returns `None` when its shape is not one it recognizes. Spec 8.2.1.
type DependencyReader = fn(&str, &[u8]) -> Result<Vec<Dependency>, Error>;
type LockfileReader = fn(&str, &[u8]) -> Result<Option<Vec<String>>, Error>;

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
    let accepted = ratchet::accepted(at.config(), at.gate, METRICS)?;
    let ok = format!(
        "OK: {} dependenc{} in {} manifest(s), each locked and pinned as the base had it{}",
        sites.judged,
        plural(sites.judged),
        sites.manifests,
        sites.coverage().said(out)
    );
    let code = evaluator().evaluate(sites.findings, sites.prior, accepted, at, &ok, out);
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
    let wanted = candidates(&judged);
    let mut now = Side::working(config.root(), &wanted);
    let mut before = Side::at(config.root(), &commit, &wanted)?;
    let mut sites = Sites {
        listed: found.len(),
        excluded: dropped.len(),
        ..Sites::default()
    };
    for (manifest, format) in judged {
        sites.add(&mut now, &mut before, manifest, format)?;
    }
    Ok(sites)
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

/// Every path a judged manifest could read: the manifest, and each lockfile name its format
/// knows at its own directory and at every directory above it.
fn candidates(judged: &[&(&str, &Format)]) -> Vec<String> {
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
/// every lockfile holds, parsed once however many manifests share it.
struct Side {
    bytes: HashMap<String, Vec<u8>>,
    locked: HashMap<String, Result<Option<HashSet<String>>, String>>,
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

    /// The same paths at the base commit, through one git process. A read git refuses is an
    /// error: an empty base would read every dependency as new.
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
        let none = HashSet::new();
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
        let (deps, unparsed) = match (format.dependencies)(manifest, text) {
            Ok(deps) => (taken(deps, names), None),
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

/// The names one lockfile holds, parsed on the first manifest that reads it and borrowed by
/// every manifest after, and the error a malformed one gives each of them.
fn locked<'a>(
    parsed: &'a mut HashMap<String, Result<Option<HashSet<String>>, String>>,
    bytes: &HashMap<String, Vec<u8>>,
    at: &str,
    format: &Format,
) -> Result<Option<&'a HashSet<String>>, Error> {
    let names = parsed.entry(at.to_string()).or_insert_with(|| {
        (format.locked)(at, &bytes[at])
            .map(|names| names.map(|names| names.into_iter().collect()))
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
        condition: "that the lockfile beside the manifest does not lock",
        fix_advice: REMEDY,
        ceiling: None,
        format_metrics: show,
    }
}

fn show(values: &Values) -> String {
    format!(
        "{UNLOCKED} {}, {UNPINNED} {}",
        number(values, UNLOCKED),
        number(values, UNPINNED)
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
        now: &mut Side,
        before: &mut Side,
        manifest: &str,
        format: &Format,
    ) -> Result<(), Error> {
        let (now, before) = match reading(now, before, manifest, format)? {
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
            if before.deps.contains_key(name) || number(values, UNLOCKED) > 0 {
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

fn finding(manifest: &str, name: &str, values: Values) -> Finding {
    Finding {
        file: manifest.to_string(),
        line: 0,
        text: name.to_string(),
        values,
        body: None,
    }
}

/// What one tree says about one manifest: its dependencies with both values already taken, the
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

fn reading(
    now: &mut Side,
    before: &mut Side,
    manifest: &str,
    format: &Format,
) -> Result<Reading, Error> {
    let Some(now) = now.state(manifest, format)? else {
        return Ok(Reading::Absent);
    };
    let before = before.state(manifest, format)?;
    if let Some(noted) = unparseable(&now, before.as_ref(), manifest)? {
        return Ok(noted);
    }
    let before = before.unwrap_or_default();
    if let Some(at) = now.unreadable.clone().or_else(|| before.unreadable.clone()) {
        return Ok(Reading::Noted(
            at.clone(),
            format!(
                "{at} is a lockfile format klin cannot read yet, so the dependencies of \
                 {manifest} are not judged"
            ),
        ));
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

/// A manifest klin cannot parse now is a tool error when it parsed at the base, because the work
/// broke it. One that never parsed is a fixture, and a NOTE. Spec 8.2.1.
fn unparseable(
    now: &State,
    before: Option<&State>,
    manifest: &str,
) -> Result<Option<Reading>, Error> {
    let Some(why) = &now.unparsed else {
        return Ok(None);
    };
    match before.is_some_and(|before| before.unparsed.is_none()) {
        true => Err(Error(why.clone())),
        false => Ok(Some(Reading::Noted(
            manifest.to_string(),
            format!("{why}, so the dependencies of {manifest} are not judged"),
        ))),
    }
}

/// Both values of every dependency, taken against the names the lockfile holds.
fn taken(deps: Vec<Dependency>, locked: &HashSet<String>) -> BTreeMap<String, Values> {
    deps.into_iter()
        .map(|dep| {
            let unlocked = dep.registry && !locked.contains(&dep.package);
            (dep.name, values(unlocked, dep.registry && !dep.exact))
        })
        .collect()
}

fn values(unlocked: bool, unpinned: bool) -> Values {
    let mut out = Values::new();
    out.insert(UNLOCKED.into(), u64::from(unlocked).into());
    out.insert(UNPINNED.into(), u64::from(unpinned).into());
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
        entry.exact &= unquoted(value).starts_with('=');
    }
}

/// One field of a dependency, wherever it was written: a sub-table, a dotted key or an inline
/// table on one line or several.
fn cargo_field(name: &str, key: &str, value: &str, found: &mut BTreeMap<String, Dependency>) {
    let Some(entry) = entry_of(name, found) else {
        return;
    };
    match key {
        "version" => entry.exact &= unquoted(value).starts_with('='),
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

/// Every package name a `Cargo.lock` holds: the `name` of each `[[package]]` block.
fn cargo_locked(_at: &str, bytes: &[u8]) -> Result<Option<Vec<String>>, Error> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    let mut package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            package = line == "[[package]]";
        } else if package
            && let Some(value) = line.strip_prefix("name").map(str::trim)
            && let Some(value) = value.strip_prefix('=')
        {
            out.push(unquoted(value).to_string());
        }
    }
    Ok(Some(out))
}

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
fn npm_locked(at: &str, bytes: &[u8]) -> Result<Option<Vec<String>>, Error> {
    match basename(at) {
        "package-lock.json" => package_locked(at, bytes).map(Some),
        "pnpm-lock.yaml" => Ok(pnpm_locked(bytes)),
        "yarn.lock" => Ok(yarn_locked(bytes)),
        _ => unreachable!("unsupported npm lockfile: {at}"),
    }
}

/// Every package name a `package-lock.json` holds, in both lockfile shapes: the keys of
/// `packages` with everything up to the last `node_modules/` stripped, and the keys of the
/// nested `dependencies` tree that version 1 writes.
fn package_locked(at: &str, bytes: &[u8]) -> Result<Vec<String>, Error> {
    let data = json(at, bytes)?;
    let mut out = Vec::new();
    if let Some(packages) = data.get("packages").and_then(Value::as_object) {
        out.extend(
            packages
                .keys()
                .filter_map(|key| key.rsplit_once("node_modules/"))
                .map(|(_, name)| name.to_string()),
        );
    }
    npm_nested(data.get("dependencies"), &mut out);
    Ok(out)
}

/// Package keys in the `packages` mapping of the pnpm lockfile versions klin recognizes.
fn pnpm_locked(bytes: &[u8]) -> Option<Vec<String>> {
    let text = String::from_utf8_lossy(bytes);
    let version = pnpm_version(&text)?;
    let packages = pnpm_packages(&text)?;
    version.then_some(packages)
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

fn pnpm_packages(text: &str) -> Option<Vec<String>> {
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

fn pnpm_package_lines(lines: std::str::Lines<'_>) -> Option<Vec<String>> {
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
            out.push(pnpm_name(yaml_key(trimmed)?)?);
        }
    }
    Some(out)
}

fn pnpm_name(key: &str) -> Option<String> {
    let key = yaml_text(key)
        .split_once('(')
        .map_or(yaml_text(key), |(name, _)| name)
        .trim_start_matches('/');
    if let Some((name, version)) = key.rsplit_once('@')
        && !name.is_empty()
        && !version.is_empty()
    {
        return Some(name.to_string());
    }
    let (name, version) = key.rsplit_once('/')?;
    if version.is_empty() {
        return None;
    }
    Some(
        name.strip_prefix("registry.npmjs.org/")
            .unwrap_or(name)
            .to_string(),
    )
}

/// Yarn classic has top-level selectors after its v1 marker; Yarn Berry has top-level locators
/// after a `__metadata` mapping whose version is at least 4. Both only need package names here.
fn yarn_locked(bytes: &[u8]) -> Option<Vec<String>> {
    let text = String::from_utf8_lossy(bytes);
    if text.lines().any(|line| line.trim() == "# yarn lockfile v1") {
        Some(yarn_classic(&text))
    } else {
        yarn_berry(&text)
    }
}

fn yarn_classic(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(yarn_classic_names)
        .flatten()
        .collect()
}

fn yarn_classic_names(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if line.len() != line.trim_start().len() {
        return None;
    }
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" {
        return None;
    }
    yarn_names(yaml_key(trimmed)?)
}

fn yarn_berry(text: &str) -> Option<Vec<String>> {
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

fn yarn_locators(text: &str) -> Option<Vec<String>> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if yarn_metadata_header(line) {
            return yarn_locator_lines(lines);
        }
    }
    None
}

fn yarn_locator_lines(lines: std::str::Lines<'_>) -> Option<Vec<String>> {
    let mut out = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" {
            continue;
        }
        if line.len() == line.trim_start().len() {
            out.extend(yarn_names(yaml_key(trimmed)?)?);
        }
    }
    Some(out)
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

fn npm_nested(held: Option<&Value>, out: &mut Vec<String>) {
    let Some(named) = held.and_then(Value::as_object) else {
        return;
    };
    for (name, entry) in named {
        out.push(name.clone());
        npm_nested(entry.get("dependencies"), out);
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
    let local = go_replaced(&text);
    let mut found: BTreeMap<String, Dependency> = BTreeMap::new();
    let mut inside = false;
    for line in text.lines() {
        let line = bare(line);
        if inside {
            inside = !line.starts_with(')');
            go_require(line, &local, &mut found);
        } else if let Some(rest) = line.strip_prefix("require").map(str::trim) {
            inside = rest == "(";
            go_require(rest, &local, &mut found);
        }
    }
    Ok(found.into_values().collect())
}

fn go_require(line: &str, local: &[String], found: &mut BTreeMap<String, Dependency>) {
    let Some(name) = line.split_whitespace().next() else {
        return;
    };
    if name == "(" || name == ")" {
        return;
    }
    found.insert(
        name.to_string(),
        Dependency {
            registry: !local.iter().any(|held| held == name),
            ..Dependency::named(name)
        },
    );
}

/// The modules a `replace` directive sends to a local path, in both forms of the directive.
fn go_replaced(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = bare(line).trim_start_matches("replace").trim();
        let Some((from, to)) = line.split_once("=>") else {
            continue;
        };
        if let Some(name) = from.split_whitespace().next()
            && (to.trim().starts_with('.') || to.trim().starts_with('/'))
        {
            out.push(name.to_string());
        }
    }
    out
}

/// Every module a `go.sum` holds: the first field of each line.
fn go_locked(_at: &str, bytes: &[u8]) -> Result<Option<Vec<String>>, Error> {
    let text = String::from_utf8_lossy(bytes);
    Ok(Some(
        text.lines()
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_string)
            .collect(),
    ))
}

fn bare(line: &str) -> &str {
    match line.find("//") {
        Some(at) => line[..at].trim(),
        None => line.trim(),
    }
}
