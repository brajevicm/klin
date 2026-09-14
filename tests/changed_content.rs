mod harness;

use harness::{Run, Tree};
use serde_json::Value;

const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;
const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const GATES: [&str; 3] = ["complexity", "escapes", "stubs"];

fn report(run: &Run) -> Value {
    run.out
        .lines()
        .find_map(|line| serde_json::from_str(line).ok())
        .unwrap_or_else(|| panic!("no JSON report in:\n{}", run.out))
}

#[test]
fn changed_file_local_gates_skip_unchanged_hook_contents() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"escapes":{"in":"src"}}"#);
    tree.write(
        "src/unchanged.rs",
        "fn untouched() { let _ = Some(1).unwrap(); }\n",
    );
    tree.write("src/changed.rs", "fn changed() {}\n");
    tree.base();
    let radius = harness::feed(tree.root(), &["radius"], A_PROMPT);
    assert_eq!(radius.code, 0, "{}", radius.out);
    assert!(!tree.field("commit").is_empty(), "{}", radius.out);

    tree.write(
        "src/changed.rs",
        "fn changed() { let _ = Some(1).unwrap(); }\n",
    );
    let run = harness::feed(
        tree.root(),
        &["gate", "--json", "--hook", "--changed"],
        A_STOP,
    );
    assert_eq!(run.code, 2, "{}", run.out);
    let report = report(&run);
    assert_eq!(report["window"]["kind"], "turn", "{report}");
    let row = &report["gates"][0];
    assert_eq!(row["work"]["reads"], Value::from(2), "{report}");
    assert_eq!(row["work"]["parses"], Value::from(2), "{report}");
    assert!(
        !report["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|finding| finding["file"] == "src/unchanged.rs"),
        "{report}"
    );
}

#[test]
fn changed_file_local_gates_read_only_changed_current_contents() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{
          "complexity": { "in": "src", "cc": 1, "lines": 60 },
          "escapes": { "in": "src" },
          "stubs": { "in": "src" }
        }"#,
    );
    tree.write(
        "src/unchanged.rs",
        "fn untouched() {\n    let _ = Some(1).unwrap();\n    todo!();\n}\n",
    );
    tree.write(
        "src/changed.rs",
        "fn changed(value: i32) { drop(value); }\n",
    );
    tree.base();
    tree.write(
        "src/changed.rs",
        "fn changed(value: Option<i32>) {\n    if value.is_some() {\n        let _ = value.unwrap();\n    }\n    todo!();\n}\n",
    );
    for gate in GATES {
        let run = tree.run(&["gate", "--json", "--changed", "--gate", gate]);
        assert_eq!(run.code, 1, "{gate}: {}", run.out);
        let report = run.json();
        let row = &report["gates"][0];
        assert_eq!(row["work"]["reads"], Value::from(2), "{gate}: {report}");
        assert_eq!(row["work"]["parses"], Value::from(2), "{gate}: {report}");
        assert!(
            !report["findings"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|finding| finding["file"] == "src/unchanged.rs"),
            "{gate}: {report}"
        );
    }

    tree.write("src/changed.rs", "fn changed() {}\n");
    for gate in GATES {
        let run = tree.run(&["gate", "--json", "--changed", "--gate", gate]);
        assert_eq!(run.code, 0, "{gate}: {}", run.out);
        let row = &run.json()["gates"][0];
        assert_eq!(
            row["coverage"]["found"],
            Value::from(1),
            "{gate}: {}",
            run.out
        );
        assert_eq!(
            row["coverage"]["measured"],
            Value::from(1),
            "{gate}: {}",
            run.out
        );
        let work = &row["work"];
        assert_eq!(work["reads"], Value::from(2), "{gate}: {}", run.out);
        assert_eq!(work["parses"], Value::from(2), "{gate}: {}", run.out);

        for flags in [
            vec!["gate", "--json", "--gate", gate],
            vec!["gate", "--json", "--strict", "--gate", gate],
        ] {
            let run = tree.run(&flags);
            assert_eq!(run.code, 0, "{gate}: {}", run.out);
            let row = &run.json()["gates"][0];
            assert_eq!(
                row["coverage"]["found"],
                Value::from(2),
                "{gate}: {}",
                run.out
            );
            assert_eq!(
                row["coverage"]["measured"],
                Value::from(2),
                "{gate}: {}",
                run.out
            );
            assert_eq!(row["work"]["reads"], Value::from(4), "{gate}: {}", run.out);
            assert_eq!(row["work"]["parses"], Value::from(4), "{gate}: {}", run.out);
        }

        let run = tree.run(&["gate", "--json", "--changed", "--strict", "--gate", gate]);
        assert_eq!(run.code, 0, "{gate}: {}", run.out);
        let row = &run.json()["gates"][0];
        assert_eq!(
            row["coverage"]["found"],
            Value::from(1),
            "{gate}: {}",
            run.out
        );
        assert_eq!(row["work"]["reads"], Value::from(3), "{gate}: {}", run.out);
        assert_eq!(row["work"]["parses"], Value::from(3), "{gate}: {}", run.out);
    }
}
