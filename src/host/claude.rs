use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Adapter, Decision, Event, flag, input, plugin_named_klin, refused, text};

/// A field only Claude Code sends. One of them is enough to place the event.
const FIELDS: &[&str] = &[
    "hook_event_name",
    "tool_name",
    "tool_input",
    "session_id",
    "stop_hook_active",
];

/// Claude Code. `asks` says whether the host is credited with the `ask` decision: the placed
/// host is, and the unplaced host that is read in Claude's shape is not.
pub struct Claude {
    pub asks: bool,
}

impl Adapter for Claude {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn marker(&self) -> &'static str {
        ".claude"
    }

    fn hook_file(&self) -> &'static str {
        ".claude/settings.json"
    }

    fn skill_file(&self) -> &'static str {
        ".claude/skills/klin/SKILL.md"
    }

    fn matcher(&self) -> &'static str {
        super::CLAUDE_CODE_AND_CODEX_MATCHER
    }

    /// A write into a tree is covered by that tree's settings, the local settings beside them
    /// and the user's. A write into the home directory is covered by the user's alone, because
    /// a plugin one repository enables gates that repository and not the machine.
    fn plugin_enabled(&self, root: &Path, shared: bool) -> Option<PathBuf> {
        let mut looked = Vec::new();
        if !shared {
            looked.push(root.join(self.hook_file()));
            looked.push(root.join(".claude/settings.local.json"));
        }
        looked.extend(std::env::home_dir().map(|home| home.join(self.hook_file())));
        looked.into_iter().find(|settings| lists_klin(settings))
    }

    fn placed(&self, payload: &Value) -> bool {
        FIELDS.iter().any(|field| payload.get(field).is_some())
    }

    fn event(&'static self, payload: &Value) -> Event {
        let path = match input(payload, "file_path") {
            path if path.is_empty() => input(payload, "notebook_path"),
            path => path,
        };
        Event {
            tool: text(payload.get("tool_name")),
            file_paths: Vec::from_iter((!path.is_empty()).then_some(path)),
            command: input(payload, "command"),
            blocked_before: flag(payload, "stop_hook_active"),
            session: text(payload.get("session_id")),
            prompt: text(payload.get("prompt")),
            ..Event::of(self)
        }
    }

    /// A pre-tool decision goes out as `hookSpecificOutput.permissionDecision` on stdout under
    /// exit 0, and a refusal as exit 2 with the reason on stderr. Section 9.1. A host klin
    /// cannot ask has no third answer, so the question is refused, and it is worded as a
    /// refusal: an agent that is blocked cannot act on advice to let a person decide.
    fn decide(&self, decision: &Decision) -> u8 {
        match decision {
            Decision::Allow => 0,
            Decision::Ask(reason) if self.asks => {
                println!("{}", ask(&format!("klin: ask — {reason}")));
                0
            }
            Decision::Ask(reason) => refused(&format!(
                "klin: refused — {reason} This host has no question to ask."
            )),
            Decision::Deny(reason) => refused(reason),
        }
    }
}

/// Claude Code lists its enabled plugins under `enabledPlugins` in the settings file.
fn lists_klin(settings: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(settings) else {
        return false;
    };
    let Ok(held) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    held.get("enabledPlugins")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .any(|(named, on)| plugin_named_klin(named) && on.as_bool().unwrap_or(false))
}

fn ask(reason: &str) -> String {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": reason,
        }
    })
    .to_string()
}
