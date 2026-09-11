use std::io::{BufRead, BufReader, Write};
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

/// The fields of 11.4 every kind of line carries, so a fifth verb cannot forget one. Spec 9.6.
fn base(kind: &'static str) -> Map<String, Value> {
    let mut line = Map::new();
    line.insert("schema".into(), SCHEMA.into());
    line.insert("version".into(), env!("CARGO_PKG_VERSION").into());
    line.insert("time".into(), now().into());
    line.insert("kind".into(), kind.into());
    line
}

/// The stop's line: the 11.2 object the run built, plus what only the hook knew. Spec 11.4.
pub fn stop(root: &Path, stop: &Stop) {
    let Ok(at) = state::ready(root) else {
        return;
    };
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
        Some(Value::Object(section)) => section
            .get("prompt")
            .and_then(Value::as_bool)
            .unwrap_or(true),
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
