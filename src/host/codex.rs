use serde_json::Value;

use super::{Adapter, Decision, Event, flag, input, refused, text};

/// The field Codex CLI adds to every turn-scoped event and Claude Code never sends.
/// `permission_mode` is not it: both hosts send that one.
const TURN: &str = "turn_id";
const PATCH: &str = "apply_patch";
/// The headers of the patch language that name a file. The text below them is data.
const HEADERS: &[&str] = &[
    "*** Add File: ",
    "*** Delete File: ",
    "*** Update File: ",
    "*** Move to: ",
];

pub struct Codex;

impl Adapter for Codex {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn marker(&self) -> &'static str {
        ".codex"
    }

    fn hook_file(&self) -> &'static str {
        ".codex/hooks.json"
    }

    /// `Bash` is the shell tool's name on stdin, `apply_patch` the edit tool's, and an MCP tool
    /// is `mcp__server__tool`.
    fn matcher(&self) -> &'static str {
        "Bash|apply_patch|mcp__.*"
    }

    fn placed(&self, payload: &Value) -> bool {
        payload.get(TURN).is_some()
    }

    /// A shell or MCP call carries what it carries in `tool_input.command`. A patch carries its
    /// file paths in its headers, and the patch text is not a command.
    fn event(&'static self, payload: &Value) -> Event {
        let tool = text(payload.get("tool_name"));
        let command = input(payload, "command");
        let (file_paths, command) = match tool == PATCH {
            true => (patch_paths(&command), String::new()),
            false => (Vec::new(), command),
        };
        Event {
            host: self,
            tool,
            file_paths,
            command,
            blocked_before: flag(payload, "stop_hook_active"),
        }
    }

    /// Codex rejects `permissionDecision: ask` on this event as unsupported, and reads a refusal
    /// from exit 2 with the reason on stderr. So both a deny and an ask are refusals here.
    fn decide(&self, decision: &Decision) -> u8 {
        match decision {
            Decision::Allow => 0,
            Decision::Ask(reason) => refused(&format!(
                "klin: refused — {reason} Codex CLI has no question to ask on this event."
            )),
            Decision::Deny(reason) => refused(reason),
        }
    }
}

fn patch_paths(patch: &str) -> Vec<String> {
    patch
        .lines()
        .filter_map(|line| HEADERS.iter().find_map(|header| line.strip_prefix(header)))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}
