use std::io::Read;

use serde_json::Value;

const REFUSAL: &str = "klin: refused — this would change the configuration (klin.json), the \
    hooks, or the code owners. Fix the code the gate names instead. Only a person changes those, \
    in a reviewed commit.";

const READ_TOOLS: &[&str] = &["Read", "NotebookRead"];
const HOOKS: &[&str] = &[".claude/settings", ".cursor/hooks", ".codex/config"];
/// Guarded wherever they sit. GitHub honours the code owners at the root of the tree, under
/// `.github/` and under `docs/`, so a path that names one of those is not enough.
const NAMES: &[&str] = &["klin.json", "CODEOWNERS"];
const RESTORERS: &[&str] = &["checkout", "restore"];
const READERS: &[&str] = &[
    "cat", "head", "tail", "less", "grep", "rg", "diff", "wc", "stat", "ls", "file", "jq",
];
const GIT_READERS: &[&str] = &["diff", "show", "log", "status", "blame", "add", "commit"];

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
    let edits = !READ_TOOLS.contains(&tool);
    if (edits && (guarded(field("file_path")) || guarded(field("notebook_path"))))
        || command_touches_guarded(field("command"))
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
    NAMES.contains(&basename(path))
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn command_touches_guarded(command: &str) -> bool {
    command
        .replace("\\\n", " ")
        .replace('>', " > ")
        .split([';', '&', '|', '\n'])
        .any(segment_touches_guarded)
}

fn segment_touches_guarded(segment: &str) -> bool {
    let words: Vec<&str> = segment.split_whitespace().collect();
    redirects_to_guarded(&words)
        || restores_a_tree(&words)
        || fills_in_the_configuration(&words)
        || (!segment_is_a_reader(&words) && path_tokens(segment).any(guarded))
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

/// A guarded name may appear as an argument to one of these, and nowhere else. `git` is a
/// reader only for the subcommands that change no content. An interpreter is never a reader,
/// so its inline script and its heredoc body are held to the same rule as any other command.
fn segment_is_a_reader(words: &[&str]) -> bool {
    let Some(command) = words.first() else {
        return false;
    };
    let name = basename(command);
    if name == "git" {
        return words[1..]
            .iter()
            .find(|word| !word.starts_with('-'))
            .is_some_and(|sub| GIT_READERS.contains(sub));
    }
    READERS.contains(&name)
}

/// Path-shaped tokens, split on the punctuation a shell, a heredoc body, or an interpreter's
/// inline script wraps a filename in, so `open('klin.json')` names it as plainly as `cat` does.
fn path_tokens(segment: &str) -> impl Iterator<Item = &str> {
    segment
        .split(|c: char| !(c.is_alphanumeric() || matches!(c, '.' | '/' | '\\' | '_' | '-')))
        .filter(|token| !token.is_empty())
}
