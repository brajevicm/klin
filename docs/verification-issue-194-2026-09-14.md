# Issue #194 verification — Codex and Claude Code hooks

Date: 2026-09-14

## Verdict

The existing lifecycle-hook architecture works in both hosts with the normal
repository-work sandbox. Classification is **A — works unchanged** for the
hook runtime. No production code, hook configuration, state layout, turn
semantics, or benchmark files were changed.

The tests used disposable Git repositories and the existing project-local
hook route. Their hook commands pointed at the already-built \`klin 0.1.1\`
binary from the current worktree. Codex needed
\`--dangerously-bypass-hook-trust\` only because the disposable copy was an
untrusted temporary project; this bypassed hook trust, not the sandbox, and
did not grant the agent a \`.git\` write exception. The current repository is
already trusted by Codex.

| Host | Version | Sandbox and approval | Hook route | Classification |
| --- | --- | --- | --- | --- |
| Codex | \`codex-cli 0.154.0\` | \`--sandbox workspace-write\`; no agent approval prompt | project \`.codex/hooks.json\` | A |
| Claude Code | \`2.1.270\` | project sandbox enabled, \`autoAllowBashIfSandboxed=true\`, \`allowUnsandboxedCommands=false\`; \`acceptEdits\`; no agent approval prompt | project \`.claude/settings.json\` | A |

Host: macOS \`26.6.2\`, \`arm64\`; Git \`2.55.0\`. Klin source commit:
\`08157d54a92b59e9141b9ac42c73862356d81f74\`.

## Real-session scenarios

Each scenario ran against a disposable repository whose base contained a
five-word \`README.md\` and a \`klin.json\` ceiling of ten words. The only
working-tree change left by the agent was the intentional README edit; the
index stayed clean.

| Scenario | Codex | Claude Code |
| --- | --- | --- |
| SessionStart/UserPrompt stamp | Hooks ran and created the \`.git/klin\` turn state and \`refs/worktree/klin/turn\`. | SessionStart and UserPromptSubmit hook events both returned success and created the same state. |
| Failing Stop on the exact stamped turn | A 30-word README produced a new \`doc-size\` finding against turn \`4ebb58c1eb00700695ab232718e5f1f166512b8f\`; Stop blocked with exit 2. | A 30-word README produced the same new finding against turn \`189a32aa5fad68da75a828bfbc9bc1a0f831e393\`; Stop blocked with exit 2. |
| Red persists across another prompt | A second real prompt left the README unresolved; the same turn \`4ebb58c1...\` and finding remained red. The second Stop attempt was allowed only by the existing \`stop_hook_active\` path. | A second real prompt likewise left turn \`189a32aa...\` red with the same finding; the second Stop attempt was allowed by the existing guard. |
| Fix and next prompt | The agent shortened the README to eight words; Stop passed. The next prompt advanced the turn stamp (\`33aa51ee...\`, then \`ab28000b...\` in the follow-up fixture). | The agent shortened the README to eight words; Stop passed. The next prompt advanced the turn stamp to \`85c81c87...\`. |
| Missing turn-file recovery | Removing only \`.git/klin/turn\` restored it from \`refs/worktree/klin/turn\`, kept the ref unchanged, and restored a red verdict while the README was still dirty. | The same recovery restored the turn from \`refs/worktree/klin/turn\`, kept the ref unchanged, and restored red. |

The hook output named the changed file and the precise \`doc-size\` finding.
After the fix, the next prompt advanced the turn state without changing the
working-tree result.

## Sandbox boundary

The tests also probed whether an agent shell could write Git metadata:

- In Codex, \`touch .git/agent-shell-probe\` returned
  \`Operation not permitted\`; the lifecycle hook still wrote
  \`.git/klin/turn\`, \`.git/klin/mark\`, and the journal.
- In Claude Code, the same shell probe returned success under the enabled
  Claude project sandbox. The probe was removed from the disposable
  repository afterward. Lifecycle hooks also wrote the normal \`.git/klin\`
  state.

The host difference does not require a layout or permission change: hook
execution and agent file edits exercised the existing paths in both sessions.
This is not classification C; the hook runtime could write its Git state in
both hosts.

The first launches through the terminal wrapper exposed unrelated harness
restrictions: Codex could not initialize its nested app-server client, and
Claude could not create \`~/.claude/session-env\` or see the local auth keychain.
Rerunning the host sessions with the outer launcher allowed to bootstrap while
keeping the nested repository sandbox enabled succeeded. This is an
environment/integration caveat, not a klin failure.

## Timings

Representative \`/usr/bin/time -p\` wall-clock measurements for the direct
lifecycle commands were:

| Host | \`klin radius\` | Failing Stop | Passing Stop |
| --- | ---: | ---: | ---: |
| Codex | 0.08 s | 0.12 s | 0.11 s |
| Claude Code | 0.08 s | 0.12 s | 0.11 s |

The real-session journal timings were in the same range. The first failing
Stop measured 132 ms in Codex and 163 ms in Claude Code; passing Stops measured
119 ms and 116 ms respectively. The direct timing runs used the same binary
and hook payloads, with the agent-independent lifecycle command isolated from
model latency.

## Reproduction prompts

The prompts used for the core real sessions were:

1. “Use the file-edit tool to replace \`README.md\` with exactly 30 words. Do
   not fix any klin finding.”
2. “Do not edit files. Report that \`README.md\` remains intentionally
   unresolved, then stop. Do not fix the klin finding.”
3. “Use the file-edit tool to shorten \`README.md\` to exactly 8 words so the
   klin finding passes.”
4. “Do not edit files; report the current fixed state, then stop.”
5. “Use Bash to run \`touch .git/agent-shell-probe\`; report the result.”

The second prompt was run in a separate real session for each host, which
confirmed red persistence across a prompt boundary rather than only across
the two Stop attempts within one prompt.

## Acceptance summary

- Codex real session: pass.
- Claude Code real session: pass.
- Exact stamped-turn gating: pass.
- Red persistence across another prompt: pass.
- Fix to green and next-prompt advancement: pass.
- Turn-file-only recovery from \`refs/worktree/klin/turn\`: pass.
- Normal repository-work sandbox with no \`.git\` exception or state-layout change: pass.
- Timings for \`radius\`, failing Stop, and passing Stop: recorded above.
- Production/state/layout/benchmark changes: none.
