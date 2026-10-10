use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::sys::state;
use crate::sys::write::{AtomicWrite, atomic_write};

/// Where the state directory keeps what each session of a host that submits a stop's text as
/// its next prompt was handed: one file per session id, so one session's write never replaces
/// another session's record. Only that session's own hooks write its file, and a host runs one
/// session's hooks in order. Spec 9.1, ADR 0052.
const DIR: &str = "handed";

/// What one session was handed.
#[derive(Default)]
struct Handed {
    /// The hash of the exact text the host will submit next. It is consumed once, so
    /// protocol-generated text cannot open a fresh turn, and another prompt clears it.
    followup: Option<u64>,
}

/// Remember the exact report a session's host will submit as its next prompt. False when the
/// record could not be written, so the caller does not hand the host text it cannot recognize.
/// Spec 9.1.
pub fn expect_followup(root: &Path, session: &str, report: &str) -> bool {
    hand(root, session, |handed| {
        handed.followup = Some(state::hash(report.as_bytes()));
    })
}

/// Consume one expected follow-up of this session. A different prompt clears the expectation
/// and remains a person's prompt; an exact match is host-generated and opens no turn.
pub fn consumes(root: &Path, session: &str, prompt: &str) -> bool {
    let Some(at) = state::dir(root) else {
        return false;
    };
    let mut handed = read(&at, session);
    let expected = handed.followup.take();
    if expected.is_some() {
        write(&at, session, &handed);
    }
    expected == Some(state::hash(prompt.as_bytes()))
}

/// Forget what a session was handed, because its next prompt opened a turn. Spec 9.1.
pub fn clear(root: &Path, session: &str) {
    if let Some(at) = state::dir(root) {
        write(&at, session, &Handed::default());
    }
}

/// One change to what a session was handed, written back. False when it could not be.
fn hand(root: &Path, session: &str, change: impl FnOnce(&mut Handed)) -> bool {
    let Ok(at) = state::ready(root) else {
        return false;
    };
    let mut handed = read(&at, session);
    change(&mut handed);
    write(&at, session, &handed)
}

fn file(at: &Path, session: &str) -> PathBuf {
    at.join(DIR)
        .join(format!("{:016x}", state::hash(session.as_bytes())))
}

fn read(at: &Path, session: &str) -> Handed {
    let held = std::fs::read_to_string(file(at, session))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_default();
    Handed {
        followup: held.get("followup").and_then(Value::as_u64),
    }
}

/// The record written in place, or removed when it holds nothing. True when the disk holds
/// what was asked for.
fn write(at: &Path, session: &str, handed: &Handed) -> bool {
    let target = file(at, session);
    let mut fields = Map::new();
    if let Some(hash) = handed.followup {
        fields.insert("followup".into(), hash.into());
    }
    if fields.is_empty() {
        return std::fs::remove_file(&target).is_ok() || !target.exists();
    }
    if std::fs::create_dir_all(at.join(DIR)).is_err() {
        return false;
    }
    let text = Value::Object(fields).to_string() + "\n";
    atomic_write(AtomicWrite {
        target: &target,
        bytes: text.as_bytes(),
        keep_mode_from: None,
    })
    .is_ok()
}
