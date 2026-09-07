use std::io::Read;

use serde_json::Value;

const REFUSAL: &str = "klin: refused — this would change the configuration (klin.json) or the \
    hooks. Fix the code the gate names instead. Only a person changes those, in a reviewed \
    commit.";

const WRITERS: &[&str] = &["tee", "cp", "mv", "rm", "truncate", "install"];
const READERS: &[&str] = &["Read", "NotebookRead"];
const HOOKS: &[&str] = &[".claude/settings", ".cursor/hooks", ".codex/config"];
const RESTORERS: &[&str] = &["checkout", "restore"];

pub fn run() -> u8 {
    let mut text = String::new();
    if std::io::stdin().read_to_string(&mut text).is_err() {
        return 0;
    }
    let Ok(event) = serde_json::from_str::<Value>(&text) else {
        return 0;
    };
    let field = |key| {
        event
            .get("tool_input")
            .and_then(|input| input.get(key))
            .and_then(Value::as_str)
            .unwrap_or("")
    };
    let tool = event.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let edits = !READERS.contains(&tool);
    if (edits && (guarded(field("file_path")) || guarded(field("notebook_path"))))
        || command_writes_guarded(field("command"))
    {
        eprintln!("{REFUSAL}");
        return 2;
    }
    0
}

fn guarded(path: &str) -> bool {
    let path = path.trim_matches(['\'', '"']);
    if HOOKS.iter().any(|hook| path.contains(hook)) {
        return true;
    }
    basename(path) == "klin.json"
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn command_writes_guarded(command: &str) -> bool {
    command
        .replace('>', " > ")
        .split([';', '&', '|', '\n'])
        .any(segment_writes_guarded)
}

fn segment_writes_guarded(segment: &str) -> bool {
    let words: Vec<&str> = segment.split_whitespace().collect();
    redirects_to_guarded(&words)
        || (writes(&words) && words.iter().any(|word| guarded(word)))
        || restores_a_tree(&words)
        || fills_in_the_configuration(&words)
}

/// `init --add` rewrites the configuration, so it is a person's command, like an edit to it.
fn fills_in_the_configuration(words: &[&str]) -> bool {
    words.contains(&"init") && words.contains(&"--add")
}

fn restores_a_tree(words: &[&str]) -> bool {
    words.iter().any(|word| basename(word) == "git")
        && words.iter().any(|word| RESTORERS.contains(word))
        && words.iter().any(|word| *word == "." || word.ends_with('/'))
}

fn redirects_to_guarded(words: &[&str]) -> bool {
    words
        .iter()
        .enumerate()
        .any(|(at, word)| *word == ">" && words.get(at + 1).is_some_and(|target| guarded(target)))
}

fn writes(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(at, word)| {
        let name = basename(word);
        WRITERS.contains(&name)
            || (name == "git"
                && words[at..]
                    .iter()
                    .any(|subcommand| RESTORERS.contains(subcommand)))
            || (name == "sed"
                && words[at..]
                    .iter()
                    .any(|flag| flag.starts_with('-') && flag.contains('i')))
    })
}
