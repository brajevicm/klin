mod harness;

use std::fs;

use harness::Tree;
use serde_json::Value;

fn schema() -> Value {
    let run = Tree::bare().run(&["reference", "--schema"]);
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

#[test]
fn the_committed_schema_is_what_the_binary_generates() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/klin.json");
    let committed = fs::read_to_string(path)
        .unwrap_or_else(|why| panic!("committed schema could not be read: {why}"));
    let run = Tree::bare().run(&["reference", "--schema"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        committed, run.printed,
        "schemas/klin.json is stale — run `klin reference --schema > schemas/klin.json`"
    );
}
