use std::fmt::Write;
use std::path::Path;

use crate::config::{Config, Error};
use crate::state;
use crate::turn;

/// The section that pins how wide this project's usual change is. Absent, #92 derives it, and
/// with neither the report on a prompt prints nothing. ADR 0014.
const SECTION: &str = "radius";
const LINES: &str = "lines";
const DIRECTORIES: &str = "directories";
/// Every invocation pins these, because a line count moves with the algorithm and with rename
/// detection, and an inherited git setting would measure the same turn twice over. ADR 0014.
const PINNED: &[&str] = &[
    "-c",
    "core.quotepath=false",
    "-c",
    "diff.relative=false",
    "-c",
    "diff.renameLimit=32767",
    "diff",
    "--numstat",
    "--diff-algorithm=histogram",
];

/// What the project usually does, against which a turn reads as wide. Either one exceeded is
/// enough, because a turn can spread by line count or by directory alone.
struct Usual {
    lines: u64,
    directories: u64,
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
/// nothing, and anything it cannot measure it says nothing about. ADR 0014.
pub fn spread(root: &Path, at: &Path, opened: Option<&str>, out: &mut String) {
    let Some(opened) = opened else {
        return;
    };
    let Ok(usual) = pinned(root) else {
        return;
    };
    let Some(spread) = measured(root, at, opened) else {
        return;
    };
    if spread.lines <= usual.lines && spread.directories.len() as u64 <= usual.directories {
        return;
    }
    let _ = writeln!(
        out,
        "klin: the previous turn was wider than this project's usual change."
    );
    describe(&spread, &usual, out);
}

/// `klin radius --report`, which a person runs. It moves nothing, it raises no counter, and
/// unlike the hook it names what it cannot read rather than staying quiet. ADR 0014.
pub fn asked(root: &Path, out: &mut String) -> Result<u8, Error> {
    let at = state::ready(root).map_err(Error)?;
    let usual = pinned(root)?;
    let opened = turn::opened(root, &at).ok_or_else(|| {
        Error("no turn stamp is readable, so there is no turn to measure".to_string())
    })?;
    let spread = measured(root, &at, &opened)
        .ok_or_else(|| Error("git could not measure this turn".to_string()))?;
    let _ = writeln!(out, "klin: this turn, measured against the turn stamp.");
    describe(&spread, &usual, out);
    Ok(0)
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

/// The values the config pins, and an error naming the key when it pins none. The hook drops
/// the error and prints nothing, and `--report` raises it.
fn pinned(root: &Path) -> Result<Usual, Error> {
    let config = Config::load(None, root)?;
    let section = config.section(SECTION)?;
    let number = |key: &str| match section.get(key) {
        None => Err(config.missing(SECTION, key)),
        Some(found) => found
            .as_u64()
            .ok_or_else(|| config.malformed(SECTION, key, "a whole number")),
    };
    Ok(Usual {
        lines: number(LINES)?,
        directories: number(DIRECTORIES)?,
    })
}

/// The turn, measured. The tree is the working directory as the stamp would take it, so
/// uncommitted and untracked work counts, and nothing is written for it.
fn measured(root: &Path, at: &Path, opened: &str) -> Option<Spread> {
    let tree = turn::tree(root, at)?;
    let raw = numstat(root, opened, &tree, &["--no-renames"])?;
    let spaced = numstat(root, opened, &tree, &["-w", "--no-renames"])?;
    let renamed = numstat(root, opened, &tree, &["-M"])?;
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
    let mut args = PINNED.to_vec();
    args.extend_from_slice(how);
    args.extend([opened, tree]);
    let text = turn::git(root, None, &args)?;
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
