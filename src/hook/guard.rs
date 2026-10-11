use std::cell::OnceCell;
use std::path::{Component, Path, PathBuf};

use crate::hook::copy;
use crate::hook::host::adapter::{Decision, Event};
use crate::hook::journal;
use crate::hook::lexer::{self, Op, Segment};
use crate::sys::state;

/// Every wildcard a shell expands. A token holding one proves nothing about the file it stands
/// for, so the guard matches no path against it. ADR 0033.
const WILDCARDS: &[char] = &['*', '?', '['];
/// The mark of a parameter the shell expands. A shell token holding one proves nothing about
/// its file, while a tool's own path holds it as a character. ADR 0068.
const PARAMETER: char = '$';
/// The command word that moves the ground every relative path stands on.
const CD: &str = "cd";
/// Words that move the directory in a way the guard does not follow, wherever they stand.
const MOVERS: &[&str] = &["cd", "pushd", "popd"];
/// Command words that run another file's shell in this one, which may move the directory.
const SOURCES: &[&str] = &[".", "source"];
/// The command word that runs its words as shell text in this shell.
const EVAL: &str = "eval";
/// The shells whose `-c` text, or whose heredoc, the guard reads as the command they run.
const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash", "ksh"];

const REFUSAL: &str = "klin: refused — this would change the configuration (klin.json). Fix \
    the code the gate names instead. Only a person changes it, in a reviewed commit.";

const STATE_REFUSAL: &str = "klin: refused — this is klin's own record of the turn, under \
    .git/klin. It holds the window a gate failed in and the questions this turn already put to \
    you. No command moves it: fix what the gate names.";

const SETUP_REFUSAL: &str = "klin: refused — `klin setup` writes the configuration and \
    the host's hook files. Only a person runs it, in a reviewed commit.";
const UPDATE_REFUSAL: &str = "klin: refused — `klin update` replaces the binary that judges \
    this work. Only a person runs it.";
const AGENT_REFUSAL: &str = "klin: refused — `klin __agent` answers the host's own hook \
    events, and the host runs it. Only the host runs it.";

/// klin's own subcommands that only a person runs, the reason each is refused, and the
/// hyphenated tag a journal line names the refusal by. Spec 10.8.
const KLIN_REFUSED: &[(&[&str], &str, &str)] = &[
    (&["setup"], SETUP_REFUSAL, "setup"),
    (&["update"], UPDATE_REFUSAL, "update"),
    (&["__agent"], AGENT_REFUSAL, "agent"),
];

/// The journal's name for a call the guard allowed and the host refused anyway, which is a host
/// that answers every call the same way: a custom integration on a protocol version klin does not
/// speak (9.7). The line is the only record a person has of why the agent is blocked. Spec 9.6.
const HOST_REFUSAL: &str = "host-refusal";

const READ_TOOLS: &[&str] = &["Read", "NotebookRead"];
/// The one file klin guards, beside klin's own state directory. A check's own configuration, a
/// host's hook file and the code owners are ordinary files: klin cannot tell a loosening from a
/// fix in any of them, and refusing a whole settings file refuses the work that has nothing to
/// do with klin. ADR 0027, ADR 0032.
const NAME: &str = "klin.json";
/// The commands that write every argument they take, so a guarded path among the arguments is a
/// write under any reading. `cp` and `install` write their destination alone, which the copy
/// reader finds. ADR 0033, ADR 0068.
const WRITERS: &[&str] = &["rm", "rmdir", "unlink", "shred", "mv", "truncate", "tee"];
/// The two that write the file they are given, and only under `-i`.
const IN_PLACE: &[&str] = &["sed", "perl"];

/// A `pre_tool` event of the agent ingress, in the worktree the opt-in walk found. Every copy of
/// klin's hooks the host runs for one call gives the same answer, so a copy that yields still
/// refuses: a call's identity that matched another call would otherwise open it. Only the copy
/// that took the call journals it. Spec 9.8, 10.8.
pub fn run(event: &Event, root: &Path) -> u8 {
    let guarded = Guarded::at(event.root.clone(), root.to_path_buf());
    let (decision, reason) = decided(&guarded, event);
    let delivered = event.host.decide(&decision);
    let refused = !matches!(decision, Decision::Allow);
    if (refused || delivered != 0)
        && let Some(at) = guarded.paths().state.as_deref()
        && let Some(_claim) = state::claim(at, &event.identity)
    {
        let named = match refused {
            true => reason,
            false => HOST_REFUSAL,
        };
        journal::guard(at, event, delivered, named);
    }
    delivered
}

/// The strictest decision over every path an edit call names, and when none of them decides
/// anything, the decision over its command, paired with the hyphenated reason a journal line
/// names it by. Section 9.4, spec 9.6.
fn decided(guarded: &Guarded, event: &Event) -> (Decision, &'static str) {
    let edits = !READ_TOOLS.contains(&event.tool.as_str());
    let paths = edits.then(|| {
        strictest(
            event
                .file_paths
                .iter()
                .flat_map(|path| guarded.denied(guarded.here(), path)),
        )
    });
    match paths {
        Some((Decision::Allow, _)) | None => command_decision(guarded, &event.command),
        Some(decided) => decided,
    }
}

/// The strictest `(decision, reason)` pair over a set, by section 9.4's priority — a deny
/// anywhere, otherwise the first ask, otherwise allow — so the reason always names the decision
/// the guard actually gave. The public `Decision` stays as it is; only this private pairing
/// carries the reason. Spec 9.6.
fn strictest(
    decisions: impl IntoIterator<Item = (Decision, &'static str)>,
) -> (Decision, &'static str) {
    let mut asked = None;
    for (decision, reason) in decisions {
        match decision {
            Decision::Deny(_) => return (decision, reason),
            Decision::Ask(_) => {
                asked.get_or_insert((decision, reason));
            }
            Decision::Allow => {}
        }
    }
    asked.unwrap_or((Decision::Allow, ""))
}

/// What klin guards in this tree: the configuration beside the tree root, and the state
/// directory `state::dir` resolves. Both are found when the first path needs proving and not
/// before, so a tool call that names none never reaches for git. Section 9.4.
struct Guarded {
    /// The tree the host's event named, for a host that does not run its hooks in it.
    named: Option<PathBuf>,
    /// The worktree root the opt-in walk found.
    root: PathBuf,
    /// Where the guard runs, which is what a relative path in a command names from.
    here: OnceCell<Option<PathBuf>>,
    paths: OnceCell<Paths>,
}

struct Paths {
    config: PathBuf,
    state: Option<PathBuf>,
}

/// Which guarded thing a path names.
enum Which {
    Config,
    State,
}

impl Guarded {
    fn at(named: Option<PathBuf>, root: PathBuf) -> Guarded {
        Guarded {
            named,
            root,
            here: OnceCell::new(),
            paths: OnceCell::new(),
        }
    }

    /// The tree the event named, or the one the guard runs in.
    fn here(&self) -> Option<&Path> {
        self.here
            .get_or_init(|| {
                let at = match &self.named {
                    Some(root) => root.clone(),
                    None => std::env::current_dir().ok()?,
                };
                Some(real(&at))
            })
            .as_deref()
    }

    fn paths(&self) -> &Paths {
        self.paths.get_or_init(|| Paths::at(&self.root))
    }

    /// Which guarded thing a path names from `base`, where a relative path needs a `base` the
    /// guard can place.
    fn which(&self, base: Option<&Path>, path: &str) -> Option<Which> {
        let at = resolved(base, path)?;
        let paths = self.paths();
        if at == paths.config {
            return Some(Which::Config);
        }
        let state = paths.state.as_ref()?;
        at.starts_with(state).then_some(Which::State)
    }

    /// What one path an edit tool or a redirect names is worth: a deny for the one file only a
    /// person changes, a deny for klin's own record of the turn, and nothing otherwise. Paired
    /// with the hyphenated reason a journal line names the deny by. Spec 9.6.
    fn denied(&self, base: Option<&Path>, path: &str) -> Option<(Decision, &'static str)> {
        let (why, reason) = match self.which(base, path)? {
            Which::Config => (REFUSAL, "config-write"),
            Which::State => (STATE_REFUSAL, "state-write"),
        };
        Some((Decision::Deny(why.to_string()), reason))
    }

    /// The guarded path a writer takes as an argument. klin can prove the write, and a person
    /// decides whether it goes through. Paired with the hyphenated reason a journal line names
    /// the ask by. Spec 9.6.
    fn asked(&self, base: Option<&Path>, token: &str) -> Option<(Decision, &'static str)> {
        let (what, reason) = match self.which(base, token)? {
            Which::Config => ("the configuration klin guards", "config-mention"),
            Which::State => ("klin's own record of the turn", "state-mention"),
        };
        Some((
            Decision::Ask(format!("this names \"{token}\", which is {what}.")),
            reason,
        ))
    }
}

impl Paths {
    fn at(root: &Path) -> Paths {
        Paths {
            config: real(&root.join(NAME)),
            state: state::dir(root).map(|at| real(&at)),
        }
    }
}

/// The file a token names from `base`, and `None` for a token that proves nothing about any
/// file: one a shell expands, or a relative one where the guard cannot place the directory.
fn resolved(base: Option<&Path>, token: &str) -> Option<PathBuf> {
    if token.is_empty() || token.contains(WILDCARDS) {
        return None;
    }
    let named = Path::new(token);
    let at = match named.is_absolute() {
        true => named.to_path_buf(),
        false => base?.join(named),
    };
    Some(real(&plain(&at)))
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

fn command_decision(guarded: &Guarded, command: &str) -> (Decision, &'static str) {
    let mut cwd = Cwd {
        at: guarded.here().map(Path::to_path_buf),
        moved: false,
        placed: false,
    };
    walk(guarded, command, &mut cwd, false)
}

/// The strictest decision over every segment of a command, with the directory a static `cd`
/// moves carried from one segment to the next. Shell the guard does not read suppresses path
/// matching alone, and an open quote allows the whole command. Nested text the outer shell
/// expanded is unread wherever it holds a `$`, because the expansion may change how it splits.
/// ADR 0033, ADR 0068.
fn walk(guarded: &Guarded, command: &str, cwd: &mut Cwd, nested: bool) -> (Decision, &'static str) {
    let Some(lexed) = lexer::lexed(command) else {
        cwd.at = None;
        return (Decision::Allow, "");
    };
    let proven = !lexed.unreadable && !(nested && command.contains('$'));
    let mut decisions = Vec::new();
    let mut before = Op::List;
    for segment in &lexed.segments {
        cwd.enter(before);
        let at = cwd.at.as_deref();
        decisions.push(segment_decision(guarded, segment, at, proven));
        let start = cwd.at.clone();
        let alone = cwd.alone(segment, before, lexed.grouped);
        if proven {
            decisions.extend(nested_decision(guarded, segment, cwd));
        }
        let evaled = cwd.at != start;
        cwd.leave(segment, alone, evaled);
        before = segment.after;
    }
    if !proven {
        cwd.at = None;
    }
    strictest(decisions)
}

/// The directory relative paths resolve from, `None` once the guard cannot place it, whether a
/// command has moved it, and whether the last segment was a `cd` the guard placed.
#[derive(Clone)]
struct Cwd {
    at: Option<PathBuf>,
    moved: bool,
    placed: bool,
}

impl Cwd {
    /// A command after `||` runs when the one before failed, which may be the `cd`.
    fn enter(&mut self, before: Op) {
        if before == Op::Or && self.moved {
            self.at = None;
        }
    }

    /// A `cd`, or an `eval` that ran one, moves the directory only when it surely runs in this
    /// shell and the next command runs after it. A directory that is not there leaves the `cd`
    /// failing, so the guard cannot place what runs next.
    fn leave(&mut self, segment: &Segment, alone: bool, evaled: bool) {
        let target = match moves(&segment.words) {
            Move::Stays if !evaled => {
                self.placed = false;
                return;
            }
            Move::Stays => self.at.clone(),
            Move::To(dir) => resolved(self.at.as_deref(), dir).filter(|at| at.is_dir()),
            Move::Lost => None,
        };
        self.moved = true;
        self.at = target.filter(|_| alone);
        self.placed = self.at.is_some();
    }

    /// The segment runs in this shell whenever the command reaches it: not behind an `&&` whose
    /// left side may fail, not in a pipe, a background job or a group, and not before `||`.
    fn alone(&self, segment: &Segment, before: Op, grouped: bool) -> bool {
        let runs =
            matches!(before, Op::List | Op::Background) || (before == Op::And && self.placed);
        !grouped && runs && matches!(segment.after, Op::List | Op::And)
    }

    /// A child shell starts where this one stands, and its `cd` stays its own.
    fn child(&self) -> Cwd {
        Cwd {
            at: self.at.clone(),
            moved: false,
            placed: false,
        }
    }
}

/// What a segment does to the directory.
enum Move<'a> {
    Stays,
    /// A static `cd` to the directory it names.
    To(&'a str),
    /// A move the guard does not follow.
    Lost,
}

fn moves(words: &[String]) -> Move<'_> {
    let Some((command, args)) = command(words) else {
        return Move::Stays;
    };
    if command == CD {
        return cd_target(args).map_or(Move::Lost, Move::To);
    }
    match SOURCES.contains(&command) || words.iter().any(|word| MOVERS.contains(&word.as_str())) {
        true => Move::Lost,
        false => Move::Stays,
    }
}

/// The command word by its basename, found behind the prefixes of `a_prefix`, and the words
/// after it.
fn command(words: &[String]) -> Option<(&str, &[String])> {
    let at = words.iter().position(|word| !a_prefix(word))?;
    Some((basename(&words[at]), &words[at + 1..]))
}

fn cd_target(args: &[String]) -> Option<&str> {
    let operands: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .skip_while(|arg| matches!(*arg, "-L" | "-P" | "-e" | "-@" | "--"))
        .collect();
    match operands.as_slice() {
        [dir] if !dir.starts_with(['-', '~']) && !dir.contains(PARAMETER) => Some(dir),
        _ => None,
    }
}

fn segment_decision(
    guarded: &Guarded,
    segment: &Segment,
    cwd: Option<&Path>,
    proven: bool,
) -> (Decision, &'static str) {
    if let Some((why, reason)) = a_persons_command(&segment.words) {
        return (Decision::Deny(why.to_string()), reason);
    }
    if !proven {
        return (Decision::Allow, "");
    }
    let written = written_by(&segment.words, cwd);
    strictest(
        segment
            .targets
            .iter()
            .filter(|target| !target.contains(PARAMETER))
            .flat_map(|target| guarded.denied(cwd, target))
            .chain(
                written
                    .iter()
                    .filter(|word| !word.contains(PARAMETER))
                    .flat_map(|word| guarded.asked(cwd, word)),
            ),
    )
}

/// The files a command writes. The command word is read by its basename and found behind the
/// same prefixes klin's own name is found behind, so a script of the tree's own named `rm` is
/// read as `rm`. ADR 0033, ADR 0068.
fn written_by(words: &[String], cwd: Option<&Path>) -> Vec<String> {
    let Some((command, args)) = command(words) else {
        return Vec::new();
    };
    let is_dir = |path: &str| resolved(cwd, path).is_some_and(|at| at.is_dir());
    match command {
        "cp" => copy::destinations(&copy::CP, args, is_dir),
        "install" => copy::destinations(&copy::INSTALL, args, is_dir),
        command if IN_PLACE.contains(&command) => match args.iter().any(|w| w.starts_with("-i")) {
            true => args.to_vec(),
            false => Vec::new(),
        },
        command if WRITERS.contains(&command) => args.to_vec(),
        _ => Vec::new(),
    }
}

/// The decision over shell text a segment runs: `eval`'s words in this shell, and a shell's
/// `-c` text or heredoc in a child. ADR 0068.
fn nested_decision(
    guarded: &Guarded,
    segment: &Segment,
    cwd: &mut Cwd,
) -> Option<(Decision, &'static str)> {
    let (command, args) = command(&segment.words)?;
    match command {
        EVAL => Some(walk(guarded, &args.join(" "), cwd, true)),
        command if SHELLS.contains(&command) => {
            let text = shell_text(args, segment.body.as_deref())?;
            Some(walk(guarded, &text, &mut cwd.child(), true))
        }
        _ => None,
    }
}

/// The text a shell runs: the word after a `-c` flag, or the heredoc it reads when it names no
/// script. An option that takes an argument, or a long one, leaves the text unread.
fn shell_text(args: &[String], body: Option<&str>) -> Option<String> {
    for (at, arg) in args.iter().enumerate() {
        let flags = arg
            .strip_prefix('-')
            .filter(|flags| !flags.is_empty() && flags.chars().all(|c| c.is_ascii_alphabetic()))?;
        if flags.contains(['o', 'O']) {
            return None;
        }
        if flags.contains('c') {
            return args.get(at + 1).cloned();
        }
    }
    body.map(str::to_owned)
}

/// One of klin's own subcommands that only a person runs, whatever flags it carries, paired
/// with the hyphenated reason a journal line names the refusal by.
fn a_persons_command(words: &[String]) -> Option<(&'static str, &'static str)> {
    let at = words.iter().position(|word| {
        basename(word).trim_end_matches(".exe") == "klin" && !word.starts_with('-')
    })?;
    if words[..at].iter().any(|word| !a_prefix(word)) {
        return None;
    }
    let rest: Vec<&str> = words[at + 1..]
        .iter()
        .map(String::as_str)
        .filter(|word| !word.starts_with('-'))
        .collect();
    KLIN_REFUSED
        .iter()
        .find(|(command, _, _)| rest.starts_with(command))
        .map(|(_, why, reason)| (*why, *reason))
}

/// What may stand in front of a command without changing which command it is: an assignment,
/// a command substitution, `env` and its assignments, and a runner that passes the rest through.
fn a_prefix(word: &str) -> bool {
    word.contains('=')
        || word.contains("$(")
        || word.contains('`')
        || ["env", "npx", "pnpm", "bunx", "time", "nice", "sudo"].contains(&word)
}
