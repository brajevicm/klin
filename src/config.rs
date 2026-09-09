use std::cell::RefCell;
use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::Value;

const FILENAME: &str = "klin.json";
const VERSION: &str = "version";

/// The keys of section 5.2. The gate sections come from the checks themselves, so the two
/// lists cannot drift apart.
const KEYS: &[&str] = &["project", VERSION, "build", "accepted", "radius", "gates"];

#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        out.write_str(&self.0)
    }
}

impl Error {
    pub fn unreadable(path: &Path, problem: impl fmt::Display) -> Error {
        Error(format!("{} could not be read: {problem}", path.display()))
    }
}

/// The outcome of a file no grammar reads. The hook counts these to report the holes a
/// person must close, and nothing else in a run turns on it.
pub const UNPARSED: &str = "unparsed";

#[derive(Default)]
pub struct Records {
    pub findings: Vec<Value>,
    pub notes: Vec<Value>,
}

pub struct Flags {
    pub config: Option<PathBuf>,
    /// The name of the gate being run, which the accepted list names.
    pub gate: String,
    /// The base commit, already laid out as a directory by the runner.
    pub prior: Option<PathBuf>,
    /// The base commit the runner chose, for a gate that reads the base tree out of git.
    pub base: Option<String>,
    pub quiet: bool,
    pub strict: bool,
    /// The Stop hook runs this gate, so a hole the agent cannot fix is a note, not a failure.
    pub hook: bool,
    pub only: Option<Vec<String>>,
    pub records: Option<RefCell<Records>>,
    pub with: Option<(String, Value)>,
}

impl Flags {
    pub fn record(&self, add: impl FnOnce(&mut Records)) {
        if let Some(records) = &self.records {
            add(&mut records.borrow_mut());
        }
    }
}

pub struct Config {
    pub file: PathBuf,
    root: PathBuf,
    data: Value,
}

impl Config {
    pub fn load(explicit: Option<&Path>, start: &Path) -> Result<Config, Error> {
        let file = match explicit {
            Some(named) => absolute(named, start),
            None => find(start).ok_or_else(|| {
                Error(format!(
                    "no {FILENAME} at or above {} — write one at the repository root, \
                     listing the gates this project runs",
                    start.display()
                ))
            })?,
        };
        let text = std::fs::read_to_string(&file).map_err(|why| Error::unreadable(&file, why))?;
        let data = serde_json::from_str(&text).map_err(|why| Error::unreadable(&file, why))?;
        let root = file.parent().unwrap_or(Path::new("")).to_path_buf();
        well_formed(&file, &data)?;
        Ok(Config { file, root, data })
    }

    pub fn open(flags: &Flags, start: &Path) -> Result<Config, Error> {
        let mut config = Config::load(flags.config.as_deref(), start)?;
        if let Some((section, values)) = &flags.with
            && let Some(data) = config.data.as_object_mut()
        {
            data.insert(section.clone(), values.clone());
        }
        Ok(config)
    }

    /// What to say when the config names a klin version other than the one running, and
    /// nothing when it names this one or none. A mismatch is a note. Section 5.2.
    pub fn version_note(&self) -> Option<String> {
        let running = env!("CARGO_PKG_VERSION");
        let named = self.data.get(VERSION)?.as_str()?;
        if named == running {
            return None;
        }
        Some(format!(
            "NOTE: {} names version {named} and this binary is {running} \u{2014} the version it \
             names changes no gate and no exit code.",
            self.file.display()
        ))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn section(&self, name: &str) -> Result<&Value, Error> {
        self.data
            .get(name)
            .ok_or_else(|| Error(format!("{} has no \"{name}\" section", self.file.display())))
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        let expanded = expand_home(relative);
        if expanded.is_absolute() {
            expanded
        } else {
            self.root.join(expanded)
        }
    }

    pub fn missing(&self, section: &str, key: &str) -> Error {
        Error(format!(
            "{}: a \"{section}\" entry has no \"{key}\"",
            self.file.display()
        ))
    }

    pub fn malformed(&self, section: &str, key: &str, must_be: &str) -> Error {
        Error(format!(
            "{}: a \"{section}\" entry's \"{key}\" must be {must_be}",
            self.file.display()
        ))
    }
}

/// What every command refuses before it reads a section: the config errors of section 14,
/// named against the file that holds them.
fn well_formed(file: &Path, data: &Value) -> Result<(), Error> {
    a_version_is_a_string(file, data)?;
    every_key_is_one_klin_reads(file, data)?;
    no_section_names_a_retired_key(file, data)?;
    crate::ceiling::every_schedule(file, data)
}

fn no_section_names_a_retired_key(file: &Path, data: &Value) -> Result<(), Error> {
    let Some(fields) = data.as_object() else {
        return Ok(());
    };
    for (name, section) in fields {
        if let Some(values) = section.as_object() {
            crate::ratchet::no_retired_key(file, name, values)?;
        }
    }
    Ok(())
}

/// A "version" that is not a string is a malformed key, and every command refuses it. Whether
/// the version it names is the one running is a note instead. Sections 5.2 and 14.
fn a_version_is_a_string(file: &Path, data: &Value) -> Result<(), Error> {
    match data.get(VERSION) {
        None | Some(Value::String(_)) => Ok(()),
        Some(_) => Err(Error(format!(
            "{}: \"{VERSION}\" must be a klin version as a string",
            file.display()
        ))),
    }
}

/// Whether a klin.json is there to read at all, which tells a failure of `load` that names a
/// config error apart from one that names no file. Section 14.
pub fn present(explicit: Option<&Path>, start: &Path) -> bool {
    match explicit {
        Some(named) => absolute(named, start).is_file(),
        None => find(start).is_some(),
    }
}

/// A key klin does not read measures nothing and would otherwise pass in silence, so it is a
/// config error naming the file and the key. Sections 5.2 and 14.
fn every_key_is_one_klin_reads(file: &Path, data: &Value) -> Result<(), Error> {
    let Some(fields) = data.as_object() else {
        return Ok(());
    };
    let known = |key: &str| KEYS.contains(&key) || crate::gate::sections().any(|read| read == key);
    let Some(unknown) = fields.keys().find(|key| !known(key)) else {
        return Ok(());
    };
    if let Some(section) = crate::gate::command_named(unknown) {
        return Err(Error(format!(
            "{}: \"{unknown}\" is what the command is called — the section it reads is \
             \"{section}\"",
            file.display()
        )));
    }
    Err(Error(format!(
        "{}: \"{unknown}\" is not a key klin reads — one of: {}",
        file.display(),
        KEYS.iter()
            .copied()
            .chain(crate::gate::sections())
            .collect::<Vec<&str>>()
            .join(", ")
    )))
}

fn find(start: &Path) -> Option<PathBuf> {
    let mut here = start.to_path_buf();
    loop {
        let candidate = here.join(FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !here.pop() {
            return None;
        }
    }
}

fn absolute(path: &Path, start: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        start.join(path)
    }
}

fn expand_home(relative: &str) -> PathBuf {
    match relative
        .strip_prefix("~/")
        .and_then(|rest| std::env::home_dir().map(|home| home.join(rest)))
    {
        Some(expanded) => expanded,
        None => PathBuf::from(relative),
    }
}
