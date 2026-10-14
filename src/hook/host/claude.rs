use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::hook::rewrite::{Replacement, Rewrite};

use super::adapter::{
    Adapter, Decision, Event, flag, input, plugin_named_klin, referred, refused, text,
};

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
        super::adapter::CLAUDE_CODE_AND_CODEX_MATCHER
    }

    /// Claude Code turns plugin auto-update off for a marketplace that is not its own, so the
    /// step also names the switch.
    fn plugin_update(&self, _tag: &str) -> String {
        "In Claude Code, run /plugin, open Marketplaces, select klin and choose Update \
         marketplace. Enable auto-update there to get the next release on its own."
            .to_string()
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
        let tool = text(payload.get("tool_name"));
        let rewrites = match (path.is_empty(), self.asks) {
            (true, _) => Vec::new(),
            (false, true) => vec![(path.clone(), rewrite(&tool, payload))],
            (false, false) => vec![(path.clone(), Rewrite::Unread)],
        };
        Event {
            tool,
            file_paths: rewrites.iter().map(|(path, _)| path.clone()).collect(),
            rewrites,
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
            Decision::Refer { reason, person } if self.asks => referred(reason, person),
            Decision::Deny(reason) | Decision::Refer { reason, .. } => refused(reason),
        }
    }
}

/// What a `Write`, an `Edit` or a `MultiEdit` writes, from the fields Claude Code sends for it.
/// Spec 10.8.2.
fn rewrite(tool: &str, payload: &Value) -> Rewrite {
    let input = payload.get("tool_input");
    let edits = match tool {
        "Write" => {
            return input
                .and_then(|input| input.get("content"))
                .and_then(Value::as_str)
                .map_or(Rewrite::Unread, |text| Rewrite::Whole(text.to_string()));
        }
        "Edit" => input.map(std::slice::from_ref),
        "MultiEdit" => input
            .and_then(|input| input.get("edits"))
            .and_then(Value::as_array)
            .map(Vec::as_slice),
        _ => None,
    };
    edits
        .and_then(|edits| edits.iter().map(replacement).collect::<Option<Vec<_>>>())
        .map_or(Rewrite::Unread, Rewrite::Replaced)
}

fn replacement(edit: &Value) -> Option<Replacement> {
    let field = |key: &str| edit.get(key).and_then(Value::as_str).map(str::to_string);
    Some(Replacement {
        old: field("old_string")?,
        new: field("new_string")?,
        all: edit
            .get("replace_all")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
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
