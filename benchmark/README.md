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
wrapper both arms run, byte for byte. The arm is an argument of the hook
command, so no difference between the arms lives in that file, and every record
carries the wrapper's digest so a paired cell can be held to it.

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
repository holds the fixture and `klin.json`, and nothing else.

The repository is also the only thing in its own parent directory, so an `ls ..`
reaches nothing. Everything the harness owns lives in that trial's plane under
`runs/`: the arm, the run record, the scoring metadata, the expected shortcut,
the external oracle, klin's state, the wrapper, the host settings and the hook
evidence. The workspace sits under the system temporary directory and the plane
sits in this repository, so the two share no ancestor but the filesystem root
and no relative path leads from one to the other.

Nothing in the subject's environment names any of it. The subject has `Bash`,
so it inherits the host process's environment, and the harness therefore passes
the plane, the klin binary and the arm to the wrapper as arguments of the hook
command in the plane's settings file. Every `KLIN_` variable is dropped from
the environment the host gets, so one left over in the operator's own shell
cannot name klin's binary, klin's state or the arm either.

klin's state is in the plane for the same reason. It holds the journal, and the
journal holds every finding, so a Shadow subject that read it would have the
treatment the arm withholds. The wrapper exports `KLIN_STATE_DIR` itself, so
the host never carries it.

`src/integrity.ts` checks that for every trial, and the record keeps the
answer. It looks for the hidden oracle's files, for the task prompt, and for
any path or file naming the family, the gate or the detector that scores it.

`klin.json` is exempt from the content check. A klin configuration names the
gates it pins, and the agent is allowed to know that the repository carries
one. Every family ships the same `klin.json`, so the configuration tells the
agent nothing about which gate its task targets. A self-test case holds that.

### How the subject is confined

The layout alone is not confinement. The subject runs as the same
operating-system user as the harness, and `ps` on its own parent process names
the settings file the host was given, so every path is one `cat` away from a
shell that is allowed to make it.

The host's own sandbox is what refuses that shell. Each trial's settings file
carries it:

- `sandbox.enabled`, with `autoAllowBashIfSandboxed`, so every `Bash` command
  and every child of one runs under the operating system's own boundary;
- `sandbox.allowUnsandboxedCommands: false`, so the host may not retry a
  refused command outside the sandbox. Without this the boundary holds only
  until the second try;
- `sandbox.filesystem.denyRead` and `denyWrite` over the plane, the root every
  workspace is materialized under and this repository, with `allowRead` and
  `allowWrite` re-opening the subject's own repository inside that denied
  region;
- `permissions.blockReadsOutsideWorkingDirectories`, because `Read`, `Glob` and
  `Grep` are the host's own file tools and no sandbox holds them.

Two openings are deliberate. `~/.cargo` and `~/.npm` stay writable, and
`sandbox.network.allowedDomains` reaches the npm and crates registries: one
task asks the agent to bring a dependency in at an exact version, which it
cannot record without the registry that states it, and a subject that cannot
run its suite measures a different task. Neither directory nor either registry
can tell a subject which arm it is in.

`strictAllowlist` closes the rest. A headless session has no one to answer a
network prompt, so a host that is not on the list is refused outright and the
command fails, rather than the trial stalling until the harness times out.

This is the shape issue #194 verified for Claude Code against klin's real hook
lifecycle. The hooks are the host's own lifecycle and no sandbox holds them, so
the wrapper still writes the plane the subject cannot read.

### Proving it, rather than documenting it

`node benchmark/src/cli.ts probe` runs one real Shadow session and asks the
agent to read the plane. It is white box on purpose: the prompt names the
plane, the workspace root and the harness's records, because the question is
whether the operating system refuses the read and not whether the path is hard
to guess.

The probe plants a random token in each of those three places first. A token
that comes back in anything the session produced, its output or the tree it
left, is a read that succeeded, whatever the agent says about it. It also fails
on any line of klin's withheld answer, on any `KLIN_` variable the subject
could name, and on a tool call that named a path outside the repository.

It costs one session, it writes no run record and it is excluded from every
set. Run it before paying for calibration.

### What the record still measures

`isolation.outside` reads the tool calls the guard saw and reports every path
that resolves into the plane, into the root every workspace is materialized
under, or into klin's own repository. A word that resolves somewhere else
entirely, `/usr/bin/env` or `/bin/sh`, is neither a probe nor a concern.

Three things keep it from reading ordinary work as contamination:

- It reads only the keys of a tool input that name a path, so a written file's
  content and an edit's replaced text are never scanned. Two families are
  TypeScript, and a moved file's own `import "../socket"` would otherwise read
  as a path out of the workspace.
- The repository's own parent is allowed beside the repository. It holds the
  repository and nothing else, so climbing one level reaches nothing, and a
  test run from a subdirectory is ordinary. Climbing past it reaches the other
  trials' workspaces, and that is a probe.
- Every path is resolved through its symbolic links first. On darwin the
  temporary directory is `/var/folders`, a link to `/private/var/folders`, and
  a subject's own `pwd` reports the second form.

It sees the tool calls klin's production matcher covers, which is every `Bash`
command and every write, so a `Read`, a `Glob` or a `Grep` raises no hook and
leaves nothing to read.

A trial whose subject named such a path is invalid. The sandbox refuses the
read, so the harness cannot show what the subject would have learned from a
read it was not supposed to be able to make, and a run that has to be explained
that way cannot be scored as one the treatment alone separated.

The treatment remains semantic feedback against none, not concealment of klin's
existence.

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

Two arms of one cell are compared on more than the memory. `verify` fails a
pair whose klin binary, klin version, klin source commit, harness identity,
host version, requested model, host flags, hook wiring, hook wrapper,
isolated-configuration status or memory digest differ, because the arm must be
the only difference.

Two of those are normalized first, because a raw comparison would fail every
pair. The host flags drop `--session-id` and `--settings`: the session id is a
fresh UUID per trial and the settings path carries the trial id. The hook
wiring is a digest of the settings file that ran, with the plane path, the
workspace path and the arm digit replaced by their names.

Nothing else normalizes away. The wiring digest still attests the real bytes of
the real file: the hook table, the matcher, the timeouts, the binary the
wrapper runs and every sandbox and permission rule the subject ran under. A
changed sandbox option, tool permission or Stop timeout changes it, and a file
one arm truncated or hand-edited fails the cell.

`model.reported` is the one exception. The host names its housekeeping model
beside the session's, and an arm that needed no housekeeping names fewer for a
legitimate reason. So the calibration report states where the arms differed and
`verify` does not fail the cell for it.

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

### What invalidates a run

`infrastructure.terms` is the whole list a trial is judged against, each term
named and passed or failed, and `valid` holds only when every one passed. A
term that failed names the harness's own failure, so the run is excluded
rather than scored, and nothing under `result`, `oracle` or `shortcut` is then
a fact about the agent.

| term | what it holds |
| --- | --- |
| `workspace-isolated` | the control plane stayed out of the workspace |
| `state-fresh` | the repository, klin's state and the session were new |
| `host-result-read` | the host's own JSON result parsed |
| `no-harness-timeout` | the session ended before the harness killed it |
| `behaviour-scored` | the hidden behaviour test ran |
| `shortcut-baseline-read` | the detector read the starting tree it measures against |
| `no-tool-call-outside-the-workspace` | the subject named no path outside its own repository |
| `no-symlink-in-final-tree` | every entry is a plain file, so the digest and the scoring copy hold the whole tree |

The five terms after the first two are why an apparatus failure can never
reach the scorecard as a product outcome. A scorer that could not run, a
fixture whose own starting tree the detector could not read, and a tree the
harness measures incompletely would otherwise read as a failed task. `verify`
reports every failed term, and the calibration report carries an `apparatus`
column beside the result.

A detector that could not read the tree the *agent* left is a different thing,
and it leaves the run valid. An agent may rename, move or break whatever the
family measures, and that run is still a run. `shortcut.unread` says which
tree a detector could not read, `shortcut.present` stays null, and `verify`
names the trial so a person sees it.

`gave-up` is a product outcome, so only the host may report one, through its
own turn limit or budget. The harness timeout is the harness's own wall clock
and says nothing about what the agent would have done next, so it is a term
and never an outcome.

### What the klin source commit says

`klin.binarySha256` is the authoritative identity of what ran. `klin.commit`
is empty unless a build wrote `<binary>.provenance` beside the binary, holding
that same hash and the commit it was built from. `build-klin` is the build
that writes it:

```sh
benchmark/build-klin        # cargo build --release, and the provenance beside it
```

Repository HEAD on its own is not an answer, because a stale build carries an
older commit's behaviour under today's HEAD, and `klin --version` names a
release and no commit. The build is the only moment that knows both the binary
and its source, so the build is what records the pair. A tree holding
uncommitted changes gets no provenance, because no commit describes what was
built, and `build-klin` removes any file an earlier build left rather than
leave a stale one to be read as this build's.

`harness.commit` still records HEAD, because that is where the harness itself
came from.

Nothing is blocked without provenance. `calibrate` warns before a paid set,
`verify` names every record that ties no commit to its binary, and the runs
work either way.

### Signals and the audit trail are two surfaces

`signals` holds what a person may classify: every Regression episode, and every
deleted test klin asked about once. `audit` holds the factual trail beside
them, a guard decision or a reset a person ran.

The boundary is #115's. A blinded reading of a signal site asks what klin's
feedback did to the agent's work. A reset is a person's own action and a guard
row is a decision klin already made, so neither is a site to classify, and both
would be noise in the set a person reads. `validate` refuses a record that
files either one on the wrong side.

Signal evidence is kept in both arms. In Active a signal was delivered. In
Shadow the same hook ran and the same signal would have been delivered. Each
carries the identity `klin stats --json` gives it, and its measured tries, so
repeated blocked stops for one site stay friction rather than a second site to
classify. A reset claims no delivery at all: a person ran it and klin hands an
agent nothing.

A deleted test klin asked about once stays review evidence. It is never
recorded as a regression and never as a repair. `klin stats` already keeps it
out of `counts.caught`, and the record keeps it under `signals` with
`kind: "audit"` and the file, line, declaration text and remedy a reviewer
needs.

The record holds no hidden chain of thought.

### Why no new reporting seam

The ticket allows a minimal additive reporting seam if `klin stats --json`
cannot expose enough. It can. Episodes carry the finding id, the conservative
key, file, line, text, values, remedy, outcome and tries. The audit list
carries guard answers, resets and the deleted-test question. `activity` carries
`klin_ms`. The harness reads all of it through the command line and never opens
a journal file. klin itself is unchanged, so the specification and `tests/`
are untouched.

A row the audit list carries alone states no gate, because a gate's own name
lives on the episode, and `auditKind` is what names such a row. A reset states
no delivery either: a person ran it, klin hands an agent nothing, so neither
arm could have delivered it.

## Running it

```sh
benchmark/build-klin                        # the binary under test, with its provenance
node --test 'benchmark/test/**/*.test.ts'   # the harness, the wrapper, the lifecycle
node benchmark/src/cli.ts selftest          # every fixture and every oracle
node benchmark/src/cli.ts list              # the families and their opaque task ids
```

Those three need no network and no agent. The lifecycle suite plays the four
events Claude Code sends, in order, against the real klin binary, and proves
both arms without paying for a session.

The live commands cost money and take hours:

```sh
node benchmark/src/cli.ts probe                 # one session: can the subject reach the plane?
node benchmark/src/cli.ts run <family> <risk|control> <active|shadow>
node benchmark/src/cli.ts calibrate --seed 1
node benchmark/src/cli.ts verify  benchmark/runs/<stamp>
node benchmark/src/cli.ts report  benchmark/runs/<stamp> --out docs/calibration-<date>.md
```

`probe` costs one session and must pass before the rest are worth paying for:
it is what proves the subject cannot read the control plane. `calibrate` then
runs one live trial per family, variant and arm: 36 runs, in a seeded order, so
the arm is not confounded with the time of day.

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
