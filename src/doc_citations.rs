use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::base;
use crate::changed::{self, git};
use crate::config::{Config, Error, Flags};
use crate::files;
use crate::ratchet::{self, Evaluator, Finding, Values};

const SECTION: &str = "doc_citations";
const DEFAULT_EXTENSIONS: &[&str] = &[
    ".py", ".ts", ".tsx", ".js", ".jsx", ".swift", ".rs", ".go", ".kt", ".java", ".rb", ".sh",
    ".md", ".json", ".yml", ".yaml", ".toml",
];
const EVERY_FILE: &str = "";
const REMEDY: &str = "Point the citation at where the file is now (a bare filename resolves \
    when exactly one file under the roots has that name), or delete the sentence that cites it.";

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Judge this one document instead of the config's list
    #[arg(long)]
    file: Option<PathBuf>,
    /// Where --file's citations may resolve (repeatable; standalone, needs no config entry)
    #[arg(long = "root")]
    roots: Vec<PathBuf>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
    /// Fail when an accepted entry matches nothing — what CI runs
    #[arg(long)]
    strict: bool,
}

struct Document {
    path: PathBuf,
    roots: Vec<PathBuf>,
    extensions: Vec<String>,
    name: String,
}

/// The documents a run judges, and the directory their paths and the base commit are read from.
struct Listing {
    documents: Vec<Document>,
    root: PathBuf,
    config: Option<Config>,
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

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if let Some(named) = &args.file
        && !named.is_file()
    {
        return Err(Error(format!("no such file: {}", named.display())));
    }
    evaluate(&flags(args), args.file.as_deref(), &args.roots, start, out)
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(flags, None, &[], start, out)
}

fn flags(args: &Args) -> Flags {
    Flags {
        config: args.config.clone(),
        gate: SECTION.to_string(),
        prior: None,
        base: None,
        quiet: args.quiet,
        strict: args.strict,
        hook: false,
        only: None,
        records: None,
        with: None,
    }
}

fn evaluate(
    flags: &Flags,
    named: Option<&Path>,
    roots: &[PathBuf],
    start: &Path,
    out: &mut String,
) -> Result<u8, Error> {
    let listing = listing(flags, named, roots, start)?;
    if let Some(config) = &listing.config {
        config.say(flags, SECTION, out);
    }
    let commit = base::commit(&listing.root, flags, out)?;
    let (now, before) = sides(&listing, &commit)?;
    let sites = ratchet::scoped(&now, flags.only.as_deref());
    let accepted = match &listing.config {
        Some(config) => ratchet::accepted(config, &flags.gate, evaluator().metrics)?,
        None => Vec::new(),
    };
    Ok(evaluator().evaluate(
        now,
        before,
        accepted,
        flags,
        &format!("OK: {sites} citation(s) resolve nowhere, all held at the base"),
        out,
    ))
}

/// What each document cites and resolves nowhere today, and what it did at the base.
fn sides(listing: &Listing, commit: &str) -> Result<(Vec<Finding>, Vec<Finding>), Error> {
    let (mut now, mut before) = (Vec::new(), Vec::new());
    let mut trees: HashMap<Vec<PathBuf>, Index> = HashMap::new();
    let mut priors: HashMap<Vec<PathBuf>, Index> = HashMap::new();
    for document in &listing.documents {
        roots_exist(document)?;
        if document.path.exists() {
            let text = std::fs::read(&document.path)
                .map_err(|why| Error::unreadable(&document.path, why))?;
            let index = cached(&mut trees, &document.roots, || working(&document.roots))?;
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
        fix_advice: REMEDY,
        format_metrics: show,
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
    for (path, line) in citations(text, &document.extensions) {
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

/// Every backticked span on each line that looks like a path with one of `extensions` — no
/// spaces, no `*`, and a `/` or a `.` — with a trailing `:line` suffix stripped.
fn citations(text: &str, extensions: &[String]) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for (number, line) in text.split('\n').enumerate() {
        let ticks: Vec<usize> = line.match_indices('`').map(|(at, _)| at).collect();
        for pair in ticks.as_chunks::<2>().0 {
            if let Some(candidate) = candidate(&line[pair[0] + 1..pair[1]], extensions) {
                out.push((candidate, number as u64 + 1));
            }
        }
    }
    out
}

fn candidate(span: &str, extensions: &[String]) -> Option<String> {
    let candidate = span.trim().split(':').next().unwrap_or("");
    if candidate.contains(' ') || candidate.contains('*') {
        return None;
    }
    if !extensions
        .iter()
        .any(|extension| candidate.ends_with(extension.as_str()))
    {
        return None;
    }
    (candidate.contains('/') || candidate.contains('.')).then(|| candidate.to_string())
}

fn working(roots: &[PathBuf]) -> Result<Index, Error> {
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
        for file in files::under(std::slice::from_ref(root), &wanted)? {
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
        let listed = git(root, &["ls-tree", "-r", "--name-only", commit, "--", "."])
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

fn listing(
    flags: &Flags,
    named: Option<&Path>,
    roots: &[PathBuf],
    start: &Path,
) -> Result<Listing, Error> {
    if let Some(named) = named
        && !roots.is_empty()
    {
        return Ok(Listing {
            documents: vec![Document {
                path: named.to_path_buf(),
                roots: roots.to_vec(),
                extensions: default_extensions(),
                name: named.display().to_string(),
            }],
            root: start.to_path_buf(),
            config: None,
        });
    }
    let config = Config::open(flags, start)?;
    let listed = listed_documents(&config)?;
    let root = config.root().to_path_buf();
    let Some(named) = named else {
        return Ok(Listing {
            documents: listed,
            root,
            config: Some(config),
        });
    };
    let wanted = identity(named);
    for document in listed {
        if identity(&document.path) == wanted {
            return Ok(Listing {
                documents: vec![document],
                root,
                config: Some(config),
            });
        }
    }
    Err(Error(format!(
        "{}: no \"{SECTION}\" entry for {} — pass --root DIR",
        config.file.display(),
        named.display()
    )))
}

fn listed_documents(config: &Config) -> Result<Vec<Document>, Error> {
    let Some(entries) = config.section(SECTION)?.as_array() else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must be a list of {{\"file\", \"roots\"}} entries",
            config.file.display()
        )));
    };
    entries
        .iter()
        .map(|entry| document(config, entry))
        .collect()
}

fn document(config: &Config, entry: &Value) -> Result<Document, Error> {
    let values = entry
        .as_object()
        .ok_or_else(|| config.malformed(SECTION, "file", "an object"))?;
    let name = values
        .get("file")
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, "file"))?;
    let roots = files::roots(config, SECTION, values, "roots")?
        .unwrap_or_else(|| vec![config.root().to_path_buf()]);
    let extensions = files::strings(config, SECTION, values, "extensions")?;
    Ok(Document {
        path: config.path(name),
        roots,
        extensions: if extensions.is_empty() {
            default_extensions()
        } else {
            extensions
        },
        name: name.to_string(),
    })
}

fn default_extensions() -> Vec<String> {
    DEFAULT_EXTENSIONS
        .iter()
        .map(|extension| extension.to_string())
        .collect()
}

fn identity(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// How git names the document, which a temporary directory's two spellings would otherwise break.
fn repo_path(path: &Path, root: &Path) -> String {
    files::relative(&identity(path), &identity(root))
}
