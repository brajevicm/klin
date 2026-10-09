use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::Error;
use crate::key::{Key, Section, SectionShape, Shape};

pub const FILENAME: &str = "klin.json";

/// The top-level keys, beside one key per gate named for its section. Every module that reads
/// one reads it through the declaration here, and `klin policy --reference` prints them. Spec 5.2, 5.8.
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

/// The section keys klin no longer reads, each with what to do instead. Section 14.
const RETIRED_KEYS: &[(&str, &str)] = &[
    (
        "baseline",
        "which is not a key klin reads — a run compares the working tree against the base \
         commit, and a person accepts debt in the \"accepted\" list. Delete the key and the \
         file it names.",
    ),
    (
        "sources",
        "which klin now spells \"roots\", the name every section uses for the same thing. \
         Rename the key, so nothing measures a different set in silence.",
    ),
];

pub const BUILD: Key = Key {
    name: "build",
    holds: "a build command a person chose over the derived one: a command, a list of entries of a `run` and an optional `root`, or `false` to build nothing",
    required: false,
    rule: Some("one command per standard manifest, from the fixed table of ADR 0012"),
    default: "",
    shape: Shape::Build,
};

pub const ACCEPTED: Key = Key {
    name: "accepted",
    holds: "the debt a person accepted, each entry a site and a reason. Only a person writes it",
    required: false,
    rule: None,
    default: "nothing is accepted",
    shape: Shape::Accepted,
};

pub const RADIUS: Key = Key {
    name: "radius",
    holds: "the change radius a turn may not pass, as `lines` and `directories`",
    required: false,
    rule: Some(
        "the 90th percentile over the last 200 non-merge commits, and no section below 50 commits",
    ),
    default: "",
    shape: Shape::Radius,
};

pub const JOURNAL: Key = Key {
    name: "journal",
    holds: "how the journal records a turn, as `prompt`, `false` to record no prompt excerpt",
    required: false,
    rule: None,
    default: "the prompt excerpt is recorded",
    shape: Shape::Journal,
};

/// The policy a person wrote, and nothing klin computed. What the file leaves out is derived
/// from the tree by the run that reads it, through `project::Project`, never here. ADR 0038.
pub struct Config {
    pub file: PathBuf,
    root: PathBuf,
    data: Value,
    /// The sections the file was judged against, which say how each section names its paths.
    sections: Vec<Section>,
}

impl Config {
    /// The config, or an empty one when there is no file. `klin.json` is optional: a tree that
    /// has none is gated by every Automatic check over its facts. The sections are the ones the
    /// catalogue declares, which the file is judged against. ADR 0016, ADR 0040, spec 5.1.
    pub fn load(
        explicit: Option<&Path>,
        start: &Path,
        sections: &[Section],
    ) -> Result<Config, Error> {
        let Some(file) = named(explicit, start) else {
            return Ok(Config::unwritten(start));
        };
        let text = std::fs::read_to_string(&file).map_err(|why| Error::unreadable(&file, why))?;
        let data = serde_json::from_str(&text).map_err(|why| Error::unreadable(&file, why))?;
        let root = file.parent().unwrap_or(Path::new("")).to_path_buf();
        well_formed(&file, &data, sections)?;
        Ok(Config {
            file,
            root,
            data,
            sections: sections.to_vec(),
        })
    }

    /// A tree with no configuration at all. The file it names is the one `init` would write, so
    /// an error still points a person at the place a section belongs.
    fn unwritten(start: &Path) -> Config {
        let root = Discovered::from(start)
            .root
            .unwrap_or_else(|| start.to_path_buf());
        Config {
            file: root.join(FILENAME),
            root,
            data: Value::Object(serde_json::Map::new()),
            sections: Vec::new(),
        }
    }

    /// A configuration that states nothing yet, at the file `init --pin` is about to write.
    pub fn empty(file: &Path) -> Config {
        Config {
            file: file.to_path_buf(),
            root: file.parent().unwrap_or(Path::new("")).to_path_buf(),
            data: Value::Object(Map::new()),
            sections: Vec::new(),
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

    /// Every section the config states as an object, which is every one a scope can sit in.
    pub fn objects(&self) -> impl Iterator<Item = (&str, &Map<String, Value>)> {
        self.data
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(name, value)| Some((name.as_str(), value.as_object()?)))
    }

    /// Every section the config states as an object, with the shape the file was judged by.
    pub fn shaped(&self) -> impl Iterator<Item = (&str, SectionShape, &Map<String, Value>)> {
        self.objects().filter_map(|(name, fields)| {
            let section = self.sections.iter().find(|section| section.name == name)?;
            Some((name, section.shape, fields))
        })
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
        Error::malformed(&self.file, section, key, must_be)
    }

    /// One Automatic check's human policy, empty when absent and refused when it names a field
    /// that check does not read. Check-specific meaning stays in the check.
    pub fn policy(&self, section: &str, keys: &[Key]) -> Result<Map<String, Value>, Error> {
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
fn well_formed(file: &Path, data: &Value, sections: &[Section]) -> Result<(), Error> {
    every_key_is_one_klin_reads(file, data, sections)?;
    no_section_names_a_retired_key(file, data)?;
    structure(file, data, sections)?;
    no_stale_debt(file, data, sections)?;
    crate::ceiling::every_schedule(file, data)
}

/// The structural contract shared by the native reader and the generated schema. A rule belongs
/// here only when klin.json alone decides it, with each section's shape passed in as data. A rule
/// that needs a tree, a parser or a check's own reading stays in the check that owns it, and the
/// dated steps stay in `ceiling`, which imports nothing above `error`. Spec 5.2, 5.3, 5.5.
fn structure(file: &Path, data: &Value, sections: &[Section]) -> Result<(), Error> {
    let fields = data
        .as_object()
        .ok_or_else(|| Error(format!("{}: klin.json must be an object", file.display())))?;
    for key in KEYS {
        if let Some(value) = fields.get(key.name) {
            value_shape(file, key.name, key, value)?;
        }
    }
    for check in sections {
        if let Some(value) = fields.get(check.name) {
            section_shape(file, check, value)?;
        }
    }
    Ok(())
}

fn section_shape(file: &Path, check: &Section, value: &Value) -> Result<(), Error> {
    match check.shape {
        SectionShape::Object => object_section_shape(file, check, value),
        SectionShape::DocumentMap(document) => document_map_shape(file, check, document, value),
        SectionShape::FalseOnly(_) => false_only_shape(file, check, value),
        SectionShape::Conventions(instead) => dynamic_conventions(file, check, instead, value),
        SectionShape::Sarif => named_entries_shape(file, check, value),
    }
}

fn object_section_shape(file: &Path, check: &Section, value: &Value) -> Result<(), Error> {
    if check.automatic && matches!(value, Value::Array(_)) {
        return retired_list(file, check);
    }
    disabled_object(file, check.name, value, check.keys)
}

fn false_only_shape(file: &Path, check: &Section, value: &Value) -> Result<(), Error> {
    match (check.automatic, value) {
        (_, Value::Bool(false)) => Ok(()),
        (true, Value::Array(_)) => retired_list(file, check),
        (true, Value::Object(_)) => Err(Error(format!(
            "{}: \"{}\" reads no policy — {}",
            file.display(),
            check.name,
            policy(check.shape)
        ))),
        _ => Err(shape_error(file, check.name, check.name, "false")),
    }
}

fn document_map_shape(
    file: &Path,
    check: &Section,
    document: &Key,
    value: &Value,
) -> Result<(), Error> {
    let section = check.name;
    match value {
        Value::Bool(false) => Ok(()),
        Value::Array(_) => retired_list(file, check),
        Value::Object(fields) => {
            if fields.is_empty() {
                return Err(Error(format!(
                    "{}: \"{section}\" must pin at least one document — remove the section to derive every ceiling",
                    file.display()
                )));
            }
            fields
                .iter()
                .try_for_each(|(name, value)| document_shape(file, section, document, name, value))
        }
        _ => Err(Error(format!(
            "{}: \"{section}\" must be an object or false — {}",
            file.display(),
            policy(check.shape)
        ))),
    }
}

fn document_shape(
    file: &Path,
    section: &str,
    document: &Key,
    name: &str,
    value: &Value,
) -> Result<(), Error> {
    if name.is_empty() {
        return Err(document_error(file, section, name));
    }
    match value_shape(file, section, document, value) {
        Ok(()) => Ok(()),
        Err(error) if value.is_object() => Err(error),
        Err(_) => Err(document_error(file, section, name)),
    }
}

fn document_error(file: &Path, section: &str, name: &str) -> Error {
    Error(format!(
        "{}: \"{section}\" \"{name}\" must be a whole number of words or an object of dated steps — \"{section}\" maps a document path to its ceiling, such as {{\"README.md\": 1200}}",
        file.display()
    ))
}

fn retired_list(file: &Path, check: &Section) -> Result<(), Error> {
    Err(Error(format!(
        "{}: \"{}\" no longer accepts a list of entries, because klin discovers what it applies to — {}",
        file.display(),
        check.name,
        policy(check.shape)
    )))
}

fn disabled_object(file: &Path, section: &str, value: &Value, keys: &[Key]) -> Result<(), Error> {
    match value {
        Value::Bool(false) => Ok(()),
        Value::Object(fields) => fields_shape(file, section, fields, keys, true),
        _ => Err(shape_error(file, section, section, "an object or false")),
    }
}

fn fields_shape(
    file: &Path,
    section: &str,
    fields: &Map<String, Value>,
    keys: &[Key],
    non_empty: bool,
) -> Result<(), Error> {
    if non_empty && fields.is_empty() {
        return Err(Error(format!(
            "{}: \"{section}\" must state at least one of: {}",
            file.display(),
            keys.iter()
                .map(|key| key.name)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let names: Vec<&str> = keys.iter().map(|key| key.name).collect();
    known_fields(file, section, fields, &names)?;
    keys.iter()
        .try_for_each(|key| field_shape(file, section, fields, key))
}

fn field_shape(
    file: &Path,
    section: &str,
    fields: &Map<String, Value>,
    key: &Key,
) -> Result<(), Error> {
    match fields.get(key.name) {
        Some(value) => value_shape(file, section, key, value),
        None if key.required => missing(file, section, key.name),
        None => Ok(()),
    }
}

fn value_shape(file: &Path, section: &str, key: &Key, value: &Value) -> Result<(), Error> {
    match key.shape {
        Shape::String => require(file, section, key.name, value.is_string(), "a string"),
        Shape::Boolean => require(file, section, key.name, value.is_boolean(), "true or false"),
        Shape::WholeNumber => require(file, section, key.name, value.is_u64(), "a whole number"),
        Shape::Ceiling => ceiling_shape(file, section, key, value),
        Shape::Strings => require(
            file,
            section,
            key.name,
            value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string)),
            "a list of strings",
        ),
        Shape::StringOrList => require(
            file,
            section,
            key.name,
            string_or_list(value),
            "a non-empty path or list of paths",
        ),
        shape => complex_value_shape(file, section, key, value, shape),
    }
}

fn ceiling_shape(file: &Path, section: &str, key: &Key, value: &Value) -> Result<(), Error> {
    if value.is_u64() {
        return Ok(());
    }
    if value.is_object() {
        return crate::ceiling::read(file, section, key.name, value, "a whole number").map(|_| ());
    }
    Err(shape_error(
        file,
        section,
        key.name,
        "a whole number or an object of dated steps",
    ))
}

fn complex_value_shape(
    file: &Path,
    section: &str,
    key: &Key,
    value: &Value,
    shape: Shape,
) -> Result<(), Error> {
    match shape {
        Shape::Language(languages) => require(
            file,
            section,
            key.name,
            value
                .as_str()
                .is_some_and(|name| languages().iter().any(|(known, _)| *known == name)),
            "a supported language name",
        ),
        Shape::Build => build_shape(file, section, key, value),
        Shape::Accepted => accepted_shape(file, section, key, value),
        Shape::Radius => radius_shape(file, section, key, value),
        Shape::Journal => journal_shape(file, section, key, value),
        Shape::Layers => layers_shape(file, section, key, value),
        _ => Ok(()),
    }
}

fn build_shape(file: &Path, section: &str, key: &Key, value: &Value) -> Result<(), Error> {
    match value {
        Value::Bool(false) | Value::String(_) => Ok(()),
        Value::Array(entries) if entries.iter().all(Value::is_object) => {
            entries.iter().try_for_each(|entry| {
                let Some(fields) = entry.as_object() else {
                    return Err(build_error(file, key.name));
                };
                fields_shape(
                    file,
                    section,
                    fields,
                    &[
                        Key {
                            name: crate::key::BUILD_RUN,
                            holds: "",
                            required: true,
                            rule: None,
                            default: "",
                            shape: Shape::String,
                        },
                        Key {
                            name: crate::key::BUILD_ROOT,
                            holds: "",
                            required: false,
                            rule: None,
                            default: "",
                            shape: Shape::String,
                        },
                    ],
                    false,
                )
            })
        }
        _ => Err(build_error(file, key.name)),
    }
}

fn build_error(file: &Path, key: &str) -> Error {
    Error(format!(
        "{}: \"{key}\" is a command, a list of {{\"root\", \"run\"}} entries, or false",
        file.display()
    ))
}

/// The name of the built-in row of lost files and of the accepted entries that hold one. No
/// capability, `sarif` entry or other accepted entry may take it. Spec 7.2.
pub const MEASUREMENT_LOST: &str = "measurement-lost";

pub const ACCEPTED_REASON: &str = "reason";

type FieldShape = (&'static str, fn(&Value) -> bool, &'static str);

const ACCEPTED_OPTIONAL: &[FieldShape] = &[
    ("line", Value::is_u64, "a whole number"),
    (ACCEPTED_REASON, Value::is_string, "a string"),
];

fn accepted_shape(file: &Path, section: &str, key: &Key, value: &Value) -> Result<(), Error> {
    let Some(entries) = value.as_array() else {
        return Err(shape_error(
            file,
            section,
            key.name,
            "a list of accepted entries",
        ));
    };
    for entry in entries {
        let Some(fields) = entry.as_object() else {
            return Err(shape_error(
                file,
                section,
                key.name,
                "a list of accepted entries",
            ));
        };
        accepted_entry_shape(file, section, fields)?;
    }
    Ok(())
}

fn accepted_entry_shape(
    file: &Path,
    section: &str,
    fields: &Map<String, Value>,
) -> Result<(), Error> {
    let by_file = fields.get("gate").and_then(Value::as_str) == Some(MEASUREMENT_LOST);
    let required: &[&str] = match by_file {
        true => &["gate", "file"],
        false => &["gate", "file", "text"],
    };
    for name in required {
        if !fields.get(*name).is_some_and(Value::is_string) {
            return missing(file, section, name);
        }
    }
    match ACCEPTED_OPTIONAL
        .iter()
        .find(|(name, fits, _)| fields.get(*name).is_some_and(|value| !fits(value)))
    {
        Some((name, _, must_be)) => Err(shape_error(file, section, name, must_be)),
        None => Ok(()),
    }
}

fn radius_shape(file: &Path, section: &str, key: &Key, value: &Value) -> Result<(), Error> {
    object_shape(
        file,
        section,
        key,
        value,
        &[
            ("lines", Shape::WholeNumber),
            ("directories", Shape::WholeNumber),
        ],
    )
}

fn journal_shape(file: &Path, section: &str, key: &Key, value: &Value) -> Result<(), Error> {
    object_shape(file, section, key, value, &[("prompt", Shape::Boolean)])
}

fn object_shape(
    file: &Path,
    section: &str,
    key: &Key,
    value: &Value,
    fields: &[(&'static str, Shape)],
) -> Result<(), Error> {
    let Some(object) = value.as_object() else {
        return Err(shape_error(file, section, key.name, "an object"));
    };
    let names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
    known_fields(file, section, object, &names)?;
    for (name, shape) in fields {
        if let Some(value) = object.get(*name) {
            value_shape(
                file,
                section,
                &Key {
                    name,
                    holds: "",
                    required: false,
                    rule: None,
                    default: "",
                    shape: *shape,
                },
                value,
            )?;
        }
    }
    Ok(())
}

fn layers_shape(file: &Path, section: &str, key: &Key, value: &Value) -> Result<(), Error> {
    let Some(layers) = value.as_object().filter(|layers| !layers.is_empty()) else {
        return Err(shape_error(
            file,
            section,
            key.name,
            "a non-empty map of layers",
        ));
    };
    for (name, value) in layers {
        let Some(fields) = value.as_object() else {
            return Err(shape_error(
                file,
                section,
                name,
                "an object with an \"in\" path",
            ));
        };
        known_fields(
            file,
            &format!("{section} layer {name}"),
            fields,
            &["in", "can_use"],
        )?;
        let within = fields.get("in").ok_or_else(|| {
            Error(format!(
                "{}: \"{section}\" layer \"{name}\" has no \"in\"",
                file.display()
            ))
        })?;
        if !string_or_list(within) {
            return Err(shape_error(
                file,
                section,
                "in",
                "a non-empty path or list of paths",
            ));
        }
        if let Some(can_use) = fields.get("can_use")
            && !can_use.is_null()
            && !can_use
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string))
        {
            return Err(shape_error(
                file,
                section,
                "can_use",
                "a list of layer names or null",
            ));
        }
    }
    Ok(())
}

fn dynamic_conventions(
    file: &Path,
    check: &Section,
    instead: &[(&'static str, &'static str)],
    value: &Value,
) -> Result<(), Error> {
    match value {
        Value::Bool(false) => Ok(()),
        Value::Object(conventions) if !conventions.is_empty() => {
            conventions.iter().try_for_each(|(name, value)| {
                convention_shape_entry(file, check.keys, instead, name, value)
            })
        }
        Value::Object(_) => Err(Error(format!(
            "{}: \"{}\" names no convention — write one, or set the section to false",
            file.display(),
            check.name
        ))),
        _ => Err(Error(format!(
            "{}: \"{}\" is an object of convention names, each with a \"remedy\" and one of: text, code, files",
            file.display(),
            check.name
        ))),
    }
}

fn convention_shape_entry(
    file: &Path,
    keys: &[Key],
    instead: &[(&'static str, &'static str)],
    name: &str,
    value: &Value,
) -> Result<(), Error> {
    let fields = value.as_object().ok_or_else(|| {
        Error(format!(
            "{}: convention \"{name}\" must be an object with a matcher and a remedy",
            file.display()
        ))
    })?;
    known_convention(fields, keys, instead)
        .map_err(|why| Error(format!("{}: convention \"{name}\" {why}", file.display())))?;
    convention_fields(file, keys, name, fields)?;
    convention_matcher(file, name, fields)?;
    convention_language(file, keys, name, fields)?;
    convention_remedy(file, name, fields)
}

/// A key a convention does not read would measure nothing, so it is refused, naming the key a
/// person most likely meant: one the convention reads, or the one `instead` maps a near miss to.
pub fn known_convention(
    fields: &Map<String, Value>,
    keys: &[Key],
    instead: &[(&'static str, &'static str)],
) -> Result<(), String> {
    let Some(unknown) = fields
        .keys()
        .find(|key| !keys.iter().any(|held| held.name == *key))
    else {
        return Ok(());
    };
    let candidates = || {
        keys.iter()
            .map(|key| (key.name, key.name))
            .chain(instead.iter().copied())
    };
    let meant = nearest(unknown, candidates().map(|(near, _)| near))
        .and_then(|near| candidates().find(|(held, _)| *held == near));
    Err(match meant {
        Some((_, key)) => format!("has unknown field \"{unknown}\"\nDid you mean \"{key}\"?"),
        None => format!(
            "has unknown field \"{unknown}\" — a convention reads only: {}",
            keys.iter()
                .map(|key| key.name)
                .collect::<Vec<&str>>()
                .join(", ")
        ),
    })
}

/// Each convention a `Conventions` section names runs as its own gate, so an accepted entry for one
/// of those gates must name a convention the section defines. A convention renamed or removed
/// retires its debt, and an entry left behind would otherwise hold nothing in silence while the
/// gate it names never runs. Refused before any gate runs. A section that is absent or `false`
/// runs no gate, so its entries wait for it, as an excluded gate's entries do. Spec 8.4.
fn no_stale_debt(file: &Path, data: &Value, sections: &[Section]) -> Result<(), Error> {
    let listed = data.get(ACCEPTED.name).and_then(Value::as_array);
    for check in sections {
        let SectionShape::Conventions(_) = check.shape else {
            continue;
        };
        let Some(defined) = data.get(check.name).and_then(Value::as_object) else {
            continue;
        };
        for entry in listed.into_iter().flatten() {
            let Some(gate) = entry.get("gate").and_then(Value::as_str) else {
                continue;
            };
            let Some(name) = crate::key::entry_named(check.name, gate) else {
                continue;
            };
            if !defined.contains_key(name) {
                return Err(Error(format!(
                    "{}: the accepted entry for {gate} names no convention the \"{}\" section \
                     defines — renaming or removing a convention retires its debt, so delete the \
                     entry or restore the convention",
                    file.display(),
                    check.name
                )));
            }
        }
    }
    Ok(())
}

fn convention_fields(
    file: &Path,
    keys: &[Key],
    name: &str,
    fields: &Map<String, Value>,
) -> Result<(), Error> {
    for key in keys {
        if key.name != "language"
            && let Some(value) = fields.get(key.name)
            && !convention_shape(key, value)
        {
            return Err(convention_shape_error(file, name, key.name, value));
        }
    }
    Ok(())
}

fn convention_matcher(file: &Path, name: &str, fields: &Map<String, Value>) -> Result<(), Error> {
    let matchers: Vec<&str> = ["text", "code", "files"]
        .into_iter()
        .filter(|matcher| fields.contains_key(*matcher))
        .collect();
    match matchers.as_slice() {
        [] => Err(Error(format!(
            "{}: convention \"{name}\" defines none of: text, code, files\nChoose exactly one of: text, code, files.",
            file.display()
        ))),
        [first, second, ..] => Err(Error(format!(
            "{}: convention \"{name}\" defines both \"{first}\" and \"{second}\"\nChoose exactly one of: text, code, files.",
            file.display()
        ))),
        [_] => Ok(()),
    }
}

fn convention_language(
    file: &Path,
    keys: &[Key],
    name: &str,
    fields: &Map<String, Value>,
) -> Result<(), Error> {
    let Some(language) = fields.get("language") else {
        return Ok(());
    };
    if !fields.contains_key("code") {
        return Err(Error(format!(
            "{}: convention \"{name}\" sets \"language\" on a rule that is not \"code\"",
            file.display()
        )));
    }
    if !language_is_known(keys, language) {
        let named = language.as_str().unwrap_or_default();
        let known = language_names(keys).join(", ");
        return Err(Error(format!(
            "{}: convention \"{name}\" names language \"{named}\", which no code pattern is written in — one of: {known}",
            file.display()
        )));
    }
    Ok(())
}

fn convention_remedy(file: &Path, name: &str, fields: &Map<String, Value>) -> Result<(), Error> {
    if fields
        .get("remedy")
        .and_then(Value::as_str)
        .is_some_and(|remedy| !remedy.trim().is_empty())
    {
        return Ok(());
    }
    Err(Error(format!(
        "{}: convention \"{name}\" has no \"remedy\" — write the exact action to take instead",
        file.display()
    )))
}

fn convention_shape(key: &Key, value: &Value) -> bool {
    match key.shape {
        Shape::String => value.is_string(),
        Shape::StringOrList => string_or_list(value),
        Shape::Language(languages) => value
            .as_str()
            .is_some_and(|name| languages().iter().any(|(known, _)| *known == name)),
        _ => true,
    }
}

fn convention_shape_error(file: &Path, name: &str, key: &str, value: &Value) -> Error {
    let expected = match key {
        "in" | "except" => format!(
            "has an \"{key}\" that is not a repository-relative path or a non-empty list of them"
        ),
        "language" => format!(
            "names language \"{}\", which no code pattern is written in",
            value.as_str().unwrap_or_default()
        ),
        "remedy" => "has no \"remedy\" — write the exact action to take instead".to_string(),
        key => format!("has a \"{key}\" that is not a string"),
    };
    Error(format!(
        "{}: convention \"{name}\" {expected}",
        file.display()
    ))
}

fn language_is_known(keys: &[Key], value: &Value) -> bool {
    keys.iter()
        .find(|key| key.name == "language")
        .is_some_and(|key| convention_shape(key, value))
}

fn language_names(keys: &[Key]) -> Vec<&'static str> {
    keys.iter()
        .find(|key| key.name == "language")
        .and_then(|key| match key.shape {
            Shape::Language(languages) => {
                Some(languages().into_iter().map(|(name, _)| name).collect())
            }
            _ => None,
        })
        .unwrap_or_default()
}

fn named_entries_shape(file: &Path, check: &Section, value: &Value) -> Result<(), Error> {
    let Value::Bool(false) = value else {
        let Some(entries) = value.as_array() else {
            return Err(Error(format!(
                "{}: \"{}\" is a list of entries, each its own gate under its own \"name\"",
                file.display(),
                check.name
            )));
        };
        for entry in entries {
            let Some(fields) = entry.as_object() else {
                return Err(shape_error(
                    file,
                    check.name,
                    check.name,
                    "a list of entries, or false",
                ));
            };
            fields_shape(file, check.name, fields, check.keys, false)?;
        }
        return Ok(());
    };
    Ok(())
}

fn string_or_list(value: &Value) -> bool {
    match value {
        Value::String(value) => !value.is_empty(),
        Value::Array(values) => {
            !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|value| !value.is_empty()))
        }
        _ => false,
    }
}

fn require(
    file: &Path,
    section: &str,
    key: &str,
    valid: bool,
    expected: &str,
) -> Result<(), Error> {
    valid
        .then_some(())
        .ok_or_else(|| shape_error(file, section, key, expected))
}

fn missing(file: &Path, section: &str, key: &str) -> Result<(), Error> {
    Err(Error(format!(
        "{}: a \"{section}\" entry has no \"{key}\"",
        file.display()
    )))
}

fn shape_error(file: &Path, section: &str, key: &str, expected: &str) -> Error {
    if section == key {
        Error(format!(
            "{}: \"{section}\" must be {expected}",
            file.display()
        ))
    } else {
        Error(format!(
            "{}: a \"{section}\" entry's \"{key}\" must be {expected}",
            file.display()
        ))
    }
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

/// A section key klin renamed, and the name the section reads it by now. Spec 5.2.
const RENAMED: &[(&str, &str)] = &[("skip_rust_tests", "skip_test_idioms")];

/// What an object section may say to narrow its check.
const NARROW: &str = "narrow the check only with \"in\" / \"except\", or set it to false";

/// What a section may say, in the words of the error that refused what it said.
fn policy(shape: SectionShape) -> &'static str {
    match shape {
        SectionShape::DocumentMap(_) => {
            "write a map of document path to ceiling, such as {\"README.md\": 1200}, or false"
        }
        SectionShape::FalseOnly(instead) => instead,
        _ => NARROW,
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
             discovered; {NARROW}",
            file.display()
        )));
    }
    if let Some((_, now)) = RENAMED
        .iter()
        .find(|(was, now)| unknown == was && known.contains(now))
    {
        return Err(Error(format!(
            "{}: \"{section}\" no longer reads \"{unknown}\", which klin renamed \"{now}\". \
             Rename the key.",
            file.display()
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
            no_retired_key(file, name, values)?;
        }
    }
    Ok(())
}

/// A section naming a key klin retired, refused before any gate runs. Section 14.
pub fn no_retired_key(file: &Path, name: &str, values: &Map<String, Value>) -> Result<(), Error> {
    for (retired, why) in RETIRED_KEYS {
        if values.contains_key(*retired) {
            return Err(Error(format!(
                "{}: \"{name}\" names a \"{retired}\", {why}",
                file.display()
            )));
        }
    }
    Ok(())
}

/// The lines `check`, `status` and `policy` print about discovery: that no `klin.json` is at
/// the worktree root, and each one below it that klin ignores. A `--config` names its own file,
/// so it has none. Spec 5.1.
pub fn notes(explicit: Option<&Path>, start: &Path) -> Vec<String> {
    match explicit {
        Some(_) => Vec::new(),
        None => Discovered::from(start).said(),
    }
}

/// The paths of every `klin.json` the walk from `start` passed and never read. Spec 5.1.
pub fn ignored(start: &Path) -> Vec<String> {
    Discovered::from(start).ignored_paths()
}

/// The configuration a run names or finds, whether or not it exists or reads.
pub fn located(explicit: Option<&Path>, start: &Path) -> Option<PathBuf> {
    match explicit {
        Some(named) => Some(absolute(named, start)),
        None => find(start),
    }
}

/// A key klin does not read measures nothing and would otherwise pass in silence, so it is a
/// config error naming the file and the key. Sections 5.2 and 14.
fn every_key_is_one_klin_reads(
    file: &Path,
    data: &Value,
    sections: &[Section],
) -> Result<(), Error> {
    let Some(fields) = data.as_object() else {
        return Ok(());
    };
    let every = || {
        KEYS.iter()
            .map(|key| key.name)
            .chain(sections.iter().map(|section| section.name))
    };
    let Some(unknown) = fields.keys().find(|key| !every().any(|read| read == *key)) else {
        return Ok(());
    };
    if let Some(section) = sections
        .iter()
        .find(|section| section.command != section.name && section.command == unknown)
    {
        return Err(Error(format!(
            "{}: \"{unknown}\" is what the command is called — the section it reads is \
             \"{}\"",
            file.display(),
            section.name
        )));
    }
    if let Some((_, instead)) = RETIRED.iter().find(|(retired, _)| retired == unknown) {
        return Err(Error(format!(
            "{}: \"{unknown}\" is not a key klin reads — {instead}",
            file.display()
        )));
    }
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
    Discovered::from(start).config
}

/// What the walk of spec 5.1 finds from one directory: the worktree root, the root's
/// `klin.json` when it holds one, and every `klin.json` between the directory and the root,
/// which klin never reads. Outside a worktree there is no root, and a command reads the
/// `klin.json` of the directory it starts in. The walk starts no git process and parses no file.
pub struct Discovered {
    pub root: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub ignored: Vec<PathBuf>,
}

impl Discovered {
    pub fn from(start: &Path) -> Discovered {
        let mut ignored = Vec::new();
        for here in start.ancestors() {
            let candidate = here.join(FILENAME);
            if worktree_root(here) {
                return Discovered {
                    root: Some(here.to_path_buf()),
                    config: candidate.is_file().then_some(candidate),
                    ignored,
                };
            }
            if candidate.is_file() {
                ignored.push(candidate);
            }
        }
        let candidate = start.join(FILENAME);
        Discovered {
            root: None,
            config: candidate.is_file().then_some(candidate),
            ignored: Vec::new(),
        }
    }

    /// The lines `check`, `status` and `policy` print about this walk: that it found no
    /// `klin.json` to read, and each file it passed and never read. Spec 5.1.
    pub fn said(&self) -> Vec<String> {
        let none = self
            .config
            .is_none()
            .then(|| "config: none, running under {}".to_string());
        let ignored = self.ignored.iter().map(|file| {
            format!(
                "NOTE: {} is ignored — klin reads only the klin.json at the worktree root.",
                file.display()
            )
        });
        none.into_iter().chain(ignored).collect()
    }

    /// The paths of every `klin.json` the walk passed and never read.
    pub fn ignored_paths(&self) -> Vec<String> {
        self.ignored
            .iter()
            .map(|file| file.display().to_string())
            .collect()
    }
}

/// A directory that holds a `.git` directory, or a `.git` file whose first line names an
/// existing git directory. Spec 5.1.
fn worktree_root(here: &Path) -> bool {
    let git = here.join(".git");
    if git.is_dir() {
        return true;
    }
    let Ok(text) = std::fs::read_to_string(&git) else {
        return false;
    };
    text.lines()
        .next()
        .and_then(|line| line.strip_prefix("gitdir: "))
        .is_some_and(|named| here.join(named.trim()).exists())
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
