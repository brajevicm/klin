//! `doc-size` judges each document's word count against a ceiling. Every Markdown file at the
//! tree root is judged under a ceiling derived from the derivation commit, and a person may pin
//! a document's ceiling in a map of path to ceiling. A pin names its document by path, which
//! is how a document outside the tree root is judged too. The identity is the document's path;
//! nothing here is ratcheted beyond the base's own word count. Spec 5.4, 8.2.1, ADR 0040.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base;
use crate::cache;
use crate::ceiling::{self, Ceiling};
use crate::changed;
use crate::check::{self, Context, Said, Sink};
use crate::config::{Config, Error};
use crate::coverage::Coverage;
use crate::project::Project;
use crate::reference::Key;

pub const SECTION: &str = "doc_size";

/// The one kind of key this section holds, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[DOCUMENT];

pub const DOCUMENT: Key = Key {
    name: "<document path>",
    holds: "the words the document at that path, from the configuration's directory, may not pass: a whole number or dated steps. Every other document at the tree root keeps a derived ceiling",
    required: false,
    rule: Some(
        "every Markdown file at the tree root that the derivation commit holds: its word count there, rounded up to the next 50 and never below 50",
    ),
    default: "",
};

const CEILING_STEP: u64 = 50;
pub const RULE: &str = "the word count at the derivation commit, rounded up to the next 50";
const MARGIN_FRACTION: f64 = 0.02;
const REMEDY: &str = "An instruction that can be a gate costs no words — encode it as a gate and \
    point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling \
    is a decision to say why in the commit.";

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Judge this one document instead of every document
    #[arg(long)]
    file: Option<PathBuf>,
    /// The ceiling for --file (default: its pinned or derived ceiling)
    #[arg(long)]
    ceiling: Option<u64>,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
}

struct Document {
    path: PathBuf,
    ceiling: Ceiling,
    name: String,
    /// Where the base's copy of this document sits, and `None` for a document outside the tree
    /// klin compares, which has no base copy to hold it.
    relative: Option<PathBuf>,
}

/// The documents a run judges, and what it says about where each ceiling came from.
struct Listing {
    documents: Vec<Document>,
    said: Vec<Said>,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if let Some(named) = &args.file
        && !named.is_file()
    {
        return Err(Error(format!("no such file: {}", named.display())));
    }
    let project = Project::load(args.config.as_deref(), start)?;
    evaluate(
        &context(args, &project),
        args.file.as_deref(),
        args.ceiling,
        &mut Sink::unrecorded(out),
    )
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    evaluate(at, None, None, out)
}

fn context<'a>(args: &'a Args, project: &'a Project) -> Context<'a> {
    Context {
        quiet: args.quiet,
        ..Context::by_hand(SECTION, project)
    }
}

fn evaluate(
    at: &Context,
    named: Option<&Path>,
    ceiling: Option<u64>,
    out: &mut Sink,
) -> Result<u8, Error> {
    let documents = documents(at, named, ceiling, out)?;
    let against = against(at, &documents, out)?;
    out.record(|records| {
        records.held = Some(0);
        records.accepted = Some(0);
    });
    let over = judged(&documents, &against, at, out)?;
    let measured = documents.len();
    let said = Coverage::whole(measured).said(out);
    if over == 0 && !at.quiet {
        let _ = writeln!(out.text, "OK: {measured} document(s) judged{said}");
    }
    Ok(if over > 0 { 1 } else { 0 })
}

/// How many documents are over their ceilings. A document that cannot be judged hides no other
/// document's failure: every document is judged, and the first error comes back after them.
fn judged(
    documents: &[Document],
    against: &Option<(String, PathBuf)>,
    at: &Context,
    out: &mut Sink,
) -> Result<usize, Error> {
    let mut over = 0;
    let mut problem = None;
    for document in documents {
        match one(document, against, at, out) {
            Ok(failed) => over += usize::from(failed),
            Err(why) => {
                problem.get_or_insert(why);
            }
        }
    }
    problem.map_or(Ok(over), Err)
}

/// One document judged, and whether it is over its ceiling. A document the tree does not hold
/// is an error naming it: a pin is a person's, so a path that resolves nowhere is a config
/// error and not a measurement.
fn one(
    document: &Document,
    against: &Option<(String, PathBuf)>,
    at: &Context,
    out: &mut Sink,
) -> Result<bool, Error> {
    if !document.path.is_file() {
        return Err(Error(format!("no such file: {}", document.path.display())));
    }
    let words = count_words(&document.path)?;
    Ok(judge(
        document,
        words,
        held(against, document, words),
        at,
        out,
    ))
}

/// The base commit a document is compared against, and the directory its path is relative to.
/// `None` outside a repository and wherever no base resolves, and then the ceiling judges the
/// working tree alone.
fn against(
    at: &Context,
    documents: &[Document],
    out: &mut Sink,
) -> Result<Option<(String, PathBuf)>, Error> {
    if !documents.iter().any(|document| document.relative.is_some()) {
        return Ok(None);
    }
    let config = at.config();
    let Some(commit) = commit(config, at, out) else {
        return Ok(None);
    };
    Ok(Some((commit, config.root().to_path_buf())))
}

fn commit(config: &Config, at: &Context, out: &mut Sink) -> Option<String> {
    if let Some(named) = at.base {
        return Some(named.to_string());
    }
    let base = base::choose(config.root(), at.strict).ok()?;
    if at.context() {
        let _ = writeln!(out.text, "{}", base.line());
    }
    Some(base.before)
}

/// Whether the base holds this document over the same ceiling. A ceiling a person lowers must
/// fail no document the base holds, so a document over the ceiling in both trees that did not
/// grow is held, whatever the ceiling is. A document the base holds under another path reads as
/// new debt.
fn held(against: &Option<(String, PathBuf)>, document: &Document, words: u64) -> Option<u64> {
    if words <= document.ceiling.value {
        return None;
    }
    let (Some((commit, root)), Some(relative)) = (against, &document.relative) else {
        return None;
    };
    let text = changed::blob(root, commit, &relative.to_string_lossy())?;
    let before = words_in(&text);
    (before > document.ceiling.value && words <= before).then_some(before)
}

fn judge(document: &Document, words: u64, held: Option<u64>, at: &Context, out: &mut Sink) -> bool {
    let (name, ceiling) = (&document.name, &document.ceiling);
    if words > ceiling.value {
        let Some(before) = held else {
            return failed(document, words, out);
        };
        out.record(|records| records.held = Some(records.held.unwrap_or(0) + 1));
        if !at.quiet {
            let _ = writeln!(
                out.text,
                "OK: {name} is {words} words, over its ceiling of {ceiling}, held at the base \
                 at {before} words"
            );
        }
        return false;
    }
    if !at.quiet {
        let _ = writeln!(out.text, "OK: {name} is {words} words, ceiling {ceiling}");
    }
    let remaining = ceiling.value - words;
    if remaining as f64 <= ceiling.value as f64 * MARGIN_FRACTION {
        let _ = writeln!(
            out.text,
            "WARN: {name} is {words} words, {remaining} from its ceiling of {ceiling}."
        );
        out.record(|records| {
            let near = site("near-ceiling", name, words, ceiling.value);
            records.notes.push(Value::Object(near));
        });
    }
    false
}

fn failed(document: &Document, words: u64, out: &mut Sink) -> bool {
    let (name, ceiling) = (&document.name, &document.ceiling);
    let _ = writeln!(
        out.text,
        "FAIL: {name} is {words} words, over its ceiling of {ceiling}."
    );
    let _ = writeln!(out.text, "{REMEDY}");
    out.record(|records| {
        let mut over = site("new", name, words, ceiling.value);
        over.insert("condition".into(), "over its word ceiling".into());
        over.insert("fix_advice".into(), REMEDY.into());
        records.findings.push(Value::Object(over));
    });
    true
}

fn site(outcome: &str, name: &str, words: u64, ceiling: u64) -> Map<String, Value> {
    let mut values = Map::new();
    values.insert("words".into(), words.into());
    values.insert("ceiling".into(), ceiling.into());
    let mut out = Map::new();
    out.insert("outcome".into(), outcome.into());
    out.insert("file".into(), name.into());
    out.insert("values".into(), Value::Object(values));
    out
}

fn documents(
    at: &Context,
    named: Option<&Path>,
    ceiling: Option<u64>,
    out: &mut Sink,
) -> Result<Vec<Document>, Error> {
    if let (Some(named), Some(ceiling)) = (named, ceiling) {
        return Ok(vec![Document {
            path: named.to_path_buf(),
            ceiling: Ceiling {
                value: ceiling,
                step: None,
            },
            name: named.display().to_string(),
            relative: None,
        }]);
    }
    let listing = listing(at.project)?;
    let Some(named) = named else {
        for (line, entry) in listing.said.into_iter().filter(|_| !at.quiet) {
            out.provenance(line, entry);
        }
        return Ok(listing.documents);
    };
    let wanted = identity(named);
    listing
        .documents
        .into_iter()
        .find(|document| identity(&document.path) == wanted)
        .map(|document| vec![document])
        .ok_or_else(|| {
            Error(format!(
                "{}: no \"{SECTION}\" ceiling for {} — it is neither pinned nor a document the \
                 derivation commit holds at the tree root; pass --ceiling N",
                at.config().file.display(),
                named.display()
            ))
        })
}

/// Every pinned document under its pin, then every discovered document the config does not pin
/// under its derived ceiling. A discovered document the derivation commit lacks has no ceiling
/// that is not read out of the working tree, which 4.3 forbids, so a NOTE names it and it is not
/// judged. Spec 4.3, 5.4.
fn listing(project: &Project) -> Result<Listing, Error> {
    let config = &project.config;
    let none = Map::new();
    let pins = match config.pinned(SECTION) {
        Some(Value::Object(fields)) => fields,
        _ => &none,
    };
    let mut listing = pinned(config, pins)?;
    let unpinned: Vec<&String> = project
        .facts()
        .found
        .documents
        .iter()
        .filter(|name| {
            !pins
                .keys()
                .any(|pin| config.path(pin) == config.root().join(name))
        })
        .collect();
    if !unpinned.is_empty() {
        derived(project, &unpinned, &mut listing);
    }
    Ok(listing)
}

/// Every document a person pinned, under its pin.
fn pinned(config: &Config, pins: &Map<String, Value>) -> Result<Listing, Error> {
    let mut listing = Listing {
        documents: Vec::new(),
        said: Vec::new(),
    };
    for (name, value) in pins {
        let ceiling = ceiling::read(config, SECTION, name, value, "a whole number of words")?;
        listing
            .said
            .push((format!("pinned: {SECTION} {name} {ceiling}"), None));
        listing.documents.push(document(config, name, ceiling));
    }
    Ok(listing)
}

/// Every discovered document the config does not pin, under the ceiling the derivation commit
/// gives it, or named in a NOTE when that commit lacks it.
fn derived(project: &Project, unpinned: &[&String], listing: &mut Listing) {
    let config = &project.config;
    let derived = derived_ceilings(project);
    for name in unpinned {
        let Some(value) = derived.get(*name) else {
            listing.said.push(unjudged(config, name));
            continue;
        };
        listing.said.push((
            format!("derived: {SECTION} {name} {value}, {RULE}"),
            Some(check::derived_entry(
                SECTION,
                Some(name),
                (*value).into(),
                RULE,
            )),
        ));
        let ceiling = Ceiling {
            value: *value,
            step: None,
        };
        listing.documents.push(document(config, name, ceiling));
    }
}

fn unjudged(config: &Config, name: &str) -> Said {
    let words = count_words(&config.root().join(name)).unwrap_or_default();
    (
        format!(
            "NOTE: {SECTION} {name} is {words} words and is not judged — it gets a ceiling when \
             the stamp moves and the derivation commit holds it"
        ),
        None,
    )
}

fn document(config: &Config, name: &str, ceiling: Ceiling) -> Document {
    let path = config.path(name);
    Document {
        relative: path.strip_prefix(config.root()).ok().map(Path::to_path_buf),
        path,
        ceiling,
        name: name.to_string(),
    }
}

/// One word ceiling per document the derivation commit holds at the tree root: its word count
/// there, rounded up to the next 50, so an empty document gets 50 rather than a ceiling its
/// first word breaks. Read through one git process once per commit and cached under it. A
/// document the commit lacks is not here. Spec 5.4.
pub fn derived_ceilings(project: &Project) -> BTreeMap<String, u64> {
    let Some((held, commit, at)) = project.source_derivation() else {
        return BTreeMap::new();
    };
    if let Some(cached) = at
        .and_then(|at| cache::read(at, commit, SECTION))
        .and_then(|cached| read_ceilings(&cached))
    {
        return cached;
    }
    let mut out = BTreeMap::new();
    let names: Vec<&str> = held.documents.iter().map(String::as_str).collect();
    let read = changed::blobs(project.root(), commit, &names, |name, bytes| {
        let words = bytes.map(words_in).unwrap_or_default();
        out.insert(name.to_string(), (words / CEILING_STEP + 1) * CEILING_STEP);
    });
    if read.is_none() {
        return BTreeMap::new();
    }
    if let Some(at) = at {
        let kept = out
            .iter()
            .map(|(name, ceiling)| (name.clone(), Value::from(*ceiling)))
            .collect();
        cache::write(at, commit, SECTION, Value::Object(kept));
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

/// A section a person wrote: each key a document path, each value a ceiling. Refused before any
/// gate runs, naming what the section holds instead. A dated schedule is judged by `ceiling`.
pub fn well_formed(file: &Path, fields: &Map<String, Value>) -> Result<(), Error> {
    if fields.is_empty() {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must pin at least one document — remove the section to derive \
             every ceiling",
            file.display()
        )));
    }
    for (name, value) in fields {
        let ceiling = value.is_u64() || value.as_object().is_some_and(ceiling::is_schedule);
        if !ceiling || name.is_empty() {
            return Err(Error(format!(
                "{}: \"{SECTION}\" \"{name}\" must be a whole number of words or an object of \
                 dated steps — \"{SECTION}\" maps a document path to its ceiling, such as \
                 {{\"README.md\": 1200}}",
                file.display()
            )));
        }
    }
    Ok(())
}

fn identity(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn count_words(path: &Path) -> Result<u64, Error> {
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    Ok(words_in(&bytes))
}

fn words_in(bytes: &[u8]) -> u64 {
    String::from_utf8_lossy(bytes).split_whitespace().count() as u64
}
