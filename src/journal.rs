use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use crate::host;
use crate::state;

/// The journal: one JSON line per hook stop, appended under the state directory. It is the
/// record `klin stats` and the benchmark read, so its shape is a contract. The write rule is
/// `state.rs`'s: nothing klin writes for itself may change a block or a pass, so the append is
/// best-effort, a failed one prints nothing to the agent, the hook never prunes, and `cache
/// clean` leaves the file alone. Spec 9.6, 11.4.
pub const FILE: &str = "journal.jsonl";

/// The line format's version. A reader skips a line whose schema it does not know.
const SCHEMA: u64 = 1;

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
    pub gate_spent: bool,
    pub build_blocks: u64,
    /// The verdict this stop wrote: `green`, `red`, or `none` when it wrote nothing.
    pub verdict: &'static str,
    /// Why the stop wrote no verdict, beside `verdict: "none"` alone.
    pub why: Option<&'static str>,
    /// The site ids this stop asked about, as the turn stamp records them. Spec 8.2.
    pub asked: Vec<String>,
    /// The unusual paths this stop took, empty on a clean one: `turn-restored`,
    /// `branch-fallback`, `count-unwritable`.
    pub flags: Vec<&'static str>,
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
            gate_spent: false,
            build_blocks: 0,
            verdict: "none",
            why: None,
            asked: Vec::new(),
            flags: Vec::new(),
            timing: Timing::default(),
            config_hash,
        }
    }
}

/// The stop's line: the 11.2 object the run built, plus what only the hook knew. Spec 11.4.
pub fn stop(root: &Path, stop: &Stop) {
    let Ok(at) = state::ready(root) else {
        return;
    };
    let mut line = match &stop.report {
        Some(Value::Object(fields)) => fields.clone(),
        _ => Map::new(),
    };
    line.insert("schema".into(), SCHEMA.into());
    line.insert("version".into(), env!("CARGO_PKG_VERSION").into());
    line.insert("time".into(), now().into());
    line.insert("kind".into(), "stop".into());
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
            "gate_spent": stop.gate_spent,
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
            "klin_ms": stop.timing.total_ms.saturating_sub(stop.timing.build_ms),
        }),
    );
    line.insert("asked".into(), stop.asked.clone().into());
    line.insert("flags".into(), stop.flags.clone().into());
    line.insert("config_hash".into(), stop.config_hash.clone().into());
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
        match serde_json::from_str::<Value>(&text) {
            Ok(line) if known(&line) => lines.push(line),
            _ => skipped += 1,
        }
    }
    (lines, skipped)
}

/// Whether this binary understands the line's format. The match is exhaustive up to the current
/// schema, so a bump without an upgrade arm does not compile.
fn known(line: &Value) -> bool {
    match line.get("schema").and_then(Value::as_u64) {
        Some(1) => true,
        Some(_) | None => false,
    }
}
