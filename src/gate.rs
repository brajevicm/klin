use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::base::{self, Kind, Prior, Window};
use crate::budget::{self, Budget, BuildBlock, GateBlock};
use crate::changed::Change;
use crate::check::contract::{
    self, Activation, Caller, Context, DELETED, DERIVATION, Hole, Incomplete, NO_SOURCE_ROOT,
    Plain, Reason, Records, Sink, Told, UNBUILT, UNRESOLVED,
};
use crate::check::contract::{Cause, Class};
use crate::check::holes::{self, Seen, Unmeasured};
use crate::check::{catalogue, render};
use crate::config;
use crate::config::MEASUREMENT_LOST;
use crate::error::Error;
use crate::host;
use crate::host::adapter::{Event, Stop};
use crate::plan::{Gate, Plan, State, every_check};
use crate::project::Project;
use crate::scope::{Moved, Moves};
use crate::stamp::Verdict;
use crate::syntax::{LanguageId, structural};
use crate::{build, handoff, journal, stamp, state, stats, survey, turn};

/// The `ERR` row of 11.1 as `--json` names it, which a run that could not measure prints
/// whatever exit code it ends with.
const ERROR: &str = "ERROR";
const HOOK_REPORT: &str = "KLIN_HOOK_REPORT";
/// A stop's run decided to block. The host's own exit code for a block is `block_exit`, which
/// the stop ends with. Spec 9.1.
const BLOCKED: u8 = 2;

/// What one run of the runner is asked: by `check` or by the Stop hook.
#[derive(Default)]
struct Args {
    config: Option<PathBuf>,
    gates: Vec<String>,
    changed: bool,
    hook: bool,
    json: bool,
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

pub fn check(check: &Check, start: &Path, out: &mut String) -> Result<u8, Error> {
    let args = Args {
        config: check.config.clone(),
        gates: check.checks.clone(),
        changed: check.changed,
        json: check.json,
        ..Args::default()
    };
    run(&args, start, out)
}

/// One Stop of the agent ingress, over the tree the event names, after the opt-in walk found
/// the worktree root's `klin.json`. Spec 10.2, 10.3.
pub fn stop(event: Event, start: &Path, out: &mut String) -> u8 {
    let args = Args {
        changed: true,
        hook: true,
        ..Args::default()
    };
    let Some(_claim) = state::claimed(start, &event.identity) else {
        return 0;
    };
    match Project::load(None, start, &catalogue::sections()) {
        Ok(mut project) => stopped(&args, &mut project, Some(event), out),
        Err(problem) => unjudged(&event, start, &problem),
    }
}

/// A Stop under a klin.json klin cannot read: it measures nothing, blocks nothing, writes
/// `unjudged`, and tells the person once per stamp. Spec 6.6, 10.7, 15.
fn unjudged(event: &Event, root: &Path, problem: &Error) -> u8 {
    let said = format!(
        "klin: nothing judged — {problem}. Only a person edits that file, so this stop blocks \
         nothing."
    );
    let mut log = journal::Stop::begun(Some(event), "invalid".to_string());
    let lock = state::ready(root)
        .ok()
        .and_then(|at| state::lock(&at, BUDGET));
    let Some(_lock) = lock else {
        eprintln!("{said}");
        return 0;
    };
    let told = [key(&said)];
    let fresh = !turn::told(root).contains(&told[0]);
    let left = turn::Left {
        prior: turn::aborting(root).unwrap_or_default(),
        verdict: Some(Verdict::unjudged(problem.to_string())),
        asked: None,
    };
    written(root, false, left, &mut log);
    let config = json!({
        "path": config::located(None, root).map(|file| file.display().to_string()),
        "present": true,
    });
    let fault = Fault {
        kind: ErrorKind::Configuration,
        error: Error(problem.to_string()),
    };
    log.result = Some(Report::stopped_by(config, None, fault));
    let said = fresh.then_some(said);
    log.notice = noticed(root, said.as_deref(), Some(event));
    journal::stop(root, &log);
    if let Some(said) = said {
        turn::heard(root, &told);
        host::answering(Some(event)).stop(&Stop::Tell(said));
    }
    0
}

/// The notice a Stop that blocks nothing leaves for the person, and whether the host has a
/// channel that delivers it. Spec 10.7, 13.1.
fn noticed(root: &Path, said: Option<&str>, event: Option<&Event>) -> Option<journal::Notice> {
    said.map(|message| journal::Notice {
        message: message.to_string(),
        delivered: host::answering(event).delivers_notices(),
        stamp: turn::stamp_commit(root),
    })
}

fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if !args.json {
        for note in config::notes(args.config.as_deref(), start) {
            let _ = writeln!(out, "{note}");
        }
    }
    let loaded = Project::load(args.config.as_deref(), start, &catalogue::sections());
    Ok(checked(args, start, loaded, out))
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
    let (window, fresh) = windowed(project, lost, &mut log, out);
    let project = &*project;
    budgeted(root, lost, &mut log, Budget::open);
    let prior = (!lost).then(|| turn::aborting(root)).flatten();
    let (code, leaves, asked, note) = ran(
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
    log.blocked = code == BLOCKED;
    let advised = matches!(leaves, Leaves::Fresh);
    let fresh = fresh.filter(|_| advised);
    log.advisory = log.advisory.filter(|_| advised);
    let left = turn::Left {
        prior: prior.unwrap_or_default(),
        verdict: leaves.verdict(),
        asked: asked.as_deref(),
    };
    let (note, told) = leave(root, (lost, fresh), note, left, &mut log);
    log.asked = asked.unwrap_or_default();
    if let Some(spent) = budgeted(root, lost, &mut log, |budget, _| budget.spent()) {
        log.gate_blocks = spent.gate_blocks;
        log.build_blocks = spent.builds;
        log.prompt = spent.prompt;
    }
    let said = tell(args, root, code, note, &mut log);
    log.notice = noticed(root, said.as_deref(), event.as_ref());
    if !lost && (log.blocked || log.told.contains(&"note")) {
        turn::heard(root, &told);
    }
    observe_hook_report(log.result.as_ref());
    log.timing.total_ms = journal::millis(begun.elapsed());
    journal::stop(root, &log);
    if let Some(said) = said {
        host::answering(event.as_ref()).stop(&Stop::Tell(said));
    }
    exit
}

/// The window this stop judges, bound to the project, and why the stop is advisory when the
/// history moved under the turn. An advisory Stop that holds the state lock also captures the
/// tree it measures before the build runs, so the fresh stamp it takes is that tree and holds
/// no build output. A repository with no remote judges the branch instead, which the journal
/// records as `branch-fallback`. Spec 6.6, 13.1.
fn windowed(
    project: &mut Project,
    lost: bool,
    log: &mut journal::Stop,
    out: &mut String,
) -> (Option<Window>, Option<stamp::Capture>) {
    let Ok((window, advisory)) = turn::window(project.root(), lost, &mut log.flags, out) else {
        return (None, None);
    };
    project.bind(&window);
    log.advisory = advisory.map(turn::Reason::name);
    if advisory.is_none() && matches!(window.kind, Kind::Branch) {
        log.flags.push("branch-fallback");
    }
    let fresh = advisory
        .filter(|_| !lost)
        .and_then(|_| turn::capture(project.root()));
    (Some(window), fresh)
}

/// What this stop leaves under the stamp, and the note it still tells with the records it
/// tells: the verdict, or after an advisory Stop the fresh stamp of the tree it captured, whose
/// empty `told` lets the whole note through. Spec 2.3, 6.6.
fn leave(
    root: &Path,
    (lost, fresh): (bool, Option<stamp::Capture>),
    note: Option<String>,
    left: turn::Left,
    log: &mut journal::Stop,
) -> (Option<String>, Vec<String>) {
    if let Some(fresh) = fresh {
        refreshed(root, fresh, log);
        return (note, Vec::new());
    }
    let kept = once(root, note, log.result.as_ref());
    written(root, lost, left, log);
    kept
}

/// The note this stop still tells, and the records it tells, which the stamp holds as told once
/// the host took them. A note whose every record a Stop already told under the stamp stays
/// quiet. Spec 2.3.
fn once(
    root: &Path,
    note: Option<String>,
    report: Option<&Value>,
) -> (Option<String>, Vec<String>) {
    let told = told_records(report);
    let heard = turn::told(root);
    let note =
        note.filter(|_| told.is_empty() || told.iter().any(|record| !heard.contains(record)));
    (note, told)
}

/// The host session an event names, and none for a stop no event placed.
fn session(event: Option<&Event>) -> &str {
    event.map_or("", |event| event.session.as_str())
}

/// Asks the block budget of the stop this journal line records, which adds its flags to the
/// line.
fn budgeted<'a, T>(
    root: &'a Path,
    lost: bool,
    log: &'a mut journal::Stop,
    ask: impl FnOnce(&Budget<'a>, &mut Vec<&'static str>) -> T,
) -> T {
    let budget = Budget {
        root,
        session: log.session.as_deref(),
        continued: log.continued,
        lost,
    };
    ask(&budget, &mut log.flags)
}

/// The exit code a stop ends with: the host's own code for a block where the run blocked, and
/// the run's code otherwise. Spec 9.1.
fn exit_code(code: u8, event: Option<&Event>) -> u8 {
    match code {
        BLOCKED => host::answering(event).block_exit(),
        code => code,
    }
}

/// The benchmark wrapper may observe the check document this stop already built. A failed write
/// leaves the harness without evidence and cannot change the hook's verdict or delivery.
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
fn written(root: &Path, lost: bool, left: turn::Left, log: &mut journal::Stop) {
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
    let wrote = turn::verdict(root, left, &mut said);
    eprint!("{said}");
    match wrote {
        Ok(verdict) => log.verdict = verdict,
        Err(why) => log.why = Some(why),
    }
}

/// What a Stop's run leaves under the stamp: the verdict it reached, nothing where it measured
/// nothing, so the `aborted` it wrote stays, or a fresh stamp where it measured in an advisory
/// window. Spec 6.6.
enum Leaves {
    Verdict(Verdict),
    Nothing,
    Fresh,
}

impl Leaves {
    /// What a run that measured leaves: a fresh stamp after an advisory Stop, and otherwise its
    /// verdict, where a block that asked about every finding leaves no unasked deleted test.
    fn measured(verdict: Option<Verdict>, asked: bool, advised: bool) -> Leaves {
        match (verdict, advised) {
            (_, true) => Leaves::Fresh,
            (Some(Verdict::Red { open, .. }), false) if asked => Leaves::Verdict(Verdict::Red {
                open,
                unasked: Vec::new(),
            }),
            (Some(verdict), false) => Leaves::Verdict(verdict),
            (None, false) => Leaves::Nothing,
        }
    }

    fn verdict(self) -> Option<Verdict> {
        match self {
            Leaves::Verdict(verdict) => Some(verdict),
            Leaves::Nothing | Leaves::Fresh => None,
        }
    }
}

/// The fresh stamp an advisory Stop takes in place of a verdict, so the next Stop is ordinary,
/// and the journal's `advisory` verdict once it is written. A stamp git could not take leaves
/// the `aborted` the Stop wrote, and the next Stop is advisory again. Spec 6.6, 13.1.
fn refreshed(root: &Path, fresh: stamp::Capture, log: &mut journal::Stop) {
    let mut said = String::new();
    match turn::refreshed(root, fresh, &mut said) {
        true => log.verdict = "advisory",
        false => {
            log.why = Some("git could not take a fresh stamp, so this advisory stop wrote none")
        }
    }
    eprint!("{said}");
}

/// What a Stop's check document tells that no later Stop under the same stamp repeats: each
/// note, error and review item, keyed by its record, and each file lost to measurement, keyed
/// by its file, reason and position, so new words for the same loss are not a new record.
/// Spec 2.3.
fn told_records(document: Option<&Value>) -> Vec<String> {
    let Some(document) = document else {
        return Vec::new();
    };
    let listed = |field: &str| document[field].as_array().into_iter().flatten();
    let lost = listed("findings")
        .filter(|finding| finding["kind"] == MEASUREMENT_LOST)
        .map(|finding| json!([MEASUREMENT_LOST, finding["file"], finding["values"]]));
    listed("notes")
        .chain(listed("errors"))
        .chain(listed("reviews"))
        .map(Value::to_string)
        .chain(lost.map(|site| site.to_string()))
        .map(|record| key(&record))
        .collect()
}

fn key(text: &str) -> String {
    format!("{:016x}", state::hash(text.as_bytes()))
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
             the host runs klin's session and prompt hooks."
                .to_string(),
        ));
    }
}

/// Whether no `prompt` line of this stop's session reached the journal. The tail the stop read
/// reaches back past the turn stamp, which the prompt event that appends that line takes
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
) -> (u8, Leaves, Option<Vec<String>>, Option<String>) {
    let handing = Handing {
        window,
        event,
        lost,
    };
    let (outcome, build_ms) = journal::timed(|| built(args, project, window));
    log.timing.build_ms = build_ms;
    let (failure, said, unbuilt) = match outcome {
        Ok(outcome) => sorted(outcome),
        Err(problem) => {
            let fault = Fault {
                kind: ErrorKind::Configuration,
                error: problem,
            };
            let (code, note, _) = handed(args, project, Err(fault), handing, log, out);
            return (code, Leaves::Nothing, None, note);
        }
    };
    match failure {
        Some(failure) => {
            let blocks = budgeted(project.root(), lost, log, Budget::build_block);
            let (code, text) = reported(args, &failure, &said, window, &blocks, out);
            log.result = Some(Report::unbuilt(
                Report::config_of(project),
                window,
                &failure,
            ));
            (
                blocked_build(project.root(), event, text, code),
                Leaves::Verdict(Verdict::red()),
                None,
                None,
            )
        }
        None => {
            let judged = judge(args, project, window, &said, unbuilt.as_deref(), out);
            if let (Err(_), Some(note)) = (&judged, &unbuilt) {
                eprintln!("klin: {note}");
            }
            let verdict = judged_verdict(&judged);
            let reported = judged
                .as_ref()
                .map(|tally| tally.reported.clone())
                .unwrap_or_default();
            let (code, note, advised) = handed(args, project, judged, handing, log, out);
            let asked = (code == BLOCKED).then_some(reported);
            let leaves = Leaves::measured(verdict, asked.is_some(), advised);
            (code, leaves, asked, note)
        }
    }
}

/// The verdict of a run that measured: red for a failing finding, which an unasked deleted test
/// is, and green otherwise, because an error, a hole or a note keeps nothing red. A run that
/// failed before it measured reaches none, so the `aborted` the Stop wrote stays and the stamp
/// never moves past work no Stop judged. Only a klin.json klin cannot read writes `unjudged`.
/// Spec 6.6, 10.4.
fn judged_verdict(judged: &Result<Tally, Fault>) -> Option<Verdict> {
    let tally = judged.as_ref().ok()?;
    Some(match tally.failed {
        0 => Verdict::Green,
        _ => Verdict::Red {
            open: tally.reported.clone(),
            unasked: tally.unasked.clone(),
        },
    })
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

/// What a Stop hands its finished run over with: the window it judged, the host event, and
/// whether it lost the state lock.
#[derive(Clone, Copy)]
struct Handing<'a> {
    window: Option<&'a Window>,
    event: Option<&'a Event>,
    lost: bool,
}

/// What the hook does with a run it finished: report it, and block the stop or let it end with
/// the note it leaves for the person, and whether it was an advisory Stop that measured, which
/// blocks nothing for a finding and tells what it found. Spec 6.6.
fn handed(
    args: &Args,
    project: &Project,
    outcome: Result<Tally, Fault>,
    Handing {
        window,
        event,
        lost,
    }: Handing,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, Option<String>, bool) {
    let (mut tally, measured) = match outcome {
        Ok(tally) => (tally, true),
        Err(fault) => {
            let problem = fault.error.to_string();
            log.result = Some(Report::stopped_by(
                Report::config_of(project),
                window,
                fault,
            ));
            let _ = writeln!(out, "ERR: {problem}");
            let tally = Tally {
                errored: 1,
                ..Tally::default()
            };
            (tally, false)
        }
    };
    if let Some(document) = tally.document.take() {
        log.result = Some(document);
    }
    if let Some(reason) = log.advisory.filter(|_| measured) {
        let said = advised(reason, &tally, &std::mem::take(out));
        return (0, Some(said), true);
    }
    let (code, note) = hook(
        args,
        tally,
        &std::mem::take(out),
        project.root(),
        event,
        lost,
        log,
    );
    (code, note, false)
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
    /// The deleted tests among those findings, as sites, which klin has not asked about yet.
    unasked: Vec<String>,
    /// The check document the run built, which the Stop tells from and the journal line holds.
    /// Spec 13.1.
    document: Option<Value>,
}

impl Tally {
    /// One gate's exit in the tally. A gate that could not run fully and still holds a failing
    /// finding counts as failed too, so its FAIL spends a block. Spec 10.4.
    fn count(&mut self, code: u8, recorded: &Recorded) {
        match code {
            0 => (),
            1 => self.failed += 1,
            _ => self.errored += 1,
        }
        if code > 1 && recorded.findings.iter().any(failing) {
            self.failed += 1;
        }
    }
}

/// Who builds a tree outside the hook: `klin gate` outside it runs no build. ADR 0012, spec 9.3.
const OWN_CI: &str = "`klin check` runs no build, so the project's own CI must run it";

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

/// A host that cannot read stderr still has to show the build failure. An adapter whose stop
/// already reads stderr ignores the text and returns 2.
fn blocked_build(root: &Path, event: Option<&Event>, text: String, code: u8) -> u8 {
    match code {
        2 => block(root, event, text),
        _ => code,
    }
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
    blocks: &BuildBlock,
    out: &mut String,
) -> (u8, String) {
    let (blocking, said, note) = blocks.outcome();
    let code = if blocking { BLOCKED } else { 0 };
    let note = note.map(|note| format!("{note} {OWN_CI}."));
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
        return (code, text);
    }
    out.clear();
    let _ = writeln!(out, "{object}");
    (code, text)
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
    let base = chosen(window, project)?;
    project.changes(&base.before).map(Some)
}

fn judge(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
    built: &[contract::Said],
    unbuilt: Option<&str>,
    out: &mut String,
) -> Result<Tally, Fault> {
    let plan = Plan::of(project).map_err(fault(ErrorKind::Configuration))?;
    let wanted: Vec<&Gate> = plan
        .select(&args.gates, project)
        .map_err(fault(ErrorKind::Invocation))?
        .into_iter()
        .filter(|gate| gate.check.placement.at_stop())
        .collect();
    let against = against_or_stop(args, &wanted, project, window, out)?;
    said(args, built, out);
    if let (Some(unbuilt), false) = (unbuilt, args.json) {
        let _ = writeln!(out, "  {unbuilt}");
    }
    let rootless =
        no_source_root(args, &plan, &wanted, project, out).map_err(fault(ErrorKind::Internal))?;
    let mut doc = Report::stopping(project, against.base.as_ref(), &wanted);
    doc.built = built
        .iter()
        .filter_map(|(_, entry)| entry.clone())
        .collect();
    let (mut tally, mut records) = each(args, &wanted, project, &against, &mut doc, out);
    tally.document = Some(doc.stopped_with(&plan, unbuilt, rootless.as_deref()));
    tally.told += usize::from(rootless.is_some());
    records
        .notes
        .extend(rootless.map(|said| record("note", &said)));
    let gone = gone_moves(args, project, &wanted, out);
    tally.told += gone.len();
    records.notes.extend(gone);
    if let Some(unbuilt) = unbuilt {
        records.notes.push(record(UNBUILT, unbuilt));
        tally.told += 1;
    }
    let derived = built.iter().filter_map(|(_, entry)| entry.clone());
    records.derived.splice(0..0, derived);
    finish(args, &plan, wanted.len(), &tally, records, &against, out);
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

/// The run as the Stop takes it, where a base tree klin could not lay out stops
/// the run.
fn against_or_stop<'a>(
    args: &Args,
    wanted: &[&Gate],
    project: &'a Project,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Against<'a>, Fault> {
    let against = against(args, wanted, project, window, out)?;
    match against.unlaid {
        Some(why) => Err(Fault {
            kind: ErrorKind::Git,
            error: Error(why),
        }),
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
    let base = chosen(window, project)?;
    if !args.json {
        let _ = writeln!(out, "  {}", base.line());
    }
    Ok(Some(base))
}

/// The window the run judges: the one the hook already read, or the base a run by hand and CI
/// choose for themselves. Spec 6.1, 6.3.
fn chosen(window: Option<&Window>, project: &Project) -> Result<Window, Error> {
    match window {
        Some(window) => Ok(window.clone()),
        None => base::choose(project.root()),
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
) -> Result<Option<String>, Error> {
    if !rootless(args, plan, wanted, project) {
        return Ok(None);
    }
    let said = no_source_root_said(project);
    if !args.json {
        let _ = writeln!(out, "  NOTE: {said}");
    }
    Ok(Some(said))
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
) {
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
        return;
    }
    out.clear();
    let _ = writeln!(out, "{object}");
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

/// The records of spec 11.2 only the runner writes: the findings, notes and derived entries it
/// renders from each gate's typed result or records itself, and one row per gate.
#[derive(Default, Clone)]
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
    unwritable(root);
    if tally.failed == 0 {
        return nothing_blocks(args, tally.told + tally.errored, report, event);
    }
    let Some(event) = event else {
        eprint!("{report}");
        return (1, None);
    };
    let gate_block =
        |budget: &Budget, flags: &mut _| budget.gate_block(event.blocked_before, flags);
    let number = match budgeted(root, lost, log, gate_block) {
        GateBlock::Spent(number) => number,
        GateBlock::Passed(why) => return not_blocked(args, &tally, report, event, &why),
    };
    let numbered = budget::gate_numbered(number);
    let lead = match tally.errored {
        0 => format!(
            "klin: a quality gate failed — fix what each names, then stop again \
             ({numbered}):"
        ),
        _ => format!(
            "klin: a quality gate failed — fix what each FAIL names, then stop again \
             ({numbered}). A capability that could not run is a \
             limitation of this stop and asks for no change to the source:"
        ),
    };
    eprintln!("{lead}");
    eprint!("{report}");
    let code = block(root, Some(event), format!("{lead}\n{report}"));
    if code == BLOCKED {
        log.gate_block = Some(number);
    }
    (code, None)
}

/// What an advisory Stop tells in place of a block: that the history moved, that klin blocks
/// nothing until it settles, and what the run found, once, because the fresh stamp it takes
/// makes the next Stop ordinary. Spec 6.6.
fn advised(reason: &str, tally: &Tally, report: &str) -> String {
    let line = format!(
        "klin: the history moved under this turn ({reason}), so klin does not block until the \
         history settles, and `klin check` judges the branch."
    );
    match tally.failed + tally.told + tally.errored {
        0 => line,
        _ => format!("{line}\n{report}"),
    }
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
    eprintln!("klin: {} — this stop is not blocked:", lead(tally.errored));
    eprint!("{report}");
    eprintln!(
        "klin: not blocking again; {why}, and the window stays open until a person \
         fixes, accepts or resets it."
    );
    let person = (!args.json).then(|| person_note(tally, report)).flatten();
    (event.host.stop(&Stop::Pass), person)
}

/// What a failing Stop that spends no gate block still tells the person: the run's notes and
/// errors, which the told-once record filters, and how to hold a file lost to a parse. A spent
/// budget never keeps a new limitation from the person. Spec 2.3, 7.2.
fn person_note(tally: &Tally, report: &str) -> Option<String> {
    let mut parts = Vec::new();
    if tally.told + tally.errored > 0 {
        parts.push(format!(
            "klin: this stop is not blocked, and the run left a note:\n{report}"
        ));
    }
    if tally.grammar_lag {
        parts.push(render::LOST_TO_A_PERSON.to_string());
    }
    (!parts.is_empty()).then(|| parts.join("\n"))
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

/// Record the exact report a follow-up host will echo under the event's session, then deliver
/// the block. The stop's run reads `BLOCKED` as its decision whatever exit code the host takes
/// for a block, and the stop ends with that code. The echo opens no turn and no fresh gate
/// budget. Spec 9.1, 9.3, 10.4.
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

fn lead(errored: usize) -> &'static str {
    match errored {
        0 => "a quality gate failed",
        _ => "a quality gate failed, and another could not run",
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

fn each(
    args: &Args,
    wanted: &[&Gate],
    project: &Project,
    against: &Against,
    doc: &mut Report,
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
        tally.count(code, &recorded);
        if rendered(args, code, &recorded) {
            printed(args, (&gate.name, status(code)), (&told, &text), out);
        }
        totals
            .gates
            .push(row(gate, code, (&records, &recorded), ms));
        reported.extend(unmeasured_by(&gate.name, &told));
        gather(&mut totals, recorded.clone(), &gate.name);
        let ran = (code, told, records, recorded);
        doc.took(
            &QUIET,
            (gate, project, against),
            ran,
            ms,
            &mut String::new(),
        );
    }
    let base = against.base.as_ref().map(|base| base.before.as_str());
    let sorted = holes::sorted(project, base, reported);
    doc.settled(
        &QUIET,
        (project, wanted, against),
        &sorted,
        &mut String::new(),
    );
    stop_unmeasured(
        args,
        (project, wanted),
        sorted,
        (&mut tally, &mut totals),
        out,
    );
    tally.told += totals.notes.iter().filter(|note| told(note)).count();
    (tally.reported, tally.unasked) = asked_sites(&totals.findings);
    (tally, totals)
}

/// The site id of every finding a Stop reports, which a block records as asked, and the deleted
/// tests among them as sites, which klin has not asked about yet. Spec 8.2, 9.2.
fn asked_sites(findings: &[Value]) -> (Vec<String>, Vec<String>) {
    let reported = findings
        .iter()
        .filter_map(|finding| finding.get("id")?.as_str().map(str::to_string))
        .collect();
    let unasked = findings
        .iter()
        .filter(|finding| finding["gate"] == INVENTORY)
        .map(|finding| {
            format!(
                "{}:{}  {}",
                word(finding, "file"),
                finding["line"],
                word(finding, "text")
            )
        })
        .collect();
    (reported, unasked)
}

const INVENTORY: &str = "inventory";

fn word<'a>(record: &'a Value, key: &str) -> &'a str {
    record[key].as_str().unwrap_or_default()
}

/// What the Stop says of the files the run could not measure: a lost file the accepted list does
/// not hold fails like any gate, and an opened gap and a coverage limit the change did not open
/// are notes the Stop tells. Spec 2.3, 7.2.
fn stop_unmeasured(
    args: &Args,
    (project, wanted): (&Project, &[&Gate]),
    (sorted, failed): (Vec<Unmeasured>, Option<String>),
    (tally, totals): (&mut Tally, &mut Recorded),
    out: &mut String,
) {
    if let Some(why) = failed {
        tally.errored += 1;
        if !args.json {
            let _ = writeln!(out, "  ERR: {why}");
        }
    }
    let sorted = sorted.as_slice();
    let held = holes::held_files(&project.config);
    let failing = failing_lost(sorted, &held);
    let unmatched: Vec<String> = unmatched_lost(&held, sorted, wanted).collect();
    if !failing.is_empty() {
        tally.failed += 1;
        tally.grammar_lag = failing.iter().any(|item| item.reason == Cause::Parse);
    }
    totals
        .findings
        .extend(failing.iter().map(|item| stop_finding(item)));
    tally.told += unmatched.len()
        + sorted
            .iter()
            .filter(|item| item.class != Class::Lost)
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
        Cause::LeftScope | Cause::Form | Cause::Filtered => contract::LOST,
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
        Some(DELETED | DERIVATION | UNRESOLVED | AMBIGUOUS | UNBUILT | UNMATCHED)
    )
}

/// The reason of a form a resolver found two answers for, which the Stop tells as it tells an
/// unresolved one. Spec 7.2.
const AMBIGUOUS: &str = "ambiguous";
/// An accepted entry that matched nothing, which the Stop tells as a note. Spec 15.
const UNMATCHED: &str = "unmatched";

/// The files one gate could not measure, as it saw them, for the run to sort once. Spec 7.2.
fn unmeasured_by(gate: &str, told: &[Told]) -> Vec<(String, String, Seen)> {
    let seen = |file: &str, seen| (gate.to_string(), file.to_string(), seen);
    told.iter()
        .flat_map(|item| match item {
            Told::Hole(Hole::Unparsed(files)) => files
                .iter()
                .map(|file| seen(&file.file, Seen::Unread))
                .collect(),
            Told::Hole(Hole::Lost { file, why }) => vec![seen(file, Seen::Left(*why))],
            Told::Hole(Hole::Formed {
                file,
                form,
                measured,
            }) => vec![seen(file, Seen::Form(*form, *measured))],
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

/// One gate's run: its exit code, what it told, what it recorded and its findings as JSON.
type Ran = (u8, Vec<Told>, Records, Recorded);

fn one(args: &Args, gate: &Gate, project: &Project, against: &Against) -> Ran {
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
    moved_out(project.moves(), gate.check.section, &mut recorded.findings);
    errored(code, &mut records.errors, &mut recorded);
    (code, told, records, recorded)
}

/// The one `error` finding of a gate that is exit 2, from the reasons it gave, beside any failing
/// finding it holds, so the error stays in the record the Stop tells from. Spec 10.4, 11.2.
fn errored(code: u8, errors: &mut Vec<String>, recorded: &mut Recorded) {
    let named = recorded
        .findings
        .iter()
        .any(|finding| finding["outcome"] == "error");
    if code != 2 || named || errors.is_empty() {
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

    /// The one state that wins: an error, then a failing finding, then a hole, then a review
    /// item. Spec 7.4, 11.3.
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
            Axes {
                judgement: Judgement::Review,
                ..
            } => Worst::Review,
            _ => Worst::Ok,
        }
    }

    /// ERROR 2 > FAIL 1 > INCOMPLETE 3 > success 0. Spec 7.4.
    fn exit(self) -> u8 {
        match self.worst() {
            Worst::Error => 2,
            Worst::Fail => 1,
            Worst::Incomplete => 3,
            Worst::Review | Worst::Ok => 0,
        }
    }

    /// The row word: the first of `ERR`, `FAIL`, `INCOMPLETE`, `REVIEW`, `ok`. Spec 11.3.
    fn state(self) -> &'static str {
        match self.worst() {
            Worst::Error => "ERR ",
            Worst::Fail => "FAIL",
            Worst::Incomplete => "INCOMPLETE",
            Worst::Review => "REVIEW",
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
    Review,
    Ok,
}

/// One gate's axes from its exit code and its result. A gate that erred still carries the
/// findings it validated, so they still fail it. Spec 7.5.
fn axes(code: u8, recorded: &Recorded) -> Axes {
    let fails = code == 1 || recorded.findings.iter().any(failing);
    Axes {
        judgement: match (fails, recorded.reviews.is_empty()) {
            (true, _) => Judgement::Fail,
            (false, false) => Judgement::Review,
            (false, true) => Judgement::Pass,
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
    tree: Option<Value>,
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
    /// The derived build commands the Stop ran before it measured, in the run's basis. Spec 8.1.
    built: Vec<Value>,
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
                "ignored": config::ignored(start),
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
    let window = base::choose(project.root()).ok();
    if let Some(window) = &window {
        project.bind(window);
    }
    let project = &*project;
    if let Some(note) = deleted_config(args, project, window.as_ref()) {
        if !args.json {
            let _ = writeln!(out, "  NOTE: {}", note["message"].as_str().unwrap_or(""));
        }
        report.notes.push(note);
    }
    let plan = Plan::of(project).map_err(fault(ErrorKind::Configuration))?;
    let (wanted, unsupported) = chosen_gates(&args.gates, &plan, project)?;
    let against = against(args, &wanted, project, window.as_ref(), out)?;
    report.ran(args, project, (&plan, &wanted, unsupported), &against, out);
    Ok(())
}

/// What the Stop notes of a moved pinned path, one whose files went with no rename or that
/// selects nothing in either tree, and of a file moved under a skipped directory. A pin whose
/// files were all renamed is followed in silence, and `klin check` names it. Spec 7.3.
fn gone_moves(args: &Args, project: &Project, wanted: &[&Gate], out: &mut String) -> Vec<Value> {
    let gone = selected(project.moves(), wanted).filter(|moved| moved.gone());
    gone.filter_map(Moved::said)
        .map(|said| {
            if !args.json {
                let _ = writeln!(out, "  NOTE: {said}");
            }
            record("note", &said)
        })
        .collect()
}

/// The moves of the sections whose gates this run selected, and every move no section decides,
/// such as a file moved under a skipped directory, whichever gates run.
fn selected<'a>(moves: &'a Moves, wanted: &'a [&Gate]) -> impl Iterator<Item = &'a Moved> {
    moves.iter().filter(|moved| {
        moved
            .section()
            .is_none_or(|section| wanted.iter().any(|gate| gate.check.section == section))
    })
}

/// The review item of a moved pinned path or of a file moved under a skipped directory, and
/// nothing for a file moved out of a scope, whose findings carry `moved_out_of_scope`.
/// Spec 7.3, 11.7.
fn moved_review(moved: &Moved) -> Option<Value> {
    let (kind, path) = match moved {
        Moved::Pin { path, .. } => (contract::MOVED_PIN, path),
        Moved::Skipped { path, .. } => (contract::MOVED_SKIPPED, path),
        Moved::Out { .. } => return None,
    };
    let check = moved
        .section()
        .and_then(|section| {
            catalogue::CATALOGUE
                .iter()
                .find(|row| row.section == section)
        })
        .map(|row| row.name);
    Some(json!({
        "check": check,
        "kind": kind,
        "file": path,
        "line": null,
        "text": moved.said(),
        "reason": moved.reason(),
    }))
}

/// A finding at a file the change moved out of its gate's scope, which keeps the scope of its
/// base path, carries `moved_out_of_scope`, which no ratchet compares. Spec 7.3.
fn moved_out(moves: &Moves, section: &str, findings: &mut [Value]) {
    let out = moves.out_of(section);
    for finding in findings
        .iter_mut()
        .filter(|finding| out.contains(&finding["file"].as_str().unwrap_or_default()))
    {
        if let Some(values) = finding.get_mut("values").and_then(Value::as_object_mut) {
            values.insert("moved_out_of_scope".into(), true.into());
        }
    }
}

/// The note of a run under `{}` whose base still holds the worktree root's `klin.json`, so the
/// change deleted the policy the base was judged under. Spec 5.1.
fn deleted_config(args: &Args, project: &Project, window: Option<&Window>) -> Option<Value> {
    if args.config.is_some() || project.config.written() {
        return None;
    }
    let window = window?;
    crate::git::Repo::at(project.root()).blob(&window.before, config::FILENAME)?;
    Some(json!({
        "check": null,
        "kind": "config-deleted",
        "file": config::FILENAME,
        "message": format!(
            "{} is deleted: the base holds it and the working tree does not, so this run is \
             under {{}}",
            config::FILENAME
        ),
    }))
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
            error: plan.no_gate(project),
        });
    }
    let mut unsupported = Vec::new();
    for name in named {
        let missing = plan.needs_a_section.iter().find(|check| check.name == name);
        match missing {
            Some(check) if !plan.gates.iter().any(|gate| &gate.name == name) => {
                unsupported.push(*check)
            }
            _ => plan
                .known(name, project)
                .map_err(fault(ErrorKind::Invocation))?,
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

/// The arguments the Stop's document is gathered under: no text, because the Stop prints its own,
/// and the Stop's own window and scope.
const QUIET: Args = Args {
    config: None,
    gates: Vec::new(),
    changed: true,
    hook: true,
    json: true,
};

impl Report {
    /// The document a Stop gathers beside its own records: the configuration it ran under and
    /// what binding its window found. Spec 13.1.
    fn stopping(project: &Project, base: Option<&Window>, wanted: &[&Gate]) -> Report {
        let mut doc = Report {
            config: Some(Report::config_of(project)),
            ..Report::default()
        };
        doc.windowed(&QUIET, (project, wanted), base, &mut String::new());
        doc
    }

    /// The configuration a run judged under, as the check document names it. Spec 11.7.
    fn config_of(project: &Project) -> Value {
        json!({
            "path": project.config.file.display().to_string(),
            "present": project.config.written(),
        })
    }

    /// The Stop's document once every gate ran: the note of a build that could not run, the note
    /// of a tree with no source root, and the rows of the capabilities that do not apply.
    /// Spec 11.7, 13.1.
    fn stopped_with(mut self, plan: &Plan, unbuilt: Option<&str>, rootless: Option<&str>) -> Value {
        self.notes
            .extend(unbuilt.map(|unbuilt| note(UNBUILT, unbuilt)));
        self.notes
            .extend(rootless.map(|said| note(NO_SOURCE_ROOT, said)));
        self.not_applicable(plan);
        self.closed()
    }

    /// The document of a Stop whose build failed, so no capability measured. Spec 6.4, 13.1.
    fn unbuilt(config: Value, window: Option<&Window>, failure: &str) -> Value {
        Report {
            config: Some(config),
            window: window.map(Window::record),
            stopped: true,
            notes: vec![note("build", failure)],
            ..Report::default()
        }
        .closed()
    }

    /// The document of a Stop that stopped before any capability measured, with the error that
    /// stopped it. Spec 7.3, 13.1.
    fn stopped_by(config: Value, window: Option<&Window>, fault: Fault) -> Value {
        let mut doc = Report {
            config: Some(config),
            window: window.map(Window::record),
            ..Report::default()
        };
        doc.stop(&QUIET, fault, &mut String::new());
        doc.closed()
    }

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
        self.windowed(args, (project, wanted), against.base.as_ref(), out);
        for gate in wanted {
            self.gate(args, gate, project, against, out);
        }
        let base = against.base.as_ref().map(|base| base.before.as_str());
        let reported = std::mem::take(&mut self.reported);
        let sorted = holes::sorted(project, base, reported);
        self.settled(args, (project, wanted, against), &sorted, out);
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

    /// What the run could not measure, sorted once over every gate: the error of a sort git
    /// could not finish, the files the gates could not measure, and the files of a language each
    /// gate does not read. Spec 7.2, 11.7.
    fn settled(
        &mut self,
        args: &Args,
        (project, wanted, against): (&Project, &[&Gate], &Against),
        (sorted, failed): &(Vec<Unmeasured>, Option<String>),
        out: &mut String,
    ) {
        if let Some(why) = failed {
            self.axes.error = true;
            self.errors.push(error_record(ErrorKind::Git, None, why));
            if !args.json {
                let _ = writeln!(out, "  ERR: {why}");
            }
        }
        self.unmeasured(args, (project, wanted), sorted, out);
        self.not_read(wanted, &by_extension(project, against));
    }

    /// What binding the window found: the window and the tree the run judges, a `moved-pin`
    /// review item per moved pinned path, a `moved-skipped` one per file moved under a skipped
    /// directory, a note each for a rewritten push base or a base equal to HEAD, and the hole
    /// of a local base equal to HEAD that may hide unpushed commits.
    /// Spec 6.5, 7.3.
    fn windowed(
        &mut self,
        args: &Args,
        (project, wanted): (&Project, &[&Gate]),
        base: Option<&Window>,
        out: &mut String,
    ) {
        self.window = base.map(Window::record);
        self.tree = (!args.hook).then(|| base::tree_record(project.root()));
        for review in selected(project.moves(), wanted).filter_map(moved_review) {
            if !args.json {
                let _ = writeln!(
                    out,
                    "  REVIEW: {}",
                    review["text"].as_str().unwrap_or_default()
                );
            }
            self.reviews.push(review);
        }
        let Some(base) = base else {
            return;
        };
        for note in &base.notes {
            if !args.json {
                let _ = writeln!(out, "  NOTE: {note}");
            }
            self.notes.push(json!({
                "check": null,
                "kind": contract::WINDOW,
                "file": null,
                "message": note,
            }));
        }
        if let Some(text) = &base.unproven {
            self.holes.push(Incomplete {
                reason: Reason::ComparisonUnproven,
                detail: None,
                text: text.clone(),
            });
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
        let (ran, ms) = journal::timed(|| one(args, gate, project, against));
        self.took(args, (gate, project, against), ran, ms, out);
    }

    /// One gate's result in the document: its row, its measurement record, its errors and what
    /// it found. The Stop hands it the result its own run already took. Spec 11.7.
    fn took(
        &mut self,
        args: &Args,
        (gate, project, against): (&Gate, &Project, &Against),
        (code, told, records, recorded): Ran,
        ms: u64,
        out: &mut String,
    ) {
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
        let basis = basis(gate, (project, against), &records, &recorded);
        self.measurements
            .push(measurement(&gate.name, axes, &recorded.holes, basis));
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
        (project, wanted): (&Project, &[&Gate]),
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
        self.reviews.extend(
            unmatched_lost(&held, sorted, wanted).map(|file| render::unmatched_lost_json(&file)),
        );
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
                Class::Opened if item.reason == Cause::LeftScope && self.fails_at(&item.file) => (),
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
            "state": State::Active.name(),
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
            .push(measurement(&gate.name, axes, &[], produced(gate.check)));
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
            .push(measurement(check.name, axes, &holes, produced(check)));
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

    /// The run's axes once every record is in: a hole makes it incomplete, and a review item
    /// makes a pass a review. Spec 7.5.
    fn aggregate(&mut self) {
        self.axes.incomplete |= !self.holes.is_empty();
        if !self.reviews.is_empty() {
            self.axes.judgement = self.axes.judgement.max(Judgement::Review);
        }
    }

    /// The document of a Stop's run, which the journal line holds under `result`. Spec 13.1.
    fn closed(mut self) -> Value {
        self.aggregate();
        let exit = self.axes.exit();
        self.document(exit)
    }

    fn finish(mut self, args: &Args, out: &mut String) -> u8 {
        self.aggregate();
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
                    "basis": {
                        "producer": null,
                        "klin": env!("CARGO_PKG_VERSION"),
                        "policy": self.built,
                    },
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
            "tree": self.tree,
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
        "state": State::Active.name(),
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
    let files: Vec<&String> = match &against.scope {
        Some(scope) => scope.iter().collect(),
        None => project.tree().files().unwrap_or_default().iter().collect(),
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
    wanted: &'a [&Gate],
) -> impl Iterator<Item = String> + 'a {
    held.iter()
        .filter(move |file| read_by(wanted, file))
        .filter(|file| !sorted.iter().any(|item| &item.file == *file))
        .cloned()
}

/// Whether a selected gate reads this file's language, so a run that measured it can say an
/// accepted entry for it matches nothing.
fn read_by(wanted: &[&Gate], file: &str) -> bool {
    let Some((_, extension)) = file.rsplit_once('.') else {
        return false;
    };
    let extension = format!(".{extension}");
    wanted.iter().any(|gate| reads(gate.check, &extension))
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
    fields
        .entry("kind")
        .or_insert_with(|| holes::UNMEASURED.into());
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
        "state": State::NotApplicable.name(),
        "judgement": null,
        "measurement": named.map(Axes::measurement),
        "execution": "ok",
        "coverage": null,
        "coverage_claim": coverage_claim(check),
        "held": null,
        "accepted": null,
    })
}

/// One measurement record: its basis, its state and its holes. Spec 8.1, 11.7.
fn measurement(name: &str, axes: Axes, holes: &[Value], basis: Value) -> Value {
    json!({
        "check": name,
        "basis": basis,
        "state": axes.measurement(),
        "holes": holes,
    })
}

/// The basis of a capability that measured nothing: what would have produced it, and the klin
/// that ran. Spec 8.1.
fn produced(check: &catalogue::Row) -> Value {
    json!({
        "producer": { "capability": check.name, "semantics_version": check.semantics },
        "klin": env!("CARGO_PKG_VERSION"),
    })
}

/// What one gate's measurement rests on: the producer and its semantics version, the klin that
/// ran, the policy by provenance (the derived values under `policy`, what a person pinned under
/// `pinned`, built-in for the rest) with the derivation commit, the integration entry, the scope
/// with its coverage counts, the window and the reason of each hole. It holds no machine path,
/// time, process id or duration, and it reads only what the run already holds, so the Stop
/// starts no git process and reads no file for it. Spec 8.1.
fn basis(
    gate: &Gate,
    (project, against): (&Project, &Against),
    records: &Records,
    recorded: &Recorded,
) -> Value {
    let pinned = pinned_entry(gate, project);
    let integration = (gate.check.activation == Activation::Integration).then(|| {
        json!({
            "entry": gate.name,
            "run": pinned.and_then(|entry| entry.get("run")),
            "report": pinned.and_then(|entry| entry.get("report")),
        })
    });
    let mut basis = produced(gate.check);
    basis["policy"] = recorded.derived.clone().into();
    basis["pinned"] = pinned.cloned().into();
    basis["derivation"] = project.surveyed_commit().into();
    basis["integration"] = integration.into();
    basis["scope"] = json!({
        "changed": against.scope.is_some(),
        "coverage": records.coverage,
    });
    basis["window"] = against.base.as_ref().map_or(Value::Null, |base| {
        json!({ "kind": base.kind.name(), "before": base.before, "after": "the working tree" })
    });
    basis["holes"] = recorded
        .holes
        .iter()
        .map(|hole| json!({ "reason": hole["reason"] }))
        .collect();
    basis
}

/// What a person pinned for this gate: its own entry where each entry is a gate, and the whole
/// section otherwise.
fn pinned_entry<'a>(gate: &Gate, project: &'a Project) -> Option<&'a Value> {
    let section = project.config.pinned(gate.check.section)?;
    match gate.check.gate_per_entry {
        true => section.as_array()?.iter().find(|entry| {
            entry.get(contract::NAMED.name).and_then(Value::as_str) == Some(gate.name.as_str())
        }),
        false => Some(section),
    }
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

/// A note of the run itself, with no check. Spec 11.7.
fn note(kind: &str, message: &str) -> Value {
    json!({ "check": null, "kind": kind, "coverage": false, "message": message })
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
