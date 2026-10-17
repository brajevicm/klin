//! Direct TypeScript paths rules, compiled once from this tree's held configuration files.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::resolver::{Topology, directory, joined};
use crate::sys::jsonc;

pub(super) struct Paths {
    /// One per held config, compiled once and shared by every scope it owns.
    programs: Vec<Program>,
    scopes: HashMap<String, Scope>,
}

struct Config {
    file: String,
    directory: String,
    reached: BTreeSet<String>,
    settings_only: bool,
    base_url: Option<String>,
    paths_base: Option<String>,
    rules: Vec<(String, Value)>,
    roots: Roots,
    uncertain: bool,
    base_url_uncertain: bool,
    packages: bool,
}

impl Paths {
    pub(super) fn read(topology: &Topology) -> Self {
        let named: Vec<&String> = topology
            .files()
            .iter()
            .filter(|file| config_name(file))
            .collect();
        let parsed = configurations(topology, &named);
        let configs: Vec<Config> = named
            .into_iter()
            .map(|file| Config::read(file, &parsed))
            .collect();
        let directories: BTreeSet<&str> = configs
            .iter()
            .map(|config| config.directory.as_str())
            .collect();
        let scopes = directories
            .into_iter()
            .map(|directory| (directory.to_string(), Scope::compile(directory, &configs)))
            .collect();
        let programs = configs.into_iter().map(Program::compile).collect();
        Self { programs, scopes }
    }

    /// Select ownership and root membership once per source file, before its imports.
    pub(super) fn for_file(&self, file: &str) -> Option<FilePaths<'_>> {
        let scope = self.scope(file)?;
        let owners: Vec<&Program> = scope
            .owners
            .iter()
            .map(|&index| &self.programs[index])
            .collect();
        if owners.iter().all(|program| program.is_inert()) {
            return None;
        }
        let root = owners.iter().any(|program| program.roots.contains(file));
        Some(FilePaths {
            owners,
            base_url_unproved: scope.base_url_unproved,
            root,
        })
    }

    /// Whether every config that owns this file looks bare names up among packages, and true
    /// where none owns it, as TypeScript's defaults do.
    pub(super) fn packages(&self, file: &str) -> bool {
        self.scope(file).is_none_or(|scope| {
            scope
                .owners
                .iter()
                .all(|&index| self.programs[index].packages)
        })
    }

    fn scope(&self, file: &str) -> Option<&Scope> {
        if self.scopes.is_empty() {
            return None;
        }
        let mut ancestor = directory(file);
        loop {
            if let Some(scope) = self.scopes.get(ancestor) {
                return Some(scope);
            }
            if ancestor.is_empty() {
                return None;
            }
            ancestor = directory(ancestor);
        }
    }
}

fn contains(directory: &str, file: &str) -> bool {
    directory.is_empty()
        || file
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/'))
}

struct Scope {
    /// The programs of the owning configs. An alias is proved only where every one resolves it
    /// the same way.
    owners: Vec<usize>,
    /// More than one owner, or one klin cannot prove.
    base_url_unproved: bool,
}

/// The roots and alias rules of one owning config.
struct Program {
    roots: Roots,
    /// The config's own `baseUrl`, which bare imports resolve under.
    base_url: Option<String>,
    /// The directory paths targets resolve from, read through the chain.
    paths_base: Option<String>,
    exact: HashMap<String, Option<String>>,
    wildcards: Vec<Wildcard>,
    uncertain: bool,
    packages: bool,
}

pub(super) struct FilePaths<'a> {
    owners: Vec<&'a Program>,
    base_url_unproved: bool,
    root: bool,
}

impl FilePaths<'_> {
    /// None means no owning config recognizes this specifier; Err means known local but
    /// unproved, including owners that resolve it to different paths.
    pub(super) fn resolve(&self, specifier: &str) -> Option<Result<String, String>> {
        if let [only] = self.owners.as_slice() {
            return only.resolve(specifier, self.root);
        }
        let resolved: Vec<_> = self
            .owners
            .iter()
            .map(|program| program.resolve(specifier, self.root))
            .collect();
        match resolved.as_slice() {
            [first @ Some(Ok(_)), rest @ ..] if rest.iter().all(|other| other == first) => {
                first.clone()
            }
            _ if resolved.iter().all(Option::is_none) => None,
            _ => Some(Err(unproved(specifier))),
        }
    }

    /// The paths a bare specifier names under each `baseUrl` an owning config writes, and
    /// whether klin proves the one config and the file's root membership.
    pub(super) fn under_base_url(&self, specifier: &str) -> Option<(Vec<String>, bool)> {
        if absolute(specifier) {
            return None;
        }
        let bases: Vec<String> = self
            .owners
            .iter()
            .filter_map(|program| joined(program.base_url.as_deref()?, specifier))
            .collect();
        (!bases.is_empty()).then_some((bases, !self.base_url_unproved && self.root))
    }
}

struct Wildcard {
    prefix: String,
    suffix: String,
    target: Option<String>,
}

impl Scope {
    fn compile(directory: &str, configs: &[Config]) -> Self {
        let candidates: Vec<usize> = (0..configs.len())
            .filter(|&index| {
                let config = &configs[index].directory;
                config == directory || contains(config, directory)
            })
            .collect();
        let competing: Vec<&Config> = candidates.iter().map(|&index| &configs[index]).collect();
        let owners: Vec<usize> = candidates
            .into_iter()
            .filter(|&index| !configs[index].extended(&competing))
            .collect();
        Self {
            base_url_unproved: owners.len() != 1
                || owners
                    .iter()
                    .any(|&index| configs[index].base_url_uncertain),
            owners,
        }
    }
}

fn unproved(specifier: &str) -> String {
    format!("{specifier} is a local paths alias whose configuration klin cannot prove")
}

impl Program {
    fn compile(config: Config) -> Self {
        let mut program = Self {
            roots: config.roots,
            base_url: config.base_url,
            paths_base: config.paths_base,
            exact: HashMap::new(),
            wildcards: Vec::new(),
            uncertain: config.uncertain,
            packages: config.packages,
        };
        for (pattern, targets) in &config.rules {
            program.insert(pattern, targets);
        }
        program
            .wildcards
            .sort_by_key(|rule| std::cmp::Reverse(rule.prefix.len()));
        program
    }

    /// Whether the config gives neither an alias rule nor a `baseUrl`.
    fn is_inert(&self) -> bool {
        self.exact.is_empty() && self.wildcards.is_empty() && self.base_url.is_none()
    }

    fn insert(&mut self, pattern: &str, targets: &Value) {
        match pattern.split_once('*') {
            None => {
                self.exact
                    .insert(pattern.to_string(), target(targets, false));
            }
            Some((prefix, suffix)) if !suffix.contains('*') => self.wildcards.push(Wildcard {
                prefix: prefix.to_string(),
                suffix: suffix.to_string(),
                target: target(targets, true),
            }),
            Some(_) => {}
        }
    }

    /// None means no explicit rule recognizes this specifier; Err means known local but unproved.
    fn resolve(&self, specifier: &str, root: bool) -> Option<Result<String, String>> {
        if absolute(specifier) {
            return None;
        }
        if let Some(target) = self.exact.get(specifier) {
            return Some(self.expand(target, "", specifier, !root));
        }
        let (index, rule, wild) = self
            .wildcards
            .iter()
            .enumerate()
            .find_map(|(index, rule)| Some((index, rule, rule.capture(specifier)?)))?;
        let ambiguous = self.wildcards[index + 1..]
            .iter()
            .take_while(|next| next.prefix.len() == rule.prefix.len())
            .any(|next| next.capture(specifier).is_some());
        Some(self.expand(&rule.target, wild, specifier, ambiguous || !root))
    }

    fn expand(
        &self,
        target: &Option<String>,
        wild: &str,
        specifier: &str,
        ambiguous: bool,
    ) -> Result<String, String> {
        if self.uncertain || ambiguous {
            return Err(unproved(specifier));
        }
        let (Some(base), Some(target)) = (&self.paths_base, target) else {
            return Err(format!(
                "{specifier} is a local paths alias whose target klin cannot prove"
            ));
        };
        joined(base, &target.replace('*', wild))
            .ok_or_else(|| format!("{specifier} is a local paths alias that leaves the tree"))
    }
}

impl Wildcard {
    fn capture<'a>(&self, specifier: &'a str) -> Option<&'a str> {
        specifier
            .strip_prefix(&self.prefix)?
            .strip_suffix(&self.suffix)
    }
}

fn target(targets: &Value, wildcard: bool) -> Option<String> {
    let [target] = targets.as_array()?.as_slice() else {
        return None;
    };
    target
        .as_str()
        .filter(|target| target.matches('*').count() <= usize::from(wildcard) && !absolute(target))
        .map(str::to_string)
}

fn absolute(path: &str) -> bool {
    path.starts_with('/') || path.contains('\\') || path.contains(':')
}

fn config_name(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    name == "tsconfig.json" || (name.starts_with("tsconfig.") && name.ends_with(".json"))
}

/// Read every held config that a held config reaches through local `extends`, once each. The
/// visited map bounds cycles.
fn configurations(topology: &Topology, named: &[&String]) -> BTreeMap<String, Value> {
    let mut parsed = BTreeMap::new();
    let mut pending: Vec<String> = named.iter().map(|file| (*file).clone()).collect();
    while let Some(file) = pending.pop() {
        if parsed.contains_key(&file) {
            continue;
        }
        let value = topology
            .read(&file)
            .and_then(|bytes| jsonc::parse(&bytes).ok())
            .unwrap_or(Value::Null);
        for parent in parents(&file, &value) {
            let parent = if topology.holds(&parent) {
                parent
            } else {
                format!("{parent}.json")
            };
            if topology.holds(&parent) {
                pending.push(parent);
            }
        }
        parsed.insert(file, value);
    }
    parsed
}

fn parents(file: &str, value: &Value) -> Vec<String> {
    let Some(extends) = value.get("extends") else {
        return Vec::new();
    };
    let names: Vec<&Value> = match extends.as_array() {
        Some(names) => names.iter().collect(),
        None => vec![extends],
    };
    names
        .into_iter()
        .filter_map(|value| local(file, value.as_str()?))
        .collect()
}

/// A parent TypeScript reads as a file: one written from `./` or `../`. Any other name is a
/// package to TypeScript, even `.config/base.json`.
fn local(file: &str, extends: &str) -> Option<String> {
    (extends.starts_with("./") || extends.starts_with("../"))
        .then(|| joined(directory(file), extends))
        .flatten()
}

/// The configs one config reaches through single-string held local `extends`, nearest first,
/// as TypeScript's `parseConfig` merges them, and whether that is the whole chain: no package,
/// unreadable or array-form parent and no cycle. Every value a config inherits is read here.
struct Lineage<'a> {
    configs: Vec<(&'a str, &'a Value)>,
    whole: bool,
}

impl<'a> Lineage<'a> {
    fn read(file: &str, parsed: &'a BTreeMap<String, Value>) -> Self {
        let mut lineage = Self {
            configs: Vec::new(),
            whole: false,
        };
        let mut next = parsed.get_key_value(file);
        while let Some((file, value)) = next {
            if lineage.configs.iter().any(|&(seen, _)| seen == file) {
                break;
            }
            lineage.configs.push((file, value));
            let Some(extends) = value.get("extends") else {
                lineage.whole = !value.is_null();
                break;
            };
            next = extends
                .as_str()
                .and_then(|extends| local(file, extends))
                .and_then(|parent| parsed.get_key_value(&held(parent, parsed)));
        }
        lineage
    }

    fn directory(&self) -> &'a str {
        self.configs
            .first()
            .map_or("", |&(file, _)| directory(file))
    }

    /// The nearest value written at `pointer` and the directory it resolves from. Where the chain
    /// is not whole, a parent klin cannot read may supply it, so only the config's own counts.
    fn written(&self, pointer: &str) -> Option<(&'a Value, &'a str)> {
        self.configs
            .iter()
            .take(if self.whole { usize::MAX } else { 1 })
            .find_map(|&(file, value)| Some((value.pointer(pointer)?, directory(file))))
    }

    /// The nearest paths object. It names local aliases even where the chain is not whole; an
    /// empty or malformed one names none.
    fn rules(&self) -> Vec<(String, Value)> {
        self.configs
            .iter()
            .find_map(|(_, value)| value.pointer("/compilerOptions/paths"))
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .map(|(name, target)| (name.clone(), target.clone()))
            .collect()
    }

    /// TypeScript's `baseUrl ?? pathsBasePath`: the nearest `baseUrl`, resolved from the config
    /// that writes it, or else the directory of the config that writes the nearest paths. A
    /// `null` `baseUrl` gives none.
    fn paths_base(&self) -> Option<String> {
        match self.written("/compilerOptions/baseUrl") {
            Some((value, directory)) => value
                .as_str()
                .filter(|path| !absolute(path))
                .and_then(|path| joined(directory, path)),
            None => self
                .written("/compilerOptions/paths")
                .map(|(_, directory)| directory.to_string()),
        }
    }

    /// Whether a value the paths rules or the roots read starts with `${configDir}`, which klin
    /// does not substitute.
    fn names_config_dir(&self, rules: &[(String, Value)]) -> bool {
        [
            "/compilerOptions/baseUrl",
            "/compilerOptions/outDir",
            "/compilerOptions/declarationDir",
            "/include",
            "/exclude",
            "/files",
        ]
        .into_iter()
        .filter_map(|pointer| self.written(pointer))
        .any(|(value, _)| config_dir(value))
            || rules.iter().any(|(_, targets)| config_dir(targets))
    }
}

fn config_dir(value: &Value) -> bool {
    match value {
        Value::String(path) => path.starts_with("${configDir}"),
        Value::Array(values) => values.iter().any(config_dir),
        _ => false,
    }
}

fn held(parent: String, parsed: &BTreeMap<String, Value>) -> String {
    if parsed.contains_key(&parent) {
        parent
    } else {
        format!("{parent}.json")
    }
}

/// The values TypeScript may give a config for the option at `pointer`: its own, or else the
/// nearest one each held local `extends` branch writes.
fn nearest<'a>(file: &str, parsed: &'a BTreeMap<String, Value>, pointer: &str) -> Vec<&'a Value> {
    let value = &parsed[file];
    if let Some(own) = value.pointer(pointer) {
        return vec![own];
    }
    let mut written = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pending = parents(file, value);
    while let Some(parent) = pending.pop() {
        let parent = held(parent, parsed);
        let Some(value) = parsed.get(&parent) else {
            continue;
        };
        if !seen.insert(parent.clone()) {
            continue;
        }
        match value.pointer(pointer) {
            Some(found) => written.push(found),
            None => pending.extend(parents(&parent, value)),
        }
    }
    written
}

fn suffixes_apply(file: &str, parsed: &BTreeMap<String, Value>) -> bool {
    nearest(file, parsed, "/compilerOptions/moduleSuffixes")
        .into_iter()
        .any(|suffixes| match suffixes.as_array().map(Vec::as_slice) {
            Some([]) => false,
            Some([only]) => only.as_str() != Some(""),
            _ => true,
        })
}

/// Whether TypeScript 5 looks a bare name up among packages under this config: a written
/// `moduleResolution` other than `classic`, or else a `module` that implies one, or else no
/// `target` above ES5, which an unwritten `module` follows. Every value the held chain may give
/// must agree. A package parent is not read.
fn looks_up_packages(file: &str, parsed: &BTreeMap<String, Value>) -> bool {
    let agree = |pointer: &str, looking: &[&str]| {
        let written = nearest(file, parsed, pointer);
        (!written.is_empty()).then(|| {
            written.iter().all(|value| {
                value
                    .as_str()
                    .is_some_and(|mode| looking.contains(&mode.to_ascii_lowercase().as_str()))
            })
        })
    };
    agree(
        "/compilerOptions/moduleResolution",
        &["node", "node10", "node16", "nodenext", "bundler"],
    )
    .or_else(|| {
        agree(
            "/compilerOptions/module",
            &[
                "commonjs", "node16", "node18", "node20", "nodenext", "preserve",
            ],
        )
    })
    .unwrap_or_else(|| agree("/compilerOptions/target", &["es3", "es5"]).unwrap_or(true))
}

/// The held local configs one config reaches through `extends`, whether a parent on the way is
/// a package or a config klin cannot read, and whether the chain holds a cycle.
struct Chain {
    reached: BTreeSet<String>,
    opaque: bool,
    cyclic: bool,
}

impl Chain {
    fn walk(file: &str, parsed: &BTreeMap<String, Value>) -> Self {
        let mut chain = Self {
            reached: BTreeSet::new(),
            opaque: false,
            cyclic: false,
        };
        chain.visit(file, parsed, &mut Vec::new());
        chain
    }

    fn visit(&mut self, file: &str, parsed: &BTreeMap<String, Value>, path: &mut Vec<String>) {
        let value = &parsed[file];
        let written = value
            .get("extends")
            .map_or(0, |extends| extends.as_array().map_or(1, Vec::len));
        let parents = parents(file, value);
        self.opaque |= value.is_null() || parents.len() != written;
        path.push(file.to_string());
        for parent in parents {
            let parent = held(parent, parsed);
            if path.contains(&parent) {
                self.cyclic = true;
            } else if !parsed.contains_key(&parent) {
                self.opaque = true;
            } else if self.reached.insert(parent.clone()) {
                self.visit(&parent, parsed, path);
            }
        }
        path.pop();
    }

    /// Whether TypeScript may hand the config the value at `pointer`, such as `/files`, from a
    /// parent: one that writes it, or one klin cannot read.
    fn inherits(&self, pointer: &str, parsed: &BTreeMap<String, Value>) -> bool {
        self.opaque
            || self
                .reached
                .iter()
                .any(|parent| parsed[parent].pointer(pointer).is_some())
    }
}

/// What a config that only supplies settings writes and inherits none of: roots that would make
/// it a project of its own, and alias names that would give a file another local target.
const OWN_PROJECT: [&str; 4] = [
    "/include",
    "/files",
    "/compilerOptions/paths",
    "/compilerOptions/baseUrl",
];

fn default_project(file: &str) -> bool {
    file.ends_with("/tsconfig.json") || file == "tsconfig.json"
}

impl Config {
    fn read(file: &str, parsed: &BTreeMap<String, Value>) -> Self {
        let value = &parsed[file];
        let chain = Chain::walk(file, parsed);
        let lineage = Lineage::read(file, parsed);
        let rules = lineage.rules();
        let base_url = value
            .pointer("/compilerOptions/baseUrl")
            .and_then(Value::as_str)
            .filter(|path| !absolute(path))
            .and_then(|path| joined(directory(file), path));
        let unprovable = chain.cyclic || Self::unprovable(file, parsed);
        let partial = Self::partial(value, base_url.is_some());
        Self {
            file: file.to_string(),
            directory: directory(file).to_string(),
            settings_only: OWN_PROJECT.into_iter().all(|pointer| {
                value.pointer(pointer).is_none() && !chain.inherits(pointer, parsed)
            }),
            uncertain: unprovable
                || (partial && !lineage.whole)
                || lineage.names_config_dir(&rules),
            base_url_uncertain: unprovable || partial,
            packages: looks_up_packages(file, parsed),
            reached: chain.reached,
            base_url,
            paths_base: lineage.paths_base(),
            rules,
            roots: Roots::read(&lineage),
        }
    }

    /// A `tsconfig.*.json` with no roots and no alias names that another config in its directory
    /// extends, outside a cycle, only supplies settings.
    fn extended(&self, candidates: &[&Config]) -> bool {
        self.settings_only
            && !default_project(&self.file)
            && candidates.iter().any(|other| {
                other.directory == self.directory
                    && other.reached.contains(&self.file)
                    && !self.reached.contains(&other.file)
            })
    }

    /// Whether a config that extends another takes its paths or `baseUrl` from a parent. Under a
    /// parent klin cannot read whole, only a child that writes both proves its aliases.
    fn partial(value: &Value, own_base_url: bool) -> bool {
        value.get("extends").is_some()
            && !(value.pointer("/compilerOptions/paths").is_some() && own_base_url)
    }

    fn unprovable(file: &str, parsed: &BTreeMap<String, Value>) -> bool {
        let value = &parsed[file];
        !default_project(file)
            || value.is_null()
            || value.get("references").is_some()
            || suffixes_apply(file, parsed)
    }
}

/// A conservative root subset; import reachability never upgrades a non-root to proof.
struct Roots {
    files: BTreeSet<String>,
    include: Option<RootPatterns>,
    exclude: Option<RootPatterns>,
}

struct RootPatterns {
    matching: Vec<RootPattern>,
    unknown: bool,
}

enum RootPattern {
    Exact(String),
    Subtree(String),
    RecursiveSuffix {
        prefix: String,
        suffix: &'static str,
    },
}

impl Roots {
    fn read(lineage: &Lineage) -> Self {
        Self {
            files: Self::files(lineage),
            include: Self::include(lineage),
            exclude: Self::exclude(lineage),
        }
    }

    fn files(lineage: &Lineage) -> BTreeSet<String> {
        let Some((Value::Array(values), directory)) = lineage.written("/files") else {
            return BTreeSet::new();
        };
        values
            .iter()
            .filter_map(|value| {
                let path = value.as_str()?;
                (!absolute(path)).then(|| joined(directory, path)).flatten()
            })
            .collect()
    }

    /// The nearest `include`, or else TypeScript's default: nothing beside `files`, or the
    /// config's own directory. None where a parent klin cannot read may write one.
    fn include(lineage: &Lineage) -> Option<RootPatterns> {
        if let Some((value, directory)) = lineage.written("/include") {
            return root_patterns(directory, value);
        }
        let matching = match lineage.written("/files") {
            Some(_) => Vec::new(),
            None => vec![RootPattern::Subtree(lineage.directory().to_string())],
        };
        lineage.whole.then_some(RootPatterns {
            matching,
            unknown: false,
        })
    }

    /// The nearest `exclude`, or else none, and the nearest `outDir` and `declarationDir` either
    /// way, each resolved from the config that writes it.
    fn exclude(lineage: &Lineage) -> Option<RootPatterns> {
        let mut exclude = match lineage.written("/exclude") {
            Some((value, directory)) => root_patterns(directory, value),
            None => lineage.whole.then_some(RootPatterns {
                matching: Vec::new(),
                unknown: false,
            }),
        };
        for option in ["outDir", "declarationDir"] {
            if let Some((value, directory)) = lineage.written(&format!("/compilerOptions/{option}"))
            {
                exclude = exclude.and_then(|mut patterns| {
                    patterns.matching.push(RootPattern::Subtree(joined(
                        directory,
                        value.as_str().filter(|path| !absolute(path))?,
                    )?));
                    Some(patterns)
                });
            }
        }
        exclude
    }

    fn contains(&self, file: &str) -> bool {
        if self.files.contains(file) {
            return true;
        }
        let (Some(include), Some(exclude)) = (&self.include, &self.exclude) else {
            return false;
        };
        !file.split('/').any(|part| {
            part.starts_with('.')
                || matches!(part, "node_modules" | "bower_components" | "jspm_packages")
        }) && include.matching.iter().any(|pattern| pattern.matches(file))
            && !exclude.unknown
            && !exclude.matching.iter().any(|pattern| pattern.matches(file))
    }
}

fn root_patterns(directory: &str, value: &Value) -> Option<RootPatterns> {
    let mut patterns = RootPatterns {
        matching: Vec::new(),
        unknown: false,
    };
    for value in value.as_array()? {
        let Some(written) = value.as_str() else {
            patterns.unknown = true;
            continue;
        };
        match root_pattern(directory, written) {
            Some(pattern) => patterns.matching.push(pattern),
            None => patterns.unknown = true,
        }
    }
    Some(patterns)
}

fn root_pattern(directory: &str, written: &str) -> Option<RootPattern> {
    if let Some((prefix, suffix)) = recursive_extension(written) {
        return recursive_suffix(directory, &prefix, suffix);
    }
    let path = if written == "**/*" {
        ""
    } else {
        written.strip_suffix("/**/*").unwrap_or(written)
    };
    if absolute(path) || path.contains(['*', '?']) {
        return None;
    }
    let path = joined(directory, path)?;
    Some(
        if written.ends_with("/**/*") || std::path::Path::new(written).extension().is_none() {
            RootPattern::Subtree(path)
        } else {
            RootPattern::Exact(path)
        },
    )
}

fn recursive_extension(written: &str) -> Option<(String, &'static str)> {
    let rooted = format!("/{written}");
    [".ts", ".tsx"].into_iter().find_map(|suffix| {
        let prefix = rooted.strip_suffix(&format!("/**/*{suffix}"))?;
        Some((prefix.trim_start_matches('/').to_string(), suffix))
    })
}

fn recursive_suffix(directory: &str, prefix: &str, suffix: &'static str) -> Option<RootPattern> {
    if absolute(prefix) {
        return None;
    }
    Some(RootPattern::RecursiveSuffix {
        prefix: joined(directory, prefix)?,
        suffix,
    })
}

impl RootPattern {
    fn matches(&self, file: &str) -> bool {
        match self {
            RootPattern::Exact(path) => path == file,
            RootPattern::Subtree(path) => path == file || contains(path, file),
            RootPattern::RecursiveSuffix { prefix, suffix } => {
                file.ends_with(suffix) && (prefix.is_empty() || contains(prefix, file))
            }
        }
    }
}
