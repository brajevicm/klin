use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::hook::rewrite::{Chunk, Rewrite};

use super::adapter::{
    Adapter, Decision, Event, flag, input, plugin_named_klin, referred, refused, text,
};

/// The field Codex CLI adds to every turn-scoped event and Claude Code never sends.
/// `permission_mode` is not it: both hosts send that one.
const TURN: &str = "turn_id";
const PATCH: &str = "apply_patch";
const END_OF_FILE: &str = "*** End of File";
const END_OF_PATCH: &str = "*** End Patch";

/// The file Codex lists its enabled plugins in, beside the hook file. Each plugin is a table
/// named `[plugins."name@marketplace"]`, on by default and off under `enabled = false`.
const CONFIG: &str = ".codex/config.toml";

pub struct Codex;

impl Adapter for Codex {
    /// `codex plugin marketplace upgrade` refetches the marketplace and replaces an installed
    /// plugin with the release the marketplace now pins. Codex has no plugin auto-update.
    fn plugin_update(&self, _tag: &str) -> String {
        "Run codex plugin marketplace upgrade klin, and start a new Codex session.".to_string()
    }

    fn name(&self) -> &'static str {
        "codex"
    }

    fn marker(&self) -> &'static str {
        ".codex"
    }

    fn hook_file(&self) -> &'static str {
        ".codex/hooks.json"
    }

    /// The shared matcher also names Claude Code's tools; Codex never emits those alternatives,
    /// so they are dead text here while the two install routes stay in step.
    fn matcher(&self) -> &'static str {
        super::adapter::CLAUDE_CODE_AND_CODEX_MATCHER
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
        let (rewrites, command) = match tool == PATCH {
            true => (patch_rewrites(&command), String::new()),
            false => (Vec::new(), command),
        };
        Event {
            tool,
            file_paths: rewrites.iter().map(|(path, _)| path.clone()).collect(),
            rewrites,
            command,
            blocked_before: flag(payload, "stop_hook_active"),
            session: text(payload.get("session_id")),
            prompt: text(payload.get("prompt")),
            ..Event::of(self)
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
            Decision::Refer { reason, person } => referred(reason, person),
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

/// Each file a patch names, with what it writes there. Codex finds a header after it trims the
/// line, and so does klin. A file the patch deletes or moves, and
/// a section klin cannot read, is unread. Spec 10.8.2.
fn patch_rewrites(patch: &str) -> Vec<(String, Rewrite)> {
    let mut sections: Vec<(String, Rewrite)> = Vec::new();
    for line in patch.lines() {
        let Some((header, path)) = header(line) else {
            if let Some((_, rewrite)) = sections.last_mut() {
                read_body(rewrite, line);
            }
            continue;
        };
        if header == Header::Move
            && let Some((_, moved)) = sections.last_mut()
        {
            *moved = Rewrite::Unread;
        }
        if !path.is_empty() {
            sections.push((path.to_string(), header.opened()));
        }
    }
    sections
}

/// A header of the patch language that names a file. The text below it is data.
#[derive(Clone, Copy, PartialEq)]
enum Header {
    Add,
    Delete,
    Update,
    Move,
}

impl Header {
    const ALL: [Header; 4] = [Header::Add, Header::Delete, Header::Update, Header::Move];

    fn prefix(self) -> &'static str {
        match self {
            Header::Add => "*** Add File: ",
            Header::Delete => "*** Delete File: ",
            Header::Update => "*** Update File: ",
            Header::Move => "*** Move to: ",
        }
    }

    /// What the section under this header writes, before klin reads its body. klin cannot
    /// read a file that is deleted or moved.
    fn opened(self) -> Rewrite {
        match self {
            Header::Add => Rewrite::Whole(String::new()),
            Header::Update => Rewrite::Patched(Vec::new()),
            Header::Delete | Header::Move => Rewrite::Unread,
        }
    }
}

/// The header a line is, after Codex's trim, and the path it names.
fn header(line: &str) -> Option<(Header, &str)> {
    Header::ALL.into_iter().find_map(|header| {
        let path = line.trim().strip_prefix(header.prefix())?;
        Some((header, path.trim()))
    })
}

/// One line under a section's header, in the patch language: `+` lines of a file it adds, and
/// `@@` chunks of context, `-` and `+` lines of a file it updates. Codex reads a `***` marker
/// after it trims the line, so a marker klin does not know, and anything else it cannot read,
/// leaves the section unread.
fn read_body(rewrite: &mut Rewrite, line: &str) {
    let marker = line.trim();
    if marker == END_OF_PATCH {
        return;
    }
    if marker.starts_with("***") && marker != END_OF_FILE {
        *rewrite = Rewrite::Unread;
        return;
    }
    let read = match rewrite {
        Rewrite::Whole(text) => line.strip_prefix('+').map(|added| {
            text.push_str(added);
            text.push('\n');
        }),
        Rewrite::Patched(chunks) => chunk_line(chunks, line),
        _ => Some(()),
    };
    if read.is_none() {
        *rewrite = Rewrite::Unread;
    }
}

fn chunk_line(chunks: &mut Vec<Chunk>, line: &str) -> Option<()> {
    if line.trim() == END_OF_FILE {
        chunks.last_mut()?.end = true;
        return Some(());
    }
    if let Some(marker) = line.strip_prefix("@@") {
        chunks.push(Chunk {
            anchor: anchor(marker)?,
            ..Chunk::default()
        });
        return Some(());
    }
    if chunks.is_empty() {
        chunks.push(Chunk::default());
    }
    changed_line(chunks.last_mut()?, line)
}

/// The line a `@@` marker names for its chunk to seek first: none for a bare `@@`, and the text
/// after `@@ ` otherwise. Any other marker is unread.
fn anchor(marker: &str) -> Option<Option<String>> {
    match marker {
        "" => Some(None),
        marker => marker.strip_prefix(' ').map(|line| Some(line.to_string())),
    }
}

/// A line of context, a line the chunk removes, or a line it adds. An empty line is context.
fn changed_line(chunk: &mut Chunk, line: &str) -> Option<()> {
    let mut chars = line.chars();
    let (sign, text) = (chars.next(), chars.as_str().to_string());
    match sign {
        None | Some(' ') => {
            chunk.old.push(text.clone());
            chunk.new.push(text);
        }
        Some('-') => chunk.old.push(text),
        Some('+') => chunk.new.push(text),
        Some(_) => return None,
    }
    Some(())
}
