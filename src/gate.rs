use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::base::{self, Kind, Prior, Window};
use crate::changed::Change;
use crate::check::contract::Class;
use crate::check::contract::{
    self, Activation, Caller, Context, DELETED, DERIVATION, Hole, Incomplete, Plain, Reason,
    Records, Sink, Told, UNBUILT, UNRESOLVED,
};
use crate::check::holes::{self, MEASUREMENT_LOST, Seen, Unmeasured};
use crate::check::{catalogue, render};
use crate::config;
use crate::error::Error;
use crate::host;
use crate::host::adapter::{Event, Stop};
use crate::project::Project;
use crate::syntax::{LanguageId, structural};
use crate::{build, handoff, journal, reference, stamp, state, stats, survey, turn, write};

/// Where klin records what one prompt already spent, so the stop that follows knows how many
/// build blocks and gate blocks are left. In the state directory, which an agent does not
/// empty. ADR 0019, ADR 0022, ADR 0052.
const BUILD_BLOCKED: &str = "build-blocked";
/// The index the build stamp hashes the tree through, apart from the turn stamp's own.
const BUILD_INDEX: &str = "build-index";
/// How many stops one prompt's build failures may block. klin bounds this itself, because the
/// host documents no cap of its own. ADR 0022, spec 9.3.
const BLOCKS: u64 = 8;
/// How many stops one prompt's gate failures may block. The second needs a tree that changed
/// since the first. ADR 0052, spec 9.3.
const GATE_BLOCKS: u64 = 2;
/// What `--list` indents a gate's own lines by, under the row that names it.
const UNDER: &str = "      ";
/// The `ERR` row of 11.1 as `--json` names it, which a run that could not measure prints
/// whatever exit code it ends with.
const ERROR: &str = "ERROR";
const HOOK_REPORT: &str = "KLIN_HOOK_REPORT";
/// A stop's run decided to block. The host's own exit code for a block is `block_exit`, which
/// the stop ends with. Spec 9.1.
const BLOCKED: u8 = 2;

struct Gate {
    name: String,
    check: &'static catalogue::Row,
}

#[derive(Default)]
struct Plan {
    gates: Vec<Gate>,
    excluded: Vec<String>,
    /// The checks klin offers that neither the config nor the survey supplies a section for.
    /// Each needs a section a person writes, and none of them runs.
    needs_a_section: Vec<&'static catalogue::Row>,
}

impl Plan {
    /// The one gate a check runs as when its section is not a list of named entries.
    fn one(&mut self, check: &'static catalogue::Row) {
        self.gates.push(Gate {
            name: check.name.to_string(),
            check,
        });
    }
}

/// What one run of the runner is asked: by `check`, by `policy`, or by the Stop hook.
#[derive(Default)]
struct Args {
    config: Option<PathBuf>,
    strict: bool,
    gates: Vec<String>,
    list: bool,
    entry: Option<String>,
    changed: bool,
    hook: bool,
    json: bool,
    host: Option<String>,
}

#[derive(clap::Args)]
pub struct Check {
    /// Run only these checks, by the name a gate takes
    #[arg(value_name = "CHECK")]
    checks: Vec<String>,
    /// Judge only the files changed against the base
    #[arg(long)]
    changed: bool,
    /// Print one JSON object for the run instead of the human report
    #[arg(long)]
    json: bool,
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(clap::Args)]
pub struct Policy {
    /// Explain only this check, by the name a gate takes
    section: Option<String>,
    /// Explain only this entry of the check, such as one convention
    #[arg(requires = "section")]
    entry: Option<String>,
    /// Print the configuration reference, as Markdown, from the keys the checks declare
    #[arg(long, conflicts_with_all = ["section", "schema", "json", "config"])]
    reference: bool,
    /// Print the JSON Schema of klin.json
    #[arg(long, conflicts_with_all = ["section", "json", "config"])]
    schema: bool,
    /// Print one JSON object instead of the text
    #[arg(long)]
    json: bool,
    /// The klin.json to read (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
}

/// The Stop hook line `klin gate --hook --changed` that `setup` writes, which the agent ingress
/// replaces. It is no public command, so only an invocation that names `--hook` reaches it.
#[derive(clap::Parser)]
#[command(name = "klin gate")]
pub struct Hook {
    #[arg(long, required = true)]
    hook: bool,
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    host: Option<String>,
}

impl Hook {
    pub fn called() -> Option<Hook> {
        let mut words = std::env::args_os().skip(1);
        let gate = words.next().is_some_and(|word| word == "gate");
        (gate && words.any(|word| word == "--hook"))
            .then(|| <Hook as clap::Parser>::parse_from(std::env::args_os().skip(1)))
    }
}

pub fn check(check: &Check, start: &Path, out: &mut String) -> Result<u8, Error> {
    let args = Args {
        config: check.config.clone(),
        strict: true,
        gates: check.checks.clone(),
        changed: check.changed,
        json: check.json,
        ..Args::default()
    };
    run(&args, start, out)
}

pub fn policy(policy: &Policy, start: &Path, out: &mut String) -> Result<u8, Error> {
    if policy.reference || policy.schema {
        return reference::run(policy.schema, out);
    }
    let args = Args {
        config: policy.config.clone(),
        gates: policy.section.iter().cloned().collect(),
        entry: policy.entry.clone(),
        list: true,
        json: policy.json,
        ..Args::default()
    };
    run(&args, start, out)
}

pub fn hooked(hook: &Hook, start: &Path, out: &mut String) -> Result<u8, Error> {
    let args = Args {
        config: hook.config.clone(),
        changed: hook.changed,
        hook: hook.hook,
        json: hook.json,
        host: hook.host.clone(),
        ..Args::default()
    };
    run(&args, start, out)
}

fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let event = args
        .hook
        .then(|| host::read(args.host.as_deref()))
        .flatten();
    let start = event
        .as_ref()
        .and_then(|event| event.root.as_deref())
        .unwrap_or(start);
    if args.hook && !config::present(args.config.as_deref(), start) {
        return Ok(0);
    }
    let identity = event.as_ref().map_or("", |event| event.identity.as_str());
    let Some(_claim) = state::claimed(start, identity) else {
        return Ok(0);
    };
    let loaded = Project::load(args.config.as_deref(), start, &catalogue::sections());
    if !args.hook && !args.list {
        return Ok(checked(args, start, loaded, out));
    }
    if args.list {
        let judged = loaded.and_then(|mut project| by_hand(args, &mut project, out));
        return refused(args, judged, out).map(|tally| code(&tally));
    }
    match loaded {
        Ok(mut project) => Ok(stopped(args, &mut project, event, out)),
        Err(problem) => {
            eprintln!(
                "klin: FAIL: {problem} — only a person edits that file, so this stop is not \
                 blocked."
            );
            Ok(1)
        }
    }
}

fn by_hand(args: &Args, project: &mut Project, out: &mut String) -> Result<Tally, Error> {
    let window = base::choose(project.root(), args.strict).ok();
    if let Some(window) = &window {
        project.bind(window);
    }
    judge(args, project, window.as_ref(), &[], None, out)
}

/// How long a stop waits for the stop before it to finish. A fraction of the hook's five
/// seconds, because the stop still has a build and every gate to run inside them. Spec 13.
const BUDGET: Duration = Duration::from_secs(1);

/// One stop in the hook: the lock, the turn window, the build, the gates, and the verdict the
/// next prompt reads. The lock is held from before the run measures until after the verdict is
/// written, so an older stop cannot leave green over a newer red. Spec 6.5, 16.3.
fn stopped(args: &Args, project: &mut Project, event: Option<Event>, out: &mut String) -> u8 {
    let begun = std::time::Instant::now();
    let root = &project.root().to_path_buf();
    let mut log = journal::Stop::begun(event.as_ref(), config_hash(project));
    let (lock, lock_ms) =
        journal::timed(|| state::ready(root).ok().map(|at| state::lock(&at, BUDGET)));
    log.timing.lock_ms = lock_ms;
    let lost = matches!(&lock, Some(None));
    let window = turn::window(root, lost, &mut log.flags, out).ok();
    if let Some(window) = &window {
        project.bind(window);
    }
    let project = &*project;
    opened(root, lost, &mut log);
    if matches!(&window, Some(window) if matches!(window.kind, Kind::Branch)) {
        log.flags.push("branch-fallback");
    }
    let (code, green, asked, note) = ran(
        args,
        project,
        window.as_ref(),
        event.as_ref(),
        lost,
        &mut log,
        out,
    );
    let teardown = project.teardown_base();
    log.timing.base_remove_ms = journal::millis(teardown.remove);
    log.timing.base_prune_ms = journal::millis(teardown.prune);
    let exit = exit_code(code, event.as_ref());
    if let Some(Value::Object(report)) = &mut log.report {
        report.insert("exit".into(), exit.into());
        if let Some(window) = &window {
            report.entry("window").or_insert_with(|| window.record());
        }
    }
    log.blocked = code == BLOCKED;
    written(root, lost, green, asked.as_deref(), &mut log);
    log.asked = asked.unwrap_or_default();
    if let Ok(at) = state::ready(root) {
        let held = count(&at, &log);
        log.gate_blocks = held.gate_blocks;
        log.build_blocks = held.builds;
        log.prompt = held.prompt;
    }
    let said = tell(args, root, code, note, &mut log)
        .filter(|said| !keeps_quiet(root, event.as_ref(), lost, said, &mut log));
    observe_hook_report(log.report.as_ref());
    log.timing.total_ms = journal::millis(begun.elapsed());
    journal::stop(root, &log);
    if let Some(said) = said {
        host::answering(event.as_ref()).stop(&Stop::Tell(said));
    }
    exit
}

/// Whether this stop keeps its told message to itself. A host that submits a told message as
/// its next prompt opens no turn with it, so a stop over the same state would tell it again and
/// the pair would replay forever. Such a host hears one message once per prompt, and only a
/// message klin recorded first: a stop that lost the state lock, or whose stamp would not take
/// the record, tells it nothing. The journal records the stop as having told nothing, and a
/// repeat carries `told-before`. Spec 9.1, ADR 0052.
fn keeps_quiet(
    root: &Path,
    event: Option<&Event>,
    lost: bool,
    said: &str,
    log: &mut journal::Stop,
) -> bool {
    if !host::answering(event).follows_up() {
        return false;
    }
    let session = session(event);
    let heard = heard(said);
    let repeated = !lost && handoff::told_before(root, session, &heard);
    if repeated {
        log.flags.push("told-before");
    }
    if !lost && !repeated && handoff::expect_told(root, session, said, &heard) {
        return false;
    }
    log.told.clear();
    true
}

/// What a told message says, less its window line, whose age moves each minute and says
/// nothing new. Spec 9.1.
fn heard(said: &str) -> String {
    said.lines()
        .filter(|line| !line.trim_start().starts_with("window:"))
        .collect::<Vec<&str>>()
        .join("\n")
}

/// The host session an event names, and none for a stop no event placed.
fn session(event: Option<&Event>) -> &str {
    event.map_or("", |event| event.session.as_str())
}

/// The exit code a stop ends with: the host's own code for a block where the run blocked, and
/// the run's code otherwise. Spec 9.1.
fn exit_code(code: u8, event: Option<&Event>) -> u8 {
    match code {
        BLOCKED => host::answering(event).block_exit(),
        code => code,
    }
}

/// The benchmark wrapper may observe the report this stop already built. A failed write leaves
/// the harness without evidence and cannot change the hook's verdict or delivery.
fn observe_hook_report(report: Option<&Value>) {
    let Some(path) = std::env::var_os(HOOK_REPORT) else {
        return;
    };
    let Some(report) = report else {
        return;
    };
    let _ = std::fs::write(path, report.to_string());
}

/// The verdict this stop leaves for the next prompt, or the reason it left none: another event
/// held the lock for the whole budget, or the stamp could not be read or written. Spec 6.5.
fn written(
    root: &Path,
    lost: bool,
    green: bool,
    asked: Option<&[String]>,
    log: &mut journal::Stop,
) {
    if lost {
        eprintln!(
            "klin: NOTE: another klin event in this worktree held the state directory for the whole \
             {} ms klin waits, so this stop wrote no verdict, spent no block, and the window \
             stays as it is.",
            BUDGET.as_millis()
        );
        log.why =
            Some("another klin event held the state directory, so this stop wrote no verdict");
        return;
    }
    let mut said = String::new();
    let wrote = turn::verdict(root, green, asked, &mut said);
    eprint!("{said}");
    match (wrote, green) {
        (Ok(()), true) => log.verdict = "green",
        (Ok(()), false) => log.verdict = "red",
        (Err(why), _) => log.why = Some(why),
    }
}

/// What this stop tells the person when nothing blocks it, as one `systemMessage`: the notes the
/// run left, then the turn end and the week's headline. The turn end reads the journal, so it
/// runs only in a turn whose stamp says a stop spent a gate block, or under a prompt whose build
/// stamp says so when the verdict could not be written. The journal records each part by name.
/// Spec 9.5, 11.4.
fn tell(
    args: &Args,
    root: &Path,
    code: u8,
    note: Option<String>,
    log: &mut journal::Stop,
) -> Option<String> {
    let mut parts: Vec<(&'static str, String)> =
        note.into_iter().map(|note| ("note", note)).collect();
    let intervened = log.gate_blocks > 0 || turn::intervened(root);
    if code == 0 && !args.json && log.host.is_some() && intervened {
        let tail = stats::stop_tail(root);
        add_prompt_note(&tail, log, &mut parts);
        parts.extend(stats::turn_end(root, tail, journal::line(log)));
    }
    log.told = parts.iter().map(|(part, _)| *part).collect();
    let said: Vec<String> = parts.into_iter().map(|(_, text)| text).collect();
    (!said.is_empty()).then(|| said.join("\n"))
}

fn add_prompt_note(
    tail: &journal::Tail,
    log: &mut journal::Stop,
    parts: &mut Vec<(&'static str, String)>,
) {
    if log.gate_blocks > 0 && log.verdict == "red" && no_prompt_event(tail, log.session.as_deref())
    {
        log.flags.push("no-prompt-event");
        parts.push((
            "note",
            "klin: no prompt event reached this session; klin grants no fresh gate blocks until \
             `klin radius` runs on session start and on prompt submitted."
                .to_string(),
        ));
    }
}

/// Whether no `prompt` line of this stop's session reached the journal. The tail the stop read
/// reaches back past the turn stamp, which the `klin radius` run that appends that line takes
/// after appending it, so a tail with no such line is the absence and not a short read.
/// Spec 16.3.
fn no_prompt_event(tail: &journal::Tail, session: Option<&str>) -> bool {
    let Some(session) = session else {
        return false;
    };
    !tail.lines.iter().any(|line| {
        line.get("kind").and_then(Value::as_str) == Some("prompt")
            && line.get("session").and_then(Value::as_str) == Some(session)
    })
}

/// A hash of the config in force, recorded in the journal and not read, so a later reader can
/// tell a fix from a config change. A tree with no file says `derived`, and a file klin could
/// not read says `unreadable`, so neither reads as an edit to the other.
fn config_hash(project: &Project) -> String {
    if !project.config.written() {
        return "derived".to_string();
    }
    match std::fs::read_to_string(&project.config.file) {
        Ok(text) => format!("{:016x}", state::hash(text.as_bytes())),
        Err(_) => "unreadable".to_string(),
    }
}

/// One stop's run: the exit code the host reads, whether the gates left the tree green, the
/// findings a gate block put in front of the agent, which is `None` unless the stop blocked on
/// a gate, and the note a stop nothing blocks leaves for the person. A stop that `lost` the
/// state lock runs unserialized with another stop, so it measures and reports and spends no
/// block of either kind. Spec 6.5, 8.2, 16.3.
fn ran(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
    event: Option<&Event>,
    lost: bool,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, bool, Option<Vec<String>>, Option<String>) {
    let (outcome, build_ms) = journal::timed(|| built(args, project, window));
    log.timing.build_ms = build_ms;
    let (failure, said, unbuilt) = match outcome {
        Ok(outcome) => sorted(outcome),
        Err(problem) => {
            let (code, note) = handed(args, project, Err(problem), event, lost, log, out);
            return (code, false, None, note);
        }
    };
    match failure {
        Some(failure) => {
            let blocks = build_block(project.root(), lost, log);
            let (code, text) = does_not_build(args, &failure, &said, window, &blocks, log, out);
            (
                blocked_build(project.root(), event, text, code),
                false,
                None,
                None,
            )
        }
        None => {
            let judged = judge(args, project, window, &said, unbuilt.as_deref(), out);
            if let (Err(_), Some(note)) = (&judged, &unbuilt) {
                eprintln!("klin: {note}");
            }
            let green = matches!(&judged, Ok(tally) if tally.failed == 0 && tally.errored == 0);
            let reported = judged
                .as_ref()
                .map(|tally| tally.reported.clone())
                .unwrap_or_default();
            let (code, note) = handed(args, project, judged, event, lost, log, out);
            let asked = (code == BLOCKED).then_some(reported);
            (code, green, asked, note)
        }
    }
}

/// A finished build sorted into what blocks and what is told: the failure text of a build that
/// ran and failed, the provenance lines, and the note for a command the shell could not find.
fn sorted(
    (failure, said): (Option<build::Failure>, Vec<contract::Said>),
) -> (Option<String>, Vec<contract::Said>, Option<String>) {
    match failure {
        Some(build::Failure::Failed(failure)) => (Some(failure), said, None),
        Some(build::Failure::Missing { run, output }) => {
            (None, said, Some(unbuilt_said(&run, &output)))
        }
        None => (None, said, None),
    }
}

/// What the hook does with a run it finished: report it, and block the stop or let it end with
/// the note it leaves for the person.
fn handed(
    args: &Args,
    project: &Project,
    outcome: Result<Tally, Error>,
    event: Option<&Event>,
    lost: bool,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, Option<String>) {
    let mut tally = match refused(args, outcome, out) {
        Ok(tally) => tally,
        Err(problem) => {
            let _ = writeln!(out, "FAIL: {problem}");
            let mut records = Recorded::default();
            records.findings.push(record("error", &problem.to_string()));
            Tally {
                errored: 1,
                record: Some(as_json(
                    ERROR,
                    2,
                    &format!("klin: {problem}"),
                    records,
                    None,
                )),
                ..Tally::default()
            }
        }
    };
    log.report = tally.record.take();
    hook(
        args,
        tally,
        &std::mem::take(out),
        project.root(),
        event,
        lost,
        log,
    )
}

/// What one run came to: the gates that failed, the gates that could not run, and the notes
/// that fail nothing in the hook and still reach a person.
#[derive(Default)]
struct Tally {
    failed: usize,
    errored: usize,
    told: usize,
    /// Whether a file the base parsed is lost to a parse, which a person may hold where the
    /// grammar lags. Spec 7.2.
    grammar_lag: bool,
    /// The site id of every finding the run reported, which a stop that blocks records as asked.
    reported: Vec<String>,
    /// The 11.2 object the run built, which the journal writes as the stop's line. Spec 11.4.
    record: Option<Value>,
}

/// What the hook says about a tree that does not build. The messages name the bound from
/// `BLOCKS`, so the cap and the words for it cannot drift apart, and the block this stop spends,
/// so the agent reads how many are left. Spec 9.3.
fn does_not_build_said(block: Option<u64>) -> String {
    match block {
        Some(block) => format!(
            "the tree does not build, so no gate ran (a stop that changed the tree blocks until \
             it does, block {block} of {BLOCKS} in this turn)"
        ),
        None => "the tree does not build, so no gate ran".to_string(),
    }
}

/// Who builds a tree outside the hook: `klin gate` outside it runs no build. ADR 0012, spec 9.3.
const OWN_CI: &str = "`klin check` runs no build, so the project's own CI must run it";

fn stopped_blocking() -> String {
    format!(
        "the build has blocked {BLOCKS} stops under this prompt, so klin stops blocking; the \
         failure stands. {OWN_CI}."
    )
}

fn unchanged() -> String {
    format!(
        "the tree did not change since the stop klin last blocked, so klin does not block again; \
         the failure stands. {OWN_CI}."
    )
}

fn unbounded_note() -> String {
    format!(
        "klin could not safely spend a build block, so this build failure blocks nothing. \
         {OWN_CI}."
    )
}

/// The note for a build whose command the shell could not find. It names the command, quotes
/// the shell, and says the one action left, because the failing output alone told the agent
/// nothing it could act on. ADR 0048.
fn unbuilt_said(run: &str, output: &str) -> String {
    format!(
        "NOTE: the build `{run}` could not run ({output}), so klin judged the source as it \
         stands; {OWN_CI}. Install the project's dependencies, or a person sets `build` to \
         `false` in klin.json."
    )
}

/// The build stamp: one record per prompt. The prompt counter of the turn file it was taken
/// under, the host session that took it, how many stops a build failure and a gate failure
/// already blocked, and the tree each kind of block last saw. The two kinds never share a count
/// or a tree. A record taken under an earlier prompt reads as zero, so every prompt gets the
/// whole budget. Spec 16.3, ADR 0052.
struct Count {
    prompt: u64,
    session: Option<String>,
    builds: u64,
    /// The working tree the last build block was taken over, so a stop that changed nothing
    /// since is reported and not blocked again. ADR 0048.
    build_tree: Option<String>,
    gate_blocks: u64,
    /// The working tree the last gate block was taken over. Only a tree klin recorded here can
    /// prove that a later stop changed it. ADR 0052.
    gate_tree: Option<String>,
}

/// The record as this prompt left it. A stop that `continued` a chain of messages its host
/// submitted by itself keeps its own session's record whatever prompt it was taken under:
/// another hook's message may have won the host's merge, and klin read it as a person's prompt,
/// but the chain belongs to the prompt that opened it, and the stop that opened it wrote the
/// record (`opened`). The record is written back under the current counter, so every later stop
/// of the chain reads it too. A record an older klin wrote names its build tree `tree` and its
/// one gate block `gate_spent`, and names no gate tree or session, so it can never prove a
/// second gate block or carry into a chain. Spec 9.3, ADR 0052.
fn count(at: &Path, log: &journal::Stop) -> Count {
    let prompt = turn::prompts(at);
    let held = held(at)
        .filter(|held| taken_under(held, prompt) || log.continued && taken_by(held, log))
        .unwrap_or_default();
    let text = |key: &str| held.get(key)?.as_str().map(str::to_string);
    let legacy_spent = held.get("gate_spent").and_then(Value::as_bool) == Some(true);
    Count {
        prompt,
        session: log.session.clone(),
        builds: held
            .get("builds")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        build_tree: text("build_tree").or_else(|| text("tree")),
        gate_blocks: held
            .get("gate_blocks")
            .and_then(Value::as_u64)
            .unwrap_or(u64::from(legacy_spent)),
        gate_tree: text("gate_tree"),
    }
}

fn held(at: &Path) -> Option<Value> {
    std::fs::read_to_string(at.join(BUILD_BLOCKED))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
}

fn taken_under(held: &Value, prompt: u64) -> bool {
    held.get("prompt").and_then(Value::as_u64) == Some(prompt)
}

fn taken_by(held: &Value, log: &journal::Stop) -> bool {
    log.session
        .as_deref()
        .is_some_and(|session| held.get("session").and_then(Value::as_str) == Some(session))
}

/// A stop that no automatic message came before opens its prompt's budget, even where it
/// spends none, so a later stop of the chain it opens inherits that budget and never one this
/// session left under an earlier prompt. A record another session left stays: no chain of this
/// session inherits it, and its own chain still may. Spec 9.3, ADR 0052.
fn opened(root: &Path, lost: bool, log: &mut journal::Stop) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    let prompt = turn::prompts(&at);
    let stale = held(&at).is_some_and(|held| taken_by(&held, log) && !taken_under(&held, prompt));
    if lost || log.continued || !stale {
        return;
    }
    if !counted(&at, &count(&at, log)) {
        log.flags.push("count-unwritable");
    }
}

/// Whether the record reached the disk. A count klin cannot write bounds nothing, so the
/// caller reports the build failure and does not block on it. Spec 14.
fn counted(at: &Path, count: &Count) -> bool {
    let text = serde_json::json!({
        "prompt": count.prompt,
        "session": count.session,
        "builds": count.builds,
        "build_tree": count.build_tree,
        "gate_blocks": count.gate_blocks,
        "gate_tree": count.gate_tree,
    })
    .to_string()
        + "\n";
    let target = at.join(BUILD_BLOCKED);
    if write::atomic_write(write::AtomicWrite {
        target: &target,
        bytes: text.as_bytes(),
        keep_mode_from: None,
    })
    .is_ok()
    {
        return true;
    }
    false
}

fn does_not_build(
    args: &Args,
    failure: &str,
    said: &[contract::Said],
    window: Option<&Window>,
    blocks: &Blocks,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, String) {
    let (code, report, text) = reported(args, failure, said, window, blocks, out);
    log.report = Some(report);
    (code, text)
}

/// What a build failure at this stop spends: the block it took and its number in this turn, no
/// block because the tree did not change since the last one, or no block because klin could
/// not record one.
enum Blocks {
    Spent(u64),
    Unchanged,
    Unbounded,
}

impl Blocks {
    /// The exit code, the block's number when the stop blocks, and the note that says why it
    /// does not.
    fn outcome(&self) -> (u8, Option<u64>, Option<String>) {
        match self {
            Blocks::Spent(builds) if *builds <= BLOCKS => (2, Some(*builds), None),
            Blocks::Spent(_) => (0, None, Some(stopped_blocking())),
            Blocks::Unchanged => (0, None, Some(unchanged())),
            Blocks::Unbounded => (0, None, Some(unbounded_note())),
        }
    }
}

/// A host that cannot read stderr still has to show the build failure. An adapter whose stop
/// already reads stderr ignores the text and returns 2.
fn blocked_build(root: &Path, event: Option<&Event>, text: String, code: u8) -> u8 {
    match code {
        2 => block(root, event, text),
        _ => code,
    }
}

/// The build block a failing build may spend. None at a stop that lost the state lock, because
/// another stop may be writing the count; the lock's own NOTE tells it. The build twin of the
/// gate's `next`. Spec 6.5.
fn build_block(root: &Path, lost: bool, log: &mut journal::Stop) -> Blocks {
    match lost {
        true => Blocks::Unbounded,
        false => raised(root, log),
    }
}

/// The block this build failure spends. `Unchanged` when the working tree is the one the last
/// block was taken over, because blocking again on a tree the agent did not touch teaches it
/// nothing. `Unbounded` when klin could not record the block, either because the state
/// directory is gone or because the record itself would not write: neither count could bound
/// the blocks, so the NOTE names the write that failed and the stop is not blocked. Spec 14.
fn raised(root: &Path, log: &mut journal::Stop) -> Blocks {
    let at = match state::ready(root) {
        Ok(at) => at,
        Err(why) => return unbounded(&why, log),
    };
    let held = count(&at, log);
    let tree = working_tree(root, &at);
    if held.builds > 0 && tree.is_some() && tree == held.build_tree {
        return Blocks::Unchanged;
    }
    let count = Count {
        builds: held.builds + 1,
        build_tree: tree,
        ..held
    };
    match counted(&at, &count) {
        true => Blocks::Spent(count.builds),
        false => unbounded(
            &format!("{} could not be written", at.join(BUILD_BLOCKED).display()),
            log,
        ),
    }
}

fn unbounded(why: &str, log: &mut journal::Stop) -> Blocks {
    log.flags.push("count-unwritable");
    eprintln!(
        "klin: NOTE: {why} — so no count could bound the build blocks, and this build failure \
         blocks nothing."
    );
    Blocks::Unbounded
}

/// The build failure as a person and an agent read it, and as `--json` records it. The text
/// opens with where each command came from, so a derived build is never a command with no
/// origin, and the note says why klin did not block, because the exit code alone no longer
/// says it. Spec 11, ADR 0040.
fn reported(
    args: &Args,
    failure: &str,
    built: &[contract::Said],
    window: Option<&Window>,
    blocks: &Blocks,
    out: &mut String,
) -> (u8, Value, String) {
    let (code, block, note) = blocks.outcome();
    let said = does_not_build_said(block);
    let mut records = Recorded {
        derived: built
            .iter()
            .filter_map(|(_, entry)| entry.clone())
            .collect(),
        ..Recorded::default()
    };
    records
        .findings
        .push(record("error", &format!("{said}:\n{failure}")));
    if let Some(note) = &note {
        records.notes.push(record("note", note));
    }
    let object = as_json(ERROR, code, &format!("klin: {said}."), records, window);
    let mut text = String::new();
    for (line, _) in built {
        let _ = writeln!(text, "klin: {line}");
    }
    let _ = writeln!(text, "klin: {said}:");
    text.push_str(failure);
    if let Some(note) = &note {
        let _ = writeln!(text, "klin: {note}");
    }
    if !args.json {
        eprint!("{text}");
        return (code, object, text);
    }
    out.clear();
    let _ = writeln!(out, "{object}");
    (code, object, text)
}

/// The build a person chose or the one the manifests derive, run before any gate judges the
/// tree it produces, with the provenance of each command the gates' report carries. Only the
/// hook builds, so no other run derives a build. Spec 5.4, ADR 0012.
fn built(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
) -> Result<(Option<build::Failure>, Vec<contract::Said>), Error> {
    let plan = build::plan(project)?;
    if plan.entries.is_empty() {
        return Ok((None, plan.said));
    }
    let changes = scoped(args, project, &plan.entries, window)?;
    let (failure, resolved) = build::failure(
        project.root(),
        &build::wanted(&plan.entries, changes.as_deref()),
    );
    let mut said = plan.said;
    said.extend(resolved);
    Ok((failure, said))
}

/// The changed set the build is narrowed to, which is the one the gates read after it. Spec 9.
fn scoped<'a>(
    args: &Args,
    project: &'a Project,
    entries: &[build::Entry],
    window: Option<&Window>,
) -> Result<Option<Cow<'a, [Change]>>, Error> {
    if !args.changed || entries.iter().all(|entry| entry.root.is_none()) {
        return Ok(None);
    }
    let base = chosen(window, project, args.strict)?;
    project.changes(&base.before).map(Some)
}

fn judge(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
    built: &[contract::Said],
    unbuilt: Option<&str>,
    out: &mut String,
) -> Result<Tally, Error> {
    let plan = plan(project)?;
    if args.list {
        return listed(args, project, &plan, window, out);
    }
    let wanted: Vec<&Gate> = select(&args.gates, &plan, project)?
        .into_iter()
        .filter(|gate| gate.check.placement.at_stop())
        .collect();
    let against = against_or_stop(args, &wanted, project, window, out)?;
    said(args, built, out);
    if let (Some(unbuilt), false) = (unbuilt, args.json) {
        let _ = writeln!(out, "  {unbuilt}");
    }
    let rootless = no_source_root(args, &plan, &wanted, project, out)?;
    let (mut tally, mut records) = each(args, &wanted, project, &against, out);
    records.notes.extend(rootless);
    if let Some(unbuilt) = unbuilt {
        records.notes.push(record(UNBUILT, unbuilt));
        tally.told += 1;
    }
    let derived = built.iter().filter_map(|(_, entry)| entry.clone());
    records.derived.splice(0..0, derived);
    tally.record = Some(finish(
        args,
        &plan,
        wanted.len(),
        &tally,
        records,
        &against,
        out,
    ));
    Ok(tally)
}

/// What this run judges the working tree against: the base commit, laid out, and the files
/// a scoped run looks at.
#[derive(Default)]
struct Against<'a> {
    base: Option<Window>,
    changes: Option<Cow<'a, [Change]>>,
    scope: Option<Vec<String>>,
    prior: Option<Prior>,
    /// Why the base tree could not be laid out, which fails only the gates that read it.
    /// Spec 7.3.
    unlaid: Option<String>,
}

/// The run as the Stop and `klin policy` take it, where a base tree klin could not lay out stops
/// the run.
fn against_or_stop<'a>(
    args: &Args,
    wanted: &[&Gate],
    project: &'a Project,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Against<'a>, Error> {
    let against = against(args, wanted, project, window, out)?;
    match against.unlaid {
        Some(why) => Err(Error(why)),
        None => Ok(against),
    }
}

fn against<'a>(
    args: &Args,
    wanted: &[&Gate],
    project: &'a Project,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Against<'a>, Fault> {
    let base = base(args, wanted, project, window, out).map_err(fault(ErrorKind::Base))?;
    let changes = changes(args, project, base.as_ref(), out).map_err(fault(ErrorKind::Base))?;
    let scope = changes
        .as_ref()
        .map(|changed| changed.iter().map(|change| change.path.clone()).collect());
    let (prior, unlaid) = match prior(project, base.as_ref(), changes.as_deref(), wanted) {
        Ok(prior) => (prior, None),
        Err(why) => (None, Some(why.to_string())),
    };
    Ok(Against {
        scope,
        changes,
        prior,
        base,
        unlaid,
    })
}

fn base(
    args: &Args,
    wanted: &[&Gate],
    project: &Project,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Option<Window>, Error> {
    if !args.changed && !wanted.iter().any(|gate| gate.check.needs.the_commit()) {
        return Ok(None);
    }
    let base = chosen(window, project, args.strict)?;
    if !args.json {
        let _ = writeln!(out, "  {}", base.line());
    }
    Ok(Some(base))
}

/// The window the run judges: the one the hook already read, or the base a run by hand and CI
/// choose for themselves. Spec 6.1, 6.3.
fn chosen(window: Option<&Window>, project: &Project, strict: bool) -> Result<Window, Error> {
    match window {
        Some(window) => Ok(window.clone()),
        None => base::choose(project.root(), strict),
    }
}

/// The effective policy of every gate, or of the one `policy` names: its state, and each value
/// it uses with where the value came from. A value the section leaves out is said by the run
/// that derives it, so each gate that runs is run and only its provenance is kept. Spec 11.6.
// ponytail: derives by running the gate, a derive-only seam per check if policy must stay cheap
fn listed(
    args: &Args,
    project: &Project,
    plan: &Plan,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Tally, Error> {
    if plan.gates.is_empty() && plan.excluded.is_empty() {
        return Err(no_gate(project, plan));
    }
    for name in &args.gates {
        stated(name, plan, project)?;
    }
    let mut capabilities = active(args, project, plan, window)?;
    capabilities.extend(inactive(args, plan));
    let state = state::dir(project.root());
    match args.json {
        true => policy_json(project, capabilities, state.as_deref(), out),
        false => policy_text(&capabilities, args.gates.is_empty(), state.as_deref(), out),
    }
    Ok(Tally::default())
}

/// A name `policy` takes: a gate that runs, one a person excluded, or one that needs a section.
fn stated(name: &str, plan: &Plan, project: &Project) -> Result<(), Error> {
    let inactive = plan.excluded.iter().any(|excluded| excluded == name)
        || plan.needs_a_section.iter().any(|check| check.name == name);
    match inactive {
        true => Ok(()),
        false => known(name, plan, project),
    }
}

fn inactive(args: &Args, plan: &Plan) -> Vec<Value> {
    let excluded = plan
        .excluded
        .iter()
        .filter(|name| named(args, name))
        .map(|name| {
            let placement = catalogue::CATALOGUE
                .iter()
                .find(|check| check.name == name)
                .map(|check| check.placement.names());
            json!({ "name": name, "placement": placement, "state": "excluded" })
        });
    let needed = plan
        .needs_a_section
        .iter()
        .filter(|check| named(args, check.name))
        .map(|check| {
            json!({
                "name": check.name,
                "section": check.section,
                "kind": check.kind(),
                "placement": check.placement.names(),
                "state": "needs-policy",
            })
        });
    excluded.chain(needed).collect()
}

fn named(args: &Args, name: &str) -> bool {
    args.gates.is_empty() || args.gates.iter().any(|one| one == name)
}

/// Each gate that runs, with the values its run used and where each came from.
fn active(
    args: &Args,
    project: &Project,
    plan: &Plan,
    window: Option<&Window>,
) -> Result<Vec<Value>, Error> {
    let wanted: Vec<&Gate> = plan
        .gates
        .iter()
        .filter(|gate| named(args, &gate.name))
        .collect();
    if let (Some(entry), [gate]) = (&args.entry, wanted.as_slice())
        && gate.check.explain.is_none()
    {
        return Err(Error(format!(
            "{} has no entries to explain one by one, so drop {entry}",
            gate.name
        )));
    }
    let quiet = Args {
        config: args.config.clone(),
        json: true,
        ..Args::default()
    };
    let run: Vec<&Gate> = wanted
        .iter()
        .copied()
        .filter(|gate| gate.check.explain.is_none())
        .collect();
    let against = against_or_stop(&quiet, &run, project, window, &mut String::new())?;
    let mut capabilities = Vec::new();
    for gate in &wanted {
        capabilities.push(match gate.check.explain {
            Some(explain) => capability(
                project,
                gate,
                explain(project, args.entry.as_deref())?,
                Vec::new(),
            ),
            None => provenance(&quiet, project, gate, &against),
        });
    }
    Ok(capabilities)
}

/// One gate's run, kept only for the `derived:` and `pinned:` lines of the values it used.
fn provenance(quiet: &Args, project: &Project, gate: &Gate, against: &Against) -> Value {
    let (_, told, _, recorded) = one(quiet, gate, project, against);
    let mut lines = render::provenance(&told);
    for line in pinned(project, gate) {
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    capability(project, gate, lines, recorded.derived)
}

fn capability(project: &Project, gate: &Gate, lines: Vec<String>, derived: Vec<Value>) -> Value {
    json!({
        "name": gate.name,
        "section": gate.check.section,
        "kind": gate.check.kind(),
        "placement": gate.check.placement.names(),
        "state": "active",
        "values": values(project, gate, derived),
        "lines": lines,
    })
}

/// Each value a gate uses as the policy document names it: the ones a person pinned in the
/// section, then the ones the run derived with their rule. Spec 11.7.
fn values(project: &Project, gate: &Gate, derived: Vec<Value>) -> Vec<Value> {
    let section = gate.check.section;
    let mut values: Vec<Value> = match project.config.pinned(section) {
        Some(Value::Object(fields)) => fields
            .iter()
            .map(|(key, value)| json!({ "key": key, "value": value, "provenance": "pinned" }))
            .collect(),
        _ => Vec::new(),
    };
    values.extend(derived.into_iter().map(|entry| {
        json!({
            "key": entry.get("key"),
            "value": entry.get("value"),
            "provenance": "derived",
            "rule": entry.get("rule"),
        })
    }));
    values
}

fn policy_text(capabilities: &[Value], whole: bool, state: Option<&Path>, out: &mut String) {
    for capability in capabilities {
        let name = capability["name"].as_str().unwrap_or_default();
        let said = match capability["state"].as_str() {
            Some("active") => "runs",
            Some("excluded") => "excluded",
            _ => "needs a section a person writes",
        };
        let _ = writeln!(out, "{name} — {said}");
        let placement: Vec<&str> = capability["placement"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if !placement.is_empty() {
            let _ = writeln!(out, "{UNDER}placement: {}", placement.join(", "));
        }
        for line in capability["lines"].as_array().into_iter().flatten() {
            let _ = writeln!(out, "{UNDER}{}", line.as_str().unwrap_or_default());
        }
    }
    if whole {
        out.push_str(&render::lost_policy());
    }
    if let Some(at) = state {
        let _ = writeln!(out, "state: {}", at.display());
    }
}

fn policy_json(
    project: &Project,
    mut capabilities: Vec<Value>,
    state: Option<&Path>,
    out: &mut String,
) {
    for capability in &mut capabilities {
        if let Value::Object(fields) = capability {
            fields.remove("lines");
        }
    }
    let document = json!({
        "schema_version": 1,
        "command": "policy",
        "config": {
            "path": project.config.file.display().to_string(),
            "present": project.config.written(),
        },
        "capabilities": capabilities,
        "state_dir": state.map(|at| at.display().to_string()),
    });
    let _ = writeln!(out, "{document}");
}

/// The `pinned:` line of each value a person wrote into a gate's section.
fn pinned(project: &Project, gate: &Gate) -> Vec<String> {
    let section = gate.check.section;
    let Some(Value::Object(fields)) = project.config.pinned(section) else {
        return Vec::new();
    };
    fields
        .iter()
        .map(|(key, value)| format!("pinned: {section} {key} {}", shown(value)))
        .collect()
}

/// A pinned value as a person reads it: a path or a list of paths as text, anything else as JSON.
fn shown(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) if items.iter().all(Value::is_string) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<&str>>()
            .join(", "),
        other => other.to_string(),
    }
}

/// A survey that finds no source root, with a check that measures code left for it to supply:
/// exit 2 under `klin check`, and a NOTE in the hook, which lets the turn end. Only the gates a
/// run selected count, so `klin check lockfile` is not this hole. Without
/// it a CI job in the wrong directory applies every gate to nothing and prints green. A person
/// who excluded those gates, or who pinned their roots, is not this hole. A clone with no base
/// is a different error, of spec 14. ADR 0016, spec 10, 14.
///
/// This runs after the base is laid out, so a `--strict` run pays for a layout it then refuses.
/// A rootless tree is rare, and the NOTE reads after the lines it explains rather than before
/// the window line, so the order stands.
fn no_source_root(
    args: &Args,
    plan: &Plan,
    wanted: &[&Gate],
    project: &Project,
    out: &mut String,
) -> Result<Option<Value>, Error> {
    if !rootless(args, plan, wanted, project) {
        return Ok(None);
    }
    let said = no_source_root_said(project);
    if args.strict {
        return Err(Error(said));
    }
    if !args.json {
        let _ = writeln!(out, "  NOTE: {said}");
    }
    Ok(Some(record("note", &said)))
}

fn no_source_root_said(project: &Project) -> String {
    format!(
        "the survey of {} found no source root — a source root is a directory that holds \
         nothing but source files, so no gate that reads code ran here at all; run klin from \
         the tree you mean to gate, or set those gates to false to exclude them",
        project.root().display()
    )
}

/// Whether the run selected a check that reads code in a tree with no source root, where the
/// check would measure nothing. Only the checks a run selects count, so `klin check lockfile`
/// is not this hole. A gate whose section pins `in` measures what the pin names, and one a
/// person excluded measures nothing on purpose, so neither is this hole. ADR 0016, spec 7.2,
/// 11.3.
fn rootless(args: &Args, plan: &Plan, wanted: &[&Gate], project: &Project) -> bool {
    project.found_no_source_root() && leaves_code(args, plan, wanted, project)
}

/// Whether the run leaves a check that reads code to find its own roots: one it selected, or one
/// a whole run would select once the tree holds code.
fn leaves_code(args: &Args, plan: &Plan, wanted: &[&Gate], project: &Project) -> bool {
    let derives = |check: &catalogue::Row| check.reads_code() && !pins_in(project, check);
    let unselected =
        args.gates.is_empty() && plan.needs_a_section.iter().any(|check| derives(check));
    unselected || wanted.iter().any(|gate| derives(gate.check))
}

fn pins_in(project: &Project, check: &catalogue::Row) -> bool {
    project
        .config
        .pinned(check.section)
        .is_some_and(|section| section.get("in").is_some())
}

fn finish(
    args: &Args,
    plan: &Plan,
    gates: usize,
    tally: &Tally,
    records: Recorded,
    against: &Against,
    out: &mut String,
) -> Value {
    let (failed, errored) = (tally.failed, tally.errored);
    let excluded = match plan.excluded.len() {
        0 => String::new(),
        count => format!("{count} excluded, "),
    };
    let line = format!(
        "klin: {gates} gate(s), {excluded}{}",
        summary(failed, errored)
    );
    let code = code(tally);
    let object = as_json(
        status_row(code),
        code,
        &line,
        records,
        against.base.as_ref(),
    );
    if !args.json {
        let _ = writeln!(out, "{line}");
        return object;
    }
    out.clear();
    let _ = writeln!(out, "{object}");
    object
}

/// Where the build the hook ran came from, printed once above the gates, each of which says
/// its own values beside its row. Spec 4.3.
fn said(args: &Args, built: &[contract::Said], out: &mut String) {
    if args.json {
        return;
    }
    for (line, _) in built {
        let _ = writeln!(out, "  {line}");
    }
}

/// The row of 11.1 a run's own exit code names, for the runs whose status and code agree.
fn status_row(code: u8) -> &'static str {
    match code {
        0 => "PASS",
        1 => "FAIL",
        _ => ERROR,
    }
}

/// The object of spec 11.2. `status` is the row of 11.1, which a caller gives rather than reads
/// off `code`, because a build failure that stops blocking is an `ERROR` row that exits 0.
fn as_json(status: &str, code: u8, tally: &str, records: Recorded, base: Option<&Window>) -> Value {
    let mut out = Map::new();
    out.insert("status".into(), status.into());
    out.insert("summary".into(), tally.into());
    if let Some(base) = base {
        out.insert("window".into(), base.record());
    }
    out.insert("derived".into(), Value::Array(records.derived));
    out.insert("gates".into(), Value::Array(records.gates));
    out.insert("findings".into(), Value::Array(records.findings));
    out.insert("notes".into(), Value::Array(records.notes));
    out.insert("exit".into(), code.into());
    Value::Object(out)
}

fn refused(args: &Args, outcome: Result<Tally, Error>, out: &mut String) -> Result<Tally, Error> {
    let Err(problem) = outcome else {
        return outcome;
    };
    if !args.json {
        return Err(problem);
    }
    let mut records = Recorded::default();
    records.findings.push(record("error", &problem.to_string()));
    let object = as_json(ERROR, 2, &format!("klin: {problem}"), records, None);
    out.clear();
    let _ = writeln!(out, "{object}");
    Ok(Tally {
        errored: 1,
        record: Some(object),
        ..Tally::default()
    })
}

/// The records of spec 11.2 only the runner writes: the findings, notes and derived entries it
/// renders from each gate's typed result or records itself, and one row per gate.
#[derive(Default)]
struct Recorded {
    findings: Vec<Value>,
    notes: Vec<Value>,
    derived: Vec<Value>,
    holes: Vec<Value>,
    reviews: Vec<Value>,
    gates: Vec<Value>,
}

impl From<render::Json> for Recorded {
    fn from(rendered: render::Json) -> Recorded {
        Recorded {
            findings: rendered.findings,
            notes: rendered.notes,
            derived: rendered.derived,
            holes: rendered.holes,
            reviews: rendered.reviews,
            gates: Vec::new(),
        }
    }
}

fn record(outcome: &str, text: &str) -> Value {
    let mut out = Map::new();
    out.insert("outcome".into(), outcome.into());
    out.insert("text".into(), text.trim_end().into());
    Value::Object(out)
}

fn hook(
    args: &Args,
    tally: Tally,
    report: &str,
    root: &Path,
    event: Option<&Event>,
    lost: bool,
    log: &mut journal::Stop,
) -> (u8, Option<String>) {
    let (failed, errored) = (tally.failed, tally.errored);
    let held = state::ready(root).ok().map(|at| (count(&at, log), at));
    unwritable(root);
    if failed == 0 && errored == 0 {
        return nothing_blocks(args, tally.told, report, event);
    }
    let Some(event) = event else {
        eprint!("{report}");
        return (1, None);
    };
    let next = match lost {
        true => GateBlock::Pass(LOCKED.to_string()),
        false => next(root, held.as_ref(), event.blocked_before),
    };
    let number = match spend(held, next, log) {
        GateBlock::Take { number, .. } => number,
        GateBlock::Pass(why) => return not_blocked(args, &tally, report, event, &why),
    };
    let lead = format!(
        "klin: {} — fix what each names, then stop again (gate block {number} of {GATE_BLOCKS} \
         in this turn):",
        lead(failed, errored)
    );
    eprintln!("{lead}");
    eprint!("{report}");
    let code = block(root, Some(event), format!("{lead}\n{report}"));
    if code == BLOCKED {
        log.gate_block = Some(number);
    }
    (code, None)
}

/// A stop that failed and spends no gate block: the report, why klin does not block again, and
/// where a file is lost to a parse, the words that tell the person how to hold it. The person
/// hears them only through a channel the agent does not read. Spec 7.2, ADR 0052.
fn not_blocked(
    args: &Args,
    tally: &Tally,
    report: &str,
    event: &Event,
    why: &str,
) -> (u8, Option<String>) {
    eprintln!(
        "klin: {} — this stop is not blocked:",
        lead(tally.failed, tally.errored)
    );
    eprint!("{report}");
    eprintln!(
        "klin: not blocking again; {why}, and the window stays open until a person \
         fixes, accepts or resets it."
    );
    let person = (tally.grammar_lag && !args.json && !event.host.follows_up())
        .then(|| render::LOST_TO_A_PERSON.to_string());
    (event.host.stop(&Stop::Pass), person)
}

/// What the hook says about a stop nothing blocks: nothing at all, or the notes the run left for
/// a person, which the stop tells through the host once it has written its line. A `--json` run
/// hands its report to stderr, because a host that reads JSON reads the report itself, and so
/// does a run whose host event klin cannot read, because then klin does not know whose shape to
/// tell it in. Spec 9.1, 16.5.
fn nothing_blocks(
    args: &Args,
    told: usize,
    report: &str,
    event: Option<&Event>,
) -> (u8, Option<String>) {
    if told == 0 {
        return (0, None);
    }
    let said = format!("klin: nothing blocks the stop, and the run left a note:\n{report}");
    if args.json || event.is_none() {
        eprint!("{said}");
        return (1, None);
    }
    (0, Some(said))
}

/// What a gate failure at this stop spends: the gate block it takes, with its number under this
/// prompt and the tree it is taken over, or no block and the reason the report gives.
enum GateBlock {
    Take { number: u64, tree: Option<String> },
    Pass(String),
}

/// The gate block this failure may take. The first is free. The second needs a tree that
/// differs from the one klin recorded for the first, so a stop over the tree the agent left
/// alone reports and lets the turn end. None comes after the second, and none at all without a
/// state directory to record it in. The host's flag says a block happened, never which tree it
/// saw: where klin's record holds no gate block, the flag counts as one klin never recorded, so
/// the stop spends none, and it never proves a second. After a build block that flag is true
/// while no gate block is spent, so it counts only where no build block was spent either.
/// Spec 16.3, ADR 0052.
fn next(root: &Path, held: Option<&(Count, PathBuf)>, blocked_before: bool) -> GateBlock {
    let Some((count, at)) = held else {
        return GateBlock::Pass(UNRECORDED.to_string());
    };
    if count.gate_blocks >= GATE_BLOCKS {
        return GateBlock::Pass(capped());
    }
    let flagged = count.builds == 0 && blocked_before;
    if count.gate_blocks == 0 && !flagged {
        return GateBlock::Take {
            number: 1,
            tree: working_tree(root, at),
        };
    }
    changed_since(root, count, at)
}

/// The next gate block after one that already happened, which only a tree klin recorded for
/// that block and a current tree that differs from it can prove.
fn changed_since(root: &Path, count: &Count, at: &Path) -> GateBlock {
    let Some(before) = count.gate_tree.as_deref() else {
        return GateBlock::Pass(UNPROVEN.to_string());
    };
    match working_tree(root, at) {
        Some(tree) if tree == before => GateBlock::Pass(UNCHANGED_SINCE_GATE.to_string()),
        Some(tree) => GateBlock::Take {
            number: count.gate_blocks + 1,
            tree: Some(tree),
        },
        None => GateBlock::Pass(UNPROVEN.to_string()),
    }
}

/// The gate block recorded before it is delivered. A block klin cannot record could not be
/// bounded, because the next stop would read it as never spent and take it again, so it
/// becomes a report. ADR 0052.
fn spend(held: Option<(Count, PathBuf)>, next: GateBlock, log: &mut journal::Stop) -> GateBlock {
    let (number, tree) = match &next {
        GateBlock::Take { number, tree } => (*number, tree.clone()),
        GateBlock::Pass(_) => return next,
    };
    let Some((count, at)) = held else {
        return GateBlock::Pass(UNRECORDED.to_string());
    };
    let recorded = Count {
        gate_blocks: number,
        gate_tree: tree,
        ..count
    };
    if counted(&at, &recorded) {
        return next;
    }
    log.flags.push("count-unwritable");
    GateBlock::Pass(UNRECORDED.to_string())
}

/// Why a stop after a gate block spends none: klin has no tree of its own to compare against.
const UNPROVEN: &str = "klin holds no record of the tree the last gate block saw, so it cannot \
    tell whether this stop changed it";
/// Why a stop spends no gate block when its record would not write. ADR 0052.
const UNRECORDED: &str = "klin could not record a gate block, so nothing would bound it";
/// Why a stop that lost the state lock spends no gate block. Spec 6.5.
const LOCKED: &str = "another klin event held the state directory, so this stop could not count a \
    gate block";
/// Why a stop over the tree the last gate block saw spends none. ADR 0052.
const UNCHANGED_SINCE_GATE: &str = "the tree did not change since the last gate block";

/// Why a stop after the prompt's last gate block spends none, with the bound from
/// `GATE_BLOCKS`, so the cap and the words for it cannot drift apart.
fn capped() -> String {
    format!(
        "the gate has blocked {GATE_BLOCKS} stops under this prompt, which is the most it blocks"
    )
}

/// The working tree as the build stamp records it, hashed through the build stamp's own index.
fn working_tree(root: &Path, at: &Path) -> Option<String> {
    stamp::tree_through(root, &at.join(BUILD_INDEX))
}

/// Record the exact report a follow-up host will echo under the event's session, then deliver
/// the block. The stop's run reads `BLOCKED` as its decision whatever exit code the host takes
/// for a block, and the stop ends with that code. A stop that tells records its message the same way, so neither echo opens a turn
/// or a fresh gate budget. Spec 9.1, 9.3.
///
/// A host that submits the report hears it only once klin recorded it. A report klin could
/// not record would open a turn and a fresh budget when the host submits it, so the stop is
/// reported and not blocked, and the block its count already took stays spent, so the budget
/// only shrinks. ADR 0052.
fn block(root: &Path, event: Option<&Event>, said: String) -> u8 {
    let host = host::answering(event);
    if host.follows_up() && !handoff::expect_followup(root, session(event), &said) {
        eprintln!(
            "klin: NOTE: klin could not record the report the host will submit as its next \
             prompt, so this stop is not blocked."
        );
        return host.stop(&Stop::Pass);
    }
    host.stop(&Stop::Block(said));
    BLOCKED
}

/// A state directory klin cannot write costs a wider window and nothing else. Section 14.
fn unwritable(root: &Path) {
    if let Err(why) = state::ready(root) {
        eprintln!(
            "klin: NOTE: {why} — so this stop wrote no verdict, and a state klin cannot \
             keep blocks nothing."
        );
    }
}

fn lead(failed: usize, errored: usize) -> &'static str {
    match (failed > 0, errored > 0) {
        (true, true) => "a quality gate failed, and another could not run",
        (true, false) => "a quality gate failed",
        _ => "could not run a quality gate",
    }
}

fn prior(
    project: &Project,
    base: Option<&Window>,
    changes: Option<&[Change]>,
    wanted: &[&Gate],
) -> Result<Option<Prior>, Error> {
    let Some(base) = base.filter(|_| wanted.iter().any(|gate| gate.check.needs.the_tree())) else {
        return Ok(None);
    };
    base::materialize(project, &base.before, changes).map(Some)
}

/// The changed set the scoped gates judge, computed once for the run and reused by the base
/// laid out for them. Spec 4.5, ADR 0038.
fn changes<'a>(
    args: &Args,
    project: &'a Project,
    base: Option<&Window>,
    out: &mut String,
) -> Result<Option<Cow<'a, [Change]>>, Error> {
    let (true, Some(base)) = (args.changed, base) else {
        return Ok(None);
    };
    let changed = project.changes(&base.before)?;
    if !args.json {
        let _ = writeln!(
            out,
            "  changed: {} file(s) against the base — the scoped gates judge those; \
             CI judges everything",
            changed.len()
        );
    }
    Ok(Some(changed))
}

fn names<'a>(named: impl Iterator<Item = &'a str>) -> String {
    named.collect::<Vec<&str>>().join(", ")
}

fn every_check() -> String {
    names(catalogue::names())
}

fn no_gate(project: &Project, plan: &Plan) -> Error {
    let config = &project.config;
    if !plan.excluded.is_empty() {
        return Error(format!(
            "{} excludes every gate it names: {} — a run that measures nothing cannot pass, \
             so lift one exclusion",
            config.file.display(),
            names(plan.excluded.iter().map(String::as_str))
        ));
    }
    if !config.written() {
        return Error(format!(
            "{} does not exist and the survey of {} found no source root, no document and no \
             manifest, so there is nothing to gate — run klin from the tree you mean to measure, \
             or write the file naming one of: {}",
            config.file.display(),
            config.root().display(),
            every_check()
        ));
    }
    Error(format!(
        "{} configures no gate — name at least one of: {}",
        config.file.display(),
        every_check()
    ))
}

fn plan(project: &Project) -> Result<Plan, Error> {
    let mut plan = Plan::default();
    for check in catalogue::CATALOGUE {
        add(project, check, &mut plan)?;
    }
    distinct(project, &plan)?;
    Ok(plan)
}

/// A check's gates, from what the config states for its section and, where it states nothing,
/// from what the section's absence means for this check: an Automatic check runs when its facts
/// are available, and a Policy or Integration check runs nothing until a person writes the
/// section. Planning derives no expensive number or topology. Spec 4.6, 5.2, ADR 0038.
fn add(project: &Project, check: &'static catalogue::Row, plan: &mut Plan) -> Result<(), Error> {
    let Some(stated) = project.config.pinned(check.section) else {
        absent(project, check, plan);
        return Ok(());
    };
    match stated {
        Value::Bool(false) => plan.excluded.push(check.name.to_string()),
        _ if check.gate_per_entry => {
            for (name, _) in contract::named_entries(&project.config, check.section)? {
                plan.gates.push(Gate { name, check });
            }
        }
        _ => plan.one(check),
    }
    Ok(())
}

/// The gate a section's absence plans: the check itself for an Automatic check whose facts are
/// available, and otherwise a check that needs a section a person writes.
fn absent(project: &Project, check: &'static catalogue::Row, plan: &mut Plan) {
    match check.activation == Activation::Automatic && (check.available)(project) {
        true => plan.one(check),
        false => plan.needs_a_section.push(check),
    }
}

fn distinct(project: &Project, plan: &Plan) -> Result<(), Error> {
    let mut seen: Vec<&str> = Vec::new();
    let named = plan
        .gates
        .iter()
        .map(|gate| gate.name.as_str())
        .chain(plan.excluded.iter().map(String::as_str));
    for name in named {
        if name == MEASUREMENT_LOST {
            return Err(Error(format!(
                "{}: {MEASUREMENT_LOST} is the name of klin's own row of files it can no longer \
                 measure, so no gate may take it — name the gate otherwise",
                project.config.file.display()
            )));
        }
        if seen.contains(&name) {
            return Err(Error(format!(
                "{}: two gates are named {name} — a name selects one gate, so each must differ",
                project.config.file.display()
            )));
        }
        seen.push(name);
    }
    Ok(())
}

fn each(
    args: &Args,
    wanted: &[&Gate],
    project: &Project,
    against: &Against,
    out: &mut String,
) -> (Tally, Recorded) {
    let mut tally = Tally::default();
    let mut totals = Recorded::default();
    let mut reported = Vec::new();
    for gate in wanted {
        let ((code, told, records, recorded), ms) =
            journal::timed(|| one(args, gate, project, against));
        let text = match args.hook {
            true => render::stop(&told, code == 0),
            false => render::text(&told),
        };
        match code {
            0 => (),
            1 => tally.failed += 1,
            _ => tally.errored += 1,
        }
        if rendered(args, code, &recorded) {
            printed(args, (&gate.name, status(code)), (&told, &text), out);
        }
        totals
            .gates
            .push(row(gate, code, (&records, &recorded), ms));
        reported.extend(unmeasured_by(&gate.name, &told));
        gather(&mut totals, recorded, &gate.name);
    }
    let base = against.base.as_ref().map(|base| base.before.as_str());
    reported.extend(formless(project, wanted, against));
    let sorted = holes::sorted(project, base, reported);
    stop_unmeasured(args, project, &sorted, (&mut tally, &mut totals), out);
    tally.told += totals.notes.iter().filter(|note| told(note)).count();
    tally.reported = totals
        .findings
        .iter()
        .filter_map(|finding| finding.get("id")?.as_str().map(str::to_string))
        .collect();
    (tally, totals)
}

/// What the Stop says of the files the run could not measure: a lost file the accepted list does
/// not hold fails like any gate, and an opened gap is a note the agent sees. A coverage limit
/// the change did not open says nothing here. Spec 7.2.
fn stop_unmeasured(
    args: &Args,
    project: &Project,
    sorted: &[Unmeasured],
    (tally, totals): (&mut Tally, &mut Recorded),
    out: &mut String,
) {
    let held = holes::held_files(&project.config);
    let failing = failing_lost(sorted, &held);
    let unmatched: Vec<String> = unmatched_lost(&held, sorted).collect();
    if !failing.is_empty() {
        tally.failed += 1;
        tally.grammar_lag = failing.iter().any(|item| item.reason == "parse");
    }
    totals
        .findings
        .extend(failing.iter().map(|item| stop_finding(item)));
    tally.told += unmatched.len()
        + sorted
            .iter()
            .filter(|item| item.class == Class::Opened)
            .count();
    totals.notes.extend(sorted.iter().map(journal_note));
    if args.json {
        return;
    }
    if !failing.is_empty() {
        out.push_str(&render::lost_row(sorted, &held).unwrap_or_default());
    }
    for file in &unmatched {
        let _ = writeln!(out, "  NOTE: {}", render::unmatched_lost_text(file));
    }
    out.push_str(&render::unmeasured_lines(sorted, true));
}

/// The lost files no accepted entry holds, which fail. Spec 7.2.
fn failing_lost<'a>(sorted: &'a [Unmeasured], held: &[String]) -> Vec<&'a Unmeasured> {
    sorted
        .iter()
        .filter(|item| item.class == Class::Lost && !held.contains(&item.file))
        .collect()
}

/// The finding of a lost file as the Stop's journal line records it, under the built-in row.
fn stop_finding(item: &Unmeasured) -> Value {
    let mut finding = render::lost_json(item, false);
    if let Some(fields) = finding.as_object_mut() {
        fields.insert("gate".into(), MEASUREMENT_LOST.into());
    }
    finding
}

/// The journal note of one file the Stop could not measure, under the outcome `klin report`
/// counts: a file no reader read, or a file that left a scope or its text form. Spec 7.2, 13.2.
fn journal_note(item: &Unmeasured) -> Value {
    let outcome = match item.reason {
        "left-scope" | "form" | "filtered" => contract::LOST,
        _ => contract::UNPARSED,
    };
    json!({
        "outcome": outcome,
        "file": item.file,
        "text": item.text,
        "gate": item.gates.first(),
    })
}

/// One gate's block of the text report: its provenance, its status row, and its rendered result.
fn printed(
    args: &Args,
    (gate, state): (&str, &str),
    (told, text): (&[Told], &str),
    out: &mut String,
) {
    if args.json {
        return;
    }
    for line in render::provenance(told) {
        let _ = writeln!(out, "  {line}");
    }
    let _ = writeln!(out, "  {state}  {gate}");
    for line in text.lines() {
        let _ = writeln!(out, "        {line}");
    }
}

/// Whether the text report prints this gate. The hook prints a gate that did not pass, and a
/// passing gate only where it left a note the hook tells, so the agent reads what it must act
/// on. Every other run prints every gate. The records keep every gate either way. Spec 9.5.
fn rendered(args: &Args, code: u8, recorded: &Recorded) -> bool {
    !args.hook || code != 0 || recorded.notes.iter().any(told)
}

/// What one gate's structural work came to, with the declaration states of the gate that builds
/// them. A gate that reads no structural facts records none. Spec 11.2.
fn facts(records: &Records) -> Value {
    let Some(facts) = records.facts else {
        return Value::Null;
    };
    let mut out = serde_json::json!({
        "reads": facts.reads,
        "parses": facts.parses,
        "extracted": facts.extracted,
        "shared": facts.shared,
        "cached": facts.cached,
        "ms": journal::millis(facts.time),
        "cache_read_ms": journal::millis(facts.cache_read),
        "cache_write_ms": journal::millis(facts.cache_write),
    });
    if let (Some(fields), Some(states)) = (out.as_object_mut(), records.states) {
        fields.insert("states".into(), states.into());
    }
    out
}

/// What one name-resolving gate's evidence cost, each tree apart. Spec 11.2.
fn name_evidence(
    cost: &crate::syntax::structural::NameCost,
    layout: Option<base::Layout>,
) -> Value {
    let mut out = Map::new();
    out.insert(
        "layout".into(),
        layout.map_or(Value::Null, |layout| {
            serde_json::json!({
                "written": layout.written,
                "worktree_add_ms": journal::millis(layout.worktree_add),
                "changes_ms": journal::millis(layout.changes),
                "renames_ms": journal::millis(layout.renames),
                "cache_name_ms": journal::millis(layout.cache_name),
                "ignored_ms": journal::millis(layout.ignored),
                "walk_ms": journal::millis(layout.walk),
            })
        }),
    );
    out.insert("base_ms".into(), journal::millis(cost.base).into());
    if let Some(lost) = cost.lost {
        out.insert("lost_ms".into(), journal::millis(lost).into());
    }
    for (tree, part) in [("before", &cost.before), ("after", &cost.after)] {
        let part = serde_json::json!({
            "measure_ms": journal::millis(part.measure),
            "index_ms": journal::millis(part.index),
            "query_ms": journal::millis(part.query),
            "files": part.files,
            "declarations": part.declarations,
            "references": part.references,
            "distinct_names": part.distinct_names,
        });
        out.insert(tree.into(), part);
    }
    Value::Object(out)
}

/// What the facts one gate held cost, and what one of each structural value costs. Spec 11.2.
fn footprint(held: &crate::syntax::structural::footprint::Footprint) -> Value {
    let mut out = Map::new();
    for (name, value) in held.rows() {
        out.insert(name.into(), value.into());
    }
    let mut sizes = Map::new();
    for (name, value) in crate::syntax::structural::footprint::sizes() {
        sizes.insert(name.into(), value.into());
    }
    out.insert("sizes".into(), Value::Object(sizes));
    out.insert(
        "reference_canonical_allocation_ratio".into(),
        json!(if held.reference_distinct_names == 0 {
            0.0
        } else {
            held.reference_canonical_allocations as f64 / held.reference_distinct_names as f64
        }),
    );
    Value::Object(out)
}

/// One gate's row in the JSON: what it is called, what it came to, how many findings and notes
/// it left, the scope it measured, how long its own measure and judge took, the count its `OK:`
/// line prints as held at the base, and the structural facts it extracted or shared. Spec 11.2.
fn row(gate: &Gate, code: u8, (records, recorded): (&Records, &Recorded), ms: u64) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), gate.name.clone().into());
    out.insert("status".into(), status(code).trim_end().into());
    out.insert("findings".into(), recorded.findings.len().into());
    out.insert("notes".into(), recorded.notes.len().into());
    out.insert(
        "coverage".into(),
        records.coverage.clone().unwrap_or(Value::Null),
    );
    out.insert("ms".into(), ms.into());
    out.insert("held".into(), records.held.map_or(Value::Null, Value::from));
    out.insert(
        "accepted".into(),
        records.accepted.map_or(Value::Null, Value::from),
    );
    out.insert("facts".into(), facts(records));
    out.insert(
        "names".into(),
        records
            .names
            .as_ref()
            .map_or(Value::Null, |names| name_evidence(names, records.layout)),
    );
    out.insert(
        "footprint".into(),
        records.footprint.as_ref().map_or(Value::Null, footprint),
    );
    costs(&mut out, records);
    Value::Object(out)
}

/// The content, module-graph and public-surface work of one gate's row. Spec 11.2.
fn costs(out: &mut Map<String, Value>, records: &Records) {
    out.insert(
        "work".into(),
        records.work.map_or(Value::Null, |work| {
            serde_json::json!({
                "reads": work.reads,
                "parses": work.parses,
            })
        }),
    );
    out.insert(
        "graph".into(),
        records.graph.map_or(Value::Null, |graph| {
            serde_json::json!({
                "modules": graph.modules,
                "sources": graph.sources,
                "dependencies": graph.dependencies,
                "edges": graph.edges,
                "dispatches": by_language(graph.dispatched()),
                "ms": journal::millis(graph.time),
            })
        }),
    );
    out.insert(
        "surface".into(),
        records.surface.map_or(Value::Null, |surface| {
            serde_json::json!({
                "surfaces": surface.surfaces,
                "items": surface.items,
                "measured": surface.measured,
                "opaque": surface.opaque,
                "holes": surface.holes,
                "dispatches": by_language(surface.dispatched()),
                "ms": journal::millis(surface.time),
            })
        }),
    );
}

/// One count per structural language, under the name a config names the language by.
fn by_language(counts: impl Iterator<Item = (LanguageId, usize)>) -> Value {
    let names = structural::languages();
    counts
        .filter_map(|(language, count)| {
            let (name, _) = names.iter().find(|(_, id)| *id == language)?;
            Some((name.to_string(), Value::from(count)))
        })
        .collect::<Map<String, Value>>()
        .into()
}

/// A note the hook tells a person even when nothing blocks the stop: a file the run could not
/// read or stopped measuring, and a deleted test the run let through. Spec 8.2, 8.6, 14.
fn told(note: &Value) -> bool {
    let outcome = note.get("outcome").and_then(Value::as_str);
    matches!(
        outcome,
        Some(DELETED | DERIVATION | UNRESOLVED | AMBIGUOUS | UNBUILT)
    )
}

/// The reason of a form a resolver found two answers for, which the Stop tells as it tells an
/// unresolved one. Spec 7.2.
const AMBIGUOUS: &str = "ambiguous";

/// The files in the run's scope that the working tree's `.gitattributes` make not text, in a
/// language a selected gate that reads code reads, so the run sorts them though no gate saw
/// them. Spec 7.2.
fn formless(project: &Project, wanted: &[&Gate], against: &Against) -> Vec<(String, String, Seen)> {
    let Some(gate) = wanted.iter().find(|gate| gate.check.reads_code()) else {
        return Vec::new();
    };
    project
        .tree()
        .formless()
        .iter()
        .filter(|(file, _)| crate::syntax::language_of(file).is_some())
        .filter(|(file, _)| {
            against
                .scope
                .as_ref()
                .is_none_or(|scope| scope.contains(file))
        })
        .map(|(file, form)| (gate.name.clone(), file.clone(), Seen::Form(*form)))
        .collect()
}

/// The files one gate could not measure, as it saw them, for the run to sort once. Spec 7.2.
fn unmeasured_by(gate: &str, told: &[Told]) -> Vec<(String, String, Seen)> {
    let seen = |file: &str, seen| (gate.to_string(), file.to_string(), seen);
    told.iter()
        .flat_map(|item| match item {
            Told::Hole(Hole::Unparsed(files)) => files
                .iter()
                .map(|file| seen(&file.file, Seen::Unread))
                .collect(),
            Told::Hole(Hole::Lost(site)) => vec![seen(&site.file, Seen::Left(site.text.clone()))],
            Told::Hole(Hole::Manifest { site, class }) => vec![seen(
                &site.file,
                Seen::Manifest {
                    class: *class,
                    why: site.text.clone(),
                },
            )],
            _ => Vec::new(),
        })
        .collect()
}

fn gather(totals: &mut Recorded, mut records: Recorded, name: &str) {
    for record in records.findings.iter_mut().chain(records.notes.iter_mut()) {
        if let Some(fields) = record.as_object_mut() {
            fields.insert("gate".into(), name.into());
        }
    }
    totals.findings.append(&mut records.findings);
    totals.notes.append(&mut records.notes);
    totals.derived.append(&mut records.derived);
}

fn code(tally: &Tally) -> u8 {
    if tally.errored > 0 {
        2
    } else if tally.failed > 0 {
        1
    } else {
        0
    }
}

fn select<'a>(named: &[String], plan: &'a Plan, project: &Project) -> Result<Vec<&'a Gate>, Error> {
    for name in named {
        known(name, plan, project)?;
    }
    let wanted: Vec<&Gate> = plan
        .gates
        .iter()
        .filter(|gate| named.is_empty() || named.iter().any(|wanted| wanted == &gate.name))
        .collect();
    if wanted.is_empty() {
        return Err(no_gate(project, plan));
    }
    Ok(wanted)
}

fn known(name: &str, plan: &Plan, project: &Project) -> Result<(), Error> {
    let config = &project.config;
    if plan.gates.iter().any(|gate| gate.name == name) {
        return Ok(());
    }
    if plan.excluded.iter().any(|excluded| excluded == name) {
        return Err(Error(format!(
            "the gate named {name} is excluded in {} — naming a gate is a claim that it runs, \
             so lift the exclusion or drop {name}",
            config.file.display()
        )));
    }
    Err(Error(format!(
        "no gate named {name} — {} configures: {}",
        config.file.display(),
        match plan.gates.is_empty() {
            true => "nothing".to_string(),
            false => names(plan.gates.iter().map(|gate| gate.name.as_str())),
        }
    )))
}

fn one(
    args: &Args,
    gate: &Gate,
    project: &Project,
    against: &Against,
) -> (u8, Vec<Told>, Records, Recorded) {
    let mut told = Vec::new();
    let mut records = Records::default();
    let at = Context {
        gate: &gate.name,
        project,
        prior: against.prior.as_ref(),
        base: against.base.as_ref().map(|base| base.before.as_str()),
        only: against.scope.as_deref().filter(|_| gate.check.takes_scope),
        changes: against.changes.as_deref(),
        caller: match args.hook {
            true => Caller::Hook,
            false => Caller::Gate,
        },
        strict: args.strict && gate.check.needs.the_commit(),
    };
    let outcome = (gate.check.run)(
        &at,
        &mut Sink {
            told: &mut told,
            records: &mut records,
        },
    );
    let code = match outcome {
        Ok(code) => code,
        Err(problem) => {
            records.errors.push(problem.to_string());
            told.push(Plain::Error(problem.to_string()).into());
            2
        }
    };
    let mut recorded = Recorded::from(render::json(&told));
    errored(code, &mut records.errors, &mut recorded);
    (code, told, records, recorded)
}

/// The one `error` finding of a gate that is exit 2 and recorded no other finding, from the
/// reasons it gave. A gate with findings already names what failed, so it gets none. Spec 11.2.
fn errored(code: u8, errors: &mut Vec<String>, recorded: &mut Recorded) {
    if code != 2 || !recorded.findings.is_empty() || errors.is_empty() {
        return;
    }
    let errors = std::mem::take(errors);
    recorded.findings.push(record("error", &errors.join("\n")));
}

fn status(code: u8) -> &'static str {
    match code {
        0 => "ok  ",
        1 => "FAIL",
        _ => "ERR ",
    }
}

fn summary(failed: usize, errored: usize) -> String {
    let mut parts = Vec::new();
    if failed > 0 {
        parts.push(format!("{failed} failed"));
    }
    if errored > 0 {
        let plural = if errored == 1 { "" } else { "s" };
        parts.push(format!("{errored} tool error{plural}"));
    }
    match parts.is_empty() {
        true => "all passed.".to_string(),
        false => parts.join(", ") + ".",
    }
}

/// One error of spec 7.3: its kind, and what went wrong.
struct Fault {
    kind: ErrorKind,
    error: Error,
}

impl From<Fault> for Error {
    fn from(fault: Fault) -> Error {
        fault.error
    }
}

/// The error kinds of spec 7.3 the runner tells apart.
#[derive(Clone, Copy)]
enum ErrorKind {
    Invocation,
    Configuration,
    Base,
    Git,
    Internal,
}

impl ErrorKind {
    fn name(self) -> &'static str {
        match self {
            ErrorKind::Invocation => "invocation",
            ErrorKind::Configuration => "configuration",
            ErrorKind::Base => "base",
            ErrorKind::Git => "git",
            ErrorKind::Internal => "internal",
        }
    }
}

fn fault(kind: ErrorKind) -> impl Fn(Error) -> Fault {
    move |error| Fault { kind, error }
}

/// A judgement of spec 7.1, in the order aggregation takes the worst of.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
enum Judgement {
    #[default]
    Pass,
    Review,
    Fail,
}

impl Judgement {
    fn name(self) -> &'static str {
        match self {
            Judgement::Pass => "pass",
            Judgement::Review => "review",
            Judgement::Fail => "fail",
        }
    }
}

/// The three axes of one gate, or of a whole run. Spec 4.5, 7.5.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Axes {
    judgement: Judgement,
    incomplete: bool,
    error: bool,
}

impl Axes {
    fn and(self, other: Axes) -> Axes {
        Axes {
            judgement: self.judgement.max(other.judgement),
            incomplete: self.incomplete || other.incomplete,
            error: self.error || other.error,
        }
    }

    /// The one state that wins: an error, then a failing finding, then a hole. Spec 7.4.
    fn worst(self) -> Worst {
        match self {
            Axes { error: true, .. } => Worst::Error,
            Axes {
                judgement: Judgement::Fail,
                ..
            } => Worst::Fail,
            Axes {
                incomplete: true, ..
            } => Worst::Incomplete,
            _ => Worst::Ok,
        }
    }

    /// ERROR 2 > FAIL 1 > INCOMPLETE 3 > success 0. Spec 7.4.
    fn exit(self) -> u8 {
        match self.worst() {
            Worst::Error => 2,
            Worst::Fail => 1,
            Worst::Incomplete => 3,
            Worst::Ok => 0,
        }
    }

    /// The row word: the first of `ERR`, `FAIL`, `INCOMPLETE`, `ok`. Spec 11.3.
    fn state(self) -> &'static str {
        match self.worst() {
            Worst::Error => "ERR ",
            Worst::Fail => "FAIL",
            Worst::Incomplete => "INCOMPLETE",
            Worst::Ok => "ok  ",
        }
    }

    fn measurement(self) -> &'static str {
        match self.incomplete {
            true => "incomplete",
            false => "complete",
        }
    }

    fn execution(self) -> &'static str {
        match self.error {
            true => "error",
            false => "ok",
        }
    }
}

/// The state of a gate or run that the exit code and the row word both follow.
#[derive(Clone, Copy)]
enum Worst {
    Error,
    Fail,
    Incomplete,
    Ok,
}

/// One gate's axes from its exit code and its result. A gate that erred still carries the
/// findings it validated, so they still fail it. Spec 7.5.
fn axes(code: u8, recorded: &Recorded) -> Axes {
    let fails = code == 1 || recorded.findings.iter().any(failing);
    Axes {
        judgement: match fails {
            true => Judgement::Fail,
            false => Judgement::Pass,
        },
        incomplete: !recorded.holes.is_empty(),
        error: code == 2,
    }
}

fn failing(finding: &Value) -> bool {
    matches!(
        finding.get("outcome").and_then(Value::as_str),
        Some("new" | "worsened")
    )
}

/// What one `klin check` came to, gathered for the check document and its text. Spec 11.7.
#[derive(Default)]
struct Report {
    config: Option<Value>,
    window: Option<Value>,
    axes: Axes,
    /// Whether a run-scope error stopped the run before any capability measured. Spec 7.3.
    stopped: bool,
    capabilities: Vec<Value>,
    findings: Vec<Value>,
    notes: Vec<Value>,
    measurements: Vec<Value>,
    holes: Vec<Incomplete>,
    /// The files the selected gates measured, all of them and the ones gates that read code
    /// measured, which decide the hole of a whole run that measured nothing. Spec 7.2.
    measured: Measured,
    not_measured: std::collections::BTreeSet<String>,
    /// What each gate could not measure, as it saw it, for the run to sort once. Spec 7.2.
    reported: Vec<(String, String, Seen)>,
    reviews: Vec<Value>,
    errors: Vec<Value>,
    gates: Vec<Value>,
}

/// One `klin check`: each run-scope step under the kind of error it can raise, every selected
/// gate, and the check document or its text. Spec 7, 11.3, 11.7.
fn checked(args: &Args, start: &Path, loaded: Result<Project, Error>, out: &mut String) -> u8 {
    let located = config::located(args.config.as_deref(), start);
    let named_nothing =
        args.config.is_some() && !located.as_ref().is_some_and(|file| file.exists());
    let mut report = Report::default();
    let measured = loaded
        .map_err(fault(match named_nothing {
            true => ErrorKind::Invocation,
            false => ErrorKind::Configuration,
        }))
        .and_then(|mut project| {
            report.config = Some(json!({
                "path": project.config.file.display().to_string(),
                "present": project.config.written(),
            }));
            measured(args, &mut project, &mut report, out)
        });
    if let Err(fault) = measured {
        report.config.get_or_insert_with(|| {
            json!({
                "path": located.as_ref().map(|file| file.display().to_string()),
                "present": located.as_ref().is_some_and(|file| file.is_file()),
            })
        });
        report.stop(args, fault, out);
    }
    report.finish(args, out)
}

fn measured(
    args: &Args,
    project: &mut Project,
    report: &mut Report,
    out: &mut String,
) -> Result<(), Fault> {
    let window = base::choose(project.root(), args.strict).ok();
    if let Some(window) = &window {
        project.bind(window);
    }
    let project = &*project;
    let plan = plan(project).map_err(fault(ErrorKind::Configuration))?;
    let (wanted, unsupported) = chosen_gates(&args.gates, &plan, project)?;
    let against = against(args, &wanted, project, window.as_ref(), out)?;
    report.ran(args, project, (&plan, &wanted, unsupported), &against, out);
    Ok(())
}

/// The gates a run selects, and the capabilities a selector named that do not apply to this
/// tree or need a section the configuration does not hold. A name klin does not know, and a gate
/// a person set to `false`, is an invocation error. Spec 7.2, 7.3.
fn chosen_gates<'a>(
    named: &[String],
    plan: &'a Plan,
    project: &Project,
) -> Result<(Vec<&'a Gate>, Vec<&'static catalogue::Row>), Fault> {
    if named.is_empty() && plan.gates.is_empty() && !plan.excluded.is_empty() {
        return Err(Fault {
            kind: ErrorKind::Configuration,
            error: no_gate(project, plan),
        });
    }
    let mut unsupported = Vec::new();
    for name in named {
        let missing = plan.needs_a_section.iter().find(|check| check.name == name);
        match missing {
            Some(check) if !plan.gates.iter().any(|gate| &gate.name == name) => {
                unsupported.push(*check)
            }
            _ => known(name, plan, project).map_err(fault(ErrorKind::Invocation))?,
        }
    }
    let wanted = plan
        .gates
        .iter()
        .filter(|gate| named.is_empty() || named.contains(&gate.name))
        .collect();
    Ok((wanted, unsupported))
}

/// The hole of a whole-tree run in which the checks that read code measured nothing, because
/// the survey found no source root. Spec 7.2.
/// How many files the gates of a run measured: all of them, and the ones a gate that reads code
/// measured.
#[derive(Default, Clone, Copy)]
struct Measured {
    files: u64,
    code: u64,
}

impl Measured {
    fn add(&mut self, check: &catalogue::Row, records: &Records) {
        let measured = records
            .coverage
            .as_ref()
            .and_then(|coverage| coverage.get("measured")?.as_u64())
            .unwrap_or(0);
        self.files += measured;
        if check.reads_code() {
            self.code += measured;
        }
    }
}

/// The hole of a whole run that a check reading code was left to measure and that measured no
/// code: where the survey found no source root and nothing at all was measured, or where the
/// derivation commit held a source root that no source file of the working tree sits under. A documentation-only tree whose documents were measured is no
/// hole, and a tree that lost its source does not pass on its documents. #500 owns the finer
/// classification of the lost source. Spec 7.2.
fn unmeasured_run(
    args: &Args,
    (plan, wanted): (&Plan, &[&Gate]),
    project: &Project,
    measured: Measured,
) -> Option<Incomplete> {
    if !leaves_code(args, plan, wanted, project) || measured.code > 0 {
        return None;
    }
    let lost = held_roots(project);
    let nothing = measured.files == 0 && project.found_no_source_root();
    let text = match (nothing, lost.is_empty()) {
        (_, false) => format!(
            "the derivation commit held source under {} and the working tree holds no source \
             file there, so no check that reads code measured anything — restore the source, or \
             set those gates to false to exclude them",
            lost.join(", ")
        ),
        (true, true) => format!(
            "the repository holds no language or document klin measures: {} — write klin.json \
             naming one of: {}",
            no_source_root_said(project),
            every_check()
        ),
        (false, true) => return None,
    };
    Some(Incomplete {
        reason: Reason::NothingMeasured,
        detail: None,
        text,
    })
}

/// The source roots the derivation commit's survey held that no source file of the working
/// tree sits under any more. A file of no language klin reads, left where the source was, keeps
/// no root.
fn held_roots(project: &Project) -> Vec<String> {
    let files = project.tree().files().unwrap_or_default();
    let holds = |root: &str| {
        files.iter().any(|file| {
            survey::surveyed(file)
                && survey::language_of(file).is_some()
                && crate::scope::under_or_at(file, root)
        })
    };
    project
        .source_derivation()
        .map(|(held, _, _)| {
            held.roots
                .iter()
                .filter(|root| !holds(root))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

impl Report {
    /// Every selected gate, the rows of what a selector named that does not apply, and the hole
    /// of a whole run that measured nothing. Spec 7.2, 11.7.
    fn ran(
        &mut self,
        args: &Args,
        project: &Project,
        (plan, wanted, unsupported): (&Plan, &[&Gate], Vec<&'static catalogue::Row>),
        against: &Against,
        out: &mut String,
    ) {
        self.window = against.base.as_ref().map(Window::record);
        for gate in wanted {
            self.gate(args, gate, project, against, out);
        }
        let base = against.base.as_ref().map(|base| base.before.as_str());
        let mut reported = std::mem::take(&mut self.reported);
        reported.extend(formless(project, wanted, against));
        self.unmeasured(args, project, &holes::sorted(project, base, reported), out);
        self.not_read(wanted, &by_extension(project, against));
        if !args.changed
            && let Some(hole) = unmeasured_run(args, (plan, wanted), project, self.measured)
        {
            self.holes.push(hole);
        }
        for check in unsupported {
            self.unsupported(args, check, out);
        }
        if args.gates.is_empty() {
            self.not_applicable(plan);
        }
    }

    fn gate(
        &mut self,
        args: &Args,
        gate: &Gate,
        project: &Project,
        against: &Against,
        out: &mut String,
    ) {
        if let (Some(why), true) = (&against.unlaid, gate.check.needs.the_tree()) {
            return self.unlaid(args, gate, why, out);
        }
        let ((code, told, records, recorded), ms) =
            journal::timed(|| one(args, gate, project, against));
        let axes = axes(code, &recorded);
        self.axes = self.axes.and(axes);
        printed(
            args,
            (&gate.name, axes.state()),
            (&told, &render::text(&told)),
            out,
        );
        self.gates.push(row(gate, code, (&records, &recorded), ms));
        self.measured.add(gate.check, &records);
        self.capabilities.push(active_row(gate, axes, &records));
        self.measurements.push(measurement(
            &gate.name,
            gate.check,
            axes,
            (&recorded.holes, &recorded.derived),
        ));
        self.errors.extend(gate_errors(&gate.name, code, &told));
        self.reported.extend(unmeasured_by(&gate.name, &told));
        self.gathered(&gate.name, recorded);
    }

    /// The files the run could not measure, sorted once: the `measurement-lost` row and its
    /// findings, a review item per opened gap, a coverage note per limit, and the gap and limit
    /// counts on the row of each gate that reported one. Spec 7.2, 11.7.
    fn unmeasured(
        &mut self,
        args: &Args,
        project: &Project,
        sorted: &[Unmeasured],
        out: &mut String,
    ) {
        let held = holes::held_files(&project.config);
        if let Some(row) = render::lost_row(sorted, &held) {
            self.lost(sorted, &held);
            if !args.json {
                out.push_str(&row);
            }
        }
        self.reviewed(sorted);
        self.reviews
            .extend(unmatched_lost(&held, sorted).map(|file| render::unmatched_lost_json(&file)));
        if !args.json {
            out.push_str(&render::unmeasured_lines(sorted, false));
        }
    }

    /// A review item per opened gap and a coverage note per limit, every sorted file counted as
    /// not measured, and each gate's gap and limit counts. Spec 7.2, 11.7.
    fn reviewed(&mut self, sorted: &[Unmeasured]) {
        for item in sorted {
            self.not_measured.insert(item.file.clone());
            match item.class {
                Class::Lost => (),
                Class::Opened if item.reason == "left-scope" && self.fails_at(&item.file) => (),
                Class::Opened => self.reviews.push(render::opened_json(item)),
                Class::Limit => self.notes.push(render::limit_json(item)),
            }
        }
        for row in &mut self.capabilities {
            sort_counted(row, sorted);
        }
    }

    /// Whether a gate failed a finding at this file, which a file that left a scope keeps under
    /// the base's scope, so the file is that FAIL and not also a review item. Spec 7.2.
    fn fails_at(&self, file: &str) -> bool {
        self.findings
            .iter()
            .any(|finding| finding["file"] == file && failing(finding))
    }

    /// The files in the run's scope in a language each gate that reads code does not read, on
    /// that gate's row. Spec 7.2, 11.7.
    fn not_read(&mut self, wanted: &[&Gate], counted: &BTreeMap<String, usize>) {
        for gate in wanted.iter().filter(|gate| gate.check.reads_code()) {
            let not_read: usize = counted
                .iter()
                .filter(|(extension, _)| !reads(gate.check, extension))
                .map(|(_, count)| count)
                .sum();
            let row = self
                .capabilities
                .iter_mut()
                .find(|row| row["name"] == gate.name.as_str());
            if let Some(coverage) = row.and_then(|row| row["coverage"].as_object_mut()) {
                coverage.insert("not_read".into(), not_read.into());
            }
        }
    }

    /// The built-in row of the lost files and their findings, held where an accepted entry names
    /// the file. Spec 7.2, 11.7.
    fn lost(&mut self, sorted: &[Unmeasured], held: &[String]) {
        let lost: Vec<&Unmeasured> = sorted
            .iter()
            .filter(|item| item.class == Class::Lost)
            .collect();
        let accepted = lost.iter().filter(|item| held.contains(&item.file)).count();
        let axes = Axes {
            judgement: match accepted < lost.len() {
                true => Judgement::Fail,
                false => Judgement::Pass,
            },
            ..Axes::default()
        };
        self.axes = self.axes.and(axes);
        self.findings.extend(
            lost.iter()
                .map(|item| render::lost_json(item, held.contains(&item.file))),
        );
        self.capabilities.push(json!({
            "name": MEASUREMENT_LOST,
            "kind": "built-in",
            "placement": ["stop", "check"],
            "state": "active",
            "judgement": axes.judgement.name(),
            "measurement": axes.measurement(),
            "execution": axes.execution(),
            "coverage": null,
            "coverage_claim": "verified",
            "held": accepted,
            "accepted": accepted,
        }));
    }

    /// The row of a gate that reads the base tree klin could not lay out. The other gates still
    /// run. Spec 7.3.
    fn unlaid(&mut self, args: &Args, gate: &Gate, why: &str, out: &mut String) {
        let message = format!("the base tree could not be laid out for this gate: {why}");
        let told = [Told::Plain(Plain::Error(message.clone()))];
        let axes = Axes {
            error: true,
            ..Axes::default()
        };
        self.axes = self.axes.and(axes);
        printed(
            args,
            (&gate.name, axes.state()),
            (&told, &render::text(&told)),
            out,
        );
        self.capabilities
            .push(active_row(gate, axes, &Records::default()));
        self.measurements
            .push(measurement(&gate.name, gate.check, axes, (&[], &[])));
        self.errors
            .push(error_record(ErrorKind::Git, Some(&gate.name), &message));
    }

    /// The findings and notes of one gate, under the names the check document gives them.
    fn gathered(&mut self, gate: &str, recorded: Recorded) {
        self.reviews.extend(
            recorded
                .reviews
                .into_iter()
                .map(|review| review_record(gate, review)),
        );
        self.findings.extend(
            recorded
                .findings
                .into_iter()
                .filter(failing)
                .map(|finding| finding_record(gate, finding)),
        );
        self.notes.extend(
            recorded
                .notes
                .into_iter()
                .map(|note| note_record(gate, note)),
        );
    }

    /// The row of a capability a selector named that does not apply to this tree. Spec 7.2.
    fn unsupported(&mut self, args: &Args, check: &'static catalogue::Row, out: &mut String) {
        let told = [Told::Incomplete(Incomplete {
            reason: Reason::Unsupported,
            detail: None,
            text: format!(
                "{} does not apply to this tree, or needs a \"{}\" section klin.json does not \
                 hold, so it measured nothing",
                check.name, check.section
            ),
        })];
        let axes = Axes {
            incomplete: true,
            ..Axes::default()
        };
        self.axes = self.axes.and(axes);
        printed(
            args,
            (check.name, axes.state()),
            (&told, &render::text(&told)),
            out,
        );
        let holes = render::json(&told).holes;
        self.capabilities.push(inapplicable(check, Some(axes)));
        self.measurements
            .push(measurement(check.name, check, axes, (&holes, &[])));
    }

    /// The rows of the capabilities that do not apply, which a whole run lists. Spec 11.7.
    fn not_applicable(&mut self, plan: &Plan) {
        self.capabilities.extend(
            plan.needs_a_section
                .iter()
                .map(|check| inapplicable(check, None)),
        );
    }

    /// A run-scope error, which stops the run before any capability measures. Spec 7.3.
    fn stop(&mut self, args: &Args, fault: Fault, out: &mut String) {
        self.stopped = true;
        self.axes.error = true;
        if !args.json {
            let _ = writeln!(out, "ERR: {}", fault.error);
        }
        self.errors
            .push(error_record(fault.kind, None, &fault.error.to_string()));
    }

    fn finish(mut self, args: &Args, out: &mut String) -> u8 {
        self.axes.incomplete |= !self.holes.is_empty();
        if !self.reviews.is_empty() {
            self.axes.judgement = self.axes.judgement.max(Judgement::Review);
        }
        let exit = self.axes.exit();
        let (judgement, measurement) = match self.stopped {
            true => ("none", "none"),
            false => (self.axes.judgement.name(), self.axes.measurement()),
        };
        if !args.json {
            for hole in &self.holes {
                let mut said = String::new();
                render::incomplete(hole, &mut said);
                let _ = write!(out, "  {said}");
            }
            let _ = writeln!(
                out,
                "judgement: {judgement}, measurement: {measurement}, execution: {}, exit {exit}",
                self.axes.execution()
            );
            return exit;
        }
        out.clear();
        let _ = writeln!(out, "{}", self.document(exit));
        exit
    }

    /// The check document of spec 11.7.
    fn document(self, exit: u8) -> Value {
        let ran = !self.stopped;
        let mut measurements = self.measurements;
        if ran {
            let holes: Vec<Value> = self.holes.iter().map(hole_record).collect();
            measurements.insert(
                0,
                json!({
                    "check": null,
                    "basis": {},
                    "state": if holes.is_empty() { "complete" } else { "incomplete" },
                    "holes": holes,
                }),
            );
        }
        json!({
            "schema_version": 1,
            "command": "check",
            "klin": { "version": env!("CARGO_PKG_VERSION") },
            "config": self.config,
            "window": self.window,
            "tree": null,
            "judgement": ran.then(|| self.axes.judgement.name()),
            "measurement": ran.then(|| self.axes.measurement()),
            "execution": self.axes.execution(),
            "exit": exit,
            "capabilities": self.capabilities,
            "findings": self.findings,
            "reviews": self.reviews,
            "notes": self.notes,
            "measurements": measurements,
            "not_measured": self.not_measured.len(),
            "errors": self.errors,
            "diagnostics": { "gates": self.gates },
        })
    }
}

/// The row of a gate the run selected, with its axes and what its run recorded. Spec 11.7.
fn active_row(gate: &Gate, axes: Axes, records: &Records) -> Value {
    json!({
        "name": gate.name,
        "kind": gate.check.kind(),
        "placement": gate.check.placement.names(),
        "state": "active",
        "judgement": axes.judgement.name(),
        "measurement": axes.measurement(),
        "execution": axes.execution(),
        "coverage": records.coverage.as_ref().map(checked_coverage),
        "coverage_claim": coverage_claim(gate.check),
        "held": records.held,
        "accepted": records.accepted,
    })
}

/// How many files of the run's scope each extension names, of the files in a language the
/// survey knows, counted once for every gate. A whole run's scope is every surveyed file, and a
/// changed run's is its changed files. Spec 7.2.
fn by_extension(project: &Project, against: &Against) -> BTreeMap<String, usize> {
    let listed = project.tree().files().unwrap_or_default();
    let files: Vec<&String> = match &against.scope {
        Some(scope) => scope.iter().collect(),
        None => listed.iter().collect(),
    };
    let mut counted = BTreeMap::new();
    for file in files {
        if !survey::surveyed(file) || survey::language_of(file).is_none() {
            continue;
        }
        if let Some((_, extension)) = file.rsplit_once('.') {
            *counted.entry(format!(".{extension}")).or_insert(0) += 1;
        }
    }
    counted
}

/// Whether a gate reads files of this extension, by the extensions its languages name.
fn reads(check: &catalogue::Row, extension: &str) -> bool {
    let quoted = format!("`{extension}`");
    check.languages.is_some_and(|languages| {
        languages()
            .iter()
            .any(|(_, listed)| listed.contains(&quoted))
    })
}

/// The files the accepted list holds as lost that the run did not find unmeasurable for any
/// reason, so their entries match nothing. An entry stays matched while its file is lost, a gap
/// or a limit, so it never goes stale while the grammar still lags. Spec 7.2, 7.6.
fn unmatched_lost<'a>(
    held: &'a [String],
    sorted: &'a [Unmeasured],
) -> impl Iterator<Item = String> + 'a {
    held.iter()
        .filter(|file| !sorted.iter().any(|item| &item.file == *file))
        .cloned()
}

/// A gate's coverage as the check document names it, with the files of a language it does not
/// read and the gaps and limits that the run fills in once it has sorted them. Spec 11.7.
fn checked_coverage(coverage: &Value) -> Value {
    let count = |key: &str| coverage.get(key).cloned().unwrap_or(Value::from(0));
    json!({
        "found": count("found"),
        "measured": count("measured"),
        "not_read": 0,
        "excluded": count("excluded"),
        "gaps": 0,
        "limits": 0,
    })
}

/// How many of the files the run sorted this gate reported as opened gaps and as limits.
fn sort_counted(row: &mut Value, sorted: &[Unmeasured]) {
    let Some(name) = row.get("name").and_then(Value::as_str).map(str::to_string) else {
        return;
    };
    let Some(coverage) = row.get_mut("coverage").and_then(Value::as_object_mut) else {
        return;
    };
    for (key, class) in [("gaps", Class::Opened), ("limits", Class::Limit)] {
        let count = sorted
            .iter()
            .filter(|item| item.class == class && item.gates.contains(&name))
            .count();
        coverage.insert(key.into(), count.into());
    }
}

/// A review item a gate told, under the gate that told it. Spec 11.7.
fn review_record(gate: &str, review: Value) -> Value {
    let Value::Object(mut fields) = review else {
        return review;
    };
    fields.remove("outcome");
    fields.insert("check".into(), gate.into());
    fields.insert("kind".into(), holes::UNMEASURED.into());
    Value::Object(fields)
}

fn hole_record(hole: &Incomplete) -> Value {
    json!({ "reason": hole.reason.name(), "detail": hole.detail, "text": hole.text })
}

/// An integration claims only the results its report states, never that a file it did not
/// report on is clean. Spec 9.4.
fn coverage_claim(check: &catalogue::Row) -> &'static str {
    match check.activation {
        Activation::Integration => "unverified",
        Activation::Automatic | Activation::Policy => "verified",
    }
}

/// The row of a capability that does not apply to this tree, with the axes of a selector that
/// named it. Spec 11.7.
fn inapplicable(check: &catalogue::Row, named: Option<Axes>) -> Value {
    json!({
        "name": check.name,
        "kind": check.kind(),
        "placement": check.placement.names(),
        "state": "not-applicable",
        "judgement": null,
        "measurement": named.map(Axes::measurement),
        "execution": "ok",
        "coverage": null,
        "coverage_claim": coverage_claim(check),
        "held": null,
        "accepted": null,
    })
}

/// One measurement record: what produced it, the policy it used, and its holes. Spec 8.1, 11.7.
fn measurement(
    name: &str,
    check: &catalogue::Row,
    axes: Axes,
    (holes, policy): (&[Value], &[Value]),
) -> Value {
    json!({
        "check": name,
        "basis": {
            "producer": { "capability": check.name, "semantics_version": check.semantics },
            "policy": policy,
        },
        "state": axes.measurement(),
        "holes": holes,
    })
}

/// The capability-scope errors of a gate that is exit 2, each under its kind. A gate that
/// could not read its own configuration is a configuration error. A file or form klin could not
/// measure is klin's own limit until spec 7.2 sorts it into its class. Spec 7.3.
fn gate_errors(gate: &str, code: u8, told: &[Told]) -> Vec<Value> {
    if code != 2 {
        return Vec::new();
    }
    let mut errors: Vec<Value> = told.iter().flat_map(|item| erred(gate, item)).collect();
    if errors.is_empty() {
        errors.push(error_record(
            ErrorKind::Internal,
            Some(gate),
            "the gate stopped before it finished its measurement",
        ));
    }
    errors
}

/// The errors one item of a gate's result names: one per site for a file or form klin could
/// not measure, with its site and its 0.x outcome as the reason, so no site is lost.
fn erred(gate: &str, item: &Told) -> Vec<Value> {
    let one = |kind, message: &str| vec![error_record(kind, Some(gate), message)];
    match item {
        Told::Plain(Plain::Error(problem)) => one(ErrorKind::Configuration, problem),
        Told::Plain(Plain::PathMissing(named)) => one(
            ErrorKind::Configuration,
            &format!("{named} — correct the path, or take it out of \"in\"."),
        ),
        _ => Vec::new(),
    }
}

fn error_record(kind: ErrorKind, check: Option<&str>, message: &str) -> Value {
    json!({ "kind": kind.name(), "check": check, "message": message })
}

/// A failing finding as the check document names it. Spec 11.7.
fn finding_record(gate: &str, finding: Value) -> Value {
    let Value::Object(mut fields) = finding else {
        return finding;
    };
    fields.insert("check".into(), gate.into());
    fields.insert("kind".into(), "metric".into());
    if let Some(remedy) = fields.remove("fix_advice") {
        fields.insert("remedy".into(), remedy);
    }
    Value::Object(fields)
}

/// A note as the check document names it: its kind, and the words a person reads. Spec 11.7.
fn note_record(gate: &str, note: Value) -> Value {
    let Value::Object(mut fields) = note else {
        return note;
    };
    let kind = fields.remove("outcome").unwrap_or(Value::Null);
    let message = fields.remove("text").unwrap_or_else(|| kind.clone());
    if fields.get("file").and_then(Value::as_str) == Some("") {
        fields.remove("file");
    }
    fields.insert("check".into(), gate.into());
    fields.insert("kind".into(), kind);
    fields.entry("coverage").or_insert(false.into());
    fields.insert("message".into(), message);
    Value::Object(fields)
}

/// The aggregation of spec 7.5 over in-memory results, for the holes no shipped capability can
/// reach yet. `work-limit` is one: no shipped capability has a work bound. AGENTS.md.
#[cfg(test)]
mod tests {
    use super::*;

    fn work_limited() -> Recorded {
        Recorded::from(render::json(&[Told::Incomplete(Incomplete {
            reason: Reason::WorkLimit,
            detail: None,
            text: "stopped at its bound".to_string(),
        })]))
    }

    #[test]
    fn a_work_limit_hole_makes_the_gate_incomplete_and_the_run_exit_3() {
        let gate = axes(0, &work_limited());

        assert_eq!(gate.state(), "INCOMPLETE");
        assert_eq!(gate.measurement(), "incomplete");
        assert_eq!(Axes::default().and(gate).exit(), 3);
        assert_eq!(work_limited().holes[0]["reason"], "work-limit");
    }

    #[test]
    fn a_failing_gate_beside_a_work_limit_hole_exits_1_and_an_error_exits_2() {
        let hole = axes(0, &work_limited());
        let failed = Axes {
            judgement: Judgement::Fail,
            ..Axes::default()
        };
        let erred = Axes {
            error: true,
            ..Axes::default()
        };

        assert_eq!(hole.and(failed).exit(), 1);
        assert_eq!(hole.and(failed).and(erred).exit(), 2);
    }
}
