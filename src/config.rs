use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::reference::Key;

const FILENAME: &str = "klin.json";

/// The top-level keys, beside one key per gate named for its section. Every module that reads
/// one reads it through the declaration here, and `klin reference` prints them. Spec 5.2, 5.8.
pub const KEYS: &[Key] = &[BUILD, ACCEPTED, RADIUS, JOURNAL];

/// The top-level keys klin no longer reads, each with what to do instead. Spec 5.2, ADR 0040.
const RETIRED: &[(&str, &str)] = &[
    (
        "project",
        "klin reads the repository's identity from the repository — delete the key",
    ),
    (
        "version",
        "the binary's version changes no gate, and a configuration names none — delete the key",
    ),
];

pub const BUILD: Key = Key {
    name: "build",
    holds: "a build command a person chose over the derived one: a command, a list of entries of a `run` and an optional `root`, or `false` to build nothing",
    required: false,
    rule: Some("one command per standard manifest, from the fixed table of ADR 0012"),
    default: "",
    shape: crate::reference::Shape::Build,
};

pub const ACCEPTED: Key = Key {
    name: "accepted",
    holds: "the debt a person accepted, each entry a site and a reason. Only a person writes it",
    required: false,
    rule: None,
    default: "nothing is accepted",
    shape: crate::reference::Shape::Accepted,
};

pub const RADIUS: Key = Key {
    name: "radius",
    holds: "the change radius a turn may not pass, as `lines` and `directories`",
    required: false,
    rule: Some(
        "the 90th percentile over the last 200 non-merge commits, and no section below 50 commits",
    ),
    default: "",
    shape: crate::reference::Shape::Radius,
};

pub const JOURNAL: Key = Key {
    name: "journal",
    holds: "how the journal records a turn, as `prompt`, `false` to record no prompt excerpt",
    required: false,
    rule: None,
    default: "the prompt excerpt is recorded",
    shape: crate::reference::Shape::Journal,
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
    /// The config, or an empty one when there is no file. `klin.json` is optional: a tree that
    /// has none is gated by every Automatic check over its facts. ADR 0016, ADR 0040, spec 5.1.
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

    /// A configuration that states nothing yet, at the file `init --pin` is about to write.
    pub fn empty(file: &Path) -> Config {
        Config {
            file: file.to_path_buf(),
            root: file.parent().unwrap_or(Path::new("")).to_path_buf(),
            data: Value::Object(Map::new()),
        }
    }

    /// Whether a person wrote this configuration, which tells an error that names a missing
    /// section from one that names a tree the survey found nothing in.
    pub fn written(&self) -> bool {
        self.file.is_file()
    }

    /// The section as the config itself states it, with nothing klin derives.
    pub fn pinned(&self, name: &str) -> Option<&Value> {
        self.data.get(name)
    }

    /// A section the file must state, because nothing derives it: the value, or the error that
    /// names the file and the key. Spec 5.1, 14.
    pub fn required(&self, name: &str) -> Result<&Value, Error> {
        self.pinned(name)
            .ok_or_else(|| Error(format!("{} has no \"{name}\" section", self.file.display())))
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
        let names: Vec<&str> = keys.iter().map(|key| key.name).collect();
        known_fields(&self.file, section, fields, &names)?;
        Ok(fields.clone())
    }
}

/// What every command refuses before it reads a section: the config errors of section 14,
/// named against the file that holds them.
fn well_formed(file: &Path, data: &Value) -> Result<(), Error> {
    every_key_is_one_klin_reads(file, data)?;
    no_section_names_a_retired_key(file, data)?;
    every_automatic_section_is_policy(file, data)?;
    nested_fields(file, data)?;
    crate::conventions::no_stale_debt(file, data)?;
    crate::ceiling::every_schedule(file, data)
}

/// The fields a section's entries once described the repository with. A person who still
/// writes one is told where that knowledge went. ADR 0040.
const TOPOLOGY: &[&str] = &[
    "roots",
    "languages",
    "patterns",
    "skip_dirs",
    "exclude",
    "exclude_except",
    "ceilings",
    "name",
    "path",
    "pattern",
    "file",
    "manifests",
    "extensions",
];

/// Every Automatic section is absent, `false`, or a person's policy: fields for most checks, a
/// document-to-ceiling map for `doc_size`, and nothing at all for `doc_citations`. A list of
/// generated entries is refused whole. Spec 5.2, 5.3.
fn every_automatic_section_is_policy(file: &Path, data: &Value) -> Result<(), Error> {
    crate::check::CATALOGUE
        .iter()
        .filter(|check| check.activation == crate::check::Activation::Automatic)
        .filter_map(|check| Some((check, data.get(check.section)?)))
        .try_for_each(|(check, value)| policy(file, check, value))
}

/// One Automatic section a person wrote, judged against the shape its check reads.
fn policy(file: &Path, check: &crate::check::Row, value: &Value) -> Result<(), Error> {
    let section = check.section;
    let refused = |why: &str| {
        Err(Error(format!(
            "{}: \"{section}\" {why} — {}",
            file.display(),
            policy_shape(section)
        )))
    };
    match value {
        Value::Bool(false) => Ok(()),
        Value::Array(_) => refused(
            "no longer accepts a list of entries, because klin discovers what it applies to",
        ),
        Value::Object(fields) if section == crate::doc_size::SECTION => {
            crate::doc_size::well_formed(file, fields)
        }
        Value::Object(_) if section == crate::doc_citations::SECTION => refused("reads no policy"),
        Value::Object(_) if section == crate::public_api::SECTION => refused("reads no policy"),
        Value::Object(fields) => {
            let names: Vec<&str> = check.keys.iter().map(|key| key.name).collect();
            known_fields(file, section, fields, &names)
        }
        _ => refused("must be an object or false"),
    }
}

/// What a section may say, in the words of the error that refused what it said.
fn policy_shape(section: &str) -> &'static str {
    match section {
        crate::doc_size::SECTION => {
            "write a map of document path to ceiling, such as {\"README.md\": 1200}, or false"
        }
        crate::doc_citations::SECTION => {
            "documents and citation roots are discovered; remove the section, or set it to false"
        }
        crate::public_api::SECTION => {
            "public surfaces are derived from Cargo library targets and package entry points; \
             remove the section, or set it to false"
        }
        _ => "narrow the check only with \"in\" / \"except\", or set it to false",
    }
}

/// A field a section does not read measures nothing and would pass in silence, so it is refused.
/// A retired topology field says where its knowledge went, and any other names the field a
/// person most likely meant. Spec 5.2, 14.
pub fn known_fields(
    file: &Path,
    section: &str,
    fields: &Map<String, Value>,
    known: &[&str],
) -> Result<(), Error> {
    let Some(unknown) = fields.keys().find(|key| !known.contains(&key.as_str())) else {
        return Ok(());
    };
    if TOPOLOGY.contains(&unknown.as_str()) {
        return Err(Error(format!(
            "{}: \"{section}\" no longer reads \"{unknown}\" — repository topology is \
             discovered; {}",
            file.display(),
            policy_shape(section)
        )));
    }
    Err(Error(match nearest(unknown, known.iter().copied()) {
        Some(meant) => format!(
            "{}: \"{section}\" has unknown field \"{unknown}\"\nDid you mean \"{meant}\"?",
            file.display()
        ),
        None => format!(
            "{}: \"{section}\" has unknown field \"{unknown}\" — it reads only: {}",
            file.display(),
            known.join(", ")
        ),
    }))
}

/// The top-level sections whose fields klin reads by name, each refused a field it does not
/// read, before any command runs. Spec 5.2.
fn nested_fields(file: &Path, data: &Value) -> Result<(), Error> {
    for (section, known) in [
        (RADIUS.name, &["lines", "directories"][..]),
        (JOURNAL.name, &["prompt"][..]),
    ] {
        if let Some(Value::Object(fields)) = data.get(section) {
            known_fields(file, section, fields, known)?;
        }
    }
    build_entries(file, data)
}

/// A `build` is a command, a list of entries of a `run` and an optional `root`, or `false`.
fn build_entries(file: &Path, data: &Value) -> Result<(), Error> {
    let entries = match data.get(BUILD.name) {
        None | Some(Value::String(_) | Value::Bool(false)) => return Ok(()),
        Some(Value::Array(entries)) if entries.iter().all(Value::is_object) => entries,
        Some(_) => {
            return Err(Error(format!(
                "{}: \"{}\" is a command, a list of {{\"root\", \"run\"}} entries, or false",
                file.display(),
                BUILD.name
            )));
        }
    };
    for fields in entries.iter().filter_map(Value::as_object) {
        known_fields(
            file,
            BUILD.name,
            fields,
            &[crate::build::RUN, crate::build::ROOT],
        )?;
    }
    Ok(())
}

/// The candidate a misspelling most likely meant: the nearest within two edits.
pub fn nearest<'a>(written: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .map(|candidate| (distance(written, candidate), candidate))
        .filter(|(apart, _)| *apart <= 2)
        .min()
        .map(|(_, candidate)| candidate)
}

/// The edits that turn one key into another, which is how near a misspelling is.
fn distance(from: &str, to: &str) -> usize {
    let to: Vec<char> = to.chars().collect();
    let mut row: Vec<usize> = (0..=to.len()).collect();
    for (at, left) in from.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = at + 1;
        for (column, right) in to.iter().enumerate() {
            let above = row[column + 1];
            row[column + 1] = (above + 1)
                .min(row[column] + 1)
                .min(diagonal + usize::from(left != *right));
            diagonal = above;
        }
    }
    row[to.len()]
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
    if let Some((_, instead)) = RETIRED.iter().find(|(retired, _)| retired == unknown) {
        return Err(Error(format!(
            "{}: \"{unknown}\" is not a key klin reads — {instead}",
            file.display()
        )));
    }
    let every = || {
        KEYS.iter()
            .map(|key| key.name)
            .chain(crate::check::sections())
    };
    let meant = nearest(unknown, every())
        .map(|meant| format!("\nDid you mean \"{meant}\"?"))
        .unwrap_or_default();
    Err(Error(format!(
        "{}: \"{unknown}\" is not a key klin reads — one of: {}{meant}",
        file.display(),
        every().collect::<Vec<&str>>().join(", ")
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
    let found = crate::git::Repo::at(start).text(&["rev-parse", "--show-toplevel"])?;
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
