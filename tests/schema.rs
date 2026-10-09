mod harness;

use std::fs;

use Holds::{Named, Nothing, Schema, Schemas};
use harness::Tree;
use serde_json::Value;

fn schema() -> Value {
    let run = Tree::bare().run(&["policy", "--schema"]);
    assert_eq!(run.code, 0, "{}", run.out);
    serde_json::from_str(&run.printed)
        .unwrap_or_else(|why| panic!("reference --schema prints JSON: {why}"))
}

#[test]
fn the_schema_describes_the_compact_configuration_shapes() {
    let schema = schema();
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("top-level properties: {schema}"));

    top_level_schema(&schema, properties);
    compact_policy_schema(properties);
    document_map_schema(properties);
    convention_schema(properties);
    layering_schema(properties);
    sarif_schema(properties);
    accepted_schema(properties);
}

fn top_level_schema(schema: &Value, properties: &serde_json::Map<String, Value>) {
    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    assert_eq!(
        schema.get("additionalProperties"),
        Some(&Value::Bool(false))
    );
    for name in ["complexity", "doc_size", "conventions", "layering", "sarif"] {
        assert!(properties.contains_key(name));
    }
    assert!(!properties.contains_key("version"));
    assert!(schema.get("required").is_none());
}

fn compact_policy_schema(properties: &serde_json::Map<String, Value>) {
    let complexity = &properties["complexity"]["anyOf"];
    assert!(complexity.as_array().is_some_and(|shapes| {
        shapes
            .iter()
            .any(|shape| shape.get("const") == Some(&Value::Bool(false)))
    }));
    let complexity_object = &complexity[1];
    assert_eq!(
        complexity_object["additionalProperties"],
        Value::Bool(false)
    );
    assert_eq!(
        complexity_object["properties"]["in"]["anyOf"][0]["type"],
        "string"
    );
    assert_eq!(
        complexity_object["properties"]["in"]["anyOf"][1]["type"],
        "array"
    );
    assert_eq!(
        complexity_object["properties"]["cc"]["anyOf"][0]["type"],
        "integer"
    );
    assert_eq!(
        complexity_object["properties"]["cc"]["anyOf"][1]["type"],
        "object"
    );
}

fn document_map_schema(properties: &serde_json::Map<String, Value>) {
    let document_map = &properties["doc_size"]["anyOf"][1];
    assert_eq!(document_map["minProperties"], 1);
    assert_eq!(
        document_map["additionalProperties"]["anyOf"][0]["type"],
        "integer"
    );
}

fn convention_schema(properties: &serde_json::Map<String, Value>) {
    let convention = &properties["conventions"]["anyOf"][1]["additionalProperties"];
    assert_eq!(convention["additionalProperties"], Value::Bool(false));
    assert_eq!(convention["required"][0], "remedy");
    assert!(
        convention["oneOf"]
            .as_array()
            .is_some_and(|one_of| !one_of.is_empty())
    );
    assert_eq!(convention["properties"]["language"]["enum"][0], "rust");
}

fn layering_schema(properties: &serde_json::Map<String, Value>) {
    let layer = &properties["layering"]["anyOf"][1];
    assert_eq!(layer["required"][0], "layers");
    assert_eq!(
        layer["properties"]["layers"]["additionalProperties"]["required"][0],
        "in"
    );
    assert_eq!(
        layer["properties"]["layers"]["additionalProperties"]["properties"]["can_use"]["anyOf"][0]
            ["type"],
        "null"
    );
}

fn sarif_schema(properties: &serde_json::Map<String, Value>) {
    let sarif = &properties["sarif"]["anyOf"][1]["items"];
    assert_eq!(sarif["required"][0], "name");
    assert_eq!(sarif["required"][1], "report");
    assert_eq!(sarif["additionalProperties"], Value::Bool(false));
}

fn accepted_schema(properties: &serde_json::Map<String, Value>) {
    let entry = &properties["accepted"]["items"];
    assert_eq!(entry["properties"]["reason"]["type"], "string");
    assert_eq!(entry["additionalProperties"]["type"], "number");
    assert!(
        entry["required"]
            .as_array()
            .is_some_and(|required| !required.contains(&Value::from("reason")))
    );
}

#[test]
fn the_schema_descriptions_carry_the_policy_metadata() {
    let schema = schema();
    let described = |at: &Value| {
        at["description"]
            .as_str()
            .unwrap_or_else(|| panic!("a description: {at}"))
            .to_string()
    };
    let root = described(&schema);
    assert!(root.contains("at the worktree root"), "{root}");
    assert!(root.contains("run under `{}`"), "{root}");
    assert!(root.contains("the hooks stay silent"), "{root}");

    let properties = &schema["properties"];
    let complexity = described(&properties["complexity"]);
    assert!(
        complexity
            .contains("automatic: when the section is absent, the check runs when the tree holds"),
        "{complexity}"
    );
    assert!(
        complexity.contains("`false` excludes the gate"),
        "{complexity}"
    );
    let escapes = described(&properties["escapes"]);
    assert!(!escapes.contains("derives"), "{escapes}");
    assert!(
        escapes.contains("every key follows its own derivation rule or default"),
        "{escapes}"
    );
    let layering = described(&properties["layering"]);
    assert!(layering.contains("the check does not run"), "{layering}");
    let sarif = described(&properties["sarif"]);
    assert!(sarif.contains("no external tool"), "{sarif}");
    let public_api = described(&properties["public_api"]);
    assert!(
        public_api.contains("public surfaces are derived from Cargo library targets"),
        "{public_api}"
    );
    assert!(public_api.contains("ADR 0050"), "{public_api}");

    let complexity = &properties["complexity"]["anyOf"][1]["properties"];
    let cc = described(&complexity["cc"]);
    assert!(cc.starts_with("the cyclomatic complexity"), "{cc}");
    assert!(
        cc.contains("Source: derived when absent. Derivation rule: the 95th percentile"),
        "{cc}"
    );
    let test_lines = described(&complexity["test_lines"]);
    assert!(test_lines.contains("Source: pinned only."), "{test_lines}");
    assert!(
        test_lines.contains("Default: test code is not judged on length."),
        "{test_lines}"
    );
}

#[test]
fn the_readme_accepted_example_fits_the_schema_and_the_binary() {
    let readme =
        fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
            .unwrap_or_else(|why| panic!("README.md could not be read: {why}"));
    let example = readme
        .split("Policy lives in `klin.json`:\n\n```json\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .unwrap_or_else(|| panic!("README.md has no klin.json example"));
    let config: Value = serde_json::from_str(example)
        .unwrap_or_else(|why| panic!("the README example is JSON: {why}"));
    let schema = schema();
    let item = &schema["properties"]["accepted"]["items"];
    let entries = config["accepted"].as_array().map_or(&[][..], Vec::as_slice);
    assert!(!entries.is_empty(), "{config}");
    for entry in entries {
        fits(entry, item);
    }

    let tree = Tree::new();
    tree.write("klin.json", example);
    let run = tree.run(&["policy"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

fn fits(entry: &Value, item: &Value) {
    let fields = entry.as_object().unwrap_or_else(|| panic!("{entry}"));
    for name in item["required"].as_array().into_iter().flatten() {
        let name = name.as_str().unwrap_or_default();
        assert!(fields.contains_key(name), "{entry} has no {name}");
    }
    for (name, value) in fields {
        let shape = item["properties"]
            .get(name)
            .unwrap_or(&item["additionalProperties"]);
        let fit = match shape["type"].as_str() {
            Some("string") => value.is_string(),
            Some("integer") => value.is_u64(),
            Some("number") => value.is_number(),
            _ => false,
        };
        assert!(fit, "{name} of {entry} does not fit {shape}");
    }
}

/// One `klin.json` per row, and whether klin accepts it. Each special shape the schema generates
/// on its own has a row that fits it and a row that breaks it.
const AGREED: &[(&str, bool)] = &[
    (r#"{}"#, true),
    (r#"{"version": 1}"#, false),
    (r#"{"nope": 1}"#, false),
    (
        r#"{"accepted": [{"gate": "complexity", "file": "a.rs", "text": "f", "cc": 12, "line": 3, "reason": "r"}]}"#,
        true,
    ),
    (
        r#"{"accepted": [{"gate": "complexity", "file": "a.rs", "cc": 12}]}"#,
        false,
    ),
    (
        r#"{"accepted": [{"gate": "complexity", "file": "a.rs", "text": "f", "line": -1}]}"#,
        false,
    ),
    (r#"{"accepted": {}}"#, false),
    (r#"{"accepted": [1]}"#, false),
    (r#"{"build": "make"}"#, true),
    (r#"{"build": [{"run": "make", "root": "a"}]}"#, true),
    (r#"{"build": false}"#, true),
    (r#"{"build": true}"#, false),
    (r#"{"build": [{"root": "a"}]}"#, false),
    (r#"{"build": [{"run": "make", "cwd": "a"}]}"#, false),
    (r#"{"radius": {"lines": 10, "directories": 2}}"#, true),
    (r#"{"radius": {"lines": -1}}"#, false),
    (r#"{"radius": {"lines": {"2020-01-01": 5}}}"#, false),
    (r#"{"radius": {"files": 1}}"#, false),
    (r#"{"journal": {"prompt": false}}"#, true),
    (r#"{"journal": {"prompt": "no"}}"#, false),
    (
        r#"{"layering": {"layers": {"core": {"in": "src", "can_use": null}, "app": {"in": ["app"], "can_use": ["core"]}}}}"#,
        true,
    ),
    (r#"{"layering": {"layers": {}}}"#, false),
    (
        r#"{"layering": {"layers": {"core": {"can_use": []}}}}"#,
        false,
    ),
    (
        r#"{"layering": {"layers": {"core": {"in": "src", "can_use": [1]}}}}"#,
        false,
    ),
    (r#"{"layering": {}}"#, false),
    (
        r#"{"conventions": {"a": {"text": "x", "remedy": "Do.", "in": "src", "except": ["src/b"]}}}"#,
        true,
    ),
    (
        r#"{"conventions": {"a": {"code": "f()", "language": "rust", "remedy": "Do."}}}"#,
        true,
    ),
    (
        r#"{"conventions": {"a": {"files": "**/scratch.*", "remedy": "Do."}}}"#,
        true,
    ),
    (
        r#"{"conventions": {"a": {"text": "x", "remedy": "\ufeff"}}}"#,
        true,
    ),
    (r#"{"conventions": {"a": {"text": "x"}}}"#, false),
    (
        r#"{"conventions": {"a": {"text": "x", "remedy": ""}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": "x", "remedy": " \t\n"}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": "x", "remedy": "\u00a0\u0085\u3000"}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": "x", "code": "y", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": "x", "language": "rust", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"code": "f()", "language": "cobol", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": "x", "in": "", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"": {"text": "x", "remedy": "Do."}}}"#,
        false,
    ),
    (r#"{"conventions": {}}"#, false),
    (r#"{"conventions": []}"#, false),
    (r#"{"doc_size": {"README.md": 1200}}"#, true),
    (r#"{"doc_size": {"README.md": {"2020-01-01": 1200}}}"#, true),
    (r#"{"doc_size": {"  ": 1200}}"#, true),
    (r#"{"doc_size": {"": 1200}}"#, false),
    (r#"{"doc_size": {"README.md": "x"}}"#, false),
    (r#"{"doc_size": {}}"#, false),
    (r#"{"doc_size": []}"#, false),
    (
        r#"{"sarif": [{"name": "lint", "report": "lint.sarif"}]}"#,
        true,
    ),
    (r#"{"sarif": []}"#, true),
    (r#"{"sarif": false}"#, true),
    (r#"{"sarif": [{"name": "lint"}]}"#, false),
    (r#"{"sarif": {}}"#, false),
    (r#"{"doc_citations": false}"#, true),
    (r#"{"doc_citations": true}"#, false),
    (r#"{"doc_citations": {}}"#, false),
    (r#"{"complexity": {"cc": 12, "in": "src"}}"#, true),
    (r#"{"complexity": false}"#, true),
    (r#"{"complexity": {}}"#, false),
    (r#"{"complexity": true}"#, false),
    (r#"{"complexity": []}"#, false),
    (r#"{"complexity": {"in": []}}"#, false),
    (r#"{"complexity": {"cc": -1}}"#, false),
    (r#"{"complexity": {"cc": {"2020-01-31": 12}}}"#, true),
    (r#"{"complexity": {"cc": {"2020-02-31": 12}}}"#, true),
    (r#"{"complexity": {"cc": {"2020-12-01": 12}}}"#, true),
    (r#"{"complexity": {"cc": {"2020-13-01": 12}}}"#, false),
    (r#"{"complexity": {"cc": {"2020-00-01": 12}}}"#, false),
    (r#"{"complexity": {"cc": {"2020-01-00": 12}}}"#, false),
    (r#"{"complexity": {"cc": {"2020-01-32": 12}}}"#, false),
    (r#"{"complexity": {"cc": {"2020-1-01": 12}}}"#, false),
    (
        r#"{"complexity": {"cc": {"2020-01-01": 12, "soon": 10}}}"#,
        false,
    ),
    (r#"{"complexity": {"cc": {}}}"#, false),
];

/// Rows where klin and the schema disagree, with whether klin accepts the row; the schema answers
/// the other way. Spec B.5.8 lists each one, so a row that comes to agree is taken out of both
/// places.
const KNOWN: &[(&str, bool)] = &[
    (
        r#"{"conventions": {" ": {"text": "x", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": " ", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": "x\ny", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"accepted": [{"gate": "complexity", "file": "a.rs", "text": "f", "cc": 12, "note": "x"}]}"#,
        true,
    ),
    (r#"{"complexity": {"cc": 5.0}}"#, false),
    (
        r#"{"conventions": {"a": {"text": "", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"files": "[", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"sarif": [{"name": "lint", "report": "a.sarif"}, {"name": "lint", "report": "b.sarif"}]}"#,
        false,
    ),
    (
        r#"{"sarif": [{"name": "measurement-lost", "report": "a.sarif"}]}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"files": "/tmp/scratch", "remedy": "Do."}}}"#,
        false,
    ),
    (
        r#"{"conventions": {"a": {"text": "x", "remedy": "Do."}}, "accepted": [{"gate": "conventions/b", "file": "src/a.rs", "text": "x"}]}"#,
        false,
    ),
];

#[test]
fn the_schema_accepts_a_configuration_exactly_when_klin_does() {
    let schema = schema();
    let tree = Tree::new();
    tree.write("src/a.rs", "fn a() {}\n");
    for (config, accepted) in AGREED {
        assert_eq!(klin_accepts(&tree, config), *accepted, "klin on {config}");
        assert_eq!(
            schema_accepts(&schema, config),
            *accepted,
            "schema on {config}"
        );
    }
}

#[test]
fn every_known_disagreement_still_disagrees() {
    let schema = schema();
    let tree = Tree::new();
    for (config, accepted) in KNOWN {
        assert_eq!(klin_accepts(&tree, config), *accepted, "klin on {config}");
        assert_eq!(
            schema_accepts(&schema, config),
            !accepted,
            "schema on {config}"
        );
    }
}

#[test]
fn the_validator_knows_every_keyword_the_schema_uses() {
    vocabulary(&schema());
}

#[test]
fn the_remedy_pattern_refuses_exactly_the_characters_klin_trims() {
    let schema = schema();
    let remedy = &schema["properties"]["conventions"]["anyOf"][1]["additionalProperties"]["properties"]
        ["remedy"];
    let pattern = regex::Regex::new(remedy["pattern"].as_str().unwrap_or_default())
        .unwrap_or_else(|why| panic!("{remedy} has no pattern: {why}"));
    for character in char::MIN..=char::MAX {
        assert_eq!(
            pattern.is_match(&character.to_string()),
            !character.is_whitespace(),
            "U+{:04X}",
            u32::from(character)
        );
    }
}

fn klin_accepts(tree: &Tree, config: &str) -> bool {
    tree.write("klin.json", config);
    let run = tree.run(&["policy"]);
    assert!(matches!(run.code, 0 | 2), "{config}: {}", run.out);
    run.code == 0
}

fn schema_accepts(schema: &Value, config: &str) -> bool {
    let config: Value =
        serde_json::from_str(config).unwrap_or_else(|why| panic!("{config} is JSON: {why}"));
    valid(schema, &config)
}

/// The draft 2020-12 keywords the generated schema uses, evaluated as a validator does. A keyword
/// this does not know fails the test rather than passing in silence.
fn valid(schema: &Value, value: &Value) -> bool {
    match schema {
        Value::Bool(verdict) => *verdict,
        Value::Object(rules) => rules
            .iter()
            .all(|(keyword, rule)| holds(rules, keyword, rule, value)),
        _ => panic!("not a schema: {schema}"),
    }
}

type Rules = serde_json::Map<String, Value>;

type Keyword = fn(&Rules, &Value, &Value) -> bool;

/// Where a keyword's value holds subschemas, so the vocabulary walk reaches every node.
enum Holds {
    Nothing,
    Schema,
    Schemas,
    Named,
}

const KEYWORDS: &[(&str, Holds, Keyword)] = &[
    ("$schema", Nothing, |_, _, _| true),
    ("$id", Nothing, |_, _, _| true),
    ("title", Nothing, |_, _, _| true),
    ("description", Nothing, |_, _, _| true),
    ("type", Nothing, |_, rule, value| typed(rule, value)),
    ("const", Nothing, |_, rule, value| value == rule),
    ("enum", Nothing, |_, rule, value| {
        listed(rule).contains(value)
    }),
    ("anyOf", Schemas, |_, rule, value| {
        listed(rule).iter().any(|shape| valid(shape, value))
    }),
    ("oneOf", Schemas, |_, rule, value| {
        listed(rule)
            .iter()
            .filter(|shape| valid(shape, value))
            .count()
            == 1
    }),
    ("not", Schema, |_, rule, value| !valid(rule, value)),
    ("minimum", Nothing, |_, rule, value| {
        value
            .as_f64()
            .is_none_or(|number| number >= count(rule) as f64)
    }),
    ("minLength", Nothing, |_, rule, value| {
        value
            .as_str()
            .is_none_or(|text| text.chars().count() as u64 >= count(rule))
    }),
    ("pattern", Nothing, |_, rule, value| {
        value.as_str().is_none_or(|text| matches(rule, text))
    }),
    ("minItems", Nothing, |_, rule, value| {
        value
            .as_array()
            .is_none_or(|items| items.len() as u64 >= count(rule))
    }),
    ("items", Schema, |_, rule, value| {
        value
            .as_array()
            .is_none_or(|items| items.iter().all(|item| valid(rule, item)))
    }),
    ("minProperties", Nothing, |_, rule, value| {
        value
            .as_object()
            .is_none_or(|fields| fields.len() as u64 >= count(rule))
    }),
    ("required", Nothing, required),
    ("propertyNames", Schema, property_names),
    ("properties", Named, properties),
    ("patternProperties", Named, pattern_properties),
    ("additionalProperties", Schema, additional_properties),
];

fn holds(rules: &Rules, keyword: &str, rule: &Value, value: &Value) -> bool {
    let (_, _, check) = keyword_named(keyword);
    check(rules, rule, value)
}

fn keyword_named(keyword: &str) -> &'static (&'static str, Holds, Keyword) {
    KEYWORDS
        .iter()
        .find(|(name, _, _)| *name == keyword)
        .unwrap_or_else(|| panic!("the test validator does not know \"{keyword}\""))
}

/// Every keyword of every node the schema holds, whether or not a fixture reaches the node.
fn vocabulary(schema: &Value) {
    let Value::Object(rules) = schema else {
        return;
    };
    for (keyword, rule) in rules {
        let (_, holds, _) = keyword_named(keyword);
        match holds {
            Nothing => {}
            Schema => vocabulary(rule),
            Schemas => listed(rule).iter().for_each(vocabulary),
            Named => rule
                .as_object()
                .unwrap_or_else(|| panic!("not a map of schemas: {rule}"))
                .values()
                .for_each(vocabulary),
        }
    }
}

fn required(_: &Rules, rule: &Value, value: &Value) -> bool {
    value.as_object().is_none_or(|fields| {
        listed(rule)
            .iter()
            .all(|name| name.as_str().is_some_and(|name| fields.contains_key(name)))
    })
}

fn property_names(_: &Rules, rule: &Value, value: &Value) -> bool {
    value.as_object().is_none_or(|fields| {
        fields
            .keys()
            .all(|name| valid(rule, &Value::from(name.as_str())))
    })
}

fn properties(_: &Rules, rule: &Value, value: &Value) -> bool {
    value.as_object().is_none_or(|fields| {
        fields
            .iter()
            .all(|(name, field)| rule.get(name).is_none_or(|shape| valid(shape, field)))
    })
}

fn pattern_properties(_: &Rules, rule: &Value, value: &Value) -> bool {
    let patterns = rule.as_object().into_iter().flatten();
    value.as_object().is_none_or(|fields| {
        fields.iter().all(|(name, field)| {
            patterns
                .clone()
                .filter(|(pattern, _)| matches(&Value::from(pattern.as_str()), name))
                .all(|(_, shape)| valid(shape, field))
        })
    })
}

fn additional_properties(rules: &Rules, rule: &Value, value: &Value) -> bool {
    value.as_object().is_none_or(|fields| {
        fields
            .iter()
            .filter(|(name, _)| !covered(rules, name))
            .all(|(_, field)| valid(rule, field))
    })
}

fn covered(rules: &Rules, name: &str) -> bool {
    let named = rules
        .get("properties")
        .is_some_and(|properties| properties.get(name).is_some());
    let patterned = rules
        .get("patternProperties")
        .and_then(Value::as_object)
        .is_some_and(|patterns| {
            patterns
                .keys()
                .any(|pattern| matches(&Value::from(pattern.as_str()), name))
        });
    named || patterned
}

fn typed(rule: &Value, value: &Value) -> bool {
    match rule.as_str() {
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        Some("string") => value.is_string(),
        Some("boolean") => value.is_boolean(),
        Some("null") => value.is_null(),
        Some("number") => value.is_number(),
        Some("integer") => value.as_f64().is_some_and(|number| number.fract() == 0.0),
        _ => panic!("the test validator does not know type {rule}"),
    }
}

fn listed(rule: &Value) -> &[Value] {
    rule.as_array()
        .unwrap_or_else(|| panic!("not a list: {rule}"))
}

fn count(rule: &Value) -> u64 {
    rule.as_u64()
        .unwrap_or_else(|| panic!("not a whole number: {rule}"))
}

fn matches(pattern: &Value, text: &str) -> bool {
    let pattern = pattern
        .as_str()
        .unwrap_or_else(|| panic!("not a pattern: {pattern}"));
    regex::Regex::new(pattern)
        .unwrap_or_else(|why| panic!("{pattern} is not a pattern: {why}"))
        .is_match(text)
}

#[test]
fn the_committed_schema_is_what_the_binary_generates() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/klin.json");
    let committed = fs::read_to_string(path)
        .unwrap_or_else(|why| panic!("committed schema could not be read: {why}"));
    let run = Tree::bare().run(&["policy", "--schema"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        committed, run.printed,
        "schemas/klin.json is stale — run `klin policy --schema > schemas/klin.json`"
    );
}
