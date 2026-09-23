use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{
    Adapter, Decision, Event, Filter, Hook, HookFile, Stop, input, plugin_named_klin, refused, text,
};

/// Cursor's native events, not Claude Code's translated ones. `cursor_version` is the field
/// Claude Code never sends, and Cursor sends it on every request.
const VERSION: &str = "cursor_version";
/// The one event that carries a command the agent is about to run at the top level.
const SHELL_EVENT: &str = "beforeShellExecution";

pub struct Cursor;

impl Adapter for Cursor {
    fn name(&self) -> &'static str {
        "cursor"
    }

    fn marker(&self) -> &'static str {
        ".cursor"
    }

    fn hook_file(&self) -> &'static str {
        ".cursor/hooks.json"
    }

    fn matcher(&self) -> &'static str {
        "Write|Edit|Delete"
    }

    /// Cursor splits the guard over three events. Only `preToolUse` carries a tool name, so it
    /// is the one that filters by the tools of `matcher`; a matcher on the shell or MCP event
    /// would match no tool name and gate nothing. Section 19.3.
    fn hooks(&self) -> &'static [Hook] {
        &[
            Hook {
                event: "sessionStart",
                arguments: "radius",
                filter: Filter::Every,
            },
            Hook {
                event: "beforeSubmitPrompt",
                arguments: "radius",
                filter: Filter::Every,
            },
            Hook {
                event: "preToolUse",
                arguments: "guard",
                filter: Filter::Tools,
            },
            Hook {
                event: "beforeShellExecution",
                arguments: "guard",
                filter: Filter::Every,
            },
            Hook {
                event: "beforeMCPExecution",
                arguments: "guard",
                filter: Filter::Every,
            },
            Hook {
                event: "stop",
                arguments: "gate --hook --changed",
                filter: Filter::Every,
            },
        ]
    }

    fn hook_file_kind(&self) -> HookFile {
        HookFile::Flat { version: 1 }
    }

    /// A write into a tree is covered by a plugin this tree or the user already holds. A write
    /// into the home directory is covered by the user's plugins alone.
    fn plugin_enabled(&self, root: &Path, shared: bool) -> Option<PathBuf> {
        let mut looked = Vec::new();
        if !shared {
            looked.push(root.join(".cursor/plugins"));
        }
        looked.extend(std::env::home_dir().map(|home| home.join(".cursor/plugins")));
        looked.into_iter().find_map(|dir| finds_klin(&dir))
    }

    fn placed(&self, payload: &Value) -> bool {
        payload.get(VERSION).is_some()
    }

    /// Cursor runs a project hook from the workspace root and a user hook from `~/.cursor`, so
    /// the working directory is not the tree on a user-scope install. Every request carries
    /// `workspace_roots`, and a tool event carries the `cwd` a relative path in a command stands
    /// on, which is the closer of the two. Section 9.1.
    fn root(&self, payload: &Value) -> Option<PathBuf> {
        let named = match text(payload.get("cwd")) {
            cwd if !cwd.is_empty() => cwd,
            _ => text(
                payload
                    .get("workspace_roots")
                    .and_then(Value::as_array)
                    .and_then(|roots| roots.first()),
            ),
        };
        (!named.is_empty()).then(|| PathBuf::from(named))
    }

    /// Cursor sends no flag for a stop it already blocked. Its `loop_count` counts the automatic
    /// follow-ups before this stop, not the blocks this turn spent, so `blocked_before` is false
    /// here and klin's own record bounds the block. A count above 0 says the stop continues a
    /// chain of automatic messages, which is what `continued` records. Spec 9.3, ADR 0022, 0052.
    fn event(&'static self, payload: &Value) -> Event {
        let path = path(payload);
        Event {
            tool: tool(payload),
            file_paths: Vec::from_iter((!path.is_empty()).then_some(path)),
            command: command(payload),
            blocked_before: false,
            continued: payload
                .get("loop_count")
                .and_then(Value::as_u64)
                .is_some_and(|count| count > 0),
            session: session(payload),
            prompt: text(payload.get("prompt")),
            ..Event::of(self)
        }
    }

    /// Cursor reads `permission` on the pre-tool, shell and MCP events. An allow is exit 0 with
    /// no stdout, matching Claude Code and Codex, so a user-scope plugin does not speak in a tree
    /// that never wrote `klin.json`. Its documented `ask` was not enforced by 3.20.21, so klin
    /// fails that answer closed like Codex does. A refusal carries its reason in `agent_message`
    /// and still exits 2, so it holds even where stdout goes unread. Section 9.1.
    fn decide(&self, decision: &Decision) -> u8 {
        match decision {
            Decision::Allow => 0,
            Decision::Ask(reason) => deny(&format!(
                "klin: refused — {reason} Cursor did not enforce a question on this event."
            )),
            Decision::Deny(reason) => deny(reason),
        }
    }

    /// A stop that tells the person uses `followup_message`, which Cursor submits as the next
    /// prompt. A block uses that field too, because Cursor's stop has no other channel, and it
    /// exits 0: Cursor 3.21.18 did not submit the follow-up of a stop hook that exited 2, and
    /// did submit one from a hook that exited 0. Nothing enforces an exit-0 block, so a Cursor
    /// that ignored stdout would let the turn end, which fails open. Spec 9.1.
    fn stop(&self, stop: &Stop) -> u8 {
        match stop {
            Stop::Block(text) => {
                followup(text);
                self.block_exit()
            }
            Stop::Pass => 0,
            Stop::Tell(text) => {
                followup(text);
                0
            }
        }
    }

    fn follows_up(&self) -> bool {
        true
    }

    fn block_exit(&self) -> u8 {
        0
    }
}

/// A plugin Cursor already holds under the documented local tree or the observed marketplace
/// cache tree. Marketplace roots are four directories below `plugins`: cache, marketplace,
/// plugin and revision.
fn finds_klin(dir: &Path) -> Option<PathBuf> {
    find_manifest(dir, 4)
}

fn find_manifest(dir: &Path, depth: usize) -> Option<PathBuf> {
    let manifest = dir.join(".cursor-plugin/plugin.json");
    if names_klin(&manifest) {
        return Some(manifest);
    }
    if depth == 0 {
        return None;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
    entries.sort();
    entries
        .into_iter()
        .filter(|path| path.is_dir())
        .find_map(|path| find_manifest(&path, depth - 1))
}

fn names_klin(manifest: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return false;
    };
    let Ok(held) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    held.get("name")
        .and_then(Value::as_str)
        .is_some_and(plugin_named_klin)
}

fn followup(text: &str) {
    println!("{}", serde_json::json!({ "followup_message": text }));
}

fn deny(reason: &str) -> u8 {
    println!(
        "{}",
        serde_json::json!({ "permission": "deny", "agent_message": reason })
    );
    refused(reason)
}

fn tool(payload: &Value) -> String {
    let shell = if shell_event(payload) {
        "Shell".to_string()
    } else {
        String::new()
    };
    first_nonempty(text(payload.get("tool_name")), shell)
}

fn path(payload: &Value) -> String {
    first_nonempty(input(payload, "file_path"), text(payload.get("file_path")))
}

/// The shell command the event carries. `beforeShellExecution` puts it at the top level, and
/// every other event puts what it has under `tool_input`. An MCP event carries the server's own
/// launch command at the top level, which the agent did not run, so only the shell event is read
/// there. Section 9.1.
fn command(payload: &Value) -> String {
    let top = if shell_event(payload) {
        text(payload.get("command"))
    } else {
        String::new()
    };
    first_nonempty(input(payload, "command"), top)
}

fn shell_event(payload: &Value) -> bool {
    text(payload.get("hook_event_name")) == SHELL_EVENT
}

fn first_nonempty(preferred: String, fallback: String) -> String {
    match preferred.is_empty() {
        true => fallback,
        false => preferred,
    }
}

fn session(payload: &Value) -> String {
    first_nonempty(
        text(payload.get("conversation_id")),
        text(payload.get("session_id")),
    )
}
