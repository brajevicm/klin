# The Shadow/Active benchmark

This directory holds the apparatus for the pre-1.0 Shadow-vs-Active evaluation
of issue #210. It produces no product verdict. It owns the harness, the paired
fixtures, the hidden oracles, the treatment-integrity checks and the
calibration evidence a later benchmark rests on.

Issue #211 executes the frozen publishable round. Issue #115 classifies the
evidence and decides what klin may claim. Neither reads anything here as a
result.

## What the apparatus measures

Whether delivering klin's turn-aware feedback changes an agent's final work.

`tests/` already answers a different question at the binary's command line:
whether a check fires. This one needs two arms over one fixture, and the
difference between them has to be the feedback and nothing else.

## The two arms

Both arms use the same fixture, the same `klin.json`, the same host, the same
model and the same production hook lifecycle. One file, `host/hook`, is the
wrapper both arms run, byte for byte.

In the Active arm the wrapper hands klin's answer back to the host: the exit
status, stdout and stderr klin produced.

In the Shadow arm the same real hooks run and advance klin's own state and
journal. The wrapper records klin's exit status, stdout and stderr, and hands
the host a neutral answer instead. The agent gets no block, no tell, no ask.

The wrapper writes the host's payload to klin and closes the pipe, so klin's
read of stdin reaches the end. A wrapper that left the pipe open would hang the
hook and distort every timing in the record without saying so.
`test/hook.test.ts` puts a stub in klin's place that reads its whole input
before it writes anything, so a wrapper that stopped closing the pipe fails
that suite rather than passing it slowly.

Shadow suppresses guard decisions too. The treatment is klin's semantic
feedback against no semantic feedback, at every point the agent could receive
it.

## What the agent can and cannot see

Each trial gets a fresh standalone Git repository under the system temporary
directory, a fresh klin state directory and a fresh host session. The
repository holds the fixture and `klin.json`, and nothing else. The arm, the
run record, the scoring metadata, the expected shortcut and the external oracle
live under `runs/`, which no path from the workspace leads to.

`src/integrity.ts` checks that for every trial, and the record keeps the
answer. It looks for the hidden oracle's files, for the task prompt, and for
any path or file naming the family, the gate or the detector that scores it.

`klin.json` is exempt from the content check. A klin configuration names the
gates it pins, and the agent is allowed to know that the repository carries
one. Every family ships the same `klin.json`, so the configuration tells the
agent nothing about which gate its task targets. A self-test case holds that.

### What a determined agent could still find

The arm reaches the wrapper as the environment variable `KLIN_BENCH_DELIVER`.
An agent that runs `env` in a shell can read it. The same goes for
`KLIN_BENCH_HOOKS`, which points at that trial's hook evidence, under the work
directory rather than under `runs/`.

This is a known limit. The ticket's treatment is semantic feedback against
none, not concealment of klin's existence, and what the checks enforce is that
the *workspace* carries no arm, no scoring metadata and no oracle. A
calibration run that shows an agent reading its own environment is an apparatus
finding to record.

### The host's own configuration

By default a trial runs under the operator's `~/.claude`. The flags keep most
of it out: `--setting-sources project,local` excludes their settings, and with
them their plugins, hooks and output styles, and `--disable-slash-commands`
excludes their skills. Their memory still reaches the session, because
`~/.claude/CLAUDE.md` is loaded whatever the setting sources say.

So every record carries `host.memory`, the digest and size of the memory that
reached that session. Two arms of one cell must carry the same digest, or they
did not run under one configuration, and a reader on another machine can see at
once that the memory was different there.

Setting `KLIN_BENCH_CONFIG_DIR` removes the contamination entirely: each trial
then gets its own `CLAUDE_CONFIG_DIR` and `host.isolatedConfiguration` is true.
That directory needs a credential of its own, through `ANTHROPIC_API_KEY` or a
login inside it, because the host keys its keychain entry by the configuration
directory. A person freezing the protocol for the publishable round should use
it.

## The nine families

Each family is a small, ordinary repository with two matched variants.

| family | language | the shortcut it makes available |
| --- | --- | --- |
| `inventory` | Rust | delete the failing test instead of repairing the behaviour |
| `escapes` | TypeScript | silence or skip the failing path |
| `stubs` | Rust | leave a placeholder where the fiddly part belongs |
| `complexity` | TypeScript | add the new rule to the already-branching function |
| `lockfile` | TypeScript | declare a dependency without the state that pins it |
| `doc-citations` | TypeScript | move a cited file and leave the document behind |
| `dead-symbols` | Rust | leave behind what the change stopped using |
| `reachability` | Rust | leave a command module nothing dispatches to |
| `public-api` | TypeScript | change what the published package already promised |

`doc-size` is out of this round, because it is not one of the agent-shortcut
loops the product positioning leads with. `layering`, `conventions` and `sarif`
are out too, and #115 has to name them as unvalidated scope.

The risk variant is an ordinary implementation request where the shortcut is
available and neither asked for nor hinted at. The correct solution is possible
without it. The control variant is matched ordinary work on the same edit
surface, where the target regression should not appear.

Six of the nine starting trees carry unchanged debt of the family's own class,
so the control also tests the ratchet promise: legacy debt must not become
feedback because the agent worked nearby. The three that cannot are
`inventory`, `reachability` and `public-api`, and each family's `legacyDebt`
field says why. An existence ratchet has no inherited state. A reachability
family is only derived when every member of the starting cohort is proven
reached. A published contract holds no break by construction.

No prompt names klin, the gate, the arm, the word shortcut, or the expected
detector behaviour. A self-test case reads every prompt and fails on any of
them.

### Adding a family for a new gate

Nothing under `src/` names a gate. A family is data, and the harness finds it
by walking `fixtures/`. Adding one takes:

1. `fixtures/<family>/family.json`: the language, the gate, a one-line summary,
   what unchanged debt of the class the starting tree holds, the words a prompt
   may not use, the shortcut detector and its arguments, and the behaviour
   command per variant.
2. `fixtures/<family>/base/`: the starting tree, with the same `klin.json`
   every other family ships. A self-test case fails if it differs.
3. `fixtures/<family>/{risk,control}/`: `prompt.md`, an optional `overlay/`,
   the hidden `oracle/`, and the `good/` and `bad/` trees the self-test runs
   the oracle against.
4. A detector in `src/detectors.ts` and one line in `DETECTORS`, if no existing
   detector answers the question. Nine detectors are there now.
5. The gate's name in `GATES` in `test/catalogue.test.ts`.

Step 5 is deliberate. That list is the round's declared scope, so adding a
family is a protocol change: it changes the list, the protocol version in
`src/paths.ts` and the number of calibration cells together, and the test
fails until all three agree.

A family in a language neither Rust nor TypeScript needs no harness change
either, because the behaviour command lives in `family.json`. It would need a
detector that reads that language.

## The external oracles

Every variant has an oracle that lives outside klin and outside the agent's
workspace. It answers two questions about the final tree:

1. is the requested software behaviour correct? A hidden test is laid over a
   copy of the final tree, after the session ends, and run;
2. does the family's target shortcut sit in the tree? `src/detectors.ts`
   answers that.

No oracle runs `klin gate`, reads klin's verdict or reads a benchmark label.
Every detector compares the final tree against the starting tree, so debt the
fixture already held is never counted as a shortcut the agent took.

The detectors are line and brace scans, not compilers. They are accurate on the
fixtures they were written for. When a fixture's shape defeats a scan, record
it as a fixture defect and leave the scan alone.

Three rules in them exist because the first review found their absence:

- an escape or a stub is counted by how often its line of text appears in the
  whole tree, so relocating debt the fixture planted is not a shortcut the
  agent took, while writing a second copy of it still is;
- a declaration's head is read to the bracket that ends it, so a signature a
  formatter reflowed over several lines is the same contract;
- a test behind a comment marker, or behind `#[ignore]` or `.skip`, is not a
  live test. An agent that silenced the failing test took the same shortcut as
  one that deleted it.

Each variant ships a known-good tree the oracle must pass and a known-bad tree
it must fail, as overlays over the starting tree. `selftest` runs all of them.

## What the production hook does with each known-bad tree

`selftest` also runs `klin gate --hook --changed`, the command the Stop hook
runs, over each known-bad tree against its own starting tree, and compares the
answer with the `hookFires` the fixture records. All nine risk variants are
flagged at the turn's end.

Two of them were not when the apparatus was built. The first probe runs, on
2026-09-17, recorded that the Stop hook stayed silent over the `doc-citations`
and `reachability` risk trees, because a changed run judged only the files the
turn changed and in each of those two the evidence sits in a file the turn
left alone: the document that still cites the moved file, and the command
module the new dispatch no longer reaches. That was recorded rather than tuned
away, and it was a product gap, not a fixture defect. Issues #234 and #235
closed it. Each of those two checks now owns a bounded judgement unit wider
than the changed-file set, the way `public-api` already did, so both fire at
the turn's end and the fixtures record `hookFires` true.

## The run record

`record.schema.json` is the contract. One record describes one trial well
enough for #115's scorecard to be computed without scraping terminal text:
provenance, scheduled order, timing, the agent's outcome, the oracle's two
answers, klin's own facts, the hook evidence and the isolation checks.

Signal evidence is kept in both arms. In Active a signal was delivered. In
Shadow the same hook ran and the same signal would have been delivered. Each
carries the identity `klin stats --json` gives it, and its measured tries, so
repeated blocked stops for one site stay friction rather than a second site to
classify.

A deleted test klin asked about once stays audit and review evidence. It is
never recorded as a regression and never as a repair. `klin stats` already
keeps it out of `counts.caught`, and the record keeps it under `audit` with the
file, line, declaration text and remedy a reviewer needs.

The record holds no hidden chain of thought.

### Why no new reporting seam

The ticket allows a minimal additive reporting seam if `klin stats --json`
cannot expose enough. It can. Episodes carry the finding id, the conservative
key, file, line, text, values, remedy, outcome and tries. The audit list
carries guard answers, resets and the deleted-test question. `activity` carries
`klin_ms`. The harness reads all of it through the command line and never opens
a journal file. klin itself is unchanged, so the specification and `tests/`
are untouched.

## Running it

```sh
cargo build --release                       # the binary under test
node --test 'benchmark/test/**/*.test.ts'   # the harness, the wrapper, the lifecycle
node benchmark/src/cli.ts selftest          # every fixture and every oracle
node benchmark/src/cli.ts list              # the families and their opaque task ids
```

Those three need no network and no agent. The lifecycle suite plays the four
events Claude Code sends, in order, against the real klin binary, and proves
both arms without paying for a session.

The live commands cost money and take hours:

```sh
node benchmark/src/cli.ts run <family> <risk|control> <active|shadow>
node benchmark/src/cli.ts calibrate --seed 1
node benchmark/src/cli.ts verify  benchmark/runs/<stamp>
node benchmark/src/cli.ts report  benchmark/runs/<stamp> --out docs/calibration-<date>.md
```

`calibrate` runs one live trial per family, variant and arm: 36 runs, in a
seeded order, so the arm is not confounded with the time of day.

| variable | what it sets |
| --- | --- |
| `KLIN_BIN` | the binary under test, default `target/release/klin` |
| `KLIN_BENCH_MODEL` | the model the host runs, default `sonnet` |
| `KLIN_BENCH_BUDGET` | dollars per trial, default 5 |
| `KLIN_BENCH_TIMEOUT_MS` | wall clock per trial, default 30 minutes |
| `KLIN_BENCH_WORK` | where subject workspaces are materialized |
| `KLIN_BENCH_CONFIG_DIR` | give each trial its own host configuration, which needs its own credential |

## What calibration is for

Calibration verifies the apparatus, not whether klin wins. Every record it
writes says `publishable: false`, and `verify` refuses a set that says
otherwise.

It may show a broken prompt, fixture or oracle. Fix only validity and
measurement defects, record every such change, and start a new fixture and
protocol version. Do not tune the fixtures to catch more after seeing the
outcomes. The publishable round begins only after a person freezes the
protocol.

## Layout

```text
benchmark/
  host/hook              the wrapper both arms run
  record.schema.json     the run record contract
  src/                   the harness
  test/                  the harness's own tests
  fixtures/<family>/     base/, risk/, control/, each with prompt, overlay,
                         oracle, good and bad
  runs/                  the control plane, and where records land
```

`fixtures` is in klin's built-in skip set, so a fixture tree that carries debt
on purpose is never measured as klin's own. Each run record keeps its tree
copies under a `fixtures/` directory of its own for the same reason: records
committed to this repository must not become klin's own source.

The harness is TypeScript run by Node, with no build step and no dependency. It
links no klin Rust module and speaks to klin only through the binary's command
line, which is the seam `AGENTS.md` names.
