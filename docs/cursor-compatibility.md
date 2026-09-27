# Cursor hook compatibility

Verified with Cursor 3.20.21 on 2026-09-16 on macOS.

The native plugin carries klin's hooks for Cursor (ADR 0045, ADR 0053). These notes record what that
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
| Stop block | `followup_message` and exit 0 | measured on 3.21.18 (below): a stop hook that exits 2 has its `followup_message` dropped. stderr still holds the report; native stop has no other channel. Nothing enforces an exit-0 block |
| Stop note | `followup_message` | not `systemMessage` |
| klin's own follow-up | exact report hash recorded per session and consumed once | **klin implementation**. On 3.21.18 `beforeSubmitPrompt` **does** fire for an automatic `followup_message` (below), so the hash is what keeps a firing from refreshing the block budget. It recognizes only klin's own text: another stop hook's follow-up that wins Cursor's merge reads as a person's prompt |
| repeated follow-up bound | klin's gate block count and gate tree; a stop with `loop_count` above 0 keeps its session's build stamp of the prompt its chain continues, and a stop at `loop_count` 0 writes a fresh stamp over a stale one of its session | **klin implementation**. Cursor sends no `stop_hook_active`. On 3.21.18 `loop_count` rises by one for each automatic follow-up and returns to 0 after a person's message (below), so a person's prompt still gets a fresh budget |
| no `klin.json` silence | hook-mode gate exits 0 with no output; guard allow is exit 0 with no stdout | ADR 0028; CLI fixtures cover both. A deny of a write to the `klin.json` path still speaks, as on every host |
| Team Marketplace import | **unverified** | README documents Dashboard → Plugins → Team Marketplaces → import; this matrix has no recorded result for that route. Local `plugins/local/<name>` and marketplace cache detection are covered by CLI fixtures |

## Measured on Cursor 3.21.18, 2026-09-23

Probe runs on macOS for PR #314: run 1 and run 2 with klin 0.3.0 at c4a0075,
and run 2 again at fc87e51, after a Cursor block began to exit 0. Each ran
klin's hooks at project scope behind logging wrappers, over a tree whose
README went over its `doc_size` ceiling.

| Behaviour | Result |
|---|---|
| a stop hook that exits 2 with `followup_message` | the follow-up is **not** submitted (run 2: klin the only stop hook, two blocks, no automatic message) |
| a stop hook that exits 0 with `followup_message` | the follow-up is submitted as the next user message (run 1) |
| two stop hooks that both return `followup_message` | the user-scope hook's text wins over the project-scope hook's, as the docs' "last response wins" says (run 1) |
| `beforeSubmitPrompt` on an automatic follow-up | **fires**, and `prompt` carries the submitted (merged) text (run 1) |
| `generation_id` on an automatic follow-up | changes, the same as on a person's message (run 1) |
| stop `loop_count` | rises by one for each automatic follow-up: 0, 1, 2 over klin's block report and its turn-end message. It returns to 0 on the stop after a person's message, then rises 0, 1, 2 again (run 2 at fc87e51) |
| klin's exit-0 block and its told message | both are submitted, both reach `beforeSubmitPrompt` with klin's exact text, and klin consumes both: the prompt counter ended at 2, one for each message the person sent. The repeated told message was not told again, so the chain ended (run 2 at fc87e51) |

Issue #67 originally required the Claude-compatible hook route to be tested
first. That route was not tested. The later product decision requires a
first-class native Cursor plugin, so ADR 0045 explicitly supersedes that
process constraint instead of presenting this native matrix as compatibility
evidence. When #67 closes, AC 1 (compat-first) should be struck, not ticked.

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

## Measured on Cursor 3.22.7, 2026-09-27: several copies of one event

A person drove Cursor 3.22.7 on macOS over a probe repository for #317. It held
logging hooks in `.cursor/hooks.json`, in `.claude/settings.json`, and in a
Claude Code plugin enabled for the project. The hooks used the same events and
matchers as klin's.

| Behaviour | Result |
|---|---|
| hooks in Claude Code's settings files | run beside Cursor's own, with Third-Party Imports on (the default) |
| a Claude Code plugin the project enables | runs too |
| the payload each copy gets | byte-identical, in Cursor's shape and with Cursor's event names |
| a shell call | `beforeShellExecution` for Cursor's own hook, `preToolUse` on the `Shell` tool for the imported and plugin copies |
| one copy answers a stop with `followup_message`, the others print nothing | the follow-up is submitted, whichever copy answered |

Spec 9.8 reads these results. `HOST_COMPATIBILITY.md` has the identity fields.
