use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::reference::Key;

const FILENAME: &str = "klin.json";

/// The top-level keys, beside one key per gate named for its section. Every module that reads
/// one reads it through the declaration here, and `klin reference` prints them. Spec 5.2, 5.8.
pub const KEYS: &[Key] = &[PROJECT, VERSION, BUILD, ACCEPTED, RADIUS, JOURNAL];

pub const PROJECT: Key = Key {
    name: "project",
    holds: "a name for reports",
    required: false,
    rule: None,
    default: "no name",
};

pub const VERSION: Key = Key {
    name: "version",
    holds: "the klin version this configuration was written for. A run under another version prints a NOTE naming both and continues",
    required: false,
    rule: None,
    default: "no version",
};

pub const BUILD: Key = Key {
    name: "build",
    holds: "the commands a run builds with, each an entry of a `run` and an optional `root`",
    required: false,
    rule: Some("one entry per manifest, from the fixed table of ADR 0012"),
    default: "",
};

pub const ACCEPTED: Key = Key {
    name: "accepted",
    holds: "the debt a person accepted, each entry a site and a reason. Only a person writes it",
    required: false,
    rule: None,
    default: "nothing is accepted",
};

pub const RADIUS: Key = Key {
    name: "radius",
    holds: "the change radius a turn may not pass, as `lines` and `directories`",
    required: false,
    rule: Some(
        "the 90th percentile over the last 200 non-merge commits, and no section below 50 commits",
    ),
    default: "",
};

pub const JOURNAL: Key = Key {
    name: "journal",
    holds: "how the journal records a turn, as `prompt`, `false` to record no prompt excerpt",
    required: false,
    rule: None,
    default: "the prompt excerpt is recorded",
};

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

/// The policy a person wrote, and nothing klin computed. What the file leaves out is derived
/// from the tree by the run that reads it, through `project::Project`, never here. ADR 0038.
pub struct Config {
    pub file: PathBuf,
    root: PathBuf,
    data: Value,
}

impl Config {
    /// The config, or the one klin derives when there is no file. `klin.json` is optional: a
    /// tree that has none is gated over the sections the survey supplies. ADR 0016, spec 5.1.
    pub fn load(explicit: Option<&Path>, start: &Path) -> Result<Config, Error> {
        let Some(file) = named(explicit, start) else {
            return Ok(Config::unwritten(start));
        };
        let text = std::fs::read_to_string(&file).map_err(|why| Error::unreadable(&file, why))?;
        let data = serde_json::from_str(&text).map_err(|why| Error::unreadable(&file, why))?;
        let root = file.parent().unwrap_or(Path::new("")).to_path_buf();
        well_formed(&file, &data)?;
        Ok(Config { file, root, data })
    }

    /// A tree with no configuration at all. The file it names is the one `init` would write, so
    /// an error still points a person at the place a section belongs.
    fn unwritten(start: &Path) -> Config {
        let root = repository(start).unwrap_or_else(|| start.to_path_buf());
        Config {
            file: root.join(FILENAME),
            root,
            data: Value::Object(serde_json::Map::new()),
        }
    }

    /// Whether a person wrote this configuration, which tells an error that names a missing
    /// section from one that names a tree the survey found nothing in.
    pub fn written(&self) -> bool {
        self.file.is_file()
    }

    /// The section as the config itself states it, with nothing the survey would supply.
    pub fn pinned(&self, name: &str) -> Option<&Value> {
        self.data.get(name)
    }

    /// Everything the file states, which the survey reads to tell a pinned key from a derived one.
    pub fn values(&self) -> &Value {
        &self.data
    }

    /// A section the file must state, because nothing derives it: the value, or the error that
    /// names the file and the key. Spec 5.1, 14.
    pub fn required(&self, name: &str) -> Result<&Value, Error> {
        self.pinned(name)
            .ok_or_else(|| Error(format!("{} has no \"{name}\" section", self.file.display())))
    }

    /// What to say when the config names a klin version other than the one running, and
    /// nothing when it names this one or none. A mismatch is a note. Section 5.2.
    pub fn version_note(&self) -> Option<String> {
        let running = env!("CARGO_PKG_VERSION");
        let named = self.data.get(VERSION.name)?.as_str()?;
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

    /// One Automatic check's human policy, empty when absent and refused when it names a field
    /// that check does not read. Check-specific meaning stays in the check.
    pub fn policy(
        &self,
        section: &str,
        keys: &[crate::reference::Key],
    ) -> Result<Map<String, Value>, Error> {
        let Some(value) = self.pinned(section) else {
            return Ok(Map::new());
        };
        let fields = value.as_object().ok_or_else(|| {
            Error(format!(
                "{}: \"{section}\" must be an object or false",
                self.file.display()
            ))
        })?;
        if fields.is_empty() {
            return Err(Error(format!(
                "{}: \"{section}\" must state at least one of: {}",
                self.file.display(),
                keys.iter()
                    .map(|key| key.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        if let Some(unknown) = fields
            .keys()
            .find(|name| !keys.iter().any(|key| key.name == *name))
        {
            return Err(Error(format!(
                "{}: \"{section}\" has unknown field \"{unknown}\" — it reads only: {}",
                self.file.display(),
                keys.iter()
                    .map(|key| key.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        Ok(fields.clone())
    }
}

/// What every command refuses before it reads a section: the config errors of section 14,
/// named against the file that holds them.
fn well_formed(file: &Path, data: &Value) -> Result<(), Error> {
    a_version_is_a_string(file, data)?;
    every_key_is_one_klin_reads(file, data)?;
    no_section_names_a_retired_key(file, data)?;
    no_source_gate_describes_the_repository(file, data)?;
    crate::conventions::no_stale_debt(file, data)?;
    crate::ceiling::every_schedule(file, data)
}

fn no_source_gate_describes_the_repository(file: &Path, data: &Value) -> Result<(), Error> {
    for check in crate::check::CATALOGUE.iter().filter(|check| {
        check.activation == crate::check::Activation::Automatic && check.derives.is_none()
    }) {
        let Some(value) = data.get(check.section) else {
            continue;
        };
        if value.is_array() {
            return Err(Error(format!(
                "{}: \"{}\" no longer accepts person-authored families — remove the list; narrow discovery only with \"in\" / \"except\"",
                file.display(),
                check.section
            )));
        }
        let Some(fields) = value.as_object() else {
            continue;
        };
        if let Some(key) = fields
            .keys()
            .find(|key| !check.keys.iter().any(|known| known.name == *key))
        {
            return Err(Error(format!(
                "{}: \"{}\" no longer reads \"{key}\" — repository topology is discovered; narrow the check only with \"in\" / \"except\"",
                file.display(),
                check.section
            )));
        }
    }
    Ok(())
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
    match data.get(VERSION.name) {
        None | Some(Value::String(_)) => Ok(()),
        Some(_) => Err(Error(format!(
            "{}: \"{}\" must be a klin version as a string",
            file.display(),
            VERSION.name
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
    let known = |key: &str| {
        KEYS.iter().any(|held| held.name == key) || crate::check::sections().any(|read| read == key)
    };
    let Some(unknown) = fields.keys().find(|key| !known(key)) else {
        return Ok(());
    };
    if let Some(section) = crate::check::command_named(unknown) {
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
            .map(|key| key.name)
            .chain(crate::check::sections())
            .collect::<Vec<&str>>()
            .join(", ")
    )))
}

/// The file this run reads, and `None` when there is none to read. An explicit `--config` that
/// is not there is still read, so a path a person typed wrong is an error and not a derivation.
fn named(explicit: Option<&Path>, start: &Path) -> Option<PathBuf> {
    match explicit {
        Some(named) => Some(absolute(named, start)),
        None => find(start),
    }
}

/// The top of the repository, which is where a configuration would sit and what paths resolve
/// against when there is none.
pub fn repository(start: &Path) -> Option<PathBuf> {
    let found = crate::changed::git(start, &["rev-parse", "--show-toplevel"])?;
    let named = PathBuf::from(found.trim());
    named.is_dir().then_some(named)
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
