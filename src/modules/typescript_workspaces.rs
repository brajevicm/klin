//! The workspace packages the tree root `package.json` lists under `workspaces`, which a bare
//! specifier names the way a package manager's `node_modules` symlink does. Only literal entries
//! and entries whose segments are literal or exactly `*` are read; any other glob leaves every
//! name external. A package that writes `exports`, or a name two packages share, stays external.

use std::collections::BTreeMap;

use serde_json::Value;

use super::resolver::{Topology, joined};

const ROOT: &str = "package.json";
const ENTRIES: &[&str] = &["types", "typings", "main"];

pub(super) struct Workspaces {
    packages: BTreeMap<String, Vec<Package>>,
}

struct Package {
    directory: String,
    entries: Vec<String>,
    exports: bool,
}

impl Workspaces {
    pub(super) fn read(topology: &Topology) -> Self {
        let mut packages: BTreeMap<String, Vec<Package>> = BTreeMap::new();
        for directory in directories(topology) {
            let manifest = format!("{directory}/{ROOT}");
            let Some(parsed) = parse(topology, &manifest) else {
                continue;
            };
            let Some(name) = parsed.get("name").and_then(Value::as_str) else {
                continue;
            };
            let entries = ENTRIES
                .iter()
                .filter_map(|field| parsed.get(*field)?.as_str())
                .map(str::to_string)
                .collect();
            packages.entry(name.to_string()).or_default().push(Package {
                directory,
                entries,
                exports: parsed.get("exports").is_some(),
            });
        }
        Self { packages }
    }

    /// The paths a bare specifier may name inside the one workspace package its name matches, in
    /// the order TypeScript tries them, before the candidate rule. Empty where it names none.
    pub(super) fn bases(&self, specifier: &str) -> Vec<String> {
        let (name, rest) = split(specifier);
        let Some([package]) = self.packages.get(name).map(Vec::as_slice) else {
            return Vec::new();
        };
        if package.exports {
            return Vec::new();
        }
        match rest {
            Some(rest) => joined(&package.directory, rest).into_iter().collect(),
            None => package
                .entries
                .iter()
                .filter_map(|entry| joined(&package.directory, entry))
                .chain(Some(format!("{}/index", package.directory)))
                .collect(),
        }
    }
}

/// A specifier's package name, its first segment or its first two where the first starts with
/// `@`, and the subpath after it.
fn split(specifier: &str) -> (&str, Option<&str>) {
    let end = match specifier.find('/') {
        Some(slash) if specifier.starts_with('@') => specifier[slash + 1..]
            .find('/')
            .map(|next| slash + 1 + next),
        found => found,
    };
    match end {
        Some(end) => (&specifier[..end], Some(&specifier[end + 1..])),
        None => (specifier, None),
    }
}

fn parse(topology: &Topology, manifest: &str) -> Option<Value> {
    topology
        .read(manifest)
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

/// The directories the root's `workspaces` entries match that hold a `package.json`, and none
/// where any entry uses a glob beyond a one-segment `*`.
fn directories(topology: &Topology) -> Vec<String> {
    if !topology.holds(ROOT) {
        return Vec::new();
    }
    let Some(root) = parse(topology, ROOT) else {
        return Vec::new();
    };
    let listed = root.get("workspaces");
    let Some(entries) = listed
        .and_then(|field| field.get("packages"))
        .or(listed)
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut patterns = Vec::new();
    for entry in entries {
        let Some(pattern) = entry.as_str().and_then(|entry| joined("", entry)) else {
            return Vec::new();
        };
        if !pattern.split('/').all(supported) {
            return Vec::new();
        }
        if !pattern.is_empty() {
            patterns.push(pattern);
        }
    }
    topology
        .files()
        .iter()
        .filter_map(|file| file.strip_suffix(ROOT)?.strip_suffix('/'))
        .filter(|found| patterns.iter().any(|pattern| matches(pattern, found)))
        .map(str::to_string)
        .collect()
}

fn supported(segment: &str) -> bool {
    segment == "*" || !segment.contains(['*', '?', '[', ']', '{', '}', '!'])
}

fn matches(pattern: &str, directory: &str) -> bool {
    let mut written = pattern.split('/');
    let mut found = directory.split('/');
    loop {
        match (written.next(), found.next()) {
            (None, None) => return true,
            (Some(want), Some(have)) if want == "*" || want == have => {}
            _ => return false,
        }
    }
}
