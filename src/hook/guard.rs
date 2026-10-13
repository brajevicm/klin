use std::cell::OnceCell;
use std::path::{Component, Path, PathBuf};

use crate::hook::approval;
use crate::hook::copy;
use crate::hook::host::adapter::{Decision, Event};
use crate::hook::journal;
use crate::hook::lexer::{self, Op, Segment};
use crate::sys::{release, state};

/// Every wildcard a shell expands. A token holding one proves nothing about the file it stands
/// for, so the guard matches no path against it. ADR 0033.
const WILDCARDS: &[char] = &['*', '?', '['];
/// The mark of a parameter the shell expands. A shell token holding one proves nothing about
/// its file, while a tool's own path holds it as a character. ADR 0068.
const PARAMETER: char = '$';
/// The command word that moves the ground every relative path stands on.
const CD: &str = "cd";
/// Words that move the directory in a way the guard does not follow, wherever they stand.
const MOVERS: &[&str] = &["cd", "pushd", "popd", "source"];
/// Command words that run another file's shell in this one, which may move the directory.
const SOURCES: &[&str] = &[".", "source"];
/// Command words that run the builtin named after them.
const BUILTINS: &[&str] = &["builtin", "command"];
/// The command word that runs its words as shell text in this shell.
const EVAL: &str = "eval";
/// How deep the guard reads shell text nested in shell text. Text deeper than this is unread,
/// so a long chain of `eval` cannot make the work grow with its square.
const NESTING: u8 = 4;
/// The shells whose `-c` text, or whose heredoc, the guard reads as the command they run.
const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash", "ksh"];

const REFUSAL: &str = "klin: refused — this would change the configuration (klin.json). Fix \
    the code the gate names instead. Only a person changes it, in a reviewed commit.";

/// The agent's reason for a `klin.json` edit klin put to the person. It never holds the code,
/// because an agent that knew the code could relay it into a prompt. Spec 10.8.2.
const ASKED: &str = "klin: refused — this edit changes klin.json, so klin asked the person to \
    approve it. Make the same edit again only after the person approves it.";

const STATE_REFUSAL: &str = "klin: refused — this is klin's own record of the turn, under \
    .git/klin. It holds the window a gate failed in and the questions this turn already put to \
    you. No command moves it: fix what the gate names.";

const SETUP_REFUSAL: &str = "klin: refused — `klin setup` writes the configuration and \
    the host's hook files. Only a person runs it, in a reviewed commit.";
const UPDATE_REFUSAL: &str = "klin: refused — `klin update` replaces the binary that judges \
    this work. Only a person runs it. `klin status` names a newer release.";
const AGENT_REFUSAL: &str = "klin: refused — `klin __agent` answers the host's own hook \
    events, and the host runs it. Only the host runs it.";

/// klin's own subcommands that only a person runs, the reason each is refused, and the
/// hyphenated tag a journal line names the refusal by. Spec 10.8.
const KLIN_REFUSED: &[(&[&str], &str, &str)] = &[
    (&["setup"], SETUP_REFUSAL, "setup"),
    (&["update"], UPDATE_REFUSAL, "update"),
    (&[release::CHILD], UPDATE_REFUSAL, "update"),
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
    let state = || guarded.paths().state.as_deref();
    let claimed = || state().and_then(|at| state::claim(at, &event.identity));
    let answer = decided(&guarded, event);
    let refused = !matches!(answer, Answer::Decided(Decision::Allow, _));
    let mut claim = refused.then(claimed).flatten();
    let (decision, reason) = answer.decision(state(), claim.is_some());
    let delivered = event.host.decide(&decision);
    if !refused && delivered != 0 {
        claim = claimed();
    }
    if let (Some(_), Some(at)) = (&claim, state()) {
        let named = match refused {
            true => reason,
            false => HOST_REFUSAL,
        };
        journal::guard(at, event, delivered_as(&decision, delivered), named);
    }
    delivered
}

/// What the host delivered: an ask only where it asked, because a host that cannot ask refuses
/// instead, and a deny otherwise, whatever the exit code carried it. Spec 9.6, 11.4.
fn delivered_as(decision: &Decision, delivered: u8) -> &'static str {
    match (decision, delivered) {
        (Decision::Ask(_), 0) => "ask",
        _ => "deny",
    }
}

/// What the guard decides about a call before a copy of klin's hooks delivers it: a decision
/// and the hyphenated reason a journal line names it by, or an edit of `klin.json` the person
/// may approve, with the file as it is and as the edit leaves it. Spec 9.6, 10.8.2.
enum Answer {
    Decided(Decision, &'static str),
    Approval { current: String, result: String },
}

impl Answer {
    /// The decision a copy of klin's hooks delivers, `took` when it holds the call's claim.
    fn decision(self, at: Option<&Path>, took: bool) -> (Decision, &'static str) {
        match self {
            Answer::Decided(decision, reason) => (decision, reason),
            Answer::Approval { current, result } => referred(at, took, &current, &result),
        }
    }
}

/// The deny of a `klin.json` edit the person may approve. The copy of klin's hooks that took
/// the call records the request and tells the person its code. A copy that yields denies with
/// the same reason and tells nothing. A request klin cannot record is the refusal of an edit
/// with no approval. Spec 10.8.2.
fn referred(
    at: Option<&Path>,
    took: bool,
    current: &str,
    result: &str,
) -> (Decision, &'static str) {
    let code = took
        .then(|| at.and_then(|at| approval::requested(at, result)))
        .flatten();
    match code {
        Some(code) => (
            Decision::Refer {
                reason: ASKED.to_string(),
                person: approval::message(current, result, &code),
            },
            "config-approval",
        ),
        None if took => (Decision::Deny(REFUSAL.to_string()), "config-write"),
        None => (Decision::Deny(ASKED.to_string()), "config-approval"),
    }
}

/// The answer over the paths an edit call names: a deny for klin's own state, the person's
/// approval for `klin.json`, and when the call names neither, the decision over its command.
/// Section 9.4, spec 9.6, 10.8.2.
fn decided(guarded: &Guarded, event: &Event) -> Answer {
    let edits = !READ_TOOLS.contains(&event.tool.as_str());
    let named: Vec<Which> = match edits {
        true => event
            .file_paths
            .iter()
            .filter_map(|path| guarded.which(guarded.here(), path))
            .collect(),
        false => Vec::new(),
    };
    if named.iter().any(|which| matches!(which, Which::State)) {
        return Answer::Decided(Decision::Deny(STATE_REFUSAL.to_string()), "state-write");
    }
    if named.is_empty() {
        let (decision, reason) = command_decision(guarded, &event.command);
        return Answer::Decided(decision, reason);
    }
    guarded.edited(event)
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
            Decision::Deny(_) | Decision::Refer { .. } => return (decision, reason),
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

    /// An edit of `klin.json` the person may approve: the call names it once, and klin can
    /// compute the file the call leaves. The person's approval of that exact file allows it.
    /// Any other edit of it is denied with no approval. Spec 10.8.2.
    fn edited(&self, event: &Event) -> Answer {
        let refused = Answer::Decided(Decision::Deny(REFUSAL.to_string()), "config-write");
        let mut named = event
            .rewrites
            .iter()
            .filter(|(path, _)| matches!(self.which(self.here(), path), Some(Which::Config)));
        let (Some((_, rewrite)), None) = (named.next(), named.next()) else {
            return refused;
        };
        let Ok(current) = std::fs::read_to_string(&self.paths().config) else {
            return refused;
        };
        let Some(result) = rewrite.result(&current) else {
            return refused;
        };
        match self.paths().state.as_deref() {
            Some(at) if approval::allows(at, &result, &event.identity) => {
                Answer::Decided(Decision::Allow, "")
            }
            Some(_) => Answer::Approval { current, result },
            None => refused,
        }
    }

    /// What one path a redirect names is worth: a deny for the one file only a person changes,
    /// a deny for klin's own record of the turn, and nothing otherwise. Paired with the
    /// hyphenated reason a journal line names the deny by. Spec 9.6.
    fn denied(&self, base: Option<&Path>, path: &str) -> Option<(Decision, &'static str)> {
        let (why, reason) = match self.which(base, path)? {
            Which::Config => (REFUSAL, "config-write"),
            Which::State => (STATE_REFUSAL, "state-write"),
        };
        Some((Decision::Deny(why.to_string()), reason))
    }

    /// The guarded path a writer takes as an argument, which klin can prove the command
    /// writes. `klin.json` changes only through an edit the person approves, so its write is a
    /// deny. For klin's own record of the turn a person decides whether it goes through.
    /// Paired with the hyphenated reason a journal line names it by. Spec 9.6, 10.8.2.
    fn written(&self, base: Option<&Path>, token: &str) -> Option<(Decision, &'static str)> {
        Some(match self.which(base, token)? {
            Which::Config => (
                Decision::Deny(format!(
                    "klin: refused — this writes \"{token}\", the configuration klin guards, and \
                     no shell command changes it. Fix the code the gate names instead. If the \
                     person asked for this change, make it with the edit tool, and klin asks the \
                     person to approve it."
                )),
                "config-mention",
            ),
            Which::State => (
                Decision::Ask(format!(
                    "this names \"{token}\", which is klin's own record of the turn."
                )),
                "state-mention",
            ),
        })
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
    Some(real(&plain(&joined(base, token)?)))
}

/// The path a token names from `base`, and `None` for a relative one with no `base`.
fn joined(base: Option<&Path>, token: &str) -> Option<PathBuf> {
    let named = Path::new(token);
    match named.is_absolute() {
        true => Some(named.to_path_buf()),
        false => Some(base?.join(named)),
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

fn command_decision(guarded: &Guarded, command: &str) -> (Decision, &'static str) {
    let mut cwd = Cwd {
        at: guarded.here().map(Place::at),
        ..Cwd::default()
    };
    walk(guarded, command, &mut cwd, 0)
}

/// The strictest decision over every segment of a command, with the directory a static `cd`
/// moves carried from one segment to the next. Shell the guard does not read suppresses path
/// matching alone, and an open quote allows the whole command. Nested text the outer shell
/// expanded is unread wherever it holds a `$`, because the expansion may change how it splits,
/// and so is text nested deeper than `NESTING`. ADR 0033, ADR 0068.
fn walk(guarded: &Guarded, command: &str, cwd: &mut Cwd, depth: u8) -> (Decision, &'static str) {
    let Some(lexed) = lexer::lexed(command) else {
        cwd.set(None);
        return (Decision::Allow, "");
    };
    let proven =
        !lexed.unreadable && depth <= NESTING && !(depth > 0 && command.contains(PARAMETER));
    let mut decisions = Vec::new();
    let mut before = Op::List;
    for segment in &lexed.segments {
        cwd.enter(before);
        let at = cwd.physical();
        decisions.push(segment_decision(guarded, segment, at, proven));
        let changes = cwd.changes;
        let alone = cwd.alone(segment, before, lexed.grouped);
        if proven {
            decisions.extend(nested_decision(guarded, segment, cwd, depth));
        }
        let evaled = cwd.changes != changes;
        cwd.leave(segment, alone, evaled);
        before = segment.after;
    }
    if !proven {
        cwd.set(None);
    }
    strictest(decisions)
}

/// The directory relative paths resolve from, `None` once the guard cannot place it, whether a
/// command has moved it, whether the last segment was a `cd` the guard placed, and how many
/// times it was set, so a caller sees whether nested text changed it.
#[derive(Default)]
struct Cwd {
    at: Option<Place>,
    moved: bool,
    placed: bool,
    changes: u32,
}

/// One directory under its two names. `physical` is where a process stands, with every link
/// followed. `logical` is the shell's own name for it, which a `cd` without `-P` moves from, so
/// `..` leaves a link the way the `cd` came in.
#[derive(Clone)]
struct Place {
    physical: PathBuf,
    logical: PathBuf,
}

impl Place {
    /// A directory whose name holds no link.
    fn at(path: &Path) -> Place {
        Place {
            physical: path.to_path_buf(),
            logical: path.to_path_buf(),
        }
    }

    /// Where a `cd` from `from` lands: from the logical name by default, from the physical one
    /// under `-P`, and only at a directory that exists. An absolute `dir` needs no `from`.
    fn landing(from: Option<&Place>, dir: &str, physical: bool) -> Option<Place> {
        if physical {
            let base = from.map(|place| place.physical.as_path());
            let found = joined(base, dir)?.canonicalize().ok()?;
            return found.is_dir().then(|| Place::at(&found));
        }
        let base = from.map(|place| place.logical.as_path());
        let logical = plain(&joined(base, dir)?);
        let physical = logical.canonicalize().ok().filter(|at| at.is_dir())?;
        Some(Place { physical, logical })
    }
}

impl Cwd {
    fn set(&mut self, at: Option<Place>) {
        self.at = at;
        self.changes += 1;
    }

    fn physical(&self) -> Option<&Path> {
        self.at.as_ref().map(|place| place.physical.as_path())
    }

    /// A command after `||` runs when the one before failed, which may be the `cd`.
    fn enter(&mut self, before: Op) {
        if before == Op::Or && self.moved {
            self.set(None);
        }
    }

    /// A `cd`, or an `eval` that ran one, moves the directory only when it surely runs in this
    /// shell and the next command runs after it. A directory that is not there leaves the `cd`
    /// failing, so the guard cannot place what runs next.
    fn leave(&mut self, segment: &Segment, alone: bool, evaled: bool) {
        match moves(&segment.words) {
            Move::Stays if !evaled => {
                self.placed = false;
                return;
            }
            Move::Stays => {}
            Move::To { dir, physical } => {
                let place = Place::landing(self.at.as_ref(), dir, physical);
                self.set(place);
            }
            Move::Lost => self.set(None),
        }
        if !alone {
            self.set(None);
        }
        self.moved = true;
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
            ..Cwd::default()
        }
    }
}

/// What a segment does to the directory.
enum Move<'a> {
    Stays,
    /// A static `cd` to the directory it names, under `-P` when `physical`.
    To {
        dir: &'a str,
        physical: bool,
    },
    /// A move the guard does not follow.
    Lost,
}

fn moves(words: &[String]) -> Move<'_> {
    let Some((command, args)) = command(words) else {
        return Move::Stays;
    };
    if command == CD {
        return cd_target(args).map_or(Move::Lost, |(dir, physical)| Move::To { dir, physical });
    }
    let builtin = BUILTINS.contains(&command).then(|| args.first()).flatten();
    let sourced =
        SOURCES.contains(&command) || builtin.is_some_and(|word| SOURCES.contains(&word.as_str()));
    match sourced || words.iter().any(|word| MOVERS.contains(&word.as_str())) {
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

/// The one directory a `cd` names, and whether `-P` makes it follow links, where the last of
/// `-L` and `-P` wins.
fn cd_target(args: &[String]) -> Option<(&str, bool)> {
    let flags = args
        .iter()
        .take_while(|arg| matches!(arg.as_str(), "-L" | "-P" | "-e" | "-@" | "--"));
    let physical = flags
        .clone()
        .filter(|arg| matches!(arg.as_str(), "-L" | "-P"))
        .last()
        .is_some_and(|arg| arg == "-P");
    match &args[flags.count()..] {
        [dir] if !dir.starts_with(['-', '~']) && !dir.contains(PARAMETER) => {
            Some((dir.as_str(), physical))
        }
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
                    .flat_map(|word| guarded.written(cwd, word)),
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
    let look = |path: &str| resolved(cwd, path).and_then(|at| at.metadata().ok());
    match command {
        "cp" => copy::destinations(&copy::CP, args, look),
        "install" => copy::destinations(&copy::INSTALL, args, look),
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
    depth: u8,
) -> Option<(Decision, &'static str)> {
    let (command, args) = command(&segment.words)?;
    match command {
        EVAL => Some(walk(guarded, &args.join(" "), cwd, depth + 1)),
        command if SHELLS.contains(&command) => {
            let text = shell_text(args, segment.body.as_deref())?;
            Some(walk(guarded, text, &mut cwd.child(), depth + 1))
        }
        _ => None,
    }
}

/// The text a shell runs: the word after a `-c` flag, or the heredoc it reads when it names no
/// script. An option that takes an argument, or a long one, leaves the text unread.
fn shell_text<'a>(args: &'a [String], body: Option<&'a str>) -> Option<&'a str> {
    for (at, arg) in args.iter().enumerate() {
        let flags = arg
            .strip_prefix('-')
            .filter(|flags| !flags.is_empty() && flags.chars().all(|c| c.is_ascii_alphabetic()))?;
        if flags.contains(['o', 'O']) {
            return None;
        }
        if flags.contains('c') {
            return args.get(at + 1).map(String::as_str);
        }
    }
    body
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
