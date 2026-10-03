//! Direct TypeScript paths rules, compiled once from this tree's held configuration files.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::resolver::{Topology, directory, joined};

pub(super) struct Paths {
    configs: Vec<Config>,
}

struct Config {
    directory: String,
    anchor: Option<String>,
    rules: Vec<(String, Value)>,
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
        let configs = named
            .into_iter()
            .map(|file| Config::read(file, &parsed))
            .collect();
        Self { configs }
    }

    /// None means no explicit rule recognizes this specifier; Err means known local but unproved.
    pub(super) fn resolve(&self, file: &str, specifier: &str) -> Option<Result<String, String>> {
        if absolute(specifier) {
            return None;
        }
        let owners: Vec<&Config> = self
            .configs
            .iter()
            .filter(|config| {
                config.directory.is_empty()
                    || file
                        .strip_prefix(&config.directory)
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .collect();
        let mut matches: Vec<(&Config, &str, &Value, &str)> = owners
            .iter()
            .flat_map(|config| {
                config.rules.iter().filter_map(move |(pattern, targets)| {
                    capture(pattern, specifier)
                        .map(|wild| (*config, pattern.as_str(), targets, wild))
                })
            })
            .collect();
        matches.sort_by_key(|(_, pattern, _, _)| priority(pattern));
        let (config, pattern, targets, wild) = *matches.first()?;
        let unproved = || {
            Err(format!(
                "{specifier} is a local paths alias whose configuration klin cannot prove"
            ))
        };
        if owners.len() != 1
            || config.uncertain
            || matches
                .get(1)
                .is_some_and(|(_, next, _, _)| priority(next) == priority(pattern))
        {
            return Some(unproved());
        }
        Some(config.expand(pattern, targets, wild, specifier))
    }
}

fn absolute(path: &str) -> bool {
    path.starts_with('/') || path.contains('\\') || path.contains(':')
}

fn capture<'a>(pattern: &str, specifier: &'a str) -> Option<&'a str> {
    match pattern.split_once('*') {
        None => (pattern == specifier).then_some(""),
        Some((prefix, suffix)) if !suffix.contains('*') => {
            specifier.strip_prefix(prefix)?.strip_suffix(suffix)
        }
        Some(_) => None,
    }
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

fn rules(file: &str, parsed: &BTreeMap<String, Value>) -> Vec<(String, Value)> {
    let mut rules = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut pending = vec![file.to_string()];
    while let Some(file) = pending.pop() {
        if !seen.insert(file.clone()) {
            continue;
        }
        let Some(value) = parsed.get(&file) else {
            continue;
        };
        if let Some(paths) = value
            .pointer("/compilerOptions/paths")
            .and_then(Value::as_object)
        {
            for (name, target) in paths {
                rules.entry(name.clone()).or_insert_with(|| target.clone());
            }
        }
        for parent in parents(&file, value) {
            pending.push(if parsed.contains_key(&parent) {
                parent
            } else {
                format!("{parent}.json")
            });
        }
    }
    rules.into_iter().collect()
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
            uncertain: !file.ends_with("/tsconfig.json") && file != "tsconfig.json"
                || value.is_null()
                || value.get("extends").is_some()
                || value.get("references").is_some(),
        }
    }

    fn expand(
        &self,
        pattern: &str,
        targets: &Value,
        wild: &str,
        specifier: &str,
    ) -> Result<String, String> {
        let unproved = || {
            Err(format!(
                "{specifier} is a local paths alias whose target klin cannot prove"
            ))
        };
        let Some(anchor) = &self.anchor else {
            return unproved();
        };
        let Some(targets) = targets.as_array() else {
            return unproved();
        };
        let [target] = targets.as_slice() else {
            return unproved();
        };
        let Some(target) = target.as_str().filter(|target| {
            target.matches('*').count() <= usize::from(pattern.contains('*')) && !absolute(target)
        }) else {
            return unproved();
        };
        joined(anchor, &target.replace('*', wild))
            .ok_or_else(|| format!("{specifier} is a local paths alias that leaves the tree"))
    }
}

fn priority(pattern: &str) -> (bool, std::cmp::Reverse<usize>) {
    (
        pattern.contains('*'),
        std::cmp::Reverse(pattern.split('*').next().unwrap_or("").len()),
    )
}
