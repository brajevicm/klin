use std::cell::{OnceCell, RefCell};
use std::fmt::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::reference::Key;
use crate::survey;

const FILENAME: &str = "klin.json";

/// The top-level keys, beside one key per gate named for its section. Every module that reads
/// one reads it through the declaration here, and `klin reference` prints them. Spec 5.2, 5.8.
pub const KEYS: &[Key] = &[PROJECT, VERSION, BUILD, ACCEPTED, RADIUS, JOURNAL, GATES];

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

pub const GATES: Key = Key {
    name: "gates",
    holds: "extra gates, each an entry of a `name`, a `check`, a `with` and an optional `off`",
    required: false,
    rule: None,
    default: "no gate beyond the sections",
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

/// The outcome of a file no grammar reads. The hook counts these to report the holes a
/// person must close, and nothing else in a run turns on it.
pub const UNPARSED: &str = "unparsed";

/// The outcome of a test the base holds that went in the window, which fails nothing. A stop the
/// hook lets end hands it to a person. Spec 8.2.
pub const DELETED: &str = "deleted";

#[derive(Default)]
pub struct Records {
    pub findings: Vec<Value>,
    pub notes: Vec<Value>,
    /// One row per gate the run judged, which only the runner fills in. Spec 11.2.
    pub gates: Vec<Value>,
    /// What scope the gate measured, which every check records once. Spec 11.2.
    pub coverage: Option<Value>,
    /// One `{section, key, value, rule}` entry per value the run derived. Spec 11.2.
    pub derived: Vec<Value>,
    /// The count the check's own `OK:` line prints as held at the base, which the runner puts
    /// on the gate's row. `None` for a gate that never got that far. Spec 11.2.
    pub held: Option<u64>,
}

pub struct Flags {
    pub config: Option<PathBuf>,
    /// The name of the gate being run, which the accepted list names.
    pub gate: String,
    /// The base commit, already laid out as a directory by the runner.
    pub prior: Option<PathBuf>,
    /// The base commit the runner chose, for a gate that reads the base tree out of git.
    pub base: Option<String>,
    /// Print nothing on success: no `OK:` line, and nothing under it.
    pub quiet: bool,
    /// Whether this gate says the run's own context for itself: the `window:` line and the
    /// `derived:` lines. The runner prints those once for the whole run, so it clears this and
    /// still gets each gate's `OK:` line. Spec 4.3, 11.1.
    pub context: bool,
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
    /// What the survey says for the sections the config does not pin, computed on the first
    /// section that needs it and never for a config that pins everything. Spec 4.3.
    derived: OnceCell<survey::Derived>,
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
        Ok(Config {
            file,
            root,
            data,
            derived: OnceCell::new(),
        })
    }

    /// A tree with no configuration at all. The file it names is the one `init` would write, so
    /// an error still points a person at the place a section belongs.
    fn unwritten(start: &Path) -> Config {
        let root = repository(start).unwrap_or_else(|| start.to_path_buf());
        Config {
            file: root.join(FILENAME),
            root,
            data: Value::Object(serde_json::Map::new()),
            derived: OnceCell::new(),
        }
    }

    /// Whether a person wrote this configuration, which tells an error that names a missing
    /// section from one that names a tree the survey found nothing in.
    pub fn written(&self) -> bool {
        self.file.is_file()
    }

    /// One line per value the run derived, and one per value the config pinned beside it. Empty
    /// for a config that pinned every section, because then nothing was derived. Spec 4.3.
    pub fn said(&self) -> &[String] {
        match self.derived.get() {
            Some(derived) => &derived.lines,
            None => &[],
        }
    }

    /// Whether the derivation commit's survey held the path this finding sits under. A site the
    /// survey did not hold matches nothing in `before`, whatever `before` holds there, so a
    /// directory that becomes a root cannot bring inherited debt with it. Spec 7.1.
    pub fn was_held(&self, file: &str) -> bool {
        let unheld = match self.derived.get() {
            Some(derived) => &derived.unheld,
            None => return true,
        };
        !unheld.iter().any(|root| survey::under_or_at(file, root))
    }

    /// The same lines, written out by a check a person ran by hand. The gate runner prints its
    /// own once for the whole run, so a gate stays quiet here. Spec 4.3.
    pub fn say(&self, flags: &Flags, section: &str, out: &mut String) {
        if !flags.context {
            return;
        }
        for line in self.said().iter().filter(|line| names(line, section)) {
            let _ = writeln!(out, "{line}");
        }
    }

    /// The section as the config itself states it, with nothing the survey would supply.
    pub fn pinned(&self, name: &str) -> Option<&Value> {
        self.data.get(name)
    }

    /// Whether any derivable section is left for the survey to fill in. A config that states
    /// every one of them derives nothing, so nothing walks the tree for it.
    pub fn derives_anything(&self) -> bool {
        survey::derivable().any(|name| match self.data.get(name) {
            Some(pinned) => !survey::pinned_whole(name, pinned),
            None => true,
        })
    }

    /// Whether the survey found no source root in this tree. The caller asks only when a check
    /// that measures code takes its roots from the survey, so a config that names its own roots
    /// surveys nothing for this. Spec 10, 14.
    pub fn found_no_source_root(&self) -> bool {
        self.derivation().roots.is_empty()
    }

    /// The `derived:` and `pinned:` lines about one section, which `--list` prints under the
    /// gate that reads it. Empty without running the survey when the config pins every
    /// derivable section, so `--list` on a fully pinned config walks no tree. Spec 10.
    pub fn said_about(&self, section: &str) -> Vec<String> {
        if !self.derives_anything() {
            return Vec::new();
        }
        self.derived_said()
            .iter()
            .filter(|line| names(line, section))
            .cloned()
            .collect()
    }

    /// The lines above, with the survey run if it has not run yet. The gate runner prints these
    /// once for the whole run, before any check reads a section of its own.
    pub fn derived_said(&self) -> &[String] {
        &self.derivation().lines
    }

    /// The `derived_said` lines again, as the `{section, key, value, rule}` entries `--json`
    /// prints instead. One entry per `derived:` line, built beside it so the two cannot drift.
    /// Empty without running the survey when the config pins every derivable section, so a
    /// fully-pinned run reports `derived` as empty without walking the tree for it. Spec 11.2.
    pub fn derived_values(&self) -> Vec<Value> {
        if !self.derives_anything() {
            return Vec::new();
        }
        self.derivation().values.clone()
    }

    fn derivation(&self) -> &survey::Derived {
        self.derived
            .get_or_init(|| survey::derive(&self.root, &self.data))
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

    /// The section a check reads: what the config pins, filled in from the survey for a key it
    /// leaves out. A section klin cannot derive and the config does not name is an error naming
    /// the key. Spec 5.1, 5.2.
    pub fn section(&self, name: &str) -> Result<&Value, Error> {
        let missing = || Error(format!("{} has no \"{name}\" section", self.file.display()));
        if survey::keys(name).is_none() {
            return self.data.get(name).ok_or_else(missing);
        }
        if let Some(pinned) = self.data.get(name)
            && survey::pinned_whole(name, pinned)
        {
            return Ok(pinned);
        }
        self.derivation()
            .sections
            .get(name)
            .or_else(|| self.data.get(name))
            .ok_or_else(missing)
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
        KEYS.iter().any(|held| held.name == key) || crate::gate::sections().any(|read| read == key)
    };
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
            .map(|key| key.name)
            .chain(crate::gate::sections())
            .collect::<Vec<&str>>()
            .join(", ")
    )))
}

/// Whether a `derived:` or `pinned:` line is about this section, so a check a person ran by
/// hand prints the values it used and not another gate's.
fn names(line: &str, section: &str) -> bool {
    line.split_once(": ")
        .and_then(|(_, rest)| rest.strip_prefix(section))
        .is_some_and(|rest| rest.starts_with(' '))
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
