use std::cell::RefCell;
use std::fmt::Write;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::base::{self, Base, Prior};
use crate::changed::{self, Change};
use crate::config::{Config, Error, Flags, Records};
use crate::{complexity, doc_size, escapes};

const BUILD_BLOCKED: &str = ".klin-build-blocked";
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
}

pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let outcome = refused(args, judge(args, start, out), out);
    if !args.hook {
        return outcome.map(|(failed, errored)| code(failed, errored));
    }
    let (failed, errored) = match outcome {
        Ok(tally) => tally,
        Err(problem) => {
            let _ = writeln!(out, "FAIL: {problem}");
            (0, 1)
        }
    };
    Ok(hook(
        failed,
        errored,
        &std::mem::take(out),
        &root(args, start),
    ))
}

fn judge(args: &Args, start: &Path, out: &mut String) -> Result<(usize, usize), Error> {
    let config = Config::load(args.config.as_deref(), start)?;
    let plan = plan(&config)?;
    if args.list {
        return listed(&config, &plan, out);
    }
    let wanted = select(&args.gates, &plan, &config)?;
    accounted(args, &plan, &config)?;
    let against = against(args, &wanted, &config, out)?;
    let (failed, errored, records) = each(args, &wanted, &config.file, start, &against, out);
    finish(
        args,
        &plan,
        wanted.len(),
        (failed, errored),
        records,
        &against,
        out,
    );
    Ok((failed, errored))
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
        self.prior.as_ref().map(Prior::dir)
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
    let base = base::choose(config.root())?;
    if !args.json {
        let _ = writeln!(out, "  {}", base.line());
    }
    Ok(Some(base))
}

fn listed(config: &Config, plan: &Plan, out: &mut String) -> Result<(usize, usize), Error> {
    if plan.gates.is_empty() && plan.excluded.is_empty() {
        return Err(no_gate(config, plan));
    }
    list(plan, out);
    Ok((0, 0))
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
    tally: (usize, usize),
    records: Records,
    against: &Against,
    out: &mut String,
) {
    let (failed, errored) = tally;
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
        as_json(code(failed, errored), &line, records, against.base.as_ref())
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

fn refused(
    args: &Args,
    outcome: Result<(usize, usize), Error>,
    out: &mut String,
) -> Result<(usize, usize), Error> {
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
    Ok((0, 1))
}

fn problem_record(text: &str) -> Value {
    let mut out = Map::new();
    out.insert("outcome".into(), "error".into());
    out.insert("text".into(), text.trim_end().into());
    Value::Object(out)
}

fn hook(failed: usize, errored: usize, report: &str, root: &Path) -> u8 {
    let unspent = build_blocked(root);
    if failed == 0 && errored == 0 {
        return 0;
    }
    let Some(event) = event() else {
        eprint!("{report}");
        return 1;
    };
    let again = !unspent
        && event
            .get("stop_hook_active")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let tail = match again {
        true => " — still, after one round of fixes:",
        false => " — fix what each names, then stop again:",
    };
    eprintln!("klin: {}{tail}", lead(failed, errored));
    eprint!("{report}");
    if !again {
        return 2;
    }
    eprintln!("klin: not blocking a second time; the failure stands and CI will refuse it.");
    0
}

fn lead(failed: usize, errored: usize) -> &'static str {
    match (failed > 0, errored > 0) {
        (true, true) => "a quality gate failed, and another could not run",
        (true, false) => "a quality gate failed",
        _ => "could not run a quality gate",
    }
}

fn root(args: &Args, start: &Path) -> PathBuf {
    Config::load(args.config.as_deref(), start)
        .map(|config| config.root().to_path_buf())
        .unwrap_or_else(|_| start.to_path_buf())
}

fn build_blocked(root: &Path) -> bool {
    std::fs::remove_file(root.join(BUILD_BLOCKED)).is_ok()
}

fn event() -> Option<Value> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

fn prior(
    config: &Config,
    base: Option<&Base>,
    changes: Option<&[Change]>,
) -> Result<Option<Prior>, Error> {
    let Some(base) = base else {
        return Ok(None);
    };
    base::materialize(config.root(), base, changes).map(Some)
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
) -> (usize, usize, Records) {
    let (mut failed, mut errored) = (0, 0);
    let mut totals = Records::default();
    for gate in wanted {
        let (code, text, records) = one(args, gate, config, start, against);
        match code {
            0 => (),
            1 => failed += 1,
            _ => errored += 1,
        }
        let _ = writeln!(out, "  {}  {}", status(code), gate.name);
        for line in text.lines() {
            let _ = writeln!(out, "        {line}");
        }
        gather(&mut totals, records, &gate.name, code, &text);
    }
    (failed, errored, totals)
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

fn code(failed: usize, errored: usize) -> u8 {
    if errored > 0 {
        2
    } else if failed > 0 {
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
        quiet: true,
        strict: args.strict && gate.check.compares_to_base,
        only: against
            .scope
            .as_deref()
            .filter(|_| gate.check.takes_scope)
            .map(<[String]>::to_vec),
        records: args.json.then(|| RefCell::new(Records::default())),
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
