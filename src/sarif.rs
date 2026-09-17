//! The `sarif` check: a scanner's own report, judged against the lines the window changed.
//!
//! Identity: the repository path plus `rule id: message text`, so a result that shifted lines
//! still matches. Ratcheted value: `count`, how many results one site holds. Derivation: none,
//! because nothing in a tree says which scanner ran. A person writes the section, and each
//! entry of it is one gate. Spec 8.3.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

use serde_json::{Map, Value};

use crate::base;
use crate::check::{self, Context, Sink};
use crate::config::{Config, Error};
use crate::coverage::Coverage;
use crate::hunks::Hunks;
use crate::project::Project;
use crate::ratchet::{self, Evaluator, Finding, Line, Values};
use crate::reference::Key;

pub const SECTION: &str = "sarif";

/// The keys this section reads, which `klin reference` prints. Spec 5.4, 5.8.
pub const KEYS: &[Key] = &[check::NAMED, REPORT, RUN, DIFFERENTIAL];

const REPORT: Key = Key {
    name: "report",
    holds: "the SARIF file this gate reads",
    required: true,
    rule: None,
    default: "",
};

const RUN: Key = Key {
    name: "run",
    holds: "the command that writes the report before the gate reads it",
    required: false,
    rule: None,
    default: "klin reads the report as it finds it and refuses one that predates the change",
};

const DIFFERENTIAL: Key = Key {
    name: "differential",
    holds: "whether only a finding on a line the window changed is judged",
    required: false,
    rule: None,
    default: "`false`",
};
const COUNT: &str = "count";
const METRICS: &[&str] = &[COUNT];
/// How many `originalUriBaseIds` entries one location is resolved through, so a report whose
/// ids point at each other resolves rather than recurring forever.
const DEPTH: usize = 4;
const REMEDY: &str = "Fix the result the scanner reports on the line this window wrote, or \
    delete the line. A result on a line the window did not change is held, so what this gate \
    names is what this change brought.";

/// One entry of the section: the report the scanner writes, the command that writes it, and
/// whether the scanner already reports only what is new. The entry's `name` is the gate's name,
/// which the runner read before this check ran. Spec 8.3.
struct Entry {
    report: PathBuf,
    run: Option<String>,
    differential: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// The klin.json to run under (default: the nearest one above the working directory)
    #[arg(long)]
    config: Option<PathBuf>,
    /// Fail when an accepted entry matches nothing — what CI runs
    #[arg(long)]
    strict: bool,
    /// Print nothing on success
    #[arg(long)]
    quiet: bool,
}

/// Every entry of the section, judged one after another, which is what `klin gate` does with
/// one gate per entry. The worst outcome is the command's. Spec 8.3, 8.6.
pub fn run(args: &Args, start: &Path, out: &mut String) -> Result<u8, Error> {
    let project = Project::load(args.config.as_deref(), start)?;
    let mut worst = 0;
    for (name, _) in check::named_entries(&project.config, SECTION)? {
        worst = worst.max(gate(
            &context(args, &project, &name),
            &mut Sink::unrecorded(out),
        )?);
    }
    Ok(worst)
}

fn context<'a>(args: &'a Args, project: &'a Project, name: &'a str) -> Context<'a> {
    Context {
        strict: args.strict,
        quiet: args.quiet,
        ..Context::by_hand(name, project)
    }
}

pub fn gate(at: &Context, out: &mut Sink) -> Result<u8, Error> {
    let config = at.config();
    let entry = entry(config, at.gate)?;
    let (found, changed) = read(config, &entry, at, out)?;
    let coverage = covered(&found);
    let judged = judge(found.placed, &changed, entry.differential);
    let accepted = ratchet::accepted(config, at.gate, METRICS)?;
    let state = said(&judged, entry.differential);
    let tail = coverage.said(out);
    let code = evaluator().evaluate(
        judged.findings,
        Vec::new(),
        accepted,
        at,
        Line {
            state: &state,
            tail: &tail,
        },
        out,
    );
    ratchet::noted(&found.notes, out);
    Ok(code)
}

/// What this gate discovered, in files, which is the unit every other gate counts: a scanner
/// reports results, and each one names a file. It measures every result it placed, because the
/// window decides which of them fail and not which it read. A location it could not place is
/// unreadable, and every result with no location at all is one such place. Spec 8.6.
fn covered(found: &Placed) -> Coverage {
    let measured = distinct(found.placed.iter().map(|site| site.file.as_str()));
    let unreadable = distinct(found.notes.iter().map(|(at, _)| at.as_str()));
    Coverage {
        found: measured + unreadable,
        measured,
        not_measured: 0,
        excluded: 0,
        unreadable,
    }
}

fn distinct<'a>(places: impl Iterator<Item = &'a str>) -> usize {
    let mut places: Vec<&str> = places.collect();
    places.sort_unstable();
    places.dedup();
    places.len()
}

/// The one entry of the section this gate runs under, found by the name the runner gave it.
/// Spec 8.3.
fn entry(config: &Config, gate: &str) -> Result<Entry, Error> {
    let entries = check::named_entries(config, SECTION)?;
    let (_, held) = entries
        .into_iter()
        .find(|(name, _)| name == gate)
        .ok_or_else(|| shape(config))?;
    let held = held.as_object().ok_or_else(|| shape(config))?;
    let report = held
        .get(REPORT.name)
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, REPORT.name))?;
    Ok(Entry {
        report: config.path(report),
        run: command(config, held.get(RUN.name))?,
        differential: only_the_new(config, held.get(DIFFERENTIAL.name))?,
    })
}

fn shape(config: &Config) -> Error {
    Error(format!(
        "{}: \"{SECTION}\" is a list of {{\"name\", \"report\"}} entries, each its own gate, \
         and each with an optional \"run\" and \"differential\"",
        config.file.display()
    ))
}

fn command(config: &Config, held: Option<&Value>) -> Result<Option<String>, Error> {
    match held {
        None => Ok(None),
        Some(Value::String(command)) => Ok(Some(command.clone())),
        Some(_) => Err(config.malformed(SECTION, RUN.name, "a command that writes the report")),
    }
}

fn only_the_new(config: &Config, held: Option<&Value>) -> Result<bool, Error> {
    match held {
        None => Ok(false),
        Some(Value::Bool(only_the_new)) => Ok(*only_the_new),
        Some(_) => Err(config.malformed(SECTION, DIFFERENTIAL.name, "true or false")),
    }
}

/// What this gate reads before it judges: the report, and the lines the window changed. With
/// `run` klin writes the report over this tree first. Without it klin reads the report as it
/// finds it, and refuses one that predates the change. Spec 8.3.
fn read(
    config: &Config,
    entry: &Entry,
    at: &Context,
    out: &mut Sink,
) -> Result<(Placed, Hunks), Error> {
    let root = config.root();
    if let Some(command) = &entry.run {
        wrote(root, command, &entry.report)?;
    }
    let data = sarif(&entry.report)?;
    let changed = Hunks::read(root, &base::commit(root, at, out)?, None)?;
    if entry.run.is_none() {
        fresh(&entry.report, root, &changed)?;
    }
    Ok((placed(&data, root), changed))
}

/// The report the command writes over the tree klin is about to judge. The old report goes
/// first, so the only file at that path is the one the tool wrote over this tree, and the
/// command's exit status is not judged, because a linter exits non-zero when it finds
/// something. Spec 8.3.
fn wrote(root: &Path, command: &str, report: &Path) -> Result<(), Error> {
    let _ = std::fs::remove_file(report);
    match Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(root)
        .output()
    {
        Err(why) => Err(Error(format!("{command} could not run: {why}"))),
        Ok(_) if report.is_file() => Ok(()),
        Ok(done) => Err(Error(format!(
            "{command} wrote no report at {} — klin deletes the report before it runs the \
             command, so this gate has nothing to read:\n{}",
            report.display(),
            output(&done)
        ))),
    }
}

fn output(done: &Output) -> String {
    String::from_utf8_lossy(&done.stdout).into_owned() + &String::from_utf8_lossy(&done.stderr)
}

/// The report as SARIF, or the tool error that says why it is not. Spec 8.3, 14.
fn sarif(report: &Path) -> Result<Value, Error> {
    let bytes = std::fs::read(report).map_err(|why| Error::unreadable(report, why))?;
    let data: Value =
        serde_json::from_slice(&bytes).map_err(|why| not_sarif(report, &why.to_string()))?;
    match data.get("runs").is_some_and(Value::is_array) {
        true => Ok(data),
        false => Err(not_sarif(report, "it names no \"runs\" list")),
    }
}

fn not_sarif(report: &Path, why: &str) -> Error {
    Error(format!("{} is not SARIF: {why}", report.display()))
}

/// A report written before the window it must describe cannot describe it, so klin refuses it.
/// Only an entry with no `run` reads a report klin did not write. Spec 8.3.
fn fresh(report: &Path, root: &Path, changed: &Hunks) -> Result<(), Error> {
    let Some(written) = modified(report) else {
        return Ok(());
    };
    for path in changed.paths() {
        if modified(&root.join(path)).is_some_and(|at| at > written) {
            return Err(Error(format!(
                "{} is older than {path}, which the window changed — a report that predates the \
                 change cannot describe it, so write the report after the change, or give the \
                 entry a \"run\" command and let klin write it",
                report.display()
            )));
        }
    }
    Ok(())
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|held| held.modified())
        .ok()
}

/// One result klin placed: the repository path, the line the result starts on, and the
/// `rule id: message text` that identifies the site wherever its lines move to. Spec 8.3.
struct Site {
    file: String,
    line: u64,
    text: String,
}

/// What one report says: the results klin placed in the tree, and one NOTE per result it could
/// not place.
#[derive(Default)]
struct Placed {
    placed: Vec<Site>,
    notes: Vec<(String, String)>,
}

fn placed(data: &Value, root: &Path) -> Placed {
    let mut found = Placed::default();
    for run in listed(data, "runs") {
        let bases = run
            .get("originalUriBaseIds")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for result in listed(run, "results") {
            found.add(root, &bases, result);
        }
    }
    found
}

impl Placed {
    fn add(&mut self, root: &Path, bases: &Map<String, Value>, result: &Value) {
        let named = format!("{}: {}", rule(result), message(result));
        let Some(location) = location(result) else {
            self.notes.push((
                String::new(),
                format!(
                    "a result of {named} has no physical location, so klin cannot say which \
                     line it sits on, and it is not judged"
                ),
            ));
            return;
        };
        let Some(file) = under(root, bases, &location) else {
            self.notes.push((
                location.uri.clone(),
                format!(
                    "{} is a location klin could not place under {}, so the result of {named} \
                     there is not judged",
                    location.uri,
                    root.display()
                ),
            ));
            return;
        };
        self.placed.push(Site {
            file,
            line: location.line,
            text: named,
        });
    }
}

fn rule(result: &Value) -> String {
    let named = result
        .get("ruleId")
        .and_then(Value::as_str)
        .or_else(|| result.pointer("/rule/id").and_then(Value::as_str));
    named
        .unwrap_or("a rule the report does not name")
        .to_string()
}

fn message(result: &Value) -> String {
    result
        .pointer("/message/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// What one location says before klin places it: the uri as the scanner wrote it, the base id
/// it is relative to, and the line it starts on.
struct Location {
    uri: String,
    base: Option<String>,
    line: u64,
}

/// What the first location of a result says. A location with no region starts at line 1, which
/// is the line a result about a whole file is judged on.
fn location(result: &Value) -> Option<Location> {
    let physical = listed(result, "locations")
        .first()?
        .get("physicalLocation")?;
    let at = physical.get("artifactLocation")?;
    Some(Location {
        uri: at.get("uri")?.as_str()?.to_string(),
        base: at
            .get("uriBaseId")
            .and_then(Value::as_str)
            .map(str::to_string),
        line: physical
            .pointer("/region/startLine")
            .and_then(Value::as_u64)
            .unwrap_or(1),
    })
}

/// The repository-relative path a location names. Four forms reach klin, because every scanner
/// writes a location its own way: a path relative to the repository, an absolute path, a
/// `file://` URI, and a path under an entry of `originalUriBaseIds`. klin decodes each, strips
/// the tree's own directory off an absolute one, and then resolves the `.` and `..` segments of
/// what is left. A location that lands outside the tree is placed nowhere, and the caller
/// leaves a NOTE, because a path klin cannot resolve says nothing about which lines the window
/// changed. Spec 8.3.
fn under(root: &Path, bases: &Map<String, Value>, location: &Location) -> Option<String> {
    let text = joined(
        &prefix(bases, location.base.as_deref(), 0),
        &decoded(&location.uri),
    );
    let path = PathBuf::from(&text);
    let relative = match path.is_absolute() {
        true => inside(root, &path)?,
        false => text,
    };
    within(&relative)
}

/// The path an `originalUriBaseIds` entry stands for, through the id that entry may name in
/// turn, and nothing when the report names no base for this location.
fn prefix(bases: &Map<String, Value>, id: Option<&str>, depth: usize) -> String {
    let Some(entry) = id.and_then(|id| bases.get(id)).filter(|_| depth < DEPTH) else {
        return String::new();
    };
    let above = prefix(
        bases,
        entry.get("uriBaseId").and_then(Value::as_str),
        depth + 1,
    );
    let held = entry.get("uri").and_then(Value::as_str).unwrap_or_default();
    joined(&above, &decoded(held))
}

fn joined(above: &str, text: &str) -> String {
    match above.is_empty() {
        true => text.to_string(),
        false => format!(
            "{}/{}",
            above.trim_end_matches('/'),
            text.trim_start_matches('/')
        ),
    }
}

/// Where an absolute path sits under the tree klin judges, through the real directories both
/// name, so a tree reached by a symlink still places.
fn inside(root: &Path, path: &Path) -> Option<String> {
    let rest = match path.strip_prefix(root) {
        Ok(rest) => rest.to_path_buf(),
        Err(_) => real(path).strip_prefix(real(root)).ok()?.to_path_buf(),
    };
    Some(rest.to_string_lossy().into_owned())
}

fn real(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// A relative path with its `.` and `..` segments resolved, and nothing when it climbs above
/// the tree or names the tree itself, because neither is a file the window could have changed.
fn within(text: &str) -> Option<String> {
    let mut segments: Vec<&str> = Vec::new();
    for segment in text.split('/') {
        match segment {
            "" | "." => (),
            ".." => {
                segments.pop()?;
            }
            named => segments.push(named),
        }
    }
    (!segments.is_empty()).then(|| segments.join("/"))
}

/// A uri as a path: without its `file://` scheme, and with every percent escape decoded, which
/// is the form SARIF writes a uri in.
fn decoded(uri: &str) -> String {
    let text = uri.strip_prefix("file://").unwrap_or(uri).as_bytes();
    let mut out = Vec::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        match escaped(text, at) {
            Some(byte) => {
                out.push(byte);
                at += 3;
            }
            None => {
                out.push(text[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn escaped(text: &[u8], at: usize) -> Option<u8> {
    if text[at] != b'%' || at + 3 > text.len() {
        return None;
    }
    let digit = |byte: u8| char::from(byte).to_digit(16);
    Some((digit(text[at + 1])? * 16 + digit(text[at + 2])?) as u8)
}

/// How many results one site holds, and the first line one of them sits on.
struct Tally {
    line: u64,
    count: u64,
}

/// What this gate made of the report: one finding per site, how many results those findings
/// hold, and how many results it held because the window did not change the line they sit on.
struct Judged {
    findings: Vec<Finding>,
    judged: u64,
    held: u64,
}

/// A site is one file and one `rule id: message text`, so several results of one rule in one
/// file are one finding with a count. With `differential` the scanner already reports only what
/// is new, so every result is judged. Spec 8.3.
fn judge(placed: Vec<Site>, changed: &Hunks, differential: bool) -> Judged {
    let mut seen: BTreeMap<(String, String), Tally> = BTreeMap::new();
    let mut held = 0;
    for site in placed {
        if !differential && !changed.holds(&site.file, site.line) {
            held += 1;
            continue;
        }
        seen.entry((site.file, site.text))
            .and_modify(|tally| {
                tally.line = tally.line.min(site.line);
                tally.count += 1;
            })
            .or_insert(Tally {
                line: site.line,
                count: 1,
            });
    }
    Judged {
        judged: seen.values().map(|tally| tally.count).sum(),
        findings: collected(seen),
        held,
    }
}

fn collected(seen: BTreeMap<(String, String), Tally>) -> Vec<Finding> {
    let mut out: Vec<Finding> = seen
        .into_iter()
        .map(|((file, text), tally)| {
            let mut values = Values::new();
            values.insert(COUNT.into(), tally.count.into());
            Finding {
                file,
                line: tally.line,
                text,
                values,
                body: None,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

/// The `OK:` line, which says what the gate judged. A `differential` entry judges every result
/// the scanner wrote, wherever it sits, so that line does not name the changed lines. Spec 8.3,
/// 8.6.
fn said(judged: &Judged, differential: bool) -> String {
    match differential {
        true => format!(
            "{} result(s) judged, which is every result the scanner reported",
            judged.judged
        ),
        false => format!(
            "{} result(s) on lines this window changed, {} held on lines it did not",
            judged.judged, judged.held
        ),
    }
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: METRICS,
        unit: "result(s)",
        condition: "the scanner reports on a line this window changed",
        fix_advice: REMEDY,
        ceiling: None,
        format_metrics: show,
    }
}

fn show(values: &Values) -> String {
    format!(
        "{COUNT} {}",
        values
            .get(COUNT)
            .and_then(Value::as_u64)
            .unwrap_or_default()
    )
}

fn listed<'a>(held: &'a Value, key: &str) -> &'a [Value] {
    held.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}
