//! The file an edit tool leaves, computed from the host's payload before the tool runs, so the
//! person approves the exact text the tool writes. Spec 10.8.2.

/// What one edit tool call writes into one file, in klin's own words.
pub enum Rewrite {
    /// The whole new text: Claude Code's `Write`, or a Codex patch that adds the file.
    Whole(String),
    /// Claude Code's `Edit` and `MultiEdit`, applied in order.
    Replaced(Vec<Replacement>),
    /// The chunks of one Codex `*** Update File:` section, applied in order.
    Patched(Vec<Chunk>),
    /// A payload klin cannot compute a result from.
    Unread,
}

pub struct Replacement {
    pub old: String,
    pub new: String,
    pub all: bool,
}

/// One `@@` chunk of a Codex patch: the line it seeks first, the lines it replaces, the lines it
/// writes, and whether it sits at the end of the file.
#[derive(Default)]
pub struct Chunk {
    pub anchor: Option<String>,
    pub old: Vec<String>,
    pub new: Vec<String>,
    pub end: bool,
}

impl Rewrite {
    /// The text the tool leaves in a file that holds `current`, or `None` where the host would
    /// fail the call or klin cannot tell what it writes.
    pub fn result(&self, current: &str) -> Option<String> {
        match self {
            Rewrite::Whole(text) => Some(text.clone()),
            Rewrite::Replaced(edits) => edits
                .iter()
                .try_fold(current.to_string(), |text, edit| replaced(&text, edit)),
            Rewrite::Patched(chunks) => patched(current, chunks),
            Rewrite::Unread => None,
        }
    }
}

/// Claude Code fails an edit whose old text is empty, missing, the same as the new text, or
/// found more than once without `replace_all`.
fn replaced(text: &str, edit: &Replacement) -> Option<String> {
    let found = text.matches(edit.old.as_str()).count();
    if edit.old.is_empty() || edit.old == edit.new || found == 0 || (found > 1 && !edit.all) {
        return None;
    }
    Some(text.replace(&edit.old, &edit.new))
}

/// Codex's own reading of a patch: each chunk seeks its anchor, then its old lines from where the
/// chunk before ended, and the result ends with a newline. Codex falls back to a match that
/// ignores whitespace; klin reads only the exact one, so such a patch is unread. Codex fails a
/// section with no chunk.
fn patched(current: &str, chunks: &[Chunk]) -> Option<String> {
    if chunks.is_empty() {
        return None;
    }
    let mut lines: Vec<&str> = current.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let found = chunks
        .iter()
        .scan(0, |from, chunk| {
            let placed = placed(&lines, chunk, *from);
            *from = placed.map_or(0, |(_, next)| next);
            Some(placed.map(|(at, _)| (at, chunk)))
        })
        .collect::<Option<Vec<_>>>()?;
    for (at, chunk) in found.into_iter().rev() {
        lines.splice(
            at..at + chunk.old.len(),
            chunk.new.iter().map(String::as_str),
        );
    }
    Some(lines.join("\n") + "\n")
}

/// Where a chunk's old lines start, at or after `from`, and where the next chunk seeks from. A
/// chunk with no old lines adds its lines at the end of the file and moves no further than its
/// anchor.
fn placed(lines: &[&str], chunk: &Chunk, from: usize) -> Option<(usize, usize)> {
    let from = match &chunk.anchor {
        Some(anchor) => seek(lines, std::slice::from_ref(anchor), from, false)? + 1,
        None => from,
    };
    match chunk.old.is_empty() {
        true => Some((lines.len(), from)),
        false => seek(lines, &chunk.old, from, chunk.end).map(|at| (at, at + chunk.old.len())),
    }
}

fn seek(lines: &[&str], pattern: &[String], from: usize, end: bool) -> Option<usize> {
    let last = lines.len().checked_sub(pattern.len())?;
    let start = match end {
        true => last,
        false => from,
    };
    (start..=last).find(|&at| {
        lines[at..at + pattern.len()]
            .iter()
            .zip(pattern)
            .all(|(line, wanted)| line == wanted)
    })
}
