use std::cell::RefCell;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::base::{self, Kind, Prior, Window};
use crate::changed::{self, Change};
use crate::config::{self, Config, DELETED, Error, Flags, Records, UNPARSED};
use crate::host::{self, Stop};
use crate::reference::{self, Key};
use crate::{
    build, complexity, coverage, doc_citations, doc_size, escapes, inventory, journal, lockfile,
    sarif, state, stats, stubs, survey, turn,
};

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
const GATES: &str = config::GATES.name;
/// What `--list` indents a gate's own lines by, under the row that names it.
const UNDER: &str = "      ";
/// The `ERR` row of 11.1 as `--json` names it, which a run that could not measure prints
/// whatever exit code it ends with.
const ERROR: &str = "ERROR";

struct Check {
    name: &'static str,
    section: &'static str,
    /// The configuration keys the section reads, declared in the check's own module and printed
    /// by `klin reference`. Spec 5.8.
    keys: &'static [Key],
    /// The language names this section selects a file set by, and none for a check that selects
    /// no language. Spec 5.8.
    languages: Option<reference::Languages>,
    run: fn(&Flags, &Path, &mut String) -> Result<u8, Error>,
    needs: Needs,
    takes_scope: bool,
    /// Whether the section is a list of entries a person writes, each its own gate under its
    /// own `name`, rather than one section the whole check runs under. Spec 8.3.
    gate_per_entry: bool,
}

/// What a check needs of the base: nothing, the commit the window names, or that commit laid
/// out as a tree beside the working one. A check that needs the commit or the tree is also the
/// kind `--strict` reaches, because it has a comparison or an accepted list to judge. Spec
/// 4.6, 10.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Needs {
    Nothing,
    TheCommit,
    TheTree,
}

impl Needs {
    /// Whether the run resolves the base commit for this check, which is also whether
    /// `--strict` reaches it. Spec 4.6, 10.
    fn the_commit(self) -> bool {
        self >= Needs::TheCommit
    }

    /// Whether the run lays the base commit out as a tree for this check.
    fn the_tree(self) -> bool {
        self == Needs::TheTree
    }
}

const CHECKS: &[Check] = &[
    Check {
        name: "doc-size",
        section: doc_size::SECTION,
        keys: doc_size::KEYS,
        languages: None,
        run: doc_size::gate,
        needs: Needs::Nothing,
        takes_scope: false,
        gate_per_entry: false,
    },
    Check {
        name: "doc-citations",
        section: doc_citations::SECTION,
        keys: doc_citations::KEYS,
        languages: None,
        run: doc_citations::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Check {
        name: "lockfile",
        section: lockfile::SECTION,
        keys: lockfile::KEYS,
        languages: None,
        run: lockfile::gate,
        needs: Needs::TheTree,
        takes_scope: false,
        gate_per_entry: false,
    },
    Check {
        name: "escapes",
        section: escapes::SECTION,
        keys: escapes::KIND.keys,
        languages: Some(escapes::language_extensions),
        run: escapes::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Check {
        name: "stubs",
        section: stubs::SECTION,
        keys: stubs::KIND.keys,
        languages: Some(stubs::language_extensions),
        run: stubs::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Check {
        name: "inventory",
        section: inventory::SECTION,
        keys: inventory::KEYS,
        languages: None,
        run: inventory::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Check {
        name: "complexity",
        section: complexity::SECTION,
        keys: complexity::KEYS,
        languages: Some(complexity::language_extensions),
        run: complexity::gate,
        needs: Needs::TheTree,
        takes_scope: true,
        gate_per_entry: false,
    },
    Check {
        name: "sarif",
        section: sarif::SECTION,
        keys: sarif::KEYS,
        languages: None,
        run: sarif::gate,
        needs: Needs::TheCommit,
        takes_scope: false,
        gate_per_entry: true,
    },
];

/// The section each check reads, which config.rs judges the top-level keys against, and the
/// command name that is not that section. A new check is one edit, here.
pub fn sections() -> impl Iterator<Item = &'static str> {
    CHECKS.iter().map(|check| check.section)
}

/// What each check tells `klin reference` about its section, in the order a run takes the
/// checks. Spec 5.8.
pub fn catalogue() -> impl Iterator<Item = reference::Section> {
    CHECKS.iter().map(|check| reference::Section {
        name: check.section,
        keys: check.keys,
        languages: check.languages,
    })
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
    /// The checks klin offers that neither the config nor the survey supplies a section for.
    /// Each needs a section a person writes, and none of them runs.
    needs_a_section: Vec<&'static Check>,
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
    if args.hook && !config::present(args.config.as_deref(), start) {
        return Ok(0);
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
        return refused(args, judged, out).map(|tally| code(&tally));
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
    let begun = std::time::Instant::now();
    let root = root(args, start);
    let event = host::read(args.host.as_deref());
    let mut log = journal::Stop::begun(event.as_ref(), config_hash(args, start));
    let (lock, lock_ms) =
        journal::timed(|| state::ready(&root).ok().map(|at| state::lock(&at, BUDGET)));
    log.timing.lock_ms = lock_ms;
    let lost = matches!(&lock, Some(None));
    let window = turn::window(&root, &mut log.flags, out).ok();
    if matches!(&window, Some(window) if matches!(window.kind, Kind::Branch)) {
        log.flags.push("branch-fallback");
    }
    let (code, green, asked, note) =
        ran(args, start, window.as_ref(), event.as_ref(), &mut log, out);
    if let (Some(Value::Object(report)), Some(window)) = (&mut log.report, &window) {
        report.entry("window").or_insert_with(|| window.record());
    }
    log.blocked = code == 2;
    written(&root, lost, green, asked.as_deref(), &mut log);
    log.asked = asked.unwrap_or_default();
    if let Ok(at) = state::ready(&root) {
        let held = count(&at);
        log.gate_spent = held.gate_spent;
        log.build_blocks = held.builds;
        log.prompt = held.prompt;
    }
    let said = tell(args, &root, code, note, &mut log);
    log.timing.total_ms = journal::millis(begun.elapsed());
    journal::stop(&root, &log);
    if let Some(said) = said {
        host::stop(&Stop::Tell(said));
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
        parts.extend(stats::turn_end(root, journal::line(log)));
    }
    log.told = parts.iter().map(|(part, _)| *part).collect();
    let said: Vec<String> = parts.into_iter().map(|(_, text)| text).collect();
    (!said.is_empty()).then(|| said.join("\n"))
}

/// A hash of the config in force, recorded in the journal and not read, so a later reader can
/// tell a fix from a config change. A tree with no file says `derived`, and a file klin could
/// not read says `unreadable`, so neither reads as an edit to the other.
fn config_hash(args: &Args, start: &Path) -> String {
    let Ok(config) = Config::load(args.config.as_deref(), start) else {
        return "unreadable".to_string();
    };
    if !config.written() {
        return "derived".to_string();
    }
    match std::fs::read_to_string(&config.file) {
        Ok(text) => format!("{:016x}", state::hash(text.as_bytes())),
        Err(_) => "unreadable".to_string(),
    }
}

/// One stop's run: the exit code the host reads, whether the gates left the tree green, the
/// findings a gate block put in front of the agent, which is `None` unless the stop blocked on
/// a gate, and the note a stop nothing blocks leaves for the person. Spec 8.2, 16.3.
fn ran(
    args: &Args,
    start: &Path,
    window: Option<&Window>,
    event: Option<&host::Event>,
    log: &mut journal::Stop,
    out: &mut String,
) -> (u8, bool, Option<Vec<String>>, Option<String>) {
    let (outcome, build_ms) = journal::timed(|| built(args, start, window));
    log.timing.build_ms = build_ms;
    match outcome {
        Ok(Some((root, failure))) => (
            does_not_build(args, &root, &failure, window, log, out),
            false,
            None,
            None,
        ),
        Err(problem) => {
            let (code, note) = handed(args, start, Err(problem), event, log, out);
            (code, false, None, note)
        }
        Ok(None) => {
            let judged = judge(args, start, window, out);
            let green = matches!(&judged, Ok(tally) if tally.failed == 0 && tally.errored == 0);
            let reported = judged
                .as_ref()
                .map(|tally| tally.reported.clone())
                .unwrap_or_default();
            let (code, note) = handed(args, start, judged, event, log, out);
            let asked = (code == 2).then_some(reported);
            (code, green, asked, note)
        }
    }
}

/// What the hook does with a run it finished: report it, and block the stop or let it end with
/// the note it leaves for the person.
fn handed(
    args: &Args,
    start: &Path,
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
        &root(args, start),
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
    log: &mut journal::Stop,
    out: &mut String,
) -> u8 {
    let builds = raised(root, log);
    let stopped = builds.is_some_and(|builds| builds > BLOCKS);
    let code = match builds {
        Some(builds) if builds <= BLOCKS => 2,
        _ => 0,
    };
    log.report = Some(reported(args, failure, window, stopped, code, out));
    code
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
    window: Option<&Window>,
    stopped: bool,
    code: u8,
    out: &mut String,
) -> Value {
    let said = does_not_build_said();
    let mut records = Records::default();
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
    let against = against(args, &wanted, &config, window, out)?;
    said(args, &config, out);
    let rootless = no_source_root(args, &plan, &config, out)?;
    let (mut tally, mut records) = each(args, &wanted, &config, start, &against, out);
    records.notes.extend(note);
    records.notes.extend(rootless);
    records.derived = config.derived_values();
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
        prior: prior(config, base.as_ref(), changes.as_deref(), wanted)?,
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
    if !args.changed
        && !wanted
            .iter()
            .any(|gate| gate.check.needs >= Needs::TheCommit)
    {
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
    list(config, plan, out);
    if let Some(at) = state::dir(config.root()) {
        let _ = writeln!(out, "state: {}", at.display());
    }
    Ok(Tally::default())
}

fn list(config: &Config, plan: &Plan, out: &mut String) {
    for gate in &plan.gates {
        let _ = writeln!(out, "{} — runs", gate.name);
        for line in stated(config, gate) {
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

/// Where each of a gate's values came from: the run's own `derived:` and `pinned:` lines for
/// the section it reads, and the `pinned:` lines of a section the config states in full, which
/// a run derives nothing for and so says nothing about. Spec 10.
fn stated(config: &Config, gate: &Gate) -> Vec<String> {
    let section = gate.check.section;
    let states = gate.with.as_ref().or_else(|| {
        config
            .pinned(section)
            .filter(|pinned| survey::pinned_whole(section, pinned))
    });
    match states {
        Some(value) => beside(config, section, value),
        None => config.said_about(section),
    }
}

/// A section a person states, key by key. An entry that states some of its check's keys leaves
/// the rest to the survey, and the run fills those in, so the row says `pinned` for what the
/// person named and `derived` for what the survey supplied. Spec 5.2, 10.
fn beside(config: &Config, section: &str, with: &Value) -> Vec<String> {
    let mut out = survey::pinned_lines(section, with);
    let said = config.said_about(section);
    for key in survey::underived(section, with) {
        let derived = format!("derived: {section} {key} ");
        out.extend(
            said.iter()
                .filter(|line| line.starts_with(&derived))
                .cloned(),
        );
    }
    out
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
    config: &Config,
    out: &mut String,
) -> Result<Option<Value>, Error> {
    let dropped = plan
        .needs_a_section
        .iter()
        .any(|check| survey::reads_code(check.section));
    if !dropped || !config.found_no_source_root() {
        return Ok(None);
    }
    let said = format!(
        "the survey of {} found no source root — a source root is a directory that holds \
         nothing but source files, so no gate that reads code ran here at all; run klin from \
         the tree you mean to gate, or set those gates to false to exclude them",
        config.root().display()
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

/// Every value this run derived and every one the config pinned beside it, printed once for
/// the whole run. Spec 4.3.
fn said(args: &Args, config: &Config, out: &mut String) {
    if args.json || !config.derives_anything() {
        return;
    }
    for line in config.derived_said() {
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
/// `exit` is exactly the code the caller is about to return, which holds for a direct `--json`
/// run. A `--hook` stop instead asks `hook()` for its own code afterward, from state this
/// function never sees, so `exit` there is the gates' code and not the stop's — the gap
/// `does_not_build` closes for itself, and the journal closes for the rest by recording the
/// stop's own outcome beside this object rather than inside it. Spec 11.4.
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
    eprintln!("klin: {}{tail}", lead(failed, errored));
    eprint!("{report}");
    if !again {
        return (spend(held, log), None);
    }
    eprintln!(
        "klin: not blocking a second time; the window stays open until a person fixes, accepts \
         or resets it."
    );
    (host::stop(&Stop::Pass), None)
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
fn spend(held: Option<(Count, PathBuf)>, log: &mut journal::Stop) -> u8 {
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
    wanted: &[&Gate],
) -> Result<Option<Prior>, Error> {
    let Some(base) = base.filter(|_| wanted.iter().any(|gate| gate.check.needs.the_tree())) else {
        return Ok(None);
    };
    base::materialize(config, &base.before, changes).map(Some)
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

fn plan(config: &Config) -> Result<Plan, Error> {
    let entries = entries(config)?;
    let mut plan = Plan::default();
    for check in CHECKS {
        add(config, check, &entries, &mut plan)?;
    }
    distinct(config, &plan)?;
    Ok(plan)
}

/// A check's gates: the one its section names, and one per `gates` entry that names it. A
/// `gates` entry is the person's statement of how that check runs, so klin derives no section
/// beside it and the whole tree is not measured twice. Spec 5.2.
fn add(
    config: &Config,
    check: &'static Check,
    entries: &[Entry],
    plan: &mut Plan,
) -> Result<(), Error> {
    let mine: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.check == check.name)
        .collect();
    let section = match mine.is_empty() {
        true => config.section(check.section).ok(),
        false => config.pinned(check.section),
    };
    if section.is_none() && mine.is_empty() {
        plan.needs_a_section.push(check);
    }
    from_section(config, check, section, plan)?;
    for entry in mine {
        from_entry(check, entry, plan);
    }
    Ok(())
}

fn from_section(
    config: &Config,
    check: &'static Check,
    section: Option<&Value>,
    plan: &mut Plan,
) -> Result<(), Error> {
    match section {
        Some(Value::Bool(false)) => plan.excluded.push(check.name.to_string()),
        Some(_) if check.gate_per_entry => {
            for (name, entry) in named_entries(config, check.section)? {
                plan.gates.push(Gate {
                    name,
                    check,
                    with: Some(entry),
                });
            }
        }
        Some(_) => plan.gates.push(Gate {
            name: check.name.to_string(),
            check,
            with: None,
        }),
        None => (),
    }
    Ok(())
}

/// The key every entry of a named section carries, whichever check reads the section.
pub const NAMED: Key = Key {
    name: "name",
    holds: "the gate's own name, which `--gate` takes",
    required: true,
    rule: None,
    default: "",
};

/// The entries of a section a person writes entry by entry, each with the name its gate takes.
/// Such a section is a list, and an entry with no `name` is a config error naming the key,
/// because nothing in a tree says which tool the entry runs. The check that reads one entry
/// reads its own list through this, so a gate's name is the name the check judges under.
/// Spec 8.3.
pub fn named_entries(config: &Config, section: &str) -> Result<Vec<(String, Value)>, Error> {
    let held = config.section(section)?;
    let listed = held.as_array().ok_or_else(|| {
        Error(format!(
            "{}: \"{section}\" is a list of entries, each its own gate under its own \"name\"",
            config.file.display()
        ))
    })?;
    listed
        .iter()
        .map(|entry| {
            let name = entry
                .get(NAMED.name)
                .and_then(Value::as_str)
                .ok_or_else(|| config.missing(section, NAMED.name))?;
            Ok((name.to_string(), entry.clone()))
        })
        .collect()
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
        name: text(NAMED.name)?,
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
    config: &Config,
    start: &Path,
    against: &Against,
    out: &mut String,
) -> (Tally, Records) {
    let mut tally = Tally::default();
    let mut totals = Records::default();
    for gate in wanted {
        let ((code, text, records), ms) =
            journal::timed(|| one(args, gate, config, start, against));
        match code {
            0 => (),
            1 => tally.failed += 1,
            _ => tally.errored += 1,
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

/// One gate's row in the JSON: what it is called, what it came to, how many findings and notes
/// it left, the scope it measured, how long its own measure and judge took, and the count its
/// `OK:` line prints as held at the base. Spec 11.2.
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
    Value::Object(out)
}

/// A note the hook tells a person even when nothing blocks the stop: a file the run could not
/// read or stopped measuring, and a deleted test the run let through. Spec 8.2, 8.6, 14.
fn told(note: &Value) -> bool {
    let outcome = note.get("outcome").and_then(Value::as_str);
    matches!(outcome, Some(UNPARSED | DELETED)) || coverage::is_lost(note)
}

fn gather(totals: &mut Records, mut records: Records, name: &str) {
    for record in records.findings.iter_mut().chain(records.notes.iter_mut()) {
        if let Some(fields) = record.as_object_mut() {
            fields.insert("gate".into(), name.into());
        }
    }
    totals.findings.append(&mut records.findings);
    totals.notes.append(&mut records.notes);
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
    config: &Config,
    start: &Path,
    against: &Against,
) -> (u8, String, Records) {
    let mut text = String::new();
    let flags = Flags {
        config: config.written().then(|| config.file.clone()),
        gate: gate.name.clone(),
        prior: against.dir().map(Path::to_path_buf),
        base: against.base.as_ref().map(|base| base.before.clone()),
        quiet: false,
        context: false,
        strict: args.strict && gate.check.needs.the_commit(),
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
    let mut records = flags.records.map(RefCell::into_inner).unwrap_or_default();
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
