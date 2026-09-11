use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::config::{Config, Error, Flags};
use crate::coverage::Coverage;
use crate::ratchet::{self, Evaluator, Finding, Section, Values};
use crate::reference::{self, Key};
use crate::{base, changed, files};

pub const SECTION: &str = "lockfile";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[MANIFESTS, reference::EXCLUDE];

/// The key of this section the survey supplies, named off the declaration so the two cannot
/// spell it differently. It is the key below that carries a rule, restated because the survey
/// merges a section key by key. Spec 5.4.
pub const DERIVED: &[&str] = &[MANIFESTS.name];

pub const MANIFESTS: Key = Key {
    name: "manifests",
    holds: "the manifest files this check proves against their lockfiles",
    required: true,
    rule: Some(
        "one entry per manifest klin has a lockfile reader for: `Cargo.toml`, `package.json` \
                and `go.mod`",
    ),
    default: "",
};
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

/// One ecosystem: the manifest klin reads, the lockfiles it can read beside it, the lockfiles it
/// cannot read yet, and the two readers. Spec 8.2.1.
struct Format {
    manifest: &'static str,
    lockfiles: &'static [&'static str],
    unreadable: &'static [&'static str],
    dependencies: fn(&str, &[u8]) -> Result<Vec<Dependency>, Error>,
    locked: fn(&str, &[u8]) -> Result<Vec<String>, Error>,
}

const FORMATS: &[Format] = &[
    Format {
        manifest: "Cargo.toml",
        lockfiles: &["Cargo.lock"],
        unreadable: &[],
        dependencies: cargo_dependencies,
        locked: cargo_locked,
    },
    Format {
        manifest: "package.json",
        lockfiles: &["package-lock.json"],
        unreadable: &["pnpm-lock.yaml", "yarn.lock"],
        dependencies: npm_dependencies,
        locked: npm_locked,
    },
    Format {
        manifest: "go.mod",
        lockfiles: &["go.sum"],
        unreadable: &[],
        dependencies: go_dependencies,
        locked: go_locked,
    },
];

/// Whether klin has a reader for the manifest with this basename, which is what the survey
/// writes a `manifests` list out of.
pub fn reads(name: &str) -> bool {
    FORMATS.iter().any(|format| format.manifest == name)
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::open(flags, start)?;
    config.say(flags, SECTION, out);
    let sites = surveyed(&config, flags, out)?;
    let accepted = ratchet::accepted(&config, &flags.gate, METRICS)?;
    let ok = format!(
        "OK: {} dependenc{} in {} manifest(s), each locked and pinned as the base had it{}",
        sites.judged,
        plural(sites.judged),
        sites.manifests,
        sites.coverage().said(flags)
    );
    let code = evaluator().evaluate(sites.findings, sites.prior, accepted, flags, &ok, out);
    ratchet::noted(&sites.notes, flags, out);
    Ok(code)
}

/// Every manifest the section names, read in both trees, minus the ones `exclude` drops.
fn surveyed(config: &Config, flags: &Flags, out: &mut String) -> Result<Sites, Error> {
    let section = ratchet::section(config, SECTION)?;
    let manifests = listed(&section, MANIFESTS)?;
    let exclude = optional(&section, reference::EXCLUDE)?;
    let commit = base::commit(config.root(), flags, out)?;
    let mut sites = Sites::default();
    let (dropped, judged): (Vec<&String>, Vec<&String>) =
        manifests.iter().partition(|path| excluded(path, &exclude));
    sites.listed = manifests.len();
    sites.excluded = dropped.len();
    for manifest in judged {
        sites.add(config.root(), &commit, manifest)?;
    }
    Ok(sites)
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
    /// the section names, how many of those an exclusion dropped, and how many the working
    /// tree no longer holds, which it discovers nothing of. A manifest it reached and did not
    /// read left a NOTE instead, and the coverage calls that one unreadable. Spec 8.6.
    listed: usize,
    excluded: usize,
    absent: usize,
}

impl Sites {
    fn coverage(&self) -> Coverage {
        Coverage {
            found: self.listed - self.absent,
            measured: self.manifests,
            excluded: self.excluded,
            unreadable: self.notes.len(),
        }
    }

    fn add(&mut self, root: &Path, commit: &str, manifest: &str) -> Result<(), Error> {
        let (now, before) = match reading(root, commit, manifest)? {
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
/// lockfile klin read them against, and a lockfile klin found and cannot read.
#[derive(Default)]
struct State {
    deps: BTreeMap<String, Values>,
    lockfile: Option<String>,
    unreadable: Option<String>,
}

/// Whether klin judges this manifest, and the NOTE that says why not. A manifest the working
/// tree no longer holds has no dependencies to judge and says nothing.
enum Reading {
    Judged(State, State),
    Noted(String, String),
    Absent,
}

fn reading(root: &Path, commit: &str, manifest: &str) -> Result<Reading, Error> {
    let Some(format) = FORMATS
        .iter()
        .find(|format| format.manifest == basename(manifest))
    else {
        return Ok(Reading::Noted(
            manifest.to_string(),
            format!(
                "{manifest} is a manifest klin has no lockfile reader for, so nothing it names \
                 is judged"
            ),
        ));
    };
    let Some(now) = state(
        &|path| std::fs::read(root.join(path)).ok(),
        manifest,
        format,
    )?
    else {
        return Ok(Reading::Absent);
    };
    let before =
        state(&|path| changed::blob(root, commit, path), manifest, format)?.unwrap_or_default();
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

fn state(
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
    manifest: &str,
    format: &Format,
) -> Result<Option<State>, Error> {
    let Some(text) = read(manifest) else {
        return Ok(None);
    };
    let found = beside(read, manifest, format.lockfiles);
    let locked = match &found {
        Some((at, bytes)) => (format.locked)(at, bytes)?,
        None => Vec::new(),
    };
    Ok(Some(State {
        deps: taken((format.dependencies)(manifest, &text)?, &locked),
        unreadable: unreadable(read, manifest, format, found.is_some()),
        lockfile: found.map(|(at, _)| at),
    }))
}

/// Both values of every dependency, taken against the names the lockfile holds.
fn taken(deps: Vec<Dependency>, locked: &[String]) -> BTreeMap<String, Values> {
    deps.into_iter()
        .map(|dep| {
            let unlocked = dep.registry && !locked.contains(&dep.package);
            (dep.name, values(unlocked, dep.registry && !dep.exact))
        })
        .collect()
}

/// The lockfile klin found and cannot read yet, and nothing when it read one.
fn unreadable(
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
    manifest: &str,
    format: &Format,
    found: bool,
) -> Option<String> {
    match found {
        true => None,
        false => beside(read, manifest, format.unreadable).map(|(at, _)| at),
    }
}

fn values(unlocked: bool, unpinned: bool) -> Values {
    let mut out = Values::new();
    out.insert(UNLOCKED.into(), u64::from(unlocked).into());
    out.insert(UNPINNED.into(), u64::from(unpinned).into());
    out
}

/// The nearest lockfile at or above the manifest's own directory, which is where a workspace
/// keeps the one lockfile its members share.
fn beside(
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
    manifest: &str,
    names: &[&str],
) -> Option<(String, Vec<u8>)> {
    let mut at = parent(manifest);
    loop {
        for name in names {
            let path = joined(&at, name);
            if let Some(bytes) = read(&path) {
                return Some((path, bytes));
            }
        }
        if at.is_empty() {
            return None;
        }
        at = parent(&at);
    }
}

/// A list the section may leave out, and an empty one when it does.
fn optional(section: &Section, key: Key) -> Result<Vec<String>, Error> {
    match section.values.get(key.name) {
        Some(_) => listed(section, key),
        None => Ok(Vec::new()),
    }
}

fn listed(section: &Section, key: Key) -> Result<Vec<String>, Error> {
    let held = section.values.get(key.name).and_then(Value::as_array);
    let Some(items) = held else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must name \"{}\", a list of paths",
            section.config.file.display(),
            key.name
        )));
    };
    items
        .iter()
        .map(|item| {
            item.as_str().map(str::to_string).ok_or_else(|| {
                section
                    .config
                    .malformed(SECTION, key.name, "a list of paths")
            })
        })
        .collect()
}

fn excluded(path: &str, globs: &[String]) -> bool {
    globs.iter().any(|glob| {
        files::glob_matches(glob.as_bytes(), path.as_bytes())
            || files::glob_matches(glob.as_bytes(), basename(path).as_bytes())
    })
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
fn cargo_locked(_at: &str, bytes: &[u8]) -> Result<Vec<String>, Error> {
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
    Ok(out)
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

/// Every package name a `package-lock.json` holds, in both lockfile shapes: the keys of
/// `packages` with everything up to the last `node_modules/` stripped, and the keys of the
/// nested `dependencies` tree that version 1 writes.
fn npm_locked(at: &str, bytes: &[u8]) -> Result<Vec<String>, Error> {
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
fn go_locked(_at: &str, bytes: &[u8]) -> Result<Vec<String>, Error> {
    let text = String::from_utf8_lossy(bytes);
    Ok(text
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect())
}

fn bare(line: &str) -> &str {
    match line.find("//") {
        Some(at) => line[..at].trim(),
        None => line.trim(),
    }
}
