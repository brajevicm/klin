use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::{escapes, files};

const FILENAME: &str = "klin.json";
const CEILINGS: (u64, u64) = (8, 60);
const CEILING_STEP: u64 = 50;

/// A manifest names a project klin can build, and the command that builds it.
const MANIFESTS: &[(&str, &str, &str)] = &[
    ("Cargo.toml", "", "cargo build --all-targets"),
    ("go.mod", "", "go build ./..."),
    ("package.json", "tsconfig.json", "tsc --noEmit"),
];

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
    let file = match &args.config {
        Some(named) if named.is_absolute() => named.clone(),
        Some(named) => start.join(named),
        None => start.join(FILENAME),
    };
    let held = read(&file)?;
    if held.is_some() && !args.add {
        let _ = writeln!(
            out,
            "{} already names this project's gates — klin init --add fills in the sections it \
             does not name, and a person edits the rest.",
            file.display()
        );
        return Ok(0);
    }
    let root = file.parent().unwrap_or(start);
    let (config, written) = surveyed(root, held.unwrap_or_default())?;
    write(&file, &config)?;
    let _ = writeln!(
        out,
        "{}: wrote {}. Read it before you commit it: klin gates what it names, and nothing else.",
        file.display(),
        match written.is_empty() {
            true => "nothing this tree could not already say".to_string(),
            false => written.join(", "),
        }
    );
    Ok(0)
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

/// Every section the tree can say for itself. A key the configuration already holds stays as it
/// is, so a gate a person excluded with `false` is left alone.
fn surveyed(
    root: &Path,
    mut config: Map<String, Value>,
) -> Result<(Map<String, Value>, Vec<String>), Error> {
    let sources = sources(root)?;
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
    add("build", build(root)?, "build".to_string());
    let documents = documents(root)?;
    add(
        "doc_size",
        (!documents.is_empty()).then(|| Value::Array(documents.clone())),
        format!("doc_size over {} document(s)", documents.len()),
    );
    add(
        "escapes",
        escapes_section(&sources),
        format!("escapes over {}", named(&sources.roots)),
    );
    add(
        "complexity",
        complexity_section(&sources),
        format!("complexity over {}", named(&sources.roots)),
    );
    Ok((config, written))
}

fn named(roots: &[String]) -> String {
    match roots.is_empty() {
        true => "nothing".to_string(),
        false => roots.join(", "),
    }
}

fn project(root: &Path) -> Option<Value> {
    root.canonicalize()
        .ok()?
        .file_name()
        .map(|name| name.to_string_lossy().to_string().into())
}

struct Sources {
    roots: Vec<String>,
    languages: Vec<String>,
}

fn sources(root: &Path) -> Result<Sources, Error> {
    let suffixes: Vec<&str> = escapes::suffixes().collect();
    let wanted = files::Wanted {
        extensions: &suffixes,
        skip_dirs: &files::default_skip_dirs(),
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let mut roots = Vec::new();
    let mut languages = Vec::new();
    for file in files::under(&[root.to_path_buf()], &wanted)? {
        let relative = files::relative(&file, root);
        let first = match relative.split_once('/') {
            Some((first, _)) => first.to_string(),
            None => ".".to_string(),
        };
        if !roots.contains(&first) {
            roots.push(first);
        }
        if let Some(language) = escapes::language_of(&relative)
            && !languages.iter().any(|held| held == language)
        {
            languages.push(language.to_string());
        }
    }
    roots.sort();
    languages.sort();
    Ok(Sources { roots, languages })
}

fn escapes_section(sources: &Sources) -> Option<Value> {
    if sources.roots.is_empty() || sources.languages.is_empty() {
        return None;
    }
    let mut section = Map::new();
    section.insert("roots".into(), list(&sources.roots));
    section.insert("languages".into(), list(&sources.languages));
    Some(Value::Object(section))
}

fn complexity_section(sources: &Sources) -> Option<Value> {
    if sources.roots.is_empty() {
        return None;
    }
    let mut ceilings = Map::new();
    ceilings.insert("cc".into(), CEILINGS.0.into());
    ceilings.insert("lines".into(), CEILINGS.1.into());
    let mut section = Map::new();
    section.insert("sources".into(), list(&sources.roots));
    section.insert("ceilings".into(), Value::Object(ceilings));
    Some(Value::Object(section))
}

fn list(values: &[String]) -> Value {
    Value::Array(values.iter().map(|value| value.clone().into()).collect())
}

/// One entry per document at the top of the tree, with a ceiling above what it holds today, so
/// day one is green and the next paragraph is the one that has to argue for itself.
fn documents(root: &Path) -> Result<Vec<Value>, Error> {
    let wanted = files::Wanted {
        extensions: &[".md"],
        skip_dirs: &files::default_skip_dirs(),
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    let mut out = Vec::new();
    for file in files::under(&[root.to_path_buf()], &wanted)? {
        let name = files::relative(&file, root);
        if name.contains('/') {
            continue;
        }
        let mut entry = Map::new();
        entry.insert("file".into(), name.into());
        entry.insert(
            "ceiling".into(),
            ceiling(crate::doc_size::words(&file)?).into(),
        );
        out.push(Value::Object(entry));
    }
    Ok(out)
}

fn ceiling(words: u64) -> u64 {
    (words / CEILING_STEP + 1) * CEILING_STEP
}

/// One build entry per manifest. A single manifest at the top of the tree is one command.
fn build(root: &Path) -> Result<Option<Value>, Error> {
    let mut found: Vec<(String, &str)> = Vec::new();
    for (manifest, beside, command) in MANIFESTS {
        for at in holding(root, manifest)? {
            if !beside.is_empty() && !root.join(&at).join(beside).is_file() {
                continue;
            }
            found.push((at, command));
        }
    }
    found.sort();
    let Some((at, command)) = found.first() else {
        return Ok(None);
    };
    if found.len() == 1 && at.is_empty() {
        return Ok(Some((*command).into()));
    }
    Ok(Some(Value::Array(
        found
            .iter()
            .map(|(at, command)| entry(at, command))
            .collect(),
    )))
}

fn entry(at: &str, command: &str) -> Value {
    let mut out = Map::new();
    if !at.is_empty() {
        out.insert("root".into(), at.into());
    }
    out.insert("run".into(), command.into());
    Value::Object(out)
}

/// The directories that hold a manifest of this name, relative to the root of the tree.
fn holding(root: &Path, manifest: &str) -> Result<Vec<String>, Error> {
    let wanted = files::Wanted {
        extensions: &[manifest],
        skip_dirs: &files::default_skip_dirs(),
        exclude: &[],
        exclude_except: &[],
        skip_hidden: true,
    };
    Ok(files::under(&[root.to_path_buf()], &wanted)?
        .iter()
        .filter(|file| files::relative(file, root).ends_with(manifest))
        .map(|file| {
            let name = files::relative(file, root);
            match name.rsplit_once('/') {
                Some((at, _)) => at.to_string(),
                None => String::new(),
            }
        })
        .collect())
}
