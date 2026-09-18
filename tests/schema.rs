mod harness;

use std::fs;

use harness::Tree;
use serde_json::Value;

fn schema() -> Value {
    let run = Tree::bare().run(&["reference", "--schema"]);
    assert_eq!(run.code, 0, "{}", run.out);
    serde_json::from_str(&run.printed).expect("reference --schema prints JSON")
}

#[test]
fn the_schema_describes_the_compact_configuration_shapes() {
    let schema = schema();
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .expect("top-level properties");

    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    assert_eq!(
        schema.get("additionalProperties"),
        Some(&Value::Bool(false))
    );
    assert!(properties.contains_key("complexity"));
    assert!(properties.contains_key("doc_size"));
    assert!(properties.contains_key("conventions"));
    assert!(properties.contains_key("layering"));
    assert!(properties.contains_key("sarif"));
    assert!(!properties.contains_key("version"));
    assert!(schema.get("required").is_none());

    let complexity = &properties["complexity"]["anyOf"];
    assert!(
        complexity
            .as_array()
            .unwrap()
            .iter()
            .any(|shape| shape.get("const") == Some(&Value::Bool(false)))
    );
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

    let document_map = &properties["doc_size"]["anyOf"][1];
    assert_eq!(document_map["minProperties"], 1);
    assert_eq!(
        document_map["additionalProperties"]["anyOf"][0]["type"],
        "integer"
    );

    let convention = &properties["conventions"]["anyOf"][1]["additionalProperties"];
    assert_eq!(convention["additionalProperties"], Value::Bool(false));
    assert_eq!(convention["required"][0], "remedy");
    assert!(!convention["oneOf"].as_array().unwrap().is_empty());
    assert_eq!(convention["properties"]["language"]["enum"][0], "rust");

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

    let sarif = &properties["sarif"]["anyOf"][1]["items"];
    assert_eq!(sarif["required"][0], "name");
    assert_eq!(sarif["required"][1], "report");
    assert_eq!(sarif["additionalProperties"], Value::Bool(false));
}

#[test]
fn the_committed_schema_is_what_the_binary_generates() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/klin.json");
    let committed = fs::read_to_string(path).expect("committed schema");
    let run = Tree::bare().run(&["reference", "--schema"]);

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(
        committed, run.printed,
        "schemas/klin.json is stale — run `klin reference --schema > schemas/klin.json`"
    );
}
