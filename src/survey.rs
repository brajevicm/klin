use std::cell::OnceCell;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::changed::git;
use crate::project::{self, Tree};
use crate::scope::{ROOT, under_or_at};
use crate::{
    build, cache, check, complexity, config, dead_symbols, doc_citations, doc_size, escapes, files,
    inventory, lockfile, markers, reachability, reference, state, stubs, syntax, turn,
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

/// What one run derives: the facts of the two trees, read once, and each section and number
/// the config leaves out, computed on the first call that asks for it and never for a section
/// no selected check reads. A run that names one gate derives that gate's values and no other's.
/// Spec 4.3, ADR 0038.
pub struct Derived {
    root: PathBuf,
    /// The state directory the caches sit in, and `None` where klin keeps none.
    at: Option<PathBuf>,
    commit: Option<String>,
    /// The derivation commit's own survey, and `None` where there is no commit to survey.
    held: Option<Survey>,
    /// The union of that survey and the working tree's, kept to what the working tree holds.
    found: Survey,
    pinned: Value,
    /// Every derivable section, in the order the lines print, and one cell per section.
    names: Vec<&'static str>,
    sections: Vec<OnceCell<Option<Value>>>,
    /// The complexity sample of the derivation commit, and `None` where no run needs it.
    sample: OnceCell<Option<(Sample, String)>>,
    documents: OnceCell<BTreeMap<String, u64>>,
    families: OnceCell<Option<Value>>,
    /// The derived roots the derivation commit's survey did not hold. A site under one matches
    /// nothing in `before`, so a directory that becomes a root brings no inherited debt with
    /// it. Empty when there is no commit to survey. Spec 7.1.
    pub unheld: Vec<String>,
    /// The source roots the survey found. Empty is the hole of ADR 0016: a gate the survey was
    /// left to supply reads code, and there is no code to read. Spec 10, 14.
    pub roots: Vec<String>,
}

/// The keys klin derives for a section, and `None` for a section it never derives. Every check
/// says this on its own catalogue row; `build` is the one derivable section no check reads. A
/// section the config pins whole is never derived, so the survey does not run for it.
pub fn keys(section: &str) -> Option<&'static [&'static str]> {
    match section == config::BUILD.name {
        true => Some(&[]),
        false => check::derives(section),
    }
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

fn leaf(key: &str) -> &str {
    key.rsplit_once('.').map_or(key, |(_, last)| last)
}

pub fn derivable() -> impl Iterator<Item = &'static str> {
    std::iter::once(config::BUILD.name).chain(
        check::CATALOGUE
            .iter()
            .filter(|spec| spec.derives.is_some())
            .map(|spec| spec.section),
    )
}

/// The facts every derivable value is read from: the derivation commit's survey and a walk of
/// the working tree, as one union kept to what the working tree still holds, because a root it
/// no longer has has nothing to measure. Nothing expensive is computed here. Spec 4.3.
pub fn derive(tree: &Tree, pinned: &Value) -> Derived {
    let root = tree.root();
    let at = state::ready(root).ok();
    let commit = turn::derivation(root, at.as_deref());
    let surveyed = at_commit(root, at.as_deref(), commit.as_deref());
    let held = surveyed.clone().unwrap_or_default();
    let found = union(&held, &walked(tree), root);
    let unheld = unheld(&found, &held, surveyed.is_some());
    let mut names: Vec<&'static str> = derivable().collect();
    names.sort_unstable();
    Derived {
        root: root.to_path_buf(),
        at,
        commit,
        held: surveyed,
        roots: found.roots.clone(),
        found,
        pinned: pinned.clone(),
        sections: names.iter().map(|_| OnceCell::new()).collect(),
        names,
        sample: OnceCell::new(),
        documents: OnceCell::new(),
        families: OnceCell::new(),
        unheld,
    }
}

impl Derived {
    fn at_commit(&self) -> Option<(&Survey, &str)> {
        self.held.as_ref().zip(self.commit.as_deref())
    }

    /// The section the survey supplies under this name, merged under what the config pins, and
    /// `None` where the tree gives the survey nothing to say. Computed once. Spec 5.2.
    pub fn section(&self, name: &str) -> Option<&Value> {
        let at = self.names.iter().position(|held| *held == name)?;
        self.sections[at]
            .get_or_init(|| self.computed(name))
            .as_ref()
    }

    fn computed(&self, name: &str) -> Option<Value> {
        let (_, derive) = SECTIONS.iter().find(|(section, _)| *section == name)?;
        Some(merged(self.pinned.get(name), derive(self)?))
    }

    /// Whether the survey supplies this section at all, from the facts alone where a number
    /// would otherwise be computed only to answer yes. Planning a run asks this. Spec 10.
    pub fn supplies(&self, name: &str) -> bool {
        match name {
            COMPLEXITY => !self.found.roots.is_empty(),
            DOC_SIZE => !self.found.documents.is_empty(),
            _ => self.section(name).is_some(),
        }
    }

    /// The two complexity ceilings the derivation commit sets, and the line each is explained
    /// by. Parsed once per commit and cached under it, the first time a run needs them.
    fn ceilings(&self) -> (Number, Number) {
        let taken = self.sample.get_or_init(|| {
            self.at_commit()
                .filter(|_| needs_ceilings(&self.found, &self.pinned))
                .map(|(held, commit)| {
                    let sampled = Sampled::of(&self.pinned, held);
                    (
                        sample(&self.root, self.at.as_deref(), commit, &sampled),
                        commit.to_string(),
                    )
                })
        });
        let counted = taken
            .as_ref()
            .map(|(sample, commit)| (sample.functions, commit.as_str()));
        let (cc, lines) = taken
            .as_ref()
            .map_or((CC_FLOOR, LINES_FLOOR), |(at, _)| (at.cc, at.lines));
        (
            number(cc, CC_FLOOR, counted),
            number(lines, LINES_FLOOR, counted),
        )
    }

    /// One word ceiling per document the derivation commit holds, read once per commit and
    /// cached under it. A document the commit lacks is not here. Spec 5.4.
    fn document_ceilings(&self) -> &BTreeMap<String, u64> {
        self.documents.get_or_init(|| {
            match self
                .at_commit()
                .filter(|_| wants_documents(&self.found, &self.pinned))
            {
                Some((held, commit)) => {
                    document_ceilings(&self.root, self.at.as_deref(), commit, held)
                }
                None => BTreeMap::new(),
            }
        })
    }

    /// The reachability families the commit proves, and `None` where it proves none or the
    /// config states the section. Policy read from the commit alone, never from the tree.
    fn families(&self) -> &Option<Value> {
        self.families.get_or_init(|| {
            self.at_commit()
                .filter(|_| !states(&self.pinned, reachability::SECTION))
                .and_then(|(held, commit)| {
                    reachability::derived(&self.root, self.at.as_deref(), commit, held)
                })
        })
    }

    /// One `derived:` or `pinned:` line per value, for the sections named and for every
    /// section when none is. Spec 4.3.
    pub fn lines(&self, only: Option<&[&str]>) -> Vec<String> {
        self.said(only).into_iter().map(|said| said.line).collect()
    }

    /// The `{section, key, value, rule}` entry beside each `derived:` line, built by the same
    /// pass so the two cannot say different things. Spec 11.2.
    pub fn values(&self, only: Option<&[&str]>) -> Vec<Value> {
        self.said(only)
            .into_iter()
            .filter_map(|said| said.entry)
            .collect()
    }
}

/// How each derivable section is read off the facts. The three that need a number of the
/// derivation commit ask the cell that holds it, so the number is computed for that section
/// and no other.
type Derive = fn(&Derived) -> Option<Value>;

const SECTIONS: &[(&str, Derive)] = &[
    (escapes::SECTION, |derived| escapes_section(&derived.found)),
    (stubs::SECTION, |derived| stubs_section(&derived.found)),
    (dead_symbols::SECTION, |derived| {
        dead_symbols_section(&derived.found)
    }),
    (reachability::SECTION, |derived| derived.families().clone()),
    (COMPLEXITY, |derived| {
        complexity_section(&derived.found, &derived.ceilings())
    }),
    (DOC_SIZE, |derived| {
        doc_size_section(&derived.found, derived.document_ceilings())
    }),
    (doc_citations::SECTION, |derived| {
        doc_citations_section(&derived.found)
    }),
    (inventory::SECTION, |derived| {
        inventory_section(&derived.found)
    }),
    (lockfile::SECTION, |derived| {
        lockfile_section(&derived.found)
    }),
    (config::BUILD.name, |derived| build_section(&derived.found)),
];

/// A number one commit sets, and what the line that prints it says about where it came from.
struct Number {
    value: u64,
    rule: String,
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
            extensions: crate::syntax::extensions(&named("languages")),
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
/// a root cannot move the ceiling it is judged against. One git process reads every file, which
/// the whole-tree parse of 6.6 pays for once per derivation commit. A read git did not finish
/// is no sample at all, as a commit git could not list is, so a torn read never lowers a
/// ceiling. Spec 4.3, 5.4.
fn counts(root: &Path, commit: &str, sampled: &Sampled) -> Sample {
    let mut cc = Vec::new();
    let mut lines = Vec::new();
    let listed = listed(root, commit).unwrap_or_default();
    let kept: Vec<&str> = listed
        .iter()
        .map(String::as_str)
        .filter(|path| sampled.keeps(path))
        .collect();
    let read = crate::changed::blobs(root, commit, &kept, |path, text| {
        let Some(text) = text else {
            return;
        };
        for function in crate::complexity::measured(path, &String::from_utf8_lossy(text)) {
            cc.push(function.cc);
            lines.push(function.lines);
        }
    });
    if read.is_none() {
        cc.clear();
        lines.clear();
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
pub(crate) fn listed(root: &Path, commit: &str) -> Option<Vec<String>> {
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

/// Every path the working tree holds, off the tree's one file list, which prunes the default
/// skip set and every path `.gitignore` excludes and reads names only. Spec 4.3.
fn walked(tree: &Tree) -> Survey {
    let files = tree.files().unwrap_or_default();
    of(&files
        .iter()
        .filter(|path| surveyed(path))
        .cloned()
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

/// Whether a path is source, which is a fact of the path and no check's opinion. ADR 0038.
fn source(path: &str) -> bool {
    project::language_of(path).is_some()
}

fn languages(paths: &[String], roots: &[String]) -> Vec<String> {
    sorted(
        paths
            .iter()
            .filter(|path| roots.iter().any(|root| under_or_at(path, root)))
            .filter_map(|path| project::language_of(path))
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

/// The escapes table names its rows by the languages the tree is classified into, and a
/// language it holds no rows for would refuse every run, so the survey leaves such a language
/// out. Spec 5.4.
fn escapes_section(found: &Survey) -> Option<Value> {
    let languages: Vec<String> = found
        .languages
        .iter()
        .filter(|language| markers::holds_rows_for(&escapes::KIND, language))
        .cloned()
        .collect();
    markers_section(&found.roots, &languages)
}

/// The stubs table names fewer languages than the escapes table, and a section naming one it
/// lacks would refuse every run, so the survey leaves such a language out. Spec 5.4.
fn stubs_section(found: &Survey) -> Option<Value> {
    let languages: Vec<String> = found
        .languages
        .iter()
        .filter(|language| markers::holds_rows_for(&stubs::KIND, language))
        .cloned()
        .collect();
    markers_section(&found.roots, &languages)
}

/// The structural table names only the languages its adapters can measure, so an unrelated
/// parser-backed language does not make a default derived gate claim it measured nothing.
fn dead_symbols_section(found: &Survey) -> Option<Value> {
    let supported: Vec<&str> = syntax::structural::languages()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let languages: Vec<String> = found
        .languages
        .iter()
        .filter(|language| supported.contains(&language.as_str()))
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

fn complexity_section(found: &Survey, (cc, lines): &(Number, Number)) -> Option<Value> {
    if found.roots.is_empty() {
        return None;
    }
    let mut ceilings = Map::new();
    ceilings.insert(complexity::CC.inner().into(), cc.value.into());
    ceilings.insert(complexity::LINES.inner().into(), lines.value.into());
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
fn doc_size_section(found: &Survey, ceilings: &BTreeMap<String, u64>) -> Option<Value> {
    if found.documents.is_empty() {
        return None;
    }
    let entries: Vec<Value> = found
        .documents
        .iter()
        .filter_map(|name| Some((name, ceilings.get(name)?)))
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

impl Derived {
    /// One line per derived value, and one per value the config pinned beside a derived one, so
    /// a run says where every number and every path set came from. A section the config states
    /// in full derived nothing and prints nothing, and a section no selected gate reads is not
    /// derived for its line. Spec 4.3, 5.2.
    fn said(&self, only: Option<&[&str]>) -> Vec<Said> {
        let wanted = |name: &str| only.is_none_or(|only| only.contains(&name));
        let mut out = Vec::new();
        for name in self.names.iter().filter(|name| wanted(name)) {
            out.extend(self.section_said(name));
        }
        if wanted(DOC_SIZE) {
            out.extend(self.unjudged_said());
        }
        if wanted(inventory::SECTION) {
            out.extend(self.test_roots_said());
        }
        out
    }

    /// The lines of one section: one per key of an object the survey filled in, one for a
    /// section with no keys of its own, and none for a section the config states.
    fn section_said(&self, name: &str) -> Vec<Said> {
        let Some(value) = self.section(name) else {
            return Vec::new();
        };
        match value {
            Value::Object(_) if states(&self.pinned, name) => Vec::new(),
            Value::Object(fields) => self.keys_of(name, fields),
            Value::Array(entries) if entries.is_empty() => Vec::new(),
            _ if pins(&self.pinned, name, None) => Vec::new(),
            _ => vec![said(name, None, value, false)],
        }
    }

    /// One NOTE per document the derivation commit does not hold. Spec 4.3, 5.4.
    fn unjudged_said(&self) -> Vec<Said> {
        let unjudged = unjudged(
            &self.found,
            self.document_ceilings(),
            &self.root,
            &self.pinned,
        );
        noted(&unjudged)
            .into_iter()
            .map(|line| Said { line, entry: None })
            .collect()
    }

    /// The test-root set, which is a derived value of 4.3 and no key a config states.
    fn test_roots_said(&self) -> Vec<Said> {
        if self.found.test_roots.is_empty() {
            return Vec::new();
        }
        let rule = "the roots that match a language's test convention";
        let value = list(&self.found.test_roots);
        vec![Said {
            line: format!(
                "derived: {TEST_ROOTS} {}, {rule}",
                self.found.test_roots.join(", ")
            ),
            entry: Some(derived_value(
                inventory::SECTION,
                Some(TEST_ROOTS),
                &value,
                rule,
            )),
        }]
    }

    /// One line per key of a section the survey filled in, with the two complexity ceilings as
    /// lines of their own so each says what set it.
    fn keys_of(&self, name: &str, fields: &Map<String, Value>) -> Vec<Said> {
        fields
            .iter()
            .flat_map(|(key, held)| match (name, key.as_str()) {
                (COMPLEXITY, "ceilings") => ceiling_lines(held, &self.ceilings(), &self.pinned),
                _ => vec![said(
                    name,
                    Some(key),
                    held,
                    pins(&self.pinned, name, Some(key)),
                )],
            })
            .collect()
    }
}

/// The two complexity ceilings, each on its own line, so a run says what set each number and
/// whether a person pinned it. Pinning one and leaving the other to the survey is allowed, and
/// then one line says `pinned` and the other `derived`. Spec 4.3, 5.4.
fn ceiling_lines(held: &Value, (cc, lines): &(Number, Number), pinned: &Value) -> Vec<Said> {
    [("cc", cc), ("lines", lines)]
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

/// The rule each derived value is explained by: a key's rule holds in every section, and a
/// section with no keys of its own has one rule for the whole section.
const RULES: &[(Option<&str>, Option<&str>, &str)] = &[
    (
        None,
        Some("roots"),
        "the shallowest directories that hold nothing but source",
    ),
    (
        None,
        Some("languages"),
        "the languages of the files under those roots",
    ),
    (
        None,
        Some("manifests"),
        "the manifests the survey found that klin can read a lockfile for",
    ),
    (
        Some("doc_size"),
        None,
        "every Markdown file at the tree root the derivation commit holds, each ceiling its \
         word count there rounded up to the next 50",
    ),
    (
        Some("doc_citations"),
        None,
        "every Markdown file at the tree root, resolved against it",
    ),
    (Some("inventory"), None, "one entry per test root"),
    (
        Some("reachability"),
        None,
        "one family per directory whose files share a name prefix or suffix and one extension, \
         where the derivation commit proves every member reached without ambiguity",
    ),
    (Some("build"), None, "one command per manifest"),
];

fn rule(section: &str, key: Option<&str>) -> &'static str {
    RULES
        .iter()
        .find(|(of, named, _)| of.is_none_or(|of| of == section) && *named == key)
        .map_or("the survey of this tree", |(_, _, rule)| rule)
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
