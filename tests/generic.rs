use std::path::Path;

use crate::harness::{Run, Tree, feed};
use serde_json::{Value, json};

const A_CONFIG: &str = r#"{
  "doc_size": {"README.md": 10}
}"#;

/// One generic event of the version klin speaks, over the fields a harness proves.
fn event(kind: &str, root: &Path, held: Value) -> String {
    let mut payload = json!({
        "klin_protocol": 1,
        "event": kind,
        "root": root,
        "session": "s1"
    });
    let (Some(payload_fields), Some(held)) = (payload.as_object_mut(), held.as_object()) else {
        panic!("a generic event is a JSON object");
    };
    payload_fields.extend(held.iter().map(|(key, held)| (key.clone(), held.clone())));
    payload.to_string()
}

fn failing() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", A_CONFIG);
    tree.words("README.md", 5);
    tree.base();
    tree.words("README.md", 30);
    tree
}

fn guard(tree: &Tree, held: Value) -> Run {
    let event = event("pre_tool", tree.root(), held);
    feed(tree.root(), &["guard"], &event)
}

/// The decision a run printed on stdout, which is the whole answer a shim reads.
fn answer(run: &Run) -> Value {
    match run.printed.lines().find_map(parsed) {
        Some(answer) => answer,
        None => panic!("no generic decision was printed:\n{}", run.out),
    }
}

fn parsed(line: &str) -> Option<Value> {
    let held: Value = serde_json::from_str(line).ok()?;
    held.get("action").is_some().then_some(held)
}

#[test]
fn a_generic_session_and_prompt_open_a_turn() {
    let tree = failing();
    for kind in ["session", "prompt"] {
        let opened = feed(
            tree.root(),
            &["radius"],
            &event(kind, tree.root(), json!({})),
        );
        assert_eq!(opened.code, 0, "{kind}: {}", opened.out);
        assert!(!opened.says("reading it as"), "{kind}: {}", opened.out);
    }
}

#[test]
fn a_generic_pre_tool_denies_a_proven_configuration_write() {
    let tree = failing();
    let run = guard(
        &tree,
        json!({"tool": "write_file", "file_paths": ["klin.json"]}),
    );

    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(answer(&run)["action"], "deny", "{}", run.out);
    assert!(
        answer(&run)["reason"].as_str().is_some_and(refuses),
        "{}",
        run.out
    );
}

fn refuses(reason: &str) -> bool {
    reason.contains("refused") && reason.contains("klin.json")
}

#[test]
fn a_generic_pre_tool_passes_an_unrelated_tool() {
    let tree = failing();
    let run = guard(
        &tree,
        json!({"tool": "write_file", "file_paths": ["src/main.rs"]}),
    );

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(answer(&run)["action"], "allow", "{}", run.out);
}

/// A tool name is diagnostic. A call whose paths and command the harness cannot prove carries
/// neither, and klin reads no write out of the name or out of opaque arguments.
#[test]
fn a_generic_pre_tool_reads_no_write_out_of_a_tool_name() {
    let tree = failing();
    let run = guard(
        &tree,
        json!({"tool": "write_klin_json", "arguments": {"path": "klin.json"}}),
    );

    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(answer(&run)["action"], "allow", "{}", run.out);
}

/// klin's `ask` has no channel a custom harness proves, so the ambiguous class fails closed.
#[test]
fn a_generic_ambiguous_command_fails_closed_instead_of_asking() {
    let tree = failing();
    let run = guard(&tree, json!({"tool": "shell", "command": "rm klin.json"}));

    assert_eq!(run.code, 2, "{}", run.out);
    assert_eq!(answer(&run)["action"], "deny", "{}", run.out);
    assert!(run.says("fails closed"), "{}", run.out);
}

#[test]
fn a_generic_stop_returns_the_failing_report_in_its_decision() {
    let tree = failing();
    let run = feed(
        tree.root(),
        &["gate", "--hook", "--changed"],
        &event("stop", tree.root(), json!({"blocked_before": false})),
    );

    assert_eq!(run.code, 2, "{}", run.out);
    let answer = answer(&run);
    assert_eq!(answer["action"], "block", "{}", run.out);
    let message = answer["message"].as_str().unwrap_or_default();
    assert!(message.contains("README.md"), "{}", run.out);
}

#[test]
fn a_generic_stop_over_a_green_tree_lets_the_turn_end() {
    let tree = Tree::new();
    tree.write("klin.json", A_CONFIG);
    tree.words("README.md", 5);
    tree.base();
    let run = feed(
        tree.root(),
        &["gate", "--hook", "--changed"],
        &event("stop", tree.root(), json!({})),
    );

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.printed.contains("\"action\""), "{}", run.out);
}

/// A version klin does not speak is refused whole. It is never read as another host's event,
/// because klin does not know what the fields of that version mean.
#[test]
fn an_unknown_protocol_version_refuses_and_is_not_read_as_another_host() {
    let tree = failing();
    let unknown = json!({
        "klin_protocol": 2,
        "event": "pre_tool",
        "root": tree.root(),
        "tool": "write_file",
        "file_paths": ["src/main.rs"]
    });

    let run = feed(tree.root(), &["guard"], &unknown.to_string());

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(!run.says("reading it as"), "{}", run.out);
    assert!(run.says("version 1"), "{}", run.out);
    assert_eq!(answer(&run)["action"], "deny", "{}", run.out);
}

/// A host that refuses a call the guard itself allowed still leaves a journal line. It is the
/// only record a person has of why every tool call of that session was blocked.
#[test]
fn an_unknown_protocol_version_records_its_refusal_in_the_journal() {
    let tree = failing();
    let unknown = json!({
        "klin_protocol": 2,
        "event": "pre_tool",
        "root": tree.root(),
        "tool": "write_file"
    });

    let run = feed(tree.root(), &["guard"], &unknown.to_string());
    assert_eq!(run.code, 2, "{}", run.out);

    let lines = journal(&tree);
    let guards: Vec<&Value> = lines
        .iter()
        .filter(|line| line["kind"] == "guard")
        .collect();
    assert_eq!(guards.len(), 1, "{lines:?}");
    assert_eq!(guards[0]["decision"], "deny", "{lines:?}");
    assert_eq!(guards[0]["reason"], "host-refusal", "{lines:?}");
}

/// Every line klin appended to this tree's journal, spec 11.4's record.
fn journal(tree: &Tree) -> Vec<Value> {
    let text = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[test]
fn a_named_harness_host_without_a_version_is_refused() {
    let tree = failing();
    let payload = json!({"event": "pre_tool", "file_paths": ["src/main.rs"]});

    let run = feed(
        tree.root(),
        &["guard", "--host", "harness"],
        &payload.to_string(),
    );

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no version"), "{}", run.out);
    assert!(run.says("harness protocol"), "{}", run.out);
}

/// The protocol's name is `harness`. `generic` was its name before 1.0 and names no host now.
#[test]
fn the_generic_host_name_names_no_host() {
    let tree = failing();
    let event = event("pre_tool", tree.root(), json!({"tool": "write_file"}));

    let run = feed(tree.root(), &["guard", "--host", "generic"], &event);

    assert!(
        run.says("--host generic names no host klin knows"),
        "{}",
        run.out
    );
}

#[test]
fn the_journal_names_a_protocol_event_the_harness_host() {
    let tree = failing();
    let run = feed(
        tree.root(),
        &["gate", "--hook", "--changed"],
        &event("stop", tree.root(), json!({})),
    );
    assert_eq!(run.code, 2, "{}", run.out);

    let lines = journal(&tree);
    assert!(
        lines
            .iter()
            .any(|line| line["kind"] == "stop" && line["host"] == "harness"),
        "{lines:?}"
    );
}

/// The shipped fixtures are the contract a port reads first, so every one of them must place as
/// a generic event rather than fall back to a host's shape.
#[test]
fn every_shipped_fixture_places_as_a_generic_event() {
    let tree = failing();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("harness-protocol/fixtures");
    let Ok(entries) = std::fs::read_dir(&fixtures) else {
        panic!(
            "the harness protocol ships no fixtures at {}",
            fixtures.display()
        );
    };
    let mut read = 0;
    for entry in entries.flatten() {
        let at = entry.path();
        let Ok(text) = std::fs::read_to_string(&at) else {
            panic!("{} could not be read", at.display());
        };
        let Ok(held) = serde_json::from_str::<Value>(&text) else {
            panic!("{} is not JSON", at.display());
        };
        assert_eq!(held["klin_protocol"], 1, "{}", at.display());
        let run = feed(tree.root(), &["guard"], &text);
        assert!(!run.says("reading it as"), "{}", run.out);
        read += 1;
    }
    assert!(
        read >= 5,
        "the harness protocol ships fewer fixtures than its README names"
    );
}
