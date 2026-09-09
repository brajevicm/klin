use std::iter::Peekable;
use std::path::{Path, PathBuf};
use std::str::Chars;
use std::sync::OnceLock;

use crate::files::glob_matches;
use crate::host::{self, Decision, Event};
use crate::state;

const SPLIT: char = '\u{0}';
const SEPARATORS: &[char] = &[';', '&', '|', '\n'];
/// Every wildcard a shell expands, so `?lin.json` and `klin.jso[n]` are read as patterns too.
const WILDCARDS: &[char] = &['*', '?', '['];

const REFUSAL: &str = "klin: refused — this would change the configuration (klin.json), the \
    hooks, or the code owners. Fix the code the gate names instead. Only a person changes those, \
    in a reviewed commit.";

const INIT_REFUSAL: &str = "klin: refused — `klin init` writes the configuration. Only a \
    person runs it, in a reviewed commit.";
const RESET_REFUSAL: &str = "klin: refused — `klin turn reset` reopens the window a gate \
    failed in. Only a person runs it.";

/// klin's own subcommands that only a person runs, and the reason each is refused.
const KLIN_REFUSED: &[(&[&str], &str)] = &[
    (&["init"], INIT_REFUSAL),
    (&["turn", "reset"], RESET_REFUSAL),
];

/// The ref each worktree keeps its turn stamp under, and the state directory under a plain
/// `.git`. Both are guarded so that a write to them is a question, never a silent act.
const REFERENCE: &str = "refs/worktree/klin";
const GIT_STATE: &str = ".git/klin";

/// The files that configure a check klin cannot judge. An agent that edits one of these can
/// weaken every check that reads it without touching the configuration, and klin cannot tell a
/// loosening from a fix, so a person looks. A fixed table, never a config key. Section 9.4.
const VERIFICATION: &[&str] = &[
    ".eslintrc*",
    "eslint.config.*",
    "pytest.ini",
    "jest.config.*",
    "vitest.config.*",
    ".coveragerc",
    "codecov.yml",
    ".codecov.yml",
];
/// A workflow is guarded wherever GitHub reads it from.
const WORKFLOWS: &str = ".github/workflows";
/// These carry a check only when they hold its table, so the guard reads the file to tell.
const TABLED: &[&str] = &["pyproject.toml", "setup.cfg", "tox.ini"];
const TABLES: &[&str] = &["[tool.pytest", "[tool.coverage"];

const READ_TOOLS: &[&str] = &["Read", "NotebookRead"];
const HOOKS: &[&str] = &[".claude/settings", ".cursor/hooks", ".codex/config"];
/// Guarded wherever they sit. GitHub honours the code owners at the root of the tree, under
/// `.github/` and under `docs/`, so a path that names one of those is not enough.
const NAMES: &[&str] = &["klin.json", "CODEOWNERS"];
const RESTORERS: &[&str] = &["checkout", "restore"];
const READERS: &[&str] = &[
    "cat", "head", "tail", "less", "grep", "rg", "diff", "wc", "stat", "ls", "file", "jq", "du",
];
/// `find` reads the tree until one of these makes it act on what it found or write a file.
const FIND_WRITERS: &[&str] = &[
    "-delete", "-exec", "-execdir", "-ok", "-okdir", "-fprint", "-fprint0", "-fprintf", "-fls",
];
const GIT_VALUE_FLAGS: &[&str] = &["-C", "-c", "--git-dir", "--work-tree", "--exec-path"];
const GIT_READERS: &[&str] = &[
    "diff",
    "show",
    "log",
    "status",
    "blame",
    "add",
    "commit",
    "rev-parse",
    "cat-file",
    "for-each-ref",
];

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
    match edits.then(|| touched(&event.file_path)).flatten() {
        Some(decision) => decision,
        None => command_decision(&event.command),
    }
}

/// What one path an edit tool or a redirect names is worth: a deny for the files only a person
/// changes, an ask for klin's own state and for a verification file, and nothing otherwise.
fn touched(path: &str) -> Option<Decision> {
    if guarded(path) {
        return Some(Decision::Deny(REFUSAL.to_string()));
    }
    if state_guarded(path) {
        return Some(asked_about(path));
    }
    if verification(path) {
        return Some(asked_about_verification(path));
    }
    None
}

fn asked_about(token: &str) -> Decision {
    Decision::Ask(format!(
        "this names \"{token}\", which is guarded: the configuration, the hooks, the code \
         owners, or klin's own state."
    ))
}

fn asked_about_verification(token: &str) -> Decision {
    Decision::Ask(format!(
        "\"{token}\" configures a check, and klin cannot tell a loosening from a fix, so a \
         person looks."
    ))
}

/// The state directory of ADR 0019 and the turn ref. To find them the guard reads
/// `KLIN_STATE_DIR` and asks git, and it reads no configuration. Section 9.4.
fn state_guarded(path: &str) -> bool {
    let path = path.trim_matches(['\'', '"']);
    if under_the_override(path) {
        return true;
    }
    if !path.contains("klin") {
        return false;
    }
    path.contains(REFERENCE)
        || path.contains(GIT_STATE)
        || in_a_worktree(path)
        || state_dir().is_some_and(|at| path.contains(at))
}

/// A path under `KLIN_STATE_DIR`. An empty or relative-to-nothing value names no directory,
/// and a value is a prefix of a path, never a substring of one.
fn under_the_override(path: &str) -> bool {
    let Some(base) = std::env::var_os(state::OVERRIDE) else {
        return false;
    };
    let base = PathBuf::from(base);
    base.components().next().is_some() && Path::new(path).starts_with(&base)
}

/// The state directory of a linked worktree, which sits under the main repository's `.git`.
fn in_a_worktree(path: &str) -> bool {
    let Some((_, under)) = path.split_once(".git/worktrees/") else {
        return false;
    };
    under.split('/').nth(1) == Some("klin") || under.ends_with("/klin")
}

fn state_dir() -> Option<&'static String> {
    static AT: OnceLock<Option<String>> = OnceLock::new();
    AT.get_or_init(|| {
        let cwd = std::env::current_dir().ok()?;
        Some(state::dir(&cwd)?.display().to_string())
    })
    .as_ref()
}

fn verification(path: &str) -> bool {
    let path = path.trim_matches(['\'', '"']);
    if path.contains(WORKFLOWS) {
        return true;
    }
    let name = basename(path);
    if VERIFICATION.iter().any(|guarded| named(name, guarded)) {
        return true;
    }
    TABLED.contains(&name)
        && std::fs::read_to_string(path)
            .is_ok_and(|text| TABLES.iter().any(|table| text.contains(table)))
}

/// A name matches a guarded pattern, or, when the name is itself a glob, matches it as one.
fn named(name: &str, guarded: &str) -> bool {
    glob_matches(guarded.as_bytes(), name.as_bytes())
        || (name.contains(WILDCARDS) && glob_matches(name.as_bytes(), guarded.as_bytes()))
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

/// A deny anywhere in the command wins, and otherwise the first ask does.
fn command_decision(command: &str) -> Decision {
    let command = without_heredoc_bodies(command).replace("\\\n", " ");
    let mut asked = None;
    for segment in segments(&command).unwrap_or_else(|| blind_segments(&command)) {
        match segment_decision(&segment) {
            Decision::Deny(why) => return Decision::Deny(why),
            Decision::Ask(why) => asked = asked.or(Some(why)),
            Decision::Allow => {}
        }
    }
    match asked {
        Some(why) => Decision::Ask(why),
        None => Decision::Allow,
    }
}

/// A heredoc body is data. The guard keeps every command word and every redirect target,
/// including a command after the terminator, and drops the text between. Section 9.4.
fn without_heredoc_bodies(command: &str) -> String {
    let lines: Vec<&str> = command.split('\n').collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        kept.push(lines[at]);
        at += 1;
        let opened = heredoc_delimiter(lines[at - 1]);
        let ends = opened.and_then(|delimiter| ends_at(&lines[at..], &delimiter));
        if let Some(end) = ends {
            at += end + 1;
        }
    }
    kept.join("\n")
}

/// Where the body ends, and `None` when no line closes it. A `<<` that opens nothing klin can
/// see the end of is a `<<` in an argument, so the lines after it are commands again.
fn ends_at(lines: &[&str], delimiter: &str) -> Option<usize> {
    lines.iter().position(|line| line.trim() == delimiter)
}

/// The word a heredoc ends at, and `None` for a line that opens none. A here string (`<<<`)
/// carries its data on the line itself, so it opens no body.
fn heredoc_delimiter(line: &str) -> Option<String> {
    let at = unquoted_heredoc(line)?;
    let rest = &line[at + 2..];
    if rest.starts_with('<') {
        return None;
    }
    let word: String = rest
        .trim_start_matches('-')
        .trim_start()
        .chars()
        .take_while(|c| !c.is_whitespace() && !SEPARATORS.contains(c) && *c != '>')
        .collect();
    let word = word.trim_matches(['\'', '"']).to_string();
    (!word.is_empty()).then_some(word)
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

fn segment_decision(segment: &str) -> Decision {
    let words: Vec<&str> = segment.split_whitespace().collect();
    let redirect = redirected(&words);
    if matches!(redirect, Decision::Deny(_)) {
        return redirect;
    }
    if restores_a_tree(&words) {
        return Decision::Deny(REFUSAL.to_string());
    }
    if let Some(why) = a_persons_command(&words) {
        return Decision::Deny(why.to_string());
    }
    if segment_is_a_reader(&words) {
        return redirect;
    }
    path_tokens(segment).find_map(mentioned).unwrap_or(redirect)
}

/// What the segment's redirect targets are worth. A deny wins, and otherwise the first ask.
fn redirected(words: &[&str]) -> Decision {
    let mut asked = Decision::Allow;
    for target in redirect_targets(words) {
        match touched(target) {
            Some(Decision::Deny(why)) => return Decision::Deny(why),
            Some(ask) if matches!(asked, Decision::Allow) => asked = ask,
            _ => {}
        }
    }
    asked
}

/// A guarded path a command outside the reader list names. klin cannot tell a write from a
/// mention, so every one of these is an ask, whatever the guarded path is.
fn mentioned(token: &str) -> Option<Decision> {
    if guarded(token) || state_guarded(token) {
        return Some(asked_about(token));
    }
    verification(token).then(|| asked_about_verification(token))
}

/// One of klin's own subcommands that only a person runs, whatever flags it carries.
fn a_persons_command(words: &[&str]) -> Option<&'static str> {
    let at = words.iter().position(|word| {
        basename(word).trim_end_matches(".exe") == "klin" && !word.starts_with('-')
    })?;
    if words[..at].iter().any(|word| !a_prefix(word)) {
        return None;
    }
    let rest: Vec<&str> = words[at + 1..]
        .iter()
        .copied()
        .filter(|word| !word.starts_with('-'))
        .collect();
    KLIN_REFUSED
        .iter()
        .find(|(command, _)| rest.starts_with(command))
        .map(|(_, why)| *why)
}

/// What may stand in front of a command without changing which command it is: an assignment,
/// `env` and its assignments, and a runner that passes the rest through.
fn a_prefix(word: &str) -> bool {
    word.contains('=') || ["env", "npx", "pnpm", "bunx", "time", "nice", "sudo"].contains(&word)
}

fn restores_a_tree(words: &[&str]) -> bool {
    words.iter().any(|word| basename(word) == "git")
        && words.iter().any(|word| RESTORERS.contains(word))
        && words.iter().any(|word| *word == "." || word.ends_with('/'))
}

fn redirect_targets<'a>(words: &[&'a str]) -> Vec<&'a str> {
    words
        .iter()
        .enumerate()
        .filter(|(_, word)| **word == ">")
        .filter_map(|(at, _)| words.get(at + 1).copied())
        .collect()
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
    if name == "find" {
        return !words.iter().any(|word| FIND_WRITERS.contains(word));
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

/// Where the line opens a heredoc, honouring the quoting the way `segments` does: a `<<`
/// inside an argument is two characters, not a redirect.
fn unquoted_heredoc(line: &str) -> Option<usize> {
    let mut quotes = Quotes::default();
    let mut open = None;
    for (at, c) in line.char_indices() {
        if quotes.toggled_by(c) {
            continue;
        }
        match (c, open) {
            ('<', Some(_)) if !quotes.any() => return Some(at - 1),
            ('<', None) if !quotes.any() => open = Some(at),
            _ => open = None,
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
