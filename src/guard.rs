use std::cell::OnceCell;
use std::path::{Component, Path, PathBuf};

use crate::host::{self, Decision, Event};
use crate::state;

const SPLIT: char = '\u{0}';
/// The mark an unquoted `>` leaves in the flattened command. A `>` inside an argument keeps its
/// own character, so a commit message that holds one cannot read as a redirect.
const REDIRECT: &str = "\u{1}";
const SEPARATORS: &[char] = &[';', '&', '|', '\n'];
/// Every wildcard a shell expands. A token holding one proves nothing about the file it stands
/// for, so the guard matches no path against it. ADR 0033.
const WILDCARDS: &[char] = &['*', '?', '['];
/// The shell klin does not read. A command holding any of these leaves every path it names
/// unproven, so the guard matches none of them and answers `allow`. ADR 0033.
const UNREADABLE: &[&str] = &["$(", "`", "${", "<<", "\\"];
/// The command word that moves the ground every relative path stands on.
const CD: &str = "cd";

const REFUSAL: &str = "klin: refused — this would change the configuration (klin.json). Fix \
    the code the gate names instead. Only a person changes it, in a reviewed commit.";

const STATE_REFUSAL: &str = "klin: refused — this is klin's own record of the turn, under \
    .git/klin. It holds the window a gate failed in and the questions this turn already put to \
    you. Only a person moves it, with `klin turn reset`.";

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
/// The one file klin guards, beside klin's own state directory. A check's own configuration, a
/// host's hook file and the code owners are ordinary files: klin cannot tell a loosening from a
/// fix in any of them, and refusing a whole settings file refuses the work that has nothing to
/// do with klin. ADR 0027, ADR 0032.
const NAME: &str = "klin.json";
/// The commands that write every argument they take, so a guarded path among the arguments is a
/// write under any reading. `cp` and `install` read their first argument, so neither is here.
/// ADR 0033.
const WRITERS: &[&str] = &["rm", "rmdir", "unlink", "shred", "mv", "truncate", "tee"];
/// The two that write the file they are given, and only under `-i`.
const IN_PLACE: &[&str] = &["sed", "perl"];

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
    event.host.decide(&decided(&event))
}

/// The strictest decision over every path an edit call names, and when none of them decides
/// anything, the decision over its command. Section 9.4.
fn decided(event: &Event) -> Decision {
    let guarded = Guarded::default();
    let edits = !READ_TOOLS.contains(&event.tool.as_str());
    let paths = edits.then(|| {
        Decision::strictest(
            event
                .file_paths
                .iter()
                .flat_map(|path| guarded.denied(path)),
        )
    });
    match paths {
        Some(Decision::Allow) | None => command_decision(&guarded, &event.command),
        Some(decision) => decision,
    }
}

/// What klin guards in this tree: the configuration beside the tree root, and the state
/// directory `state::dir` resolves. Both are found when the first path needs proving and not
/// before, so a tool call that names none never reaches for git. Section 9.4.
#[derive(Default)]
struct Guarded(OnceCell<Option<Paths>>);

struct Paths {
    /// Where the guard runs, which is what a relative path in a command names from.
    here: PathBuf,
    config: PathBuf,
    state: Option<PathBuf>,
}

/// Which guarded thing a path names.
enum Which {
    Config,
    State,
}

impl Guarded {
    fn paths(&self) -> Option<&Paths> {
        self.0.get_or_init(Paths::here).as_ref()
    }

    fn which(&self, path: &str) -> Option<Which> {
        let paths = self.paths()?;
        let at = paths.resolved(path)?;
        if at == paths.config {
            return Some(Which::Config);
        }
        let state = paths.state.as_ref()?;
        at.starts_with(state).then_some(Which::State)
    }

    /// What one path an edit tool or a redirect names is worth: a deny for the one file only a
    /// person changes, a deny for klin's own record of the turn, and nothing otherwise.
    fn denied(&self, path: &str) -> Option<Decision> {
        let why = match self.which(path)? {
            Which::Config => REFUSAL,
            Which::State => STATE_REFUSAL,
        };
        Some(Decision::Deny(why.to_string()))
    }

    /// The guarded path a writer takes as an argument. klin can prove the write, and a person
    /// decides whether it goes through.
    fn asked(&self, token: &str) -> Option<Decision> {
        let what = match self.which(token)? {
            Which::Config => "the configuration klin guards",
            Which::State => "klin's own record of the turn",
        };
        Some(Decision::Ask(format!(
            "this names \"{token}\", which is {what}."
        )))
    }
}

impl Paths {
    fn here() -> Option<Paths> {
        let here = real(&std::env::current_dir().ok()?);
        let root = crate::config::repository(&here).unwrap_or_else(|| here.clone());
        Some(Paths {
            config: real(&root.join(NAME)),
            state: state::dir(&root).map(|at| real(&at)),
            here,
        })
    }

    /// The file a token names, from where the guard runs, and `None` for a token that proves
    /// nothing about any file.
    fn resolved(&self, token: &str) -> Option<PathBuf> {
        if token.is_empty() || token.contains(WILDCARDS) {
            return None;
        }
        let named = Path::new(token);
        let at = match named.is_absolute() {
            true => named.to_path_buf(),
            false => self.here.join(named),
        };
        Some(real(&plain(&at)))
    }
}

/// The path with `.` and `..` taken out, which `canonicalize` cannot do for a file that is not
/// there yet.
fn plain(at: &Path) -> PathBuf {
    let mut kept = PathBuf::new();
    for part in at.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                kept.pop();
            }
            other => kept.push(other),
        }
    }
    kept
}

/// The path with every symbolic link its existing part carries resolved, so a name a host wrote
/// through a link matches the one git prints. The part that is not there yet stays as it is.
fn real(at: &Path) -> PathBuf {
    if let Ok(found) = at.canonicalize() {
        return found;
    }
    match (at.parent(), at.file_name()) {
        (Some(parent), Some(name)) => real(parent).join(name),
        _ => at.to_path_buf(),
    }
}

fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn command_decision(guarded: &Guarded, command: &str) -> Decision {
    let Some(segments) = segments(command) else {
        return Decision::Allow;
    };
    let words: Vec<Vec<&str>> = segments
        .iter()
        .map(|segment| segment.split_whitespace().collect())
        .collect();
    let proven = !UNREADABLE.iter().any(|shell| command.contains(shell))
        && !words.iter().any(|words| words.first() == Some(&CD));
    Decision::strictest(
        words
            .iter()
            .map(|words| segment_decision(guarded, words, proven)),
    )
}

fn segment_decision(guarded: &Guarded, words: &[&str], proven: bool) -> Decision {
    if let Some(why) = a_persons_command(words) {
        return Decision::Deny(why.to_string());
    }
    if !proven {
        return Decision::Allow;
    }
    let written = written_by(words).unwrap_or_default();
    Decision::strictest(
        redirect_targets(words)
            .into_iter()
            .flat_map(|target| guarded.denied(target))
            .chain(written.iter().flat_map(|word| guarded.asked(word))),
    )
}

/// The arguments a command writes, and `None` for a command that writes none of them. The
/// command word is read by its basename and found behind the same prefixes klin's own name is
/// found behind, so a script of the tree's own named `rm` is read as `rm`. ADR 0033.
fn written_by<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    let at = words.iter().position(|word| !a_prefix(word))?;
    let command = basename(words[at]);
    let writes = match IN_PLACE.contains(&command) {
        true => words.iter().any(|word| word.starts_with("-i")),
        false => WRITERS.contains(&command),
    };
    writes.then(|| &words[at + 1..])
}

/// The same separators a shell honours, with the quoting honoured and the quotes taken out. A
/// single quote makes everything literal, and a double quote makes a separator literal. `None`
/// when a quote is left open, because then the command does not say where its arguments end.
fn segments(command: &str) -> Option<Vec<String>> {
    let mut flat = String::with_capacity(command.len());
    let mut quotes = Quotes::default();
    for c in command.chars() {
        if quotes.toggled_by(c) {
            continue;
        }
        if quotes.any() {
            flat.push(c);
        } else if SEPARATORS.contains(&c) {
            flat.push(SPLIT);
        } else if c == '>' {
            flat.push(' ');
            flat.push_str(REDIRECT);
            flat.push(' ');
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
/// a command substitution, `env` and its assignments, and a runner that passes the rest through.
fn a_prefix(word: &str) -> bool {
    word.contains('=')
        || word.contains("$(")
        || word.contains('`')
        || ["env", "npx", "pnpm", "bunx", "time", "nice", "sudo"].contains(&word)
}

fn redirect_targets<'a>(words: &[&'a str]) -> Vec<&'a str> {
    words
        .iter()
        .enumerate()
        .filter(|(_, word)| **word == REDIRECT)
        .filter_map(|(at, _)| words.get(at + 1).copied())
        .collect()
}
