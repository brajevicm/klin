use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{Adapter, Decision, Event, Hook, Stop, flag, refused, text};

/// The field that names an event klin's own, and the one protocol version klin speaks. No host
/// klin maintains sends the field, so it places the event before any of them. Spec 19.4.
pub const PROTOCOL: &str = "klin_protocol";
pub const VERSION: u64 = 1;

const NAME: &str = "harness";
/// The harness protocol's own name for the event a person's prompt raises.
const PROMPT_EVENT: &str = "prompt";

const UNSUPPORTED: &str = "klin: refused — this event names a version of klin's harness \
    protocol that this klin does not speak. Send version 1, or upgrade klin.";

/// A harness klin does not maintain, speaking klin's own versioned event shape rather than
/// another host's. `spoken` says whether the event names the version klin speaks: an event of
/// any other version is refused whole, and is never read as a host's. Spec 19.4.
pub struct Generic {
    pub spoken: bool,
}

const SPEAKS: &dyn Adapter = &Generic { spoken: true };
const REFUSES: &dyn Adapter = &Generic { spoken: false };

/// The adapter that answers this payload, or `None` when the payload carries no generic event
/// and `--host` named no generic one.
pub fn placed(named: Option<&str>, payload: &Value) -> Option<&'static dyn Adapter> {
    let held = payload.get(PROTOCOL);
    let mine = match named {
        Some(name) => name == NAME,
        None => held.is_some(),
    };
    if !mine {
        return None;
    }
    match held.and_then(Value::as_u64) == Some(VERSION) {
        true => Some(SPEAKS),
        false => Some(told(held)),
    }
}

/// An unknown version is named before it is refused, so an integrator reads which version klin
/// speaks rather than a bare refusal.
fn told(held: Option<&Value>) -> &'static dyn Adapter {
    let named = match held {
        Some(held) => format!("version {held}"),
        None => "no version".to_string(),
    };
    eprintln!(
        "klin: NOTE: this event names {named} of klin's harness protocol, and klin speaks version \
         {VERSION} — so klin refuses the event rather than guessing its shape."
    );
    REFUSES
}

impl Adapter for Generic {
    fn name(&self) -> &'static str {
        NAME
    }

    /// klin installs nothing for a harness it does not maintain: the integration owns its own
    /// hooks and its own skill placement, and `klin install` never reaches this adapter, so it
    /// names no marker, no hook file, no matcher and no plugin. Spec 19.4.
    fn marker(&self) -> &'static str {
        ""
    }

    fn hook_file(&self) -> &'static str {
        ""
    }

    fn matcher(&self) -> &'static str {
        ""
    }

    fn hooks(&self) -> &'static [Hook] {
        &[]
    }

    fn plugin_enabled(&self, _root: &Path, _shared: bool) -> Option<PathBuf> {
        None
    }

    fn prompt_event(&self) -> &'static str {
        PROMPT_EVENT
    }

    /// The harness protocol names its own event kinds under `event`. Every host klin maintains
    /// sends `hook_event_name`. A version klin does not speak names no event either, so a prompt
    /// klin refused to read moves no prompt counter and no mark. Spec 6.2.1, 9.7.
    fn event_name(&self, payload: &Value) -> String {
        match self.spoken {
            true => text(payload.get("event")),
            false => String::new(),
        }
    }

    fn placed(&self, payload: &Value) -> bool {
        payload.get(PROTOCOL).is_some()
    }

    /// The tree the harness names, because a harness may run its integration anywhere. A version
    /// klin does not speak names nothing, because klin does not know that shape.
    fn root(&self, payload: &Value) -> Option<PathBuf> {
        let named = match self.spoken {
            true => text(payload.get("root")),
            false => String::new(),
        };
        (!named.is_empty()).then(|| PathBuf::from(named))
    }

    /// Only the evidence the harness proves. `tool` is diagnostic: klin reads no write out of a
    /// tool name, so a call whose paths and command the harness cannot prove carries neither and
    /// is allowed. Spec 19.4.
    fn event(&'static self, payload: &Value) -> Event {
        if !self.spoken {
            return Event::of(self);
        }
        Event {
            tool: text(payload.get("tool")),
            file_paths: paths(payload),
            command: text(payload.get("command")),
            blocked_before: flag(payload, "blocked_before"),
            session: text(payload.get("session")),
            prompt: text(payload.get("prompt")),
            ..Event::of(self)
        }
    }

    /// A decision goes out as one JSON object on stdout, which the harness's own shim translates
    /// back into its host protocol. A refusal also exits 2 with the reason on stderr, so it holds
    /// where stdout goes unread — the same pairing as Cursor's. The contract carries no question:
    /// a custom harness proves no enforced question channel, so an ask fails closed as it does on
    /// Codex and Cursor. Spec 9.4, 19.4.
    fn decide(&self, decision: &Decision) -> u8 {
        if !self.spoken {
            return denied(UNSUPPORTED);
        }
        match decision {
            Decision::Allow => answered("allow", "", "", 0),
            Decision::Ask(reason) => denied(&format!(
                "klin: refused — {reason} A custom harness declares no question klin can rely \
                 on, so this call fails closed."
            )),
            Decision::Deny(reason) => denied(reason),
        }
    }

    /// A blocked stop carries the report in the answer itself, because a custom harness proves no
    /// stderr channel klin can rely on. A stop that passes prints no decision, as it does on every
    /// other host: exit 0 with no decision ends the turn. Spec 9.7.
    fn stop(&self, stop: &Stop) -> u8 {
        if !self.spoken {
            return answered("block", "message", UNSUPPORTED, refused(UNSUPPORTED));
        }
        match stop {
            Stop::Pass => 0,
            Stop::Block(said) => answered("block", "message", said, 2),
            Stop::Tell(said) => answered("tell", "message", said, 0),
        }
    }
}

fn paths(payload: &Value) -> Vec<String> {
    payload
        .get("file_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|path| text(Some(path)))
        .filter(|path| !path.is_empty())
        .collect()
}

fn denied(reason: &str) -> u8 {
    answered("deny", "reason", reason, refused(reason))
}

fn answered(action: &str, key: &str, said: &str, code: u8) -> u8 {
    let mut answer = Map::new();
    answer.insert("action".to_string(), action.into());
    if !key.is_empty() {
        answer.insert(key.to_string(), said.into());
    }
    println!("{}", Value::Object(answer));
    code
}
