//! Direct TypeScript paths rules, compiled once from this tree's held configuration files.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::resolver::{Topology, directory, joined};

pub(super) struct Paths {
    scopes: HashMap<String, Scope>,
}

struct Config {
    directory: String,
    anchor: Option<String>,
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
        let scopes = directories
            .into_iter()
            .map(|directory| (directory.to_string(), Scope::compile(directory, &configs)))
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
                if scope.exact.is_empty() && scope.wildcards.is_empty() {
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

fn contains(directory: &str, file: &str) -> bool {
    directory.is_empty()
        || file
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/'))
}

struct Scope {
    roots: Option<Roots>,
    anchor: Option<String>,
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
}

struct Wildcard {
    prefix: String,
    suffix: String,
    target: Option<String>,
}

impl Scope {
    fn compile(directory: &str, configs: &[Config]) -> Self {
        let owners: Vec<&Config> = configs
            .iter()
            .filter(|config| {
                config.directory == directory || contains(&config.directory, directory)
            })
            .collect();
        let mut scope = Self {
            roots: (owners.len() == 1).then(|| owners[0].roots.clone()),
            anchor: owners.first().and_then(|config| config.anchor.clone()),
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
        joined(anchor, &target.replace('*', wild))
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

/// JSONC tokenization keeps quoted strings intact while removing comments and trailing commas.
fn json(bytes: &[u8]) -> Option<Value> {
    use std::sync::OnceLock;
    static TOKENS: OnceLock<Option<regex::Regex>> = OnceLock::new();
    let tokens = TOKENS
        .get_or_init(|| {
            regex::Regex::new(r#""(?:\\.|[^"\\])*"|//[^\r\n]*|/\*[\s\S]*?\*/|,[ \t\r\n]*([}\]])"#)
                .ok()
        })
        .as_ref()?;
    let text = std::str::from_utf8(bytes)
        .ok()?
        .trim_start_matches('\u{feff}');
    let clean = tokens.replace_all(text, |found: &regex::Captures| {
        if found[0].starts_with('"') {
            found[0].to_string()
        } else {
            found.get(1).map_or(" ", |end| end.as_str()).to_string()
        }
    });
    // Removing a comment can expose a trailing comma, so run the same token pass again.
    let clean = tokens.replace_all(&clean, |found: &regex::Captures| {
        found
            .get(1)
            .map_or_else(|| found[0].to_string(), |end| end.as_str().to_string())
    });
    serde_json::from_str(&clean).ok()
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
            .and_then(|bytes| json(&bytes))
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
        file = if parsed.contains_key(&parent) {
            parent
        } else {
            format!("{parent}.json")
        };
    }
    Vec::new()
}

impl Config {
    fn read(file: &str, parsed: &BTreeMap<String, Value>) -> Self {
        let value = &parsed[file];
        let anchor = match value.pointer("/compilerOptions/baseUrl") {
            None => Some(directory(file).to_string()),
            Some(value) => value
                .as_str()
                .filter(|path| !absolute(path))
                .and_then(|path| joined(directory(file), path)),
        };
        Self {
            directory: directory(file).to_string(),
            anchor,
            rules: rules(file, parsed),
            roots: Roots::read(file, value),
            uncertain: !file.ends_with("/tsconfig.json") && file != "tsconfig.json"
                || value.is_null()
                || value.get("extends").is_some()
                || value.get("references").is_some(),
        }
    }
}

/// A conservative root subset; import reachability never upgrades a non-root to proof.
#[derive(Clone)]
struct Roots {
    files: BTreeSet<String>,
    include: Option<RootPatterns>,
    exclude: Option<RootPatterns>,
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
    fn read(file: &str, value: &Value) -> Self {
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
        let include = value.get("include").map_or(Some(default_include), |value| {
            root_patterns(directory, value)
        });
        let mut exclude = value.get("exclude").map_or(
            Some(RootPatterns {
                matching: Vec::new(),
                unknown: false,
            }),
            |value| root_patterns(directory, value),
        );
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
        }
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
    for (pattern, suffix) in [("*.ts", ".ts"), ("*.tsx", ".tsx")] {
        if written == format!("**/{pattern}") {
            return recursive_suffix(directory, "", suffix);
        }
        if let Some(prefix) = written.strip_suffix(&format!("/**/{pattern}")) {
            return recursive_suffix(directory, prefix, suffix);
        }
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
