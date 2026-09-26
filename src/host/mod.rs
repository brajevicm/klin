mod claude;
mod codex;
mod cursor;
mod generic;

use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

/// One line `klin install` writes for a host event. The adapter owns the event names and what
/// each one filters by, so a fourth host adds a table rather than a branch in the writer.
/// Section 19.3.
pub struct Hook {
    pub event: &'static str,
    pub arguments: &'static str,
    pub filter: Filter,
}

/// What one entry runs on: every call the event carries, or the tools the guard reads. An event
/// whose whole subject is a shell command or an MCP call filters by nothing, because the host
/// matches a tool name there and would match none. Section 19.3.
pub enum Filter {
    Every,
    Tools,
}

/// How a host stores those lines. Claude Code and Codex nest a `hooks` array; Cursor's file is
/// a flat list under `version: 1`.
pub enum HookFile {
    Nested,
    Flat { version: u64 },
}

/// The fields that name one Claude Code or Codex event, the same in every copy of klin's hooks
/// the host runs for it. Claude Code names the prompt `prompt_id` and Codex names the turn
/// `turn_id`; a stop under one prompt differs from the stop before it in `stop_hook_active` or
/// the message it ends on. Measured on Claude Code 2.1.283 and Codex 0.157.1. Spec 9.8.
const CLAUDE_CODE_AND_CODEX_IDENTITY: &[&str] = &[
    "session_id",
    "hook_event_name",
    "prompt_id",
    "turn_id",
    "tool_use_id",
    "source",
    "stop_hook_active",
    "last_assistant_message",
];

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

/// The shared plugin hook and both generated routes must cover every tool either host emits.
const CLAUDE_CODE_AND_CODEX_MATCHER: &str =
    "Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*";

const DEFAULT_HOOKS: &[Hook] = &[
    Hook {
        event: "SessionStart",
        arguments: "radius",
        filter: Filter::Every,
    },
    Hook {
        event: "UserPromptSubmit",
        arguments: "radius",
        filter: Filter::Every,
    },
    Hook {
        event: "PreToolUse",
        arguments: "guard",
        filter: Filter::Tools,
    },
    Hook {
        event: "Stop",
        arguments: "gate --hook --changed",
        filter: Filter::Every,
    },
];

/// One host klin speaks to. A port is one module that implements this and one entry in
/// `ADAPTERS`, and nothing else in the crate names the host. Section 9.1.
pub trait Adapter: Sync {
    /// The name `--host` takes.
    fn name(&self) -> &'static str;
    /// The directory whose presence says a tree uses this host.
    fn marker(&self) -> &'static str;
    /// The file the host reads its hooks from, relative to a tree root or to the home directory.
    fn hook_file(&self) -> &'static str;
    /// The skill path the standalone route writes, relative to the same scope.
    fn skill_file(&self) -> &'static str {
        ".agents/skills/klin/SKILL.md"
    }
    /// The tools the guard reads on the pre-tool event, in the host's matcher syntax.
    fn matcher(&self) -> &'static str;
    /// The host's name for the event a person's prompt raises, which is the one event that
    /// carries the spread report. It is the last `radius` line in the hook table: session start
    /// is also radius, and comes first. Spec 9.2.
    fn prompt_event(&self) -> &'static str {
        self.hooks()
            .iter()
            .rev()
            .find(|hook| hook.arguments == "radius")
            .map(|hook| hook.event)
            .unwrap_or("")
    }
    /// The events `klin install` writes. Claude Code and Codex share the table; Cursor names
    /// its own events.
    fn hooks(&self) -> &'static [Hook] {
        DEFAULT_HOOKS
    }
    /// The shape of the file those events live in.
    fn hook_file_kind(&self) -> HookFile {
        HookFile::Nested
    }
    /// The settings file that enables klin's plugin for a write into `root`, or into the home
    /// directory when `shared`. The plugin carries the hooks itself, so on this machine one of
    /// the two copies yields on each event. Section 19.3, 9.8.
    fn plugin_enabled(&self, root: &Path, shared: bool) -> Option<PathBuf>;
    /// What an install says when `proof` shows klin's plugin also carries the hooks.
    fn plugin_serves(&self, proof: &Path) -> String {
        format!(
            "the klin plugin, which {} enables, also serves this host",
            proof.display()
        )
    }
    /// The host's own name for the event this payload is. Every host klin maintains sends it
    /// under `hook_event_name`; the harness protocol of 9.7 names its own field.
    fn event_name(&self, payload: &Value) -> String {
        text(payload.get("hook_event_name"))
    }
    /// The payload fields whose values name one event. Spec 9.8.
    fn identity_fields(&self) -> &'static [&'static str] {
        CLAUDE_CODE_AND_CODEX_IDENTITY
    }
    /// Whether an event with no `--host` has this host's shape.
    fn placed(&self, payload: &Value) -> bool;
    /// The tree the event names, for a host that runs its hooks somewhere else. A host that
    /// runs them in the tree names none, and klin reads the working directory. Section 9.1.
    fn root(&self, _payload: &Value) -> Option<PathBuf> {
        None
    }
    /// The event in klin's own words.
    fn event(&'static self, payload: &Value) -> Event;
    /// The guard's decision in the host's shape, and the exit code that carries it.
    fn decide(&self, decision: &Decision) -> u8;
    /// The stop answer in the host's shape. Every host but Cursor uses exit 2 to block and a
    /// JSON `systemMessage` to tell the person. The block carries the report for a host whose
    /// stop cannot read stderr.
    fn stop(&self, stop: &Stop) -> u8 {
        emit(stop)
    }
    /// The exit code a stop this host blocks ends with. The block itself is the report the
    /// adapter delivered; the code is only the host's protocol for it. Spec 9.1.
    fn block_exit(&self) -> u8 {
        2
    }
    /// Whether a blocked stop submits its report as another prompt. The caller records the
    /// exact report before delivery, so the matching prompt does not open a fresh turn.
    fn follows_up(&self) -> bool {
        false
    }
}

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

/// One host event, in klin's own words.
pub struct Event {
    pub host: &'static dyn Adapter,
    /// The tree the payload names, when its hook may run elsewhere.
    pub root: Option<PathBuf>,
    /// Whether this is the host's prompt event.
    pub prompted: bool,
    pub tool: String,
    pub file_paths: Vec<String>,
    pub command: String,
    pub blocked_before: bool,
    /// Whether the host says this stop follows a message it submitted by itself, which Cursor
    /// says with a `loop_count` above 0. A stop that continues such a chain belongs to the prompt
    /// that opened it. Spec 9.3, ADR 0052.
    pub continued: bool,
    /// The host's grouping of many turns under one id, which klin records and never judges.
    /// Empty when the host sends none.
    pub session: String,
    /// The text of a `UserPromptSubmit` event. Empty for every other event and for a host that
    /// sends none. Spec 9.6.
    pub prompt: String,
    /// What names this event in every copy of klin's hooks the host runs for it, so one copy
    /// takes it and the others yield. Empty when the event names no session. Spec 9.8.
    pub identity: String,
}

impl Event {
    /// What every adapter starts from: the host, and the fields `read` fills after placement.
    pub fn of(host: &'static dyn Adapter) -> Event {
        Event {
            host,
            root: None,
            prompted: false,
            tool: String::new(),
            file_paths: Vec::new(),
            command: String::new(),
            blocked_before: false,
            continued: false,
            session: String::new(),
            prompt: String::new(),
            identity: String::new(),
        }
    }
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
    /// Block this stop. The text is the report; a host that already wrote it on stderr
    /// ignores it, and a host that cannot read stderr still has to show it.
    Block(String),
    /// Let the stop end, and put this text in front of the person.
    Tell(String),
}

/// The event on stdin, or `None` when there is nothing to read or the text is not JSON. A
/// caller that gets `None` must let the turn through: klin says nothing about what it cannot read.
pub fn read(flag: Option<&str>) -> Option<Event> {
    let payload = payload()?;
    let host = placed(flag, &payload);
    let name = host.event_name(&payload);
    let mut event = host.event(&payload);
    event.root = host.root(&payload);
    event.prompted = !name.is_empty() && host.prompt_event() == name;
    if !event.session.is_empty() && SCOPED.iter().any(|field| payload.get(field).is_some()) {
        event.identity = identity(&payload, host.identity_fields());
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

fn emit(stop: &Stop) -> u8 {
    match stop {
        Stop::Block(_) => 2,
        Stop::Pass => 0,
        Stop::Tell(text) => {
            println!("{}", serde_json::json!({ "systemMessage": text }));
            0
        }
    }
}

/// A refusal on any host: the reason on stderr under exit 2, which every host reads back to
/// the agent.
fn refused(reason: &str) -> u8 {
    eprintln!("{reason}");
    2
}

/// The name a host lists klin's plugin under: klin's own name, or that name and the
/// marketplace it came from. A plugin whose name only starts with klin's is another plugin.
fn plugin_named_klin(named: &str) -> bool {
    named == "klin"
        || named
            .strip_prefix("klin")
            .is_some_and(|rest| rest.starts_with('@'))
}

/// Each named field the payload holds, with its value, one to a line.
fn identity(payload: &Value, fields: &[&str]) -> String {
    fields
        .iter()
        .filter_map(|field| payload.get(field).map(|value| format!("{field}={value}\n")))
        .collect()
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
