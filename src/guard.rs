use std::iter::Peekable;
use std::str::Chars;

use crate::files::glob_matches;
use crate::host::{self, Decision, Event};

const SPLIT: char = '\u{0}';
const SEPARATORS: &[char] = &[';', '&', '|', '\n'];
/// Every wildcard a shell expands, so `?lin.json` and `klin.jso[n]` are read as patterns too.
const WILDCARDS: &[char] = &['*', '?', '['];

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
const GIT_VALUE_FLAGS: &[&str] = &["-C", "-c", "--git-dir", "--work-tree", "--exec-path"];
const GIT_READERS: &[&str] = &["diff", "show", "log", "status", "blame", "add", "commit"];

#[derive(clap::Args)]
pub struct Args {
    /// Read the hook event as this host's shape instead of the one its fields name
    #[arg(long)]
    host: Option<String>,
}

pub fn run(args: &Args) -> u8 {
    let Some(event) = host::read(args.host.as_deref()) else {
        return 0;
    };
    host::decide(&event, &decided(&event))
}

fn decided(event: &Event) -> Decision {
    let edits = !READ_TOOLS.contains(&event.tool.as_str());
    if (edits && guarded(&event.file_path)) || command_touches_guarded(&event.command) {
        return Decision::Deny(REFUSAL.to_string());
    }
    Decision::Allow
}

fn guarded(path: &str) -> bool {
    let path = path.trim_matches(['\'', '"']);
    if HOOKS.iter().any(|hook| path.contains(hook)) {
        return true;
    }
    let name = basename(path);
    if name.contains(WILDCARDS) {
        return NAMES
            .iter()
            .any(|guarded| glob_matches(name.as_bytes(), guarded.as_bytes()));
    }
    NAMES.contains(&name)
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn command_touches_guarded(command: &str) -> bool {
    let command = command.replace("\\\n", " ");
    segments(&command)
        .unwrap_or_else(|| blind_segments(&command))
        .iter()
        .any(|segment| segment_touches_guarded(segment))
}

/// The quote-blind split on every separator. It over-refuses a quoted separator, so it runs only
/// when a quote is left open. There the quoting says nothing, and one unbalanced quote would
/// otherwise hide every later separator.
fn blind_segments(command: &str) -> Vec<String> {
    command
        .replace('>', " > ")
        .replace("$(", " ; ")
        .replace('`', " ; ")
        .split([';', '&', '|', '\n'])
        .map(str::to_owned)
        .collect()
}

/// The same separators, with the quoting honoured. A single quote makes everything literal. A
/// double quote makes `;`, `&`, `|` and a newline literal, but a command substitution still runs
/// inside one, so `$(` and a backtick still split. `None` when a quote is left open, because
/// then the command does not say where its arguments end.
fn segments(command: &str) -> Option<Vec<String>> {
    let mut flat = String::with_capacity(command.len());
    let mut quotes = Quotes::default();
    let mut rest = command.chars().peekable();
    while let Some(c) = rest.next() {
        if quotes.toggled_by(c) {
            continue;
        }
        if splits(c, &quotes, &mut rest) {
            flat.push(SPLIT);
        } else if c == '>' && !quotes.any() {
            flat.push_str(" > ");
        } else {
            flat.push(c);
        }
    }
    if quotes.any() {
        return None;
    }
    Some(flat.split(SPLIT).map(str::to_owned).collect())
}

#[derive(Default)]
struct Quotes {
    single: bool,
    double: bool,
}

impl Quotes {
    fn toggled_by(&mut self, c: char) -> bool {
        match c {
            '\'' if !self.double => self.single = !self.single,
            '"' if !self.single => self.double = !self.double,
            _ => return false,
        }
        true
    }

    fn any(&self) -> bool {
        self.single || self.double
    }
}

fn splits(c: char, quotes: &Quotes, rest: &mut Peekable<Chars<'_>>) -> bool {
    if quotes.single {
        return false;
    }
    if c == '`' {
        return true;
    }
    if c == '$' && rest.peek() == Some(&'(') {
        rest.next();
        return true;
    }
    !quotes.any() && SEPARATORS.contains(&c)
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
        return git_subcommand(&words[1..]).is_some_and(|sub| GIT_READERS.contains(&sub));
    }
    READERS.contains(&name)
}

/// The global flags in `GIT_VALUE_FLAGS` take a value, so the word after one is not the
/// subcommand. Without this, `git -C sub add` reads as `sub` and a reader is refused.
fn git_subcommand<'a>(words: &[&'a str]) -> Option<&'a str> {
    let mut rest = words.iter();
    while let Some(word) = rest.next() {
        if GIT_VALUE_FLAGS.contains(word) {
            rest.next();
        } else if !word.starts_with('-') {
            return Some(word);
        }
    }
    None
}

/// Path-shaped tokens, split on the punctuation a shell, a heredoc body, or an interpreter's
/// inline script wraps a filename in, so `open('klin.json')` names it as plainly as `cat` does.
fn path_tokens(segment: &str) -> impl Iterator<Item = &str> {
    segment
        .split(|c: char| {
            !(c.is_alphanumeric()
                || matches!(
                    c,
                    '.' | '/' | '\\' | '_' | '-' | '*' | '?' | '[' | ']' | '!'
                ))
        })
        .filter(|token| !token.is_empty())
}
