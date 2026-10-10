//! `doc-size` judges each document's word count against a ceiling. Each agent instruction file
//! is judged under a ceiling derived from the derivation commit, or the new-file default where
//! that commit lacks it, and a person
//! may pin a document's ceiling in a map of path to ceiling. A pin names its document by path,
//! which is how any other document is judged. The identity is the document's path;
//! nothing here is ratcheted beyond the base's own word count. Spec 5.4, 8.2.1, ADR 0040.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::ceiling::{self, Ceiling};
use crate::config::file::Config;
use crate::config::key::Key;
use crate::config::scope::{Moves, Pinned, Selector};
use crate::contract::check::{Context, Counted, Derived, Line, Provenance, Sink, Standing, Told};
use crate::contract::coverage::Coverage;
use crate::contract::holes;
use crate::contract::project::Project;
use crate::contract::ratchet;
use crate::facts::survey;
use crate::sys::changed;
use crate::sys::error::Error;
use crate::window::base;

pub const SECTION: &str = "doc_size";

/// The one kind of key this section holds, which `klin policy --reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[DOCUMENT];

pub const DOCUMENT: Key = Key {
    name: "<document path>",
    holds: "the words the document at that path, from the configuration's directory, may not pass: a whole number or dated steps. `AGENTS.md` and `CLAUDE.md` at the tree root and `AGENTS.md` in any directory keep a derived ceiling where the map does not name them; every other document is judged only when the map names it",
    required: false,
    rule: Some(
        "each instruction file the derivation commit holds: the word count there, rounded up to the next 50 and never below 50; one it lacks: 50",
    ),
    default: "",
    shape: crate::config::key::Shape::Ceiling,
};

/// Whether the tree holds an instruction file, which is when this check runs with no section.
/// Spec 5.4.
pub fn applies(project: &Project) -> bool {
    !project.facts().found.instructions.is_empty()
}
const CEILING_STEP: u64 = 50;
const CACHE_KEY: &str = "doc_size_instructions";
pub const RULE: &str = "the word count at the derivation commit, rounded up to the next 50";
const NEW_CEILING: u64 = 50;
const NEW_RULE: &str = "the 50-word default for an instruction file the derivation commit lacks";
const MARGIN_FRACTION: f64 = 0.02;
const REMEDY: &str = "Keep in this file what the task asked for. Remove or compress redundant \
    instruction text first. An instruction that \
    can be a gate costs no words — encode it as a gate and point at it. Point at background in \
    docs/ only where the instruction keeps its intent. Only a person raises the ceiling, \
    in a reviewed commit.";

struct Document {
    path: PathBuf,
    ceiling: Ceiling,
    name: String,
    /// Where the base's copy of this document sits, and `None` for a document outside the tree
    /// klin compares, which has no base copy to hold it.
    relative: Option<PathBuf>,
    /// Whether the ceiling is automatic, which is the only kind a changed run may leave unread.
    automatic: bool,
}

/// The documents a run judges, and what it says about where each ceiling came from.
struct Listing {
    documents: Vec<Document>,
    said: Vec<Provenance>,
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    evaluate(at, None, None, out)
}

fn evaluate(
    at: &Context,
    named: Option<&Path>,
    ceiling: Option<u64>,
    out: &mut Sink,
) -> Result<u8, Error> {
    let documents = changed_only(at, documents(at, named, ceiling, out)?);
    let against = against(at, &documents)?;
    out.record(|records| {
        records.held = Some(0);
        records.accepted = Some(0);
    });
    let over = judged(at.gate, &documents, &against, out)?;
    let measured = documents.len();
    holes::formed_said(
        documents
            .iter()
            .filter_map(|document| document.relative.as_deref()?.to_str()),
        |_| true,
        at,
        out,
    );
    let said = out.covered(&Coverage::whole(measured));
    if over == 0 {
        out.tell(Told::judged(Line::new(Counted::Documents(measured), said)));
    }
    Ok(if over > 0 { 1 } else { 0 })
}

/// On a changed run, every pinned document and the automatic ones the change set touched. An
/// automatic document unchanged against the base has the base's word count, so it is under its
/// ceiling or held at the base, and reading it again could only say so. A pin is always read, so
/// a pin that names no file stays an error. Spec 8.2.1.
fn changed_only(at: &Context, documents: Vec<Document>) -> Vec<Document> {
    let Some(changes) = at.changes else {
        return documents;
    };
    documents
        .into_iter()
        .filter(
            |document| match document.relative.as_ref().filter(|_| document.automatic) {
                Some(relative) => changes
                    .iter()
                    .any(|change| Path::new(&change.path) == relative),
                None => true,
            },
        )
        .collect()
}

/// How many documents are over their ceilings. A document that cannot be judged hides no other
/// document's failure: every document is judged, and the first error comes back after them. The
/// base copies of the documents over their ceilings are read through one git process.
fn judged(
    gate: &str,
    documents: &[Document],
    against: &Option<(String, PathBuf)>,
    out: &mut Sink,
) -> Result<usize, Error> {
    let mut problem = None;
    let mut counted = Vec::new();
    for document in documents {
        match counted_words(document) {
            Ok(words) => counted.push((document, words)),
            Err(why) => {
                problem.get_or_insert(why);
            }
        }
    }
    let before = based(&counted, against)?;
    let mut over = 0;
    for (document, words) in counted {
        let held = before
            .get(document.name.as_str())
            .copied()
            .filter(|before| *before > document.ceiling.value && words <= *before);
        over += usize::from(judge(gate, document, words, held, out));
    }
    problem.map_or(Ok(over), Err)
}

/// A document's word count. A document the tree does not hold is an error naming it: a pin is
/// a person's, so a path that resolves nowhere is a config error and not a measurement.
fn counted_words(document: &Document) -> Result<u64, Error> {
    if !document.path.is_file() {
        return Err(Error(format!("no such file: {}", document.path.display())));
    }
    count_words(&document.path)
}

/// The base commit a document is compared against, and the directory its path is relative to.
/// `None` outside a repository and wherever no base resolves, and then the ceiling judges the
/// working tree alone.
fn against(at: &Context, documents: &[Document]) -> Result<Option<(String, PathBuf)>, Error> {
    if !documents.iter().any(|document| document.relative.is_some()) {
        return Ok(None);
    }
    let config = at.config();
    let Some(commit) = commit(config, at) else {
        return Ok(None);
    };
    Ok(Some((commit, config.root().to_path_buf())))
}

fn commit(config: &Config, at: &Context) -> Option<String> {
    if let Some(named) = at.base {
        return Some(named.to_string());
    }
    let base = base::choose(config.root()).ok()?;
    Some(base.before)
}

/// The base word count of every document over its ceiling, by name. A ceiling a person lowers
/// must fail no document the base holds, so a document over the ceiling in both trees that did
/// not grow is held, whatever the ceiling is. A document the base holds under another path
/// reads as new debt. A read git could not finish is an error, because base evidence klin
/// could not read is no proof of new debt.
fn based<'a>(
    counted: &[(&'a Document, u64)],
    against: &Option<(String, PathBuf)>,
) -> Result<BTreeMap<&'a str, u64>, Error> {
    let mut before = BTreeMap::new();
    let Some((commit, root)) = against else {
        return Ok(before);
    };
    let over: Vec<(&str, String)> = counted
        .iter()
        .filter(|(document, words)| *words > document.ceiling.value)
        .filter_map(|(document, _)| {
            let relative = document.relative.as_ref()?;
            Some((
                document.name.as_str(),
                relative.to_string_lossy().into_owned(),
            ))
        })
        .collect();
    if over.is_empty() {
        return Ok(before);
    }
    let paths: Vec<&str> = over.iter().map(|(_, path)| path.as_str()).collect();
    let mut names = over.iter().map(|(name, _)| *name);
    let mut answered = 0;
    let read = changed::blobs(root, commit, &paths, |_, bytes| {
        answered += 1;
        if let (Some(name), Some(bytes)) = (names.next(), bytes) {
            before.insert(name, words_in(bytes));
        }
    });
    if read.is_none() || answered != paths.len() {
        return Err(Error(format!(
            "{SECTION}: git could not read the base copies of the documents over their ceilings \
             at {commit}"
        )));
    }
    Ok(before)
}

fn judge(gate: &str, document: &Document, words: u64, held: Option<u64>, out: &mut Sink) -> bool {
    let ceiling = &document.ceiling;
    if words > ceiling.value {
        let Some(before) = held else {
            return failed(gate, document, words, out);
        };
        out.record(|records| records.held = Some(records.held.unwrap_or(0) + 1));
        told(document, words, Standing::Held(before), out);
        return false;
    }
    let remaining = ceiling.value - words;
    if remaining as f64 > ceiling.value as f64 * MARGIN_FRACTION {
        told(document, words, Standing::Under, out);
        return false;
    }
    told(document, words, Standing::Near(remaining), out);
    false
}

fn failed(gate: &str, document: &Document, words: u64, out: &mut Sink) -> bool {
    let over = Standing::Over {
        id: ratchet::site_id(gate, &document.name, ""),
        condition: "over its word ceiling",
        fix_advice: REMEDY,
    };
    told(document, words, over, out);
    true
}

fn told(document: &Document, words: u64, standing: Standing, out: &mut Sink) {
    out.tell(Told::Document {
        name: document.name.clone(),
        words,
        ceiling: document.ceiling.clone(),
        standing,
    });
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
            automatic: false,
        }]);
    }
    let listing = listing(at.project)?;
    let Some(named) = named else {
        out.tell_each(listing.said);
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
                "{}: no \"{SECTION}\" ceiling for {} — it is neither pinned nor an instruction \
                 file; pass --ceiling N",
                at.config().file.display(),
                named.display()
            ))
        })
}

pub fn derive(project: &Project) -> Result<Vec<Provenance>, Error> {
    listing(project).map(|listing| listing.said)
}

/// Every pinned document under its pin, then every instruction file the config does not pin
/// under its derived ceiling. An instruction file the derivation commit lacks takes a fixed
/// default, which reads nothing out of the working tree. Spec 4.3, 5.4.
fn listing(project: &Project) -> Result<Listing, Error> {
    let config = &project.config;
    let none = Map::new();
    let pins = match config.pinned(SECTION) {
        Some(Value::Object(fields)) => fields,
        _ => &none,
    };
    let mut listing = pinned(config, project.moves(), pins)?;
    let unpinned: Vec<&String> = project
        .facts()
        .found
        .instructions
        .iter()
        .filter(|name| {
            let path = config.root().join(name);
            !listing.documents.iter().any(|pinned| pinned.path == path)
        })
        .collect();
    if !unpinned.is_empty() {
        derived(project, &unpinned, &mut listing)?;
    }
    Ok(listing)
}

/// Every document a person pinned, under its pin. A pinned document the change renamed is
/// measured at its new path and compared with the base's copy at the old one, and one the
/// change deleted, or that names nothing in either tree, is measured nowhere. Spec 7.3.
fn pinned(config: &Config, moves: &Moves, pins: &Map<String, Value>) -> Result<Listing, Error> {
    let mut listing = Listing {
        documents: Vec::new(),
        said: Vec::new(),
    };
    for (name, value) in pins {
        let ceiling = ceiling::read(
            &config.file,
            SECTION,
            name,
            value,
            "a whole number of words",
        )?;
        listing.said.push(Provenance::Pinned {
            section: SECTION,
            key: name.clone(),
            shown: ceiling.to_string(),
        });
        let moved = Selector::parse(DOCUMENT, name).ok().and_then(|pin| {
            moves
                .pinned_where(SECTION, |kind| *kind == Pinned::Document)
                .find(|(path, _)| *path == pin.as_str())
        });
        match moved {
            None => listing
                .documents
                .push(document(config, name, ceiling, false)),
            Some((_, [(was, now), ..])) => listing.documents.push(Document {
                relative: Some(PathBuf::from(was)),
                ..document(config, now, ceiling, false)
            }),
            Some(_) => {}
        }
    }
    Ok(listing)
}

/// Every instruction file the config does not pin, under the ceiling the derivation commit
/// gives it, or the new-file default when that commit lacks it. Whether a file is new is read
/// from the commit's survey and not from the ceilings, so a ceiling klin could not read is an
/// error and never the new-file default.
fn derived(project: &Project, unpinned: &[&String], listing: &mut Listing) -> Result<(), Error> {
    let config = &project.config;
    let held = project
        .source_derivation()
        .map(|(held, _, _)| held.instructions.as_slice())
        .unwrap_or_default();
    let derived = if held.is_empty() {
        Some(BTreeMap::new())
    } else {
        derived_ceilings(project)
    };
    for name in unpinned {
        let (value, rule) = match held.contains(*name) {
            false => (NEW_CEILING, NEW_RULE),
            true => match derived.as_ref().and_then(|derived| derived.get(*name)) {
                Some(value) => (*value, RULE),
                None => {
                    return Err(Error(format!(
                        "{SECTION}: git could not read {name} at the derivation commit, so its \
                         ceiling is unknown"
                    )));
                }
            },
        };
        listing.said.push(
            Derived::keyed(SECTION, Some(name), value.into(), value.to_string(), rule).into(),
        );
        let ceiling = Ceiling { value, step: None };
        listing
            .documents
            .push(document(config, name, ceiling, true));
    }
    Ok(())
}

fn document(config: &Config, name: &str, ceiling: Ceiling, automatic: bool) -> Document {
    let path = config.path(name);
    Document {
        relative: path.strip_prefix(config.root()).ok().map(Path::to_path_buf),
        path,
        ceiling,
        name: name.to_string(),
        automatic,
    }
}

/// One word ceiling per instruction file the derivation commit holds: its count
/// there, rounded up to the next 50, so an empty document gets 50 rather than a ceiling its
/// first word breaks. Read through one git process once per commit and cached under it. A
/// document the commit lacks is not here. `None` when git could not read the commit. Spec 5.4.
pub fn derived_ceilings(project: &Project) -> Option<BTreeMap<String, u64>> {
    let Some((held, commit, at)) = project.source_derivation() else {
        return Some(BTreeMap::new());
    };
    if let Some(cached) = at
        .and_then(|cache| cache.read(commit, CACHE_KEY))
        .and_then(|cached| read_ceilings(&cached))
        .filter(|cached| cached.keys().eq(held.instructions.iter()))
    {
        return Some(cached);
    }
    let mut out = BTreeMap::new();
    let names: Vec<&str> = held.instructions.iter().map(String::as_str).collect();
    let read = changed::blobs(project.root(), commit, &names, |name, bytes| {
        let words = bytes.map(words_in).unwrap_or_default();
        out.insert(name.to_string(), (words / CEILING_STEP + 1) * CEILING_STEP);
    });
    if read.is_none() || out.len() != names.len() {
        return None;
    }
    if let Some(cache) = at {
        let kept = out
            .iter()
            .map(|(name, ceiling)| (name.clone(), Value::from(*ceiling)))
            .collect();
        cache.write(commit, CACHE_KEY, Value::Object(kept));
    }
    Some(out)
}

/// The cached ceilings when every one of them is a number for an instruction file, because a
/// file another hand edited, or one that names any other document, is no more this commit's
/// derivation than one another version wrote. The caller also requires one ceiling for each
/// instruction file the commit holds, so a partial cache makes no held file look new.
fn read_ceilings(cached: &Value) -> Option<BTreeMap<String, u64>> {
    cached
        .as_object()?
        .iter()
        .map(|(name, words)| match survey::instruction(name) {
            true => Some((name.clone(), words.as_u64()?)),
            false => None,
        })
        .collect()
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
