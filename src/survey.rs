use std::collections::HashSet;
use std::path::Path;

use serde_json::{Map, Value};

use crate::changed::git;
use crate::{cache, escapes, files, state, turn};

/// The key one derivation commit's survey is cached under, beside the other derivations of that
/// commit. Spec 6.6.
const KEY: &str = "survey";

/// A manifest names a project klin can build, and the command that builds it. ADR 0012.
const MANIFESTS: &[(&str, &str, &str)] = &[
    ("Cargo.toml", "", "cargo build --all-targets"),
    ("go.mod", "", "go build ./..."),
    ("package.json", "tsconfig.json", "tsc --noEmit"),
    ("tsconfig.json", "", ""),
];

/// The directory segments a language's test convention uses, and the affixes that mark one file
/// as a test. Spec 8.2.
const TEST_DIRS: &[&str] = &["tests", "test", "spec", "__tests__"];
const TEST_PREFIXES: &[&str] = &["test_", "spec_"];
const TEST_SUFFIXES: &[&str] = &["_test", "_spec", ".test", ".spec", "Test", "Tests"];

/// The ceilings this ticket does not derive. #104 replaces them with the percentile of the
/// derivation commit.
const CEILINGS: (u64, u64) = (8, 60);
const CEILING_STEP: u64 = 50;

pub const ROOT: &str = ".";

/// What one tree says about itself: where its source is, what languages it is in, the documents
/// at its top, which roots are tests, and the manifests that build it. Spec 5.4.
#[derive(Default, Clone)]
pub struct Survey {
    pub roots: Vec<String>,
    pub languages: Vec<String>,
    pub documents: Vec<String>,
    pub test_roots: Vec<String>,
    pub manifests: Vec<String>,
}

/// The sections a run uses for what the config does not pin, and one line per value saying
/// whether it was derived or pinned. Spec 4.3.
pub struct Derived {
    pub sections: Map<String, Value>,
    pub lines: Vec<String>,
    /// The derived roots the derivation commit's survey did not hold. A site under one matches
    /// nothing in `before`, so a directory that becomes a root brings no inherited debt with
    /// it. Empty when there is no commit to survey. Spec 7.1.
    pub unheld: Vec<String>,
}

/// The sections klin derives, and for an object section the keys it supplies. A section the
/// config pins whole is never derived, so the survey does not run for it.
const DERIVABLE: &[(&str, &[&str])] = &[
    ("build", &[]),
    ("complexity", &["roots", "ceilings"]),
    ("doc_citations", &[]),
    ("doc_size", &[]),
    ("escapes", &["roots", "languages"]),
];

pub fn keys(section: &str) -> Option<&'static [&'static str]> {
    DERIVABLE
        .iter()
        .find(|(name, _)| *name == section)
        .map(|(_, keys)| *keys)
}

pub fn derivable() -> impl Iterator<Item = &'static str> {
    DERIVABLE.iter().map(|(name, _)| *name)
}

/// Every derivable value, as the sections the checks read and the lines a run prints. The path
/// sets are the union of the derivation commit's survey and a walk of the working tree, kept to
/// what the working tree still holds, because a root it no longer has has nothing to measure.
/// Spec 4.3.
pub fn derive(root: &Path, pinned: &Value) -> Derived {
    let at = state::ready(root).ok();
    let commit = turn::derivation(root, at.as_deref());
    let surveyed = at_commit(root, at.as_deref(), commit.as_deref());
    let held = surveyed.clone().unwrap_or_default();
    let found = union(&held, &walked(root), root);
    let sections = sections(&found, root, commit.as_deref(), pinned);
    let lines = lines(&found, &sections, pinned);
    Derived {
        unheld: unheld(&found, &held, surveyed.is_some()),
        sections,
        lines,
    }
}

/// The roots the union added to what the derivation commit's survey held. Without a commit to
/// survey there is nothing to compare against, so nothing is unheld. Spec 7.1.
fn unheld(found: &Survey, held: &Survey, surveyed: bool) -> Vec<String> {
    match surveyed {
        false => Vec::new(),
        true => found
            .roots
            .iter()
            .filter(|root| !held.roots.iter().any(|was| under_or_at(root, was)))
            .cloned()
            .collect(),
    }
}

/// The derivation commit's own survey, from the cache when this binary wrote it and from one
/// walk of that commit otherwise. Outside a repository there is no commit and no survey.
fn at_commit(root: &Path, at: Option<&Path>, commit: Option<&str>) -> Option<Survey> {
    let commit = commit?;
    if let Some(held) = at
        .and_then(|at| cache::read(at, commit, KEY))
        .and_then(|held| read(&held))
    {
        return Some(held);
    }
    let found = of(&listed(root, commit)?);
    if let Some(at) = at {
        cache::write(at, commit, KEY, kept(&found));
    }
    Some(found)
}

/// Every path the commit holds, as git names them, less the ones no survey reads. `None` when
/// git could not read the commit, which is not the same as a commit that holds nothing: an
/// empty survey would discard the whole base and read every site as new. Spec 14.
fn listed(root: &Path, commit: &str) -> Option<Vec<String>> {
    let listed = git(root, &["ls-tree", "-r", "-z", "--name-only", commit])?;
    Some(
        listed
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .filter(|path| surveyed(path))
            .collect(),
    )
}

/// Every path the working tree holds. `files::under` prunes the default skip set and every path
/// `.gitignore` excludes, and this walk reads names only. Spec 4.3.
fn walked(root: &Path) -> Survey {
    let skip_dirs = files::default_skip_dirs();
    let wanted = files::Wanted {
        extensions: &[""],
        skip_dirs: &skip_dirs,
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let found = files::under(&[root.to_path_buf()], &wanted).unwrap_or_default();
    of(&found
        .iter()
        .map(|path| files::relative(path, root))
        .filter(|path| surveyed(path))
        .collect::<Vec<String>>())
}

/// A path no survey reads: one under the default skip set, or one under a hidden directory.
fn surveyed(path: &str) -> bool {
    !path
        .split('/')
        .any(|segment| segment.starts_with('.') || files::skipped(segment))
}

fn of(paths: &[String]) -> Survey {
    let roots = roots(paths);
    Survey {
        languages: languages(paths, &roots),
        documents: sorted(paths.iter().filter(|path| document(path)).cloned()),
        test_roots: test_roots(&roots, paths),
        manifests: sorted(paths.iter().filter(|path| manifest(path)).cloned()),
        roots,
    }
}

fn document(path: &str) -> bool {
    !path.contains('/') && path.ends_with(".md")
}

fn manifest(path: &str) -> bool {
    MANIFESTS.iter().any(|(name, _, _)| basename(path) == *name)
}

/// The shallowest directories that hold nothing but source: start at each directory that holds
/// a source file and merge upward while the directory above holds nothing but source. Spec 5.4.
fn roots(paths: &[String]) -> Vec<String> {
    let mixed = mixed(paths);
    let mut found: Vec<String> = Vec::new();
    for file in paths.iter().filter(|path| source(path)) {
        let mut at = parent(file);
        while let Some(up) = above(&at) {
            if mixed.contains(up.as_str()) {
                break;
            }
            at = up;
        }
        if !found.contains(&at) {
            found.push(at);
        }
    }
    found.sort();
    let nested = found.clone();
    found.retain(|root| !nested.iter().any(|other| under(root, other)));
    found
}

/// Every directory that holds something other than source somewhere beneath it, from one pass
/// over the paths, so merging a root upward is a lookup and not another scan.
fn mixed(paths: &[String]) -> HashSet<&str> {
    let mut found = HashSet::new();
    for path in paths.iter().filter(|path| !source(path)) {
        let mut at = path.as_str();
        while let Some((up, _)) = at.rsplit_once('/') {
            found.insert(up);
            at = up;
        }
        found.insert(ROOT);
    }
    found
}

fn source(path: &str) -> bool {
    escapes::suffixes().any(|suffix| path.ends_with(suffix))
}

fn languages(paths: &[String], roots: &[String]) -> Vec<String> {
    sorted(
        paths
            .iter()
            .filter(|path| roots.iter().any(|root| under_or_at(path, root)))
            .filter_map(|path| escapes::language_of(path))
            .map(str::to_string),
    )
}

/// The roots a language's test convention marks: a directory the convention names, or a root
/// whose every source file carries a test affix. Spec 5.4, 8.2.
fn test_roots(roots: &[String], paths: &[String]) -> Vec<String> {
    roots
        .iter()
        .filter(|root| named_for_tests(root) || holds_only_tests(paths, root))
        .cloned()
        .collect()
}

fn named_for_tests(root: &str) -> bool {
    root.split('/').any(|segment| TEST_DIRS.contains(&segment))
}

fn holds_only_tests(paths: &[String], root: &str) -> bool {
    let mut under = paths
        .iter()
        .filter(|path| under_or_at(path, root) && source(path))
        .peekable();
    under.peek().is_some() && under.all(|path| test_affix(basename(path)))
}

fn test_affix(name: &str) -> bool {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    TEST_PREFIXES.iter().any(|prefix| stem.starts_with(prefix))
        || TEST_SUFFIXES.iter().any(|suffix| stem.ends_with(suffix))
}

fn basename(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(|| ROOT.to_string(), |(at, _)| at.to_string())
}

fn above(directory: &str) -> Option<String> {
    match directory {
        ROOT => None,
        other => Some(parent(other)),
    }
}

pub fn under_or_at(path: &str, directory: &str) -> bool {
    directory == ROOT
        || path == directory
        || path
            .strip_prefix(directory)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn under(path: &str, directory: &str) -> bool {
    path != directory && under_or_at(path, directory)
}

fn sorted(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut found: Vec<String> = values.collect();
    found.sort();
    found.dedup();
    found
}

/// The two surveys as one. A path set may only grow between the trees, so this is their union,
/// less what the working tree no longer holds: klin cannot measure a root that is gone, and
/// dropping it loosens nothing, because neither tree is read there. Spec 4.3.
fn union(held: &Survey, now: &Survey, root: &Path) -> Survey {
    let both = |left: &[String], right: &[String]| sorted(left.iter().chain(right).cloned());
    let roots: Vec<String> = both(&held.roots, &now.roots)
        .into_iter()
        .filter(|at| root.join(at).is_dir())
        .collect();
    Survey {
        languages: both(&held.languages, &now.languages),
        documents: both(&held.documents, &now.documents)
            .into_iter()
            .filter(|name| root.join(name).is_file())
            .collect(),
        test_roots: both(&held.test_roots, &now.test_roots)
            .into_iter()
            .filter(|at| roots.contains(at))
            .collect(),
        manifests: both(&held.manifests, &now.manifests)
            .into_iter()
            .filter(|name| root.join(name).is_file())
            .collect(),
        roots,
    }
}

fn read(cached: &Value) -> Option<Survey> {
    let names = |key: &str| -> Vec<String> {
        cached
            .get(key)
            .and_then(Value::as_array)
            .map(|held| {
                held.iter()
                    .filter_map(|name| name.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    cached.as_object()?;
    Some(Survey {
        roots: names("roots"),
        languages: names("languages"),
        documents: names("documents"),
        test_roots: names("test_roots"),
        manifests: names("manifests"),
    })
}

fn kept(found: &Survey) -> Value {
    let mut fields = Map::new();
    fields.insert("roots".into(), list(&found.roots));
    fields.insert("languages".into(), list(&found.languages));
    fields.insert("documents".into(), list(&found.documents));
    fields.insert("test_roots".into(), list(&found.test_roots));
    fields.insert("manifests".into(), list(&found.manifests));
    Value::Object(fields)
}

fn list(values: &[String]) -> Value {
    Value::Array(values.iter().map(|value| value.clone().into()).collect())
}

/// The derived sections, each merged under what the config pins, so a section may pin one key
/// and leave the other to the survey. Spec 5.2.
pub fn sections(
    found: &Survey,
    root: &Path,
    commit: Option<&str>,
    pinned: &Value,
) -> Map<String, Value> {
    let mut out = Map::new();
    let mut add = |name: &str, value: Option<Value>| {
        if let Some(value) = value {
            out.insert(name.to_string(), merged(pinned.get(name), value));
        }
    };
    add("escapes", escapes_section(found));
    add("complexity", complexity_section(found));
    add("doc_size", doc_size_section(found, root, commit));
    add("doc_citations", doc_citations_section(found));
    add("build", build_section(found));
    out
}

/// What the config pins wins over what the survey found, key by key when both are objects and
/// whole otherwise.
fn merged(pinned: Option<&Value>, derived: Value) -> Value {
    let (Some(Value::Object(pinned)), Value::Object(mut fields)) = (pinned, derived.clone()) else {
        return pinned.cloned().unwrap_or(derived);
    };
    for (key, value) in pinned {
        fields.insert(key.clone(), value.clone());
    }
    Value::Object(fields)
}

fn escapes_section(found: &Survey) -> Option<Value> {
    if found.roots.is_empty() || found.languages.is_empty() {
        return None;
    }
    let mut section = Map::new();
    section.insert("roots".into(), list(&found.roots));
    section.insert("languages".into(), list(&found.languages));
    Some(Value::Object(section))
}

fn complexity_section(found: &Survey) -> Option<Value> {
    if found.roots.is_empty() {
        return None;
    }
    let mut ceilings = Map::new();
    ceilings.insert("cc".into(), CEILINGS.0.into());
    ceilings.insert("lines".into(), CEILINGS.1.into());
    let mut section = Map::new();
    section.insert("roots".into(), list(&found.roots));
    section.insert("ceilings".into(), Value::Object(ceilings));
    Some(Value::Object(section))
}

fn doc_size_section(found: &Survey, root: &Path, commit: Option<&str>) -> Option<Value> {
    if found.documents.is_empty() {
        return None;
    }
    Some(Value::Array(
        found
            .documents
            .iter()
            .map(|name| {
                let mut entry = Map::new();
                entry.insert("file".into(), name.clone().into());
                entry.insert("ceiling".into(), ceiling(root, commit, name).into());
                Value::Object(entry)
            })
            .collect(),
    ))
}

/// The word count at the derivation commit, rounded up to the next 50, so a document the
/// commit holds is judged against a number the working tree cannot move. A document the commit
/// does not hold is measured against itself, so day one is green; #104 makes that a NOTE.
fn ceiling(root: &Path, commit: Option<&str>, name: &str) -> u64 {
    let words = held_words(root, commit, name)
        .or_else(|| crate::doc_size::words(&root.join(name)).ok())
        .unwrap_or_default();
    (words / CEILING_STEP + 1) * CEILING_STEP
}

fn held_words(root: &Path, commit: Option<&str>, name: &str) -> Option<u64> {
    let text = crate::changed::blob(root, commit?, name)?;
    Some(String::from_utf8_lossy(&text).split_whitespace().count() as u64)
}

fn doc_citations_section(found: &Survey) -> Option<Value> {
    if found.documents.is_empty() {
        return None;
    }
    Some(Value::Array(
        found
            .documents
            .iter()
            .map(|name| {
                let mut entry = Map::new();
                entry.insert("file".into(), name.clone().into());
                entry.insert("roots".into(), list(&[ROOT.to_string()]));
                Value::Object(entry)
            })
            .collect(),
    ))
}

/// One build entry per manifest. A single manifest at the top of the tree is one command.
fn build_section(found: &Survey) -> Option<Value> {
    let entries = commands(found);
    let (at, run) = entries.first()?;
    if entries.len() == 1 && at.is_empty() {
        return Some(Value::String((*run).to_string()));
    }
    Some(Value::Array(
        entries.iter().map(|(at, run)| entry(at, run)).collect(),
    ))
}

/// The command each manifest names, at the directory that holds it, in path order. ADR 0012.
fn commands(found: &Survey) -> Vec<(String, &'static str)> {
    let mut entries: Vec<(String, &'static str)> = found
        .manifests
        .iter()
        .filter_map(|path| command(found, path))
        .collect();
    entries.sort();
    entries
}

/// A manifest the table names, and nothing for one that builds no project of its own or whose
/// companion file is not beside it.
fn command(found: &Survey, path: &str) -> Option<(String, &'static str)> {
    let (_, beside, run) = MANIFESTS
        .iter()
        .find(|(name, _, _)| basename(path) == *name)?;
    let at = match parent(path).as_str() {
        ROOT => String::new(),
        under => under.to_string(),
    };
    let whole = beside.is_empty()
        || found
            .manifests
            .iter()
            .any(|other| beside_it(other, &at, beside));
    (!run.is_empty() && whole).then_some((at, *run))
}

fn beside_it(path: &str, at: &str, name: &str) -> bool {
    let wanted = match at.is_empty() {
        true => name.to_string(),
        false => format!("{at}/{name}"),
    };
    path == wanted
}

fn entry(at: &str, run: &str) -> Value {
    let mut out = Map::new();
    if !at.is_empty() {
        out.insert("root".into(), at.into());
    }
    out.insert("run".into(), run.into());
    Value::Object(out)
}

/// One line per derived value, and one per value the config pinned beside a derived one, so a
/// run says where every number and every path set came from. A section the config states in
/// full derived nothing and prints nothing. Spec 4.3, 5.2.
fn lines(found: &Survey, sections: &Map<String, Value>, pinned: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for (name, value) in sections {
        match value {
            Value::Object(fields) => {
                if fields.keys().all(|key| pins(pinned, name, Some(key))) {
                    continue;
                }
                out.extend(
                    fields.iter().map(|(key, held)| {
                        said(name, Some(key), held, pins(pinned, name, Some(key)))
                    }),
                );
            }
            _ if pins(pinned, name, None) => continue,
            _ => out.push(said(name, None, value, false)),
        }
    }
    if !found.test_roots.is_empty() {
        out.push(format!(
            "derived: test roots {}, the roots that match a language's test convention",
            found.test_roots.join(", ")
        ));
    }
    out
}

fn said(section: &str, key: Option<&str>, value: &Value, pinned: bool) -> String {
    let named = match key {
        Some(key) => format!("{section} {key}"),
        None => section.to_string(),
    };
    match pinned {
        true => format!("pinned: {named} {}", shown(value)),
        false => format!("derived: {named} {}, {}", shown(value), rule(section, key)),
    }
}

fn rule(section: &str, key: Option<&str>) -> &'static str {
    match (section, key) {
        (_, Some("roots")) => "the shallowest directories that hold nothing but source",
        (_, Some("languages")) => "the languages of the files under those roots",
        (_, Some("ceilings")) => {
            "the day-one default until a pinned or derived ceiling replaces it"
        }
        ("doc_size", None) => {
            "every Markdown file at the tree root, each ceiling its word count rounded up to the \
             next 50"
        }
        ("doc_citations", None) => "every Markdown file at the tree root, resolved against it",
        ("build", None) => "one command per manifest",
        _ => "the survey of this tree",
    }
}

/// Whether the config named this value itself, which is what tells a `pinned:` line from a
/// `derived:` one.
fn pins(pinned: &Value, section: &str, key: Option<&str>) -> bool {
    let Some(held) = pinned.get(section) else {
        return false;
    };
    match key {
        None => true,
        Some(key) => held.get(key).is_some(),
    }
}

/// A value as a person reads it: a list of names as names, and anything else as its JSON.
fn shown(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) if items.iter().all(Value::is_string) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<&str>>()
            .join(", "),
        Value::Array(items) => items
            .iter()
            .map(|item| match item.get("file").or_else(|| item.get("run")) {
                Some(Value::String(text)) => text.clone(),
                _ => item.to_string(),
            })
            .collect::<Vec<String>>()
            .join(", "),
        other => other.to_string(),
    }
}
