use std::borrow::Cow;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::base::{self, Kind, Prior, Window};
use crate::changed::Change;
use crate::check::{
    self, Activation, Caller, Context, DELETED, DERIVATION, NOT_MEASURED, Records, Sink, UNPARSED,
    UNRESOLVED,
};
use crate::config::{self, Error};
use crate::host::{self, Stop};
use crate::project::Project;
use crate::{build, coverage, journal, state, stats, turn, write};

/// Where klin records what one prompt already spent, so the stop that follows knows how many
/// build blocks are left and whether the turn's gate block is still unspent. In the state
/// directory, which an agent does not empty. ADR 0019, ADR 0022.
const BUILD_BLOCKED: &str = "build-blocked";
/// How many stops one prompt's build failures may block. klin bounds this itself, because the
/// host documents no cap of its own. ADR 0022, spec 9.3.
const BLOCKS: u64 = 8;
/// What `--list` indents a gate's own lines by, under the row that names it.
const UNDER: &str = "      ";
/// The `ERR` row of 11.1 as `--json` names it, which a run that could not measure prints
/// whatever exit code it ends with.
const ERROR: &str = "ERROR";

struct Gate {
    name: String,
    check: &'static check::Row,
}

#[derive(Default)]
struct Plan {
    gates: Vec<Gate>,
    excluded: Vec<String>,
    /// The checks klin offers that neither the config nor the survey supplies a section for.
    /// Each needs a section a person writes, and none of them runs.
    needs_a_section: Vec<&'static check::Row>,
}

impl Plan {
    /// The one gate a check runs as when its section is not a list of named entries.
    fn one(&mut self, check: &'static check::Row) {
        self.gates.push(Gate {
            name: check.name.to_string(),
            check,
        });
    }
}

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Fail on the holes of spec 10: an accepted entry matching nothing, a comparison klin
    /// cannot explain, and a survey with no source root — what CI runs
    #[arg(long)]
    strict: bool,
    /// Run only this gate (repeatable)
    #[arg(long = "gate", value_name = "NAME")]
    gates: Vec<String>,
    /// Print the gates that run with derived or pinned per key, the excluded ones, the ones
    /// that need a section a person writes, and the state directory, then exit
    #[arg(long)]
    list: bool,
    /// Judge only the files changed against the base — the fast loop; CI runs the full pass
    #[arg(long)]
    changed: bool,
    /// Agent Stop hook mode: the failures to stderr, exit 2 to block the first stop, and
    /// exit 1 for what only a person can fix
    #[arg(long)]
    hook: bool,
    /// Print one JSON object for the run instead of the human report
    #[arg(long)]
    json: bool,
    /// Read the hook event as this host's shape instead of the one its fields name
    #[arg(long)]
    host: Option<String>,
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    if args.hook && args.strict {
        eprintln!(
            "klin: --hook and --strict name two callers — the hook runs a turn and never \
             blocks a person's CI failure, so pass one or the other."
        );
        return Ok(1);
    }
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
    let loaded = Project::load(args.config.as_deref(), start);
    if !args.hook {
        let judged = loaded.and_then(|project| judge(args, &project, None, &[], out));
        return refused(args, judged, out).map(|tally| code(&tally));
    }
    match loaded {
        Ok(project) => Ok(stopped(args, &project, event, out)),
        Err(problem) => {
            eprintln!(
                "klin: FAIL: {problem} — only a person edits that file, so this stop is not \
                 blocked."
            );
            Ok(1)
        }
    }
}

/// How long a stop waits for the stop before it to finish. A fraction of the hook's five
/// seconds, because the stop still has a build and every gate to run inside them. Spec 13.
const BUDGET: Duration = Duration::from_secs(1);

/// One stop in the hook: the lock, the turn window, the build, the gates, and the verdict the
/// next prompt reads. The lock is held from before the run measures until after the verdict is
/// written, so an older stop cannot leave green over a newer red. Spec 6.5, 16.3.
fn stopped(args: &Args, project: &Project, event: Option<host::Event>, out: &mut String) -> u8 {
    let begun = std::time::Instant::now();
    let root = project.root();
    let mut log = journal::Stop::begun(event.as_ref(), config_hash(project));
    let (lock, lock_ms) =
        journal::timed(|| state::ready(root).ok().map(|at| state::lock(&at, BUDGET)));
    log.timing.lock_ms = lock_ms;
    let lost = matches!(&lock, Some(None));
    let window = turn::window(root, &mut log.flags, out).ok();
    if matches!(&window, Some(window) if matches!(window.kind, Kind::Branch)) {
        log.flags.push("branch-fallback");
    }
    let (code, green, asked, note) = ran(
        args,
        project,
        window.as_ref(),
        event.as_ref(),
        &mut log,
        out,
    );
    let teardown = project.teardown_base();
    log.timing.base_remove_ms = journal::millis(teardown.remove);
    log.timing.base_prune_ms = journal::millis(teardown.prune);
    if let Some(Value::Object(report)) = &mut log.report {
        report.insert("exit".into(), code.into());
        if let Some(window) = &window {
            report.entry("window").or_insert_with(|| window.record());
        }
    }
    log.blocked = code == 2;
    written(root, lost, green, asked.as_deref(), &mut log);
    log.asked = asked.unwrap_or_default();
    if let Ok(at) = state::ready(root) {
        let held = count(&at);
        log.gate_spent = held.gate_spent;
        log.build_blocks = held.builds;
        log.prompt = held.prompt;
    }
    let said = tell(args, root, code, note, &mut log);
    log.timing.total_ms = journal::millis(begun.elapsed());
    journal::stop(root, &log);
    if let Some(said) = said {
        host::answering(event.as_ref()).stop(&Stop::Tell(said));
    }
    code
}

/// The verdict this stop leaves for the next prompt, or the reason it left none: another stop
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
            "klin: NOTE: another stop in this worktree held the state directory for the whole \
             {} ms klin waits, so this stop wrote no verdict and the window stays as it is.",
            BUDGET.as_millis()
        );
        log.why = Some("another stop held the state directory, so this stop wrote no verdict");
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
    let intervened = log.gate_spent || turn::intervened(root);
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
    if log.gate_spent && log.verdict == "red" && no_prompt_event(tail, log.session.as_deref()) {
        log.flags.push("no-prompt-event");
        parts.push((
            "note",
            "klin: no prompt event reached this session; klin will not block again until \
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
/// a gate, and the note a stop nothing blocks leaves for the person. Spec 8.2, 16.3.
fn ran(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
    event: Option<&host::Event>,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, bool, Option<Vec<String>>, Option<String>) {
    let (outcome, build_ms) = journal::timed(|| built(args, project, window));
    log.timing.build_ms = build_ms;
    match outcome {
        Ok((Some(failure), said)) => {
            let code = does_not_build(args, project.root(), &failure, &said, window, log, out);
            let text = format!("klin: {}:\n{failure}", does_not_build_said());
            (
                blocked_build(project.root(), event, text, code),
                false,
                None,
                None,
            )
        }
        Err(problem) => {
            let (code, note) = handed(args, project, Err(problem), event, log, out);
            (code, false, None, note)
        }
        Ok((None, said)) => {
            let judged = judge(args, project, window, &said, out);
            let green = matches!(&judged, Ok(tally) if tally.failed == 0 && tally.errored == 0);
            let reported = judged
                .as_ref()
                .map(|tally| tally.reported.clone())
                .unwrap_or_default();
            let (code, note) = handed(args, project, judged, event, log, out);
            let asked = (code == 2).then_some(reported);
            (code, green, asked, note)
        }
    }
}

/// What the hook does with a run it finished: report it, and block the stop or let it end with
/// the note it leaves for the person.
fn handed(
    args: &Args,
    project: &Project,
    outcome: Result<Tally, Error>,
    event: Option<&host::Event>,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, Option<String>) {
    let mut tally = match refused(args, outcome, out) {
        Ok(tally) => tally,
        Err(problem) => {
            let _ = writeln!(out, "FAIL: {problem}");
            let mut records = Records::default();
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
    /// The site id of every finding the run reported, which a stop that blocks records as asked.
    reported: Vec<String>,
    /// The 11.2 object the run built, which the journal writes as the stop's line. Spec 11.4.
    record: Option<Value>,
}

/// What the hook says about a tree that does not build. Both messages name the bound from
/// `BLOCKS`, so the cap and the words for it cannot drift apart.
fn does_not_build_said() -> String {
    format!(
        "the tree does not build, so no gate ran (each stop blocks until it does, up to \
         {BLOCKS} in one turn)"
    )
}

fn stopped_blocking() -> String {
    format!(
        "the build has blocked {BLOCKS} stops under this prompt, so klin stops blocking; the \
         failure stands and CI will refuse it."
    )
}

/// The build stamp: one record per prompt. The prompt counter of the turn file it was taken
/// under, how many stops a build failure already blocked, and whether the turn's one gate
/// block is spent. A record taken under an earlier prompt reads as zero, so every prompt gets
/// the whole budget. Spec 16.3.
struct Count {
    prompt: u64,
    builds: u64,
    gate_spent: bool,
}

fn count(at: &Path) -> Count {
    let prompt = turn::prompts(at);
    let held = std::fs::read_to_string(at.join(BUILD_BLOCKED))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(|held| held.get("prompt").and_then(Value::as_u64) == Some(prompt));
    Count {
        prompt,
        builds: held
            .as_ref()
            .and_then(|held| held.get("builds")?.as_u64())
            .unwrap_or_default(),
        gate_spent: held
            .as_ref()
            .and_then(|held| held.get("gate_spent")?.as_bool())
            .unwrap_or_default(),
    }
}

/// Whether the record reached the disk. A count klin cannot write bounds nothing, so the
/// caller reports the build failure and does not block on it. Spec 14.
fn counted(at: &Path, count: &Count) -> bool {
    let text = serde_json::json!({
        "prompt": count.prompt,
        "builds": count.builds,
        "gate_spent": count.gate_spent,
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
    root: &Path,
    failure: &str,
    said: &[check::Said],
    window: Option<&Window>,
    log: &mut journal::Stop,
    out: &mut String,
) -> u8 {
    let builds = raised(root, log);
    let stopped = builds.is_some_and(|builds| builds > BLOCKS);
    let code = match builds {
        Some(builds) if builds <= BLOCKS => 2,
        _ => 0,
    };
    log.report = Some(reported(args, failure, said, window, stopped, code, out));
    code
}

/// A host that cannot read stderr still has to show the build failure. An adapter whose stop
/// already reads stderr ignores the text and returns 2.
fn blocked_build(root: &Path, event: Option<&host::Event>, text: String, code: u8) -> u8 {
    match code {
        2 => block(root, host::answering(event), text),
        _ => code,
    }
}

/// The block this build failure spends, or `None` when klin could not record it, either
/// because the state directory is gone or because the record itself would not write. Neither
/// count could bound the blocks, so the NOTE names the write that failed and the stop is not
/// blocked. Spec 14.
fn raised(root: &Path, log: &mut journal::Stop) -> Option<u64> {
    let at = match state::ready(root) {
        Ok(at) => at,
        Err(why) => return unbounded(&why, log),
    };
    let held = count(&at);
    let count = Count {
        builds: held.builds + 1,
        ..held
    };
    match counted(&at, &count) {
        true => Some(count.builds),
        false => unbounded(
            &format!("{} could not be written", at.join(BUILD_BLOCKED).display()),
            log,
        ),
    }
}

fn unbounded(why: &str, log: &mut journal::Stop) -> Option<u64> {
    log.flags.push("count-unwritable");
    eprintln!(
        "klin: NOTE: {why} — so no count could bound the build blocks, and this build failure \
         blocks nothing."
    );
    None
}

/// The build failure as a person and an agent read it, and as `--json` records it. The note
/// says that klin stopped blocking, because the exit code alone no longer says it. Spec 11.
fn reported(
    args: &Args,
    failure: &str,
    built: &[check::Said],
    window: Option<&Window>,
    stopped: bool,
    code: u8,
    out: &mut String,
) -> Value {
    let said = does_not_build_said();
    let mut records = Records {
        derived: built
            .iter()
            .filter_map(|(_, entry)| entry.clone())
            .collect(),
        ..Records::default()
    };
    records
        .findings
        .push(record("error", &format!("{said}:\n{failure}")));
    if stopped {
        records.notes.push(record("note", &stopped_blocking()));
    }
    let object = as_json(ERROR, code, &format!("klin: {said}."), records, window);
    if !args.json {
        eprintln!("klin: {said}:");
        eprint!("{failure}");
        if stopped {
            eprintln!("klin: {}", stopped_blocking());
        }
        return object;
    }
    out.clear();
    let _ = writeln!(out, "{object}");
    object
}

/// The build a person chose or the one the manifests derive, run before any gate judges the
/// tree it produces, with the provenance of each command the gates' report carries. Only the
/// hook builds, so no other run derives a build. Spec 5.4, ADR 0012.
fn built(
    args: &Args,
    project: &Project,
    window: Option<&Window>,
) -> Result<(Option<String>, Vec<check::Said>), Error> {
    let plan = build::plan(project)?;
    if plan.entries.is_empty() {
        return Ok((None, plan.said));
    }
    let changes = scoped(args, project, &plan.entries, window)?;
    let failure = build::failure(
        project.root(),
        &build::wanted(&plan.entries, changes.as_deref()),
    );
    Ok((failure, plan.said))
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
    built: &[check::Said],
    out: &mut String,
) -> Result<Tally, Error> {
    let plan = plan(project)?;
    if args.list {
        return listed(project, &plan, out);
    }
    let wanted = select(&args.gates, &plan, project)?;
    let against = against(args, &wanted, project, window, out)?;
    said(args, built, out);
    let rootless = no_source_root(args, &plan, project, out)?;
    let (mut tally, mut records) = each(args, &wanted, project, &against, out);
    records.notes.extend(rootless);
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
}

fn against<'a>(
    args: &Args,
    wanted: &[&Gate],
    project: &'a Project,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Against<'a>, Error> {
    let base = base(args, wanted, project, window, out)?;
    let changes = changes(args, project, base.as_ref(), out)?;
    let scope = changes
        .as_ref()
        .map(|changed| changed.iter().map(|change| change.path.clone()).collect());
    let prior = prior(project, base.as_ref(), changes.as_deref(), wanted)?;
    Ok(Against {
        scope,
        changes,
        prior,
        base,
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

fn listed(project: &Project, plan: &Plan, out: &mut String) -> Result<Tally, Error> {
    if plan.gates.is_empty() && plan.excluded.is_empty() {
        return Err(no_gate(project, plan));
    }
    list(project, plan, out);
    if let Some(at) = state::dir(project.root()) {
        let _ = writeln!(out, "state: {}", at.display());
    }
    Ok(Tally::default())
}

fn list(project: &Project, plan: &Plan, out: &mut String) {
    for gate in &plan.gates {
        let _ = writeln!(out, "{} — runs", gate.name);
        for line in stated(project, gate) {
            let _ = writeln!(out, "{UNDER}{line}");
        }
    }
    for name in &plan.excluded {
        let _ = writeln!(out, "{name} — excluded");
    }
    for check in &plan.needs_a_section {
        let _ = writeln!(out, "{} — needs a section a person writes", check.name);
    }
}

/// The `pinned:` line of each value a person wrote into a gate's section. `--list` derives
/// nothing, so a value the section leaves out is said by the run that derives it. Spec 10.
fn stated(project: &Project, gate: &Gate) -> Vec<String> {
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
/// exit 2 under `--strict`, and a NOTE otherwise, which lets the turn end in the hook. Without
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
    project: &Project,
    out: &mut String,
) -> Result<Option<Value>, Error> {
    let dropped = plan.gates.iter().any(|gate| gate.check.reads_code())
        || plan.needs_a_section.iter().any(|check| check.reads_code());
    if !dropped || !project.found_no_source_root() {
        return Ok(None);
    }
    let said = format!(
        "the survey of {} found no source root — a source root is a directory that holds \
         nothing but source files, so no gate that reads code ran here at all; run klin from \
         the tree you mean to gate, or set those gates to false to exclude them",
        project.root().display()
    );
    if args.strict {
        return Err(Error(said));
    }
    if !args.json {
        let _ = writeln!(out, "  NOTE: {said}");
    }
    Ok(Some(record("note", &said)))
}

fn finish(
    args: &Args,
    plan: &Plan,
    gates: usize,
    tally: &Tally,
    records: Records,
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
fn said(args: &Args, built: &[check::Said], out: &mut String) {
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
fn as_json(status: &str, code: u8, tally: &str, records: Records, base: Option<&Window>) -> Value {
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
    let mut records = Records::default();
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
    event: Option<&host::Event>,
    log: &mut journal::Stop,
) -> (u8, Option<String>) {
    let (failed, errored) = (tally.failed, tally.errored);
    let held = state::ready(root).ok().map(|at| (count(&at), at));
    unwritable(root);
    if failed == 0 && errored == 0 {
        return nothing_blocks(args, tally.told, report, event);
    }
    let Some(event) = event else {
        eprint!("{report}");
        return (1, None);
    };
    let again = gate_spent(held.as_ref(), event.blocked_before);
    let tail = match again {
        true => " — still, after one round of fixes:",
        false => " — fix what each names, then stop again:",
    };
    let lead = format!("klin: {}{tail}", lead(failed, errored));
    eprintln!("{lead}");
    eprint!("{report}");
    if !again {
        let said = format!("{lead}\n{report}");
        return (spend(root, held, log, event.host, &said), None);
    }
    eprintln!(
        "klin: not blocking a second time; the window stays open until a person fixes, accepts \
         or resets it."
    );
    (event.host.stop(&Stop::Pass), None)
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
    event: Option<&host::Event>,
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

/// Whether the turn's one gate block is already spent. The build stamp is the record. The
/// host's flag is a second opinion for the first gate block only, because after a build block
/// that flag is true while the gate block is still unspent. Spec 16.3.
fn gate_spent(held: Option<&(Count, PathBuf)>, blocked_before: bool) -> bool {
    match held {
        Some((count, _)) => count.gate_spent || (count.builds == 0 && blocked_before),
        None => blocked_before,
    }
}

/// The block the gate takes, recorded so the stop after it reports and lets the turn end.
fn spend(
    root: &Path,
    held: Option<(Count, PathBuf)>,
    log: &mut journal::Stop,
    host: &dyn host::Adapter,
    said: &str,
) -> u8 {
    if let Some((count, at)) = held
        && !counted(
            &at,
            &Count {
                gate_spent: true,
                ..count
            },
        )
    {
        log.flags.push("count-unwritable");
    }
    block(root, host, said.to_string())
}

/// Record the exact report a follow-up host will echo, then deliver the block. Spec 9.1, 9.3.
fn block(root: &Path, host: &dyn host::Adapter, said: String) -> u8 {
    if host.follows_up() {
        turn::expect_followup(root, &said);
    }
    host.stop(&Stop::Block(said))
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
    names(check::names())
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
    for check in check::CATALOGUE {
        add(project, check, &mut plan)?;
    }
    distinct(project, &plan)?;
    Ok(plan)
}

/// A check's gates, from what the config states for its section and, where it states nothing,
/// from what the section's absence means for this check: an Automatic check runs when its facts
/// are available, and a Policy or Integration check runs nothing until a person writes the
/// section. Planning derives no expensive number or topology. Spec 4.6, 5.2, ADR 0038.
fn add(project: &Project, check: &'static check::Row, plan: &mut Plan) -> Result<(), Error> {
    let Some(stated) = project.config.pinned(check.section) else {
        absent(project, check, plan);
        return Ok(());
    };
    match stated {
        Value::Bool(false) => plan.excluded.push(check.name.to_string()),
        _ if check.gate_per_entry => {
            for (name, _) in check::named_entries(&project.config, check.section)? {
                plan.gates.push(Gate { name, check });
            }
        }
        _ => plan.one(check),
    }
    Ok(())
}

/// The gate a section's absence plans: the check itself for an Automatic check whose facts are
/// available, and otherwise a check that needs a section a person writes.
fn absent(project: &Project, check: &'static check::Row, plan: &mut Plan) {
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
) -> (Tally, Records) {
    let mut tally = Tally::default();
    let mut totals = Records::default();
    for gate in wanted {
        let ((code, text, records), ms) = journal::timed(|| one(args, gate, project, against));
        match code {
            0 => (),
            1 => tally.failed += 1,
            _ => tally.errored += 1,
        }
        for line in &records.derived_lines {
            let _ = writeln!(out, "  {line}");
        }
        let _ = writeln!(out, "  {}  {}", status(code), gate.name);
        for line in text.lines() {
            let _ = writeln!(out, "        {line}");
        }
        totals.gates.push(row(gate, code, &records, ms));
        gather(&mut totals, records, &gate.name);
    }
    tally.told = totals.notes.iter().filter(|note| told(note)).count();
    tally.reported = totals
        .findings
        .iter()
        .filter_map(|finding| finding.get("id")?.as_str().map(str::to_string))
        .collect();
    (tally, totals)
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
fn row(gate: &Gate, code: u8, records: &Records, ms: u64) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), gate.name.clone().into());
    out.insert("status".into(), status(code).trim_end().into());
    out.insert("findings".into(), records.findings.len().into());
    out.insert("notes".into(), records.notes.len().into());
    out.insert(
        "coverage".into(),
        records.coverage.clone().unwrap_or(Value::Null),
    );
    out.insert("ms".into(), ms.into());
    out.insert("held".into(), records.held.map_or(Value::Null, Value::from));
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
                "dependencies": graph.dependencies,
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
                "ms": journal::millis(surface.time),
            })
        }),
    );
    Value::Object(out)
}

/// A note the hook tells a person even when nothing blocks the stop: a file the run could not
/// read or stopped measuring, and a deleted test the run let through. Spec 8.2, 8.6, 14.
fn told(note: &Value) -> bool {
    let outcome = note.get("outcome").and_then(Value::as_str);
    matches!(
        outcome,
        Some(UNPARSED | DELETED | NOT_MEASURED | DERIVATION | UNRESOLVED)
    ) || coverage::is_lost(note)
}

fn gather(totals: &mut Records, mut records: Records, name: &str) {
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
             so lift the exclusion or drop --gate {name}",
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

fn one(args: &Args, gate: &Gate, project: &Project, against: &Against) -> (u8, String, Records) {
    let mut text = String::new();
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
        quiet: false,
    };
    let outcome = (gate.check.run)(
        &at,
        &mut Sink {
            text: &mut text,
            records: Some(&mut records),
        },
    );
    let (code, text) = match outcome {
        Ok(code) => (code, text),
        Err(problem) => (2, text + &format!("FAIL: {problem}")),
    };
    if code == 2 && records.findings.is_empty() {
        records.findings.push(record("error", &text));
    }
    (code, text, records)
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
