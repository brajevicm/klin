use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::config::{self, Config};
use crate::host;
use crate::state;

/// The journal: one JSON line per hook stop, appended under the state directory. It is the
/// record `klin stats` and the benchmark read, so its shape is a contract. The write rule is
/// `state.rs`'s: nothing klin writes for itself may change a block or a pass, so the append is
/// best-effort, a failed one prints nothing to the agent, the hook never prunes, and `cache
/// clean` leaves the file alone. Spec 9.6, 11.4.
pub const FILE: &str = "journal.jsonl";

/// The line format's version. A reader skips a line whose schema it does not know. A bump adds
/// a variant, and `known` does not compile until it has an arm for it. Spec 11.4.
enum Schema {
    One = 1,
}

const SCHEMA: Schema = Schema::One;

/// What one stop knew beyond the 11.2 object its run built: gathered as the stop goes, written
/// as one line at its end.
pub struct Stop {
    /// The 11.2 object, whichever path built it: the gates, a build failure, or an error.
    pub report: Option<Value>,
    pub host: Option<String>,
    pub session: Option<String>,
    pub prompt: u64,
    pub blocked: bool,
    pub blocked_before: bool,
    /// The build stamp as this stop left it. Spec 16.3.
    pub gate_blocks: u64,
    pub build_blocks: u64,
    /// The number of the gate block this stop itself spent, `None` where it spent none, so a
    /// reader tells the stop that blocked from a later one that only sees the spent count.
    /// ADR 0052.
    pub gate_block: Option<u64>,
    /// The verdict this stop wrote: `green`, `red`, or `none` when it wrote nothing.
    pub verdict: &'static str,
    /// Why the stop wrote no verdict, beside `verdict: "none"` alone.
    pub why: Option<&'static str>,
    /// The site ids this stop asked about, as the turn stamp records them. Spec 8.2.
    pub asked: Vec<String>,
    /// The unusual paths this stop took, empty on a clean one: `turn-restored`,
    /// `branch-fallback`, `count-unwritable`, `no-prompt-event`.
    pub flags: Vec<&'static str>,
    /// The parts of the `systemMessage` this stop put in front of the person, empty when it
    /// printed none: `note`, `turn` and `weekly`. Spec 9.5.
    pub told: Vec<&'static str>,
    pub timing: Timing,
    /// A hash of the config in force, recorded and not read, so a later reader can tell a fix
    /// from a config change without a schema bump.
    pub config_hash: String,
}

#[derive(Default)]
pub struct Timing {
    pub total_ms: u64,
    pub build_ms: u64,
    pub lock_ms: u64,
    pub base_remove_ms: u64,
    pub base_prune_ms: u64,
}

impl Stop {
    pub fn begun(event: Option<&host::Event>, config_hash: String) -> Stop {
        Stop {
            report: None,
            host: event.map(|event| event.host.name().to_string()),
            session: event
                .map(|event| event.session.clone())
                .filter(|session| !session.is_empty()),
            prompt: 0,
            blocked: false,
            blocked_before: event.is_some_and(|event| event.blocked_before),
            gate_blocks: 0,
            build_blocks: 0,
            gate_block: None,
            verdict: "none",
            why: None,
            asked: Vec::new(),
            flags: Vec::new(),
            told: Vec::new(),
            timing: Timing::default(),
            config_hash,
        }
    }
}

/// The fields of 11.4 every kind of line carries, so a fifth verb cannot forget one. Spec 9.6.
fn base(kind: &'static str) -> Map<String, Value> {
    let mut line = Map::new();
    line.insert("schema".into(), (SCHEMA as u64).into());
    line.insert("version".into(), env!("CARGO_PKG_VERSION").into());
    line.insert("time".into(), now().into());
    line.insert("kind".into(), kind.into());
    line
}

/// Appends the stop's line. Spec 11.4.
pub fn stop(root: &Path, stop: &Stop) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    append(&at, &line(stop));
}

/// The stop's line: the 11.2 object the run built, plus what only the hook knew. A reader can
/// take it before it is appended, as the turn end does to count the stop it ends on.
pub fn line(stop: &Stop) -> Value {
    let mut line = base("stop");
    if let Some(Value::Object(fields)) = &stop.report {
        for (key, value) in fields {
            line.entry(key.clone()).or_insert(value.clone());
        }
    }
    line.insert("host".into(), stop.host.clone().into());
    line.insert("session".into(), stop.session.clone().into());
    line.insert("prompt".into(), stop.prompt.into());
    line.insert(
        "hook".into(),
        serde_json::json!({
            "blocked": stop.blocked,
            "delivery": match stop.blocked {
                true => "block",
                false => "none",
            },
            "gate_spent": stop.gate_blocks > 0,
            "gate_blocks": stop.gate_blocks,
            "gate_block": stop.gate_block,
            "build_blocks": stop.build_blocks,
            "blocked_before": stop.blocked_before,
        }),
    );
    line.insert("verdict".into(), stop.verdict.into());
    if let Some(why) = stop.why {
        line.insert("why".into(), why.into());
    }
    line.insert(
        "timing".into(),
        serde_json::json!({
            "total_ms": stop.timing.total_ms,
            "build_ms": stop.timing.build_ms,
            "lock_ms": stop.timing.lock_ms,
            "base_remove_ms": stop.timing.base_remove_ms,
            "base_prune_ms": stop.timing.base_prune_ms,
            "klin_ms": stop.timing.total_ms.saturating_sub(stop.timing.build_ms),
        }),
    );
    line.insert("asked".into(), stop.asked.clone().into());
    line.insert("flags".into(), stop.flags.clone().into());
    line.insert("told".into(), stop.told.clone().into());
    line.insert("config_hash".into(), stop.config_hash.clone().into());
    Value::Object(line)
}

/// The prompt event `klin radius` runs on: the counter, the session, the prompt's first line cut
/// at 80 characters unless `journal.prompt` is `false`, and the radius facts when radius measured
/// them. `enabled` is `prompt_enabled` of the config the caller already loaded for the same
/// event, so this appends without reading klin.json a second time. Spec 9.6, 11.4.
pub fn prompt(
    root: &Path,
    counter: u64,
    event: Option<&host::Event>,
    enabled: bool,
    radius: Option<Value>,
) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    let mut line = base("prompt");
    line.insert("prompt".into(), counter.into());
    line.insert(
        "session".into(),
        event
            .map(|event| event.session.clone())
            .filter(|session| !session.is_empty())
            .into(),
    );
    let text = event
        .filter(|_| enabled)
        .map(|event| excerpt(&event.prompt))
        .filter(|text| !text.is_empty());
    if let Some(text) = text {
        line.insert("text".into(), text.into());
    }
    if let Some(radius) = radius {
        line.insert("radius".into(), radius);
    }
    append(&at, &Value::Object(line));
}

/// The first line of a prompt, cut at 80 characters and not bytes, so a multi-byte character is
/// never split.
fn excerpt(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(80)
        .collect()
}

/// Whether `journal.prompt` lets a prompt line carry the excerpt. On by default. Takes the
/// config already loaded, so a caller with one loaded for another reason reads klin.json once
/// and not twice. Spec 5.2.
pub fn prompt_enabled(loaded: &Config) -> bool {
    match loaded.pinned(config::JOURNAL.name) {
        Some(Value::Object(section)) => match section.get("prompt") {
            Some(Value::Bool(enabled)) => *enabled,
            Some(_) => false,
            None => true,
        },
        _ => true,
    }
}

/// A guard ask or deny: the decision the host delivered, and the hyphenated reason the guard
/// built it with. `delivered` is the adapter's own exit code, because a host that cannot ask
/// refuses instead (9.1), and 11.4 must count that refusal as the deny it is. An allow appends
/// nothing, because the guard runs on every tool call under its 50 ms budget and an allow tells
/// a reader nothing. `at` is the state directory the guard resolved already. Spec 9.6, 11.4.
pub fn guard(root: &Path, at: &Path, event: &host::Event, delivered: u8, reason: &'static str) {
    let kind = match delivered {
        0 => "ask",
        _ => "deny",
    };
    let Ok(at) = state::prepared(at, root) else {
        return;
    };
    let mut line = base("guard");
    line.insert(
        "session".into(),
        (!event.session.is_empty())
            .then(|| event.session.clone())
            .into(),
    );
    line.insert("decision".into(), kind.into());
    line.insert("reason".into(), reason.into());
    append(&at, &Value::Object(line));
}

/// `klin turn reset`: the prompt counter the stamp carried over. Spec 9.6, 11.4.
pub fn reset(root: &Path, counter: u64) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    let mut line = base("reset");
    line.insert("session".into(), Value::Null);
    line.insert("prompt".into(), counter.into());
    append(&at, &Value::Object(line));
}

/// The one place a write's `Result` is dropped on purpose: a journal klin cannot append to
/// costs the record and nothing else, because nothing klin writes for itself may change a
/// block or a pass, and a note about it would reach the agent instead of a person. Spec 9.6.
fn append(at: &Path, line: &Value) {
    let opened = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(at.join(FILE));
    if let Ok(mut file) = opened {
        let _ = file.write_all((line.to_string() + "\n").as_bytes());
    }
}

/// One clock for the work the journal times: the check run, the build and the lock wait.
pub fn timed<T>(work: impl FnOnce() -> T) -> (T, u64) {
    let begun = Instant::now();
    let out = work();
    (out, millis(begun.elapsed()))
}

pub fn millis(spent: Duration) -> u64 {
    u64::try_from(spent.as_millis()).unwrap_or(u64::MAX)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

/// Every line of this worktree's journal, newest last, and how many the reader skipped. A line
/// it cannot parse is a truncated last write, and a line whose `schema` it does not know is a
/// record a newer klin wrote: both are skipped and counted, so a report can say its numbers are
/// short. Spec 11.4.
pub fn read(root: &Path) -> (Vec<Value>, u64) {
    let Some(at) = state::dir(root) else {
        return (Vec::new(), 0);
    };
    let Ok(file) = std::fs::File::open(at.join(FILE)) else {
        return (Vec::new(), 0);
    };
    let mut lines = Vec::new();
    let mut skipped = 0;
    for read in BufReader::new(file).lines() {
        let Ok(text) = read else {
            skipped += 1;
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        match understood(text.as_bytes()) {
            Some(line) => lines.push(line),
            None => skipped += 1,
        }
    }
    (lines, skipped)
}

/// The line both readers take, and `None` for the one neither does: a line that will not parse,
/// which is a truncated last write, or a line whose `schema` a newer klin wrote. Spec 11.4.
fn understood(text: &[u8]) -> Option<Value> {
    serde_json::from_slice::<Value>(text).ok().filter(known)
}

/// Whether this binary understands the line's format. The match is on the current schema with no
/// wildcard arm, so a bump without an upgrade arm does not compile.
fn known(line: &Value) -> bool {
    let schema = line.get("schema").and_then(Value::as_u64);
    match SCHEMA {
        Schema::One => schema == Some(Schema::One as u64),
    }
}

/// The tail of this worktree's journal, and what the reader proved beyond it. A stop reads a
/// tail and not the whole file, so a journal that grows for a year does not lengthen a stop.
/// Spec 11.4.
#[derive(Default)]
pub struct Tail {
    /// The known lines from the cutoff onwards, newest last, as `read` orders them.
    pub lines: Vec<Value>,
    /// The lines in that range the reader could not take.
    pub skipped: u64,
    /// Whether a known line older than the cutoff exists, which is how a reader tells a journal
    /// that reaches further back from one that begins inside the window.
    pub older: bool,
}

/// How much of the file one backward read takes.
const CHUNK: u64 = 64 * 1024;

/// The journal from `cutoff` onwards, read backwards from the end in chunks and stopped at the
/// first line older than the cutoff, so older history is never parsed. A line the reader cannot
/// parse is skipped and counted, never read as the older line that would end the scan, so a
/// truncated last write and a line from a newer klin both leave the bound where it was. A cutoff
/// of zero reads the whole file. Spec 11.4.
pub fn tail(root: &Path, cutoff: u64) -> Tail {
    match state::dir(root) {
        Some(at) => scan(&at.join(FILE), cutoff),
        None => Tail::default(),
    }
}

fn scan(file: &Path, cutoff: u64) -> Tail {
    let mut tail = Tail::default();
    let Ok(mut file) = std::fs::File::open(file) else {
        return tail;
    };
    let Ok(mut end) = file.seek(SeekFrom::End(0)) else {
        return tail;
    };
    let mut held: Vec<u8> = Vec::new();
    while end > 0 && !tail.older {
        let from = end.saturating_sub(CHUNK);
        let Some(mut bytes) = chunk(&mut file, from, end) else {
            break;
        };
        bytes.extend_from_slice(&held);
        let (front, whole) = cut(&bytes, from == 0);
        held = front.to_vec();
        for text in whole.split(|byte| *byte == b'\n').rev() {
            if take(&mut tail, text, cutoff) {
                break;
            }
        }
        end = from;
    }
    tail.lines.reverse();
    tail
}

/// The bytes of one backward read, and `None` where the file moved under the reader.
fn chunk(file: &mut std::fs::File, from: u64, end: u64) -> Option<Vec<u8>> {
    let mut bytes = vec![0; usize::try_from(end - from).ok()?];
    file.seek(SeekFrom::Start(from)).ok()?;
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

/// A chunk cut into the fragment of a line the chunk before it begins, which is nothing once the
/// read reaches the start of the file, and the complete lines after it.
fn cut(chunk: &[u8], first: bool) -> (&[u8], &[u8]) {
    if first {
        return (&[], chunk);
    }
    match chunk.iter().position(|byte| *byte == b'\n') {
        Some(at) => chunk.split_at(at + 1),
        None => (chunk, &[]),
    }
}

/// One line of a backward read, newest first. The answer is whether the line proved the cutoff
/// and ended the scan: only a known line carrying a time older than the cutoff does.
fn take(tail: &mut Tail, text: &[u8], cutoff: u64) -> bool {
    if text.iter().all(u8::is_ascii_whitespace) {
        return false;
    }
    let Some(line) = understood(text) else {
        tail.skipped += 1;
        return false;
    };
    match line.get("time").and_then(Value::as_u64) {
        Some(time) if time < cutoff => tail.older = true,
        _ => tail.lines.push(line),
    }
    tail.older
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: u64 = 1_000;
    const RECENT: u64 = 100_000;
    const CUTOFF: u64 = 50_000;

    fn row(time: u64, kind: &str) -> String {
        format!("{{\"schema\":1,\"time\":{time},\"kind\":\"{kind}\",\"session\":\"s\"}}\n")
    }

    fn recent() -> String {
        (0..5).map(|step| row(RECENT + step, "stop")).collect()
    }

    fn read_back(old: usize, tail: &str) -> Tail {
        let text: String = (0..old).map(|_| row(OLD, "stop")).collect::<String>() + tail;
        let at = tempfile::tempdir().expect("a temporary directory");
        let file = at.path().join(FILE);
        std::fs::write(&file, text).expect("the journal written");
        scan(&file, CUTOFF)
    }

    /// The lines the reader parsed: the ones it took, the ones it skipped, and the older one it
    /// stopped on. Nothing else is read, so this is the work a longer history must not widen.
    fn parsed(tail: &Tail) -> usize {
        tail.lines.len() + tail.skipped as usize + usize::from(tail.older)
    }

    #[test]
    fn a_longer_history_does_not_widen_the_read() {
        let small = read_back(1_000, &recent());
        let large = read_back(100_000, &recent());
        assert_eq!(parsed(&small), 6);
        assert_eq!(parsed(&large), parsed(&small));
        assert_eq!(large.lines.len(), small.lines.len());
        assert!(small.older && large.older);
    }

    #[test]
    fn the_lines_come_back_oldest_first() {
        let tail = read_back(10, &recent());
        let times: Vec<u64> = tail
            .lines
            .iter()
            .map(|line| line.get("time").and_then(Value::as_u64).unwrap_or_default())
            .collect();
        assert_eq!(
            times,
            vec![RECENT, RECENT + 1, RECENT + 2, RECENT + 3, RECENT + 4]
        );
    }

    #[test]
    fn a_truncated_last_write_is_skipped_and_is_not_the_bound() {
        let tail = read_back(10, &(recent() + "{\"schema\":1,\"tim"));
        assert_eq!(tail.lines.len(), 5);
        assert_eq!(tail.skipped, 1);
        assert!(tail.older);
    }

    #[test]
    fn a_line_from_a_newer_klin_is_skipped_and_is_not_the_bound() {
        let tail = read_back(
            10,
            &(recent() + "{\"schema\":99,\"time\":1,\"kind\":\"stop\"}\n"),
        );
        assert_eq!(tail.lines.len(), 5);
        assert_eq!(tail.skipped, 1);
        assert!(tail.older);
    }

    #[test]
    fn a_journal_that_begins_inside_the_window_says_so() {
        let tail = read_back(0, &recent());
        assert_eq!(tail.lines.len(), 5);
        assert!(!tail.older);
    }
}
