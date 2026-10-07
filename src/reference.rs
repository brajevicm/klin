//! The configuration reference. Every check declares its keys beside the code that reads them,
//! and this module prints them as Markdown, so a person reads the keys without reading the
//! source. `docs/REFERENCE.md` holds the printed copy and a CLI test fails when the two differ,
//! so the committed copy cannot drift from the binary. A read takes the declared `Key` and not
//! a string of its own, so a key the reference does not print is a key no module can read.
//! Spec 5.8.

use std::fmt::Write;

use serde_json::{Map, Value, json};

use crate::check::catalogue;
use crate::config;
use crate::doc_citations;
use crate::error::Error;
use crate::key::{Key, Languages, SectionShape, Shape};

const HEAD: &str = "| Key | Holds | Required | Source | Derivation rule | Default |";
const RULE: &str = "| --- | --- | --- | --- | --- | --- |";
const NONE: &str = "—";

pub fn run(schema: bool, out: &mut String) -> Result<u8, Error> {
    match schema {
        true => self::schema(out),
        false => reference(out),
    }
    Ok(0)
}

fn reference(out: &mut String) {
    preamble(out);
    top_level(out);
    sections(out);
    languages(out);
    exclusion(out);
    ceilings(out);
}

const SCHEMA: &str = "https://json-schema.org/draft/2020-12/schema";
const SCHEMA_ID: &str = "https://raw.githubusercontent.com/brajevicm/klin/main/schemas/klin.json";
const DATE: &str = "^[0-9]{4}-[0-9]{2}-[0-9]{2}$";

fn schema(out: &mut String) {
    if let Ok(schema) = serde_json::to_string_pretty(&schema_value()) {
        let _ = writeln!(out, "{schema}");
    }
}

fn schema_value() -> Value {
    let mut properties = Map::new();
    for key in config::KEYS {
        properties.insert(key.name.into(), field(key));
    }
    for spec in catalogue::CATALOGUE {
        properties.insert(spec.section.into(), section(spec));
    }
    json!({
        "$schema": SCHEMA,
        "$id": SCHEMA_ID,
        "title": "klin.json",
        "description": "Human policy for klin; repository facts and semantic validation remain native to klin.",
        "type": "object",
        "properties": properties,
        "additionalProperties": false
    })
}

fn section(spec: &catalogue::Row) -> Value {
    match spec.shape {
        SectionShape::Object => disabled(object(spec.keys, true)),
        SectionShape::DocumentMap(document) => disabled(document_map(document)),
        SectionShape::FalseOnly(_) => json!({"const": false}),
        SectionShape::Conventions(_) => disabled(conventions(spec.keys)),
        SectionShape::Sarif => disabled(json!({
            "type": "array",
            "items": object(spec.keys, false)
        })),
    }
}

fn disabled(value: Value) -> Value {
    json!({"anyOf": [{"const": false}, value]})
}

fn document_map(document: &Key) -> Value {
    json!({
        "type": "object",
        "minProperties": 1,
        "propertyNames": {"minLength": 1},
        "additionalProperties": field(document)
    })
}

fn conventions(keys: &[Key]) -> Value {
    let mut item = object(keys, true);
    if let Value::Object(fields) = &mut item {
        fields.insert(
            "oneOf".into(),
            json!([
                {"required": ["text"], "not": {"anyOf": [{"required": ["code"]}, {"required": ["files"]}, {"required": ["language"]}]}},
                {"required": ["code"], "not": {"anyOf": [{"required": ["text"]}, {"required": ["files"]}]}},
                {"required": ["files"], "not": {"anyOf": [{"required": ["text"]}, {"required": ["code"]}, {"required": ["language"]}]}}
            ]),
        );
    }
    json!({
        "type": "object",
        "minProperties": 1,
        "propertyNames": {"minLength": 1},
        "additionalProperties": item
    })
}

fn object(keys: &[Key], minimum: bool) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for key in keys {
        if key.required {
            required.push(Value::from(key.name));
        }
        properties.insert(key.name.into(), field(key));
    }
    let mut out = Map::new();
    out.insert("type".into(), Value::from("object"));
    if minimum {
        out.insert("minProperties".into(), Value::from(1));
    }
    out.insert("properties".into(), Value::Object(properties));
    out.insert("additionalProperties".into(), Value::Bool(false));
    if !required.is_empty() {
        out.insert("required".into(), Value::Array(required));
    }
    Value::Object(out)
}

fn field(key: &Key) -> Value {
    let mut out = shape(key.shape);
    if let Value::Object(fields) = &mut out {
        fields.insert("description".into(), Value::from(key.holds));
    }
    out
}

fn shape(shape: Shape) -> Value {
    match shape {
        Shape::String => json!({"type": "string"}),
        Shape::Boolean => json!({"type": "boolean"}),
        Shape::WholeNumber => integer(),
        Shape::Ceiling => ceiling_schema(),
        Shape::Strings => json!({"type": "array", "items": {"type": "string"}}),
        Shape::StringOrList => string_or_list_schema(),
        shape => schema_for_shape(shape),
    }
}

fn schema_for_shape(shape: Shape) -> Value {
    match shape {
        Shape::Language(languages) => language_schema(languages),
        Shape::Build => build_schema(),
        Shape::Accepted => accepted_schema(),
        Shape::Radius => radius_schema(),
        Shape::Journal => journal_schema(),
        Shape::Layers => layers_schema(),
        _ => Value::Null,
    }
}

fn language_schema(languages: Languages) -> Value {
    json!({"type": "string", "enum": languages().into_iter().map(|(name, _)| name).collect::<Vec<_>>()})
}

fn build_schema() -> Value {
    json!({
        "anyOf": [
            {"type": "string"},
            {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "run": {"type": "string"},
                        "root": {"type": "string"}
                    },
                    "required": ["run"],
                    "additionalProperties": false
                }
            },
            {"const": false}
        ]
    })
}

fn accepted_schema() -> Value {
    json!({
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "gate": {"type": "string"},
                "file": {"type": "string"},
                "text": {"type": "string"},
                "line": integer()
            },
            "required": ["gate", "file", "text"],
            "additionalProperties": {"type": "number"}
        }
    })
}

fn radius_schema() -> Value {
    object_fields([("lines", integer()), ("directories", integer())])
}

fn journal_schema() -> Value {
    object_fields([("prompt", json!({"type": "boolean"}))])
}

fn layers_schema() -> Value {
    json!({
        "type": "object",
        "minProperties": 1,
        "additionalProperties": {
            "type": "object",
            "properties": {
                "in": string_or_list_schema(),
                "can_use": {"anyOf": [{"type": "null"}, {"type": "array", "items": {"type": "string"}}]}
            },
            "required": ["in"],
            "additionalProperties": false
        }
    })
}

fn object_fields<const N: usize>(fields: [(&str, Value); N]) -> Value {
    let properties = fields
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect::<Map<String, Value>>();
    Value::Object(Map::from_iter([
        ("type".into(), Value::from("object")),
        ("properties".into(), Value::Object(properties)),
        ("additionalProperties".into(), Value::Bool(false)),
    ]))
}

fn integer() -> Value {
    json!({"type": "integer", "minimum": 0})
}

fn string_or_list_schema() -> Value {
    json!({
        "anyOf": [
            {"type": "string", "minLength": 1},
            {"type": "array", "minItems": 1, "items": {"type": "string", "minLength": 1}}
        ]
    })
}

fn ceiling_schema() -> Value {
    let mut schedule = Map::new();
    schedule.insert(DATE.into(), integer());
    json!({
        "anyOf": [
            integer(),
            {
                "type": "object",
                "minProperties": 1,
                "patternProperties": schedule,
                "additionalProperties": false
            }
        ]
    })
}

fn preamble(out: &mut String) {
    let _ = writeln!(
        out,
        "# klin configuration reference\n\n\
         `klin policy --reference` prints this page. `docs/REFERENCE.md` holds the printed copy, and a \
         test fails when the two differ, so the reference cannot drift from the binary. Do not \
         edit the copy by hand.\n\n\
         `klin.json` is a person's policy over facts klin discovers in the tree. `{{}}` is a \
         complete configuration: every Automatic check runs over what the tree holds and \
         derives what the file leaves out. A section pins a decision and leaves the rest to \
         derivation, and a run prints one `pinned:` or `derived:` line per value it used. The \
         file never describes the repository: roots, languages, documents, manifests, test \
         roots and build commands are facts. A key or field klin does not read is an error \
         naming it. A gate is excluded by setting its section to `false`."
    );
}

fn top_level(out: &mut String) {
    let _ = writeln!(out, "\n## Top-level keys\n");
    table(config::KEYS, out);
}

fn sections(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Sections\n\n\
         One key per gate, named for its section. Every check discovers what it applies to; its \
         object holds only a person's policy, and a section reads only the keys its own table \
         names."
    );
    for spec in catalogue::CATALOGUE {
        let _ = writeln!(out, "\n### `{}`\n", spec.section);
        match spec.keys.is_empty() {
            true => {
                let _ = writeln!(
                    out,
                    "No keys: the section is absent, or `false` to exclude the gate."
                );
            }
            false => table(spec.keys, out),
        }
        if let Some(text) = spec.reference_text {
            let _ = writeln!(out, "\n{text}");
        }
    }
}

fn table(keys: &[Key], out: &mut String) {
    let _ = writeln!(out, "{HEAD}\n{RULE}");
    for key in keys {
        let source = match key.rule {
            None => "pinned only",
            Some(_) => "derived when absent",
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {source} | {} | {} |",
            key.name,
            cell(key.holds),
            yes(key.required),
            cell(key.rule.unwrap_or_default()),
            cell(key.default)
        );
    }
}

fn yes(required: bool) -> &'static str {
    match required {
        true => "yes",
        false => "no",
    }
}

/// One cell, with the pipe that would otherwise open a column escaped.
fn cell(text: &str) -> String {
    match text.is_empty() {
        true => NONE.to_string(),
        false => text.replace('|', "\\|"),
    }
}

fn languages(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Built-in language coverage\n\n\
         These tables report the source extensions each check discovers automatically. They \
         are capabilities of the binary, not selectors accepted in `klin.json`."
    );
    let named = catalogue::CATALOGUE
        .iter()
        .filter_map(|spec| Some((spec.section, spec.languages?)));
    for (section, rows) in named {
        let _ = writeln!(out, "\n### `{section}`\n");
        let _ = writeln!(out, "| Name | Extensions |\n| --- | --- |");
        for (name, extensions) in rows() {
            let _ = writeln!(out, "| `{name}` | {extensions} |");
        }
    }
}

fn exclusion(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Scope and discovery\n\n\
         Every source check discovers supported files from one repository walk, skips the \
         fixed directory list {}, and drops files git ignores. `in` narrows a check to a \
         repository-relative path (or non-empty list) and `except` takes paths back out. Each \
         path names itself and everything below it; neither key accepts globs.\n\n\
         `complexity`, `escapes`, `stubs`, `dead_symbols`, `reachability`, `inventory` and \
         `lockfile` reject the retired `roots`, `languages`, `patterns`, `skip_dirs`, \
         `exclude`, `exclude_except`, `ceilings`, `name`, `path`, `pattern` and `manifests` \
         topology keys with a migration error. A file measured under the base scope and \
         omitted by today's scope is a NOTE in the hook and exit 2 under `klin check`.\n\n\
         `doc_size` maps a document path to its ceiling, and `AGENTS.md` and `CLAUDE.md` at the \
         tree root keep a derived ceiling where it does not name them; every other document is \
         judged only when the map names it. `doc_citations` reads every Markdown file at the \
         tree root and resolves a citation against the whole tree; a citation names one of the \
         built-in extensions {}.",
        listed(&crate::files::default_skip_dirs()),
        listed(
            &doc_citations::EXTENSIONS
                .iter()
                .map(|extension| extension.to_string())
                .collect::<Vec<String>>()
        )
    );
}

fn listed(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<String>>()
        .join(", ")
}

fn ceilings(out: &mut String) {
    let _ = writeln!(
        out,
        "\n## Ceilings\n\n\
         A pinned ceiling is either a whole number or an object of dated steps:\n\n\
         ```json\n\
         \"complexity\": {{\n  \
         \"cc\": 12,\n  \
         \"lines\": {{ \"2026-09-08\": 90, \"2027-01-01\": 70, \"2027-07-01\": 60 }}\n\
         }}\n\
         ```\n\n\
         The run uses the lowest step whose date is on or before today, in UTC. A schedule with \
         no step yet due is an error. A run that uses a schedule prints the date it used beside \
         the ceiling. A derived ceiling is not monotone: it falls when simple functions arrive \
         and rises when simple functions leave, so a person who wants a ceiling that cannot \
         loosen pins one."
    );
}
