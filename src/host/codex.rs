use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Adapter, Decision, Event, flag, input, plugin_named_klin, refused, text};

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

/// The file Codex lists its enabled plugins in, beside the hook file. Each plugin is a table
/// named `[plugins."name@marketplace"]`, on by default and off under `enabled = false`.
const CONFIG: &str = ".codex/config.toml";

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

    /// A write into a tree is covered by the tree's config and the user's. A write into the
    /// home directory is covered by the user's alone.
    fn plugin_enabled(&self, root: &Path, shared: bool) -> Option<PathBuf> {
        let mut looked = Vec::new();
        if !shared {
            looked.push(root.join(CONFIG));
        }
        looked.extend(std::env::home_dir().map(|home| home.join(CONFIG)));
        looked.into_iter().find(|config| lists_klin(config))
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

/// Whether the config holds a klin plugin table that is not switched off. A line scan, the way
/// the lockfile check reads `Cargo.toml`: a `[plugins."klin@..."]` header opens the table, and
/// the table runs to the next header.
fn lists_klin(config: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(config) else {
        return false;
    };
    let mut in_klin = false;
    let mut on = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            on |= in_klin;
            in_klin = plugin_table(line).is_some_and(plugin_named_klin);
        } else if in_klin && disabled(line) {
            in_klin = false;
        }
    }
    on || in_klin
}

/// The plugin a `[plugins.NAME]` header names, with its quotes removed.
fn plugin_table(header: &str) -> Option<&str> {
    let inner = header.strip_prefix('[')?.strip_suffix(']')?.trim();
    let named = inner.strip_prefix("plugins.")?.trim();
    Some(named.trim_matches(['"', '\'']))
}

fn disabled(line: &str) -> bool {
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    key.trim() == "enabled" && value.trim().starts_with("false")
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
