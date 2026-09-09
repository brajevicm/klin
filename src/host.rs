use std::io::{IsTerminal, Read};

use serde_json::Value;

/// The hosts klin knows how to read. Cursor and Codex CLI are separate tickets, and until they
/// land an event klin cannot place is read as Claude Code's shape. Section 9.1.
#[derive(Clone, Copy)]
pub enum Host {
    Claude,
    /// An event or a `--host` name klin cannot place. It is read and answered as Claude Code,
    /// except that it is not credited with `ask`: an unplaced host gets ADR 0011's `deny`.
    Unplaced,
}

/// One host event, in klin's own words.
pub struct Event {
    pub host: Host,
    pub tool: String,
    pub file_path: String,
    pub command: String,
    pub blocked_before: bool,
}

/// What the guard decides about a tool call. ADR 0020. A deny exits 2 with the reason on
/// stderr. An ask needs the decision on stdout under exit 0, because a blocking exit says
/// only that the call is refused, and an ask leaves the answer to a person.
pub enum Decision {
    Allow,
    Ask(String),
    Deny(String),
}

pub enum Stop {
    Pass,
    Block,
}

const CLAUDE: &str = "claude";
/// A field only Claude Code sends. One of them is enough to place the event.
const CLAUDE_FIELDS: &[&str] = &[
    "hook_event_name",
    "tool_name",
    "tool_input",
    "session_id",
    "stop_hook_active",
];

/// The event on stdin, or `None` when there is nothing to read or the text is not JSON. A
/// caller that gets `None` must let the turn through: klin says nothing about what it cannot read.
pub fn read(flag: Option<&str>) -> Option<Event> {
    let event = payload()?;
    Some(claude(&event, host(flag, &event)))
}

/// The host's own name for the event, such as `UserPromptSubmit`. It places no host, because
/// a caller that only needs the name asks the host nothing, and the note about a host klin
/// cannot place belongs to the guard.
pub fn named() -> Option<String> {
    let name = text(payload()?.get("hook_event_name"));
    (!name.is_empty()).then_some(name)
}

fn payload() -> Option<Value> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).ok()?;
    serde_json::from_str(&text).ok()
}

fn host(flag: Option<&str>, event: &Value) -> Host {
    match flag {
        Some(CLAUDE) => Host::Claude,
        Some(other) => fell_back(&format!("--host {other} names no host klin knows")),
        None if CLAUDE_FIELDS.iter().any(|field| event.get(field).is_some()) => Host::Claude,
        None => fell_back("this hook event matches no host klin knows"),
    }
}

fn fell_back(why: &str) -> Host {
    eprintln!("klin: NOTE: {why} — reading it as {CLAUDE}, and asking it nothing.");
    Host::Unplaced
}

fn claude(event: &Value, host: Host) -> Event {
    let input = |key: &str| text(event.get("tool_input").and_then(|input| input.get(key)));
    let path = input("file_path");
    Event {
        host,
        tool: text(event.get("tool_name")),
        file_path: match path.is_empty() {
            true => input("notebook_path"),
            false => path,
        },
        command: input("command"),
        blocked_before: event
            .get("stop_hook_active")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The decision in the host's shape, and the exit code that carries it. A refusal is exit 2
/// with the reason on stderr, which is what Claude Code reads back to the agent. A host klin
/// cannot ask has no third answer, so the question is refused, and it is worded as a refusal:
/// an agent that is blocked cannot act on advice to let a person decide.
pub fn decide(event: &Event, decision: &Decision) -> u8 {
    match decision {
        Decision::Allow => 0,
        Decision::Ask(reason) if asks(event.host) => {
            println!("{}", claude_ask(&format!("klin: ask — {reason}")));
            0
        }
        Decision::Ask(reason) => {
            eprintln!("klin: refused — {reason} This host has no question to ask.");
            2
        }
        Decision::Deny(reason) => {
            eprintln!("{reason}");
            2
        }
    }
}

fn asks(host: Host) -> bool {
    matches!(host, Host::Claude)
}

/// Claude Code reads a pre-tool decision from stdout on exit 0. Section 9.1.
fn claude_ask(reason: &str) -> String {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": reason,
        }
    })
    .to_string()
}

/// Every host klin reads blocks a stop the same way, with exit 2.
pub fn stop(stop: &Stop) -> u8 {
    match stop {
        Stop::Block => 2,
        Stop::Pass => 0,
    }
}
