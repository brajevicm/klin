use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base;
use crate::ceiling::{self, Ceiling};
use crate::changed;
use crate::config::{Config, Error, Flags};

const SECTION: &str = "doc_size";
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
    evaluate(&flags(args), args.file.as_deref(), args.ceiling, start, out)
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(flags, None, None, start, out)
}

fn flags(args: &Args) -> Flags {
    Flags {
        config: args.config.clone(),
        gate: SECTION.to_string(),
        prior: None,
        base: None,
        quiet: args.quiet,
        strict: false,
        hook: false,
        only: None,
        records: None,
        with: None,
    }
}

fn evaluate(
    flags: &Flags,
    named: Option<&Path>,
    ceiling: Option<u64>,
    start: &Path,
    out: &mut String,
) -> Result<u8, Error> {
    let documents = documents(flags, named, ceiling, start)?;
    let against = against(flags, &documents, start, out)?;
    let mut over = 0;
    for document in &documents {
        if !document.path.is_file() {
            return Err(Error(format!("no such file: {}", document.path.display())));
        }
        let words = count_words(&document.path)?;
        over += usize::from(judge(
            document,
            words,
            held(&against, document, words),
            flags,
            out,
        ));
    }
    Ok(if over > 0 { 1 } else { 0 })
}

/// The base commit a document is compared against, and the directory its path is relative to.
/// `None` outside a repository and wherever no base resolves, and then the ceiling judges the
/// working tree alone.
fn against(
    flags: &Flags,
    documents: &[Document],
    start: &Path,
    out: &mut String,
) -> Result<Option<(String, PathBuf)>, Error> {
    if !documents.iter().any(|document| document.relative.is_some()) {
        return Ok(None);
    }
    let config = Config::open(flags, start)?;
    let Some(commit) = commit(&config, flags, out) else {
        return Ok(None);
    };
    Ok(Some((commit, config.root().to_path_buf())))
}

fn commit(config: &Config, flags: &Flags, out: &mut String) -> Option<String> {
    if let Some(named) = &flags.base {
        return Some(named.clone());
    }
    let base = base::choose(config.root(), flags.strict).ok()?;
    if !flags.quiet {
        let _ = writeln!(out, "{}", base.line());
    }
    Some(base.commit)
}

/// Whether the base holds this document over the same ceiling. A ceiling a person lowers must
/// fail no document the base holds, so a document over the ceiling in both trees that did not
/// grow is held, whatever the ceiling is. A document the base holds under another path reads as
/// new debt, and the commit that renames it edits this gate's list anyway.
fn held(against: &Option<(String, PathBuf)>, document: &Document, words: u64) -> bool {
    if words <= document.ceiling.value {
        return false;
    }
    let (Some((commit, root)), Some(relative)) = (against, &document.relative) else {
        return false;
    };
    let Some(text) = changed::blob(root, commit, &relative.to_string_lossy()) else {
        return false;
    };
    let before = words_in(&text);
    before > document.ceiling.value && words <= before
}

fn judge(document: &Document, words: u64, held: bool, flags: &Flags, out: &mut String) -> bool {
    let (name, ceiling) = (&document.name, &document.ceiling);
    if words > ceiling.value {
        if !held {
            return failed(document, words, flags, out);
        }
        if !flags.quiet {
            let _ = writeln!(
                out,
                "OK: {name} is {words} words, over its ceiling of {ceiling}, held at the base"
            );
        }
        return false;
    }
    if !flags.quiet {
        let _ = writeln!(out, "OK: {name} is {words} words, ceiling {ceiling}");
    }
    let remaining = ceiling.value - words;
    if remaining as f64 <= ceiling.value as f64 * MARGIN_FRACTION {
        let _ = writeln!(
            out,
            "WARN: {name} is {words} words, {remaining} from its ceiling of {ceiling}."
        );
        flags.record(|records| {
            let near = site("near-ceiling", name, words, ceiling.value);
            records.notes.push(Value::Object(near));
        });
    }
    false
}

fn failed(document: &Document, words: u64, flags: &Flags, out: &mut String) -> bool {
    let (name, ceiling) = (&document.name, &document.ceiling);
    let _ = writeln!(
        out,
        "FAIL: {name} is {words} words, over its ceiling of {ceiling}."
    );
    let _ = writeln!(out, "{REMEDY}");
    flags.record(|records| {
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
    flags: &Flags,
    named: Option<&Path>,
    ceiling: Option<u64>,
    start: &Path,
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
    let config = Config::open(flags, start)?;
    let listed = listed_documents(&config)?;
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
            "{}: \"{SECTION}\" must be a list of {{\"file\", \"ceiling\"}} entries",
            config.file.display()
        )));
    };
    entries
        .iter()
        .map(|entry| {
            let name = field(config, entry, "file")?
                .as_str()
                .ok_or_else(|| config.malformed(SECTION, "file", "a path"))?;
            let ceiling = ceiling::read(
                config,
                SECTION,
                "ceiling",
                field(config, entry, "ceiling")?,
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

fn field<'a>(config: &Config, entry: &'a Value, key: &str) -> Result<&'a Value, Error> {
    entry.get(key).ok_or_else(|| config.missing(SECTION, key))
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
