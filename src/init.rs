use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::Error;
use crate::{hooks, radius, survey};

const FILENAME: &str = "klin.json";

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to write (default: one at the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Fill in the sections an existing configuration does not name
    #[arg(long)]
    add: bool,
    /// Re-pin every derivable section from today's tree
    #[arg(long, conflicts_with = "hooks")]
    force: bool,
    /// Write the hook entries for the hosts this tree uses, and nothing else
    #[arg(long, conflicts_with = "add")]
    hooks: bool,
    /// The host whose hook file --hooks writes, instead of the ones this tree names
    #[arg(long, requires = "hooks")]
    host: Option<String>,
    /// Write the hooks to the host's user-level file, so one install covers every repository
    #[arg(long, requires = "hooks")]
    global: bool,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let file = wanted(args, start);
    let root = file.parent().unwrap_or(start).to_path_buf();
    if args.hooks {
        return hooks::run(
            &hooks_root(args, &root)?,
            args.host.as_deref(),
            args.global,
            out,
        );
    }
    let held = read(&file)?;
    if held.is_some() && !args.add && !args.force {
        let _ = writeln!(out, "{}", already(&file));
        inert(&root, out);
        return Ok(0);
    }
    pins(&file, &root, held.unwrap_or_default(), args.force, out)
}

/// Where `--hooks` reads a host's marker directory and writes its file: this tree, or the
/// home directory whose files every repository shares. Section 19.3.
fn hooks_root(args: &Args, root: &Path) -> Result<PathBuf, Error> {
    if !args.global {
        return Ok(root.to_path_buf());
    }
    std::env::home_dir().ok_or_else(|| {
        Error(
            "--global writes the host's user-level file, and this system names no home \
               directory — run it without --global to write this tree's file"
                .to_string(),
        )
    })
}

fn already(file: &Path) -> String {
    format!(
        "{} already names this project's gates — klin init --add fills in the sections it does \
         not name, klin init --force re-pins them from today's tree, and a person edits the rest.",
        file.display()
    )
}

fn pins(
    file: &Path,
    root: &Path,
    held: Map<String, Value>,
    force: bool,
    out: &mut String,
) -> Result<u8, Error> {
    let surveyed = surveyed(root, held, force)?;
    write(file, &surveyed.config)?;
    let _ = writeln!(out, "{}", said(file, &surveyed.written));
    for line in surveyed.derived {
        let _ = writeln!(out, "{line}");
    }
    inert(root, out);
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
    let beside = target.with_extension(format!("klin-{}", std::process::id()));
    std::fs::write(&beside, text + "\n").map_err(|why| unwritable(&why))?;
    kept_mode(target, &beside);
    std::fs::rename(&beside, target).map_err(|why| {
        let _ = std::fs::remove_file(&beside);
        unwritable(&why)
    })
}

/// The file klin replaces keeps the permissions it had. A person who narrowed a host's
/// settings file did so on purpose, and a fresh neighbour would widen it back.
fn kept_mode(target: &Path, beside: &Path) {
    if let Ok(held) = std::fs::metadata(target) {
        let _ = std::fs::set_permissions(beside, held.permissions());
    }
}

/// Every section the tree can say for itself, what it wrote, and one `derived:` line per value
/// history produced. A key the configuration already holds stays as it is, so a gate a person
/// excluded with `false` is left alone.
struct Surveyed {
    config: Map<String, Value>,
    written: Vec<String>,
    derived: Vec<String>,
}

fn surveyed(root: &Path, mut config: Map<String, Value>, force: bool) -> Result<Surveyed, Error> {
    let found = survey::derive(root, &pinned(&config, force));
    let mut written = Vec::new();
    let mut add = |key: &str, value: Option<Value>, said: String| {
        added(&mut config, &mut written, force, key, value, said);
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
        "inventory",
        "lockfile",
        "escapes",
        "stubs",
        "complexity",
    ] {
        let section = found.sections.get(name).cloned().filter(stated);
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

/// One section written into the config, and the name the report gives it. A key the file holds
/// stays as it is unless `--force` re-pins it, and a re-pin keeps what klin cannot derive, so a
/// value that comes back unchanged is not reported as written.
fn added(
    config: &mut Map<String, Value>,
    written: &mut Vec<String>,
    force: bool,
    key: &str,
    value: Option<Value>,
    said: String,
) {
    let Some(value) = value else {
        return;
    };
    match config.remove(key) {
        Some(held) if !force => {
            config.insert(key.to_string(), held);
            return;
        }
        Some(held) => {
            let value = kept(&held, value);
            let same = value == held;
            config.insert(key.to_string(), value);
            if same {
                return;
            }
        }
        None => {
            config.insert(key.to_string(), value);
        }
    };
    written.push(said);
}

/// What the survey is told the config already pins. `--force` tells it nothing, because a
/// pinned value wins over a derived one everywhere else, and re-pinning wants the tree's answer.
fn pinned(config: &Map<String, Value>, force: bool) -> Value {
    match force {
        true => Value::Object(Map::new()),
        false => Value::Object(config.clone()),
    }
}

/// The derived value, with everything klin cannot derive taken from what the file held: a
/// dated schedule where a number would go, and a `false` that switched a gate off. Spec 5.7.
fn kept(held: &Value, derived: Value) -> Value {
    if underivable(held) {
        return held.clone();
    }
    match (held, derived) {
        (Value::Object(held), Value::Object(derived)) => fields(held, derived),
        (Value::Array(held), Value::Array(derived)) => entries(held, derived),
        (_, derived) => derived,
    }
}

fn underivable(held: &Value) -> bool {
    match held {
        Value::Bool(pinned) => !pinned,
        Value::Object(fields) => crate::ceiling::is_schedule(fields),
        _ => false,
    }
}

/// Every key either side holds. A key the survey does not derive, such as an exclusion a
/// person wrote, is that person's and survives the re-pin.
fn fields(held: &Map<String, Value>, mut derived: Map<String, Value>) -> Value {
    let mut out = Map::new();
    for (key, value) in held {
        let value = match derived.remove(key) {
            Some(found) => kept(value, found),
            None => value.clone(),
        };
        out.insert(key.clone(), value);
    }
    out.extend(derived);
    Value::Object(out)
}

/// An entry the file held, paired with the derived entry for the same file, so re-pinning a
/// list of documents keeps each document's schedule wherever the derived list puts it.
fn entries(held: &[Value], derived: Vec<Value>) -> Value {
    Value::Array(
        derived
            .into_iter()
            .enumerate()
            .map(|(at, value)| match paired(held, at, &value) {
                Some(held) => kept(held, value),
                None => value,
            })
            .collect(),
    )
}

fn paired<'a>(held: &'a [Value], at: usize, derived: &Value) -> Option<&'a Value> {
    match derived.get("file") {
        Some(file) => held.iter().find(|entry| entry.get("file") == Some(file)),
        None => held.get(at),
    }
}

/// A section worth writing down. An empty list is what a survey says when it found the
/// documents but the derivation commit holds none of them, and pinning that would gate nothing
/// for ever.
fn stated(section: &Value) -> bool {
    !section.as_array().is_some_and(|entries| entries.is_empty())
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
