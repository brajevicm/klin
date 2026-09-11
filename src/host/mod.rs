mod claude;
mod codex;

use std::io::{IsTerminal, Read};

use serde_json::Value;

/// One host klin speaks to. A port is one module that implements this and one entry in
/// `ADAPTERS`, and nothing else in the crate names the host. Section 9.1.
pub trait Adapter: Sync {
    /// The name `--host` takes.
    fn name(&self) -> &'static str;
    /// The directory whose presence says a tree uses this host.
    fn marker(&self) -> &'static str;
    /// The file the host reads its hooks from, relative to a tree root or to the home directory.
    fn hook_file(&self) -> &'static str;
    /// The tools the guard reads on the pre-tool event, in the host's matcher syntax.
    fn matcher(&self) -> &'static str;
    /// The key the host's hook file lists enabled plugins under. `None` for a host that
    /// enables plugins somewhere else, or has no klin plugin to enable.
    fn plugin_key(&self) -> Option<&'static str> {
        None
    }
    /// Whether an event with no `--host` has this host's shape.
    fn placed(&self, payload: &Value) -> bool;
    /// The event in klin's own words.
    fn event(&'static self, payload: &Value) -> Event;
    /// The guard's decision in the host's shape, and the exit code that carries it.
    fn decide(&self, decision: &Decision) -> u8;
}

/// Every host klin reads, in the order an unlabelled event is tried against them. Codex CLI
/// sends Claude Code's fields and more, so it goes first.
pub const ADAPTERS: &[&dyn Adapter] = &[&codex::Codex, &claude::Claude { asks: true }];

/// An event or a `--host` name klin cannot place. It is read and answered as Claude Code,
/// except that it is not credited with `ask`: an unplaced host gets ADR 0011's `deny`.
const UNPLACED: &dyn Adapter = &claude::Claude { asks: false };

/// One host event, in klin's own words.
pub struct Event {
    pub host: &'static dyn Adapter,
    pub tool: String,
    pub file_paths: Vec<String>,
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

impl Decision {
    /// The decision that wins over a set: a deny anywhere, otherwise the first ask, otherwise
    /// allow. Section 9.4.
    pub fn strictest(decisions: impl IntoIterator<Item = Decision>) -> Decision {
        let mut asked = None;
        for decision in decisions {
            match decision {
                Decision::Deny(_) => return decision,
                Decision::Ask(_) => {
                    asked.get_or_insert(decision);
                }
                Decision::Allow => {}
            }
        }
        asked.unwrap_or(Decision::Allow)
    }
}

pub enum Stop {
    Pass,
    Block,
}

/// The event on stdin, or `None` when there is nothing to read or the text is not JSON. A
/// caller that gets `None` must let the turn through: klin says nothing about what it cannot read.
pub fn read(flag: Option<&str>) -> Option<Event> {
    let payload = payload()?;
    Some(placed(flag, &payload).event(&payload))
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

fn placed(flag: Option<&str>, payload: &Value) -> &'static dyn Adapter {
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

/// Every host klin reads blocks a stop the same way, with exit 2 and the report on stderr.
pub fn stop(stop: &Stop) -> u8 {
    match stop {
        Stop::Block => 2,
        Stop::Pass => 0,
    }
}

/// A refusal on any host: the reason on stderr under exit 2, which every host reads back to
/// the agent.
fn refused(reason: &str) -> u8 {
    eprintln!("{reason}");
    2
}

fn text(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// One string field of the event's `tool_input`, or empty.
fn input(payload: &Value, key: &str) -> String {
    text(payload.get("tool_input").and_then(|input| input.get(key)))
}

fn flag(payload: &Value, key: &str) -> bool {
    payload.get(key).and_then(Value::as_bool).unwrap_or(false)
}
