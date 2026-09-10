use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

use serde_json::{Map, Value};

use crate::base;
use crate::config::{Config, Error, Flags};
use crate::hunks::Hunks;
use crate::ratchet::{self, Evaluator, Finding, Values};

const SECTION: &str = "sarif";
const COUNT: &str = "count";
const METRICS: &[&str] = &[COUNT];
/// How many `originalUriBaseIds` entries one location may be resolved through, so a report
/// whose ids point at each other resolves rather than recurring forever.
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
    let config = Config::load(args.config.as_deref(), start)?;
    let mut worst = 0;
    for (name, entry) in named(&config)? {
        worst = worst.max(evaluate(&flags(args, &name, entry), start, out)?);
    }
    Ok(worst)
}

/// The name and the value of each entry the section holds. A section that is one entry rather
/// than a list runs under the section's own name.
fn named(config: &Config) -> Result<Vec<(String, Value)>, Error> {
    let section = config.section(SECTION)?;
    let Some(entries) = section.as_array() else {
        return Ok(vec![(SECTION.to_string(), section.clone())]);
    };
    entries
        .iter()
        .map(|entry| {
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| config.missing(SECTION, "name"))?;
            Ok((name.to_string(), entry.clone()))
        })
        .collect()
}

fn flags(args: &Args, name: &str, entry: Value) -> Flags {
    Flags {
        config: args.config.clone(),
        gate: name.to_string(),
        prior: None,
        base: None,
        quiet: args.quiet,
        strict: args.strict,
        hook: false,
        only: None,
        records: None,
        with: Some((SECTION.to_string(), entry)),
    }
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    evaluate(flags, start, out)
}

fn evaluate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::open(flags, start)?;
    let entry = entry(&config)?;
    let (read, changed) = reading(&config, &entry, flags, out)?;
    let (findings, judged, held) = judged(read.sites, &changed, entry.differential);
    let accepted = ratchet::accepted(&config, &flags.gate, METRICS)?;
    let ok = format!(
        "OK: {judged} result(s) on lines this window changed, {held} held on lines it did not"
    );
    let code = evaluator().evaluate(findings, Vec::new(), accepted, flags, &ok, out);
    noted(&read.notes, flags, out);
    Ok(code)
}

fn entry(config: &Config) -> Result<Entry, Error> {
    let held = config
        .section(SECTION)?
        .as_object()
        .ok_or_else(|| shape(config))?;
    let report = held
        .get("report")
        .and_then(Value::as_str)
        .ok_or_else(|| config.missing(SECTION, "report"))?;
    Ok(Entry {
        report: config.path(report),
        run: command(config, held.get("run"))?,
        differential: only_the_new(config, held.get("differential"))?,
    })
}

fn shape(config: &Config) -> Error {
    Error(format!(
        "{}: \"{SECTION}\" is a list of {{\"name\", \"report\"}} entries, each with an optional \
         \"run\" and \"differential\"",
        config.file.display()
    ))
}

fn command(config: &Config, held: Option<&Value>) -> Result<Option<String>, Error> {
    match held {
        None => Ok(None),
        Some(Value::String(command)) => Ok(Some(command.clone())),
        Some(_) => Err(config.malformed(SECTION, "run", "a command that writes the report")),
    }
}

fn only_the_new(config: &Config, held: Option<&Value>) -> Result<bool, Error> {
    match held {
        None => Ok(false),
        Some(Value::Bool(only_the_new)) => Ok(*only_the_new),
        Some(_) => Err(config.malformed(SECTION, "differential", "true or false")),
    }
}

/// What this gate reads before it judges: the report, and the lines the window changed. With
/// `run` klin writes the report over this tree first. Without it klin reads the report as it
/// finds it, and refuses one that predates the change. Spec 8.3.
fn reading(
    config: &Config,
    entry: &Entry,
    flags: &Flags,
    out: &mut String,
) -> Result<(Read, Hunks), Error> {
    let root = config.root();
    if let Some(command) = &entry.run {
        wrote(root, command, &entry.report)?;
    }
    let data = read(&entry.report)?;
    let changed = Hunks::read(root, &commit(config, flags, out)?, None)?;
    if entry.run.is_none() {
        fresh(&entry.report, root, &changed)?;
    }
    Ok((sites(&data, root), changed))
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
            said(&done)
        ))),
    }
}

fn said(done: &Output) -> String {
    String::from_utf8_lossy(&done.stdout).into_owned() + &String::from_utf8_lossy(&done.stderr)
}

/// The report as SARIF, or the tool error that says why it is not. Spec 8.3, 14.
fn read(report: &Path) -> Result<Value, Error> {
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

/// What one location says before klin places it: the uri as the scanner wrote it, the base id
/// it is relative to, and the line it starts on.
struct Location {
    uri: String,
    base: Option<String>,
    line: u64,
}

/// What one report says: the results klin placed, and one NOTE per result it could not.
#[derive(Default)]
struct Read {
    sites: Vec<Site>,
    notes: Vec<(String, String)>,
}

fn sites(data: &Value, root: &Path) -> Read {
    let mut read = Read::default();
    for run in listed(data, "runs") {
        let bases = run
            .get("originalUriBaseIds")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for result in listed(run, "results") {
            read.add(root, &bases, result);
        }
    }
    read
}

impl Read {
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
        let Some(file) = placed(root, bases, &location) else {
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
        self.sites.push(Site {
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
/// `file://` URI, and a path under an entry of `originalUriBaseIds`. klin decodes each, then
/// places an absolute one under the tree it judges. A location it cannot place there is a
/// NOTE, because a path klin cannot resolve says nothing about which lines changed. Spec 8.3.
fn placed(root: &Path, bases: &Map<String, Value>, location: &Location) -> Option<String> {
    let text = joined(
        &prefix(bases, location.base.as_deref(), 0),
        &decoded(&location.uri),
    );
    let path = PathBuf::from(&text);
    if !path.is_absolute() {
        let named = text.trim_start_matches("./").to_string();
        return (!named.is_empty()).then_some(named);
    }
    inside(root, &path)
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
    let named = rest.to_string_lossy().into_owned();
    (!named.is_empty()).then_some(named)
}

fn real(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
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

/// The findings this gate judges, how many results they hold, and how many results it held
/// because the window did not change the line they sit on. A site is one file and one
/// `rule id: message text`, so several results of one rule in one file are one finding with a
/// count. With `differential` the scanner already reports only what is new, so every result is
/// judged. Spec 8.3.
fn judged(sites: Vec<Site>, changed: &Hunks, differential: bool) -> (Vec<Finding>, u64, u64) {
    let mut seen: BTreeMap<(String, String), (u64, u64)> = BTreeMap::new();
    let mut held = 0;
    for site in sites {
        if !differential && !changed.holds(&site.file, site.line) {
            held += 1;
            continue;
        }
        let at = seen.entry((site.file, site.text)).or_insert((site.line, 0));
        at.0 = at.0.min(site.line);
        at.1 += 1;
    }
    let judged = seen.values().map(|(_, count)| count).sum();
    (collected(seen), judged, held)
}

fn collected(seen: BTreeMap<(String, String), (u64, u64)>) -> Vec<Finding> {
    let mut out: Vec<Finding> = seen
        .into_iter()
        .map(|((file, text), (line, count))| {
            let mut values = Values::new();
            values.insert(COUNT.into(), count.into());
            Finding {
                file,
                line,
                text,
                values,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: METRICS,
        unit: "result(s)",
        condition: "the scanner reports on a line this window changed",
        fix_advice: REMEDY,
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

fn noted(notes: &[(String, String)], flags: &Flags, out: &mut String) {
    for (_, why) in notes {
        let _ = writeln!(out, "NOTE: {why}");
    }
    flags.record(|records| {
        for (at, why) in notes {
            let mut record = Map::new();
            record.insert("outcome".into(), "note".into());
            record.insert("file".into(), at.clone().into());
            record.insert("text".into(), why.clone().into());
            records.notes.push(Value::Object(record));
        }
    });
}

fn commit(config: &Config, flags: &Flags, out: &mut String) -> Result<String, Error> {
    match &flags.base {
        Some(commit) => Ok(commit.clone()),
        None => Ok(base::announced(config.root(), flags, out)?.before),
    }
}

fn listed<'a>(held: &'a Value, key: &str) -> &'a [Value] {
    held.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}
