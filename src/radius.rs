use std::fmt::Write;
use std::path::Path;

use serde_json::{Map, Value};

use crate::cache;
use crate::config::{self, Config};
use crate::error::Error;
use crate::key::Section;
use crate::stamp;
use crate::state;

/// The section that pins how wide this project's usual change is. Absent, #92 derives it, and
/// with neither the report on a prompt prints nothing. ADR 0014.
const SECTION: &str = config::RADIUS.name;
const LINES: &str = "lines";
const DIRECTORIES: &str = "directories";
/// Every invocation pins these, because a line count moves with the algorithm and with rename
/// detection, and an inherited git setting would measure the same turn twice over. ADR 0014.
const SETTINGS: &[&str] = &[
    "-c",
    "core.quotepath=false",
    "-c",
    "diff.relative=false",
    "-c",
    "diff.renameLimit=32767",
];
const COUNTED: &[&str] = &["--numstat", "--diff-algorithm=histogram"];

/// The sample the values are derived from, and the floor under which there is none. A commit
/// count beats a day window, because a quiet month must not shrink the sample. ADR 0014.
const SAMPLE: usize = 200;
const FLOOR: usize = 50;
const PERCENTILE: f64 = 0.9;

/// What the project usually does, against which a turn reads as wide. Either one exceeded is
/// enough, because a turn can spread by line count or by directory alone.
struct Usual {
    lines: u64,
    directories: u64,
    /// One line per value, saying whether the config pinned it or history derived it, which is
    /// what `--report` prints under the facts. Spec 5.2, 11.1.
    said: Vec<String>,
}

/// What the last commits of this project usually changed, over the sample they were read from.
pub struct Derived {
    pub lines: u64,
    pub directories: u64,
    pub commits: usize,
}

/// How far the turn spread, from three diffs of the stamp against a tree of the working
/// directory. The raw total is the line count, and the two gaps are what the raw total does
/// not explain by itself.
struct Spread {
    lines: u64,
    files: usize,
    directories: Vec<String>,
    formatting: u64,
    moved: u64,
    largest: Option<(String, u64)>,
}

/// The report on a prompt: the facts, and only when the turn passed a value. It asks for
/// nothing, and anything it cannot measure it says nothing about. ADR 0014. The facts measured,
/// for the journal's prompt line (spec 11.4), and `None` for a turn nothing here could measure.
/// Takes the config the caller already loaded for the same event.
pub fn spread(
    config: &Config,
    root: &Path,
    at: &Path,
    opened: Option<&str>,
    tree: Option<&str>,
    out: &mut String,
) -> Option<Value> {
    let (Some(opened), Some(tree)) = (opened, tree) else {
        return None;
    };
    let Ok(usual) = usual(config, root, Some(at)) else {
        return None;
    };
    let spread = measured(root, opened, tree)?;
    let wide = spread.lines > usual.lines || spread.directories.len() as u64 > usual.directories;
    if wide {
        let _ = writeln!(
            out,
            "klin: the previous turn was wider than this project's usual change."
        );
        describe(&spread, &usual, out);
    }
    Some(serde_json::json!({
        "lines": spread.lines,
        "formatting": spread.formatting,
        "moved": spread.moved,
        "directories": spread.directories.len(),
        "wide": wide,
    }))
}

/// `klin radius --report`, which a person runs. It moves nothing, it raises no counter, and
/// unlike the hook it names what it cannot read rather than staying quiet. ADR 0014.
pub fn asked(root: &Path, sections: &[Section], out: &mut String) -> Result<u8, Error> {
    let at = state::ready(root).map_err(Error)?;
    let config = Config::load(None, root, sections)?;
    let usual = usual(&config, root, Some(&at))?;
    let opened = stamp::mark(root, &at).ok_or_else(|| {
        Error("no prompt mark is readable, so there is no turn to measure".to_string())
    })?;
    let tree = stamp::tree(root, &at)
        .ok_or_else(|| Error("git could not read this working tree".to_string()))?;
    let spread = measured(root, &opened, &tree)
        .ok_or_else(|| Error("git could not measure this turn".to_string()))?;
    let _ = writeln!(out, "klin: this turn, measured against the prompt mark.");
    describe(&spread, &usual, out);
    for line in &usual.said {
        let _ = writeln!(out, "{line}");
    }
    Ok(0)
}

/// One derived value as a run prints it: the section, the key, the number and the rule that
/// produced it. `init` prints the same line for what it pins. Spec 11.1.
pub fn derived_line(key: &str, value: u64, commits: usize) -> String {
    format!(
        "derived: {SECTION} {key} {value}, the 90th percentile of the last {commits} non-merge \
         commits"
    )
}

/// A value the config pins, which prints beside the derived ones so a person reads one list.
/// Spec 5.2.
fn pinned_line(key: &str, value: u64) -> String {
    format!("pinned: {SECTION} {key} {value}")
}

fn describe(spread: &Spread, usual: &Usual, out: &mut String) {
    let _ = writeln!(
        out,
        "  {} lines in {}, under {}.",
        spread.lines,
        count(spread.files as u64, "file"),
        under(&spread.directories)
    );
    let _ = writeln!(
        out,
        "  {} of those lines are formatting only, and {} are code that moved.",
        spread.formatting, spread.moved
    );
    if let Some((path, lines)) = &spread.largest {
        let _ = writeln!(out, "  Largest single file: {path}, {lines} lines.");
    }
    let _ = writeln!(
        out,
        "  Commits here usually change about {} lines under {}.",
        usual.lines,
        count(usual.directories, "directory")
    );
}

/// The directories, named up to a point. The report goes into the model's context on every
/// prompt, and a repository-wide reformat would otherwise list every directory it touched.
fn under(directories: &[String]) -> String {
    let named = directories
        .iter()
        .take(NAMED)
        .cloned()
        .collect::<Vec<String>>();
    match directories.len().checked_sub(NAMED) {
        None | Some(0) if named.is_empty() => "no directory".to_string(),
        None | Some(0) => named.join(", "),
        Some(rest) => format!("{}, and {} more", named.join(", "), rest),
    }
}

const NAMED: usize = 6;

fn count(many: u64, name: &str) -> String {
    match (many, name) {
        (1, name) => format!("1 {name}"),
        (many, "directory") => format!("{many} directories"),
        (many, name) => format!("{many} {name}s"),
    }
}

/// The values a turn is measured against: what the config pins, and history for a key it does
/// not pin. The hook drops the error and prints nothing, and `--report` raises it. #92. Takes
/// the config already loaded, so a caller with one loaded for another reason reads klin.json
/// once and not twice.
fn usual(config: &Config, root: &Path, at: Option<&Path>) -> Result<Usual, Error> {
    let (lines, directories) = numbers(config)?;
    if let (Some(lines), Some(directories)) = (lines, directories) {
        return Ok(Usual {
            lines,
            directories,
            said: vec![
                pinned_line(LINES, lines),
                pinned_line(DIRECTORIES, directories),
            ],
        });
    }
    let history = history(root, at).map_err(|why| unpinned(config, lines, directories, &why))?;
    let said = |key, pinned: Option<u64>, found| match pinned {
        Some(value) => pinned_line(key, value),
        None => derived_line(key, found, history.commits),
    };
    Ok(Usual {
        lines: lines.unwrap_or(history.lines),
        directories: directories.unwrap_or(history.directories),
        said: vec![
            said(LINES, lines, history.lines),
            said(DIRECTORIES, directories, history.directories),
        ],
    })
}

/// The two numbers the config pins, and `None` for each it leaves to history. A section may pin
/// one key and leave the other, and a section that is not an object pins neither and says so
/// rather than deriving in silence. Spec 5.2.
fn numbers(config: &Config) -> Result<(Option<u64>, Option<u64>), Error> {
    let section = match config.pinned(SECTION) {
        None => return Ok((None, None)),
        Some(Value::Object(section)) => section,
        Some(_) => {
            return Err(config.malformed(SECTION, LINES, "under a section that is an object"));
        }
    };
    let number = |key: &str| match section.get(key) {
        None => Ok(None),
        Some(found) => found
            .as_u64()
            .map(Some)
            .ok_or_else(|| config.malformed(SECTION, key, "a whole number")),
    };
    Ok((number(LINES)?, number(DIRECTORIES)?))
}

/// What `--report` says when the config pins a value short and history cannot supply it either.
fn unpinned(config: &Config, lines: Option<u64>, directories: Option<u64>, why: &str) -> Error {
    let key = match (lines, directories) {
        (None, None) => "section".to_string(),
        (None, _) => format!("\"{LINES}\""),
        _ => format!("\"{DIRECTORIES}\""),
    };
    Error(format!(
        "{} pins no \"{SECTION}\" {key}, and {why}",
        config.file.display()
    ))
}

/// What history says a commit here usually changes, cached under the derivation commit, or the
/// reason it says nothing. Below the floor a percentile would fire on almost every turn, so
/// there is no derived section and the caller says why. ADR 0014, spec 5.4.
pub fn history(root: &Path, at: Option<&Path>) -> Result<Derived, String> {
    let commit = stamp::derivation(root, at)
        .ok_or_else(|| "no commit to derive this project's usual change from".to_string())?;
    if let Some(held) = at
        .and_then(|at| cache::read(at, &commit, SECTION))
        .and_then(|cached| held(&cached))
    {
        return Ok(held);
    }
    let sample = sampled(root, &commit)
        .ok_or_else(|| "git could not read this project's history".to_string())?;
    if sample.len() < FLOOR {
        return Err(format!(
            "{} non-merge commit(s) reach {}, fewer than the {FLOOR} a percentile needs",
            sample.len(),
            &commit[..commit.len().min(12)]
        ));
    }
    let found = Derived {
        lines: percentile(sample.iter().map(|(lines, _)| *lines).collect()),
        directories: percentile(sample.iter().map(|(_, under)| *under).collect()),
        commits: sample.len(),
    };
    if let Some(at) = at {
        cache::write(at, &commit, SECTION, kept(&found));
    }
    Ok(found)
}

/// The section `init` pins, so the shape of the two keys lives in one place. Spec 5.7.
pub fn section(found: &Derived) -> Value {
    let mut fields = Map::new();
    fields.insert(LINES.into(), found.lines.into());
    fields.insert(DIRECTORIES.into(), found.directories.into());
    Value::Object(fields)
}

/// The same two values under the derivation commit, with the sample they came from, so a run
/// that reads the cache can name it.
fn kept(found: &Derived) -> Value {
    let mut fields = section(found);
    if let Some(fields) = fields.as_object_mut() {
        fields.insert(COMMITS.into(), found.commits.into());
    }
    fields
}

fn held(cached: &Value) -> Option<Derived> {
    Some(Derived {
        lines: cached.get(LINES)?.as_u64()?,
        directories: cached.get(DIRECTORIES)?.as_u64()?,
        commits: cached.get(COMMITS)?.as_u64()? as usize,
    })
}

const COMMITS: &str = "commits";

/// The nearest-rank 90th percentile: sort ascending and take the value at `ceil(0.9 * n)`.
fn percentile(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    let rank = (values.len() as f64 * PERCENTILE).ceil() as usize;
    values[rank.clamp(1, values.len()) - 1]
}

/// One pair per commit in the sample, from one walk of the last non-merge commits: the lines it
/// changed against its first parent, and how many immediate parent directories they sat under.
fn sampled(root: &Path, commit: &str) -> Option<Vec<(u64, u64)>> {
    let most = String::from("--max-count=") + &SAMPLE.to_string();
    let mut args = SETTINGS.to_vec();
    args.extend(["log", "--no-merges", "--format=%H", "--no-renames"]);
    args.extend_from_slice(COUNTED);
    args.extend([most.as_str(), commit]);
    let text = stamp::git(root, None, &args)?;
    let mut commits = Vec::new();
    let mut rows = Vec::new();
    let mut started = false;
    for line in text.lines().filter(|line| !line.is_empty()) {
        if !line.contains('\t') {
            if started {
                commits.push(changed(&rows));
                rows.clear();
            }
            started = true;
            continue;
        }
        if let Some(row) = entry(line) {
            rows.push(row);
        }
    }
    if started {
        commits.push(changed(&rows));
    }
    Some(commits)
}

fn changed(rows: &[(String, u64)]) -> (u64, u64) {
    (total(rows), directories(rows).len() as u64)
}

/// The turn, measured from the prompt mark to a tree of the working directory, so uncommitted
/// and untracked work counts and nothing is written for it.
fn measured(root: &Path, opened: &str, tree: &str) -> Option<Spread> {
    let raw = numstat(root, opened, tree, &["--no-renames"])?;
    let spaced = numstat(root, opened, tree, &["-w", "--no-renames"])?;
    let renamed = numstat(root, opened, tree, &["-M"])?;
    let lines = total(&raw);
    Some(Spread {
        files: raw.len(),
        directories: directories(&raw),
        formatting: lines.saturating_sub(total(&spaced)),
        moved: lines.saturating_sub(total(&renamed)),
        largest: largest(&raw),
        lines,
    })
}

fn numstat(root: &Path, opened: &str, tree: &str, how: &[&str]) -> Option<Vec<(String, u64)>> {
    let mut args = SETTINGS.to_vec();
    args.push("diff");
    args.extend_from_slice(COUNTED);
    args.extend_from_slice(how);
    args.extend([opened, tree]);
    let text = stamp::git(root, None, &args)?;
    Some(text.lines().filter_map(entry).collect())
}

/// One `--numstat` row. A binary file counts as no lines, which is what git says of it.
fn entry(line: &str) -> Option<(String, u64)> {
    let mut fields = line.split('\t');
    let added = fields.next()?.parse().unwrap_or(0);
    let deleted: u64 = fields.next()?.parse().unwrap_or(0);
    let path = fields.next()?.trim();
    (!path.is_empty()).then(|| (path.to_string(), added + deleted))
}

fn total(rows: &[(String, u64)]) -> u64 {
    rows.iter().map(|(_, lines)| lines).sum()
}

fn largest(rows: &[(String, u64)]) -> Option<(String, u64)> {
    rows.iter()
        .max_by_key(|(_, lines)| *lines)
        .map(|(path, lines)| (path.clone(), *lines))
}

/// The distinct immediate parents of the changed files, and `./` for a file at the tree root.
fn directories(rows: &[(String, u64)]) -> Vec<String> {
    let mut found: Vec<String> = rows
        .iter()
        .map(|(path, _)| match path.rsplit_once('/') {
            Some((parent, _)) => format!("{parent}/"),
            None => "./".to_string(),
        })
        .collect();
    found.sort();
    found.dedup();
    found
}
