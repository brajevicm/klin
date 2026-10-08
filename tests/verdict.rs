mod harness;

use harness::{AGENT, Tree, feed};
use serde_json::Value;

const A_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": false}"#;
const A_SECOND_STOP: &str = r#"{"hook_event_name": "Stop", "stop_hook_active": true}"#;
const A_PROMPT: &str = r#"{"hook_event_name": "UserPromptSubmit"}"#;

const DOC_SIZE: &str = r#"{"doc_size": {"README.md": 10}}"#;
/// A config whose escapes scope holds no applicable file, so that capability errors.
const A_BROKEN_GATE: &str = r#"{
  "doc_size": {"README.md": 10},
  "escapes": { "in": "README.md" }
}"#;
const AN_UNMATCHED_ACCEPTED: &str = r#"{
  "accepted": [{"gate": "escapes", "file": "src/lib.rs", "text": "the line that held it",
                "count": 1}],
  "doc_size": {"README.md": 10},
  "escapes": { "in": "src" }
}"#;

/// Words a source-repair instruction opens with, which no note, hole or error may hold.
const REPAIRS: [&str; 4] = ["fix what", "Make the file", "stop again", "restore"];

fn tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.words("README.md", 5);
    tree.write("src/lib.rs", "pub fn kept() -> u8 {\n    1\n}\n");
    tree.base();
    tree
}

fn stop(tree: &Tree) -> harness::Run {
    feed(tree.root(), AGENT, A_STOP)
}

fn second_stop(tree: &Tree) -> harness::Run {
    feed(tree.root(), AGENT, A_SECOND_STOP)
}

fn prompt(tree: &Tree) {
    let run = feed(tree.root(), AGENT, A_PROMPT);
    assert_eq!(run.code, 0, "{}", run.out);
}

/// What a stop hands the person through the host's `systemMessage`, or nothing.
fn told(run: &harness::Run) -> String {
    run.out
        .lines()
        .find_map(|line| {
            let held: Value = serde_json::from_str(line).ok()?;
            held.get("systemMessage")?.as_str().map(str::to_string)
        })
        .unwrap_or_default()
}

fn window(tree: &Tree) -> Value {
    let run = tree.run(&["status", "--json"]);
    assert_eq!(run.code, 0, "{}", run.out);
    run.json()["window"].clone()
}

fn no_repair(text: &str) {
    for words in REPAIRS {
        assert!(!text.contains(words), "{words:?} in: {text}");
    }
}

/// A gap the change opened is told and spends no gate block, and the stamp is green. Spec 10.4.
#[test]
fn an_opened_gap_spends_no_gate_block_and_leaves_the_stamp_green() {
    let tree = tree(DOC_SIZE);
    tree.write("src/broken.rs", "fn ( { ) unbalanced");

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(told(&run).contains("src/broken.rs"), "{}", run.out);
    no_repair(&told(&run));
    assert_eq!(tree.field("verdict"), "green", "{}", run.out);
}

/// A limit the change did not open is a coverage note: told once under the stamp, with no
/// block, no repair instruction and a green stamp. Spec 2.3, 10.4.
#[test]
fn a_coverage_note_is_told_once_per_stamp_and_leaves_the_stamp_green() {
    let tree = tree(DOC_SIZE);
    tree.write("src/broken.rs", "fn broken( {\n");
    tree.base();
    tree.write("src/broken.rs", "fn broken( { (\n");

    let first = stop(&tree);
    assert_eq!(first.code, 0, "{}", first.out);
    assert!(told(&first).contains("src/broken.rs"), "{}", first.out);
    no_repair(&told(&first));
    assert_eq!(tree.field("verdict"), "green", "{}", first.out);

    let again = second_stop(&tree);
    assert_eq!(again.code, 0, "{}", again.out);
    assert_eq!(told(&again), "", "{}", again.out);
}

/// A capability-scope error blocks nothing and keeps nothing red, and its text asks for no
/// source change. Spec 7.3, 10.4.
#[test]
fn a_capability_error_alone_spends_no_gate_block_and_leaves_the_stamp_green() {
    let tree = tree(A_BROKEN_GATE);
    tree.write("src/work.rs", "pub fn work() {}\n");

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(told(&run).contains("escapes"), "{}", run.out);
    no_repair(&told(&run));
    assert_eq!(tree.field("verdict"), "green", "{}", run.out);

    let again = second_stop(&tree);
    assert_eq!(told(&again), "", "{}", again.out);
}

/// A FAIL beside a capability-scope error spends its block, and the text names the error as a
/// limitation with no repair for it. Spec 10.4.
#[test]
fn a_fail_beside_a_capability_error_still_spends_its_block() {
    let tree = tree(A_BROKEN_GATE);
    tree.words("README.md", 30);

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("gate block 1 of 2"), "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("ERR   escapes"), "{}", run.out);
    assert!(
        run.says("capability that could not run is a limitation of this stop"),
        "{}",
        run.out
    );
    assert_eq!(tree.field("verdict"), "red", "{}", run.out);
}

/// An invalid klin.json blocks nothing, writes `unjudged`, and `klin status` says nothing was
/// judged. Spec 6.6, 15.
#[test]
fn a_configuration_error_writes_unjudged_and_status_says_nothing_judged() {
    let tree = tree(DOC_SIZE);
    assert_eq!(stop(&tree).code, 0);
    tree.write("klin.json", r#"{"doc_size": {"README.md": "ten"}}"#);

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(told(&run).contains("klin.json"), "{}", run.out);
    no_repair(&told(&run));
    assert_eq!(tree.field("verdict"), "unjudged", "{}", run.out);

    let held = window(&tree);
    assert_eq!(held["verdict"], "unjudged", "{held}");
    assert!(
        held["error"]
            .as_str()
            .is_some_and(|error| !error.is_empty()),
        "{held}"
    );
    let text = tree.run(&["status"]);
    assert!(text.says("window: nothing judged"), "{}", text.out);
}

/// An `unjudged` Stop after a green one lets the next prompt move the stamp, so the first Stop
/// after the fix never judges what came before. Spec 6.6.
#[test]
fn the_prompt_after_an_unjudged_stop_moves_the_stamp() {
    let tree = tree(DOC_SIZE);
    assert_eq!(stop(&tree).code, 0);
    tree.write("klin.json", "not json");
    assert_eq!(stop(&tree).code, 0);
    let held = tree.field("commit");

    prompt(&tree);
    assert_ne!(tree.field("commit"), held);
}

/// An `unjudged` Stop never hides a red window. Spec 6.6.
#[test]
fn an_unjudged_stop_after_a_red_stop_keeps_the_stamp_red() {
    let tree = tree(DOC_SIZE);
    tree.words("README.md", 30);
    assert_eq!(stop(&tree).code, 2);
    assert_eq!(tree.field("verdict"), "red");
    let held = tree.field("commit");

    tree.write("klin.json", "not json");
    let run = second_stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.field("verdict"), "red", "{}", run.out);

    prompt(&tree);
    assert_eq!(tree.field("commit"), held);
}

/// A Stop that dies after it took the lock leaves `aborted` and not the green before it, and
/// `klin status` says since when. Spec 6.6.
#[test]
fn a_stop_that_klin_aborts_leaves_aborted_and_not_an_earlier_green() {
    let tree = tree(DOC_SIZE);
    assert_eq!(stop(&tree).code, 0);
    assert_eq!(tree.field("verdict"), "green");

    tree.write(
        "klin.json",
        r#"{"doc_size": {"README.md": 10}, "build": "kill -9 $PPID"}"#,
    );
    let _ = stop(&tree);
    assert_eq!(tree.field("verdict"), "aborted");

    let held = window(&tree);
    assert_eq!(held["verdict"], "aborted", "{held}");
    assert!(held["aborted_since"].is_u64(), "{held}");
    let commit = tree.field("commit");
    prompt(&tree);
    assert_eq!(
        tree.field("commit"),
        commit,
        "a prompt moved an aborted stamp"
    );
}

/// A red window names what keeps it red: the open findings and the deleted tests klin has not
/// asked about. Spec 11.4.
#[test]
fn status_names_the_open_findings_and_the_unasked_deleted_tests() {
    let tree = tree(DOC_SIZE);
    tree.write("tests/test_one.py", "def test_one():\n    assert True\n");
    tree.write("tests/test_two.py", "def test_two():\n    assert True\n");
    tree.base();
    tree.words("README.md", 30);
    tree.remove("tests/test_one.py");

    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("gate block 1 of 2"), "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
    assert!(run.says("tests/test_one.py"), "{}", run.out);
    assert_eq!(window(&tree)["unasked"], serde_json::json!([]));

    tree.words("README.md", 31);
    let second = second_stop(&tree);
    assert!(second.says("gate block 2 of 2"), "{}", second.out);
    tree.remove("tests/test_two.py");
    let capped = second_stop(&tree);
    assert_eq!(capped.code, 0, "{}", capped.out);

    let held = window(&tree);
    assert_eq!(held["verdict"], "red", "{held}");
    assert!(
        held["open"].as_array().is_some_and(|open| !open.is_empty()),
        "{held}"
    );
    let unasked = held["unasked"].to_string();
    assert!(unasked.contains("tests/test_two.py"), "{held}");
    assert!(!unasked.contains("tests/test_one.py"), "{held}");
}

/// An accepted entry that matched nothing is a note the Stop tells, and it blocks nothing.
/// Spec 15.
#[test]
fn an_unmatched_accepted_entry_is_a_note_at_the_stop() {
    let tree = tree(AN_UNMATCHED_ACCEPTED);
    tree.write("src/lib.rs", "pub fn kept() -> u8 {\n    2\n}\n");

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(told(&run).contains("matched nothing"), "{}", run.out);
    assert_eq!(tree.field("verdict"), "green", "{}", run.out);
}

/// A turn that touched only files no capability measures is green. Spec 19.
#[test]
fn a_turn_that_changes_no_measurable_file_leaves_the_stamp_green() {
    let tree = tree(DOC_SIZE);
    tree.write("notes.bin", "\u{0}\u{1}\u{2}");

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert_eq!(tree.field("verdict"), "green", "{}", run.out);
}

/// A window no Stop judged stays through an `unjudged` Stop, so the first Stop after the fix
/// judges the work done before the configuration broke. Spec 6.6.
#[test]
fn an_unjudged_stop_over_a_pending_window_keeps_the_work_in_it() {
    let tree = tree(DOC_SIZE);
    assert_eq!(tree.session().code, 0);
    assert_eq!(tree.field("verdict"), "pending");
    tree.words("README.md", 30);

    tree.write("klin.json", "not json");
    assert_eq!(stop(&tree).code, 0);
    assert_eq!(tree.field("verdict"), "unjudged");
    let held = tree.field("commit");
    prompt(&tree);
    assert_eq!(
        tree.field("commit"),
        held,
        "a prompt moved a window no Stop judged"
    );

    tree.write("klin.json", DOC_SIZE);
    let run = stop(&tree);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL  doc-size"), "{}", run.out);
}

const A_CURSOR_STOP: &str = r#"{"hook_event_name":"stop","cursor_version":"3.20.21","conversation_id":"s1","session_id":"s1","loop_count":0}"#;

/// A notice the host never took is not recorded as told, so a later Stop under the same stamp
/// tells it. Spec 2.3, 10.7.
#[test]
fn a_notice_klin_could_not_hand_to_cursor_is_told_at_a_later_stop() {
    let tree = tree(A_BROKEN_GATE);
    tree.write("src/work.rs", "pub fn work() {}\n");
    tree.write(".git/klin/handed", "");

    let quiet = feed(tree.root(), AGENT, A_CURSOR_STOP);
    assert_eq!(quiet.code, 0, "{}", quiet.out);
    assert!(!quiet.says("followup_message"), "{}", quiet.out);
    assert_eq!(
        tree.field("told"),
        "",
        "an undelivered notice was recorded as told"
    );

    tree.remove(".git/klin/handed");
    let told = feed(tree.root(), AGENT, A_CURSOR_STOP);
    assert_eq!(told.code, 0, "{}", told.out);
    assert!(told.says("escapes"), "{}", told.out);
}

/// A tree the survey finds no source root in is a limitation the Stop tells, once. Spec 2.3.
#[test]
fn a_stop_in_a_tree_with_no_source_root_tells_it_once() {
    let tree = Tree::new();
    tree.write("klin.json", DOC_SIZE);
    tree.words("README.md", 5);
    tree.base();
    tree.words("README.md", 6);

    let run = stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(told(&run).contains("found no source root"), "{}", run.out);
    no_repair(&told(&run));

    let again = second_stop(&tree);
    assert_eq!(told(&again), "", "{}", again.out);
}

/// A capability error that arrives after the gate blocks are spent is still told to the person,
/// once. Spec 2.3, 10.4.
#[test]
fn a_new_error_after_the_last_gate_block_is_told_once() {
    let tree = tree(DOC_SIZE);
    tree.words("README.md", 30);
    assert!(stop(&tree).says("gate block 1 of 2"));
    tree.words("README.md", 31);
    assert!(second_stop(&tree).says("gate block 2 of 2"));

    tree.write("klin.json", A_BROKEN_GATE);
    let run = second_stop(&tree);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(told(&run).contains("ERR   escapes"), "{}", run.out);
    no_repair(&told(&run));

    let again = second_stop(&tree);
    assert_eq!(again.code, 0, "{}", again.out);
    assert!(!told(&again).contains("escapes"), "{}", again.out);
}

/// The journal keeps why an `unjudged` Stop measured nothing, after the configuration is fixed.
/// Spec 6.6, 13.1.
#[test]
fn the_journal_keeps_the_error_of_an_unjudged_stop() {
    let tree = tree(DOC_SIZE);
    tree.words("README.md", 30);
    assert_eq!(stop(&tree).code, 2);
    tree.write("klin.json", "not json");
    assert_eq!(second_stop(&tree).code, 0);
    tree.write("klin.json", DOC_SIZE);

    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    let unjudged = journal
        .lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|line| line["kind"] == "stop")
        .unwrap_or_default();
    assert_eq!(unjudged["verdict"], "red", "{unjudged}");
    assert!(unjudged.to_string().contains("klin.json"), "{unjudged}");
}
