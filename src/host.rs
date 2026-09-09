use std::io::{IsTerminal, Read};

use serde_json::Value;

/// The hosts klin knows how to read. Cursor and Codex CLI are separate tickets, and until they
/// land an event klin cannot place is read as Claude Code's shape. Section 9.1.
#[derive(Clone, Copy)]
pub enum Host {
    Claude,
}

/// One host event, in klin's own words.
pub struct Event {
    pub host: Host,
    pub tool: String,
    pub file_path: String,
    pub command: String,
    pub blocked_before: bool,
}

/// What the guard decides about a tool call. ADR 0020's `ask` lands with its own ticket, and
/// needs the decision on stdout under exit 0, which no host reads while a deny exits 2.
pub enum Decision {
    Allow,
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
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text).ok()?;
    let event = serde_json::from_str::<Value>(&text).ok()?;
    Some(match host(flag, &event) {
        Host::Claude => claude(&event),
    })
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
    eprintln!("klin: NOTE: {why} — reading it as {CLAUDE}.");
    Host::Claude
}

fn claude(event: &Value) -> Event {
    let input = |key: &str| text(event.get("tool_input").and_then(|input| input.get(key)));
    let path = input("file_path");
    Event {
        host: Host::Claude,
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

/// The decision in the host's shape, and the exit code that carries it.
pub fn decide(event: &Event, decision: &Decision) -> u8 {
    match event.host {
        Host::Claude => claude_decision(decision),
    }
}

/// A deny is exit 2 with the reason on stderr, which is what Claude Code reads back to the
/// agent. Nothing goes to stdout: on a blocking exit the host does not read it.
fn claude_decision(decision: &Decision) -> u8 {
    let Decision::Deny(reason) = decision else {
        return 0;
    };
    eprintln!("{reason}");
    2
}

pub fn stop(host: Host, stop: &Stop) -> u8 {
    match (host, stop) {
        (Host::Claude, Stop::Block) => 2,
        (Host::Claude, Stop::Pass) => 0,
    }
}
