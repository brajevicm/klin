use std::cell::RefCell;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::base::{self, Prior, Window};
use crate::changed::{self, Change};
use crate::config::{self, Config, Error, Flags, Records, UNPARSED};
use crate::host::{self, Stop};
use crate::{build, complexity, doc_citations, doc_size, escapes, state, turn};

/// Where klin records what one prompt already spent, so the stop that follows knows how many
/// build blocks are left and whether the turn's gate block is still unspent. In the state
/// directory, which an agent does not empty. ADR 0019, ADR 0022.
const BUILD_BLOCKED: &str = "build-blocked";
/// The name the build stamp is written under before the rename, so a stop that dies mid-write
/// leaves the previous record rather than a torn one.
const BUILD_WRITING: &str = "build-blocked.writing";
/// How many stops one prompt's build failures may block. klin bounds this itself, because the
/// host documents no cap of its own. ADR 0022, spec 9.3.
const BLOCKS: u64 = 8;
const GATES: &str = "gates";

struct Check {
    name: &'static str,
    section: &'static str,
    run: fn(&Flags, &Path, &mut String) -> Result<u8, Error>,
    compares_to_base: bool,
    takes_scope: bool,
}

const CHECKS: &[Check] = &[
    Check {
        name: "doc-size",
        section: "doc_size",
        run: doc_size::gate,
        compares_to_base: false,
        takes_scope: false,
    },
    Check {
        name: "doc-citations",
        section: "doc_citations",
        run: doc_citations::gate,
        compares_to_base: true,
        takes_scope: true,
    },
    Check {
        name: "escapes",
        section: "escapes",
        run: escapes::gate,
        compares_to_base: true,
        takes_scope: true,
    },
    Check {
        name: "complexity",
        section: "complexity",
        run: complexity::gate,
        compares_to_base: true,
        takes_scope: true,
    },
];

/// The section each check reads, which config.rs judges the top-level keys against, and the
/// command name that is not that section. A new check is one edit, here.
pub fn sections() -> impl Iterator<Item = &'static str> {
    CHECKS.iter().map(|check| check.section)
}

pub fn command_named(key: &str) -> Option<&'static str> {
    CHECKS
        .iter()
        .find(|check| check.name != check.section && check.name == key)
        .map(|check| check.section)
}

struct Gate {
    name: String,
    check: &'static Check,
    with: Option<Value>,
}

#[derive(Default)]
struct Plan {
    gates: Vec<Gate>,
    excluded: Vec<String>,
    unaccounted: Vec<&'static str>,
}

struct Entry {
    name: String,
    check: String,
    with: Option<Value>,
    off: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Fail when a gate is unaccounted for or an accepted entry matches nothing — what CI runs
    #[arg(long)]
    strict: bool,
    /// Run only this gate (repeatable)
    #[arg(long = "gate", value_name = "NAME")]
    gates: Vec<String>,
    /// Print the gates this config runs, excludes and leaves unaccounted, then exit
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
    if args.hook
        && let Some(problem) = unfixable_config(args, start)
    {
        eprintln!(
            "klin: FAIL: {problem} — only a person edits that file, so this stop is not blocked."
        );
        return Ok(1);
    }
    if !args.hook {
        let judged = judge(args, start, None, out);
        return refused(args, judged, out).map(code);
    }
    Ok(stopped(args, start, out))
}

/// How long a stop waits for the stop before it to finish. A fraction of the hook's five
/// seconds, because the stop still has a build and every gate to run inside them. Spec 13.
const BUDGET: Duration = Duration::from_secs(1);

/// One stop in the hook: the lock, the turn window, the build, the gates, and the verdict the
/// next prompt reads. The lock is held from before the run measures until after the verdict is
/// written, so an older stop cannot leave green over a newer red. Spec 6.5, 16.3.
fn stopped(args: &Args, start: &Path, out: &mut String) -> u8 {
    let root = root(args, start);
    let lock = state::ready(&root).ok().map(|at| state::lock(&at, BUDGET));
    let lost = matches!(&lock, Some(None));
    let window = turn::window(&root, out).ok();
    let (code, green) = ran(args, start, window.as_ref(), out);
    if lost {
        eprintln!(
            "klin: NOTE: another stop in this worktree held the state directory for the whole \
             {} ms klin waits, so this stop wrote no verdict and the window stays as it is.",
            BUDGET.as_millis()
        );
        return code;
    }
    let mut said = String::new();
    turn::verdict(&root, green, &mut said);
    eprint!("{said}");
    code
}

fn ran(args: &Args, start: &Path, window: Option<&Window>, out: &mut String) -> (u8, bool) {
    match built(args, start, window) {
        Ok(Some((root, failure))) => (does_not_build(args, &root, &failure, window, out), false),
        Err(problem) => (handed(args, start, Err(problem), out), false),
        Ok(None) => {
            let judged = judge(args, start, window, out);
            let green = matches!(&judged, Ok(tally) if tally.failed == 0 && tally.errored == 0);
            (handed(args, start, judged, out), green)
        }
    }
}

/// What the hook does with a run it finished: report it, and block the stop or let it end.
fn handed(args: &Args, start: &Path, outcome: Result<Tally, Error>, out: &mut String) -> u8 {
    let tally = match refused(args, outcome, out) {
        Ok(tally) => tally,
        Err(problem) => {
            let _ = writeln!(out, "FAIL: {problem}");
            Tally {
                errored: 1,
                ..Tally::default()
            }
        }
    };
    hook(args, tally, &std::mem::take(out), &root(args, start))
}

/// What one run came to: the gates that failed, the gates that could not run, and the files
/// no grammar read, which fail nothing in the hook and still reach a person.
#[derive(Default, Clone, Copy)]
struct Tally {
    failed: usize,
    errored: usize,
    unread: usize,
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
    let writing = at.join(BUILD_WRITING);
    if std::fs::write(&writing, text).is_ok()
        && std::fs::rename(&writing, at.join(BUILD_BLOCKED)).is_ok()
    {
        return true;
    }
    let _ = std::fs::remove_file(&writing);
    false
}

fn does_not_build(
    args: &Args,
    root: &Path,
    failure: &str,
    window: Option<&Window>,
    out: &mut String,
) -> u8 {
    let builds = raised(root);
    let stopped = builds.is_some_and(|builds| builds > BLOCKS);
    reported(args, failure, window, stopped, out);
    match builds {
        Some(builds) if builds <= BLOCKS => 2,
        _ => 0,
    }
}

/// The block this build failure spends, or `None` when klin could not record it, either
/// because the state directory is gone or because the record itself would not write. Neither
/// count could bound the blocks, so the NOTE names the write that failed and the stop is not
/// blocked. Spec 14.
fn raised(root: &Path) -> Option<u64> {
    let at = match state::ready(root) {
        Ok(at) => at,
        Err(why) => return unbounded(&why),
    };
    let held = count(&at);
    let count = Count {
        builds: held.builds + 1,
        ..held
    };
    match counted(&at, &count) {
        true => Some(count.builds),
        false => unbounded(&format!(
            "{} could not be written",
            at.join(BUILD_BLOCKED).display()
        )),
    }
}

fn unbounded(why: &str) -> Option<u64> {
    eprintln!(
        "klin: NOTE: {why} — so no count could bound the build blocks, and this build failure \
         blocks nothing."
    );
    None
}

/// The build failure as a person and an agent read it, and as `--json` records it. The note
/// says that klin stopped blocking, because the exit code alone no longer says it. Spec 11.
fn reported(args: &Args, failure: &str, window: Option<&Window>, stopped: bool, out: &mut String) {
    let said = does_not_build_said();
    if !args.json {
        eprintln!("klin: {said}:");
        eprint!("{failure}");
        if stopped {
            eprintln!("klin: {}", stopped_blocking());
        }
        return;
    }
    let mut records = Records::default();
    records
        .findings
        .push(record("error", &format!("{said}:\n{failure}")));
    if stopped {
        records.notes.push(record("note", &stopped_blocking()));
    }
    out.clear();
    let _ = writeln!(
        out,
        "{}",
        as_json(2, &format!("klin: {said}."), records, window)
    );
}

/// The build the config names, run before any gate judges the tree it produces. The key
/// belongs to the hook, so a config with no "build" builds nothing and that is not an error.
fn built(
    args: &Args,
    start: &Path,
    window: Option<&Window>,
) -> Result<Option<(PathBuf, String)>, Error> {
    let Ok(config) = Config::load(args.config.as_deref(), start) else {
        return Ok(None);
    };
    let entries = build::entries(&config)?;
    if entries.is_empty() {
        return Ok(None);
    }
    let changes = scoped(args, &config, &entries, window)?;
    let failure = build::failure(config.root(), &build::wanted(&entries, changes.as_deref()));
    Ok(failure.map(|text| (config.root().to_path_buf(), text)))
}

/// A config the hook cannot act on: the file is there and reading it failed. The agent cannot
/// edit it, so a block would repeat every stop, the loop ADR 0021 closed. Section 14.
fn unfixable_config(args: &Args, start: &Path) -> Option<Error> {
    let problem = Config::load(args.config.as_deref(), start).err()?;
    config::present(args.config.as_deref(), start).then_some(problem)
}

fn root(args: &Args, start: &Path) -> PathBuf {
    Config::load(args.config.as_deref(), start)
        .map(|config| config.root().to_path_buf())
        .unwrap_or_else(|_| start.to_path_buf())
}

fn scoped(
    args: &Args,
    config: &Config,
    entries: &[build::Entry],
    window: Option<&Window>,
) -> Result<Option<Vec<Change>>, Error> {
    if !args.changed || entries.iter().all(|entry| entry.root.is_none()) {
        return Ok(None);
    }
    let base = chosen(window, config, args.strict)?;
    changed::files(config.root(), &base.before).map(Some)
}

fn judge(
    args: &Args,
    start: &Path,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Tally, Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let note = version(args, &config, out);
    let plan = plan(&config)?;
    if args.list {
        return listed(&config, &plan, out);
    }
    let wanted = select(&args.gates, &plan, &config)?;
    accounted(args, &plan, &config)?;
    let against = against(args, &wanted, &config, window, out)?;
    let (tally, mut records) = each(args, &wanted, &config.file, start, &against, out);
    records.notes.extend(note);
    finish(args, &plan, wanted.len(), tally, records, &against, out);
    Ok(tally)
}

/// What this run judges the working tree against: the base commit, laid out, and the files
/// a scoped run looks at.
#[derive(Default)]
struct Against {
    base: Option<Window>,
    scope: Option<Vec<String>>,
    prior: Option<Prior>,
}

impl Against {
    fn dir(&self) -> Option<&Path> {
        self.prior.as_ref().map(Prior::root)
    }
}

fn against(
    args: &Args,
    wanted: &[&Gate],
    config: &Config,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Against, Error> {
    let base = base(args, wanted, config, window, out)?;
    let changes = changes(args, config, base.as_ref(), out)?;
    Ok(Against {
        scope: changes
            .as_ref()
            .map(|changed| changed.iter().map(|change| change.path.clone()).collect()),
        prior: prior(config, base.as_ref(), changes.as_deref())?,
        base,
    })
}

fn base(
    args: &Args,
    wanted: &[&Gate],
    config: &Config,
    window: Option<&Window>,
    out: &mut String,
) -> Result<Option<Window>, Error> {
    if !args.changed && !wanted.iter().any(|gate| gate.check.compares_to_base) {
        return Ok(None);
    }
    let base = chosen(window, config, args.strict)?;
    if !args.json {
        let _ = writeln!(out, "  {}", base.line());
    }
    Ok(Some(base))
}

/// The window the run judges: the one the hook already read, or the base a run by hand and CI
/// choose for themselves. Spec 6.1, 6.3.
fn chosen(window: Option<&Window>, config: &Config, strict: bool) -> Result<Window, Error> {
    match window {
        Some(window) => Ok(window.clone()),
        None => base::choose(config.root(), strict),
    }
}

fn listed(config: &Config, plan: &Plan, out: &mut String) -> Result<Tally, Error> {
    if plan.gates.is_empty() && plan.excluded.is_empty() {
        return Err(no_gate(config, plan));
    }
    list(plan, out);
    if let Some(at) = state::dir(config.root()) {
        let _ = writeln!(out, "state: {}", at.display());
    }
    Ok(Tally::default())
}

fn list(plan: &Plan, out: &mut String) {
    for gate in &plan.gates {
        let _ = writeln!(out, "{}", gate.name);
    }
    for name in &plan.excluded {
        let _ = writeln!(out, "{name} — excluded");
    }
    for name in &plan.unaccounted {
        let _ = writeln!(out, "{name} — available, not configured");
    }
}

fn accounted(args: &Args, plan: &Plan, config: &Config) -> Result<(), Error> {
    if !args.strict || plan.unaccounted.is_empty() {
        return Ok(());
    }
    Err(Error(format!(
        "{} leaves these gates unaccounted for: {} — under --strict every gate klin offers \
         takes a decision, so configure each one, or set its section to false to exclude it",
        config.file.display(),
        plan.unaccounted.join(", ")
    )))
}

fn finish(
    args: &Args,
    plan: &Plan,
    gates: usize,
    tally: Tally,
    records: Records,
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
    if !args.json {
        let _ = writeln!(out, "{line}");
        return;
    }
    out.clear();
    let _ = writeln!(
        out,
        "{}",
        as_json(code(tally), &line, records, against.base.as_ref())
    );
}

fn as_json(code: u8, tally: &str, records: Records, base: Option<&Window>) -> String {
    let status = match code {
        0 => "PASS",
        1 => "FAIL",
        _ => "ERROR",
    };
    let mut out = Map::new();
    out.insert("status".into(), status.into());
    out.insert("summary".into(), tally.into());
    if let Some(base) = base {
        out.insert("window".into(), base.record());
    }
    out.insert("findings".into(), Value::Array(records.findings));
    out.insert("notes".into(), Value::Array(records.notes));
    Value::Object(out).to_string()
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
    out.clear();
    let _ = writeln!(
        out,
        "{}",
        as_json(2, &format!("klin: {problem}"), records, None)
    );
    Ok(Tally {
        errored: 1,
        ..Tally::default()
    })
}

/// The note a config that names another klin version leaves: printed above the gates, and
/// carried into the JSON records. A version klin does not carry fails nothing. Section 5.2.
/// The hook drops a report that blocks nothing, so there the note goes straight to stderr.
fn version(args: &Args, config: &Config, out: &mut String) -> Option<Value> {
    let text = config.version_note()?;
    if args.hook {
        eprintln!("klin: {text}");
    } else if !args.json {
        let _ = writeln!(out, "  {text}");
    }
    let mut record = Map::new();
    record.insert("outcome".into(), "version".into());
    record.insert("text".into(), text.into());
    Some(Value::Object(record))
}

fn record(outcome: &str, text: &str) -> Value {
    let mut out = Map::new();
    out.insert("outcome".into(), outcome.into());
    out.insert("text".into(), text.trim_end().into());
    Value::Object(out)
}

fn hook(args: &Args, tally: Tally, report: &str, root: &Path) -> u8 {
    let (failed, errored) = (tally.failed, tally.errored);
    let held = state::ready(root).ok().map(|at| (count(&at), at));
    unwritable(root);
    if failed == 0 && errored == 0 {
        return nothing_blocks(tally.unread, report);
    }
    let Some(event) = host::read(args.host.as_deref()) else {
        eprint!("{report}");
        return 1;
    };
    let again = gate_spent(held.as_ref(), event.blocked_before);
    let tail = match again {
        true => " — still, after one round of fixes:",
        false => " — fix what each names, then stop again:",
    };
    eprintln!("klin: {}{tail}", lead(failed, errored));
    eprint!("{report}");
    if !again {
        return spend(held);
    }
    eprintln!("klin: not blocking a second time; the failure stands and CI will refuse it.");
    host::stop(&Stop::Pass)
}

/// What the hook says about a stop nothing blocks: nothing at all, or the note the run left.
fn nothing_blocks(unread: usize, report: &str) -> u8 {
    if unread == 0 {
        return 0;
    }
    eprintln!("klin: nothing blocks the stop, and the run left a note:");
    eprint!("{report}");
    1
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
fn spend(held: Option<(Count, PathBuf)>) -> u8 {
    if let Some((count, at)) = held {
        counted(
            &at,
            &Count {
                gate_spent: true,
                ..count
            },
        );
    }
    host::stop(&Stop::Block)
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
    config: &Config,
    base: Option<&Window>,
    changes: Option<&[Change]>,
) -> Result<Option<Prior>, Error> {
    let Some(base) = base else {
        return Ok(None);
    };
    base::materialize(config, base, changes).map(Some)
}

fn changes(
    args: &Args,
    config: &Config,
    base: Option<&Window>,
    out: &mut String,
) -> Result<Option<Vec<Change>>, Error> {
    let (true, Some(base)) = (args.changed, base) else {
        return Ok(None);
    };
    let changed = changed::files(config.root(), &base.before)?;
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
    names(CHECKS.iter().map(|check| check.name))
}

fn no_gate(config: &Config, plan: &Plan) -> Error {
    if !plan.excluded.is_empty() {
        return Error(format!(
            "{} excludes every gate it names: {} — a run that measures nothing cannot pass, \
             so lift one exclusion",
            config.file.display(),
            names(plan.excluded.iter().map(String::as_str))
        ));
    }
    Error(format!(
        "{} configures no gate — name at least one of: {}",
        config.file.display(),
        every_check()
    ))
}

fn plan(config: &Config) -> Result<Plan, Error> {
    let entries = entries(config)?;
    let mut plan = Plan::default();
    for check in CHECKS {
        add(config, check, &entries, &mut plan);
    }
    distinct(config, &plan)?;
    Ok(plan)
}

fn add(config: &Config, check: &'static Check, entries: &[Entry], plan: &mut Plan) {
    let mine: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.check == check.name)
        .collect();
    let section = config.section(check.section).ok();
    if section.is_none() && mine.is_empty() {
        plan.unaccounted.push(check.name);
    }
    from_section(check, section, plan);
    for entry in mine {
        from_entry(check, entry, plan);
    }
}

fn from_section(check: &'static Check, section: Option<&Value>, plan: &mut Plan) {
    match section {
        Some(Value::Bool(false)) => plan.excluded.push(check.name.to_string()),
        Some(_) => plan.gates.push(Gate {
            name: check.name.to_string(),
            check,
            with: None,
        }),
        None => (),
    }
}

fn from_entry(check: &'static Check, entry: &Entry, plan: &mut Plan) {
    if entry.off {
        plan.excluded.push(entry.name.clone());
        return;
    }
    plan.gates.push(Gate {
        name: entry.name.clone(),
        check,
        with: entry.with.clone(),
    });
}

fn distinct(config: &Config, plan: &Plan) -> Result<(), Error> {
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
                config.file.display()
            )));
        }
        seen.push(name);
    }
    Ok(())
}

fn entries(config: &Config) -> Result<Vec<Entry>, Error> {
    let Ok(named) = config.section(GATES) else {
        return Ok(Vec::new());
    };
    let shape = || {
        Error(format!(
            "{}: \"{GATES}\" is a list of {{\"name\", \"check\", \"with\"}} entries",
            config.file.display()
        ))
    };
    let mut out = Vec::new();
    for item in named.as_array().ok_or_else(shape)? {
        out.push(entry(config, item.as_object().ok_or_else(shape)?)?);
    }
    Ok(out)
}

fn entry(config: &Config, item: &Map<String, Value>) -> Result<Entry, Error> {
    let text = |key: &str| {
        item.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| config.missing(GATES, key))
    };
    let entry = Entry {
        name: text("name")?,
        check: text("check")?,
        with: item.get("with").cloned(),
        off: item.get("off").and_then(Value::as_bool).unwrap_or(false),
    };
    if !CHECKS.iter().any(|check| check.name == entry.check) {
        return Err(Error(format!(
            "{}: the gate {} names no check called \"{}\" — one of: {}",
            config.file.display(),
            entry.name,
            entry.check,
            every_check()
        )));
    }
    if entry.with.is_none() && !entry.off {
        return Err(config.missing(GATES, "with"));
    }
    Ok(entry)
}

fn each(
    args: &Args,
    wanted: &[&Gate],
    config: &Path,
    start: &Path,
    against: &Against,
    out: &mut String,
) -> (Tally, Records) {
    let mut tally = Tally::default();
    let mut totals = Records::default();
    for gate in wanted {
        let (code, text, records) = one(args, gate, config, start, against);
        match code {
            0 => (),
            1 => tally.failed += 1,
            _ => tally.errored += 1,
        }
        let _ = writeln!(out, "  {}  {}", status(code), gate.name);
        for line in text.lines() {
            let _ = writeln!(out, "        {line}");
        }
        gather(&mut totals, records, &gate.name, code, &text);
    }
    tally.unread = totals.notes.iter().filter(|note| unread(note)).count();
    (tally, totals)
}

fn unread(note: &Value) -> bool {
    note.get("outcome").and_then(Value::as_str) == Some(UNPARSED)
}

fn gather(totals: &mut Records, mut records: Records, name: &str, code: u8, text: &str) {
    if code == 2 && records.findings.is_empty() {
        records.findings.push(record("error", text));
    }
    for record in records.findings.iter_mut().chain(records.notes.iter_mut()) {
        if let Some(fields) = record.as_object_mut() {
            fields.insert("gate".into(), name.into());
        }
    }
    totals.findings.append(&mut records.findings);
    totals.notes.append(&mut records.notes);
}

fn code(tally: Tally) -> u8 {
    if tally.errored > 0 {
        2
    } else if tally.failed > 0 {
        1
    } else {
        0
    }
}

fn select<'a>(named: &[String], plan: &'a Plan, config: &Config) -> Result<Vec<&'a Gate>, Error> {
    for name in named {
        known(name, plan, config)?;
    }
    let wanted: Vec<&Gate> = plan
        .gates
        .iter()
        .filter(|gate| named.is_empty() || named.iter().any(|wanted| wanted == &gate.name))
        .collect();
    if wanted.is_empty() {
        return Err(no_gate(config, plan));
    }
    Ok(wanted)
}

fn known(name: &str, plan: &Plan, config: &Config) -> Result<(), Error> {
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

fn one(
    args: &Args,
    gate: &Gate,
    config: &Path,
    start: &Path,
    against: &Against,
) -> (u8, String, Records) {
    let mut text = String::new();
    let flags = Flags {
        config: Some(config.to_path_buf()),
        gate: gate.name.clone(),
        prior: against.dir().map(Path::to_path_buf),
        base: against.base.as_ref().map(|base| base.before.clone()),
        quiet: true,
        strict: args.strict && gate.check.compares_to_base,
        hook: args.hook,
        only: against
            .scope
            .as_deref()
            .filter(|_| gate.check.takes_scope)
            .map(<[String]>::to_vec),
        records: Some(RefCell::new(Records::default())),
        with: gate
            .with
            .clone()
            .map(|values| (gate.check.section.to_string(), values)),
    };
    let (code, text) = match (gate.check.run)(&flags, start, &mut text) {
        Ok(code) => (code, text),
        Err(problem) => (2, text + &format!("FAIL: {problem}")),
    };
    let records = flags.records.map(RefCell::into_inner).unwrap_or_default();
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
