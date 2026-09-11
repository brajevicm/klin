use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde_json::{Map, Value};

use crate::changed::git;
use crate::{
    build, cache, complexity, config, doc_citations, doc_size, escapes, files, inventory, lockfile,
    markers, reference, state, stubs, turn,
};

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
pub const TEST_DIRS: &[&str] = &["tests", "test", "spec", "__tests__"];
pub const TEST_PREFIXES: &[&str] = &["test_", "spec_"];
pub const TEST_SUFFIXES: &[&str] = &["_test", "_spec", ".test", ".spec", "Test", "Tests"];

/// The keys the two derived numbers are cached under, beside the survey of the same commit.
const COMPLEXITY: &str = "complexity";
const DOC_SIZE: &str = "doc_size";

/// What the `derived:` line and the `{section, key, value, rule}` entry both call the test-root
/// set. It is a derived value of 4.3 and not a key any config states, so the two say it once.
const TEST_ROOTS: &str = "test roots";

/// The floor a derived complexity ceiling never falls below, and the sample a percentile needs
/// before it is taken at all. Spec 5.4.
const CC_FLOOR: u64 = 5;
const LINES_FLOOR: u64 = 25;
const SAMPLE: usize = 50;
const PERCENTILE: usize = 95;
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
    /// One `{section, key, value, rule}` entry per line of `lines` that is a `derived:` one, in
    /// the same order, built by the same pass so the two cannot say different things. Spec 11.2.
    pub values: Vec<Value>,
    /// The derived roots the derivation commit's survey did not hold. A site under one matches
    /// nothing in `before`, so a directory that becomes a root brings no inherited debt with
    /// it. Empty when there is no commit to survey. Spec 7.1.
    pub unheld: Vec<String>,
    /// The source roots the survey found. Empty is the hole of ADR 0016: a gate the survey was
    /// left to supply reads code, and there is no code to read. Spec 10, 14.
    pub roots: Vec<String>,
}

/// The sections klin derives, and for an object section the keys it supplies. A section the
/// config pins whole is never derived, so the survey does not run for it.
const DERIVABLE: &[(&str, &[&str])] = &[
    (config::BUILD.name, &[]),
    (complexity::SECTION, complexity::DERIVED),
    (doc_citations::SECTION, &[]),
    (doc_size::SECTION, &[]),
    (escapes::SECTION, markers::DERIVED),
    (inventory::SECTION, &[]),
    (lockfile::SECTION, lockfile::DERIVED),
    (stubs::SECTION, markers::DERIVED),
];

pub fn keys(section: &str) -> Option<&'static [&'static str]> {
    DERIVABLE
        .iter()
        .find(|(name, _)| *name == section)
        .map(|(_, keys)| *keys)
}

/// Whether a section measures code, which is exactly the set the survey supplies roots for. A
/// section that reads documents has nothing to lose when the tree has no source root. Spec 10.
pub fn reads_code(section: &str) -> bool {
    keys(section).is_some_and(|keys| keys.contains(&reference::ROOTS.name))
}

/// A section the config states in full, so the survey does not have to run for it. A section
/// that is not an object — a list, a command, or `false` — states itself. A key written `a.b`
/// is stated only at that depth, so a config that pins one ceiling and leaves the other still
/// derives. Spec 5.2.
pub fn pinned_whole(section: &str, pinned: &Value) -> bool {
    let Some(supplies) = keys(section) else {
        return true;
    };
    match pinned.is_object() {
        true => supplies.iter().all(|key| stated(pinned, key)),
        false => true,
    }
}

fn stated(pinned: &Value, key: &str) -> bool {
    held(pinned, key).is_some()
}

/// The value a dotted key names, and `None` when the config states nothing at that depth.
fn held<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    key.split('.').try_fold(value, |at, part| at.get(part))
}

/// One `pinned:` line per key of a section the config states in full. A run prints nothing for
/// such a section, because it derived nothing there, and `--list` still says where each value
/// came from. Spec 10.
pub fn pinned_lines(section: &str, value: &Value) -> Vec<String> {
    let supplies = keys(section).unwrap_or_default();
    if supplies.is_empty() || !value.is_object() {
        return vec![said(section, None, value, true).line];
    }
    supplies
        .iter()
        .filter_map(|key| Some(said(section, Some(leaf(key)), held(value, key)?, true).line))
        .collect()
}

/// The keys of a section the value does not state, as the label a `derived:` line names each
/// by. A `gates` entry may state some of its check's keys and leave the rest to the survey.
/// Spec 10.
pub fn underived(section: &str, value: &Value) -> Vec<&'static str> {
    keys(section)
        .unwrap_or_default()
        .iter()
        .filter(|key| !stated(value, key))
        .map(|key| leaf(key))
        .collect()
}

fn leaf(key: &str) -> &str {
    key.rsplit_once('.').map_or(key, |(_, last)| last)
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
    let at_commit = surveyed.as_ref().zip(commit.as_deref());
    let numbers = numbers(root, at.as_deref(), at_commit, &found, pinned);
    let sections = sections(&found, &numbers, pinned);
    let said = lines(&found, &sections, &numbers, pinned);
    let mut lines = Vec::with_capacity(said.len());
    let mut values = Vec::new();
    for item in said {
        lines.push(item.line);
        values.extend(item.entry);
    }
    let unheld = unheld(&found, &held, surveyed.is_some());
    Derived {
        unheld,
        roots: found.roots,
        sections,
        lines,
        values,
    }
}

/// A number one commit sets, and what the line that prints it says about where it came from.
struct Number {
    value: u64,
    rule: String,
}

/// The numbers the derivation commit sets: the two complexity ceilings, and one word ceiling
/// per document that commit holds. A document the commit lacks is not here, so no ceiling of
/// it is read out of the working tree. Both are cached under the commit. Spec 4.3, 5.4, 6.6.
struct Numbers {
    cc: Number,
    lines: Number,
    documents: BTreeMap<String, u64>,
    /// Every document the working tree holds and the commit does not, with the word count a
    /// NOTE names, so nothing that turns facts into lines has to read the tree.
    unjudged: Vec<(String, u64)>,
}

fn numbers(
    root: &Path,
    at: Option<&Path>,
    at_commit: Option<(&Survey, &str)>,
    found: &Survey,
    pinned: &Value,
) -> Numbers {
    let taken = at_commit
        .filter(|_| needs_ceilings(found, pinned))
        .map(|(held, commit)| (sample(root, at, commit, &Sampled::of(pinned, held)), commit));
    let counted = taken.map(|(sample, commit)| (sample.functions, commit));
    let (cc, lines) = taken.map_or((CC_FLOOR, LINES_FLOOR), |(at, _)| (at.cc, at.lines));
    let documents = match at_commit.filter(|_| wants_documents(found, pinned)) {
        Some((held, commit)) => document_ceilings(root, at, commit, held),
        None => BTreeMap::new(),
    };
    Numbers {
        cc: number(cc, CC_FLOOR, counted),
        lines: number(lines, LINES_FLOOR, counted),
        unjudged: unjudged(found, &documents, root, pinned),
        documents,
    }
}

/// Every document the working tree holds that the derivation commit does not, and the words it
/// holds now. A config that states the section judges every document it names, so there is
/// nothing to note. Spec 4.3, 5.4.
fn unjudged(
    found: &Survey,
    documents: &BTreeMap<String, u64>,
    root: &Path,
    pinned: &Value,
) -> Vec<(String, u64)> {
    if pins(pinned, DOC_SIZE, None) {
        return Vec::new();
    }
    found
        .documents
        .iter()
        .filter(|name| !documents.contains_key(*name))
        .map(|name| {
            let words = crate::doc_size::words(&root.join(name)).unwrap_or_default();
            (name.clone(), words)
        })
        .collect()
}

/// Whether this run needs the ceilings the derivation commit sets. It does not when the config
/// states the section itself, when it pins both ceilings, or when the working tree has no root
/// to measure, and then no commit is parsed at all.
fn needs_ceilings(found: &Survey, pinned: &Value) -> bool {
    !found.roots.is_empty()
        && !states(pinned, COMPLEXITY)
        && !["cc", "lines"].iter().all(|key| pins_ceiling(pinned, key))
}

/// The same question for the word ceilings, which only a tree with a document at its top and a
/// config that leaves the section to the survey needs.
fn wants_documents(found: &Survey, pinned: &Value) -> bool {
    !found.documents.is_empty() && !states(pinned, DOC_SIZE)
}

/// Whether the config states this section itself, leaving the survey nothing to supply.
fn states(pinned: &Value, section: &str) -> bool {
    pinned
        .get(section)
        .is_some_and(|held| pinned_whole(section, held))
}

/// One derived ceiling and the line that explains it: the percentile when the derivation commit
/// held enough functions for one, and the floor otherwise. Spec 5.4.
fn number(value: u64, floor: u64, counted: Option<(usize, &str)>) -> Number {
    let rule = match counted {
        None => "the floor, with no commit to measure".to_string(),
        Some((functions, commit)) if value == floor => format!(
            "the floor of {floor}, over {} function(s) at {}",
            grouped(functions),
            short(commit)
        ),
        Some((functions, commit)) => format!(
            "95th percentile of {} functions at {}, floor {floor}",
            grouped(functions),
            short(commit)
        ),
    };
    Number { value, rule }
}

/// What the derivation commit's functions come to, and how many of them there were, which is
/// what the `derived:` line names. Spec 5.4.
#[derive(Clone, Copy)]
struct Sample {
    cc: u64,
    lines: u64,
    functions: usize,
}

/// The files the complexity gate would measure, out of what one commit holds: the roots,
/// languages and exclusions that gate names. A file the gate never judges must not set the
/// ceiling the judged files are held to, or excluding generated code would loosen the gate.
/// Spec 5.4.
struct Sampled {
    roots: Vec<String>,
    extensions: Vec<&'static str>,
    exclude: Vec<String>,
    skip_dirs: Vec<String>,
}

impl Sampled {
    fn of(pinned: &Value, held: &Survey) -> Sampled {
        let named = |key: &str| named(pinned.get(COMPLEXITY), key);
        let roots = named("roots");
        Sampled {
            roots: match roots.is_empty() {
                true => held.roots.clone(),
                false => roots,
            },
            extensions: crate::complexity::extensions(&named("languages")),
            exclude: named("exclude"),
            skip_dirs: named("skip_dirs"),
        }
    }

    fn keeps(&self, path: &str) -> bool {
        self.roots.iter().any(|root| under_or_at(path, root))
            && self.extensions.iter().any(|end| path.ends_with(end))
            && !self.under_a_skipped_directory(path)
            && !self.excluded(path)
    }

    fn under_a_skipped_directory(&self, path: &str) -> bool {
        path.split('/')
            .any(|segment| self.skip_dirs.iter().any(|dir| dir == segment))
    }

    fn excluded(&self, path: &str) -> bool {
        self.exclude.iter().any(|glob| {
            files::glob_matches(glob.as_bytes(), basename(path).as_bytes())
                || files::glob_matches(glob.as_bytes(), path.as_bytes())
        })
    }
}

/// One list of strings a section names, and nothing for a key it leaves out or states as
/// something else. The gate itself refuses a malformed key, so the sample does not.
fn named(section: Option<&Value>, key: &str) -> Vec<String> {
    section
        .and_then(|held| held.get(key))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The sample from the cache when this binary wrote it, and from one parse of the commit
/// otherwise. The first stop after a commit pays that parse and every stop after it reads the
/// cache. Spec 6.6.
fn sample(root: &Path, at: Option<&Path>, commit: &str, sampled: &Sampled) -> Sample {
    if let Some(cached) = at
        .and_then(|at| cache::read(at, commit, COMPLEXITY))
        .and_then(|cached| read_sample(&cached))
    {
        return cached;
    }
    let found = counts(root, commit, sampled);
    if let Some(at) = at {
        cache::write(at, commit, COMPLEXITY, kept_sample(found));
    }
    found
}

/// Every function at the derivation commit, under the roots that commit's own survey holds. A
/// function that exists only in the working tree is not read here, so a directory that becomes
/// a root cannot move the ceiling it is judged against. One git process reads each file, which
/// the whole-tree parse of 6.6 pays for once per derivation commit. Spec 4.3, 5.4.
fn counts(root: &Path, commit: &str, sampled: &Sampled) -> Sample {
    let mut cc = Vec::new();
    let mut lines = Vec::new();
    for path in listed(root, commit).unwrap_or_default() {
        if !sampled.keeps(&path) {
            continue;
        }
        let Some(text) = crate::changed::blob(root, commit, &path) else {
            continue;
        };
        for function in crate::complexity::measured(&path, &String::from_utf8_lossy(&text)) {
            cc.push(function.cc);
            lines.push(function.lines);
        }
    }
    Sample {
        functions: cc.len(),
        cc: percentile(cc, CC_FLOOR),
        lines: percentile(lines, LINES_FLOOR),
    }
}

/// The nearest-rank 95th percentile: sort ascending and take the value at `ceil(0.95 * n)`,
/// never below the floor. Below the sample a percentile needs, the floor is the ceiling.
fn percentile(mut values: Vec<u64>, floor: u64) -> u64 {
    if values.len() < SAMPLE {
        return floor;
    }
    values.sort_unstable();
    floor.max(values[(values.len() * PERCENTILE).div_ceil(100) - 1])
}

fn read_sample(cached: &Value) -> Option<Sample> {
    let number = |key: &str| cached.get(key).and_then(Value::as_u64);
    Some(Sample {
        cc: number("cc")?,
        lines: number("lines")?,
        functions: usize::try_from(number("functions")?).ok()?,
    })
}

fn kept_sample(found: Sample) -> Value {
    let mut fields = Map::new();
    fields.insert("cc".into(), found.cc.into());
    fields.insert("lines".into(), found.lines.into());
    fields.insert("functions".into(), found.functions.into());
    Value::Object(fields)
}

/// One word ceiling per document the derivation commit holds: its word count there, rounded up
/// to the next 50, so an empty document gets 50 rather than a ceiling its first word breaks.
/// Spec 5.4.
fn document_ceilings(
    root: &Path,
    at: Option<&Path>,
    commit: &str,
    held: &Survey,
) -> BTreeMap<String, u64> {
    if let Some(cached) = at
        .and_then(|at| cache::read(at, commit, DOC_SIZE))
        .and_then(|cached| read_ceilings(&cached))
    {
        return cached;
    }
    let mut out = BTreeMap::new();
    for name in &held.documents {
        let words = held_words(root, commit, name).unwrap_or_default();
        out.insert(name.clone(), (words / CEILING_STEP + 1) * CEILING_STEP);
    }
    if let Some(at) = at {
        cache::write(at, commit, DOC_SIZE, kept_ceilings(&out));
    }
    out
}

/// The cached ceilings when every one of them is a number, because a file another hand edited
/// is no more this commit's derivation than one another version wrote.
fn read_ceilings(cached: &Value) -> Option<BTreeMap<String, u64>> {
    cached
        .as_object()?
        .iter()
        .map(|(name, words)| Some((name.clone(), words.as_u64()?)))
        .collect()
}

fn kept_ceilings(found: &BTreeMap<String, u64>) -> Value {
    Value::Object(
        found
            .iter()
            .map(|(name, ceiling)| (name.clone(), (*ceiling).into()))
            .collect(),
    )
}

fn short(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
}

/// A count as a person reads it, in groups of three digits.
fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (at, digit) in digits.char_indices() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
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
fn sections(found: &Survey, numbers: &Numbers, pinned: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    let mut add = |name: &str, value: Option<Value>| {
        if let Some(value) = value {
            out.insert(name.to_string(), merged(pinned.get(name), value));
        }
    };
    add("escapes", escapes_section(found));
    add("stubs", stubs_section(found));
    add("complexity", complexity_section(found, numbers));
    add("doc_size", doc_size_section(found, numbers));
    add("doc_citations", doc_citations_section(found));
    add("inventory", inventory_section(found));
    add("lockfile", lockfile_section(found));
    add("build", build_section(found));
    out
}

/// What the config pins wins over what the survey found, key by key at any depth where both
/// are objects and whole otherwise, so a config may pin one ceiling and leave the other to the
/// survey.
fn merged(pinned: Option<&Value>, derived: Value) -> Value {
    match (pinned, derived) {
        (Some(Value::Object(pins)), Value::Object(fields)) => key_by_key(pins, fields),
        (Some(pinned), _) => pinned.clone(),
        (None, derived) => derived,
    }
}

fn key_by_key(pins: &Map<String, Value>, mut fields: Map<String, Value>) -> Value {
    for (key, value) in pins {
        let held = match fields.remove(key) {
            Some(held) => merged(Some(value), held),
            None => value.clone(),
        };
        fields.insert(key.clone(), held);
    }
    Value::Object(fields)
}

fn escapes_section(found: &Survey) -> Option<Value> {
    markers_section(&found.roots, &found.languages)
}

/// The stubs table names fewer languages than the escapes table, and a section naming one it
/// lacks would refuse every run, so the survey leaves such a language out. Spec 5.4.
fn stubs_section(found: &Survey) -> Option<Value> {
    let languages: Vec<String> = found
        .languages
        .iter()
        .filter(|language| stubs::holds_rows_for(language))
        .cloned()
        .collect();
    markers_section(&found.roots, &languages)
}

fn markers_section(roots: &[String], languages: &[String]) -> Option<Value> {
    if roots.is_empty() || languages.is_empty() {
        return None;
    }
    let mut section = Map::new();
    section.insert(reference::ROOTS.name.into(), list(roots));
    section.insert(reference::LANGUAGES.name.into(), list(languages));
    Some(Value::Object(section))
}

fn complexity_section(found: &Survey, numbers: &Numbers) -> Option<Value> {
    if found.roots.is_empty() {
        return None;
    }
    let mut ceilings = Map::new();
    ceilings.insert(complexity::CC.inner().into(), numbers.cc.value.into());
    ceilings.insert(complexity::LINES.inner().into(), numbers.lines.value.into());
    let mut section = Map::new();
    section.insert(reference::ROOTS.name.into(), list(&found.roots));
    section.insert(complexity::CEILINGS.into(), Value::Object(ceilings));
    Some(Value::Object(section))
}

/// One entry per document the derivation commit holds and the working tree still has. A
/// document only the working tree holds has no ceiling this run and is not judged, because
/// the only number it could be given is one read out of the working tree, which 4.3 forbids.
/// A tree whose documents are all new therefore gates on none of them, and the gate is still
/// there, so a run under `--strict` has its decision. Spec 4.3, 5.4.
fn doc_size_section(found: &Survey, numbers: &Numbers) -> Option<Value> {
    if found.documents.is_empty() {
        return None;
    }
    let entries: Vec<Value> = found
        .documents
        .iter()
        .filter_map(|name| Some((name, numbers.documents.get(name)?)))
        .map(|(name, ceiling)| {
            let mut entry = Map::new();
            entry.insert(doc_size::FILE.name.into(), name.clone().into());
            entry.insert(doc_size::CEILING.name.into(), (*ceiling).into());
            Value::Object(entry)
        })
        .collect();
    Some(Value::Array(entries))
}

fn held_words(root: &Path, commit: &str, name: &str) -> Option<u64> {
    let text = crate::changed::blob(root, commit, name)?;
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
                entry.insert(doc_citations::FILE.name.into(), name.clone().into());
                entry.insert(doc_citations::ROOTS.name.into(), list(&[ROOT.to_string()]));
                Value::Object(entry)
            })
            .collect(),
    ))
}

/// One entry per test root the survey found, named after the root, with no pattern. Spec 5.4.
fn inventory_section(found: &Survey) -> Option<Value> {
    if found.test_roots.is_empty() {
        return None;
    }
    Some(Value::Array(
        found
            .test_roots
            .iter()
            .map(|root| {
                let mut entry = Map::new();
                entry.insert(inventory::NAME.name.into(), root.clone().into());
                entry.insert(inventory::PATH.name.into(), root.clone().into());
                Value::Object(entry)
            })
            .collect(),
    ))
}

/// One entry per manifest klin has a lockfile reader for. A manifest of another ecosystem is
/// left out, because the check would only say it cannot read it. Spec 5.4, 8.2.1.
fn lockfile_section(found: &Survey) -> Option<Value> {
    let read: Vec<String> = found
        .manifests
        .iter()
        .filter(|path| crate::lockfile::reads(basename(path)))
        .cloned()
        .collect();
    if read.is_empty() {
        return None;
    }
    let mut section = Map::new();
    section.insert(lockfile::MANIFESTS.name.into(), list(&read));
    Some(Value::Object(section))
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
        out.insert(build::ROOT.into(), at.into());
    }
    out.insert(build::RUN.into(), run.into());
    Value::Object(out)
}

/// One line per derived value, and one per value the config pinned beside a derived one, so a
/// run says where every number and every path set came from. A section the config states in
/// full derived nothing and prints nothing. Spec 4.3, 5.2.
fn lines(
    found: &Survey,
    sections: &Map<String, Value>,
    numbers: &Numbers,
    pinned: &Value,
) -> Vec<Said> {
    let mut out = Vec::new();
    for (name, value) in sections {
        match value {
            Value::Object(fields) => {
                if states(pinned, name) {
                    continue;
                }
                out.extend(keys_of(name, fields, numbers, pinned));
            }
            Value::Array(entries) if entries.is_empty() => continue,
            _ if pins(pinned, name, None) => continue,
            _ => out.push(said(name, None, value, false)),
        }
    }
    out.extend(
        noted(&numbers.unjudged)
            .into_iter()
            .map(|line| Said { line, entry: None }),
    );
    if !found.test_roots.is_empty() {
        let rule = "the roots that match a language's test convention";
        let value = list(&found.test_roots);
        out.push(Said {
            line: format!(
                "derived: {TEST_ROOTS} {}, {rule}",
                found.test_roots.join(", ")
            ),
            entry: Some(derived_value(
                inventory::SECTION,
                Some(TEST_ROOTS),
                &value,
                rule,
            )),
        });
    }
    out
}

/// One line per key of a section the survey filled in, with the two complexity ceilings as
/// lines of their own so each says what set it.
fn keys_of(
    name: &str,
    fields: &Map<String, Value>,
    numbers: &Numbers,
    pinned: &Value,
) -> Vec<Said> {
    fields
        .iter()
        .flat_map(|(key, held)| match (name, key.as_str()) {
            (COMPLEXITY, "ceilings") => ceiling_lines(held, numbers, pinned),
            _ => vec![said(name, Some(key), held, pins(pinned, name, Some(key)))],
        })
        .collect()
}

/// The two complexity ceilings, each on its own line, so a run says what set each number and
/// whether a person pinned it. Pinning one and leaving the other to the survey is allowed, and
/// then one line says `pinned` and the other `derived`. Spec 4.3, 5.4.
fn ceiling_lines(held: &Value, numbers: &Numbers, pinned: &Value) -> Vec<Said> {
    [("cc", &numbers.cc), ("lines", &numbers.lines)]
        .iter()
        .filter_map(|(key, number)| {
            let raw = held.get(key)?;
            let value = shown(raw);
            Some(match pins_ceiling(pinned, key) {
                true => Said {
                    line: format!("pinned: {COMPLEXITY} {key} {value}"),
                    entry: None,
                },
                false => Said {
                    line: format!("derived: {COMPLEXITY} {key} {value} ({})", number.rule),
                    entry: Some(derived_value(COMPLEXITY, Some(key), raw, &number.rule)),
                },
            })
        })
        .collect()
}

fn pins_ceiling(pinned: &Value, key: &str) -> bool {
    pins(pinned, COMPLEXITY, Some(&format!("ceilings.{key}")))
}

/// One NOTE per document the derivation commit does not hold, naming its word count, because a
/// document klin does not judge on this run must still say so. Spec 4.3, 5.4.
fn noted(unjudged: &[(String, u64)]) -> Vec<String> {
    unjudged
        .iter()
        .map(|(name, words)| {
            format!(
                "NOTE: {DOC_SIZE} {name} is {words} words and is not judged — it gets a ceiling \
                 when the stamp moves and the derivation commit holds it"
            )
        })
        .collect()
}

/// A `pinned:` or `derived:` line, and the `{section, key, value, rule}` entry beside a
/// `derived:` one, built together so the two cannot say different things. Spec 11.2.
struct Said {
    line: String,
    entry: Option<Value>,
}

fn said(section: &str, key: Option<&str>, value: &Value, pinned: bool) -> Said {
    let named = match key {
        Some(key) => format!("{section} {key}"),
        None => section.to_string(),
    };
    if pinned {
        return Said {
            line: format!("pinned: {named} {}", shown(value)),
            entry: None,
        };
    }
    let rule = rule(section, key);
    Said {
        line: format!("derived: {named} {}, {rule}", shown(value)),
        entry: Some(derived_value(section, key, value, rule)),
    }
}

/// The `{section, key, value, rule}` entry `--json` prints beside a `derived:` line, built from
/// the same facts so the two cannot disagree. `key` is `null` for a section with no keys of its
/// own. Spec 11.2.
fn derived_value(section: &str, key: Option<&str>, value: &Value, rule: &str) -> Value {
    let mut out = Map::new();
    out.insert("section".into(), section.into());
    out.insert("key".into(), key.map_or(Value::Null, Value::from));
    out.insert("value".into(), value.clone());
    out.insert("rule".into(), rule.into());
    Value::Object(out)
}

fn rule(section: &str, key: Option<&str>) -> &'static str {
    match (section, key) {
        (_, Some("roots")) => "the shallowest directories that hold nothing but source",
        (_, Some("languages")) => "the languages of the files under those roots",
        (_, Some("manifests")) => {
            "the manifests the survey found that klin can read a lockfile for"
        }
        ("doc_size", None) => {
            "every Markdown file at the tree root the derivation commit holds, each ceiling its \
             word count there rounded up to the next 50"
        }
        ("doc_citations", None) => "every Markdown file at the tree root, resolved against it",
        ("inventory", None) => "one entry per test root",
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
    key.is_none_or(|key| stated(held, key))
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
            .map(|item| {
                match item
                    .get("file")
                    .or_else(|| item.get("run"))
                    .or_else(|| item.get("name"))
                {
                    Some(Value::String(text)) => text.clone(),
                    _ => item.to_string(),
                }
            })
            .collect::<Vec<String>>()
            .join(", "),
        other => other.to_string(),
    }
}
