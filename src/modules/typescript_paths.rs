//! Direct TypeScript paths rules, compiled once from this tree's held configuration files.
use serde_json::Value;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use super::resolver::{Topology, directory, joined};
use crate::sys::jsonc;

pub(super) struct Paths {
    scopes: HashMap<String, Scope>,
}

/// The directory paths targets resolve from: a written `baseUrl`, or the config's own directory.
#[derive(Clone)]
enum Anchor {
    Config(String),
    BaseUrl(String),
}

impl Anchor {
    fn read(file: &str, value: &Value) -> Option<Self> {
        match value.pointer("/compilerOptions/baseUrl") {
            None => Some(Anchor::Config(directory(file).to_string())),
            Some(value) => value
                .as_str()
                .filter(|path| !absolute(path))
                .and_then(|path| joined(directory(file), path))
                .map(Anchor::BaseUrl),
        }
    }

    fn path(&self) -> &str {
        match self {
            Anchor::Config(path) | Anchor::BaseUrl(path) => path,
        }
    }

    fn base_url(&self) -> Option<&str> {
        match self {
            Anchor::BaseUrl(path) => Some(path),
            Anchor::Config(_) => None,
        }
    }
}

struct Config {
    file: String,
    directory: String,
    reached: BTreeSet<String>,
    inherits_files: bool,
    settings_only: bool,
    targets: OnceCell<Vec<(usize, Option<String>)>>,
    anchor: Option<Anchor>,
    rules: Vec<(String, Value)>,
    roots: Roots,
    uncertain: bool,
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
        let specifiers = Specifiers {
            topology,
            named: OnceCell::new(),
        };
        let scopes = directories
            .into_iter()
            .map(|directory| {
                let scope = Scope::compile(directory, &configs, &specifiers);
                (directory.to_string(), scope)
            })
            .collect();
        Self { scopes }
    }

    /// Select ownership and root membership once per source file, before its imports.
    pub(super) fn for_file(&self, file: &str) -> Option<FilePaths<'_>> {
        if self.scopes.is_empty() {
            return None;
        }
        let mut ancestor = directory(file);
        loop {
            if let Some(scope) = self.scopes.get(ancestor) {
                if scope.exact.is_empty()
                    && scope.wildcards.is_empty()
                    && scope.base_urls.is_empty()
                {
                    return None;
                }
                let root = scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(file));
                return Some(FilePaths { scope, root });
            }
            if ancestor.is_empty() {
                return None;
            }
            ancestor = directory(ancestor);
        }
    }
}

/// What each TypeScript source names, read once and only when an ancestor may exclude a scope:
/// its imports and re-exports, and the string literals in `import()`, `require()` and
/// `/// <reference path>` forms that the facts do not hold. None is a source klin cannot read.
struct Specifiers<'t, 'a> {
    topology: &'t Topology<'a>,
    named: OnceCell<Vec<Named<'t>>>,
}

type Named<'t> = (&'t str, Option<Vec<String>>);

impl<'t> Specifiers<'t, '_> {
    fn all(&self) -> &[Named<'t>] {
        let topology = self.topology;
        self.named.get_or_init(|| {
            topology
                .files()
                .iter()
                .filter(|file| typescript_source(file))
                .map(|file| (file.as_str(), Self::named(topology, file)))
                .collect()
        })
    }

    fn named(topology: &Topology, file: &str) -> Option<Vec<String>> {
        let facts = topology.facts(file)?;
        let source = topology.read(file)?;
        let mut named: Vec<String> = facts
            .imports
            .iter()
            .filter_map(|import| import.module.clone())
            .collect();
        named.extend(loose(&String::from_utf8_lossy(&source))?);
        Some(named)
    }
}

fn loose(source: &str) -> Option<Vec<String>> {
    static FORMS: OnceLock<Option<regex::Regex>> = OnceLock::new();
    let forms = FORMS
        .get_or_init(|| {
            regex::Regex::new(
                r#"(?:\b(?:import|require)\s*\(\s*|<reference\s+path\s*=\s*)(?:"([^"]*)"|'([^']*)'|`([^`$]*)`)"#,
            )
            .ok()
        })
        .as_ref()?;
    Some(
        forms
            .captures_iter(source)
            .filter_map(|found| found.iter().skip(1).flatten().next())
            .map(|literal| literal.as_str().to_string())
            .collect(),
    )
}

fn typescript_source(file: &str) -> bool {
    crate::syntax::structural::language_of(file) == Some(crate::syntax::LanguageId::TypeScript)
}

fn contains(directory: &str, file: &str) -> bool {
    directory.is_empty()
        || file
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/'))
}

struct Scope {
    roots: Option<Roots>,
    anchor: Option<Anchor>,
    /// Every `baseUrl` an owning config writes, so an ambiguous owner still looks under each.
    base_urls: Vec<String>,
    exact: HashMap<String, Option<String>>,
    wildcards: Vec<Wildcard>,
    uncertain: bool,
}

#[derive(Clone, Copy)]
pub(super) struct FilePaths<'a> {
    scope: &'a Scope,
    root: bool,
}

impl FilePaths<'_> {
    pub(super) fn resolve(self, specifier: &str) -> Option<Result<String, String>> {
        self.scope.resolve(specifier, self.root)
    }

    /// The paths a bare specifier names under each `baseUrl` an owning config writes, and
    /// whether klin proves the one config and the file's root membership.
    pub(super) fn under_base_url(self, specifier: &str) -> Option<(Vec<String>, bool)> {
        if absolute(specifier) {
            return None;
        }
        let bases: Vec<String> = self
            .scope
            .base_urls
            .iter()
            .filter_map(|base| joined(base, specifier))
            .collect();
        (!bases.is_empty()).then_some((bases, !self.scope.uncertain && self.root))
    }
}

struct Wildcard {
    prefix: String,
    suffix: String,
    target: Option<String>,
}

impl Scope {
    fn compile(directory: &str, configs: &[Config], specifiers: &Specifiers) -> Self {
        let applicable: Vec<&Config> = configs
            .iter()
            .filter(|config| {
                config.directory == directory || contains(&config.directory, directory)
            })
            .filter(|config| !config.excludes(directory, specifiers))
            .collect();
        let owners: Vec<&Config> = applicable
            .iter()
            .copied()
            .filter(|config| !config.extended(&applicable))
            .collect();
        Self::of(&owners)
    }

    fn of(owners: &[&Config]) -> Self {
        let mut scope = Self {
            roots: (owners.len() == 1).then(|| owners[0].roots.clone()),
            anchor: owners.first().and_then(|config| config.anchor.clone()),
            base_urls: owners
                .iter()
                .filter_map(|config| config.anchor.as_ref()?.base_url().map(str::to_string))
                .collect(),
            exact: HashMap::new(),
            wildcards: Vec::new(),
            uncertain: owners.len() != 1 || owners.iter().any(|config| config.uncertain),
        };
        for config in owners {
            for (pattern, targets) in &config.rules {
                scope.insert(pattern, targets);
            }
        }
        scope
            .wildcards
            .sort_by_key(|rule| std::cmp::Reverse(rule.prefix.len()));
        scope
    }

    /// The paths a specifier in `file` may name: relative, then a paths rule, then each `baseUrl`.
    /// A specifier with an escape may spell any path, so it names an unknown one.
    fn names(&self, file: &str, specifier: &str) -> Vec<Option<String>> {
        if specifier.contains('\\') {
            return vec![None];
        }
        if relative(specifier) {
            return joined(directory(file), specifier)
                .map(Some)
                .into_iter()
                .collect();
        }
        match self.resolve(specifier, true) {
            Some(found) => vec![found.ok()],
            None => self
                .base_urls
                .iter()
                .filter_map(|base| joined(base, specifier))
                .map(Some)
                .collect(),
        }
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
            return Err(format!(
                "{specifier} is a local paths alias whose configuration klin cannot prove"
            ));
        }
        let (Some(anchor), Some(target)) = (&self.anchor, target) else {
            return Err(format!(
                "{specifier} is a local paths alias whose target klin cannot prove"
            ));
        };
        joined(anchor.path(), &target.replace('*', wild))
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

pub(super) fn relative(specifier: &str) -> bool {
    matches!(specifier, "." | "..") || specifier.starts_with("./") || specifier.starts_with("../")
}

fn absolute(path: &str) -> bool {
    path.starts_with('/') || path.contains('\\') || path.contains(':')
}

fn config_name(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file);
    name == "tsconfig.json" || (name.starts_with("tsconfig.") && name.ends_with(".json"))
}

/// Follow only held, relative configuration references, to recognize inherited local names.
/// Inheritance is never used to prove an edge. The visited map bounds cycles and reads each once.
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
        .filter_map(|value| {
            let path = value.as_str()?;
            path.starts_with('.')
                .then(|| joined(directory(file), path))
                .flatten()
        })
        .collect()
}

/// Only the nearest paths object in a single held local extends chain supplies names.
/// Multiple/package parents supply no inherited names; the visited set bounds cycles.
fn rules(file: &str, parsed: &BTreeMap<String, Value>) -> Vec<(String, Value)> {
    let mut seen = BTreeSet::new();
    let mut file = file.to_string();
    while seen.insert(file.clone()) {
        let Some(value) = parsed.get(&file) else {
            break;
        };
        if let Some(paths) = value.pointer("/compilerOptions/paths") {
            return paths
                .as_object()
                .into_iter()
                .flatten()
                .map(|(name, target)| (name.clone(), target.clone()))
                .collect();
        }
        let Some(extends) = value.get("extends").and_then(Value::as_str) else {
            break;
        };
        let Some(parent) = extends
            .starts_with('.')
            .then(|| joined(directory(&file), extends))
            .flatten()
        else {
            break;
        };
        file = held(parent, parsed);
    }
    Vec::new()
}

fn held(parent: String, parsed: &BTreeMap<String, Value>) -> String {
    if parsed.contains_key(&parent) {
        parent
    } else {
        format!("{parent}.json")
    }
}

fn suffixed(value: &Value) -> Option<bool> {
    let suffixes = value.pointer("/compilerOptions/moduleSuffixes")?;
    Some(match suffixes.as_array().map(Vec::as_slice) {
        Some([]) => false,
        Some([only]) => only.as_str() != Some(""),
        _ => true,
    })
}

fn suffixes_apply(file: &str, parsed: &BTreeMap<String, Value>) -> bool {
    let value = &parsed[file];
    if let Some(own) = suffixed(value) {
        return own;
    }
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
        match suffixed(value) {
            Some(true) => return true,
            Some(false) => {}
            None => pending.extend(parents(&parent, value)),
        }
    }
    false
}

fn reached(file: &str, parsed: &BTreeMap<String, Value>) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending = parents(file, &parsed[file]);
    while let Some(parent) = pending.pop() {
        let parent = held(parent, parsed);
        let Some(value) = parsed.get(&parent) else {
            continue;
        };
        if !seen.insert(parent.clone()) {
            continue;
        }
        pending.extend(parents(&parent, value));
    }
    seen
}

/// Whether TypeScript may hand this config the value at `key`, a JSON pointer such as `/files`,
/// from its extends chain: no own value, and a parent that writes one or that klin cannot read.
fn inherits(file: &str, key: &str, parsed: &BTreeMap<String, Value>) -> bool {
    if parsed[file].pointer(key).is_some() {
        return false;
    }
    let mut seen = BTreeSet::new();
    let mut pending = vec![file.to_string()];
    while let Some(file) = pending.pop() {
        let Some(value) = parsed.get(&file).filter(|value| !value.is_null()) else {
            return true;
        };
        if !seen.insert(file.clone()) {
            continue;
        }
        if seen.len() > 1 && value.pointer(key).is_some() {
            return true;
        }
        let written = value
            .get("extends")
            .map_or(0, |extends| extends.as_array().map_or(1, Vec::len));
        let parents = parents(&file, value);
        if parents.len() != written {
            return true;
        }
        pending.extend(parents.into_iter().map(|parent| held(parent, parsed)));
    }
    false
}

/// What a config that only supplies settings writes and inherits none of: roots that would make
/// it a project of its own, and alias names that would give a file another local target.
const SETTINGS_ONLY: [&str; 4] = [
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
        let anchor = Anchor::read(file, value);
        let reached = reached(file, parsed);
        let cyclic = reached.contains(file)
            || reached
                .iter()
                .any(|parent| self::reached(parent, parsed).contains(parent));
        Self {
            file: file.to_string(),
            directory: directory(file).to_string(),
            inherits_files: inherits(file, "/files", parsed),
            targets: OnceCell::new(),
            settings_only: SETTINGS_ONLY
                .into_iter()
                .all(|key| value.pointer(key).is_none() && !inherits(file, key, parsed)),
            uncertain: cyclic || Self::uncertain(file, parsed, anchor.as_ref()),
            reached,
            anchor,
            rules: rules(file, parsed),
            roots: Roots::read(file, value, value.get("extends").is_some()),
        }
    }

    /// A `tsconfig.*.json` with no roots and no alias names that another applicable config in its
    /// directory extends, outside a cycle, only supplies settings.
    fn extended(&self, applicable: &[&Config]) -> bool {
        self.settings_only
            && !default_project(&self.file)
            && applicable.iter().any(|other| {
                other.directory == self.directory
                    && other.reached.contains(&self.file)
                    && !self.reached.contains(&other.file)
            })
    }

    /// An ancestor leaves a directory out of its program when its own `files` and written
    /// `exclude` leave it out, and no import from outside resolves into it under its rules.
    fn excludes(&self, scope: &str, specifiers: &Specifiers) -> bool {
        self.directory != scope
            && !self.inherits_files
            && self.roots.excludes(scope)
            && !self.enters(scope, specifiers)
    }

    fn enters(&self, scope: &str, specifiers: &Specifiers) -> bool {
        let all = specifiers.all();
        let inside = |path: &str| path == scope || contains(scope, path);
        self.targets(specifiers).iter().any(|(at, target)| {
            !contains(scope, all[*at].0) && target.as_deref().is_none_or(inside)
        })
    }

    /// Each path a held source names under this config's own lookup, by its index in
    /// `specifiers`, resolved once per config. None is a target klin cannot work out.
    fn targets(&self, specifiers: &Specifiers) -> &[(usize, Option<String>)] {
        self.targets.get_or_init(|| {
            let mut alone = Scope::of(&[self]);
            alone.uncertain = false;
            let mut targets = Vec::new();
            for (at, (file, named)) in specifiers.all().iter().enumerate() {
                let Some(named) = named else {
                    targets.push((at, None));
                    continue;
                };
                for specifier in named {
                    targets.extend(
                        alone
                            .names(file, specifier)
                            .into_iter()
                            .map(|target| (at, target)),
                    );
                }
            }
            targets
        })
    }

    fn uncertain(file: &str, parsed: &BTreeMap<String, Value>, anchor: Option<&Anchor>) -> bool {
        let value = &parsed[file];
        let own_paths = value.pointer("/compilerOptions/paths").is_some()
            && anchor.and_then(Anchor::base_url).is_some();
        !default_project(file)
            || value.is_null()
            || (value.get("extends").is_some() && !own_paths)
            || value.get("references").is_some()
            || suffixes_apply(file, parsed)
    }
}

/// A conservative root subset; import reachability never upgrades a non-root to proof.
#[derive(Clone)]
struct Roots {
    files: BTreeSet<String>,
    include: Option<RootPatterns>,
    exclude: Option<RootPatterns>,
    written_exclude: Option<RootPatterns>,
}

#[derive(Clone)]
struct RootPatterns {
    matching: Vec<RootPattern>,
    unknown: bool,
}

#[derive(Clone)]
enum RootPattern {
    Exact(String),
    Subtree(String),
    RecursiveSuffix {
        prefix: String,
        suffix: &'static str,
    },
}

impl Roots {
    fn read(file: &str, value: &Value, inherits: bool) -> Self {
        let directory = directory(file);
        let files = value
            .get("files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|value| {
                let path = value.as_str()?;
                (!absolute(path)).then(|| joined(directory, path)).flatten()
            })
            .collect();
        let default_include = RootPatterns {
            matching: if value.get("files").is_some() {
                Vec::new()
            } else {
                vec![RootPattern::Subtree(directory.to_string())]
            },
            unknown: false,
        };
        let include = value
            .get("include")
            .map_or((!inherits).then_some(default_include), |value| {
                root_patterns(directory, value)
            });
        let written_exclude = value
            .get("exclude")
            .and_then(|value| root_patterns(directory, value));
        let mut exclude = if value.get("exclude").is_some() {
            written_exclude.clone()
        } else {
            (!inherits).then_some(RootPatterns {
                matching: Vec::new(),
                unknown: false,
            })
        };
        for option in ["outDir", "declarationDir"] {
            if let Some(value) = value.pointer(&format!("/compilerOptions/{option}")) {
                exclude = exclude.and_then(|mut patterns| {
                    patterns.matching.push(RootPattern::Subtree(joined(
                        directory,
                        value.as_str().filter(|path| !absolute(path))?,
                    )?));
                    Some(patterns)
                });
            }
        }
        Self {
            files,
            include,
            exclude,
            written_exclude,
        }
    }

    /// Whether the written `exclude` removes a whole directory that no `files` entry reaches into.
    fn excludes(&self, directory: &str) -> bool {
        !self.files.iter().any(|file| contains(directory, file))
            && self.written_exclude.as_ref().is_some_and(|exclude| {
                exclude.matching.iter().any(|pattern| {
                    matches!(pattern, RootPattern::Subtree(path) if path == directory || contains(path, directory))
                })
            })
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
