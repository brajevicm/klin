#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use crate::changed::git;
use crate::config::Error;

/// How much of an untracked file klin reads to call it binary, which is what git reads.
const SNIFF: usize = 8000;

/// A run of added-side lines one hunk introduced, both ends inclusive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Range {
    pub start: u64,
    pub end: u64,
}

/// Which lines the window changed, per file, as the added side of every hunk. A rename is two
/// files under `--no-renames`, so its lines read as additions at the new path. An untracked file
/// the tree does not ignore is one range over the whole file. A binary file has no range.
/// Spec section 12, #110.
#[derive(Default)]
pub struct Hunks {
    by_path: BTreeMap<String, Vec<Range>>,
}

impl Hunks {
    /// The lines between two trees. `after` is a commit, or the working tree when it is None.
    /// Paths are relative to `root`, the way the changed set reports them.
    pub fn read(root: &Path, before: &str, after: Option<&str>) -> Result<Self, Error> {
        let mut args = vec![
            "diff",
            "-U0",
            "--diff-algorithm=histogram",
            "--no-renames",
            "--no-ext-diff",
            "--no-textconv",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "--relative",
            before,
        ];
        args.extend(after);
        args.push("--");
        let printed = git(root, &args).ok_or_else(|| {
            Error(format!(
                "the changed lines need a git repository, and git could not diff {} against {}",
                root.display(),
                before
            ))
        })?;
        let mut by_path = hunks(&printed);
        if after.is_none() {
            by_path.extend(untracked(root));
        }
        Ok(Hunks { by_path })
    }

    pub fn ranges(&self, path: &str) -> &[Range] {
        self.by_path.get(path).map_or(&[], Vec::as_slice)
    }

    pub fn holds(&self, path: &str, line: u64) -> bool {
        self.ranges(path)
            .iter()
            .any(|range| range.start <= line && line <= range.end)
    }
}

fn hunks(printed: &str) -> BTreeMap<String, Vec<Range>> {
    let mut by_path: BTreeMap<String, Vec<Range>> = BTreeMap::new();
    let mut path = None;
    let mut heading = false;
    for line in printed.lines() {
        if line.starts_with("diff --git ") {
            (path, heading) = (None, true);
            continue;
        }
        if heading {
            if let Some(field) = line.strip_prefix("+++ ") {
                path = named(field);
                continue;
            }
            if !line.starts_with("@@ ") {
                continue;
            }
            heading = false;
        }
        let Some(header) = line.strip_prefix("@@ ") else {
            continue;
        };
        let (Some(named), Some(range)) = (path.as_ref(), added(header)) else {
            continue;
        };
        by_path.entry(named.clone()).or_default().push(range);
    }
    by_path
}

/// The new path a `+++` line names. Git appends a tab when the name holds a space, and quotes
/// the whole field when the name holds a quote, a backslash or a control character. A quoted
/// name stays quoted, because that is the form the changed set reports.
fn named(field: &str) -> Option<String> {
    match field
        .strip_prefix("\"b/")
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(inside) => Some(format!("\"{inside}\"")),
        None => field
            .trim_end_matches('\t')
            .strip_prefix("b/")
            .map(str::to_string),
    }
}

/// The added side of `@@ -a,b +c,d @@`. A count the header omits is one line, and a count of
/// zero is a deletion, which added no line.
fn added(header: &str) -> Option<Range> {
    let side = header
        .split_whitespace()
        .find_map(|part| part.strip_prefix('+'))?;
    let (start, count) = match side.split_once(',') {
        Some((start, count)) => (start.parse().ok()?, count.parse().ok()?),
        None => (side.parse().ok()?, 1),
    };
    (count > 0).then(|| Range {
        start,
        end: start + count - 1,
    })
}

fn untracked(root: &Path) -> BTreeMap<String, Vec<Range>> {
    let listed = git(root, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default();
    listed
        .lines()
        .filter(|name| !name.is_empty())
        .filter_map(|name| {
            let bytes = std::fs::read(root.join(name)).ok()?;
            let whole = whole(&bytes)?;
            Some((name.to_string(), vec![whole]))
        })
        .collect()
}

fn whole(bytes: &[u8]) -> Option<Range> {
    if bytes.is_empty() || bytes[..bytes.len().min(SNIFF)].contains(&0) {
        return None;
    }
    let breaks = bytes.iter().filter(|byte| **byte == b'\n').count() as u64;
    let end = breaks + u64::from(!bytes.ends_with(b"\n"));
    Some(Range { start: 1, end })
}
