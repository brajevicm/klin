use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::Error;
use crate::key::{Key, SectionShape, Shape};

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
    shape: crate::key::Shape::Build,
};

pub const ACCEPTED: Key = Key {
    name: "accepted",
    holds: "the debt a person accepted, each entry a site and a reason. Only a person writes it",
    required: false,
    rule: None,
    default: "nothing is accepted",
    shape: crate::key::Shape::Accepted,
};

pub const RADIUS: Key = Key {
    name: "radius",
    holds: "the change radius a turn may not pass, as `lines` and `directories`",
    required: false,
    rule: Some(
        "the 90th percentile over the last 200 non-merge commits, and no section below 50 commits",
    ),
    default: "",
    shape: crate::key::Shape::Radius,
};

pub const JOURNAL: Key = Key {
    name: "journal",
    holds: "how the journal records a turn, as `prompt`, `false` to record no prompt excerpt",
    required: false,
    rule: None,
    default: "the prompt excerpt is recorded",
    shape: crate::key::Shape::Journal,
};

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
        keys: &[crate::key::Key],
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
    structure(file, data)?;
    crate::conventions::no_stale_debt(file, data)?;
    crate::ceiling::every_schedule(file, data)
}

/// The structural contract shared by the native reader and the generated schema. Semantic rules
/// that need a tree, parser or dated step stay in the check that owns them. Spec 5.2, 5.3, 5.5.
fn structure(file: &Path, data: &Value) -> Result<(), Error> {
    let fields = data
        .as_object()
        .ok_or_else(|| Error(format!("{}: klin.json must be an object", file.display())))?;
    for key in KEYS {
        if let Some(value) = fields.get(key.name) {
            value_shape(file, key.name, key, value)?;
        }
    }
    for check in crate::check::CATALOGUE {
        if let Some(value) = fields.get(check.section) {
            section_shape(file, check, value)?;
        }
    }
    Ok(())
}

fn section_shape(file: &Path, check: &crate::check::Row, value: &Value) -> Result<(), Error> {
    match check.shape {
        SectionShape::Object => object_section_shape(file, check, value),
        SectionShape::DocumentMap => document_map_shape(file, check.section, value),
        SectionShape::FalseOnly => false_only_shape(file, check, value),
        SectionShape::Conventions => dynamic_conventions(file, check, value),
        SectionShape::Sarif => named_entries_shape(file, check, value),
    }
}

fn object_section_shape(
    file: &Path,
    check: &crate::check::Row,
    value: &Value,
) -> Result<(), Error> {
    if check.activation == crate::check::Activation::Automatic && matches!(value, Value::Array(_)) {
        return retired_list(file, check.section);
    }
    disabled_object(file, check.section, value, check.keys)
}

fn false_only_shape(file: &Path, check: &crate::check::Row, value: &Value) -> Result<(), Error> {
    match (
        check.activation == crate::check::Activation::Automatic,
        value,
    ) {
        (_, Value::Bool(false)) => Ok(()),
        (true, Value::Array(_)) => retired_list(file, check.section),
        (true, Value::Object(_)) => Err(Error(format!(
            "{}: \"{}\" reads no policy — {}",
            file.display(),
            check.section,
            policy_shape(check.section)
        ))),
        _ => Err(shape_error(file, check.section, check.section, "false")),
    }
}

fn document_map_shape(file: &Path, section: &str, value: &Value) -> Result<(), Error> {
    match value {
        Value::Bool(false) => Ok(()),
        Value::Array(_) => retired_list(file, section),
        Value::Object(fields) => {
            if fields.is_empty() {
                return Err(Error(format!(
                    "{}: \"{section}\" must pin at least one document — remove the section to derive every ceiling",
                    file.display()
                )));
            }
            fields
                .iter()
                .try_for_each(|(name, value)| document_shape(file, section, name, value))
        }
        _ => Err(Error(format!(
            "{}: \"{section}\" must be an object or false — {}",
            file.display(),
            policy_shape(section)
        ))),
    }
}

fn document_shape(file: &Path, section: &str, name: &str, value: &Value) -> Result<(), Error> {
    if name.is_empty() {
        return Err(document_error(file, section, name));
    }
    match value_shape(file, section, &crate::doc_size::DOCUMENT, value) {
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

fn retired_list(file: &Path, section: &str) -> Result<(), Error> {
    Err(Error(format!(
        "{}: \"{section}\" no longer accepts a list of entries, because klin discovers what it applies to — {}",
        file.display(),
        policy_shape(section)
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
        return crate::ceiling::read(
            &Config::empty(file),
            section,
            key.name,
            value,
            "a whole number",
        )
        .map(|_| ());
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
                            name: crate::key::RUN,
                            holds: "",
                            required: true,
                            rule: None,
                            default: "",
                            shape: Shape::String,
                        },
                        Key {
                            name: crate::key::ROOT,
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
        for name in ["gate", "file", "text"] {
            if !fields.get(name).is_some_and(Value::is_string) {
                return missing(file, section, name);
            }
        }
        if let Some(line) = fields.get("line")
            && !line.is_u64()
        {
            return Err(shape_error(file, section, "line", "a whole number"));
        }
    }
    Ok(())
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

fn dynamic_conventions(file: &Path, check: &crate::check::Row, value: &Value) -> Result<(), Error> {
    match value {
        Value::Bool(false) => Ok(()),
        Value::Object(conventions) if !conventions.is_empty() => conventions
            .iter()
            .try_for_each(|(name, value)| convention_shape_entry(file, check, name, value)),
        Value::Object(_) => Err(Error(format!(
            "{}: \"{}\" names no convention — write one, or set the section to false",
            file.display(),
            check.section
        ))),
        _ => Err(Error(format!(
            "{}: \"{}\" is an object of convention names, each with a \"remedy\" and one of: text, code, files",
            file.display(),
            check.section
        ))),
    }
}

fn convention_shape_entry(
    file: &Path,
    check: &crate::check::Row,
    name: &str,
    value: &Value,
) -> Result<(), Error> {
    let fields = value.as_object().ok_or_else(|| {
        Error(format!(
            "{}: convention \"{name}\" must be an object with a matcher and a remedy",
            file.display()
        ))
    })?;
    crate::conventions::known(fields)
        .map_err(|why| Error(format!("{}: convention \"{name}\" {why}", file.display())))?;
    convention_fields(file, check.keys, name, fields)?;
    convention_matcher(file, name, fields)?;
    convention_language(file, check.keys, name, fields)?;
    convention_remedy(file, name, fields)
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

fn named_entries_shape(file: &Path, check: &crate::check::Row, value: &Value) -> Result<(), Error> {
    let Value::Bool(false) = value else {
        let Some(entries) = value.as_array() else {
            return Err(Error(format!(
                "{}: \"{}\" is a list of entries, each its own gate under its own \"name\"",
                file.display(),
                check.section
            )));
        };
        for entry in entries {
            let Some(fields) = entry.as_object() else {
                return Err(shape_error(
                    file,
                    check.section,
                    check.section,
                    "a list of entries, or false",
                ));
            };
            fields_shape(file, check.section, fields, check.keys, false)?;
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
