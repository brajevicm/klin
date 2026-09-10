use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use serde_json::{Map, Value};

use crate::base;
use crate::changed::git;
use crate::config::{Config, Error, Flags};
use crate::coverage::{self, Coverage};
use crate::files;
use crate::ratchet::{self, Evaluator, Finding, Values};
use crate::survey::{ROOT, TEST_DIRS, TEST_PREFIXES, TEST_SUFFIXES};

const SECTION: &str = "inventory";
const LABEL: &str = "test file";
const MISSING: &str = "missing";
const REMEDY: &str = "Restore the test, or record in the accepted list why it went. A deleted \
    test is the cheapest route to green, so only a person accepts one, in a reviewed commit.";

/// The one affix table of spec 8.2, which the survey reads to find a test root and this gate
/// reads to name a test file's subject. Printed with the NOTE.
const RULE: &str = "the affix table: a test_ or spec_ prefix, a _test, _spec, .test or .spec \
    suffix, a Test or Tests suffix on the basename, and a tests/, test/, spec/ or __tests__/ \
    directory segment";

struct Entry {
    path: String,
    pattern: Option<String>,
}

/// One test file the base holds: whether the working tree still has it, and the subject that
/// went with it, if the same window deleted one.
struct Site {
    path: String,
    gone: bool,
    subject: Option<String>,
}

pub fn gate(flags: &Flags, start: &Path, out: &mut String) -> Result<u8, Error> {
    let config = Config::open(flags, start)?;
    config.say(flags, SECTION, out);
    let entries = entries(&config)?;
    let commit = base::commit(config.root(), flags, out)?;
    let listed = at_the_base(config.root(), &commit)?;
    let sites = sites(&entries, &listed, config.root());
    let (judged, mut paired): (Vec<Site>, Vec<Site>) =
        sites.into_iter().partition(|site| site.subject.is_none());
    if let Some(only) = flags.only.as_deref() {
        paired.retain(|site| only.contains(&site.path));
    }
    let now: Vec<Finding> = judged
        .iter()
        .map(|site| finding(&site.path, site.gone))
        .collect();
    let before: Vec<Finding> = judged
        .iter()
        .map(|site| finding(&site.path, false))
        .collect();
    let held = ratchet::scoped(&now, flags.only.as_deref());
    let accepted = ratchet::accepted(&config, &flags.gate, evaluator().metrics)?;
    let said = covered(&judged, &paired, flags).said(flags);
    let code = evaluator().evaluate(
        now,
        before,
        accepted,
        flags,
        &format!("OK: {held} test file(s) the base holds, all still there{said}"),
        out,
    );
    noted(&paired, flags, out);
    Ok(code)
}

/// What this gate discovered: every test file the base holds under its entries. A deleted test
/// whose subject went with it is found and not measured, because it is a NOTE and not a site
/// the gate judges. Spec 8.6.
fn covered(judged: &[Site], paired: &[Site], flags: &Flags) -> Coverage {
    let only = flags.only.as_deref();
    let paths =
        |sites: &[Site]| -> Vec<String> { sites.iter().map(|site| site.path.clone()).collect() };
    let measured = coverage::scoped(&paths(judged), only);
    Coverage {
        found: measured + coverage::scoped(&paths(paired), only),
        measured,
        excluded: 0,
        unreadable: 0,
    }
}

fn evaluator() -> Evaluator<'static> {
    Evaluator {
        metrics: &[MISSING],
        unit: "test file(s)",
        condition: "where the base holds a test file the working tree no longer has",
        fix_advice: REMEDY,
        ceiling: None,
        format_metrics: show,
    }
}

fn show(values: &Values) -> String {
    format!(
        "{MISSING} {}",
        values.get(MISSING).and_then(Value::as_u64).unwrap_or(0)
    )
}

fn finding(path: &str, gone: bool) -> Finding {
    let mut values = Values::new();
    values.insert(MISSING.into(), u64::from(gone).into());
    Finding {
        file: path.to_string(),
        line: 0,
        text: LABEL.to_string(),
        values,
        body: None,
    }
}

/// A deleted test whose subject went in the same window, which is a NOTE and not a finding.
/// Spec 8.2, 16.4.
fn noted(paired: &[Site], flags: &Flags, out: &mut String) {
    if paired.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "NOTE: {} deleted test file(s) whose subject went in the same window:",
        paired.len()
    );
    for site in paired {
        let subject = site.subject.as_deref().unwrap_or("");
        let _ = writeln!(out, "  {}  its subject {subject} went too", site.path);
    }
    let _ = writeln!(out, "  each subject matched by {RULE}");
    flags.record(|records| {
        for site in paired {
            let mut record = Map::new();
            record.insert("outcome".into(), "note".into());
            record.insert("file".into(), site.path.clone().into());
            record.insert(
                "text".into(),
                format!(
                    "the test file {} went with its subject {}, matched by {RULE}",
                    site.path,
                    site.subject.as_deref().unwrap_or("")
                )
                .into(),
            );
            records.notes.push(Value::Object(record));
        }
    });
}

/// Every test file the base holds under the entries, each with what the working tree says
/// about it. This is the one measure of `after` that reads `before`. Spec 16.4.
fn sites(entries: &[Entry], listed: &BTreeSet<String>, root: &Path) -> Vec<Site> {
    let mut wanted: Vec<&String> = listed
        .iter()
        .filter(|path| entries.iter().any(|entry| entry.holds(path)))
        .collect();
    wanted.sort();
    wanted
        .into_iter()
        .map(|path| {
            let gone = !root.join(path).is_file();
            Site {
                path: path.clone(),
                gone,
                subject: gone.then(|| subject(path, listed, root)).flatten(),
            }
        })
        .collect()
}

impl Entry {
    fn holds(&self, path: &str) -> bool {
        if !under_or_at(path, &self.path) {
            return false;
        }
        let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
        match &self.pattern {
            Some(glob) => files::glob_matches(glob.as_bytes(), name.as_bytes()),
            None => true,
        }
    }
}

fn under_or_at(path: &str, root: &str) -> bool {
    root == ROOT || path == root || path.starts_with(&format!("{root}/"))
}

/// The file the base holds at the test's path with the affixes stripped, gone from the working
/// tree too. `None` when no candidate resolves, and then the test is judged normally.
fn subject(path: &str, listed: &BTreeSet<String>, root: &Path) -> Option<String> {
    candidates(path)
        .into_iter()
        .find(|candidate| listed.contains(candidate) && !root.join(candidate).is_file())
}

/// Every path the affix table names as a subject: the directory with one test segment removed,
/// the basename with one affix removed, and both together.
fn candidates(path: &str) -> Vec<String> {
    let (directory, name) = match path.rsplit_once('/') {
        Some((directory, name)) => (directory.to_string(), name.to_string()),
        None => (String::new(), path.to_string()),
    };
    let mut directories = vec![directory.clone()];
    directories.extend(without_a_test_dir(&directory));
    let mut names = vec![name.clone()];
    names.extend(without_an_affix(&name));
    let mut out = Vec::new();
    for directory in &directories {
        for name in &names {
            let candidate = match directory.is_empty() {
                true => name.clone(),
                false => format!("{directory}/{name}"),
            };
            if candidate != path {
                out.push(candidate);
            }
        }
    }
    out
}

fn without_a_test_dir(directory: &str) -> Vec<String> {
    let segments: Vec<&str> = match directory.is_empty() {
        true => Vec::new(),
        false => directory.split('/').collect(),
    };
    segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| TEST_DIRS.contains(*segment))
        .map(|(at, _)| {
            let mut kept = segments.clone();
            kept.remove(at);
            kept.join("/")
        })
        .collect()
}

fn without_an_affix(name: &str) -> Vec<String> {
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{extension}")),
        None => (name, String::new()),
    };
    let prefixes = TEST_PREFIXES
        .iter()
        .filter_map(|prefix| stem.strip_prefix(prefix));
    let suffixes = TEST_SUFFIXES
        .iter()
        .filter_map(|suffix| stem.strip_suffix(suffix));
    prefixes
        .chain(suffixes)
        .filter(|stripped| !stripped.is_empty())
        .map(|stripped| format!("{stripped}{extension}"))
        .collect()
}

/// The base commit the runner chose, or the one this gate chooses for itself.
/// The base tree's file list, read out of git so the file level needs no base worktree. A
/// listing git refuses is an error: an empty base holds no test file and reports green.
fn at_the_base(root: &Path, commit: &str) -> Result<BTreeSet<String>, Error> {
    let listed =
        git(root, &["ls-tree", "-r", "--name-only", commit, "--", "."]).ok_or_else(|| {
            Error(format!(
                "the base commit {} could not be listed under {} — fetch history, or give CI \
                 the full clone",
                &commit[..7.min(commit.len())],
                root.display()
            ))
        })?;
    Ok(listed
        .lines()
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect())
}

fn entries(config: &Config) -> Result<Vec<Entry>, Error> {
    let Some(listed) = config.section(SECTION)?.as_array() else {
        return Err(Error(format!(
            "{}: \"{SECTION}\" must be a list of {{\"name\", \"path\"}} entries",
            config.file.display()
        )));
    };
    listed.iter().map(|item| entry(config, item)).collect()
}

fn entry(config: &Config, item: &Value) -> Result<Entry, Error> {
    let values = item
        .as_object()
        .ok_or_else(|| config.malformed(SECTION, "path", "an object"))?;
    for key in ["name", "path"] {
        if !values.get(key).is_some_and(Value::is_string) {
            return Err(config.missing(SECTION, key));
        }
    }
    let pattern = match values.get("pattern") {
        None => None,
        Some(Value::String(glob)) => Some(glob.clone()),
        Some(_) => return Err(config.malformed(SECTION, "pattern", "a glob on the basename")),
    };
    Ok(Entry {
        path: values["path"].as_str().unwrap_or_default().to_string(),
        pattern,
    })
}
