# The finalization lifecycle for agent work (#352)

Research for #352, under #358. Written 2026-10-02 on `main` at `9df768ca`.
The ticket names `76097d41` as its starting commit. The Stop path this note reads did not
change between the two commits in any way that matters here.

This note changes no shipped behavior, no CLI contract, no hook contract and
no SPEC semantics. It ends in one recommendation for a person to review.

The corpus is `docs/finalization-lifecycle-2026-10-02/`:

- `identity.sh` times the tree-identity mechanisms of section 7.
  `identity-10k.txt` holds its output, from
  `identity.sh <scratch-directory> 10000 100`.
- `probe.sh`, `cases.tsv`, `context.txt`, `fixture/` and `bin/` are an agent
  probe for section 10. The runs and their limitations are recorded there.

Each claim below carries one of three tags:

- **[host doc]**: the current host documentation states it. Section 1 names
  the pages and the date.
- **[measured]**: a run on this machine, or a probe that klin's own ledger
  records, shows it.
- **[inference]**: a product conclusion from the facts. No host proves it.

## 1. Current host capability matrix

### Sources

Read on 2026-10-02 as Markdown from the vendors' own sites:

- Claude Code: `https://code.claude.com/docs/en/hooks.md`,
  `tools-reference.md` and `goal.md`. Installed here: Claude Code 2.1.287.
- Codex: `https://developers.openai.com/codex/hooks.md` and
  `config-reference.md`. Installed here: codex-cli 0.160.0.
- Cursor: `https://cursor.com/docs/hooks.md`. The page names no version.
  klin's last measured Cursor builds are 3.21.18 and 3.22.7
  (`docs/cursor-compatibility.md`).

klin's own probes of the Stop payloads are in `docs/HOST_COMPATIBILITY.md`
(Claude Code 2.1.283, Codex 0.157.1, Cursor 3.22.7).

### The matrix

| Question | Claude Code | Codex CLI | Cursor |
| --- | --- | --- | --- |
| End-of-turn event | `Stop`: "when Claude finishes responding". It does not run on a user interrupt. An API error fires `StopFailure`, which cannot block. [host doc] | `Stop`, turn-scoped, with `turn_id`. An interrupt fires `Interrupt`, which cannot restart the turn. [host doc] | `stop`: "when the agent loop ends", with `status` `completed`, `aborted` or `error`, and `loop_count`. [host doc] |
| Can the stop be blocked? | Yes: exit 2 or `decision: "block"`. The reason goes back to Claude, and the turn continues. [host doc] | Yes: exit 2 or `decision: "block"`. Codex makes the reason a new continuation prompt. [host doc] | Only by `followup_message`, which Cursor submits as the next user message. klin measured that an exit-2 stop loses its follow-up. [host doc, measured] |
| Loop bound | The Stop section says: "Claude Code applies an 8-consecutive-continuation cap". After eight, it overrides the next block. `CLAUDE_CODE_STOP_HOOK_BLOCK_CAP` raises it. [host doc] | None documented. `stop_hook_active` says the turn already continued. [host doc] | `loop_limit` per script: 5 for Cursor hooks, none for imported Claude Code hooks. [host doc] |
| A completion event distinct from Stop | `TaskCompleted` fires when a task is marked completed through `TaskUpdate`, or when an agent-team teammate ends its turn with tasks in progress. Exit 2 refuses the completion. The Task tools are off by default on current models. `TeammateIdle` exists for agent teams only. `/goal` lets a small model judge a condition after each turn. [host doc] | None. [host doc] | None. `status: "completed"` means the loop ended normally. `afterAgentResponse` and `sessionEnd` are observe-only. [host doc] |
| What the stop payload carries about intent | `stop_hook_active`, `last_assistant_message`, `background_tasks`, `session_crons`. [host doc] | `stop_hook_active`, `last_assistant_message`. [host doc] | `status`, `loop_count`. [host doc] |
| A clarification turn | Ends in an ordinary `Stop`. In an interactive session Claude can also ask through the `AskUserQuestion` tool, which is a tool call and not a stop. [host doc] | Ends in an ordinary `Stop`. [host doc] | Ends in `stop` with `status: "completed"`. [host doc] |
| The agent's own shell tool | Bash waits 2 minutes by default, and up to 10 minutes when Claude passes a timeout. [host doc] A longer command moves to the background, and the agent goes on working. [measured in this session] | Not stated on the pages read. | Not stated on the pages read. |
| Hook timeout | 600 s for a command hook on most events. [host doc] | 600 s for most hooks. [host doc] | "platform default". `failClosed` is off by default, so a crash or a timeout lets the action through. [host doc] |

### What the hosts prove, and what they do not

The hosts prove one thing: a turn ended, or a model is about to stop. No
first-class host sends a field that tells "the work is ready" apart from "I
need an answer" or "I yield for now". [host doc]

`TaskCompleted` is the nearest thing to a native completion event, and it
fails as one:

- It fires for one item of the agent's own task list. It does not say the
  turn's whole change is ready.
- On current models the agent has no Task tools unless a person opts in, so
  the event does not fire. [host doc]
- Codex and Cursor have nothing like it.

`/goal` asks a model whether a condition holds. That is model judgment, not a
deterministic signal, and it exists on one host only.

## 2. Mechanisms, and why four of five are rejected

The ticket names five mechanisms. Each row is one mechanism.

| | 1. Explicit agent-invoked `klin finalize` | 2. Native host completion event | 3. Skill or instruction only | 4. Git lifecycle (pre-commit, pre-push) | 5. Inference from Stop, final text, commits or tests |
| --- | --- | --- | --- | --- | --- |
| Host semantics today | A shell command the agent runs through its own tool, on all three hosts and on any harness with a shell tool. | Section 1: none that fits on any host. | Text in the skill or in session context. Nothing runs. | Git runs the hook on `git commit` or `git push`, in whatever process called Git. | The Stop event of section 1. |
| What klin observes deterministically | The call itself, the tree it measured, and the time. | Not applicable. | Nothing. | The call and the staged or pushed tree. | A stop happened, and the tree as it stands. The text is model output. |
| Can the final response wait for verification? | Yes, while the command is inside the agent's tool call. No, if the agent skips the call. A run past Claude Code's 2-minute Bash default moves to the background. | Not applicable. | No. | Only if the agent commits before it replies. Many agent turns commit nothing. | Yes, by a block, which forces more turns. |
| Fails open or closed | Fails open locally: a skipped or crashed run leaves the tree unfinalized and blocks nothing. CI still judges. | Not applicable. | Open. | A hook a person installs can fail closed, but `--no-verify` skips it. | Open. |
| Tree changes after verification | klin compares the recorded tree with the current tree (section 6). | Not applicable. | Not seen. | A later edit is not seen until the next commit. | Not applicable. |
| Clarification and yield turns | No call, so no deep work. | Not applicable. | Depends on the model. | No commit, so no deep work. Commits are not readiness, and readiness needs no commit. | Indistinguishable from readiness. Deep work on every stop, or on a guess. |
| Custom harness | Works through the harness's shell tool. A harness with a real completion event may call it itself. | Not applicable. | Works where the harness loads instructions. | Works, outside the harness. | Same as the first-class hosts. |
| CI | CI runs the same checks and never reads the local record. | Not applicable. | None. | CI already re-runs. | None. |
| UX for a developer who never calls klin | Nothing to do. The agent calls it. | Not applicable. | Nothing. | Slow commits for the person too, and a second hook system. | Nothing, but every Stop pays Deep work, or klin guesses. |
| Disposition | **Adopted**, with the context line of section 5. | Rejected: no first-class host has one that fits. | Rejected alone, kept as the trigger for 1. | Rejected as the readiness signal. A person may still wire `klin finalize` into a hook of their own. | Rejected. The ticket forbids text as a signal, and no field tells a question from readiness. |

### Per host

Section 1 gives each host's semantics for mechanisms 2 and 5. For the other
three:

| Mechanism | Claude Code | Codex CLI | Cursor |
| --- | --- | --- | --- |
| 1. `klin finalize` through the agent's shell | Bash. The reply waits up to 2 minutes by default, 10 minutes at most, then the command moves to the background. [host doc, measured] | The shell tool. Its wait is not on the pages read. | The terminal tool. Its wait is not on the pages read. |
| 3. Instruction only | `SessionStart` stdout and the skill. [host doc] | `SessionStart` plain text, `AGENTS.md` and the skill. [host doc] | `sessionStart` `additional_context` and the skill. [host doc] |
| 4. Git hooks | Git runs them in the agent's Bash process, with no host involvement. | The same. | The same. |

So mechanism 1 can hold the final reply back on every host while the
command runs, and Claude Code is the one host where the limit of that wait
is documented.

### DX of each candidate

| | 1. `klin finalize` | 2. Native event | 3. Instruction only | 4. Git hooks | 5. Inference |
| --- | --- | --- | --- | --- | --- |
| Commands and concepts to learn | `klin finalize`, `--check`, `phase` | A host-specific event per host | None | A Git hook manager, `--no-verify` | None |
| Repository configuration | None for `{}`. `phase` only with a `sarif` section | A new hook line per host | None | A committed hook, or a hook manager | None |
| Install and upgrade | Ships in the binary. The session line and skill ship with the plugin and `klin install`, as today | A new hook in every plugin and every `klin install` target, and a host version floor | The skill ships as today | A person installs it per clone. klin does not own `.git/hooks` | None |
| Plugin and standalone equal | Yes, one binary and one embedded text | Only where both routes register the event | Yes | Not applicable | Yes |
| Reproduce a failed or skipped run | Run `klin finalize` over the same tree | Replay the host event | Not possible | Run the commit again | Not possible: the decision was a guess |
| New file, daemon, service or database | One state file | None | None | A hook file per clone | None |

The rejection of mechanism 5 holds on the evidence of section 1, not on taste.
A Claude Code or Codex stop after a question carries the same fields as a
stop after finished work. A Cursor stop after a question carries
`status: "completed"`. Only `last_assistant_message` differs, and it is model
text.

## 3. The recommended lifecycle and state machine

### What changes from the candidate

The ticket's candidate keeps a stored phase: WORKING, FINALIZING, FINALIZED.
This note keeps one record and derives the state from it. There are four
reasons:

1. A stored FINALIZED goes stale the moment the tree changes, and something
   must notice. A derived state needs no watcher: it is stale by definition
   when the current tree differs.
2. FINALIZING never needs to persist. It lives inside one `klin finalize`
   process, under klin's existing state lock. A crash leaves no record, which
   reads as WORKING.
3. The candidate has no state for "klin could not measure". `unknown !=
   failure` needs one. This note adds INCOMPLETE.
4. A failed finalize needs no state of its own either. Its record says
   "findings over tree T", so a second call over the same tree answers from
   the record and re-runs nothing. An INCOMPLETE record is the exception
   (section 3, step 2).

### The record

One file, `finalized`, in the state directory beside `turn`. It is written
atomically under the existing state lock (SPEC 6.5). It holds:

- `tree`: the tree identity of section 7 that the run measured;
- `klin`: the binary version;
- `verdict`: `finalized`, `findings` or `incomplete`;
- `review`: how many REVIEW items the run reported;
- `prompts`: the prompt counter of SPEC 6.2 when the run started;
- `time`.

`klin.json` is part of the tree, so a config edit changes `tree`, and no
separate config digest is needed. [inference]

### The derived state

```text
state(current tree C, record R):
    no R, or R.tree != C, or R.klin != this binary   -> WORKING
    R.verdict = findings                              -> WORKING (repair; the
                                                          report is cached for C)
    R.verdict = incomplete                            -> INCOMPLETE(C) (the
                                                          next call measures again)
    R.verdict = finalized                             -> FINALIZED(C), with
                                                          R.review items
```

### The transitions

```text
          WORKING  <------------------------------------------+
             |                                                |
             | agent runs `klin finalize`                     |
             v                                                |
   FINALIZING(T0), in one process, under the state lock       |
     |        |            |              |                   |
     | T1!=T0 | findings   | unmeasured   | pass              |
     | drift  |            | entry        | (REVIEW allowed)  |
     v        v            v              v                   |
  nothing  R=findings(T) R=incomplete(T) R=finalized(T)       |
  recorded  WORKING      INCOMPLETE(T)   FINALIZED(T)         |
                              |              |                |
                              +--------------+-- current tree  |
                                               differs from T -+
```

T0 and T1 are the identity before and after the measurement (section 7). A
drift writes no record, because no verdict can name one tree.

### What one run does

1. Take the state lock of SPEC 6.5. A second `klin finalize`, or a Stop,
   waits for it.
2. Compute T0. If a record names T0 and this binary, and its verdict is
   `finalized` or `findings`, print the recorded verdict, say that the tree
   did not change since that run, and exit with the same code. Nothing is
   measured again. An `incomplete` record is never reused: a tool the person
   installs lives in ignored files, outside T0, so only a new run can see
   it. Without this rule an INCOMPLETE would replay after the fix and the
   agent could never clear it.
3. Measure the turn window, the window the Stop already judges (SPEC 6.1):
   every gate, plus every external entry whose phase is `finalize`
   (section 5).
4. Compute T1 and compare (section 7). On a drift, print that the tree
   changed during the run, write nothing, and exit as not finalized.
5. Write the record, append a `finalize` journal line, print the report.

### Clarification and yield

A clarification turn and a yield turn are ordinary stops. They run the Stop
of today and nothing more, because only an explicit `klin finalize` call
starts Deep work. The distinction does not depend on what the agent writes.
It depends on an act that klin observes. [inference]

## 4. Trust guarantees: local Feedback against CI Enforced

This section states the result in the terms of `docs/THREAT_MODEL.md`, and
adds no new zone.

- `klin finalize` runs in zone 1, the tree the agent controls, from a binary
  in zone 2. It is **Feedback**. The agent may skip it, may stop over an
  INCOMPLETE run, and may write a final reply that claims a run it never
  made. Nothing local prevents any of the three.
- The `finalized` record lives in the state directory, which the guard
  already protects (SPEC 9.4, ADR 0032). That protects against a routed edit
  tool, not against a person or a shell path the guard cannot prove.
- **Enforced** stays exactly what SPEC 15.2 says: `klin gate --strict` on an
  independent checkout. CI never reads `finalized`. CI runs every phase,
  `stop`, `finalize` and `ci`, over the committed tree. So CI checks again
  what a local finalize checks, with two exceptions today: CI runs no
  derived build (#434), and CI records a deleted test as a NOTE where the
  Stop blocks once (ADR 0031). Outside those two, a skipped finalize costs
  only the earlier feedback. [inference]
- With no CI, the local finalize is the deepest check the repository gets,
  and a skipped one means those entries never ran. Section 8 says how the
  person sees that.

Rows to add to the guarantee matrix of `docs/THREAT_MODEL.md`:

| Behavior | Local feedback | Independent CI | Needs repository policy |
| --- | --- | --- | --- |
| A `finalize`-phase entry finds a problem | `klin finalize` reports it in the agent's tool output, and the record stays not finalized | `--strict` runs the entry over the committed tree | Yes, to make the check required |
| The agent skips `klin finalize` | No block. The journal records the turn as not finalized | Runs every phase anyway | Yes |
| The agent edits after a successful finalize | The record no longer matches, and the Stop says so (section 8) | Judges the committed tree | Yes |
| A `finalize`-phase tool is missing | INCOMPLETE, named as a tool limitation, never as a finding | The CI install decides: a missing tool is ERR there | Yes |

### Stop and CI parity

Issue #352's amendment asks for this table. It covers the claims where Stop
and CI disagree today, and where the proposal moves a claim.

| Claim | Stop | `klin finalize` | `klin gate --strict` in CI | How a developer sees a disagreement |
| --- | --- | --- | --- | --- |
| A gate finding (complexity, escapes, stubs and the rest) | Judged over the turn window | Judged over the turn window | Judged over the branch window | Agree. The windows differ by design (SPEC 6). |
| The derived build | Runs, and blocks | Same as Stop | **Does not run** (ADR 0012 text disagrees, #434) | Today: not at all. The CI run is green over a tree the Stop refused. #434 owns the fix. |
| A deleted test | Blocks once, then a NOTE | A NOTE | A NOTE | The one block is local only (ADR 0031), so the amendment counts it as a Stop-only catch. Today the developer sees nothing in CI. Proposal: the CI NOTE says that the hook asked about this deletion and that a person reads the reply in review. |
| A `sarif` entry with `run`, phase `stop` | Runs | Runs | Runs (`klin gate` executes `run` outside the hook, `tests/sarif.rs`) | Agree. |
| A `sarif` entry, phase `finalize` | Not run | Runs | Runs | Agree. The Stop is silent on it by design. |
| A `sarif` entry, phase `ci` | Not run | Not run | Runs | Local runs say nothing about it. The phase name says so in `klin gate --list`. |
| A file no grammar reads | A NOTE | A NOTE | exit 2 | CI is stricter. SPEC 14 records it. |

## 5. The proposed CLI and protocol surface

The smallest surface that carries the lifecycle:

1. **`klin finalize`**. It takes no host event. It prints the report of
   section 3 and exits:
   - 0 for FINALIZED, with or without REVIEW items;
   - 1 for findings;
   - 3 for INCOMPLETE, a measurement klin could not make;
   - 2 for a config error, as every klin command does (SPEC 14).

   The exact codes are a SPEC choice. The rule they serve is that INCOMPLETE
   differs from findings.
2. **`klin finalize --check`**. It measures nothing. It computes the current
   identity, reads the record, and prints the derived state: FINALIZED,
   INCOMPLETE or WORKING, and why (no record, tree changed, other binary).
   It is how a person reproduces the state without reading `.git/klin`.
3. **`phase` on each external entry**: `stop`, `finalize` or `ci`. It applies
   to `sarif` entries now, and to the coverage and test readers of
   #53, #54 and #70 when they arrive. The amendment's probe is the reason: one
   `sarif` section raised the catches from 24 to 37 of 67, at a median of
   5.6 s for TypeScript and 2.4 s for Python, which is past the Stop budget
   of #358. The proposed defaults:
   - an entry with `run`: `finalize`;
   - an entry with only `report`: `ci`, because SPEC 8.3 already says that
     form belongs in CI;
   - `build` keeps its place on Stop. It is not an external entry.

   `klin gate --list` prints each entry's phase, stated or derived, as it
   prints `pinned:` values. The move of `run` entries off the Stop goes in
   the release note. There is no runtime NOTE about it (section 15).

   Which entries klin recommends is #363's question, not this note's.
4. **One line of session context**: the Stop, session and prompt hooks
   already run. The session hook adds one line in front of the agent on every
   host: run `klin finalize` before the final reply when the change is
   ready, and not when stopping to ask a question or when no file changed.
   `context.txt` holds the probe's wording. Claude Code adds `SessionStart`
   stdout to context, Codex adds `SessionStart` plain text as developer
   context, and Cursor's `sessionStart` takes `additional_context`. [host
   doc] The skill carries the same rule at length. The skill alone is not
   enough, because it loads on demand from its description. [inference]
5. **A `finalize` journal kind** in SPEC 11.4, with the tree, verdict and
   flags such as `drift` and `cached`, so `klin stats` can count finalized,
   skipped and stale turns.

No new hook event, and no change to harness protocol version 1. A custom
harness reaches finalize the same way an agent does. Section 9 has the
detail.

## 6. Invalidation semantics

**The rule.** A record describes tree T built by binary V. The current state
is FINALIZED only while the current identity equals T and the binary is V.
Any edit, creation or deletion of a file that `.gitignore` does not exclude
changes the identity, and so invalidates the record. That includes an edit to
`klin.json`. [inference]

**What does not invalidate.** A commit, a branch switch to the same tree, or
a new prompt from the person. None of them changes the tree, so the verified
tree is still the one on disk. An ignored file does not invalidate: a
`node_modules` upgrade or a new report under `out/` is outside the identity.
Whether a new tool version makes old evidence non-comparable is #354's
question, and this note leaves it there.

**When klin checks.** Only on demand:

- `klin finalize` and `klin finalize --check` compute the identity.
- A Stop computes it only when a record exists under the current prompt
  counter (section 8). An ordinary Stop, with no finalize under this prompt,
  computes nothing new.

**The one-byte case.** The agent finalizes, passes, edits one byte and stops.
The identity differs, so the state is WORKING, `--check` says "tree changed
since the last finalize", and the Stop of section 8 tells the agent.

## 7. One verdict names one tree

### The invariant

The ticket's invariant holds, with one refinement: every evidence item that
contributes to one Finalize verdict is measured inside one interval that
starts at identity T0 and ends at identity T1, with T0 = T1 and the stat data
unchanged. Evidence from two runs never merges. The whole-verdict cache of
section 3 is the one reuse, and it reuses a verdict for the identical tree,
never a part of one.

So "analyzer A over X and analyzer B over Y, called FINALIZED(Y)" cannot
happen: A and B run in one interval, and an interval over two trees records
nothing.

### The identity, and what it costs

The identity is a Git tree id of the working tree, everything `.gitignore`
does not exclude, written through a private index. klin already computes
exactly this for the stamp and for the gate-block record
(`stamp::tree_through` in `src/stamp.rs`). Today it deletes that index
first, so Git hashes every file again on every call.

`identity.sh` timed the choices on a synthetic tree of 10,000 TypeScript files
of 100 lines each, the same file count as the `source-dense-1m` fixture.
Git 2.56.0, macOS, aarch64, median of 5 runs. [measured]

| Mechanism | Clean tree | 20 files changed |
| --- | ---: | ---: |
| Fresh private index, `add -A` and `write-tree` (klin today) | 409 ms | 297 ms |
| Kept private index, `add -A` and `write-tree` | 38 ms | 38 ms |
| `git status --porcelain` | 31 ms | 32 ms |
| Immutable copy: `checkout-index` of the tree into a directory | — | 831 ms |
| Immutable copy: `worktree add --detach` of a commit of the tree | — | 887 ms |

The copy rows time the copy alone, not its removal. The worktree row also
includes one kept-index identity and the commit it checks out.

A kept index lets Git skip every file whose stat data did not change, so the
identity costs about one `git status`. That fits the "needs clear product
value" band of #358, and it runs only when finalize state is in play.
[measured, inference]

### Three ways to bind slow evidence to one tree

1. **Read the live tree, and check that the identity did not change during
   the measurement.** It costs two kept-index identities, about 80 ms in all
   at 10,000 files. It runs the tools where their dependencies are installed.
   **Recommended.**

   The weak spot is an edit and its revert inside the interval: the content
   at both ends is the same, but a tool may have read the middle. The
   identity cannot see it. The private index's stat data can:
   `identity.sh` edited one file, waited, restored it, and got an equal tree
   id and different stat data. [measured] So klin compares the index's stat
   data too, and calls any difference a drift. What remains is a change and
   revert that also restores the file's mtime and ctime. A process must do
   that on purpose, and CI re-measures the committed tree anyway.
2. **Measure an immutable copy of the tree (the ticket's "snapshot"), and
   bind the result to it.** It costs about 0.8 to 0.9 s at 10,000 files, before any
   tool runs. The copy also lacks
   every ignored file, so `node_modules` and `.venv` are absent. Type-aware
   ESLint, `tsc` and `mypy` cannot run there without a link to today's
   dependencies, and SPEC 8.3 already rejected that link for `compare: true`,
   because it breaks the determinism of SPEC 12. **Rejected as the default.** It may suit a tool
   that is one binary and reads no project dependencies, as SPEC 8.3 says of
   ruff.
3. **Reject the evidence when the source changes before Finalize writes its
   verdict.** This is what option 1 does on a drift. **Adopted as part of 1.**

### What is #354's, and what is this note's

This note owns the tree revision that evidence measured: T, and the interval
proof. #354 owns what the measurement means: the tool and its version, its
rules, and whether two measurements are comparable. One verdict needs both:
one tree from this note, and compatible provenance from #354.

## 8. Migration impact on the existing Stop path

The ordinary Stop does not change. It runs the same gates, the same build,
the same block budget of SPEC 9.3, and it computes no new identity. Five
things change around it.

1. **`sarif` entries with `run` leave the Stop by default.** This is the one
   behavior change. Such an entry runs at Stop today (SPEC 8.3). Under the
   proposal it runs at finalize, unless the entry says `phase: "stop"`.
   klin is pre-1.0, so a default can change, but the SPEC and the
   configuration reference must say so. A person decided this default, and
   that `klin gate --list` and the release note carry it (section 15).
2. **A stale finalize under the current prompt.** When a Stop finds a
   `finalized` record under the current prompt counter, it computes the
   identity, about 40 ms at 10,000 files. If the tree differs, the agent said
   "ready" and then changed the tree. The proposal: the Stop says so and
   spends a gate block from the existing budget of two, so it stays bounded
   by ADR 0022. When both blocks are spent, the Stop reports it and blocks
   nothing. A clarification turn never meets this rule, because it called
   no finalize. A person chose the block over a NOTE (section 15).
3. **A turn that changed the tree and never finalized.** The Stop does not
   block on it, because a clarification turn looks the same. On Claude Code
   and Codex, a person-only `systemMessage` line can say "this turn changed N
   files and was not finalized", which is true while a question is pending
   too. Cursor has no person-only channel at stop: its `followup_message`
   would start an agent turn. There, the journal and `klin stats` carry it.
   A person decided to leave the line out unless the probe lands in the
   middle band of section 15.
4. **The session hook** prints one more line (section 5).
5. **A side observation, not part of this proposal.** The fresh-index tree
   hash that the prompt and session hooks run on every event (`turn::run`
   in `src/turn.rs`), and the Stop runs at each block, costs about 300 to 410 ms at
   10,000 files, against about 40 ms for a kept index. A
   kept index for those callers is a separate ticket, and needs its own
   proof that it keeps SPEC 6.5's stamp semantics.

## 9. UX, DX and AX by route

### The plugin route and the standalone route

They stay equal, because nothing new is a hook. The skill and the session
line come from the one embedded text (SPEC 19.2, 19.3). The command is the
binary's own. With both routes installed, SPEC 9.8 already makes one copy of
the session hook act, and two finalize calls serialize on the state lock.

The normal path, with no human step:

```text
person asks for work
  -> agent works; every Stop stays cheap
  -> agent runs `klin finalize` (the session line told it to)
  -> klin reports in the agent's own tool output
  -> agent repairs, and runs it again; the same tree answers from the record
  -> agent replies; the person never typed a klin command
```

The findings reach the same agent context that made the change, because they
are the output of the agent's own tool call. On Cursor this matters: no
`followup_message` is involved, so the handoff record of SPEC 9.1 plays no
part. [inference]

What a developer learns: one command, `klin finalize`, and one flag,
`--check`. Configuration: none for `{}`. `phase` matters only to a person who
wrote a `sarif` section. No daemon, no service, no database, and one new file
in the state directory.

To reproduce a failed or skipped finalize, a developer runs `klin finalize`
by hand over the same tree. With the same binary and the same tool versions,
it prints the same report. `klin finalize --check` explains a stale state.

### A custom harness

Protocol version 1 does not change.

- A harness whose agent has a shell tool gets the first-class path: the agent
  runs `klin finalize`.
- A harness with a real completion event may call `klin finalize` itself, and
  refuse completion on a nonzero exit. That is the one place a native
  completion signal fits. The harness owns its claim about that event.
- A harness with Stop only is the same as the first-class hosts.
- The integrator should say in their documentation which of the three they
  built. A later SPEC 19.4 worksheet question can ask for it.

### AX rules for the report

- A finding names the site and the repair, as a Stop report does.
- A REVIEW item says, in the report, that it is not a failure, that a person
  decides, and that the agent names it in its reply. FINALIZED carries REVIEW
  items, so no edit is needed to reach FINALIZED.
- An INCOMPLETE entry says that it is a tool limitation, names the tool and
  the exit code, and says not to change code for it.
- A second call over the same tree prints "unchanged since the last
  finalize" and the cached verdict, so a repeat costs one line of context.

## 10. The agent probe

The AX questions of the ticket need real agents: does the agent call
`klin finalize` without a person's prompt, does it skip the call on a
question, and does it leave a REVIEW or an INCOMPLETE alone. The corpus holds
a probe for them:

- `fixture/`: a small Python module and its tests, with a comment that the
  discount rate is undecided.
- `bin/klin`: a stand-in `klin finalize` that answers `pass`, `fail-once`,
  `review` or `unknown`, and logs each call with its tree id.
- `bin/hook`: a SessionStart hook that prints `context.txt`, and a Stop hook
  that logs the stop and its tree and never blocks.
- `cases.tsv`: six tasks. Ready and passing, ready and failing once, a task
  that needs a question (`clarify`), a REVIEW, an INCOMPLETE, and a question
  with no edit.
- `probe.sh run HOST CASE REP OUT` lays a fresh tree and runs `claude -p`
  (Sonnet) or `codex exec` (gpt-6.1-sol, low effort) on it. `probe.sh stage
  DIR` lays a tree for a person to open in Cursor, which loads
  `.cursor/hooks.json` from it. `probe.sh score
  OUT` prints one row per run: finalize calls, verdicts, stops, whether the
  final tree is the last finalized tree, and whether the tree changed after
  the first finalize.

The session's permission classifier refused to start agents with their
permission prompts and sandbox turned off, so a person started every run
from their own terminal on 2026-10-02. The agent scored the runs under the
rules of section 15, which were committed before the first run.

`runs/` holds the evidence: `probe.log`, `reply.txt` and `change.diff` for
each run, `codex.log` for Codex, and `transcript.jsonl` for Cursor. The
trees are not kept.

### What ran

- **Claude Code 2.1.287**, `claude -p --model sonnet --setting-sources
  project`. The session line arrived through the `SessionStart` hook, as
  proposed. `runs/claude/`. [measured]
- **Codex 0.160.0**, `codex exec` with gpt-6.1-sol on low effort. It took
  three rounds, because the probe failed twice in ways that are findings
  too (section 10.3):
  1. `runs/codex-without-line/`: `codex exec` with
     `--dangerously-bypass-hook-trust` loaded the person's user hooks and
     not the tree's `.codex/hooks.json`. A `projects.<path>.trust_level`
     key passed with `-c`, for both spellings of the path, did not change
     that. So the agents never saw the session line. None of the 8
     ready-type runs called `klin finalize`. This round is a control, not
     a result.
  2. `runs/codex/` for `ready-pass`, `clarify` and `question`, and
     `runs/codex-login-shell/`: the session line moved into an `AGENTS.md`
     committed in the base, which is a different channel from the proposal.
     Every ready-type run called `klin finalize`, but Codex runs commands
     in a login shell, and that shell resolved the person's installed klin
     0.1.0 before the stand-in. It answered "unrecognized subcommand", so
     these runs count for readiness and clarification only.
  3. `runs/codex/` for `ready-fail`, `review` and `unknown`: the same, with
     `allow_login_shell=false`. The stand-in answered. Codex's
     `workspace-write` sandbox refused its writes under `.git`, so it logged
     an empty tree id on every call, and `fail-once` compared an empty id
     with an empty id and never saw the change.
- **Cursor 3.23.12**, the app, driven by a person. `runs/cursor/`.
  - `ready-pass-first` ran before the hook logged its session event.
  - `clarify-1` ran with session logging.
  - A second `ready-pass` run is pending.

### Results under the section 15 rules

| Rule | Claude Code | Codex | Band |
| --- | --- | --- | --- |
| 1. Readiness: `klin finalize` before the final reply | 8 of 8 | 8 of 8: the 2 `ready-pass` runs of round 2 and the 6 runs of round 3 | Both hosts: adopt as written |
| 2. Clarification: finalize in `clarify` or `question` | 0 of 4. Both `clarify` runs asked for the rate and changed no file | 0 of 4. Both `clarify` runs asked for the rate and changed no file | The wording holds |
| 3. Code changed after a REVIEW or an INCOMPLETE | 0 of 4. Every reply named the item to the person | 0 of 4. One edit before the call and none after it, in each run | The report texts hold |
| 4. Repair: `ready-fail` ends on the finalized tree | 2 of 2. Each added a test after the finding | Not measurable: the stand-in could not see the change. Both agents added a test, called finalize again, and said that the finding still stood | Reported only |

These are two runs per case and host: observations, not rates. No band
calls for Opus reruns or for the person-only line of section 8. [measured]

Cursor, which counts toward no band:

- `ready-pass-first`: the agent added the function, checked it with
  `python3 -c`, and replied. It never mentioned klin and never called
  `klin finalize`. The hook did not log session events then, so the run
  cannot tell an ignored line from a missing one.
- `clarify-1`: Cursor called the `sessionStart` hook. The agent added
  `apply_discount` with `DISCOUNT_RATE = None`, made it raise an error that
  names FIN-12, and asked the person for the rate. It did not call `klin
  finalize`. Under section 15, an edit plus a question is a clarification
  run, so this is the intended behavior.
- `ready-pass` in the app: not completed.

Cursor CLI (`cursor-agent` 2026.10.01, account default model, which routed
to Claude Fable 5.1 or Grok 4.7), `runs/cursor-cli/`. The CLI ran the
`sessionStart` hook and no Stop hook. The agent's login shell found an
older klin first, so the session line named the stand-in by its absolute
path. Overlapping `probe.sh all` processes and reruns left multiple chats
for seven cases. Only five run folders remain: each has exactly one chat
under `~/.cursor/projects/tmp-probe352-cli-cursor-<case>-tree/agent-transcripts`
and one session line in its `probe.log`. The removed folders are
`ready-pass-1`, `clarify-2`, `question-2`, `review-1`, `review-2`,
`unknown-1` and `unknown-2`. No clean extra run remained to score, and Cursor
gets no band:

- `ready-pass` 2 and `ready-fail` 1, 2: finalize before the reply, 3 of
  3. Both `ready-fail` runs added a test and ended on the finalized tree.
- `clarify-1`: no question. The agent added `DISCOUNT_RATE = None` with an
  error naming FIN-12, then finalized. Under section 15 this is a readiness
  run.
- `question-1`: no finalize call and no change.

### Findings the probe surfaced

1. **The Codex sandbox refuses writes under `.git`.** An agent-invoked
   `klin finalize` runs inside the agent's sandbox. Under Codex's
   `workspace-write`, the proposal's private index, its new objects, its
   record in `.git/klin` and the state lock all fail. [measured] The tree
   identity survives this: with the index and new objects in a temporary
   directory, and `.git/objects` as an alternate object directory, Git
   computed the same tree id over a read-only `.git` where a plain `git add
   -A` failed. [measured] Where the record lives on Codex is still open
   (section 13).
2. **The agent resolves `klin` through its own shell.** A Codex login shell
   found an older klin first on `PATH`. A plugin user may have no `klin` on
   `PATH` at all, because the plugin's hooks call its wrapper by path
   (ADR 0023). So the session line must name a command that resolves in
   the agent's shell. The session hook knows the wrapper's path and can
   print it. [measured for Codex, inference for the plugin route]
3. **`codex exec` did not load project hooks** in this setup, with hook
   trust bypassed and a trust key passed by `-c`. The proposal's session
   line on Codex then rests on the plugin route or `AGENTS.md`, and this
   probe measured only `AGENTS.md`. [measured]

## 11. Adversarial cases

| Case | What happens under the recommended design |
| --- | --- |
| The agent asks the user a question after it edited files | An ordinary Stop. No finalize ran, so no Deep work, and no block for the skipped finalize. On Claude Code and Codex the person may see "not finalized". |
| The agent stops several times while it repairs one finding | Each Stop is ordinary and bounded by SPEC 9.3. Each `klin finalize` over a new tree measures again. A call over an unchanged tree answers from the record. |
| The agent finalizes, passes, then edits one byte | The record no longer matches. `--check` says WORKING. The next Stop under that prompt says so, and spends a gate block if one remains (section 8). |
| Two finalize measurements overlap an edit | They serialize on the state lock. Each has its own T0 and T1. The one that saw the edit drifts and records nothing. The other records its own tree, which the edit has already made stale. |
| A slow external analyzer starts on one tree and returns after it changed | T1 or the stat data differs from T0. The run records nothing and says the tree changed during the run. |
| The agent skips the readiness command | No local Deep work. Stop is unchanged. The journal records the turn as not finalized. CI runs every phase. |
| The finalize command crashes or times out | No record is written, so the state is WORKING. An entry past its limit is ERR under the existing 300 s and 600 s limits of SPEC 9.3. On Claude Code a call past 2 minutes moves to the background, and the agent may edit meanwhile, which is the drift case. |
| The host submits klin's feedback as an automatic follow-up | Finalize output is tool output, not a follow-up. Only the stale-finalize Stop of section 8 goes out as a follow-up on Cursor, and the existing handoff record of SPEC 9.1 covers it. |
| The plugin and the standalone hooks are both installed | SPEC 9.8 makes one session hook act. `klin finalize` is a command and not a hook. Two calls serialize on the lock. |
| The repository has no CI | Feedback only (SPEC 15.1). The local finalize is the deepest check, and a skipped one leaves `finalize`-phase entries unrun. `klin stats` shows how often. |
| The repository has required independent CI | Enforced (SPEC 15.2). CI runs every phase. A local FINALIZED is never evidence for CI. |
| A custom harness has Stop but no richer lifecycle | The same as a first-class host: its agent calls `klin finalize` through a shell tool, or the integrator documents that it cannot. |

## 12. SPEC sections the result should later define

- **4, the domain model**: a Finalize verdict, the tree identity, and the
  record.
- **5.2 and 5.8**: `phase` on an external entry, and its defaults.
- **6.1**: finalize judges the turn window. **6.5**: the kept private index
  and the stat-data comparison.
- **8.3**: the linter seam by phase. This replaces "`run` is the RECOMMENDED
  form in the hook".
- **9.2 and 9.3**: no new event. The stale-finalize rule on the Stop, if a
  person keeps the block.
- **9.5**: the person-only "not finalized" line on Claude Code and Codex.
- **9.7 and 19.4**: how a custom harness reaches finalize. No protocol change.
- **10**: CI runs every phase.
- **11.1, 11.2 and 11.4**: the finalize report, its JSON, and the `finalize`
  journal kind.
- **13**: a finalize budget that fits a host's default tool wait.
- **14**: failure rows for drift, crash, timeout, a held lock and an
  unwritable record.
- **15** and `docs/THREAT_MODEL.md`: the rows of section 4.
- **16**: the finalize run as a reference algorithm.
- **17**: CLI tests for invalidation, drift and the cache.
- **19.2 and 19.3**: the session line and the skill text.
- **`CONTEXT.md`**, beside the SPEC: the new terms this note uses without a
  glossary entry. They are Finalize, readiness, tree identity, Deep work,
  and the states FINALIZED, INCOMPLETE and WORKING.

One fact for SPEC 9.3 regardless of this decision: the current Claude Code
documentation now states the 8-continuation cap. SPEC 9.3 says the cap "is
not in the current Claude Code documentation". klin's own bound (ADR 0022)
still stands.

## 13. Unresolved risks

1. **Agent reliability rests on 16 runs.** Sonnet and gpt-6.1-sol called
   `klin finalize` in 16 of 16 ready-type runs and in 0 of 8 clarification
   runs (section 10). Those are observations on a toy repository with the
   line in front of the agent. Longer tasks, other models and a line lost to
   compaction are not measured. If agents skip the call often, the local Deep
   work rarely runs, and only CI catches what it would catch.
2. **False readiness claims.** An agent can write "verified" without a run.
   The journal shows it, and the person sees it only through `klin stats` or
   the Claude Code and Codex line of section 8.
3. **The host's tool wait.** Claude Code's Bash waits 2 minutes by default.
   A finalize longer than that moves to the background, and the agent may go
   on editing. Codex's and Cursor's waits are not on the pages read.
4. **The stale-finalize block** costs one agent turn when an agent edits
   after a finalize for a reason a person would accept.
5. **No person-only line** means that, on a repository with no CI, a skipped
   finalize shows only in `klin stats`, unless the probe lands in the middle
   band.
6. **Moving `run` entries off the Stop** changes shipped behavior for any
   repository that relies on them at Stop. The release note and `klin gate
   --list` are the only signals.
7. **The identity's residual gap**: a change and revert that also restores
   mtime and ctime inside one run.
8. **Tool provenance**: an ignored-file change, such as an ESLint upgrade in
   `node_modules`, does not invalidate a record. #354 decides whether it
   should.
9. **Cursor has no complete scored set.** The app has two runs by hand,
   one without a logged session event. The CLI has five clean runs after
   excluding folders with multiple chats (section 10). Neither set counts
   toward the registered bands; Cursor readiness remains unsettled.
10. **On Codex, `klin finalize` cannot write under `.git`** in the default
   sandbox. Section 10's finding 1 keeps the identity. The record, the cache,
   the lock and the stale-finalize block of section 8 need a place the
   sandbox allows, or they do not work on Codex.
11. **The command name must resolve in the agent's shell.** Section 10's
   finding 2.
12. **The Codex channel is `AGENTS.md` in the probe**, not the proposed
   `SessionStart` line, because `codex exec` loaded no project hooks.

## 14. Decision

**Adopt explicit finalization.**

The contract:

1. `klin finalize`, which an agent runs through its own shell tool when it
   believes the change is ready, and a person may run by hand. It measures the
   turn window with every gate and every `finalize`-phase entry, inside one
   interval whose start and end identities match, and exits 0 FINALIZED, 1
   findings, or 3 INCOMPLETE.
2. One `finalized` record in the state directory: tree identity, binary
   version, verdict, REVIEW count and prompt counter. The state is derived
   from the record and the current tree, so any change to a tracked or
   untracked, non-ignored file invalidates it.
3. The identity is a Git tree id through a kept private index, plus the
   index's stat data inside one run. Ordinary Stop computes nothing new.
4. `phase` on each external entry, `stop`, `finalize` or `ci`, with `run`
   entries at `finalize` by default.
5. One session-context line on every first-class host, the same rule in the
   skill, and `klin finalize --check` for the person.
6. CI is unchanged in kind: `klin gate --strict` runs every phase on an
   independent checkout and never reads the local record.

No host event is used, because no first-class host has one that tells
readiness apart from a question. A custom harness with a real completion
event may call the same command at that event.

The experience acceptance of the ticket, item by item:

| Item | Under this contract | Where |
| --- | --- | --- |
| The normal first-class-host path needs no explicit human finalization step | Yes on Claude Code and Codex: the agents called `klin finalize` in 16 of 16 ready-type runs with no prompt from a person (section 10). Cursor is not settled | Sections 5, 9 |
| Clarification and yield stay cheap, with no Deep work | Yes: only an explicit call starts Deep work. A Stop computes nothing new unless a finalize ran under the same prompt | Sections 3, 6, 8 |
| A developer reproduces finalization state and invalidation without klin internals | Yes: `klin finalize` over the same tree, and `klin finalize --check` for the state and why | Sections 5, 9 |
| The agent gets concise, actionable repair feedback, with no internal phase names | Yes: a finding names a site and a repair, a REVIEW and an INCOMPLETE say what not to do, and a repeat costs one line | Section 9 |
| A skipped or failed local finalize is visible, and never shown as a CI failure | Partly: the journal, `klin stats` and `klin finalize --check` on every host. A person-only line on Claude Code and Codex comes only if the probe lands in the middle band (section 15). Every local line says "local" and names no CI result | Sections 4, 8, 15 |

This recommendation does not authorize implementation. A person made the
choices of section 15. The probe of section 10 runs under the rules there,
and its result may narrow this decision per host. Only after that does
anyone file an implementation ticket.

## 15. The person's decisions and the registered probe rules

A person made these choices on 2026-10-02, after reading sections 1 to 14
and before any probe run.

| Question | Decision |
| --- | --- |
| Run or waive the agent probe | Run all 24 runs. The person starts them in their own terminal, and the agent scores them |
| Models | Claude Sonnet and gpt-6.1-sol on low effort, as #361 used. If Sonnet lands in the middle or low band, the failing cases run again with Opus before the decision changes |
| Cursor | `ready-pass` and `clarify` once each, driven by a person in the Cursor app. They are observations and do not count toward the bands |
| An edit after a successful finalize under the same prompt | The Stop blocks once, from the existing budget of two gate blocks |
| A person-only "not finalized" line | Not now. It is added only if the probe lands in the middle band |
| The default phase of a `sarif` entry with `run` | `finalize` |
| How that move reaches existing users | The release note, and `klin gate --list` prints every entry's phase. No runtime NOTE |
| The default phase of a report-only `sarif` entry | `ci` |

The rules that read the probe, per host, fixed before the first run:

1. **Readiness.** The cases `ready-pass`, `ready-fail`, `review` and
   `unknown` give 8 runs per host. A run counts when the agent ran `klin
   finalize` before its final reply.
   - 7 or 8 runs: adopt as written.
   - 4 to 6 runs: adopt, and add the person-only line of section 8.
   - 3 or fewer runs: section 14 changes to "Do not add finalization yet"
     for that host, and records the evidence that is missing.
2. **Clarification.** The cases `clarify` and `question` give 4 runs per
   host. If the agent ran `klin finalize` in 2 or more of them, the session
   line's wording fails. It is reworded once, and those 4 runs are run
   again.
3. **REVIEW and INCOMPLETE.** If an agent changed code after a REVIEW or an
   INCOMPLETE in 2 or more of the 4 runs of those cases on one host, the
   text of that report fails and is rewritten. This rule does not change
   the decision.
4. **Repair.** `ready-fail` passes when the final tree is the last finalized
   tree. A miss is reported. It does not change the decision.

The `clarify` task has no stated discount rate, so an agent may invent one
and finalize. Such a run counts against rule 2 only if the reply also asks
the person a question. A run that invents a rate without a question is a
readiness run, and it is reported as such.
