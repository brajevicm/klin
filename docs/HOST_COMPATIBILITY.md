# Host compatibility

This is klin's support ledger. It records the host surfaces klin claims as
first-class, and the evidence behind each claim. A row says what somebody
verified, not what is expected to work.

`tests/host.rs` and `tests/plugin.rs` stay the deterministic contract: they
prove klin still speaks the host contract klin last verified. They cannot
prove the current stable host still loads the plugin, runs the lifecycle and
honors klin's answer. That is what the canary and the release smoke cover.

`docs/cursor-compatibility.md` holds the detailed measured evidence for
Cursor. This file does not replace it.

## The claim

For 1.0, klin supports the **current stable local or native plugin surface**
of Claude Code, Codex and Cursor.

klin makes no compatibility promise for:

- old or minimum-supported host versions,
- cloud, remote, IDE or enterprise-managed surfaces of the same vendors,
- historical host releases,
- custom harness integrations, which implement the harness protocol in
  `docs/HARNESS_INTEGRATION.md`,
- native Windows, where klin ships no binary.

Do not record a surface as covered because it carries the same vendor name.

## The ledger

| host | supported surface | host version | OS | date | automated canary | release smoke | notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Claude Code | local plugin marketplace plus `claude plugin install klin@klin` | not yet recorded | ubuntu-latest | — | not yet run | not recorded | the canary needs `ANTHROPIC_API_KEY`; without it a run is inconclusive, not a pass |
| Codex | `codex plugin add klin@klin` | not yet recorded | ubuntu-latest | — | not yet run | not recorded | Codex does not trust plugin hooks on install, and no headless trust flow is documented. The canary is inconclusive until `CODEX_TRUST_COMMAND` names a CI-only trust step |
| Cursor | native plugin at `~/.cursor/plugins/local/klin` | 3.20.21 | macOS | 2026-09-16 | not automatable | not recorded | hook behavior measured by hand, see `cursor-compatibility.md`. No documented route says the headless agent loads a local plugin, so the canary stops at `INCONCLUSIVE` and this row is release-smoke-only. `CURSOR_HEADLESS_PLUGINS=1` lets the canary go further once somebody verifies such a route |

A row moves to a verified version, date and `PASS` only after a run that
concluded. An inconclusive run never rewrites a row.

## Event identity

Spec 9.8 makes one copy of klin's hooks take effect per host event. It reads
the fields below. Two copies of one event carry the same values, and two
different events differ in at least one of them.

| host | how recorded | session | prompt or turn | tool call | stop |
| --- | --- | --- | --- | --- | --- |
| Claude Code 2.1.283 | probe, macOS, 2026-09-26: two project hook copies per event, `claude -p` | `session_id` | `prompt_id`, on every event after session start | `tool_use_id` | `stop_hook_active`, `last_assistant_message` |
| Codex CLI 0.157.1 | probe, macOS, 2026-09-26: two project hook copies per event, `codex exec`, hooks trusted for the run | `session_id` | `turn_id`, on every event after session start | `tool_use_id` | `stop_hook_active`, `last_assistant_message` |
| Cursor 3.22.7 | probe, macOS, 2026-09-27: a native copy, a copy imported from `.claude/settings.json` and a Claude Code plugin copy per event, driven by a person | `conversation_id`, equal to `session_id` | `generation_id`, on every event but session start, new for each message and each follow-up | `tool_use_id` on `preToolUse`; a shell call by `generation_id` and its command | `loop_count`, `status` |

What the probes showed:

- Each copy of one event received a byte-identical payload, and the host
  started the copies within 6 milliseconds of each other.
- A session start carries no prompt or turn id on either host. `source`
  (`startup`, `resume`, `clear`, `compact`) scopes it.
- Claude Code ran two parallel `Write` calls one after the other, each with its
  own `tool_use_id`.
- After a stop hook blocked, both hosts sent the second stop with the same
  `prompt_id` or `turn_id`, `stop_hook_active: true` and the new
  `last_assistant_message`.

Cursor runs the hooks in Claude Code's settings files by default, beside its
own: `.claude/settings.local.json`, `.claude/settings.json` and
`~/.claude/settings.json`, at a lower priority than Cursor's own hooks. The
setting is "Include Third-Party Plugins, Skills, and Other Configs" under
Cursor Settings → Agents → Third-Party Imports. It maps `SessionStart`,
`UserPromptSubmit`, `PreToolUse` and `Stop` to `sessionStart`,
`beforeSubmitPrompt`, `preToolUse` and `stop`. This comes from Cursor's
third-party hooks reference.

What the Cursor probe showed:

- Cursor ran all three copies, so it loads a Claude Code plugin that the
  project enables, beside its own hooks.
- Every copy of one event got a byte-identical payload in Cursor's shape,
  with Cursor's event names. An imported copy got no Claude Code field. The
  copies of one event started at most 158 milliseconds apart.
- A shell call reached Cursor's own hook as `beforeShellExecution` and the
  imported and plugin copies as `preToolUse` on the `Shell` tool, with a
  `tool_use_id` the native copy never saw. The imported copies got it even
  under klin's Claude Code matcher, which names `Bash`.
- A stop that one copy answered with `followup_message`, while the other two
  printed nothing, had its follow-up submitted. This held for each of the three
  copies.
- Each follow-up and each stop after it carried a new `generation_id`, and
  `loop_count` rose from 0 to 1. A second chat got its own `conversation_id`.

## The canary

`.github/workflows/host-compatibility.yml` runs `ci/host-canary.sh` when a
person dispatches it. It is not part of PR gating.

Each host gets one small journey in a disposable Git repository that holds a
`klin.json` and one document under a ceiling:

1. the current stable host accepts the klin plugin package,
2. a session reaches klin's radius lifecycle, which leaves a journal line,
3. the host invokes klin's Stop hook,
4. klin returns a failing gate report for the turn's change,
5. the host carries that block instead of dropping it.

Every assertion reads observable state: the host's own exit status, the
plugin listing, klin's journal under `.git/klin/` and the session log. No
assertion depends on how a model phrases an answer.

The deterministic semantics of guards, matchers, tools and MCP shapes belong
to `tests/host.rs`. The canary does not repeat them.

## How a red canary is classified

`ci/host-canary.sh` writes one word to `classification.txt`, and the workflow
keeps the host version, logs, journal and repository state as artifacts
whenever that word is not `PASS`.

| classification | meaning | job |
| --- | --- | --- |
| `PASS` | the whole journey held | green |
| `COMPAT` | the host changed under klin: a rejected manifest, a lifecycle hook that no longer runs, an event klin can no longer place, or an ignored block | red |
| `INFRA` | the host package, the runner, the network, the fetch of klin's plugin source at its tag or klin's own pinned-release download failed, so the journey never started | green, with a warning |
| `INCONCLUSIVE` | no entitlement, no documented trust step, or a turn that changed no file, so the observed point was never reached | green, with a warning |

Only `COMPAT` fails the job. `INFRA` and `INCONCLUSIVE` keep their evidence
and leave the ledger untouched.

## Pre-release smoke

Run the smoke after `cut-release` pushes a new tag and `dist` publishes its
release, and before `promote-release` merges the tag into `main`. Until the
promotion, plugin users stay on the last release (ADR 0029). Verify each host
by hand on a clean profile with the current stable version. Record the host
version, OS, klin version, date and PASS or FAIL in the ledger above.

When the smoke fails:

1. Do not promote the tag.
2. Mark the last good release as Latest again with
   `gh release edit vX.Y.Z --latest`. `dist` made the failed release Latest,
   so the installer and `klin update` serve it until then.
3. Fix the plugin, then cut the version after the failed tag with the
   `version` input of `cut-release`. `main` still holds the version before the
   failed tag, so a `level` input names the failed version again.

Install from the new tag through the documented commands, with the tag
appended:

- Claude Code: `/plugin marketplace add brajevicm/klin#vX.Y.Z`, then
  `/plugin install klin@klin`.
- Codex: `codex plugin marketplace add brajevicm/klin --ref vX.Y.Z`, then
  `codex plugin add klin@klin`.

For every host:

1. the plugin installs or loads from the new tag through those commands,
2. a repository with a `klin.json` invokes klin,
3. the guard refuses an attempted write to `klin.json`,
4. a deterministic failing stop reaches the agent through the host's block or
   report channel.

Codex adds two steps:

1. review and trust klin's hooks through the normal `/hooks` trust flow,
2. start a fresh session and prove the trusted hooks run.

Cursor adds one step: the documented plugin copy with `--branch vX.Y.Z`, and a
window reload.

This smoke exists because marketplace, trust and reload flows have no reliable
headless API. Do not build UI automation to avoid it.
