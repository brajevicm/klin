use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base;
use crate::ceiling::{self, Ceiling};
use crate::changed;
use crate::check::{Context, Sink};
use crate::config::{Config, Error};
use crate::coverage::Coverage;
use crate::reference::Key;

pub const SECTION: &str = "doc_size";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[FILE, CEILING];

pub const FILE: Key = Key {
    name: "file",
    holds: "the document this entry judges, as a path under the tree root",
    required: true,
    rule: Some("one entry per Markdown file at the tree root that the derivation commit holds"),
    default: "",
};

pub const CEILING: Key = Key {
    name: "ceiling",
    holds: "the words the document may not pass",
    required: true,
    rule: Some(
        "the word count at the derivation commit, rounded up to the next 50 and never below 50",
    ),
    default: "",
};
const MARGIN_FRACTION: f64 = 0.02;
const REMEDY: &str = "An instruction that can be a gate costs no words — encode it as a gate and \
    point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling \
    is a decision to say why in the commit.";

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Judge this one document instead of the config's list
    #[arg(long)]
    file: Option<PathBuf>,
    /// The ceiling for --file (default: its entry in the config)
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

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if let Some(named) = &args.file
        && !named.is_file()
    {
        return Err(Error(format!("no such file: {}", named.display())));
    }
    evaluate(
        &context(args, start),
        args.file.as_deref(),
        args.ceiling,
        &mut Sink::unrecorded(out),
    )
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    evaluate(at, None, None, out)
}

fn context<'a>(args: &'a Args, start: &'a Path) -> Context<'a> {
    Context {
        quiet: args.quiet,
        ..Context::by_hand(SECTION, start, args.config.as_deref())
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
    out.record(|records| records.held = Some(0));
    let mut over = 0;
    for document in &documents {
        over += usize::from(one(document, &against, at, out)?);
    }
    let measured = documents.len();
    let said = Coverage::whole(measured).said(out);
    if over == 0 && !at.quiet {
        let _ = writeln!(out.text, "OK: {measured} document(s) judged{said}");
    }
    Ok(if over > 0 { 1 } else { 0 })
}

/// One document judged, and whether it is over its ceiling. A document the tree does not hold
/// is an error naming it: the list is a person's, so a path that resolves nowhere is a config
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
    let config = Config::load_with(at.config, at.start, at.with)?;
    let Some(commit) = commit(&config, at, out) else {
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
/// new debt, and the commit that renames it edits this gate's list anyway.
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
    let config = Config::load_with(at.config, at.start, at.with)?;
    let listed = listed_documents(&config)?;
    at.say(&config, SECTION, out);
    let Some(named) = named else {
        return Ok(listed);
    };
    let wanted = identity(named);
    for document in listed {
        if identity(&document.path) == wanted {
            return Ok(vec![document]);
        }
    }
    Err(Error(format!(
        "{}: no \"{SECTION}\" entry for {} — pass --ceiling N",
        config.file.display(),
        named.display()
    )))
}

fn listed_documents(config: &Config) -> Result<Vec<Document>, Error> {
    let Some(entries) = config.section(SECTION)?.as_array() else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must be a list of {{\"{}\", \"{}\"}} entries",
            config.file.display(),
            FILE.name,
            CEILING.name
        )));
    };
    entries
        .iter()
        .map(|entry| {
            let name = field(config, entry, FILE)?
                .as_str()
                .ok_or_else(|| config.malformed(SECTION, FILE.name, "a path"))?;
            let ceiling = ceiling::read(
                config,
                SECTION,
                CEILING.name,
                field(config, entry, CEILING)?,
                "a whole number of words",
            )?;
            let path = config.path(name);
            Ok(Document {
                relative: path.strip_prefix(config.root()).ok().map(Path::to_path_buf),
                path,
                ceiling,
                name: name.to_string(),
            })
        })
        .collect()
}

fn field<'a>(config: &Config, entry: &'a Value, key: Key) -> Result<&'a Value, Error> {
    entry
        .get(key.name)
        .ok_or_else(|| config.missing(SECTION, key.name))
}

fn identity(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The word count a ceiling is measured against, so a survey can write one a document meets.
pub fn words(path: &Path) -> Result<u64, Error> {
    count_words(path)
}

fn count_words(path: &Path) -> Result<u64, Error> {
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    Ok(words_in(&bytes))
}

fn words_in(bytes: &[u8]) -> u64 {
    String::from_utf8_lossy(bytes).split_whitespace().count() as u64
}
