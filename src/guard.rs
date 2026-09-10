use std::iter::Peekable;
use std::str::Chars;

use crate::files::glob_matches;
use crate::host::{self, Decision, Event};

const SPLIT: char = '\u{0}';
const SEPARATORS: &[char] = &[';', '&', '|', '\n'];
/// Every wildcard a shell expands, so `?lin.json` and `klin.jso[n]` are read as patterns too.
const WILDCARDS: &[char] = &['*', '?', '['];

const REFUSAL: &str = "klin: refused — this would change the configuration (klin.json). Fix \
    the code the gate names instead. Only a person changes it, in a reviewed commit.";

const INIT_REFUSAL: &str = "klin: refused — `klin init` writes the configuration. Only a \
    person runs it, in a reviewed commit.";
const RESET_REFUSAL: &str = "klin: refused — `klin turn reset` reopens the window a gate \
    failed in. Only a person runs it.";

/// klin's own subcommands that only a person runs, and the reason each is refused.
const KLIN_REFUSED: &[(&[&str], &str)] = &[
    (&["init"], INIT_REFUSAL),
    (&["turn", "reset"], RESET_REFUSAL),
];

const READ_TOOLS: &[&str] = &["Read", "NotebookRead"];
/// The one file klin guards. A check's own configuration, a host's hook file and the code
/// owners are ordinary files: klin cannot tell a loosening from a fix in any of them, and
/// refusing a whole settings file refuses the work that has nothing to do with klin.
const NAME: &str = "klin.json";
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
    if edits {
        if let Some(decision) = paths_decision(&event.file_paths) {
            return decision;
        }
        if matches!(event.host, host::Host::Codex) && event.tool == "apply_patch" {
            return Decision::Allow;
        }
    }
    command_decision(&event.command)
}

/// The strictest decision for all paths one edit call names. A direct guarded path is a deny;
/// the ordering also preserves an ask if another host adds one later. Section 9.1.
fn paths_decision(paths: &[String]) -> Option<Decision> {
    let mut asked = None;
    for path in paths {
        match touched(path) {
            Some(Decision::Deny(reason)) => return Some(Decision::Deny(reason)),
            Some(Decision::Ask(reason)) => asked = asked.or(Some(Decision::Ask(reason))),
            Some(Decision::Allow) | None => {}
        }
    }
    asked
}

/// What one path an edit tool or a redirect names is worth: a deny for the one file only a
/// person changes, and nothing otherwise.
fn touched(path: &str) -> Option<Decision> {
    guarded(path).then(|| Decision::Deny(REFUSAL.to_string()))
}

fn asked_about(token: &str) -> Decision {
    Decision::Ask(format!(
        "this names \"{token}\", which is the configuration klin guards."
    ))
}

fn guarded(path: &str) -> bool {
    let name = basename(path.trim_matches(['\'', '"']));
    if name.contains(WILDCARDS) {
        return glob_matches(name.as_bytes(), NAME.as_bytes());
    }
    name == NAME
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// A deny anywhere in the command wins, and otherwise the first ask does.
fn command_decision(command: &str) -> Decision {
    let command = without_heredoc_bodies(command).replace("\\\n", " ");
    let (outer, inner) = lifted(&command);
    let mut asked = None;
    let lines = std::iter::once(outer).chain(inner);
    for segment in lines.flat_map(|text| segments(&text).unwrap_or_else(|| blind_segments(&text))) {
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

/// A command substitution is a command of its own, so it leaves the line it sat in. The words
/// after its closing parenthesis stay with the command that owns them, and the command inside
/// it is read on its own. Section 9.4.
fn lifted(command: &str) -> (String, Vec<String>) {
    let mut nesting = Nesting::default();
    let mut inner: Vec<String> = Vec::new();
    let mut open: Vec<String> = vec![String::new()];
    let mut previous = ' ';
    for c in command.chars() {
        let step = nesting.nested(c, previous);
        previous = c;
        match step {
            Some(true) => {
                if let Some(text) = open.last_mut().filter(|text| text.ends_with('$')) {
                    text.pop();
                }
                open.push(String::new());
            }
            Some(false) => inner.extend(open.pop()),
            None => {
                nesting.quotes.toggled_by(c);
                if let Some(text) = open.last_mut() {
                    text.push(c);
                }
            }
        }
    }
    inner.extend(open.drain(1..));
    (open.pop().unwrap_or_default(), inner)
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

/// The guarded path a command outside the reader list names. klin cannot tell a write from a
/// mention, so it is an ask.
fn mentioned(token: &str) -> Option<Decision> {
    guarded(token).then(|| asked_about(token))
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

/// The quoting a character sits in, with the fresh context a command substitution opens and
/// the enclosing one it gives back when it closes.
#[derive(Default)]
struct Nesting {
    quotes: Quotes,
    outer: Vec<Quotes>,
}

impl Nesting {
    /// `Some(true)` where a command substitution opens, `Some(false)` where one closes, and
    /// `None` for a character of an argument. Opening one keeps the quoting it sits in, and
    /// closing one puts that quoting back.
    fn nested(&mut self, c: char, previous: char) -> Option<bool> {
        let step = self.punctuation(c, previous)?;
        if step {
            self.outer.push(std::mem::take(&mut self.quotes));
        } else {
            self.quotes = self.outer.pop().unwrap_or_default();
        }
        Some(step)
    }

    /// `Some(true)` for the punctuation that opens a substitution, `Some(false)` for the
    /// punctuation that closes one, `None` for a character of an argument. A single quote
    /// makes all of it literal. A backtick opens the first substitution and closes that one.
    fn punctuation(&self, c: char, previous: char) -> Option<bool> {
        if self.quotes.single {
            return None;
        }
        if (c == '(' && previous == '$') || (c == '`' && self.outer.is_empty()) {
            return Some(true);
        }
        (matches!(c, ')' | '`') && !self.outer.is_empty()).then_some(false)
    }
}

/// Where the line opens a heredoc, honouring the quoting the way `segments` does: a `<<`
/// inside an argument is two characters, not a redirect. A command substitution starts a
/// command of its own, so the `<<` in `--body "$(cat <<EOF"` opens a body the way a bare one
/// does, and the double quote is back in force after the substitution closes.
fn unquoted_heredoc(line: &str) -> Option<usize> {
    let mut nesting = Nesting::default();
    let mut open = None;
    let mut previous = ' ';
    for (at, c) in line.char_indices() {
        let nested = nesting.nested(c, previous).is_some();
        previous = c;
        if nested || nesting.quotes.toggled_by(c) {
            open = None;
            continue;
        }
        if c != '<' || nesting.quotes.any() {
            open = None;
            continue;
        }
        match open {
            Some(_) => return Some(at - 1),
            None => open = Some(at),
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
