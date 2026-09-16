# Cursor hook compatibility

Verified with Cursor 3.20.21 on 2026-09-16 on macOS.

The native plugin is the install (ADR 0045). These notes record what that
Cursor build did with hook output, so a later adapter change can compare
against a measured host rather than a guess.

| Behaviour | Native plugin | Notes |
|---|---|---|
| edit deny | `preToolUse` matcher `Write\|Edit\|Delete`; `permission: deny` and exit 2 | the exit code holds if stdout goes unread |
| shell deny | `beforeShellExecution`; `permission: deny` and exit 2 | `command` is at the top level |
| shell ask | fails closed as `permission: deny` and exit 2 | 3.20.21 accepted `ask` but proceeded |
| MCP | `beforeMCPExecution`; the top-level `command` is the server's launch line, not the agent's | reading it there would refuse an MCP call over a command nobody ran |
| session/prompt radius | `sessionStart`, `beforeSubmitPrompt` | each host names its own prompt event |
| the tree | `cwd`, then the first `workspace_roots` entry | a user-scope hook runs from `~/.cursor`, not the workspace |
| shell/MCP matcher | none written | those events name no tool, so a tool matcher would gate nothing |
| Stop block | `followup_message` and exit 2 | stderr still holds the report; native stop has no other channel |
| Stop note | `followup_message` | not `systemMessage` |
| klin's own follow-up | exact report hash recorded and consumed once | a person's different prompt always opens a turn |
| repeated follow-up bound | klin's gate-spent record; `loop_count` ignored | conversation-wide counter, and Cursor sends no `stop_hook_active` |
| no `klin.json` silence | hook-mode CLI fixture exits 0 with no output | ADR 0028 |

Issue #67 originally required the Claude-compatible hook route to be tested
first. That route was not tested. The later product decision requires a
first-class native Cursor plugin, so ADR 0045 explicitly supersedes that
process constraint instead of presenting this native matrix as compatibility
evidence.

Plugin detection covers the documented local layout
`plugins/local/<name>` and the marketplace cache layout observed in 3.20.21,
`plugins/cache/<marketplace>/<plugin>/<revision>`. CLI fixtures cover both.

One thing is still best-effort: the stop hook's `klin is not installed` notice, which
tests `klin.json` beside the working directory, because there is no binary to
read `workspace_roots` with. On a user-scope install that check finds nothing
and the notice stays quiet. Every route that has a binary reads the tree from
the event.

The field names come from Cursor's hooks reference: every request carries
`hook_event_name`, `cursor_version`, `conversation_id` and `workspace_roots`;
a tool event adds `cwd`; `beforeShellExecution` adds `command`; `stop` carries
`status` and `loop_count` and nothing about a block this turn already spent.

Cursor 3.20.21 accepted `permission: ask` on `beforeShellExecution` and still
proceeded under one verified configuration. That is a host enforcement gap,
so klin refuses the operation until Cursor provides an enforced question
channel.
