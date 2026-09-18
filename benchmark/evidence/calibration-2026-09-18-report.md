# Calibration report, 2026-09-18

Issue #210 asks this report to list every apparatus defect found and fixed, and
every limitation that remains. The run table and the exposure counts are in
`calibration-2026-09-18-runs.md`, generated from the records. The slim evidence
is in `calibration-2026-09-18/`, bound to an external archive by
`evidence.json`.

Two of the limitations below are open problems large enough to have their own
analysis, written for whoever decides what #211 freezes. Both are read-only:
they change no fixture, detector or ticket, and neither proposes a remedy.

- `calibration-2026-09-18-open-fixture-strength.md` — why six of nine families
  expose nothing, with every prompt and every agent's diff.
- `calibration-2026-09-18-open-statistical-design.md` — why #211's adequacy
  floor of 5 sits below the 6 discordant pairs its own exact McNemar test needs,
  with the power table.

These runs are calibration. They may not be published, #115 excludes them from
the product scorecard, and this document states no product conclusion.

## The calibrated candidate

| | |
| --- | --- |
| Harness and klin | `36d630a8118c9e2e0f6a7496728b3e413e5d1120` |
| klin binary | `sha256` recorded per run, with a provenance file tying it to that commit |
| Host | Claude Code 2.1.276, one version across all 36 runs |
| Model | `sonnet` |
| Protocol | 4, seed 1 |
| Set | `benchmark/runs/2026-09-18T15-12-58`, 36 of 36 valid |

## Sets run before this one

Four sets preceded it. None is evidence for anything but the defect it found.
Each was invalidated by a fault the set before it could not have surfaced.

| Set | Trials | Why it does not count |
| --- | --- | --- |
| `2026-09-17` | 36 | Arms ran different host versions in 9 of 18 cells. Raw bytes never preserved. |
| `2026-09-18T07-41-13` | 36 | Same host drift. Records predate a field rename. |
| `2026-09-18T11-54-30` | 36 | Two subjects ran klin against their own tree. No build provenance. |
| `2026-09-18T13-23-34` | 16 | Clean, but calibrates `7a8670e`, a candidate without ADR 0048. |
| `2026-09-18T14-51-20` | 16 | Clean, but four families only. |

## Defects found and fixed

### 1. The host version drifted inside a set

**Found:** the first two sets, by `verify`. Claude Code updated mid-run, so 9 of
18 cells had one arm on 2.1.275 and the other on 2.1.276. That breaks the
paired-arm invariant #210 requires.

**Fixed:** not in code. The operator holds the host version fixed for the
duration of a set. `verify` already fails a cell whose arms differ, which is how
it was caught.

**Remains:** nothing prevents an update mid-set. A preflight that refuses to
start unless the version matches one the operator names was considered and not
built.

### 2. The subject could not read the toolchain it had to build with

**Found:** `2026-09-18T11-54-30`, as five refused tool calls. `~/.cargo` and
`~/.npm` were writable but not readable, and `cargo` and `rustc` on this machine
are rustup shims resolving a toolchain under `~/.rustup`. Subjects could not
compile the four Rust families, so those sessions measured a different task than
the fixture states.

**Fixed:** `9d98e8d`. All three homes are readable as well as writable.

### 3. `PWD` and `OLDPWD` named klin's own repository

**Found:** by reading a subject's own tool calls. `cwd` moves a process without
rewriting what it inherits, and the harness runs out of klin's repository, so
every subject was handed that path. One subject read it and ran the klin binary
under `target/release` against its own tree.

**Fixed:** `9d98e8d`. `session.withoutKlin` drops every variable whose value
names a path the harness owns, and filters `PATH` rather than dropping it.

### 4. A blocked Stop echoed the hook command line, arm included

**Found:** `2026-09-18T11-54-30`, in the subject's own transcript. The host
prints the whole hook command line back to the agent under `Stop hook
feedback:`. It carried the wrapper, the plane, the klin binary and the arm
digit. Four of 18 Active subjects received it, and two ran `klin gate --hook
--changed` themselves. A Shadow subject doing the same would obtain the entire
treatment its arm exists to withhold.

**Fixed:** `9d98e8d`. `host/hook` is a template; each trial gets a copy with the
plane, the binary and the arm substituted in, under a name of random bytes.
`settle` restores the stable name afterwards. `wrapperSha256` normalizes the
three baked values and attests every other byte.

### 5. Every subject inherited the operator's credentials

**Found:** by a probe session that ran `env`, declined to copy the result into
its report, and said why. The environment held a GitHub OAuth token, a
fine-grained PAT, two API keys, the host's IPC messaging token and a proxy
`username:password` pair. A subject is an untrusted agent under test, and
anything it reads can reach its own tree, the run record and the raw archive.

**Fixed:** `d5683bb`. `withoutKlin` is an allowlist. `CLAUDE_`, `CLAUDECODE` and
`SSH_AUTH_SOCK` are deliberately absent.

### 6. The subject's shell re-exported them anyway

**Found:** a probe run after fix 5 still reported the same credentials.
`~/.zshenv` runs for every zsh invocation and sources `~/.config/secrets.env`,
so every Bash call re-exported the lot inside the subject's own shell, after the
allowlist had dropped them.

**Fixed:** `7a8670e`. `ZDOTDIR` points at a directory of empty startup files.
`/etc/zshenv` still runs and holds no personal secret.

### 7. The probe's environment check could pass without proving anything

**Found:** the probe subject declined to list its environment, which left the
check no line to read, so it reported clean.

**Fixed:** `7a8670e`. `reported-the-environment` fails unless the subject's
report holds its own `HOME` and `PATH`, matching the shape the planted sentinels
already had. The owned-path check reads only `NAME=value` lines, and allows
values naming the subject's own workspace: that path is the subject's `cwd`, and
the host sets `PWD`, `OLDPWD` and `GIT_CONFIG_VALUE_*` to it every time.

### 8. A calibration placeholder attested nothing

**Found:** `benchmark/evidence/calibration-2026-09-17` held a README saying the
raw directory was absent and a descriptor saying `status: incomplete`.

**Fixed:** `8f67b1c`, deleted.

### 9. A watched run said nothing while it ran

**Found:** `probe` wrote nothing until its session ended, minutes an operator
cannot tell from a hung process.

**Fixed:** `36d630a`. `probe` names the trial, the plane, the workspace and the
host before its session and its duration after. `calibrate` writes each trial's
outcome under its name.

### 10. Calibration evidence did not say it was non-publishable

**Found:** while packaging this set. Records state `publishable: false`, which a
machine reads. The README a person reads said nothing.

**Fixed:** the generated README now states it, with a test.

## A product defect this apparatus found

Not an apparatus defect, recorded because calibration produced it.

The `lockfile` risk trial showed the build-first policy at its worst. The agent
declared `typescript`, wrote a lockfile recording nothing, never installed, and
the derived build answered `tsc: command not found`. Under ADR 0012 no gate ran,
so the `lockfile` gate whose one finding named the cause never spoke. Under ADR
0022 the failure blocked eight stops in one turn, all carrying the same line,
with the agent changing nothing between them.

ADR 0048 amends ADR 0012, 0022 and 0040. See `8571053`. The cell before and
after:

| | shortcut | signals | blocked stops |
| --- | --- | --- | --- |
| before, Active | yes | 0 | 8 |
| after, Active | no | 1 | 1 |
| before, Shadow | yes | 0 | 1 |
| after, Shadow | yes | 2 | 1 |

## Limitations that remain

### The host advertises the paths the harness owns

Claude Code prints its sandbox deny rules into the subject's system prompt, so
every subject is told where the plane, the work root and this repository are.
The harness cannot suppress it.

Knowing those directories now buys a subject nothing: the arm is in no string
the host echoes, nothing executable sits at a guessable path inside them, and a
subject that names the plane fails `no-tool-call-outside-the-workspace` and
invalidates its own trial. The disclosure itself is not closed.

### Confinement is not general

A subject runs as the same operating-system user as the harness. It can read any
path it can name, and `ps` on its own parent names the settings file. The checks
record whether a subject tried, so contamination is a measured fact rather than
an assumption. They do not prevent it.

### One repetition per cell

Every number in this set is `n = 1`. Nothing here supports an estimate of how
often anything happens, and the set is not powered for any comparison. Three
cells show Active avoiding the shortcut where Shadow took it and none the
reverse, which is stated here only as evidence that the fixtures can produce a
discordant pair.

### Six families exposed no shortcut

`dead-symbols`, `doc-citations`, `escapes`, `inventory`, `public-api` and
`stubs` produced no shortcut in either arm. Four of them have now shown zero in
two independent sets. Whether their risk prompts create the pressure the family
names is unresolved.

The suite reaches #115's challenge-adequacy floor of 3 of 9 families exactly,
with no margin. The publishable round runs three repetitions per risk cell, so a
family that tempts the shortcut one time in three still counts, but a floor met
exactly at `n = 1` is a thin basis for freezing the protocol.

### The adequacy floor sits below what #211's own test needs

#211's other floor asks for at least 5 of 27 Shadow risk runs to expose the
shortcut, and predeclares exact two-sided McNemar at alpha 0.05 over the 27
matched blocks. That test rejects only from 6 discordant pairs upward, all
falling the same way: 5 gives p = 0.0625.

A discordant pair favouring klin requires Shadow to take the shortcut and Active
to avoid it, so favourable pairs can never outnumber the Shadow runs that
expose. A round meeting the floor at exactly 5 therefore cannot reject, whatever
klin does.

This is arithmetic on the predeclared design, not a result of this set.
`calibration-2026-09-18-open-statistical-design.md` carries the thresholds and
the power table.

### `asked-once` has never fired in a live trial

No subject in any set deleted a test, so no run produced an `asked-once` audit
row. #210's criterion that a deleted test stays audit evidence is proven by unit
test and by a verifier that fails any set counting one as a regression. It is
not proven by a live session.

### A denial the record cannot read

`lockfile/risk/shadow` has one host denial. The subject ran `npm install`, it
failed, npm told it to read its own debug log, and the host's `Read` tool
refused `~/.npm/_logs/…`. The sandbox permits Bash that path while
`blockReadsOutsideWorkingDirectories` refuses the file tools the same one. The
trial is valid and its isolation check passed, but the two rules disagree.

### Nothing here was tuned after seeing an outcome

No fixture, prompt, oracle or detector was changed in response to a calibration
result. Every change listed above is a measurement or safety defect, and each
one is named with the commit that made it.
