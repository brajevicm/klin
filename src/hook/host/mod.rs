pub mod adapter;
mod claude;
mod codex;
mod cursor;
mod generic;

use std::io::{IsTerminal, Read};

use serde_json::Value;

use adapter::{Adapter, Event};

/// The fields that scope an event within its session: a prompt, a turn, a message, a tool call,
/// or what started the session. A payload that names none of them cannot tell a copy of one
/// prompt from the next prompt, so it names no event to share. Spec 9.8.
const SCOPED: &[&str] = &[
    "prompt_id",
    "turn_id",
    "generation_id",
    "tool_use_id",
    "source",
];

/// Every host klin reads, in the order an unlabelled event is tried against them. Codex CLI
/// sends Claude Code's fields and more, so it goes first. Cursor sends `cursor_version` on
/// events that also carry Claude's fields, so it goes before Claude Code.
pub const ADAPTERS: &[&dyn Adapter] = &[
    &codex::Codex,
    &cursor::Cursor,
    &claude::Claude { asks: true },
];

/// An event or a `--host` name klin cannot place. It is read and answered as Claude Code,
/// except that it is not credited with `ask`: an unplaced host gets ADR 0011's `deny`.
const UNPLACED: &dyn Adapter = &claude::Claude { asks: false };

/// The event on stdin, or `None` when there is nothing to read or the text is not JSON. A
/// caller that gets `None` must let the turn through: klin says nothing about what it cannot read.
pub fn read(flag: Option<&str>) -> Option<Event> {
    let payload = payload()?;
    let host = placed(flag, &payload);
    let name = host.event_name(&payload);
    let mut event = host.event(&payload);
    event.root = host.root(&payload);
    event.kind = host.kind(&name);
    if !event.session.is_empty() && SCOPED.iter().any(|field| payload.get(field).is_some()) {
        event.identity = host.identity(&payload);
    }
    Some(event)
}

/// The event on stdin, read once per process and cached: `read`, `root` and `prompted` each ask
/// for it, and stdin has only one reading.
fn payload() -> Option<Value> {
    static PAYLOAD: std::sync::OnceLock<Option<Value>> = std::sync::OnceLock::new();
    PAYLOAD.get_or_init(read_stdin).clone()
}

fn read_stdin() -> Option<Value> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

fn placed(flag: Option<&str>, payload: &Value) -> &'static dyn Adapter {
    if let Some(host) = generic::placed(flag, payload) {
        return host;
    }
    let found = match flag {
        Some(name) => ADAPTERS.iter().find(|host| host.name() == name),
        None => ADAPTERS.iter().find(|host| host.placed(payload)),
    };
    match (found, flag) {
        (Some(host), _) => *host,
        (None, Some(name)) => fell_back(&format!("--host {name} names no host klin knows")),
        (None, None) => fell_back("this hook event matches no host klin knows"),
    }
}

fn fell_back(why: &str) -> &'static dyn Adapter {
    eprintln!(
        "klin: NOTE: {why} — reading it as {}, and asking it nothing.",
        UNPLACED.name()
    );
    UNPLACED
}

/// The adapter that answers this event, or Claude Code without `ask` when none placed.
pub fn answering(event: Option<&Event>) -> &'static dyn Adapter {
    event.map_or(UNPLACED, |event| event.host)
}
