use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::{Config, Error, Flags};

const SECTION: &str = "doc_size";
const MARGIN_FRACTION: f64 = 0.02;
const REMEDY: &str = "An instruction that can be a gate costs no words — encode it as a gate and \
    point at it; otherwise move narrative into docs/ and keep the instruction. Raising the ceiling \
    is a decision to say why in the commit.";

#[derive(clap::Args)]
pub struct Args {
    /// The quality.json to run under (default: the nearest one above the working directory)
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
    ceiling: u64,
    name: String,
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
        quiet: args.quiet,
        strict: false,
        only: None,
    }
}

fn evaluate(
    flags: &Flags,
    named: Option<&Path>,
    ceiling: Option<u64>,
    start: &Path,
    out: &mut String,
) -> Result<u8, Error> {
    let mut over = 0;
    for document in documents(flags, named, ceiling, start)? {
        over += usize::from(judge(&document, flags.quiet, out)?);
    }
    Ok(if over > 0 { 1 } else { 0 })
}

fn judge(document: &Document, quiet: bool, out: &mut String) -> Result<bool, Error> {
    if !document.path.is_file() {
        return Err(Error(format!("no such file: {}", document.path.display())));
    }
    let words = count_words(&document.path)?;
    let (name, ceiling) = (&document.name, document.ceiling);
    if words > ceiling {
        let _ = writeln!(
            out,
            "FAIL: {name} is {words} words, over its ceiling of {ceiling}."
        );
        let _ = writeln!(out, "{REMEDY}");
        return Ok(true);
    }
    if !quiet {
        let _ = writeln!(out, "OK: {name} is {words} words, ceiling {ceiling}");
    }
    let remaining = ceiling - words;
    if remaining as f64 <= ceiling as f64 * MARGIN_FRACTION {
        let _ = writeln!(
            out,
            "WARN: {name} is {words} words, {remaining} from its ceiling of {ceiling}."
        );
    }
    Ok(false)
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
            ceiling,
            name: named.display().to_string(),
        }]);
    }
    let config = Config::load(flags.config.as_deref(), start)?;
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
            let ceiling = field(config, entry, "ceiling")?
                .as_u64()
                .ok_or_else(|| config.malformed(SECTION, "ceiling", "a whole number of words"))?;
            Ok(Document {
                path: config.path(name),
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

fn count_words(path: &Path) -> Result<u64, Error> {
    let bytes = std::fs::read(path).map_err(|why| Error::unreadable(path, why))?;
    Ok(String::from_utf8_lossy(&bytes).split_whitespace().count() as u64)
}
