use std::fmt::Write;
use std::path::Path;

use serde_json::{Map, Value};

use crate::config::file::{self as config, Config};
use crate::contract::project::Project;
use crate::engine::catalogue;
use crate::sys::error::Error;
use crate::{
    checks::{complexity, doc_size},
    hook::radius,
    sys::write,
};

/// One suggested value `--pin` writes under a section, and the `derived:` line that says where
/// it came from.
struct Pin {
    key: String,
    value: u64,
    line: String,
}

/// Today's guardrails, written where the configuration states none: the complexity ceilings,
/// a ceiling per instruction file at the tree root, and the change radius. A value the file
/// holds, a `false`, a dated schedule, the accepted list and the journal preference are a
/// person's and stay as they are. Nothing that describes the repository is written. Spec 5.7,
/// ADR 0040.
pub fn pin(file: &Path, out: &mut String) -> Result<(), Error> {
    let root = file.parent().unwrap_or(file);
    let held = read(file)?;
    let project = Project::of(loaded(file, root, held.is_some())?, root);
    let mut config = held.unwrap_or_default();
    let mut written = Vec::new();
    let mut said = Vec::new();
    let suggested: [(&str, Suggest); 2] = [
        (complexity::SECTION, complexity_pins),
        (doc_size::SECTION, document_pins),
    ];
    for (section, pins) in suggested {
        if !excluded(&config, section) {
            added(
                &mut config,
                section,
                pins(&project),
                &mut written,
                &mut said,
            );
        }
    }
    match radius_pins(&project) {
        Ok(pins) => added(&mut config, RADIUS, pins, &mut written, &mut said),
        Err(why) => said.push(format!("derived: no \"{RADIUS}\" pinned, because {why}")),
    }
    write(file, &config)?;
    let _ = writeln!(out, "{}\n{}", pinned(file, &written), said.join("\n"));
    Ok(())
}

/// The configuration `--pin` adds to, validated like any other, or an empty one where the file
/// does not exist yet.
fn loaded(file: &Path, root: &Path, held: bool) -> Result<Config, Error> {
    match held {
        true => Config::load(Some(file), root, &catalogue::sections()),
        false => Ok(Config::empty(file)),
    }
}

fn complexity_pins(project: &Project) -> Vec<Pin> {
    complexity::suggested(project)
        .into_iter()
        .map(|(key, value, line)| Pin {
            key: key.to_string(),
            value,
            line,
        })
        .collect()
}

/// A ceiling for every instruction file the derivation commit holds.
fn document_pins(project: &Project) -> Vec<Pin> {
    let documents = &project.facts().found.instructions;
    doc_size::derived_ceilings(project)
        .unwrap_or_default()
        .into_iter()
        .filter(|(name, _)| documents.contains(name))
        .map(|(name, value)| Pin {
            line: format!(
                "derived: {} {name} {value}, {}",
                doc_size::SECTION,
                doc_size::RULE
            ),
            key: name,
            value,
        })
        .collect()
}

fn radius_pins(project: &Project) -> Result<Vec<Pin>, String> {
    let history = radius::history(project.root(), project.facts().state.as_deref())?;
    Ok([
        ("lines", history.lines),
        ("directories", history.directories),
    ]
    .into_iter()
    .map(|(key, value)| Pin {
        key: key.to_string(),
        value,
        line: radius::derived_line(key, value, history.commits),
    })
    .collect())
}

/// What suggests the pins of one section.
type Suggest = fn(&Project) -> Vec<Pin>;

const RADIUS: &str = config::RADIUS.name;

fn excluded(config: &Map<String, Value>, section: &str) -> bool {
    config.get(section) == Some(&Value::Bool(false))
}

/// Each suggested value the section does not state yet, and the name and line of each one
/// written. A section that states nothing gains no empty object.
fn added(
    config: &mut Map<String, Value>,
    section: &str,
    pins: Vec<Pin>,
    written: &mut Vec<String>,
    said: &mut Vec<String>,
) {
    let mut fields = match config.get(section) {
        Some(Value::Object(fields)) => fields.clone(),
        _ => Map::new(),
    };
    for pin in pins {
        if fields.contains_key(&pin.key) {
            continue;
        }
        fields.insert(pin.key.clone(), pin.value.into());
        written.push(format!("{section} {}", pin.key));
        said.push(pin.line);
    }
    if !fields.is_empty() {
        config.insert(section.to_string(), Value::Object(fields));
    }
}

/// klin writes nothing git can see, so an ignore line an older klin asked for does nothing.
/// `setup` never edits `.gitignore`, and says so rather than leaving a person to wonder.
pub fn inert(root: &Path, out: &mut String) {
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

fn pinned(file: &Path, written: &[String]) -> String {
    format!(
        "{}: pinned {}. Read it before you commit it: a pinned value is policy a person owns.",
        file.display(),
        match written.is_empty() {
            true => "nothing the file did not already state".to_string(),
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

/// The file is replaced whole, through a neighbour and a rename, so a run that dies partway
/// leaves the file it found rather than a truncated one. A host's settings are a person's: a
/// path that is a link is followed, so a settings file kept in a dotfiles tree stays a link.
pub fn write(file: &Path, config: &Map<String, Value>) -> Result<(), Error> {
    let unwritable = |why: &dyn std::fmt::Display| {
        Error(format!("{} could not be written: {why}", file.display()))
    };
    let text = serde_json::to_string_pretty(&Value::Object(config.clone()))
        .map_err(|why| unwritable(&why))?;
    let held = std::fs::canonicalize(file);
    let target = held.as_deref().unwrap_or(file);
    write::atomic_write(write::AtomicWrite {
        target,
        bytes: (text + "\n").as_bytes(),
        keep_mode_from: Some(target),
    })
    .map_err(|why| unwritable(&why))
}
