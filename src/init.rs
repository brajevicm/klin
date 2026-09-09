use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::{radius, survey};

const FILENAME: &str = "klin.json";

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to write (default: one at the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Fill in the sections an existing configuration does not name
    #[arg(long)]
    add: bool,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let file = wanted(args, start);
    let held = read(&file)?;
    let root = file.parent().unwrap_or(start).to_path_buf();
    if held.is_some() && !args.add {
        let _ = writeln!(
            out,
            "{} already names this project's gates — klin init --add fills in the sections it \
             does not name, and a person edits the rest.",
            file.display()
        );
        inert(&root, out);
        return Ok(0);
    }
    let surveyed = surveyed(&root, held.unwrap_or_default())?;
    write(&file, &surveyed.config)?;
    let _ = writeln!(out, "{}", said(&file, &surveyed.written));
    for line in surveyed.derived {
        let _ = writeln!(out, "{line}");
    }
    inert(&root, out);
    Ok(0)
}

/// klin writes nothing git can see, so an ignore line an older klin asked for does nothing.
/// `init` never edits `.gitignore`, and says so rather than leaving a person to wonder.
fn inert(root: &Path, out: &mut String) {
    let file = root.join(".gitignore");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let lines = [
        "/.klin",
        ".klin",
        "/.klin-build-blocked",
        ".klin-build-blocked",
    ];
    if !text
        .lines()
        .any(|line| lines.contains(&line.trim().trim_end_matches('/')))
    {
        return;
    }
    let _ = writeln!(
        out,
        "{}: the .klin line is inert — klin keeps its state under the git directory now, and \
         writes nothing the working tree can see. Delete the line when you like.",
        file.display()
    );
}

fn wanted(args: &Args, start: &Path) -> PathBuf {
    match &args.config {
        Some(named) if named.is_absolute() => named.clone(),
        Some(named) => start.join(named),
        None => start.join(FILENAME),
    }
}

fn said(file: &Path, written: &[String]) -> String {
    format!(
        "{}: wrote {}. Read it before you commit it: klin gates what it names, and nothing else.",
        file.display(),
        match written.is_empty() {
            true => "nothing this tree could not already say".to_string(),
            false => written.join(", "),
        }
    )
}

fn read(file: &Path) -> Result<Option<Map<String, Value>>, Error> {
    if !file.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(file).map_err(|why| Error::unreadable(file, why))?;
    match serde_json::from_str(&text).map_err(|why| Error::unreadable(file, why))? {
        Value::Object(held) => Ok(Some(held)),
        _ => Err(Error(format!(
            "{}: a configuration is an object of sections",
            file.display()
        ))),
    }
}

fn write(file: &Path, config: &Map<String, Value>) -> Result<(), Error> {
    let unwritable = |why: &dyn std::fmt::Display| {
        Error(format!("{} could not be written: {why}", file.display()))
    };
    let text = serde_json::to_string_pretty(&Value::Object(config.clone()))
        .map_err(|why| unwritable(&why))?;
    std::fs::write(file, text + "\n").map_err(|why| unwritable(&why))
}

/// Every section the tree can say for itself, what it wrote, and one `derived:` line per value
/// history produced. A key the configuration already holds stays as it is, so a gate a person
/// excluded with `false` is left alone.
struct Surveyed {
    config: Map<String, Value>,
    written: Vec<String>,
    derived: Vec<String>,
}

fn surveyed(root: &Path, mut config: Map<String, Value>) -> Result<Surveyed, Error> {
    let found = survey::derive(root, &Value::Object(config.clone()));
    let mut written = Vec::new();
    let mut add = |key: &str, value: Option<Value>, said: String| {
        let Some(value) = value.filter(|_| !config.contains_key(key)) else {
            return;
        };
        config.insert(key.to_string(), value);
        written.push(said);
    };
    add("project", project(root), "project".to_string());
    add(
        "version",
        Some(env!("CARGO_PKG_VERSION").into()),
        format!("version {}", env!("CARGO_PKG_VERSION")),
    );
    for name in [
        "build",
        "doc_size",
        "doc_citations",
        "escapes",
        "complexity",
    ] {
        let section = found.sections.get(name).cloned();
        add(name, section, name.to_string());
    }
    let derived = match radius::history(root, None) {
        Ok(history) => {
            add(
                SECTION,
                Some(radius::section(&history)),
                format!("{SECTION} over {} commit(s)", history.commits),
            );
            vec![
                radius::derived_line("lines", history.lines, history.commits),
                radius::derived_line("directories", history.directories, history.commits),
            ]
        }
        Err(why) => vec![format!("derived: no \"{SECTION}\" section, because {why}")],
    };
    Ok(Surveyed {
        config,
        written,
        derived: found.lines.into_iter().chain(derived).collect(),
    })
}

/// The section ADR 0014 pins: how wide this project's usual commit is, so the report on a
/// prompt has something to read a turn against. It is not a gate and it fails nothing.
const SECTION: &str = "radius";

fn project(root: &Path) -> Option<Value> {
    root.canonicalize()
        .ok()?
        .file_name()
        .map(|name| name.to_string_lossy().to_string().into())
}
