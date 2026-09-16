# One plugin directory also serves Cursor

> Amends ADR 0030, whose decision named Claude Code and Codex CLI only.

ADR 0030 put Claude Code and Codex CLI on one plugin directory,
`plugins/claude-code`, with two marketplace files, one wrapper, one version pin.
Cursor was left on the other route: a binary on PATH and `klin init --hooks`.

Cursor's plugin format is not Claude Code's. It reads
`.cursor-plugin/plugin.json` and a flat `hooks.json` at schema version 1,
with its own event names (`sessionStart`, `beforeSubmitPrompt`, `preToolUse`,
`beforeShellExecution`, `beforeMCPExecution`, `stop`) and
`${CURSOR_PLUGIN_ROOT}`. Its events carry `cursor_version`. Its stop notice
is `followup_message`, not `systemMessage`. Reusing Claude Code's nested
hooks file would make Cursor parse the wrong shape. First-class native
packaging was chosen after issue #67 asked for a compatibility-first
investigation; that process constraint is superseded here, not reported as a
compatibility test that happened. The native format itself is the requirement.

## The decision

The same plugin directory serves Cursor, renamed to `plugins/klin`. Three
hosts read it, and the local install step is a path a person types, so a
directory named after one of them names the wrong host at the other two. The
rename moves the two version pins the release tooling rewrites, which is one
edit to `Cargo.toml`. A third marketplace file at
`.cursor-plugin/marketplace.json` points at it. A Cursor manifest beside
Claude Code's names `./hooks/cursor.json`, so Cursor does not discover
Claude Code's nested file. The wrapper, the skill, the commands and the
version pin stay one copy.

Cursor is a runtime adapter, registered after Codex and before Claude Code,
because its events also carry Claude Code's fields. It owns the install
file `.cursor/hooks.json`, the matcher, the decision shape, the stop
shape, the name of its prompt event, and the tree its events name.
`init --hooks --host cursor` writes that file when the plugin is not
already installed under `.cursor/plugins`.

Two of Cursor's fields decide behaviour no other host has. Cursor runs a
user-scope hook from `~/.cursor`, so the working directory is not the tree;
the adapter names the tree from `cwd`, then from `workspace_roots`, and every
command a hook runs measures that tree: the guard, the hook-mode gate, and the
`radius` that opens the turn window. And the top-level `command` is
the agent's only on `beforeShellExecution`; on `beforeMCPExecution` it is the
server's launch line, so reading it there would refuse an MCP call over a
command nobody ran.

Two names moved out of shared modules into the adapter as a result: the
prompt event, which `turn` had as the literal `UserPromptSubmit`, and the
stop channel, which `gate` had as Claude Code's `systemMessage`. A Cursor
block uses `followup_message` and still exits 2, matching a deny. Cursor
submits that follow-up as the next prompt, so klin records a hash of the
exact report before delivery. The matching prompt consumes that record and
does not refresh the block budget. A different prompt, including a person's
prompt beginning with `klin:`, opens a turn normally.

Cursor 3.20.21 accepted `permission: ask` on `beforeShellExecution` but
proceeded. A question that does not hold is not klin's `ask`; the adapter
therefore returns `deny` and exit 2 for that decision, as Codex does where it
has no question channel.

Measured host facts for that build live in
[`docs/cursor-compatibility.md`](../cursor-compatibility.md). Rows that record
klin's own follow-up hash or its gate-spent bound are implementation, not host
measurement; the matrix marks which of those are still unmeasured.

## Consequences

A Cursor user installs klin the way a Claude Code or Codex user does: add
the GitHub repository as a marketplace (or copy the plugin directory into
`~/.cursor/plugins/local/klin`) and enable the plugin. The official Cursor
Marketplace remains a review queue klin does not control. The Team Marketplace
import route is documented in the README and has not been recorded in the
compatibility matrix.

A fourth host still adds one module and one marketplace file, not a second
plugin tree.

Plugin detection covers Cursor's documented local tree and the marketplace
cache layout observed under 3.20.21. Both are bounded searches under
`.cursor/plugins`; malformed or unrelated manifests do not count.
