use std::cell::RefCell;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base::{self, Base, Prior};
use crate::changed::{self, Change};
use crate::config::{Config, Error, Flags, Records, UNPARSED};
use crate::host::{self, Stop};
use crate::{build, complexity, doc_citations, doc_size, escapes, state};

/// Where klin records that a build failed, so the stop that follows knows the turn's gate
/// block is still unspent. In the state directory, which an agent does not empty. ADR 0019.
const BUILD_BLOCKED: &str = "build-blocked";
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
    /// Agent Stop hook mode: the failures to stderr, exit 2 to block the first stop
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
    if args.hook {
        match built(args, start) {
            Ok(Some((root, failure))) => return Ok(does_not_build(args, &root, &failure, out)),
            Err(problem) => return Ok(handed(args, start, Err(problem), out)),
            Ok(None) => (),
        }
    }
    let judged = judge(args, start, out);
    if !args.hook {
        return refused(args, judged, out).map(code);
    }
    Ok(handed(args, start, judged, out))
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

const DOES_NOT_BUILD: &str =
    "the tree does not build, so no gate ran (every stop blocks until it does)";

fn does_not_build(args: &Args, root: &Path, failure: &str, out: &mut String) -> u8 {
    match state::ready(root) {
        Ok(state) => {
            let _ = std::fs::write(state.join(BUILD_BLOCKED), "");
        }
        Err(_) => unwritable(root),
    }
    if args.json {
        let mut records = Records::default();
        records
            .findings
            .push(problem_record(&format!("{DOES_NOT_BUILD}:\n{failure}")));
        out.clear();
        let _ = writeln!(
            out,
            "{}",
            as_json(2, &format!("klin: {DOES_NOT_BUILD}."), records, None)
        );
        return 2;
    }
    eprintln!("klin: {DOES_NOT_BUILD}:");
    eprint!("{failure}");
    2
}

/// The build the config names, run before any gate judges the tree it produces. The key
/// belongs to the hook, so a config with no "build" builds nothing and that is not an error.
fn built(args: &Args, start: &Path) -> Result<Option<(PathBuf, String)>, Error> {
    let Ok(config) = Config::load(args.config.as_deref(), start) else {
        return Ok(None);
    };
    let entries = build::entries(&config)?;
    if entries.is_empty() {
        return Ok(None);
    }
    let changes = scoped(args, &config, &entries)?;
    let failure = build::failure(config.root(), &build::wanted(&entries, changes.as_deref()));
    Ok(failure.map(|text| (config.root().to_path_buf(), text)))
}

fn root(args: &Args, start: &Path) -> PathBuf {
    Config::load(args.config.as_deref(), start)
        .map(|config| config.root().to_path_buf())
        .unwrap_or_else(|_| start.to_path_buf())
}

fn build_blocked(root: &Path) -> bool {
    state::dir(root).is_some_and(|at| std::fs::remove_file(at.join(BUILD_BLOCKED)).is_ok())
}

fn scoped(
    args: &Args,
    config: &Config,
    entries: &[build::Entry],
) -> Result<Option<Vec<Change>>, Error> {
    if !args.changed || entries.iter().all(|entry| entry.root.is_none()) {
        return Ok(None);
    }
    let base = base::choose(config.root(), args.strict)?;
    changed::files(config.root(), &base.commit).map(Some)
}

fn judge(args: &Args, start: &Path, out: &mut String) -> Result<Tally, Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let note = version(args, &config, out);
    let plan = plan(&config)?;
    if args.list {
        return listed(&config, &plan, out);
    }
    let wanted = select(&args.gates, &plan, &config)?;
    accounted(args, &plan, &config)?;
    let against = against(args, &wanted, &config, out)?;
    let (tally, mut records) = each(args, &wanted, &config.file, start, &against, out);
    records.notes.extend(note);
    finish(args, &plan, wanted.len(), tally, records, &against, out);
    Ok(tally)
}

/// What this run judges the working tree against: the base commit, laid out, and the files
/// a scoped run looks at.
#[derive(Default)]
struct Against {
    base: Option<Base>,
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
    out: &mut String,
) -> Result<Against, Error> {
    let base = base(args, wanted, config, out)?;
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
    out: &mut String,
) -> Result<Option<Base>, Error> {
    if !args.changed && !wanted.iter().any(|gate| gate.check.compares_to_base) {
        return Ok(None);
    }
    let base = base::choose(config.root(), args.strict)?;
    if !args.json {
        let _ = writeln!(out, "  {}", base.line());
    }
    Ok(Some(base))
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

fn as_json(code: u8, tally: &str, records: Records, base: Option<&Base>) -> String {
    let status = match code {
        0 => "PASS",
        1 => "FAIL",
        _ => "ERROR",
    };
    let mut out = Map::new();
    out.insert("status".into(), status.into());
    out.insert("summary".into(), tally.into());
    if let Some(base) = base {
        out.insert("base".into(), base.line().into());
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
    records.findings.push(problem_record(&problem.to_string()));
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

fn problem_record(text: &str) -> Value {
    let mut out = Map::new();
    out.insert("outcome".into(), "error".into());
    out.insert("text".into(), text.trim_end().into());
    Value::Object(out)
}

fn hook(args: &Args, tally: Tally, report: &str, root: &Path) -> u8 {
    let (failed, errored) = (tally.failed, tally.errored);
    let unspent = build_blocked(root);
    unwritable(root);
    if failed == 0 && errored == 0 {
        if tally.unread == 0 {
            return 0;
        }
        eprintln!("klin: nothing blocks the stop, and the run left a note:");
        eprint!("{report}");
        return 1;
    }
    let Some(event) = host::read(args.host.as_deref()) else {
        eprint!("{report}");
        return 1;
    };
    let again = !unspent && event.blocked_before;
    let tail = match again {
        true => " — still, after one round of fixes:",
        false => " — fix what each names, then stop again:",
    };
    eprintln!("klin: {}{tail}", lead(failed, errored));
    eprint!("{report}");
    if !again {
        return host::stop(&Stop::Block);
    }
    eprintln!("klin: not blocking a second time; the failure stands and CI will refuse it.");
    host::stop(&Stop::Pass)
}

/// A state directory klin cannot write costs a wider window and nothing else. Section 14.
fn unwritable(root: &Path) {
    if let Err(why) = state::ready(root) {
        eprintln!(
            "klin: NOTE: {why} — the window comes from HEAD, and a state klin cannot keep \
             blocks nothing."
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
    base: Option<&Base>,
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
    base: Option<&Base>,
    out: &mut String,
) -> Result<Option<Vec<Change>>, Error> {
    let (true, Some(base)) = (args.changed, base) else {
        return Ok(None);
    };
    let changed = changed::files(config.root(), &base.commit)?;
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
    named_after_a_command(config)?;
    let entries = entries(config)?;
    let mut plan = Plan::default();
    for check in CHECKS {
        add(config, check, &entries, &mut plan);
    }
    distinct(config, &plan)?;
    Ok(plan)
}

fn named_after_a_command(config: &Config) -> Result<(), Error> {
    let named = CHECKS
        .iter()
        .find(|check| check.name != check.section && config.section(check.name).is_ok());
    let Some(check) = named else {
        return Ok(());
    };
    Err(Error(format!(
        "{}: \"{}\" is what the command is called — the section it reads is \"{}\"",
        config.file.display(),
        check.name,
        check.section
    )))
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
        records.findings.push(problem_record(text));
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
        base: against.base.as_ref().map(|base| base.commit.clone()),
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
