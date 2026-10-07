//! `doc-citations` judges the backticked paths a document cites. Every Markdown file at the
//! tree root is read, and a citation resolves against the whole tree with the built-in
//! extension list; the section reads no policy. A site is the document and the cited path, and
//! its `count` rises when a path that resolves nowhere is cited again. A document the base
//! holds is compared against the base's copy, so a stale citation the base holds is held.
//! Spec 5.4, 8.2.1, ADR 0040.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::changed;
use crate::check::contract::{self, Context, Sink};
use crate::config::Config;
use crate::coverage::Coverage;
use crate::error::Error;
use crate::files;
use crate::git::Repo;
use crate::key::Key;
use crate::ratchet::{self, Evaluator, Finding, Line, Remedy};
use crate::record::Values;
use crate::tree::Tree;

pub const SECTION: &str = "doc_citations";

/// What a person may write instead of a policy, which this section does not read. Spec 5.4.
pub const POLICY: &str =
    "documents and citation roots are discovered; remove the section, or set it to false";

/// The section reads no keys: it is absent, or `false`. Spec 5.4, 5.8, ADR 0040.
pub const KEYS: &[Key] = &[];

const RULE: &str = "every Markdown file at the tree root, resolved against it";

/// The extensions a citation may name, which `klin policy --reference` prints.
pub const EXTENSIONS: &[&str] = &[
    ".py", ".ts", ".tsx", ".js", ".jsx", ".swift", ".rs", ".go", ".kt", ".java", ".rb", ".sh",
    ".md", ".json", ".yml", ".yaml", ".toml",
];
const EVERY_FILE: &str = "";
const REMEDY: &str = "Point the citation at where the file is now (a bare filename resolves \
    when exactly one file under the roots has that name). A move or rename calls for updating \
    the citation. Delete the sentence only when the referenced content was intentionally removed \
    and the sentence no longer applies.";

struct Document {
    path: PathBuf,
    roots: Vec<PathBuf>,
    name: String,
}

/// The documents a run judges, and the directory their paths and the base commit are read from.
struct Listing<'a> {
    documents: Vec<Document>,
    root: PathBuf,
    config: Option<&'a Config>,
}

/// What a citation may resolve against in one tree: the paths a root holds, and the file names
/// under it, so a bare filename cited without its directory resolves when it is unique.
struct Tally {
    line: u64,
    resolution: String,
    count: u64,
}

struct Index {
    /// The base tree's paths, listed out of git. `None` in the working tree, which is on disk.
    listed: Option<BTreeSet<String>>,
    basenames: BTreeMap<String, Vec<String>>,
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    evaluate(at, None, &[], out)
}

fn evaluate(
    at: &Context,
    named: Option<&Path>,
    roots: &[PathBuf],
    out: &mut Sink,
) -> Result<u8, Error> {
    let listing = listing(at, named, roots);
    if named.is_none() {
        said(&listing, out);
    }
    let commit = contract::base_commit(&listing.root, at)?;
    let (now, before) = sides(&listing, at.project.tree(), &commit)?;
    let sites = now.len();
    let accepted = match &listing.config {
        Some(config) => ratchet::accepted(config, at.gate, evaluator().metrics)?,
        None => Vec::new(),
    };
    let said = out.covered(&covered(&listing));
    Ok(evaluator().evaluate(
        now,
        before,
        accepted,
        at,
        Line {
            state: &format!("{sites} citation(s) resolve nowhere"),
            tail: &said,
        },
        out,
    ))
}

/// The documents a run read, as the one `derived:` line and its JSON entry.
fn said(listing: &Listing, out: &mut Sink) {
    if listing.documents.is_empty() {
        return;
    }
    let names: Vec<&str> = listing
        .documents
        .iter()
        .map(|document| document.name.as_str())
        .collect();
    out.provenance(
        format!("derived: {SECTION} {}, {RULE}", names.join(", ")),
        Some(contract::derived_entry(SECTION, None, names.into(), RULE)),
    );
}

/// What this gate discovered: one document per entry, and the ones it read. A document the
/// working tree no longer holds is found and not measured, because only the base holds its
/// citations. Spec 8.6.
fn covered(listing: &Listing) -> Coverage {
    let read = listing
        .documents
        .iter()
        .filter(|document| document.path.exists())
        .count();
    Coverage {
        found: listing.documents.len(),
        measured: read,
        not_measured: 0,
        excluded: 0,
        unreadable: 0,
    }
}

/// What each document cites and resolves nowhere today, and what it did at the base.
fn sides(
    listing: &Listing,
    tree: &Tree,
    commit: &str,
) -> Result<(Vec<Finding>, Vec<Finding>), Error> {
    let (mut now, mut before) = (Vec::new(), Vec::new());
    let mut trees: HashMap<Vec<PathBuf>, Index> = HashMap::new();
    let mut priors: HashMap<Vec<PathBuf>, Index> = HashMap::new();
    for document in &listing.documents {
        roots_exist(document)?;
        if document.path.exists() {
            let text = std::fs::read(&document.path)
                .map_err(|why| Error::unreadable(&document.path, why))?;
            let index = cached(&mut trees, &document.roots, || {
                working(tree, &document.roots)
            })?;
            now.extend(found(document, &String::from_utf8_lossy(&text), index));
        }
        let repo = repo_path(&document.path, &listing.root);
        let Some(was) = changed::blob(&listing.root, commit, &repo) else {
            continue;
        };
        let index = cached(&mut priors, &document.roots, || {
            at_the_base(&document.roots, commit)
        })?;
        before.extend(found(document, &String::from_utf8_lossy(&was), index));
    }
    Ok((now, before))
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &["count"],
        unit: "citation(s)",
        condition: "where a document cites a file that resolves nowhere",
        fix_advice: Remedy::Fixed(REMEDY),
        ceiling: None,
        format_metrics: show,
        nested: None,
    }
}

fn show(values: &Values) -> String {
    let why = values
        .get("resolution")
        .and_then(Value::as_str)
        .unwrap_or("?");
    match values.get("count").and_then(Value::as_u64) {
        Some(count) if count > 1 => format!("{why} x{count}"),
        _ => why.to_string(),
    }
}

/// The base commit the runner chose, or the one this command chooses for itself.
fn roots_exist(document: &Document) -> Result<(), Error> {
    for root in &document.roots {
        if !root.is_dir() {
            return Err(Error(format!("no such directory: {}", root.display())));
        }
    }
    Ok(())
}

/// One index per distinct set of roots in a run, not one per document — several entries
/// sharing `"roots": ["."]` walk the tree once, not once each.
fn cached<'a>(
    held: &'a mut HashMap<Vec<PathBuf>, Index>,
    roots: &[PathBuf],
    build: impl FnOnce() -> Result<Index, Error>,
) -> Result<&'a Index, Error> {
    if !held.contains_key(roots) {
        held.insert(roots.to_vec(), build()?);
    }
    Ok(&held[roots])
}

/// One finding per cited path in one document that resolves nowhere, keyed by the string as
/// written, so the same string on another line is the same site.
fn found(document: &Document, text: &str, index: &Index) -> Vec<Finding> {
    let mut seen: BTreeMap<String, Tally> = BTreeMap::new();
    for (path, line) in citations(text) {
        let Some(resolution) = resolves(&path, &document.roots, index) else {
            continue;
        };
        seen.entry(path)
            .and_modify(|tally| tally.count += 1)
            .or_insert(Tally {
                line,
                resolution,
                count: 1,
            });
    }
    let mut out: Vec<Finding> = seen
        .into_iter()
        .map(|(text, tally)| {
            let mut values = Values::new();
            values.insert("resolution".into(), tally.resolution.into());
            values.insert("count".into(), tally.count.into());
            Finding {
                file: document.name.clone(),
                line: tally.line,
                text,
                values,
                body: None,
            }
        })
        .collect();
    out.sort_by_key(|finding| finding.line);
    out
}

/// Every backticked span on each line that looks like a path with one of the built-in
/// extensions — no spaces, no `*`, and a `/` or a `.` — with a trailing `:line` suffix stripped.
fn citations(text: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for (number, line) in text.split('\n').enumerate() {
        let ticks: Vec<usize> = line.match_indices('`').map(|(at, _)| at).collect();
        for pair in ticks.as_chunks::<2>().0 {
            if let Some(candidate) = candidate(&line[pair[0] + 1..pair[1]]) {
                out.push((candidate, number as u64 + 1));
            }
        }
    }
    out
}

fn candidate(span: &str) -> Option<String> {
    let candidate = span.trim().split(':').next().unwrap_or("");
    if candidate.contains(' ') || candidate.contains('*') {
        return None;
    }
    if !EXTENSIONS
        .iter()
        .any(|extension| candidate.ends_with(extension))
    {
        return None;
    }
    (candidate.contains('/') || candidate.contains('.')).then(|| candidate.to_string())
}

fn working(tree: &Tree, roots: &[PathBuf]) -> Result<Index, Error> {
    let wanted = files::Wanted {
        extensions: &[EVERY_FILE],
        skip_dirs: &files::default_skip_dirs(),
        exclude: &[],
        exclude_except: &[],
        skip_hidden: false,
    };
    let mut index = Index {
        listed: None,
        basenames: BTreeMap::new(),
    };
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for file in files::under(
            tree.root(),
            || tree.files(),
            std::slice::from_ref(root),
            &wanted,
        )? {
            index.add(&files::relative(&file, root));
        }
    }
    index.sort_basenames();
    Ok(index)
}

/// The base tree's file list, read out of git so no worktree is needed under `--changed`. A
/// root the base does not hold lists nothing, but a listing git refuses is an error: an empty
/// base would hold every stale citation in the tree and report green while measuring nothing.
fn at_the_base(roots: &[PathBuf], commit: &str) -> Result<Index, Error> {
    let skipped = files::default_skip_dirs();
    let mut index = Index {
        listed: Some(BTreeSet::new()),
        basenames: BTreeMap::new(),
    };
    for root in roots {
        let listed = Repo::at(root)
            .text(&["ls-tree", "-r", "--name-only", commit, "--", "."])
            .ok_or_else(|| unlistable(root, commit))?;
        for path in listed.lines() {
            if path.is_empty()
                || path
                    .split('/')
                    .any(|part| skipped.contains(&part.to_string()))
            {
                continue;
            }
            index.add(path);
        }
    }
    index.sort_basenames();
    Ok(index)
}

fn unlistable(root: &Path, commit: &str) -> Error {
    Error(format!(
        "the base commit {} could not be listed under {} — fetch history, or give CI the full \
         clone",
        &commit[..7.min(commit.len())],
        root.display()
    ))
}

impl Index {
    fn add(&mut self, path: &str) {
        if let Some(name) = path.rsplit('/').next() {
            self.basenames
                .entry(name.to_string())
                .or_default()
                .push(path.to_string());
        }
        if let Some(listed) = &mut self.listed {
            listed.insert(path.to_string());
        }
    }

    fn sort_basenames(&mut self) {
        for found in self.basenames.values_mut() {
            found.sort();
            found.dedup();
        }
    }

    fn holds(&self, path: &str, roots: &[PathBuf]) -> bool {
        match &self.listed {
            Some(listed) => listed.contains(path),
            None => roots.iter().any(|root| root.join(path).is_file()),
        }
    }
}

/// None when `path` resolves — under a root as written, or as a bare filename found exactly
/// once — else the reason it does not.
fn resolves(path: &str, roots: &[PathBuf], index: &Index) -> Option<String> {
    if index.holds(path, roots) {
        return None;
    }
    if path.contains('/') {
        return Some(not_under(path, index));
    }
    match index.basenames.get(path).map(Vec::as_slice) {
        Some([_]) => None,
        Some(found) if !found.is_empty() => Some(format!(
            "ambiguous — cite one: {}",
            found[..found.len().min(4)].join(", ")
        )),
        _ => Some("no file of that name under the roots".to_string()),
    }
}

/// A path under no root, with the one file that carries its name when that file is unique —
/// which is where the citation most likely meant to point after a move.
fn not_under(path: &str, index: &Index) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match index.basenames.get(name).map(Vec::as_slice) {
        Some([moved]) => format!("not under the roots — likely {moved}"),
        _ => "not under the roots".to_string(),
    }
}

/// The documents a run judges: the one a person named, against the roots they named or the
/// tree root, or every Markdown file at the tree root. Spec 5.4.
fn listing<'a>(at: &Context<'a>, named: Option<&Path>, roots: &[PathBuf]) -> Listing<'a> {
    if let Some(named) = named
        && !roots.is_empty()
    {
        return Listing {
            documents: vec![Document {
                path: named.to_path_buf(),
                roots: roots.to_vec(),
                name: named.display().to_string(),
            }],
            root: at.project.start().to_path_buf(),
            config: None,
        };
    }
    let config = &at.project.config;
    let root = config.root().to_path_buf();
    let documents = match named {
        Some(named) => vec![Document {
            path: named.to_path_buf(),
            roots: vec![root.clone()],
            name: identity(named).strip_prefix(identity(&root)).map_or_else(
                |_| named.display().to_string(),
                |at| at.display().to_string(),
            ),
        }],
        None => at
            .project
            .facts()
            .found
            .documents
            .iter()
            .map(|name| Document {
                path: root.join(name),
                roots: vec![root.clone()],
                name: name.clone(),
            })
            .collect(),
    };
    Listing {
        documents,
        root,
        config: Some(config),
    }
}

fn identity(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// How git names the document, which a temporary directory's two spellings would otherwise break.
fn repo_path(path: &Path, root: &Path) -> String {
    files::relative(&identity(path), &identity(root))
}
