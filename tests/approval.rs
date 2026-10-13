mod harness;

use harness::{Run, Tree, feed};
use serde_json::{Value, json};

const CONFIG: &str = "{\n  \"escapes\": true,\n  \"complexity\": { \"cc\": 8, \"lines\": 8 }\n}\n";

fn tree() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", CONFIG);
    tree
}

/// A Claude Code call of `tool` with `input`, under one session and its own tool call.
fn claude(tree: &Tree, tool: &str, input: Value, call: &str) -> Run {
    let event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "s-1",
        "tool_use_id": call,
        "tool_name": tool,
        "tool_input": input,
    });
    feed(tree.root(), harness::AGENT, &event.to_string())
}

fn write(tree: &Tree, content: &str, call: &str) -> Run {
    let input = json!({ "file_path": tree.at("klin.json"), "content": content });
    claude(tree, "Write", input, call)
}

fn prompt(tree: &Tree, text: &str, id: &str) -> Run {
    let event = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "s-1",
        "prompt_id": id,
        "prompt": text,
    });
    feed(tree.root(), harness::AGENT, &event.to_string())
}

/// The decision a host reads from stdout, or `Null` where the run printed none.
fn answer(run: &Run) -> Value {
    serde_json::from_str(run.printed.trim()).unwrap_or(Value::Null)
}

/// The code the person was told, from the one channel only the person reads.
#[track_caller]
fn code(run: &Run) -> String {
    let answer = answer(run);
    let told = answer["systemMessage"].as_str().unwrap_or_default();
    match told.rsplit_once("approve ") {
        Some((_, code)) => code.trim().to_string(),
        None => panic!("no code in {}", run.out),
    }
}

#[track_caller]
fn referred(run: &Run) -> String {
    assert_eq!(run.code, 0, "{}", run.out);
    let answer = answer(run);
    let decided = &answer["hookSpecificOutput"];
    assert_eq!(decided["permissionDecision"], "deny", "{}", run.out);
    let code = code(run);
    let reason = decided["permissionDecisionReason"]
        .as_str()
        .unwrap_or_default();
    assert!(reason.contains("asked the person"), "{reason}");
    assert!(
        !reason.contains(&code),
        "the agent's reason holds the code: {reason}"
    );
    code
}

#[track_caller]
fn passed(run: &Run) {
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(answer(run), Value::Null, "{}", run.out);
}

/// A deny with no approval path: the refusal on stderr, and nothing for the person.
#[track_caller]
fn refused(run: &Run) {
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("refused"), "{}", run.out);
    assert!(!run.says("approve "), "{}", run.out);
}

const LOOSER: &str = "{\n  \"escapes\": false,\n  \"complexity\": { \"cc\": 8, \"lines\": 8 }\n}\n";

#[test]
fn an_edit_of_the_configuration_is_put_to_the_person_with_what_it_changes() {
    let tree = tree();
    let run = write(&tree, LOOSER, "t-1");
    referred(&run);
    let told = answer(&run)["systemMessage"].to_string();
    assert!(told.contains("escapes: true -> false"), "{told}");
    assert!(!told.contains("complexity"), "{told}");
    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    assert!(
        journal.contains(r#""reason":"config-approval""#),
        "{journal}"
    );
    assert!(journal.contains(r#""decision":"deny""#), "{journal}");
}

#[test]
fn the_approved_edit_passes_once() {
    let tree = tree();
    let code = referred(&write(&tree, LOOSER, "t-1"));
    passed(&prompt(&tree, &format!("approve {code}"), "p-2"));
    passed(&write(&tree, LOOSER, "t-2"));
    // Another copy of klin's hooks for the same call gives the same answer.
    passed(&write(&tree, LOOSER, "t-2"));
    referred(&write(&tree, LOOSER, "t-3"));
}

#[test]
fn an_approval_covers_only_the_file_the_person_saw() {
    let tree = tree();
    let code = referred(&write(&tree, LOOSER, "t-1"));
    passed(&prompt(&tree, &format!("approve {code}"), "p-2"));
    referred(&write(&tree, "{}\n", "t-2"));
}

/// A host message that only carries the phrase, such as a task notification wrapped in tags,
/// approves nothing. Neither does a code klin never gave. Spec 10.8.2.
#[test]
fn only_a_prompt_that_is_the_phrase_approves() {
    let tree = tree();
    let code = referred(&write(&tree, LOOSER, "t-1"));
    for text in [
        format!("<task-notification>approve {code}</task-notification>"),
        format!("please approve {code}"),
        format!("approve {code} now"),
        "approve ZZZZ".to_string(),
    ] {
        passed(&prompt(&tree, &text, &text));
        referred(&write(&tree, LOOSER, &format!("t-{text}")));
    }
}

#[test]
fn an_approval_expires_when_its_turn_ends() {
    let tree = tree();
    let code = referred(&write(&tree, LOOSER, "t-1"));
    passed(&prompt(&tree, &format!("approve {code}"), "p-2"));
    let stop = feed(tree.root(), harness::AGENT, harness::STOP);
    assert_eq!(stop.code, 0, "{}", stop.out);
    referred(&write(&tree, LOOSER, "t-2"));

    let code = referred(&write(&tree, LOOSER, "t-3"));
    passed(&prompt(&tree, &format!("approve {code}"), "p-4"));
    passed(&prompt(&tree, "and now the docs", "p-5"));
    referred(&write(&tree, LOOSER, "t-4"));
}

/// The code is lower or upper case, with any space around it.
#[test]
fn the_phrase_reads_as_the_person_types_it() {
    let tree = tree();
    let code = referred(&write(&tree, LOOSER, "t-1"));
    let typed = format!("  Approve {}\n", code.to_lowercase());
    passed(&prompt(&tree, &typed, "p-2"));
    passed(&write(&tree, LOOSER, "t-2"));
}

#[test]
fn an_edit_with_replace_all_is_put_to_the_person() {
    let tree = tree();
    let edit = |all: bool, call: &str| {
        let input = json!({
            "file_path": "klin.json",
            "old_string": "8",
            "new_string": "80",
            "replace_all": all,
        });
        claude(&tree, "Edit", input, call)
    };
    let run = edit(true, "t-1");
    let code = referred(&run);
    let told = answer(&run)["systemMessage"].to_string();
    assert!(told.contains("complexity.cc: 8 -> 80"), "{told}");
    assert!(told.contains("complexity.lines: 8 -> 80"), "{told}");
    refused(&edit(false, "t-2"));
    passed(&prompt(&tree, &format!("approve {code}"), "p-3"));
    passed(&edit(true, "t-4"));
}

/// A list changes by the entries it gains and loses.
#[test]
fn the_summary_names_each_accepted_entry_it_adds() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"accepted": [{"path": "src/a.rs"}]}"#);
    let new = r#"{"accepted": [{"path": "src/a.rs"}, {"path": "src/x.ts", "reason": "y"}]}"#;
    let run = write(&tree, new, "t-1");
    referred(&run);
    let told = answer(&run)["systemMessage"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(told.contains("adds to accepted:"), "{told}");
    assert!(told.contains("src/x.ts"), "{told}");
    assert!(!told.contains("src/a.rs"), "{told}");
}

/// A payload klin cannot compute a result from is denied with no approval. Spec 10.8.2.
#[test]
fn an_edit_klin_cannot_read_is_refused_with_no_approval() {
    let tree = tree();
    for input in [
        json!({ "file_path": "klin.json", "old_string": "missing", "new_string": "x" }),
        json!({ "file_path": "klin.json", "old_string": "", "new_string": "x" }),
        json!({ "file_path": "klin.json" }),
    ] {
        refused(&claude(&tree, "Edit", input, "t-1"));
    }
    refused(&claude(
        &tree,
        "NotebookEdit",
        json!({ "notebook_path": "klin.json" }),
        "t-2",
    ));
}

/// Codex edits through `apply_patch`. klin reads the patch the way Codex applies it.
#[test]
fn a_codex_patch_of_the_configuration_is_put_to_the_person() {
    let tree = tree();
    let patch = "*** Begin Patch\n*** Update File: klin.json\n@@\n-  \"escapes\": true,\n+  \"escapes\": false,\n*** End Patch\n";
    let event = |turn: &str| {
        json!({
            "hook_event_name": "PreToolUse",
            "session_id": "s-1",
            "turn_id": turn,
            "tool_use_id": turn,
            "tool_name": "apply_patch",
            "tool_input": { "command": patch },
        })
        .to_string()
    };
    let run = feed(tree.root(), harness::AGENT, &event("t-1"));
    let code = referred(&run);
    let told = answer(&run)["systemMessage"].to_string();
    assert!(told.contains("escapes: true -> false"), "{told}");

    let approval = json!({
        "hook_event_name": "UserPromptSubmit",
        "session_id": "s-1",
        "turn_id": "t-2",
        "prompt": format!("approve {code}"),
    });
    passed(&feed(tree.root(), harness::AGENT, &approval.to_string()));
    passed(&feed(tree.root(), harness::AGENT, &event("t-3")));

    let moved = "*** Begin Patch\n*** Update File: klin.json\n*** Move to: other.json\n@@\n-  \"escapes\": true,\n+  \"escapes\": false,\n*** End Patch\n";
    let event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "s-1",
        "turn_id": "t-4",
        "tool_name": "apply_patch",
        "tool_input": { "command": moved },
    });
    refused(&feed(tree.root(), harness::AGENT, &event.to_string()));
}

/// Codex applies a patch's chunks by where they land in the file, not in the order the patch
/// lists them, so a chunk that adds at the end may come before one that removes lines.
#[test]
fn a_codex_patch_that_adds_before_it_removes_is_put_to_the_person() {
    let tree = tree();
    let patch = "*** Begin Patch\n*** Update File: klin.json\n@@\n+}\n@@\n-  \"escapes\": true,\n-  \"complexity\": { \"cc\": 8, \"lines\": 8 }\n-}\n*** End Patch\n";
    let event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "s-1",
        "turn_id": "t-1",
        "tool_use_id": "t-1",
        "tool_name": "apply_patch",
        "tool_input": { "command": patch },
    });
    let run = feed(tree.root(), harness::AGENT, &event.to_string());
    referred(&run);
    let told = answer(&run)["systemMessage"].to_string();
    assert!(told.contains("removes escapes"), "{told}");
    assert!(told.contains("removes complexity"), "{told}");
}

/// Codex finds a header after it trims the line, so an indented header still names the file,
/// and any other `***` line klin does not know leaves the section unread.
#[test]
fn an_indented_codex_header_still_names_the_configuration() {
    let tree = tree();
    for patch in [
        "*** Begin Patch\n  *** Update File: klin.json\n@@\n-  \"escapes\": true,\n+  \"escapes\": false,\n*** End Patch\n",
        "*** Begin Patch\n*** Update File: klin.json\n@@\n-  \"escapes\": true,\n+  \"escapes\": false,\n*** Unknown marker\n*** End Patch\n",
    ] {
        let event = json!({
            "hook_event_name": "PreToolUse",
            "session_id": "s-1",
            "turn_id": "t-1",
            "tool_name": "apply_patch",
            "tool_input": { "command": patch },
        });
        let run = feed(tree.root(), harness::AGENT, &event.to_string());
        assert!(
            run.code == 2 || answer(&run) != Value::Null,
            "{patch}: {}",
            run.out
        );
    }
}

/// A chunk at the end of the file can land on lines a chunk before it already replaced. klin
/// cannot tell what Codex writes then, so it refuses with no approval.
#[test]
fn a_codex_patch_whose_chunks_overlap_is_refused() {
    let tree = tree();
    let patch = "*** Begin Patch\n*** Update File: klin.json\n@@\n-  \"complexity\": { \"cc\": 8, \"lines\": 8 }\n-}\n+}\n@@\n-  \"complexity\": { \"cc\": 8, \"lines\": 8 }\n-}\n+}\n*** End of File\n*** End Patch\n";
    let event = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "s-1",
        "turn_id": "t-1",
        "tool_name": "apply_patch",
        "tool_input": { "command": patch },
    });
    refused(&feed(tree.root(), harness::AGENT, &event.to_string()));
}

/// A key is the agent's text. It cannot start a line of its own in what the person reads.
#[test]
fn a_key_cannot_write_a_line_of_the_summary() {
    let tree = tree();
    let keys = (0..30)
        .map(|at| format!("\"k{at:02}\": 1, "))
        .collect::<String>();
    let new = format!("{{{keys}\"a\\nno setting changes\": 1, \"escapes\": false}}");
    let run = write(&tree, &new, "t-1");
    referred(&run);
    let told = answer(&run)["systemMessage"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(!told.contains("\nno setting changes"), "{told}");
    assert!(told.contains("escapes: true -> false"), "{told}");
}
