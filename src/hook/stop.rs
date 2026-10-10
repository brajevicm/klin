use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::config::file::{MEASUREMENT_LOST, located};
use crate::contract::check::Said;
use crate::contract::project::Project;
use crate::engine::against::{against, chosen};
use crate::engine::document::{
    Against, Args, CheckDocument, View, leaves_code, no_source_root_said,
};
use crate::engine::plan::{Gate, Plan};
use crate::engine::{catalogue, render};
use crate::hook::budget::{self, Budget, BuildBlock, GateBlock};
use crate::hook::host;
use crate::hook::host::adapter::{Event, Stop};
use crate::sys::changed::Change;
use crate::sys::error::{Error, ErrorKind, Fault, fault};
use crate::window::base::{Kind, Window};
use crate::window::stamp::Verdict;
use crate::{
    hook::{build, handoff, journal, stats, turn},
    sys::{clock, state},
    window::stamp,
};

const HOOK_REPORT: &str = "KLIN_HOOK_REPORT";

/// A stop's run decided to block. The host's own exit code for a block is `block_exit`, which
/// the stop ends with. Spec 9.1.
const BLOCKED: u8 = 2;

/// One Stop of the agent ingress, over the tree the event names, after the opt-in walk found
/// the worktree root's `klin.json`. Spec 10.2, 10.3.
pub fn stop(event: Event, start: &Path, out: &mut String) -> u8 {
    let args = Args {
        changed: true,
        view: View::Stop,
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
        "path": located(None, root).map(|file| file.display().to_string()),
        "present": true,
    });
    let fault = Fault {
        kind: ErrorKind::Configuration,
        error: Error(problem.to_string()),
    };
    log.result = Some(CheckDocument::stopped_by(config, None, fault).into_json());
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
        clock::timed(|| state::ready(root).ok().map(|at| state::lock(&at, BUDGET)));
    log.timing.lock_ms = lock_ms;
    let lost = matches!(&lock, Some(None));
    let mark = turn::prompt_mark(root);
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
    log.timing.base_remove_ms = clock::millis(teardown.remove);
    log.timing.base_prune_ms = clock::millis(teardown.prune);
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
    let mut seen = BTreeSet::new();
    if let Some(spent) = budgeted(root, lost, &mut log, |budget, _| budget.spent()) {
        log.gate_blocks = spent.gate_blocks;
        log.build_blocks = spent.builds;
        log.prompt = spent.prompt;
        seen = spent.trees;
    }
    let quiet = || !advised && unchanged(root, mark.as_deref(), &seen);
    let said = tell(args, root, code, note, &mut log, quiet);
    log.notice = noticed(root, said.as_deref(), event.as_ref());
    if !lost && (log.blocked || log.told.contains(&"note")) {
        turn::heard(root, &told);
    }
    observe_hook_report(log.result.as_ref());
    log.timing.total_ms = clock::millis(begun.elapsed());
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
/// A `quiet` turn tells nothing. Spec 9.5, 10.7, 11.4.
fn tell(
    args: &Args,
    root: &Path,
    code: u8,
    note: Option<String>,
    log: &mut journal::Stop,
    quiet: impl FnOnce() -> bool,
) -> Option<String> {
    let mut parts: Vec<(&'static str, String)> =
        note.into_iter().map(|note| ("note", note)).collect();
    let intervened = log.gate_blocks > 0 || turn::intervened(root);
    if code == 0 && !args.json() && log.host.is_some() && intervened {
        let tail = stats::stop_tail(root);
        add_prompt_note(&tail, log, &mut parts);
        parts.extend(stats::turn_end(root, tail, journal::line(log)));
    }
    if parts.is_empty() || quiet() {
        return None;
    }
    log.told = parts.iter().map(|(part, _)| *part).collect();
    let said: Vec<String> = parts.into_iter().map(|(_, text)| text).collect();
    Some(said.join("\n"))
}

/// Whether the turn changed nothing: the working tree, and every tree a block of this prompt
/// `seen`, is the tree the prompt `mark` was taken over. False when klin cannot read the mark's
/// tree or the working tree. Spec 10.7.
fn unchanged(root: &Path, mark: Option<&str>, seen: &BTreeSet<String>) -> bool {
    let (Some(mark), Ok(at)) = (mark, state::ready(root)) else {
        return false;
    };
    let Some(prompted) = stamp::tree_of(root, mark) else {
        return false;
    };
    seen.iter().all(|tree| *tree == prompted)
        && budget::working_tree(root, &at).as_deref() == Some(prompted.as_str())
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
    let (outcome, build_ms) = clock::timed(|| built(args, project, window));
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
            let (code, text) = reported(&failure, &said, &blocks);
            log.result = Some(
                CheckDocument::unbuilt(CheckDocument::config_of(project), window, &failure)
                    .into_json(),
            );
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
                .map(CheckDocument::reported)
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
fn judged_verdict(judged: &Result<CheckDocument, Fault>) -> Option<Verdict> {
    let doc = judged.as_ref().ok()?;
    Some(match doc.failed() {
        0 => Verdict::Green,
        _ => Verdict::Red {
            open: doc.reported(),
            unasked: doc.unasked(),
        },
    })
}

/// A finished build sorted into what blocks and what is told: the failure text of a build that
/// ran and failed, the provenance lines, and the note for a command the shell could not find.
fn sorted(
    (failure, said): (Option<build::Failure>, Vec<Said>),
) -> (Option<String>, Vec<Said>, Option<String>) {
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
    outcome: Result<CheckDocument, Fault>,
    Handing {
        window,
        event,
        lost,
    }: Handing,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, Option<String>, bool) {
    let (doc, measured) = match outcome {
        Ok(doc) => (doc, true),
        Err(fault) => {
            let _ = writeln!(out, "ERR: {}", fault.error);
            let config = CheckDocument::config_of(project);
            (CheckDocument::stopped_by(config, window, fault), false)
        }
    };
    if let Some(reason) = log.advisory.filter(|_| measured) {
        let said = advised(reason, &doc, &std::mem::take(out));
        log.result = Some(doc.into_json());
        return (0, Some(said), true);
    }
    let (code, note) = hook(
        args,
        &doc,
        &std::mem::take(out),
        project.root(),
        event,
        lost,
        log,
    );
    log.result = Some(doc.into_json());
    (code, note, false)
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

/// The build failure as a person and an agent read it. The text opens with where each command
/// came from, so a derived build is never a command with no origin, and the note says why klin
/// did not block, because the exit code alone no longer says it. Spec 11, ADR 0040.
fn reported(failure: &str, built: &[Said], blocks: &BuildBlock) -> (u8, String) {
    let (blocking, said, note) = blocks.outcome();
    let code = if blocking { BLOCKED } else { 0 };
    let note = note.map(|note| format!("{note} {OWN_CI}."));
    let mut text = String::new();
    for (line, _) in built {
        let _ = writeln!(text, "klin: {line}");
    }
    let _ = writeln!(text, "klin: {said}:");
    text.push_str(failure);
    if let Some(note) = &note {
        let _ = writeln!(text, "klin: {note}");
    }
    eprint!("{text}");
    (code, text)
}

/// The build a person chose or the one the manifests derive, run before any gate judges the
/// tree it produces, with the provenance of each command the gates' report carries. Only the
/// hook builds, so no other run derives a build. Spec 5.4, ADR 0012.
fn built(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
) -> Result<(Option<build::Failure>, Vec<Said>), Error> {
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
    built: &[Said],
    unbuilt: Option<&str>,
    out: &mut String,
) -> Result<CheckDocument, Fault> {
    let plan = Plan::of(project).map_err(fault(ErrorKind::Configuration))?;
    let wanted: Vec<&Gate> = plan
        .select(&args.gates, project)
        .map_err(fault(ErrorKind::Invocation))?
        .into_iter()
        .filter(|gate| gate.check.placement.at_stop())
        .collect();
    let against = against_or_stop(args, &wanted, project, window, out)?;
    said(built, out);
    if let Some(unbuilt) = unbuilt {
        let _ = writeln!(out, "  {unbuilt}");
    }
    let rootless =
        no_source_root(args, &plan, &wanted, project, out).map_err(fault(ErrorKind::Internal))?;
    let commands = built
        .iter()
        .filter_map(|(_, entry)| entry.clone())
        .collect();
    let mut doc = CheckDocument::stopping(project, commands);
    doc.ran(args, project, (&plan, &wanted, Vec::new()), &against, out);
    doc.stopped_with(unbuilt, rootless.as_deref());
    doc.gone_moves(project, &wanted, out);
    summary_line(&plan, wanted.len(), &doc, out);
    Ok(doc)
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
    if !args.json() {
        let _ = writeln!(out, "  NOTE: {said}");
    }
    Ok(Some(said))
}

/// Whether the run selected a check that reads code in a tree with no source root, where the
/// check would measure nothing. Only the checks a run selects count, so `klin check lockfile`
/// is not this hole. A gate whose section pins `in` measures what the pin names, and one a
/// person excluded measures nothing on purpose, so neither is this hole. ADR 0016, spec 7.2,
/// 11.3.
fn rootless(args: &Args, plan: &Plan, wanted: &[&Gate], project: &Project) -> bool {
    project.found_no_source_root() && leaves_code(args, plan, wanted, project)
}

fn summary_line(plan: &Plan, gates: usize, doc: &CheckDocument, out: &mut String) {
    let excluded = match plan.excluded.len() {
        0 => String::new(),
        count => format!("{count} excluded, "),
    };
    let line = format!(
        "klin: {gates} gate(s), {excluded}{}",
        summary(doc.failed(), doc.errored())
    );
    let _ = writeln!(out, "{line}");
}

/// Where the build the hook ran came from, printed once above the gates, each of which says
/// its own values beside its row. Spec 4.3.
fn said(built: &[Said], out: &mut String) {
    for (line, _) in built {
        let _ = writeln!(out, "  {line}");
    }
}

fn hook(
    args: &Args,
    doc: &CheckDocument,
    report: &str,
    root: &Path,
    event: Option<&Event>,
    lost: bool,
    log: &mut journal::Stop,
) -> (u8, Option<String>) {
    unwritable(root);
    if doc.failed() == 0 {
        return nothing_blocks(args, doc.told() + doc.errored(), report, event);
    }
    let Some(event) = event else {
        eprint!("{report}");
        return (1, None);
    };
    let gate_block =
        |budget: &Budget, flags: &mut _| budget.gate_block(event.blocked_before, flags);
    let number = match budgeted(root, lost, log, gate_block) {
        GateBlock::Spent(number) => number,
        GateBlock::Passed(why) => return not_blocked(args, doc, report, event, &why),
    };
    let numbered = budget::gate_numbered(number);
    let lead = match doc.errored() {
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
fn advised(reason: &str, doc: &CheckDocument, report: &str) -> String {
    let line = format!(
        "klin: the history moved under this turn ({reason}), so klin does not block until the \
         history settles, and `klin check` judges the branch."
    );
    match doc.failed() + doc.told() + doc.errored() {
        0 => line,
        _ => format!("{line}\n{report}"),
    }
}

/// A stop that failed and spends no gate block: the report, why klin does not block again, and
/// where a file is lost to a parse, the words that tell the person how to hold it. The person
/// hears them only through a channel the agent does not read. Spec 7.2, ADR 0052.
fn not_blocked(
    args: &Args,
    doc: &CheckDocument,
    report: &str,
    event: &Event,
    why: &str,
) -> (u8, Option<String>) {
    eprintln!("klin: {} — this stop is not blocked:", lead(doc.errored()));
    eprint!("{report}");
    eprintln!(
        "klin: not blocking again; {why}, and the window stays open until a person \
         fixes, accepts or resets it."
    );
    let person = (!args.json()).then(|| person_note(doc, report)).flatten();
    (event.host.stop(&Stop::Pass), person)
}

/// What a failing Stop that spends no gate block still tells the person: the run's notes and
/// errors, which the told-once record filters, and how to hold a file lost to a parse. A spent
/// budget never keeps a new limitation from the person. Spec 2.3, 7.2.
fn person_note(doc: &CheckDocument, report: &str) -> Option<String> {
    let mut parts = Vec::new();
    if doc.told() + doc.errored() > 0 {
        parts.push(format!(
            "klin: this stop is not blocked, and the run left a note:\n{report}"
        ));
    }
    if doc.grammar_lag() {
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
    if args.json() || event.is_none() {
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
