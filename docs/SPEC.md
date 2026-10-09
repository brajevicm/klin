# klin vNext Core Specification

Status: accepted by the owner on 2026-10-06 (#493), after seven adversarial
reviews and the frictionless revision, with the known limits of section 18.7.
This document is klin's one specification. Appendix B holds the rules of
the 0.x specification that vNext carries forward (section 0.3, ADR 0066).
`docs/vnext-spec-review-2026-10-06.md` records the review and what changed.

Purpose: define klin vNext as deterministic, repository-grounded quality
control and repair feedback for coding agents, which an independent CI
checkout enforces.

The structure follows the Symphony service specification: problem, goals,
system, domain model, configuration, execution, results, evidence,
capabilities, host protocol, public CLI, CI, reporting, performance, failure,
trust, compatibility, admission, validation.

## Normative Language

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, RECOMMENDED, MAY
and OPTIONAL are to be read as RFC 2119 describes them.

Implementation-defined means the behavior is part of the contract, but this
document does not prescribe one policy. An implementation MUST document the
policy it chose.

The vocabulary of `CONTEXT.md` applies. Section 4 adds terms. On acceptance,
`CONTEXT.md` takes the terms that section 4 marks as new.

## 0. Authority, inputs and carried-forward contract

### 0.1 Inputs

This document synthesizes these decisions. Where two inputs disagree, the
later decision wins, and section 0.4 records the resolution.

| Input | What it decides here |
| --- | --- |
| #352 to #357, #361 to #364 | Research dispositions. #475 section 16 lists them. |
| #354 | Judgement, measurement completeness and execution are distinct. Evidence is claim-local. |
| #425 | Optional, versioned, family-owned finding identity. Ambiguity keeps the current matcher. |
| #452 | The six public commands and one hidden host ingress. Project CI owns the build. |
| #475, PR #476 | The product decision (`docs/vnext-product-decision-2026-10-04.md`). |
| #484, PR #487 | The lifecycle is Stop feedback with `klin check` enforcement. No readiness phase. |
| #492 | Stop semantics for incomplete measurement, Cursor pass-through and ask tools. |
| #358 | Product invariants, Stop cost bands and the stream split with #478. |
| Owner decisions, 2026-10-06 | Section 0.4, rows marked "owner". |

`docs/vnext-product-decision-2026-10-04.md` still describes
`stop -> ready -> check`. The #484 amendment on #475 replaces those parts.
This document is the place where the amended contract is written down.

### 0.2 What this document does not freeze

- Detector semantics of code duplication. #478 and its children own them
  (section 18.4).
- The physical parser, structural-cache and base-index architecture, while
  #480 step 1 and #489 still decide what duplication needs (section 18.5).
- Exact Rust types, module boundaries and file formats of private state.
- Release counts and dates of the migration. The roadmap owns them. Section
  17 states the invariants they must keep.

### 0.3 The carried-forward contract

vNext does not restate every measurement rule of the 0.x specification.
Appendix B holds the sections below that carry forward, each under its 0.x
number with the prefix B, so the 0.x section 6.3 is section B.6.3. They stay
normative for vNext, except where a section of this document amends them. A
row that names no amendment carries the 0.x section unchanged. A replaced
section is not in Appendix B.

| 0.x section | Subject | vNext status |
| --- | --- | --- |
| 1, 2 | Problem, goals | Replaced by sections 1 and 2. |
| 3 | System overview | Replaced by section 3. |
| 4.1, 4.2 | Tree, window | Carried forward. |
| 4.3, 4.5, 4.7 | Derived values, finding, ceiling | Carried forward. |
| 4.4 | Site and matching | Carried forward. Section 8.4 adds identity. |
| 4.6 | Check and gate | Carried forward. Section 8.2 adds placement and semantics version to the row. |
| 4.8 | Accepted entry | Carried forward. Section 7.6 amends the unmatched entry, and section 7.2 adds the `measurement-lost` entry that matches by file. |
| 4.9, 4.10 | Verdict, note | Replaced by section 7. |
| 5.1 | Optional file | Carried forward. Sections 5.1 and 6.5 amend it. |
| 5.2 to 5.6 | Keys, compact policy, derivation rules, dated ceilings, exclusion | Carried forward. Section 7.3 sorts their "exit 2" cases. |
| 5.7 | `init` | Replaced by sections 5.4 and 11.2. Its pin rules carry forward under `setup --pin`. |
| 5.8 | Configuration reference | Carried forward under `klin policy --reference` (section 11.6). |
| 6.1, 6.2.1, 6.3, 6.5, 6.6 | Turn window, prompt mark, base candidates, materialization, derivation commit | Carried forward. Section 6.5 amends 6.3. |
| 6.2 | When the stamp moves | Carried forward. Section 6.6 amends it. |
| 6.4 | A base equal to HEAD | Replaced by section 6.5. |
| 7.1 to 7.3 | Ratchet semantics | Carried forward. |
| 7.4 | State directory | Carried forward. Section 6.7 amends it. |
| 8.1 to 8.3, 8.6 | Criteria, shipped checks, linter seam, per-check contract | Carried forward. Sections 7.2, 7.3 and 9 amend them. |
| 8.4, 8.5 | Tier 2 and tier 3 backlog prose | Not normative, apart from the measurement rules of the shipped `conventions`, `public-api`, `reachability` and `layering` checks. |
| 9.1, 9.8 | Host adapter, one copy per event | Carried forward. Section 10 amends them. |
| 9.2, 9.5, 9.6 | Events table, hook text, journal writes | Replaced by sections 10.3, 10.6, 10.7 and 13.1. |
| 9.3 | Block policy and build | Carried forward. Sections 6.4 and 10.4 amend it. |
| 9.4 | Guard | Carried forward. Section 10.8 amends it. |
| 9.7 | Harness protocol | Carried forward. Section 10.9 amends it. |
| 10 | Runner and CI | Replaced by sections 11.3 and 12. |
| 11.1 | Text | Carried forward for the `FAIL:`, `NOTE:` and `OK:` line shapes that section 11.3 cites. Section 11.3 replaces the rest. |
| 11.2 | JSON | Replaced by section 11.7. |
| 11.3 | SARIF output | Not shipped. A future additive output. |
| 11.4 | Journal record | Carried forward for the fields that section 13.1 keeps. Section 13.1 replaces the schema, and a reader skips schema 1 lines. |
| 11.5 | `klin stats` | Replaced by section 13.2. The counted unit carries forward as B.11.5. |
| 12 | Determinism | Carried forward. |
| 13 | Performance budget | Carried forward. Section 14 amends it. |
| 14 | Failure model | Replaced by section 15. |
| 15 | Trust model | Replaced by section 16. |
| 16.1 | The hook's window | Carried forward. Section 6.6 amends it: no `turn_reset`, no forced RED, the verdict table, advisory windows, and the fallback kept only without a remote. |
| 16.4, 16.5 | Reference algorithms | Carried forward. Section 8.4 amends 16.5. |
| 16.2 | `klin gate` and CI window | Carried forward. Section 6.5 amends it. |
| 16.3 | The hook run | Carried forward. Section 10.4 amends it. |
| 17, 18 | Test matrix, checklist | Replaced by section 19. |
| 19.0 to 19.5 | Installation and distribution | Carried forward. Section 17 amends the command names, and B.19.5 names the Action of section 12.3. |
| 19.6 | Upgrades | Carried forward. Section 8.3 amends it. |

#505 folded every carried-forward section into Appendix B and retired
`docs/SPEC-0.x.md`.

### 0.4 Conflicts resolved between inputs, and owner decisions

| Subject | Resolution | Why |
| --- | --- | --- |
| Exit code of INCOMPLETE. #475 and its #484 amendment say 3. #452, #492 and #493 say 2. | 3 for INCOMPLETE, 2 for an execution or configuration ERROR. | Owner. CI can tell a measurement hole from a broken run. |
| Precedence of FAIL and INCOMPLETE. #475 says INCOMPLETE wins. | FAIL wins: ERROR 2 > FAIL 1 > INCOMPLETE 3 > 0. | Owner. A proven failure is never hidden behind a hole. An agent cannot turn exit 1 into exit 3 by making a file unreadable, and a CI that tolerates exit 3 still fails on a proven FAIL. |
| An integration tool failure. #492 calls it "execution tool-error". | A hole, `tool-error`, exit 3. Not an execution ERROR. | #475 section 4.2 counts a missing required tool or report as incomplete measurement. The other capabilities still give a usable result. |
| How much friction klin may cause. | Almost none. A person does nothing routine. Only a FAIL that the agent can fix blocks or keeps the local window red. Everything else klin absorbs automatically or records without a demand (section 2.3). | Owner. "klin should be almost fully frictionless." |
| #492 and #493 say that incomplete, tool-error and configuration-error states "remain non-green". | They stay non-green where they are real: at `klin check` (exit 2 or 3), in the journal and in `klin report`. They never keep the local window red, because neither the agent nor a routine person action clears them there. | Owner, the frictionless rule. Local red that no one can clear is a trap. CI stays the enforcement boundary. |
| A file that klin cannot measure. 0.x made it exit 2 by hand when the change opened it. | Section 7.2: a file the base measured and the change made unmeasurable is a `measurement-lost` FAIL; any other gap the change opened is a review item with a pull-request annotation; klin's limits that the change did not open are coverage notes. | Owner, the frictionless rule and the third review. The same binary measured the file in the base (ADR 0001), so the agent can fix a lost measurement. An opened gap reaches the person in the pull request without a block. |
| #358 says `klin check` stays non-green when authoritative measurement cannot be produced, and that unmeasured is not clean. | `klin check` is non-green for a hole. A file not measured is counted in every result and never claimed as clean. A gap the change opened is a review item. | Owner, the frictionless rule. A hole is something someone can fix. An opened gap is shown where the person decides, and an inherited limit is counted. |
| `klin turn reset`. #475 removes it. | No public command moves the stamp. When incoming commits enter the turn, the branch changes, or a recorded merge-base leaves HEAD history, that Stop is advisory and takes a fresh stamp itself (section 6.6). | Owner, the frictionless rule and the fifth and sixth reviews. The merge case was the one real need for a reset. Three precise designs failed review: a one-Stop re-anchor, a branch fallback, and a second base. Degrading one Stop to a report needs no inference, and CI judges precisely. |
| An accepted entry that matches nothing. 0.x `--strict` failed on it. | A review item at `klin check`, a note at the Stop. | Owner. A semantics change must not turn CI red (#475 section 5). |
| #475 section 9 offers integration placement `ready \| check`. | No placement key. An integration runs at `check` only. | #484 removed `ready`. A key with one value is no decision. |
| #452 and #493 say placement is "`stop`, `check`, or both". | `{stop, check}` or `{check}`. No stop-only quality capability. | #475 section 3 makes placement monotonic, and CI is the enforcement boundary. |
| #475 section 10.3 asks for a migration error when a configuration relied on SARIF at Stop. | No error. `klin setup` and `klin policy` say that a `sarif` entry runs at `klin check` only. | Without a placement key, no configuration shape proves reliance on Stop. |
| #475 section 5 lists "measured working-tree identity" in the basis. | `klin check` records HEAD and whether the tree was dirty. It computes no tree hash. | A CI checkout is clean, so HEAD identifies the tree. A tree hash costs 300 to 410 ms at 10,000 files (#484 section 9). |
| An unknown harness protocol version. #492 says a non-code failure spends no block. | The fail-closed rule of ADR 0047 stays: such a harness blocks every Stop. | A harness that speaks another version cannot read klin's answers, so an open answer is unsafe. It is a protocol mismatch, not a measurement state. |
| #475 sections 3, 7, 8 and 13 describe readiness. | Not in vNext. Section 18.6 states the bar to bring readiness back. | #484. |

### 0.5 Decisions this document reverses or amends

Each row is an ADR that this document changes. ADR 0066 records that this
document is the one specification, with the carried-forward 0.x rules in
Appendix B. The other rows take an ADR amendment
when the roadmap ticket that implements them lands.

| ADR | What it decided | What vNext does instead | Why |
| --- | --- | --- | --- |
| 0003, 0021 | A file no grammar reads is exit 2 outside the hook when the change opened the hole. | A file the base measured and the change made unmeasurable is a `measurement-lost` FAIL. Another gap the change opened is a review item. An inherited limit is a coverage note. | Section 7.2. |
| 0010 | `--strict` and the meaning of naming a gate. | No `--strict`. Every `klin check` judges the holes 0.x added under `--strict` (section 11.3). | #452. |
| 0012 | The build is a config key, and klin runs it. | The build is local Stop feedback only. `klin check` never runs it. | #452, #453. |
| 0013 | A base equal to HEAD is refused when work is hidden. | Section 6.5. A refusal is INCOMPLETE, exit 3. | `--strict` is gone, and the case is an unproven comparison. |
| 0014, 0024 | `klin radius --report` is a person's command. | No public radius command. The radius note reaches the person at prompt time. | #452. |
| 0017, 0032 | `klin turn reset` moves the stamp to the current tree. | No `turn reset`. A history move makes one Stop advisory, and that Stop takes a fresh stamp. The 0.x branch fallback stays only in a repository with no remote. | Owner. Section 6.6. |
| 0028 | `klin gate` without `klin.json` is exit 2. | `klin check` runs under `{}`. The file stays the hooks' opt-in marker. | A person or CI ran `klin check` on purpose. |
| 0031 | A deleted test is asked about once, and CI only notes it. | It is asked about once at Stop, and it stays a review item at `klin check`. | #475 section 11. |
| 0034 | An intervention is a spent gate block. | Unchanged. An ask about a deleted test that spends a gate block is an intervention. | Stated so the report counts stay as they are. |
| 0045, 0052 | Cursor tells a person-only note by `followup_message`. | Cursor never tells a non-blocking note by `followup_message`. | #492. |
| 0046, 0053, 0056 | `klin install` is the standalone reconciler and the first install route, and writes every host a repository does not show. | `klin setup` takes those roles with the same host selection. | #452. |
| 0047 | A custom harness speaks protocol v1 through the shipped commands. | It speaks protocol v1 through `klin __agent event`. The schemas do not change. | #452. |
| 0052 | A tool error after a gate block spends gate block 2. | An opened gap, a coverage note, a hole and a configuration error spend no gate block and do not keep the window red. | #492 and the frictionless rule. |
| 0054 | The red pass-through names `klin stats --turn`. | It names `klin report`. | #452. |
| 0037 | `klin conventions --report [NAME]` summarizes each convention with its sites, or explains one. | `klin policy conventions [NAME]` explains each convention, or one, and counts no match (section 11.6). | #507. `policy` runs no check. |
| 0044 | `klin public-api --report` prints the derived contract of the working tree. | `klin policy public-api` prints it (section 11.6). | #507. No per-check command survives. |
| 0036 | The catalogue is one ordered table, and each check is also a Clap command. | The catalogue also declares each capability's placement and semantics version. No check has a command of its own: `klin check NAME` selects a row, so a new check is one row. A row may declare how `klin policy` explains it. | Sections 6.2, 8.2 and 11.6. #507. |
| 0042 | Large-repository budgets. | The budgets stay. A 1,500 ms admission envelope is added. | #358, #475 section 14. |

## 1. Problem Statement

A coding agent optimizes for a green result at the end of its turn. The
cheapest routes to green are the ones a reviewer dislikes most: a silenced
check, a skipped or deleted test, a stub left behind, a function that grew
past what the project tolerates, a broken public contract, a dependency that
the lockfile does not hold. A language linter catches few of these, because
each one is legal code.

klin closes those routes with deterministic, repository-grounded checks. It
gives the agent concise repair feedback at the end of its turn, and it lets an
independent CI checkout enforce the same policy. It solves these problems:

- It compares two trees and refuses only what got worse. A project with debt
  adopts it on day one and stays green.
- It needs no configuration to start. It derives every fact it needs from the
  tree and prints the facts. A person pins a value only to override it.
- It hands each failure to the agent as a repair task, with a direction that
  fixes the problem, not one that hides the metric.
- It keeps missing evidence and tool failure visible. It never reports them as
  clean code, and it never reports them as a code failure.
- It runs offline, in the time budget of a hook.

Boundaries:

- klin is not a linter, a type checker, a build system or a test runner. The
  project's own tools and CI own those.
- klin does not decide that a task is semantically complete. A Stop is the
  observable end of an agent turn. It is not a declaration of readiness.
- A local hook is Feedback. Only a CI run on a checkout that the agent never
  touched is Enforced (section 16).

## 2. Goals and Non-Goals

### 2.1 Goals

- `{}` is a complete, useful configuration, and the configuration file is
  optional for `klin check`.
- Ordinary first-class-host use needs no routine human command after
  `klin setup`.
- One measurement engine serves the Stop and `klin check`. They select work
  from it. They are not two implementations.
- Every result keeps three distinct axes: judgement, measurement and
  execution.
- REVIEW never looks like FAIL, and missing evidence never looks like PASS.
- Existing debt stays held. Only new or worsened debt fails.
- Every measurement is deterministic: the same two trees, the same policy and
  the same binary give the same result on any machine.
- The ordinary Stop stays fast, bounded and repair-oriented.
- A disputed result exposes enough policy, measurement basis and site
  evidence to reproduce the claim.

### 2.2 Non-Goals

- A general AI-slop classifier, an AI-authorship detector, or a global
  quality score.
- Generic SOLID, design-pattern or similarity verdicts. This does not
  restrict the code-duplication capability of section 18.4.
- A repository-wide semantic index without a concrete admitted consumer.
- Running the project's build, tests or type checker as part of `klin check`.
- A readiness or finalization phase. Section 18.6 states the bar to admit
  one.
- PostTool quality hooks, ask-tool interception, an MCP lifecycle API, a web
  UI or a service.
- Preventing a determined person or agent from working around a local hook.
- Forcing an agent to pay down debt it did not add.
- Matching another tool's numbers. A ratchet needs self-consistency only
  (ADR 0001).

### 2.3 Frictionless by default

klin should be almost fully frictionless. These rules decide every case
where friction and strictness pull apart:

1. A person does nothing routine. After `klin setup`, no person command is
   part of the normal flow.
2. Only a FAIL that the agent can fix in code blocks a Stop or keeps the
   local window red. The deleted-test question of section 9.2 is the one
   other block, and it shares the FAIL's block. A deletion klin has not yet
   asked about keeps the window red until it can ask, except across an
   advisory Stop (section 6.6), which tells it and leaves it to the review
   item at `klin check`.
3. Everything else is told once per stamp and recorded: klin's own
   measurement limits, configuration errors, review items. None of them makes
   a demand on the agent or keeps the window red. "Once per stamp" means that
   the stamp records what it told, as it records `asked`, and a later Stop
   under the same stamp does not repeat it.
4. klin recovers its local state by itself (section 6.6). No public command
   resets it.
5. CI is non-green only when someone should act: a proven FAIL, evidence the
   project configured that broke, a comparison klin cannot prove, or a
   configuration error.
6. A person decides in the place they already look: the pull request. Review
   items appear as annotations there.
7. Frictionless never means a false claim. A pass never claims a file klin
   did not measure, and the coverage counts say what it did not measure.

## 3. System Overview

### 3.1 Components

One binary holds these components.

1. **Config** loads `klin.json` when it exists and rejects keys klin does not
   know (section 5).
2. **Facts** derive repository facts: source files, roots, documents,
   manifests, test roots, families. They are read once per run and shared by
   every capability (B.4.3, ADR 0038).
3. **Window** chooses the two trees that a run compares and names the
   derivation commit (section B.6, section 6.5).
4. **Catalogue** is the one ordered table of capabilities. Each row declares
   its placement, activation, needs, semantics version and labels (section
   8.2).
5. **Engine** runs the selected capabilities over the window. It returns one
   semantic result: findings, review items, notes, measurement records and
   errors (section 7). The engine is the only place that measures code.
6. **Ratchet** matches findings against the base and the accepted list and
   sorts them into new, worsened and held (section B.7, section 8.4).
7. **Host protocol** reads a host event through `klin __agent event`, applies
   the Stop policy, and writes the decision in the host's shape (section 10).
8. **Guard** answers pre-tool events about writes to the guarded set (B.9.4,
   section 10.8).
9. **Journal** records each Stop, prompt and guard refusal in the state
   directory (section 13.1).
10. **Renderers** turn one semantic result into agent text, person text and
    JSON (sections 10.6, 11.3 and 11.7).

### 3.2 Layers

- The measurement layer is the engine, the capabilities and the ratchet. It
  MUST NOT read a host event, the journal or the Stop budget.
- The policy layer is config, facts, window and catalogue.
- The integration layer is the host protocol, the guard, the journal, the
  public commands and the renderers. It MUST NOT read source code except
  through the engine.

### 3.3 The two execution paths

```text
LOCAL (Feedback)
  session / prompt event  -> stamp, prompt counter, radius note
  pre_tool event          -> guard
  agent works
  stop event              -> optional build feedback
                          -> engine(placement = stop, window = turn, scope = changed)
                          -> Stop policy: block #1, changed-tree block #2,
                             then report and pass through
  person

ENFORCED
  klin check in an independent CI checkout
                          -> engine(placement = check, window = branch | push)
                          -> exit 0 | 1 | 2 | 3
```

A person who runs `klin check` locally gets the same command and the same
semantics as CI. Only the trust boundary differs (section 16).

## 4. Domain Model

Terms marked *new* are added by vNext. The others are in `CONTEXT.md` or the
0.x specification.

### 4.1 Trees, windows and the base

- **Tree**, **Window**, **Base**, **Derived**, **Pin**: as `CONTEXT.md` and
  sections B.4.1 to B.4.3 define them.
- The Stop uses the `turn` window. `klin check` uses the `branch` or `push`
  window (section B.6, section 6.5).

### 4.2 Capability, check, integration, recipe (*new*: capability, integration, recipe)

- **Capability**: one row of the catalogue. It is a native check, a named
  klin-owned recipe, or a user-owned integration.
- **Check**: a native capability. klin owns its semantics. `CONTEXT.md`
  defines it.
- **Recipe**: a named capability that runs an external tool under a contract
  klin owns. No recipe ships. Section 9.3.
- **Integration**: a capability whose command, version and configuration the
  project owns. `sarif` is the one integration kind. Section 9.4.
- **Gate**: one configured or derived instance of a capability, as
  `CONTEXT.md` defines it. A `sarif` entry is one gate.

### 4.3 Placement (*new*)

The set of execution paths at which the engine runs a capability: `{stop,
check}` or `{check}`. The catalogue owns placement. Configuration cannot
change it. Section 6.2.

### 4.4 Evidence items

- **Finding**: one violation at one site, as `CONTEXT.md` and B.4.5 define
  it. A finding has outcome `new`, `worsened` or `held`. Only `new` and
  `worsened` fail.
- **Review item** (*new*): an observation that needs a person's judgement. It
  never fails a run and never asks the agent to clear it.
- **Note**: something klin reports and never fails on. A note does not change
  the judgement. Advisory evidence whose claim is unknown is a note.
- **Hole** (*new*): one explicit reason why a required measurement is not
  complete. Section 7.2.

### 4.5 Result axes (*new*)

- **Judgement**: `pass`, `review` or `fail`.
- **Measurement**: `complete` or `incomplete`.
- **Execution**: `ok` or `error`.

Section 7 defines them and their aggregation.

### 4.6 Measurement basis (*new*)

The facts that say what a measurement measured and whether two measurements
mean the same thing. Section 8.1.

### 4.7 Finding identity

An optional, versioned, family-owned key that pairs a finding with its
counterpart in the other tree. Section 8.4.

### 4.8 Local state

- **Session**, **Turn**, **Journal**, **Intervention**, **Regression**: as
  `CONTEXT.md` defines them.
- **Turn stamp** and **prompt mark**: B.6.2 and B.6.2.1.
- **Build stamp**: the per-prompt block counts and the trees the last blocks
  were taken over (B.9.3).
- **Handoff record**: the hash of a message that a host may submit as a
  prompt (B.9.1).
- **Advisory Stop** (*new*): a Stop that measures and blocks nothing,
  because the turn's history moved under the stamp (section 6.6).

### 4.9 Trust levels

- **Feedback**: local hooks and a local `klin check`.
- **Enforced**: `klin check` in an independent CI checkout under the
  conditions of section 16.2.

## 5. Configuration Contract

### 5.1 The file

The rules of B.5.1 to B.5.6 carry forward: `--config PATH`, compact policy,
derivation rules, dated ceilings, exclusion by `false`, and unknown-key
errors. Discovery changes.

**Discovery.** Every command and the hooks read one file: `klin.json` at the
worktree root. The worktree root is found by a filesystem walk up from the
working directory, or from the event's tree, to the first directory that
holds a `.git` directory or a `.git` file whose first line is `gitdir: PATH`
with an existing `PATH`. The walk starts no git process. It does not see
`GIT_DIR`, `GIT_WORK_TREE` or `core.worktree`. A repository that relies on
them names its configuration with `--config PATH` for `klin check`, and its
hooks stay silent. A `klin.json` below the worktree root is never read.
`klin check`, `klin status` and `klin policy` print a note that names it as
ignored. A `--config PATH` that names no file is exit 2. Outside any worktree there is
no root: `klin check` and `klin policy` read the `klin.json` of the directory
they start in, `klin status` is exit 2, and the hooks stay silent.

The hooks' walk passes every directory between the event's tree and the
worktree root, and tests each for a `klin.json` without parsing it. When it
finds one below the root and none at the root, the hooks stay silent as an
opt-out, and tell one person notice per session that names the file and
says to move it to the worktree root. The `session` event tells it, which
makes it once per session with no state. It uses the host's person channel
of section 10.7, and stderr on Cursor, because a repository that has not
opted in has no journal. With no state, the copies of klin's hooks cannot
share a claim (section 10.3), so each installed copy tells the notice once. This replaces the 0.x support for a
configuration below the repository root, which
`a_config_below_the_repository_root_holds_the_debt_the_base_holds` and
`a_config_below_the_repository_root_scopes_a_changed_run_the_same_way` in
`tests/base.rs` pin. The roadmap ticket that implements discovery rewrites
those tests.

The file is the repository's opt-in marker for the host protocol (ADR 0028).
When the worktree root holds no `klin.json`, `klin __agent event` answers
every event with no decision, prints nothing, writes no state and exits 0. It
does not read or parse the file for this test. The one exception is a harness
event of a protocol version klin does not speak: its fields name no tree klin
can trust, so it is refused wherever it runs (section 10.9), with no journal
line outside an opted-in tree. The plugin's one-time hint that the CLI exists
is told only in a repository that opted in.

`klin check`, `klin status` and `klin policy` without a `klin.json` run under
`{}`. They print one line, `config: none, running under {}`, and the JSON
says `present: false`. When the base holds a `klin.json` and the working
tree holds none, `klin check` adds a note that names the deleted file.

### 5.2 Philosophy

- `klin.json` is a person's policy over facts klin derives. It never
  describes the repository: roots, languages, documents, manifests, test
  roots, families and build commands are facts.
- `{}` is complete.
- A configuration key exists only for a real policy decision of a person. An
  implementation tuning knob stays internal.
- Native placement is not configurable. No key moves a native check onto or
  off the Stop.
- Integrations have no placement key. They run at `check` only.

### 5.3 Top-level keys

The keys of B.5.2 carry forward: `build`, `accepted`, `radius`, `journal`,
and one key per gate section. vNext adds no key and retires no key. A
configuration that 0.x accepted is valid in vNext.

- `build` is local Stop feedback policy (section 6.4). It never affects
  `klin check`.
- `accepted` holds person-authored debt (B.4.8). Only a person writes it,
  in a reviewed commit.
- A `sarif` entry whose `name` equals a capability name, or equals another
  entry's name, is a configuration error.

### 5.4 Pinning

`klin setup --pin` writes today's derived guardrails as policy a person
reviews, with the rules of B.5.7 for `init --pin`: complexity `cc` and
`lines`, a `doc_size` ceiling for each instruction file the derivation commit
holds, and the `radius` values history gives. It writes a value only where
the configuration states none. It MUST NOT write topology.

## 6. Execution Model and State

### 6.1 One engine, two paths

The Stop and `klin check` call one engine. The engine takes:

- a window: `turn` for the Stop, `branch` or `push` for `klin check`;
- a placement: `stop` or `check`;
- a scope: the window's changed files, or the whole tree;
- a selection: every applicable capability of the placement, or the named
  ones.

The engine returns one semantic result (section 7). The Stop and `klin
check` MUST NOT measure or judge code in any other way. They differ only in:

- the window and placement they pass;
- what they do with the result: the Stop applies the block policy and writes
  local state, and `klin check` maps the result to an exit code;
- the renderer they use.

The Stop policy, the journal and the state directory MUST NOT change what
the engine measures or how it judges. Where this document says that an item
is a note at the Stop and a review item at `klin check`, the engine returns
one item, and the Stop renderer shows it as a note.

### 6.2 Placement

| Placement | Runs at Stop | Runs at `klin check` |
| --- | --- | --- |
| `{stop, check}` | yes | yes |
| `{check}` | no | yes |

Rules:

1. A capability that can produce a FAIL or a review item MUST include
   `check`. CI is the enforcement boundary, so no quality judgement exists
   only at the Stop.
2. A capability placed `{check}` MUST add no work to the Stop: no file read,
   no parse, no process, no git command, no cache read. The engine filters by
   placement before it resolves a capability's needs.
3. Every integration is placed `{check}`.
4. A native check enters `{stop, check}` only under the admission rules of
   section 18.2.
5. Local feedback that is not a capability, such as the build (section 6.4)
   and the radius note (section 10.3), is not subject to placement and is
   never part of a `klin check` result.

### 6.3 Side effects

| Path | Reads | Writes |
| --- | --- | --- |
| Stop | config, facts, window, caches, state, HEAD, its reflog, the default-branch ref | turn verdict, told record, build stamp, handoff records, claims, journal, caches, the merge-base cache, the records of a stamp that had none, and on an advisory Stop a fresh stamp, its ref and the prompt mark |
| session, prompt | state, working tree, HEAD, its reflog, the default-branch ref | stamp with its records, prompt mark, prompt counter, handoff records, claims, journal |
| pre_tool | event, state directory path | journal line for `ask` and `deny` only |
| `klin check` | config, facts, window, caches | caches, and a temporary base worktree that it removes |
| `klin status`, `klin report`, `klin policy` | state, journal, config, facts | nothing |
| `klin setup` | config, host files | `klin.json` (only when absent, or under `--pin`), host integration files |
| `klin update` | the installed binary, the network | the installed binary |

Rules for `klin check`:

- It MUST NOT write the turn stamp, the build stamp, the journal or any
  handoff record. A run by an agent mid-turn therefore changes no Stop budget.
- It MUST NOT wait on the Stop's state lock. Cache writes are atomic
  replacements (ADR 0041) and take no lock that a Stop waits on.
- A full run lays the base out as a temporary worktree of its own (B.6.5).
  It removes the worktree and prunes its registration before it exits. A
  later run removes only a worktree whose owning process is gone, so two
  concurrent runs never remove each other's.

### 6.4 Build feedback

The build is local Stop feedback that is separate from the quality
judgement. The rules of B.9.3 carry forward:

- The Stop runs the configured or derived build before it measures, so the
  agent does not receive measurements over code that does not build.
- A build failure blocks each Stop that changed the tree since the last build
  block, up to eight per prompt, and writes a red verdict first.
- A build whose shell exits 127 is an absent tool. It is a note, not a build
  failure (ADR 0048).
- Command limits, deadlines and process-group cleanup are as B.9.3 states.
- B.9.3's sentence that the build rule "does not constrain a future
  agent-readiness path" does not carry forward. vNext has no readiness path
  (section 18.6).

`klin check` MUST NOT run the build, MUST NOT read the build stamp, and MUST
NOT claim that the project builds. The project's own CI owns build, test,
type check and install (section 12.2). The build has no Stop time budget of
its own. Section 14 excludes it from every Stop measurement.

### 6.5 Windows of `klin check`

The base candidates of B.6.3 carry forward, with these rules:

1. **A missing candidate fails only where more history fixes it.**
   - In a shallow repository, a candidate that is present and does not
     resolve (`GITHUB_BASE_REF`, or a push `before` that is not the null
     commit), or a merge-base that cannot be computed, is an execution ERROR,
     exit 2. The message names the missing commit and `fetch-depth: 0`.
   - `GITHUB_BASE_REF` that does not resolve in a full checkout is the same
     ERROR, because the pull request target must be fetched.
   - A push `before` that does not resolve in a full checkout was rewritten
     away, for example by a force-push. No fetch setting brings it back. The
     run falls through to the merge-base candidate and prints a note that
     names the rewritten commit.
2. **No base resolves.** Execution ERROR, exit 2, with the list of what was
   tried.
3. **A base equal to HEAD, with a clean working tree** (the 0.x case that section 6.5 replaces):
   - a remote source passes and prints that the trees are the same;
   - a local source passes when the remote default branch holds HEAD;
   - a local source whose HEAD the remote default branch does not hold is a
     hole, `comparison-unproven`, exit 3, which names the unpushed commits;
   - with no remote at all, the run passes and prints a note that no remote
     proves what to compare. This is the 0.x behavior without `--strict`.

The "same tree MUST be green" promise of B.5.1 holds with one exception: a
local source that hides unpushed commits.

### 6.6 The stamp and automatic recovery

The stamp rules of B.6.2 carry forward, with these changes.

**The stamp verdict.** A Stop writes exactly one verdict. When more than one
row applies, the first row in the table wins.

| Verdict | When | Effect |
| --- | --- | --- |
| `aborted` | klin failed during the Stop. | The stamp stays until a Stop measures. It neither blocks nor shows as red. `klin status` shows since when. |
| `red` | The build failed (section 6.4), or the result holds a failing finding, or a deleted test that klin has not yet asked about (section 9.2). An advisory Stop writes `red` only for a build failure. | The stamp stays. |
| `unjudged` | A run-scope configuration error stopped the Stop before it measured. | When the stamp was `green` before, the next session or prompt moves it, so the first Stop after a fix never judges days of work. When the stamp was `red`, it stays red. `klin status` and `klin report` show "nothing judged", never green. An advisory window stays pending until a Stop measures. |
| `advisory` | The Stop measured in an advisory window (below) and the build did not fail. | The Stop itself takes a fresh stamp. |
| `green` | The engine measured, and nothing above holds. | The next session or prompt moves the stamp (B.6.2). |

A Stop that holds the state lock writes `aborted` before it measures and
replaces it with its final verdict, so a Stop that crashes never leaves an
earlier `green` in place. A Stop that cannot take the lock writes no verdict,
as B.6.5 says.

A stamp that no Stop judged yet holds `pending`. The next session or prompt
keeps a `pending` stamp, as it keeps `red`, so the window stays open until a
Stop judges it. An `unjudged` Stop keeps an earlier `red` or `aborted`
verdict, and replaces `pending`, `green` or `unjudged`. Over `pending`, the
`unjudged` stamp records `kept`, and the next session or prompt keeps it, so
the first Stop after the fix judges the work no Stop judged before. Only an
`unjudged` Stop after a window a Stop judged `green` lets the stamp move. When no stamp exists,
the Stop writes no verdict. Only a `klin.json` that klin cannot read writes
`unjudged`. Any other error that stops the run before it measures, such as a
base klin cannot lay out, leaves the `aborted` the Stop wrote, so the next
prompt cannot move the stamp past work no Stop judged.

Review items, notes, coverage notes and errors the base had too do not keep
the stamp.

**What the stamp records as told.** The `told` record keys each note and
each error of a Stop's report by its record. A Stop records them only after
the host took the block or the notice, so a notice klin could not deliver,
such as one whose follow-up record would not write, is told at a later Stop. A later Stop under the same stamp whose every note and error
is already in `told` tells nothing. A Stop with at least one new record tells
its whole note, with the records told before it.

**Advisory windows.** Some events bring other people's commits into the turn
or take the turn's history away, so a precise local judgement is no longer
possible. klin then degrades to reporting for one Stop. The stamp records
the merge-base of HEAD with the default branch, HEAD's symbolic ref (an
in-progress rebase counts as its `head-name` branch), and the position of
HEAD's reflog. A Stop is advisory when, since the stamp was taken, one of
these happened:

1. **Incoming commits.** The default-branch merge-base moved forward, so
   the recorded one is an ancestor of it, and HEAD's
   reflog since the recorded position holds a `merge`, `pull`, `rebase` or
   `reset` entry, or the `commit (merge)` entry of a merge that stopped on a
   conflict. The default branch is found by candidate 3 of B.6.3
   without the GitHub variables, and only a remote-tracking ref
   (`refs/remotes/...`) counts. The reflog is compared by entry position,
   never by time. When HEAD has no reflog, a merge-base that moved forward
   alone is advisory. A reftable repository has no HEAD reflog when
   `git reflog exists HEAD` says so, even where `git reflog show` prints an
   empty log.
2. **A branch change.** HEAD's symbolic ref differs from the recorded one,
   and the stamp's parent is no longer an ancestor of HEAD. `git switch -c`
   at the same HEAD keeps the turn window, as B.6.2 did. A stamp that
   recorded no ref, such as one restored from its ref, takes the ancestry
   test alone.
3. **Lost history.** The recorded default-branch merge-base is no longer an
   ancestor of HEAD, for example after the default branch was rewritten and
   the agent reset onto it, or after a reset onto history that shares no
   merge-base with the default branch. Where HEAD still shares a merge-base
   with the default branch, the history is lost only when the default branch
   no longer holds the recorded merge-base either, as after someone rewound
   or rewrote it. The agent's own rewrite of commits the default branch still
   holds, such as a `reset --soft` or an amend after a push of the default
   branch, is not lost history. A Stop that is not advisory records only a
   merge-base that moved forward, so a rewind fetched before the reset onto
   it is still lost history at the Stop after the reset.
4. **Missing state.** The stamp and its ref are both missing (B.6.2).

Where HEAD has a reflog, the agent's own work never triggers these rules: a
commit, a push, an amend, a `reset --soft` or an interactive rebase of the
turn's commits on the same branch leaves the default-branch merge-base and the
branch as they were, or moves the merge-base with no incoming reflog entry. The
turn window then stays precise, because `before` is the stamp tree. When such
a rewrite drops the stamp's parent from HEAD history, the stamp keeps its
tree, and the derivation commit stays the stamp's parent, which the stamp
commit keeps readable.

A Stop that is not advisory records the history as it stands on the stamp.
So after the agent's own push or `git switch -c`, a later Stop compares
against that history and not against the history at the stamp.

An advisory Stop:

- runs the build feedback of section 6.4 as usual, and a build failure still
  blocks under the build budget;
- measures against the stamp as usual, and blocks nothing for a finding;
- tells its failing findings, review items and unasked deleted tests once,
  as notes, with one line that says the history moved, that klin does not
  block until the history settles, and that `klin check` judges the branch;
- takes a fresh stamp of the tree it measured, with the current records and
  empty `asked`, `told` and `intervened`, and moves the prompt mark, unless
  the build failed. It captures the tree while it holds the Stop's state
  lock, because the verdict and the stamp must describe the same tree. It
  captures it before the build runs, so the fresh stamp holds no build
  output;
- writes its journal line with the verdict `advisory` and its reason:
  `incoming-commits`, `branch-changed`, `history-lost` or `stamp-missing`.

A Stop that cannot take the state lock applies the same rules. When they
make it advisory, it blocks nothing for a finding and tells what it found,
and it takes no fresh stamp, as it writes no verdict. Its journal line keeps
the advisory reason. It took no fresh stamp, so the next Stop that holds the lock
is advisory again and tells what it found.

An advisory Stop for a missing stamp has no stamp to measure against. It
measures against the B.6.3 base, as the branch fallback does.

Because the advisory Stop takes the fresh stamp itself, the next Stop is
ordinary, also within the same prompt, in a host continuation, and on a
host whose prompt hook does not run. The prompt counter carries on.

**Repositories with no remote.** When no `refs/remotes/*` ref exists, no CI
can judge what an advisory Stop would skip, so such a repository takes no
advisory Stop. Rule 1 cannot fire. For rules 2 and 4 it keeps the B.6.2
branch fallback: the Stop judges a branch window, blocks as any Stop does,
writes the base it judged as the stamp with the verdict its gates gave. The
fallback's base is the B.6.3 base. The stamp's parent is not the base,
because it sits on the branch the checkout left, and a window from it would
read that branch's tests as deleted tests. Its journal outcome stays
`branch-fallback`. When remote refs exist but the default
branch is not a remote-tracking ref, rules 1 and 3 cannot fire, and rules 2
and 4 are advisory.

**Consequences.**

- A merge, a pull or a rebase of the default branch, and a branch switch,
  never block the agent on other people's code.
- Debt the agent left before the history move is not held against it locally
  after the fresh stamp. Where CI runs `klin check` (section 16.2), CI judges
  the branch. Where it does not, that debt goes unjudged, and section 16.1
  says so.
- An agent can make one Stop advisory by merging the default branch or by
  switching branches. Section 16.1 records this Feedback limit.
- Where HEAD has no reflog (`core.logAllRefUpdates` off), the agent's own
  push of the default branch moves the merge-base, so that Stop is advisory.
  Section 16.1 records this Feedback limit too, and section 18.7 lists it.
- A stamp restored from its ref has no recorded merge-base or reflog
  position. Its first Stop compares the default-branch merge-base of the
  stamp's parent with HEAD's. That Stop is advisory when the two differ,
  because the turn may already hold incoming commits. A Stop that is not
  advisory then records them.
- Section 18.7 lists the cases these rules do not cover.

**Cost.** klin caches the merge-base under the pair (HEAD commit,
default-branch commit). `session`, `prompt` and `stop` read HEAD, HEAD's
symbolic ref, the default-branch ref and HEAD's reflog as files, after one
`git rev-parse` finds the git directory. Where the repository stores refs in
a reftable, klin asks git for them, HEAD's reflog included. `pre_tool` reads
none of them. klin starts `git merge-base` only when the pair changed, which
happens on the first Stop or prompt after each commit. An advisory Stop adds
a stamp capture to an ordinary Stop (section 14.4).

**No reset command.** No public command moves the stamp. A red window ends
when the agent fixes the finding, a person accepts it in `accepted`, the
working tree returns to a green tree, or an advisory Stop takes a fresh
stamp.

**Unusable state.** When the state directory exists but cannot be used
safely, section 15 applies: the Stop fails open, and `klin status` names the
problem. A worktree whose state directory is gone while
`refs/worktree/klin/turn` survives is not a first session: the Stop restores
the stamp from the ref (B.6.2). A person who deletes the state directory
loses the journal too. `klin status` says so beside the state directory
path.

### 6.7 State

The state directory and its contents carry forward from B.7.4, with these
changes:

- The journal `reset` kind is not written, and a reader reads none, because
  it reads no schema 1 line. A Stop line can carry the verdict `advisory`
  (section 13.1).
- The stamp also records the default-branch merge-base, HEAD's symbolic ref
  and HEAD's reflog position (section 6.6).
- vNext adds no readiness record and no readiness drift check.
- `klin cache clean` does not exist. The cache stays safe to delete by hand.
  `klin status` prints the state directory and the cache path.

## 7. Result Model

### 7.1 Judgement

| Value | Meaning |
| --- | --- |
| `pass` | No failing finding and no review item. |
| `review` | No failing finding, and at least one review item. |
| `fail` | At least one failing finding. |

A note never changes the judgement. A held finding is not failing.

### 7.2 Measurement, holes, gaps and coverage notes

| Value | Meaning |
| --- | --- |
| `complete` | Neither the run nor any capability that ran has a hole. |
| `incomplete` | The run or at least one capability that ran has a hole. |

`complete` claims only that no hole exists. It never claims that every file
was measured: the JSON and the summary always carry the count of files not
measured, and a gap that the change opened is a review item.

klin sorts missing evidence into four classes.

**1. A hole** can be fixed outside klin's own code, and someone should act
on it. It makes the measurement incomplete. At the Stop a hole never blocks
and never keeps the stamp. Only `work-limit` can occur there.

| Reason | When | Site |
| --- | --- | --- |
| `tool-error` | An integration command wrote no report, the report is stale or not SARIF, the command reached its limit, or the command was not found (detail `command-not-found`). | the gate |
| `comparison-unproven` | Section 6.5 rule 3. | the run |
| `nothing-measured` | A whole-tree run in which no capability applies to the tree, so klin measured nothing. The text says that the repository holds no language or document klin measures. A tree whose documents were measured and that holds no source is complete. A whole-tree run is this hole too when the derivation commit held a source root that no source file of the working tree sits under (a file of no language klin reads does not count) and no check that reads code measured a file, so a tree does not pass on its documents after its source went. It never applies to a changed scope. | the run |
| `unsupported` | A selector named a capability that does not apply to the tree, or one that needs a policy section the configuration does not hold. | the gate |
| `work-limit` | A capability stopped at a deterministic bound on its own work before it covered its scope (#354 section 11). The text names the bound and the person's action: narrow the capability's scope with `except`, or set it to `false`. A capability with a work bound MUST show that the controlled 1M row does not reach it. | the gate |

**2. A lost measurement** is a failing finding of kind `measurement-lost`.
The base measured a file, the working tree still holds the path, and klin
can no longer measure it for a reason that depends only on the file's bytes
or its form. The agent can fix it, and it closes the route of hiding a
finding by making a file unmeasurable.

- **Parse.** A grammar-read file is lost when the base's strict parse had no
  error node and the working tree's strict parse has one. The tolerant
  readers that `conventions` and the Rust structural reader use never lose a
  file.
- **Line ceiling.** A file is lost when the base read it under the
  source-line ceiling of section B.13 and the working tree's copy holds a
  line over it.
- **Manifest.** A manifest or lockfile is lost when the base read it and the
  working tree's copy is no longer valid JSON, TOML or lockfile syntax.
- **Form.** A file is lost when the base read it as text and the working
  tree's copy is not text: it holds a NUL byte, the working-tree path is a
  symbolic link by `lstat` (the stamp tree records the same mode), or a
  `.gitattributes` change gives it `binary`, `-diff`, or an encoding klin
  cannot decode. `-text` alone is not
  a lost form, because it only turns off end-of-line conversion.
- **Decoding.** The base side is the stored blob, which needs no decoding.
  For the working-tree side, klin applies the path's `working-tree-encoding`
  attribute natively and removes carriage returns before it measures lines,
  so a CRLF checkout or `core.autocrlf` never trips the line ceiling. klin
  decodes UTF-8, and any other `working-tree-encoding` is an encoding klin
  cannot decode. klin reads attributes only from the in-tree `.gitattributes`
  files of each side, never from `.git/info/attributes` or
  `core.attributesFile`, so two machines agree. It matches their patterns as
  git does: `*` and `?` stay inside one directory, `**` spans directories
  only where it stands alone between slashes or at either end, and a pattern
  in double quotes may hold spaces. It expands the `[attr]` macros that the
  top-level `.gitattributes` defines, and git's own `binary`. klin reads the first mebibyte
  and the first 10,000 lines of each `.gitattributes` file, ignores a
  pattern longer than 256 bytes, and expands one line's macros to at most
  256 words. What klin ignores changes only how a form is
  reported, never what is measured. An attribute never takes a
  file out of measurement: the capabilities that read the file still measure
  its bytes, and the run sorts the form the attribute gives it. Each such
  capability says whether it measured the file at the base, and that
  evidence decides the class, because a reader such as a text convention
  measures a file no strict grammar reads. So no
  difference between klin's reading of the attributes and git's can hide a
  finding. An encoding that
  klin cannot decode is a coverage note where the base gave the path one
  too, a lost form where the change added it to a path the base measured,
  and an opened gap on a new path. Invalid UTF-8 alone is not a lost form, because 0.x readers
  decode it lossily and still measure.
- **Filters.** klin runs no filter program. A `filter` attribute that the
  base already gave a path, such as an LFS path, makes that path a coverage
  note with reason `filtered`. A `filter` attribute that the change added to
  a path the base measured is a lost form, `measurement-lost`. A new path
  with a new `filter` attribute is an opened gap with reason `filtered`.
- **Known limits.** git's inexact rename scoring can change between git
  versions, so a local Stop and CI may classify the same heavily edited move
  differently. Past 1,000 inexact candidates, a move reads as a deletion and
  a new file.
- **Deletion.** A path that the working tree no longer holds is a deletion,
  never a lost measurement.
- **Renames.** Every git command that klin runs to find changes pins rename
  detection to `-M50%`, exact renames without a limit and inexact renames
  under a fixed limit of 1,000 candidate files, and ignores the person's git
  configuration for both. Those commands run no filter program either: klin
  turns off every filter driver git's configuration names and every
  end-of-line check, and writes nothing to the repository. Where git still
  cannot pair a moved file with the path the base held it at, the run stops
  with a `git` error instead of reading the move as a new file. A
  file detected as renamed is compared with its base copy. The base bytes are
  read with the base path's reader, and the working-tree bytes with the
  current path's reader.
- **Scope.** The finding fires when at least one selected capability reads
  the file. A run that selects only `doc-size` does not parse source files
  for it.
- **Identity.** The finding is keyed by the file. Its `id` is the hash of
  `measurement-lost` and the file, its `check` is null, its `line` is null,
  and its `text` is the file path. Its `values` hold the reason and, for a
  parse, the line and column of the first error node. No value is ratcheted.
  It renders as its own row, named `measurement-lost`, whose `kind` in the
  JSON is `built-in`. The row appears in a run that holds a lost file.
- **Other rows.** The capabilities that read the file count it as not
  measured and add no note for it. A form that an attribute gives is the
  exception: those capabilities still measure the file's bytes. A capability reports no finding that
  depends only on the contents it could not read, such as a `public-api`
  item that the file declared at the base.
- **Remedy.** For a parse, the remedy names the line and column of the first
  error node and says to make the file valid in its language. For the other
  reasons, it names the reason.
- **Grammar lag.** A valid construct that klin's grammar does not read yet is
  the one false positive. The Stop text says to the person that a person can
  hold the file in `accepted`, and that `klin policy` shows how. It never
  says so to the agent: the Stop says it only when it does not block, through
  the host's person channel, which on Cursor is the journal (section 10.7). The entry is `{"gate": "measurement-lost", "file":
  PATH}`, and it matches by file alone, for every capability (an amendment
  of B.4.8, which otherwise requires `text` and values). The entry counts
  as matched while the file is still lost for any reason, so it never
  becomes a stale review item while the grammar still lags. Once klin can
  measure the file, the entry is unmatched (section 7.6). `measurement-lost`
  is a reserved name: no capability, `sarif` entry or other accepted entry
  may use it.

**3. An opened gap** is a file or form that the change made unmeasurable in a
way the agent did not clearly cause. It is a review item of kind
`unmeasured` at `klin check`, one per file and reason, with a pull-request
annotation (section 12.3). At the Stop it is a note that the agent sees too,
so it can fix a file it broke by mistake. It never blocks and never changes
the exit code.

| Reason | When the change opened it |
| --- | --- |
| `unreadable` | A file in a language klin reads, which the base did not hold and git did not detect as a rename, whose strict parse has an error node. |
| `not-text` | A file in a language klin reads, which the base did not hold, and which is not text by the form rule above. |
| `filtered` | A new path with a `filter` attribute or an encoding klin cannot decode. |
| `resource-limit` | A file that the base did not hold, with a line over the source-line ceiling. |
| `unresolved` | A form that a resolver supports and could not resolve, which the base did not hold at that site. |
| `ambiguous` | Evidence whose claim needs a resolution the facts do not prove, which the base did not hold at that site. |
| `left-scope` | A file both trees hold, measured in `before`, not in `after`, because a file other than `klin.json` changed the facts or a discovery rule. klin measures the file once under the base's scope: a new or worsened finding there stays a FAIL, and only a clean file is the review item. Today `layering` is the one capability whose scope a manifest narrows, because its module graph attaches files through manifests: it resolves the working tree's facts under the base's manifests. A file the working tree no longer holds at the exact path the base held, such as one a case-only rename left behind on a file system that ignores case, did not leave a scope. |

**4. A coverage note** is klin's own limit that the change did not open. It
never makes the measurement incomplete, never blocks and never keeps the
stamp. The coverage counts and the summary still count its file as not
measured.

- Any reason of the opened-gap table that the base holds at the same site
  with the same reason.
- `left-scope` caused by a change to `klin.json`, which a person made in a
  reviewed commit. A capability measures the base under the base's own
  `klin.json` scope and the working tree under today's, so a file that
  today's `in` or `except` drops is the person's decision. Each capability's
  reason decides its own loss: a person's exclusion in one capability never
  covers what a manifest took from another.

**The named 0.x holes.** Each case that section B.8 names as a file or form
klin could not measure falls into one class:

| 0.x case | Class and reason |
| --- | --- |
| A file no grammar reads (8.6) | Section 7.2 by its history: `measurement-lost`, `unreadable` gap or coverage note. |
| A form a resolver supports and could not resolve: a TypeScript path alias that names no file, a Rust path no module holds (8.2.1) | `unresolved` gap or coverage note. |
| A module two files provide, a name two globs provide, a cyclic re-export in `public-api` (8.2.1) | `ambiguous` gap or coverage note. |
| A manifest `lockfile` cannot read (8.2.1) | `measurement-lost` when the base read it, otherwise an `unreadable` gap or coverage note. |
| A file measured in `before` and not in `after` (8.6) | `measurement-lost` or `left-scope`, as above. |

An implementation MAY add a reason to a class. It MUST NOT move a reason
between classes without a change to this document. A consumer MUST tolerate
an unknown reason.

These are none of the four:

- a file in a language that a capability does not read, which the `not_read`
  coverage count records;
- a capability that does not apply to the tree and that no selector named,
  which is a row with state `not-applicable`;
- advisory evidence that a capability's contract marks as optional, which is
  a note when its claim is unknown;
- an empty changed scope, or a changed scope with no file a capability
  reads, which is complete and says so in its coverage counts.

Resource exhaustion never yields a clean claim. Findings validated before
the limit stay usable. A file over a byte ceiling falls under the classes
above, and a capability's own work bound is a `work-limit` hole.

### 7.3 Execution and errors

| Value | Meaning |
| --- | --- |
| `ok` | No error. |
| `error` | At least one error of the kinds below. |

Error kinds:

| Kind | Scope | Examples |
| --- | --- | --- |
| `invocation` | run | An unknown flag, an unknown selector, a selector of a gate set to `false`, a `--config PATH` that names no file. |
| `configuration` | run | An unknown key, a malformed section, a schedule with no due step, an empty `doc_size` map, a duplicate `sarif` entry name. |
| `configuration` | capability | An explicit `in` that selects no applicable file, a pinned document that is missing, a convention whose `in` measures nothing, a convention that cannot run. |
| `base` | run | Section 6.5 rules 1 and 2. |
| `git` | capability | A base read that git could not finish, or a base tree klin could not lay out for a capability that reads it. The capabilities that do not read the base tree still measure and report. |
| `internal` | run or capability | A klin failure that prevents a trustworthy result. |

Rules:

- A run-scope error stops the run before any capability measures. The JSON
  sets `judgement` and `measurement` to `null`.
- A capability-scope error marks that capability's row `execution: error`.
  The other capabilities still measure and report. The run's judgement and
  measurement cover the capabilities that ran.
- Every "exit 2" case of the carried-forward sections B.5 and B.8 is one of
  the kinds above. A case that names a file klin could not measure is a
  `measurement-lost` finding, an opened gap or a coverage note of section
  7.2, not an error.
- **Moved and deleted policy paths.** A path that the policy names (an `in`
  path, a pinned document, a convention's `in`) and that selects files in the
  base and none in the working tree is decided by what happened to those
  files, by a deterministic test on both trees. A pinned document names
  exactly one file, never a directory below it. One that the working tree
  holds did not move, even where no walk reaches it. A rename of it is
  followed wherever it goes, under a skipped directory too, and an
  instruction file it was renamed to is judged once, under the pin:
  - **Renamed.** When git detects every selected file as renamed (`-M50%`),
    the policy follows the rename for this run. The capability measures the
    new paths under the pinned values, so it stays switched on. `klin check`
    adds a review item of kind `moved-pin` that names the old and new path, so
    a person updates the pin in the pull request. No block and no error. The
    Stop follows the rename without a note.
  - **Deleted.** When the selected files are gone and not renamed, it is a
    note at the Stop and a review item of kind `moved-pin` at `klin check`.
    No error. A convention whose every `in` path the change deleted measures
    nothing, so it derives no language and is no error.
  - **Mixed.** When some selected files were renamed and the rest deleted,
    the renamed files are measured at their new paths under the pin, and one
    `moved-pin` review item names the path.
  - **Selects nothing in either tree.** A pinned path that selected nothing in
    the base either, such as a pin left behind after an earlier move merged,
    is a `moved-pin` review item, not an error, so an ignored review item
    never turns into a later exit 2. This holds where the base's own
    `klin.json` pins the path and no other path of the same `in` selects a
    file. A path the change itself wrote that selects nothing stays a
    `configuration` error, so a typo never passes as a moved pin. A path
    beside another that selects files was never an exit 2, and it says
    nothing.
  - **Known limit.** A move combined with a rewrite under 50% similarity, or
    past the rename limit of section 7.2, reads as a deletion, so the gate
    measures nothing at the new path until a person updates the pin. The
    `moved-pin` review item names the path.
- **Moved out of scope.** A selected file that the change moved, by a detected
  rename, out of a scope that still selects other files keeps the scope
  membership of its base path for the capabilities that measured it in the
  base. A new or worsened finding at its new path is a FAIL like any other.
  The finding is an ordinary capability finding, with an added `values` entry
  `moved_out_of_scope: true` that is not ratcheted. This closes the route of hiding a
  finding by moving its file out of scope, without blocking a requested
  move. The kept membership holds in both trees and against an `except`
  path too, so a file renamed under an `except` path is still measured.
  **Known limit:** `conventions` reads the path each tree holds, so a file
  renamed out of a convention's `in` keeps no membership there, and the
  convention does not measure it at its new path.
- **Moved into a skipped directory.** A source file, of a language klin
  reads, that the change moved by a detected rename from a path under no
  directory of the default skip set to a path under one, such as `out/`,
  `build/` or `vendor/`, is in neither tree's file list, so no capability
  measures it at its new path. It is a note at the Stop and a review item of
  kind `moved-skipped` at `klin check`. The review item's `file` is the new
  path and its `check` is null. No block and no error. This holds with or
  without an `in` scope, and whichever capabilities the run selects. A
  pinned `in` path whose files moved under a skipped directory does not
  follow them there: its `moved-pin` review item counts them as moved under a
  directory every walk skips, and never says the run measures them. A file of
  no language klin reads, such as test data moved under `fixtures/`, says
  nothing. **Known limit:** a rename under a
  hidden directory, or under a directory that only a section's own
  `skip_dirs` names, is not reported. A section's `skip_dirs` is that
  section's policy.
- At the Stop, any other capability-scope error does not stop the other
  capabilities, and it never blocks. A FAIL beside it still spends its block.
  A run-scope configuration error writes `unjudged` (section 6.6). A klin
  failure at the Stop follows section 10.10.
- A code-quality defect is never an error. An external tool failure is a
  hole, `tool-error`, not an error (section 0.4).

### 7.4 Exit codes of `klin check`

| Exit | Meaning |
| ---: | --- |
| 0 | No error, no failing finding, measurement complete. Judgement `pass` or `review`. |
| 1 | No error, at least one failing finding. Measurement may be incomplete. |
| 2 | Execution `error`. |
| 3 | No error, no failing finding, measurement incomplete: a hole of section 7.2. |

Precedence:

```text
ERROR (2) > FAIL (1) > INCOMPLETE (3) > success (0)
```

The JSON holds every finding, review item, note, hole and error, whatever
the exit code. A review item never changes the exit code.

### 7.5 Aggregation

1. Execution is `error` when any run-scope or capability-scope error exists.
2. The judgement aggregates the capabilities that ran: `fail` over `review`
   over `pass`.
3. The measurement is `incomplete` when the run or any capability that ran
   has a hole.
4. Each capability's row carries its own judgement, measurement and
   execution. A capability with a hole can still carry findings, because a
   validated finding is a positive witness.

### 7.6 An unmatched accepted entry

An accepted entry that matches nothing is a review item at `klin check`
and a note at the Stop. It never fails a run and never asks the agent for a
change. The guard refuses the agent's edits to `klin.json`, so only a person
can act on it.

## 8. Evidence and Comparison

### 8.1 Measurement basis

Every measurement record names its basis. The basis holds, where relevant,
under the field named:

- `producer`: the capability name and its semantics version (section 8.2),
  or null for the run's own record;
- `klin`: the klin version that measured;
- the effective policy that the capability used, by provenance: the derived
  values under `policy`, each with its rule, what a person pinned under
  `pinned`, and built-in for every key neither names, with the derivation
  commit under `derivation`;
- `integration`, for an integration: the configured entry name and its `run`
  command or `report` path. A tool identity the report states is not read
  yet;
- `scope`: whether the run judged the changed files only (`changed`), and
  the observed `coverage` counts;
- `window`: kind, `before` and `after`;
- `holes`: the reason of each hole.

The basis MUST NOT hold a machine path, a timestamp, a process id or a
duration. Those are run metadata, recorded and never compared.

At the Stop, the basis and the coverage counts cover the changed scope only.
Building them adds no git process, no file read and no walk beyond the
measurement itself.

### 8.2 Semantics versions

Each capability declares an integer semantics version in its catalogue row.
The version rises when the same inputs can produce a different measurement
or judgement. A grammar or parser change that can move a measurement raises
the version of every capability that reads it.

### 8.3 Comparability

Within one run, ADR 0001 holds: one binary measures both trees under one
basis. They are comparable by construction.

Two measurements from different runs are **comparable** when they name the
same capability and the same semantics version. They are **not comparable**
when either differs. A policy change, such as a derived ceiling that moved,
is reported, and it does not decide comparability.

Comparison across runs happens in three places:

1. **Persisted derived evidence**: the survey cache, the structural cache
   and any future base evidence of a detector. Each keys on the inputs that
   define it, which include the klin version or the semantics versions it
   depends on. Missing, corrupt or incompatible evidence is rebuilt. When
   klin cannot rebuild it, such as when it cannot read the base, that is a
   capability-scope `git` error (section 7.3). It never becomes a fabricated
   pass.
2. **The journal and `klin report`**: a regression counts as fixed only when
   a later measurement of the same capability, comparable with the one that
   flagged it, no longer holds it. Not comparable is reported as
   "measurement changed; not compared". It is never read as a fix or a new
   regression. The `measurement-lost` row has no producer of its own, so its
   findings compare across any two runs.
3. **The accepted list**: an accepted entry keeps its person-authored
   meaning. An entry that stops matching after a semantics change is a review
   item (section 7.6), never a failure. The release notes of the version
   change name the capability.

This amends B.19.6: an upgrade can change a verdict where a semantics
version changed, and the release notes say so.

### 8.4 Finding identity

The #425 decision is the extension rule. A family without identity keeps the
0.x matcher of 4.4 and 16.5.

**Envelope.** `{version, state, key | reason, persists}`, with `state`
`identified` or `ambiguous`. A key never holds a measured value, a line
number or a body hash. `persists` says whether the other tree's file holds
the key. The capability computes it, because only the capability measured
both trees.

**Ambiguity.** A site is ambiguous when it is `anonymous`, `computed`, under
an anonymous or computed ancestor (`ancestry`), or one of two or more sites
of the family in its file on one side with the same key (`duplicate`). The
duplicate count covers every site of the family in the file, not only the
findings.

**Pairing.** For a finding and a `before` entry in one file:

1. Both identified under one version, equal keys: eligible, whatever the
   text.
2. Both identified under one version, different keys: eligible only when the
   text is equal and neither key persists.
3. Otherwise: eligible when the text is equal, as in 0.x.

Among eligible pairs, the rank and the one-to-one greedy pass of B.16.5
are unchanged. Values still decide `held` or `worsened`.

**Move targets.** An identified entry is lost when its key does not persist.
An ambiguous entry keeps the 0.x count by text.

**Versions.** Keys are compared only under one version that is on the
binary's own list of known versions. A pair under two versions, a key of an
unknown version, an ambiguous site and a site with no identity keep the text
and body-hash rules exactly.

**Other rules.**

- Duplicates are a multiset and pair one to one. Ambiguity never guesses a
  pairing that could hide a new regression.
- The finding `id` stays the hash of gate, file and declaration text.
- Accepted entries stay keyed by text. A person never writes an opaque key.
- The comparability check of section 8.3 comes before identity matching.
- A cache that holds identities keys on the identity version.

### 8.5 Ratchet principles

- Two trees, one binary, one basis.
- Only `new` and `worsened` fail. A ceiling change never fails a site the base
  holds (B.7.3).
- A site under a path the derivation commit did not hold matches nothing in
  `before` (B.7.1).
- Forced paydown is not a mechanism.
- A derived ceiling is the day-one default and can move. A pinned number or a
  dated schedule is the only ceiling that cannot loosen.

### 8.6 Claim-local evidence

Each capability's evidence supports its own claim only. Candidate-authored
tests, coverage, mutation results, external analyzer output and
counterfactual runs earn no generic credit. No result carries a global score
or an evidence-strength tier. External identifiers stay in a klin-assigned
external namespace. A rule id named like a native check never impersonates
one.

### 8.7 Determinism

section B.12 carries forward. In addition:

- A measurement record and its basis are a pure function of the trees, the
  policy and the binary, apart from the time-dependent cases B.12 names
  (dated ceilings, report age, command limits).
- An integration's result is deterministic only when its tool is. klin
  records the command it ran beside the results.

## 9. Capabilities

### 9.1 The native catalogue

The shipped checks stay, with the dispositions of #475 section 11. Their
measurement rules are section B.8, amended below and by section 7.

| Capability | Kind | Activation | Placement | vNext change |
| --- | --- | --- | --- | --- |
| `doc-size` | check | Automatic | `{stop, check}` | None. Per instruction file, not a total budget (#435). |
| `doc-citations` | check | Automatic | `{stop, check}` | None. |
| `lockfile` | check | Automatic | `{stop, check}` | None. |
| `escapes` | check | Automatic | `{stop, check}` | None. Known rewordings stay documented limitations. |
| `stubs` | check | Automatic | `{stop, check}` | None. File-and-kind marker identity (ADR 0064). |
| `inventory` | check | Automatic | `{stop, check}` | A deleted test is a review item (section 9.2). |
| `complexity` | check | Automatic | `{stop, check}` | None. Derived `cc` floor 10, explicit `test_lines` (ADR 0059, ADR 0063). |
| `dead-symbols` | check | Automatic | `{stop, check}` | None. |
| `reachability` | check | Automatic | `{stop, check}` | None. Named re-export is a reference (ADR 0061). |
| `layering` | check | Automatic | `{stop, check}` | None. |
| `public-api` | check | Automatic | `{stop, check}` | None. |
| `conventions` | check | Policy | `{stop, check}` | None. In-process ast-grep rules. |
| `sarif` | integration | Integration | `{check}` | Never runs at Stop (section 9.4). |

The catalogue order stays cheapest first (ADR 0036). Ambiguous graph or
resolution evidence falls under the classes of section 7.2, and is never
guessed.

### 9.2 `inventory`: a deleted test

The B.8.2 rules for what counts as a deleted test carry forward, including
the case where the subject left too, which stays a note.

At the Stop:

1. When a gate block remains under the prompt, the first Stop that finds a
   deleted test klin has not asked about MUST ask the agent why, on a gate
   block. A Stop that also holds a FAIL carries the FAIL and the question on
   one gate block. The text asks for a reason. It never asks for a repair
   and never tells the agent to restore the test.
2. klin records the question in the stamp's `asked` record, so it asks once
   per deleted test under the stamp (ADR 0031).
3. When no gate block remains, the deletion is reported, the Stop records no
   `asked` entry, and the stamp stays red (section 6.6). The question comes
   under a later prompt.
4. An asked deletion is a review item, which the Stop shows as a note. It is
   no reason for a gate block. Any Stop after a gate block, whether it holds a
   question, a FAIL or both, spends gate block 2 only over a changed tree.
5. The review item stays in the Stop's result until the stamp moves past the
   deletion.

At `klin check`, the deletion is a review item. It never fails the run and
never changes the exit code. The repository's code review process owns the
decision. klin adds no acceptance workflow for it.

This is the one review item that may spend a gate block. #475 section 11
admitted it with the #361 and #484 evidence: 3 of 3 #361 runs repaired
correctly, and no run in #484 case 8 changed the tree after the question.
That evidence is small. Any other review item that wants to ask needs its
own admission (section 18.2).

### 9.3 Named klin-owned recipes

No recipe ships. The architecture allows one only after a targeted product
admission. For a recipe, klin owns the supported tool and version policy, the
fixed invocation and environment, the adapter, the rule basis, the
completeness contract and the bounded execution. The tool owns its detector
semantics. A recipe MUST run offline at measurement time: no network
resolution and no install.

### 9.4 User-owned SARIF integration

The project owns the command, the tool version and the tool configuration.
klin owns:

- the bounded execution when it launches the command (B.9.3 limits);
- report freshness, by deletion before `run` or by file age (B.8.3);
- the delta judgement on changed lines (B.8.3);
- the representation of holes;
- the rendering.

vNext rules:

- An integration never runs at Stop.
- A fresh, well-formed report makes the measurement complete for its claim:
  "the judged results of this report on the changed lines". An empty report
  is complete for that claim too. The integration never claims that a file
  it did not report on is clean. Its coverage claim is `unverified`, which
  the row shows as a limitation, not a hole, unless the report states a
  producer coverage guarantee.
- A missing report after `run`, a stale report, a report that is not SARIF,
  a command stopped at its limit, and a command not found are holes,
  `tool-error`, exit 3. They are never findings.
- A user-owned command does not inherit the determinism claim of a recipe.
  When it uses the network or ambient state, that is the project's trust
  choice, and `klin policy` lists it as a limitation.
- `klin setup` and `klin policy` say that a `sarif` entry runs at
  `klin check` only.

### 9.5 Rejected and deferred capabilities

Nothing in this document makes these a product commitment (#475 section 16):

- broad test-integrity expansion;
- generic unfinished or error-masking detection;
- design and reuse conformance;
- counterfactual simplification;
- a Ruff injection or swallowed-error recipe;
- new-dependency or undeclared-import review;
- `uv.lock` and Cargo lock-entry shape checks;
- registry existence or age checks;
- changed coverage (#53, #54 closed);
- a public postflight phase (#70 closed).

Code duplication is a required product direction that #478 researches in
parallel. Section 18.4 defines the contract it must obey.

## 10. Host Protocol

### 10.1 One hidden ingress

```text
klin __agent event [--host NAME]
```

- Installed host integrations invoke it. A person never needs it.
- It is absent from `klin --help` and from the documentation of the public
  CLI.
- It reads one host event on stdin and normalizes it to one of four kinds:
  `session`, `prompt`, `pre_tool`, `stop`. The adapter places the kind from
  the event's own fields (B.9.1, B.9.7). `--host NAME` overrides host
  detection.
- The hook-line spelling above is a stable surface, because generated and
  committed files hold it. The ingress ignores an argument it does not know
  and still answers the event.

### 10.2 Dispatch invariant

`klin __agent event` MUST determine the event kind and the host before it
loads configuration, builds facts, resolves a base or walks the tree.

- A `pre_tool` event does the opt-in walk of section 5.1 and the guard. It
  does not read or parse `klin.json`. The whole event fits in 50 ms (0.x
  9.4).
- A `session` or `prompt` event adds no work beyond B.6.2 and B.6.5, apart
  from the merge-base that section 6.6 records with a new stamp.
- A `stop` event loads configuration only after the opt-in walk.
- The ingress adds no git process, no configuration load and no tree walk to
  any kind, compared with the 0.x command that served the same event, except
  the history check of section 6.6. At `session`, `prompt` and `stop` it reads
  HEAD, HEAD's symbolic ref, the default-branch ref and HEAD's reflog as files
  (with git itself in a reftable repository). It starts
  `git merge-base` only when the cached pair changed, and `--is-ancestor`
  only to test rules 2 and 3 after a branch change or a moved merge-base.
  `pre_tool` does none of this.

### 10.3 Events

| Kind | Does | Blocks | Writes |
| --- | --- | --- | --- |
| `session` | moves the stamp per B.6.2, raises the prompt counter, moves the prompt mark | never | stamp, mark, counter |
| `prompt` | the same as `session`, consumes a matching handoff record, and tells the radius note when the window passed its value (B.6.2.1, ADR 0024) | never | stamp, mark, counter, journal `prompt` line |
| `pre_tool` | the guard (section 10.8) | `deny` or `ask` | journal line for `ask` and `deny` |
| `stop` | build feedback, then the engine at placement `stop`, then the Stop policy | per section 10.4 | verdict, build stamp, handoff record, journal `stop` line |

The adapters, the event identity, the claims that let one copy act per event,
and the handoff records carry forward from B.9.1 and B.9.8.

### 10.4 Stop block policy

The block budget of B.9.3 and ADR 0052 carries forward. In each prompt:

- **Code FAIL**: the first failing Stop spends gate block 1 of 2. A failing
  Stop over the same tree spends no block: it reports and passes through. A
  failing Stop over a changed tree spends gate block 2 of 2. After gate block
  2, nothing that the gates find blocks under that prompt.
- **`measurement-lost`**: a code FAIL like any other.
- **Deleted test**: section 9.2. It shares the FAIL's block.
- **Review item**: spends no block, except the ask of section 9.2.
- **Opened gap, coverage note, hole, error**: spends **no** gate block, at
  block 1 or block 2, and does not keep the stamp red. A run-scope
  configuration error writes `unjudged` (section 6.6). The Stop tells it once
  per stamp and passes through.
- **Build failure**: the separate build budget of section 6.4.
- A Stop with a FAIL beside any of these spends a block for the FAIL. Its
  text names the others as limitations, with no repair instruction for them.

B.16.3 is amended to match: the hook blocks for a code FAIL and for the
ask of section 9.2, and for nothing else that the engine returns.

klin proves gate block 2 from its own record (B.9.3). When klin cannot read
or write that record, it blocks nothing. Lost or unsafe local state fails
open.

A host continuation, such as a Cursor follow-up or a Codex continuation,
never refreshes the prompt's budget. Only a genuine person prompt brings a
fresh budget of two gate blocks and eight build blocks.

### 10.5 Ask tools after a partial edit

`AskQuestion` (Cursor) and `AskUserQuestion` (Claude Code) may keep the turn
open. Then the Stop judges the tree after the person answers or skips. This
is accepted host behavior (#492). klin adds no PostTool hook and no ask-tool
interception. Same-tree pass-through and the two-block cap bound the flow.

### 10.6 What every renderer says

These rules bind the Stop text, the `klin check` text and the JSON `remedy`
fields alike, because agents read all three.

Per failing site, the text leads with:

1. the site;
2. the measured fact, with the base value or the accepted entry it matched;
3. why the fact matters;
4. a repair direction that fixes the underlying problem.

The repair direction never tells the agent how to make a metric disappear.

Per evidence kind:

- **FAIL**: names the violated policy and a behavior-preserving repair
  direction.
- **Review item**: states the observation. It never pressures the agent to
  clear it.
- **Hole and error**: states what klin could not measure or run, and the
  operational or configuration action that applies, if any. It never
  prescribes a source-code change. It never tells the agent to push, to
  install a tool, or to delete or rename a file. An action on `klin.json` or
  on host integration is addressed to a person, such as "ask a person to run
  `klin setup`".

The Stop text is a bounded repair queue. When it leaves out sites, it says
how many and that `klin report` lists them. Repeated evidence that a family
identifies safely is shown once.

The Stop keeps these 0.x shapes, with vNext command names:

- a block opens with one lead line that says how many gates failed, what to
  do, and which gate block of two the Stop spends;
- the Stop prints only the gates that did not pass, each with its own
  `derived:` and `pinned:` lines, and a gate that passed with a note it
  tells;
- a Stop that spends no gate block says why klin does not block again, and
  that the window stays open until the finding is fixed or a person accepts
  it;
- the turn-end line, the weekly line, and the `public-api` wording of ADR
  0054 name `klin report`;
- the `no-prompt-event` note says that klin grants no fresh gate blocks until
  the host's prompt hook runs `klin __agent event`.

Provenance beyond these lines stays in the JSON and the journal.

### 10.7 Non-blocking person notices

A Stop that does not block may carry a notice for the person: a note, a red
pass-through ("N regressions still need your attention. `klin report` shows
them."), a configuration error, the turn-end line or the weekly line.

- On Claude Code and Codex, the notice uses the host's person-visible,
  non-blocking channel (`systemMessage` under exit 0).
- On Cursor, a non-blocking notice MUST NOT use `followup_message`, because
  Cursor submits it as the next agent prompt. The journal records it, and
  `klin status` and `klin report` show it while its window is open. A notice
  expires when the window it belongs to closes, so notices never pile up: a
  reader holds a notice open while the stamp commit its Stop left, which the
  notice records as `stamp`, is the current one.
  The stamp records the notes and errors of the notice as told (section 6.6),
  so a later Stop under the same stamp leaves no second notice for them.
- On the harness protocol, the notice is a `tell` decision.
- A blocking Stop on Cursor still uses `followup_message`.

### 10.8 Guard

The guard of B.9.4 carries forward: `deny`, `ask` and `allow`, the proof
rule, path resolution, shell parsing and the 50 ms budget. The guarded set
stays this tree's `klin.json` and this tree's state directory. In a tree that
holds no `klin.json`, the guard answers nothing (section 5.1).

The `deny` list of commands is:

- `klin setup` in any form, because it writes `klin.json` and host files;
- `klin update`, because it replaces the binary that judges the agent;
- `klin __agent` in any form from an agent's shell, because a fabricated
  event could refresh a block budget. The host runs the hook lines itself, so
  the guard never sees them.

A deny reason says that a person changes the file in a reviewed commit, or
names the command a person runs instead. It never names a command that
accepts debt. The guard's journal line names the matched command.

### 10.9 Harness protocol

Protocol v1 (B.9.7, ADR 0047) carries forward unchanged in its event and
response schemas. A custom harness invokes `klin __agent event` instead of
the 0.x commands. The `event` field supplies the kind. The `tell` decision
carries non-blocking notices (section 10.7). The fail-closed rule for an
unknown protocol version stays (section 0.4).

### 10.10 Exit codes of the ingress

On Claude Code, Codex and the harness protocol, exit 2 blocks a Stop or
denies a tool call. So:

- `klin __agent event` exits 2 only to block or to deny.
- A usage error, an unknown argument, a host event klin cannot read, a
  run-scope internal failure and a panic exit 0 with no decision at
  `pre_tool`, `session` and `prompt`, and exit 1 at `stop`. Each writes a
  notice to stderr and, where the tree opted in and the state directory
  allows, a journal note (section 13.1). A capability-scope error is not one
  of these (section 7.3).
- A well-formed harness event with an unknown `klin_protocol` version is not
  "an event klin cannot read". It keeps the fail-closed rule of section 10.9.
- An event klin cannot read, or one whose host names no kind klin answers,
  has no kind, so it exits 0 whatever event the host meant.

## 11. Public CLI

### 11.1 The surface

```text
klin setup
klin check [CHECK...]
klin status
klin report
klin policy [SECTION]
klin update
```

| Command | Contract |
| --- | --- |
| `setup` | Set up or repair klin integration for this repository, or for one person's host files. |
| `check` | Measure the repository against klin's quality policy, optionally selecting named capabilities. |
| `status` | Read repository, setup, integration and local window state without running any check. |
| `report` | Show what klin caught, what was resolved, and what still needs attention. |
| `policy` | Explain the effective policy and where each value came from. |
| `update` | Update the installed klin CLI. |

No other public command exists. A flag narrows scope, changes the
representation, or selects one of the command's own outputs. A flag never
turns a command into a lifecycle program.

An invocation error of any public command prints its message to stderr and
exits 2, also under `--json`.

### 11.2 `klin setup`

- Idempotent and safe to run again after an update. A second run with the
  same inputs changes nothing.
- Writes `{}` when no `klin.json` exists, and never changes an existing one,
  except under `--pin`.
- Reconciles host integration files for the hosts the repository proves, or
  the hosts that `--host NAME` names, with the host selection of B.19.3
  (ADR 0056). It writes the `klin __agent event` hook lines, the skill files and
  the slash-command text, all with vNext command names.
- A klin-owned file that a person changed is a conflict. `setup` reports it
  and leaves it, as B.19.3 does for `install`.
- Flags: `--host NAME` (repeatable), `--user` for one person's host files on
  this machine, `--pin` (section 5.4), `--config PATH`. The hooks read only
  the worktree root's `klin.json` (section 5.1), so a `--config PATH` that
  names any other file is an invocation error, and `setup` writes nothing.
- Prints what it changed and what it left as it was.
- Exit 0 on success. Exit 2 on an invalid invocation or a write it could
  not make.

### 11.3 `klin check [CHECK...]`

- Runs every applicable capability of placement `check` over the `branch` or
  `push` window (section 6.5), in catalogue order.
- `CHECK...` names capabilities or `sarif` entries. Each name is a selector
  through the same engine.
- `--changed` narrows the scope to the window's changed files. It does not
  change the trust model.
- `--json` prints one JSON document (section 11.7) instead of text.
- `--config PATH` names the configuration.
- Never runs the build. Never writes local state other than caches.
- Exit codes: section 7.4.

A 0.x flag such as `--gate`, `--strict`, `--hook` or `--list` given to
`klin check` is an unknown argument, and exits 2.

The holes and entries that 0.x judged only under `--strict` are part of every
`klin check`:

| 0.x `--strict` addition | vNext |
| --- | --- |
| An accepted entry that matches nothing | Review item (section 7.6) |
| A same-tree comparison klin cannot explain | Section 6.5 |
| A file measured in `before` and not in `after` | A `measurement-lost` finding, a `left-scope` review item, or a `left-scope` coverage note (section 7.2) |
| No source root | Hole, `nothing-measured`, in a whole-tree run only, and only when a check the run selected reads code. `klin check lockfile` in a tree with no source root is not this hole. |

Text output:

- one `config:` line when no `klin.json` resolves, then one `window:` line;
- a `derived:` or `pinned:` line per value a gate used, above its row;
- one row per gate: `ok`, `FAIL`, `REVIEW`, `INCOMPLETE` or `ERR`, then the
  gate name. A gate with more than one state takes the first of `ERR`,
  `FAIL`, `INCOMPLETE`, `REVIEW`, `ok`;
- `FAIL:` with its remedy, `REVIEW:`, `HOLE:` with its reason, `NOTE:`, and
  `ERR:` lines under the row, as B.11.1 shapes `FAIL:` and `NOTE:`. A
  coverage note is a `NOTE:` line that names the file, its reason and the
  words `not measured`;
- the `OK:` line and its coverage counts of B.11.1;
- one summary line: `judgement: J, measurement: M, execution: E, exit N`,
  followed by `, N file(s) not measured` when coverage notes exist. After a
  run-scope error, J and M print as `none`, and an `ERR:` line names the
  error above the summary.

### 11.4 `klin status`

- Read-only. It runs no check, takes no lock that a Stop waits on, and writes
  nothing.
- Reports:
  - whether a `klin.json` resolves and whether it is valid;
  - each host integration, with host, scope, route and state;
  - the klin version, the state directory and the cache path;
  - the local window: the stamp's verdict, its age, why it is red or
    unjudged or aborted (the open findings, any unasked deleted test, or the
    error),
    the default-branch ref that section 6.6 uses, the last advisory Stop and its
    reason, and the non-blocking notices of the open window that only the
    journal holds;
  - the last Stop, labeled as historical.
- It never claims that the current working tree passes.
- Integration states:

| State | Test on the host files |
| --- | --- |
| `current` | The host files hold the hook lines and owned files that this klin's `setup` writes. |
| `missing` | The repository proves the host, and no copy of klin's hooks is installed for it. |
| `conflict` | A klin-owned file was changed, deleted or cannot be read, or two copies disagree in a way B.9.8 cannot settle. |

- `--json` prints the document of section 11.7.
- Exit 0 when it could read what it reports, whatever it found. Exit 2 on an
  invalid invocation, or outside a git repository.

### 11.5 `klin report`

Section 13.2.

### 11.6 `klin policy [SECTION]`

- Read-only. It derives values as a run would, and it runs no check. Each
  check has one derivation step that reads the configuration, the survey and
  the derivation commit. `policy` calls only that step, and the check's own
  run calls the same step before it measures, so the two cannot disagree.
  `policy` lays out no base, measures no file of either tree, and writes
  nothing to the state directory. It reads a cache that is already there.
- Prints the effective policy of every capability, or of the one `SECTION`
  names: activation (`automatic`, `policy`, `integration`), placement, state
  (`active`, `excluded`, `needs-policy`, `not-applicable`), and each value
  with its provenance (`derived` with its rule and derivation commit,
  `pinned`, dated with the step in force, or `built-in`). An Automatic
  capability whose facts the tree does not hold is `not-applicable`. A Policy
  or Integration capability with no section is `needs-policy`. A value is
  `built-in` when its key has a default and neither a person nor a
  derivation gave it. Each `derived:` and `pinned:` line prints as a
  `klin check SECTION` run prints it. In the JSON, a pinned dated schedule
  has the step in force as its `value`, with that step's date as `step` and
  the whole schedule as `schedule`.
- A tree where no capability runs still has a policy: `klin policy` lists
  each capability with the state that keeps it from running, and exits 0.
- A whole `policy` also prints the build policy, pinned or derived from the
  manifests, and the accepted list, one line per entry with every value the
  entry allows. Each integration
  lists the limitations of section 9.4: it runs at `klin check` only, its
  coverage is unverified, and a `run` command is the project's own trust
  choice.
- A capability whose derived policy is more than a value per key explains it
  in place of those lines. `klin policy public-api` lists each derived
  surface with its items, measured or opaque, and the packages with no
  supported surface (ADR 0044). Those surfaces come from parsing the working
  tree, so only the named form lists them. A whole `policy` prints one line
  that points to `klin policy public-api`, and parses no source. `conventions` explains each convention, or the one that
  `klin policy conventions NAME` names: what it forbids and where, what its
  code pattern reads as and how its language was settled, any `in` or
  `except` path that matches nothing, and its remedy (ADR 0037). It counts no
  match, because a count is a measurement. An unknown `NAME` exits 2.
- `--reference` prints the configuration reference of B.5.8 as Markdown,
  and `--schema` prints the JSON schema of `klin.json`. Both read no
  configuration and no tree, and exit 0 anywhere, inside or outside a
  repository. `docs/REFERENCE.md` is the output of `--reference`. The flags
  are stable. The content is documentation and follows the catalogue.
- `--json` prints the document of section 11.7.
- Exit 0 on success. Exit 2 when the configuration is invalid, with the error
  named, or on an invalid invocation.

### 11.7 Machine-readable output

`check`, `status`, `report` and `policy` each print one schema-versioned
JSON document under `--json`.

Rules:

- Every document has `schema_version` (integer, 1 at first) and `command`.
- Within one schema version, new fields and new enum values MAY appear. A
  consumer MUST ignore a field it does not know and MUST tolerate an enum
  value it does not know.
- Removing a documented field or changing its meaning needs the
  compatibility policy of #344 (section 17.4).
- Fields under `diagnostics` are outside the stable schema. The 0.x per-gate
  fields `ms`, `facts`, `names`, `work`, `graph`, `surface` and `footprint`
  move there, and the performance rows read them there.

#### The `check` document

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | integer | 1 |
| `command` | string | `"check"` |
| `klin` | object | `{version}` |
| `config` | object | `{path, present}` |
| `window` | object or null | `{kind, before, after, how}`. Null when a run-scope error came before a base resolved. |
| `tree` | object or null | `{head, dirty}`. `dirty` says whether the working tree differed from HEAD. No tree hash. |
| `judgement` | string or null | `pass`, `review`, `fail`. Null after a run-scope error. |
| `measurement` | string or null | `complete`, `incomplete`. Null after a run-scope error. |
| `execution` | string | `ok`, `error` |
| `exit` | integer | The exit code of section 7.4. |
| `capabilities` | list | One row per selected gate. Empty after a run-scope error. |
| `findings` | list | Failing and held findings. |
| `reviews` | list | Review items. |
| `notes` | list | Notes. |
| `measurements` | list | Measurement records, including run-level holes. |
| `not_measured` | integer | Distinct files that at least one selected capability did not measure because of an opened gap, a coverage note or a lost measurement. |
| `errors` | list | Errors. |
| `diagnostics` | object | Not stable. |

A capability row: `name`, `kind` (`check`, `integration`, or `built-in` for
the `measurement-lost` row), `placement` (list),
`state` (`active`, `not-applicable`), `judgement`, `measurement`,
`execution`, `coverage` (`{found, measured, not_read, excluded, gaps,
limits}` or null, where `not_read` counts the files of the run's scope in a
language the survey knows and the capability does not read, `gaps` files with an opened gap, and `limits` files with a
coverage note), `coverage_claim` (`verified`, `unverified`), `held`,
`accepted`. Rows list the selected gates, and a whole run also lists the
capabilities that do not apply, with state `not-applicable`.

A finding: `id`, `check` (null for `measurement-lost`), `kind` (`metric` or
`measurement-lost`), `outcome`
(`new`, `worsened`, `held`), `file`,
`line` (null for `measurement-lost`), `text`, `values` (object), `ceiling`
(object of the values judged),
`matched` (`{file, line, text, accepted, values}` or null), `condition`
(string, what the finding breaks), `remedy` (string), and `identity` (section 8.4) when the family has one.

A review item: `check`, `kind` (`deleted-test`, `unmatched-accepted`,
`unmeasured`, `moved-pin`, `moved-skipped`), `file`, `line`, `text`, `reason`
(the agent's reply for a deleted test, the gap reason for `unmeasured`, the
old and new path for `moved-pin` and `moved-skipped`).

A note: `check` (or null), `kind`, `coverage` (boolean, true for a coverage
note), optional `file`, `line` and `text`, and `message`. A coverage note's
`kind` is its reason of section 7.2. Other kinds include the
0.x note outcomes that stay notes: `unmatched` at the Stop, `derivation`,
`config`, and `window` for what choosing the base found (section 6.5).

A measurement record: `check` (null for the run), `basis` (section 8.1),
`state` (`complete`, `incomplete`), `holes` (list of `{reason, detail, text}`,
where `text` says what is missing and how to repair it;
every hole's site is the run or the gate).

An error: `kind` (section 7.3), `check` (null for the run), `message`. A
file or form klin could not measure is never an error: section 7.2 sorts it
into a `measurement-lost` finding, a review item or a coverage note.

#### The `status` document

`schema_version`, `command`, `klin {version}`, `config {path, present,
valid, error}`, `integrations [{host, scope, route, state, detail}]`,
`state_dir`, `cache_dir`, `window {verdict, age_seconds, open [finding
ids], unasked [deleted-test sites], error, aborted_since,
default_branch, last_advisory {time, reason} or null, notices [{time,
message}]}` or null,
and `last_stop {time, verdict, historical: true}` or null.

The window `verdict` is one of `pending`, `aborted`, `red`, `unjudged` and
`green` (section 6.6). `open` and `unasked` are empty unless the verdict is
`red`. Each `unasked` entry is the site as `file:line  text`. `error` is null
unless the verdict is `unjudged`, and `aborted_since` is null unless it is
`aborted`. The text prints an `unjudged` window as "nothing judged" with its
error.

#### The `report` document

Section 13.3.

#### The `policy` document

`schema_version`, `command`, `config {path, present}`, `derivation
{commit}`, `capabilities [{name, section, kind, activation, placement,
state, values [{key, entry, value, provenance, rule, description}],
limitations}]`, `build`, `accepted`, `state_dir`.

A value's `value` is typed as a pinned one would be. A `built-in` value
carries its words for a person in `description`, and its `value` is null
where only those words state it. `entry` names the entry a value belongs to,
such as one convention or one `public-api` surface. `klin policy NAME ENTRY`
carries only that entry. `klin policy --json public-api` carries each surface
as a `surface` value with its items and holes, and each package with no
supported surface as an `unsupported` value.

### 11.8 `klin update`

Unchanged from B.19.6. It uses the network to fetch the release. When
repository integration needs reconciliation after an update, it says to run
`klin setup`.

## 12. CI and Enforcement

### 12.1 The CI command

CI runs plain `klin check`. It MAY add `--changed`, `--json` or selectors. CI
MUST fetch enough history to resolve the base. `fetch-depth: 0` is the
RECOMMENDED setting, and a missing base is exit 2 (section 6.5).

A CI job MUST treat every non-zero exit as a failed check. Exit 3 means klin
could not complete a required measurement and found no proven failure. Exit
1 means a proven failure, even when a measurement was also incomplete.

### 12.2 Build ownership

The project's own CI owns build, test, type check and dependency install.
The klin Action installs klin and runs `klin check`. It never implies that
the project's commands ran. No message, document or hook text may claim that
`klin check` builds the project.

### 12.3 The GitHub Action

- The Action runs `klin check --json $ARGS` with `fetch-depth` deep enough
  for the base, and ends the job with the exit code of `klin check`, so every
  non-zero exit fails the job. When the run prints no JSON document, the
  Action prints what the run printed. The Action adds `--json` itself, so
  `args` holds selectors and flags other than `--json`.
- When the Action cannot report, because `jq` is missing or broken, the run printed no
  JSON document, or writing the annotations or the summary failed, it writes
  one `error` annotation that says so and fails the job: with the exit of
  `klin check` when that is non-zero, else with 2. A report the person cannot
  see never leaves a green job.
- It writes review items, holes, errors, failing findings, the count of
  files not measured, and the count of what it did not annotate to the job
  summary. The summary gives every count first, then lists errors, holes,
  failing findings and review items, in that order, up to
  a budget under GitHub's step summary limit of 1 MiB, and says how many
  lines it left out past it. A hole's line gives its reason and its text. It
  writes failing findings and review
  items, including `unmeasured` gaps, as file-and-line annotations, so a
  green job still shows them in the pull request.
- The Action writes annotations as workflow commands: `error` for a failing
  finding, `warning` for a review item. GitHub shows at most 10 of each
  level per step and 50 in total per job, shared with the job's other steps.
  The Action writes at most the host's limit of each level, and the job
  summary lists the rest. Separate levels mean that review items never push
  a failing finding out within the step. The documentation recommends a job
  of its own for `klin check`, so other steps cannot use up the job total. A
  `measurement-lost` finding annotates the first error node's line, or the
  file when it has none. A held finding is not annotated.

## 13. Reporting

### 13.1 The journal

The 0.x journal stays: one JSON line per Stop, prompt and
guard `ask` or `deny`, best-effort, never pruned. A Stop that ends on a
configuration error writes a line too, so `klin status` and `klin report`
can show the error.

vNext writes journal `schema` 2:

- Every line carries `schema`, `version`, `time`, `kind` and `session`, as
  B.11.4 defines them.
- A `stop` line holds the `check` document of section 11.7 under `result`,
  built by the Stop's engine run. In it, `command` is `"stop"`, `window.kind`
  is `turn`, and `exit` is null, because the host's exit code is not a
  verdict. Beside `result`, the line keeps the B.11.4 fields `host`,
  `prompt`, `verdict` with its `why`, `asked`, `hook`, `told`, `flags`,
  `config_hash` and `timing`, and adds `notice`: the non-blocking person
  notice, and whether a host channel delivered it or only the journal holds
  it.
- An advisory Stop's line carries `verdict: advisory` and its reason under
  `advisory`: `incoming-commits`, `branch-changed`, `history-lost` or
  `stamp-missing`.
  A repository with no remote keeps the 0.x `branch-fallback` outcome for its
  fallback Stop.
- `prompt` and `guard` lines keep their 0.x fields.
- A `note` line records an ingress that failed without a decision (section
  10.10): `event`, the kind klin placed or null, and `message`.

In the `result` of a `stop` line, `tree` is null, because the Stop starts no
git process to describe the working tree, and `window.kind` names the window
the Stop judged: `turn`, or `branch` for the 0.x branch fallback. `notice` is
null or `{message, delivered, stamp}`. `delivered` is false on Cursor, where
only the journal holds the notice, and `stamp` is the stamp commit the Stop
left, which names the window the notice belongs to (section 10.7).

klin is not released, so a reader reads schema 2 lines only. A line of
another schema counts in `skipped_lines`.

### 13.2 `klin report`

- Read-only. It reads the journal, re-runs no check and reads no working
  tree.
- Default scope: the newest session the journal holds. When the journal holds
  no session, it says so and suggests `--since 7d`. It never widens the scope
  silently.
- Flags: `--since Nd`, `--details`, `--json`.
- It shows: regressions caught, regressions fixed after klin flagged them,
  regressions still open, review items, holes, files not measured,
  regressions not compared, advisory Stops, set-aside regressions, and the
  non-blocking notices of open windows that only the journal holds.
- A regression still open at an advisory Stop, or when the next prompt moves
  an `unjudged` stamp, is `set-aside`, because the fresh stamp no longer
  judges it. It is never counted as fixed: an advisory Stop measures a window
  that other people's commits entered, so a site gone at that Stop proves no
  fix.
- The counted unit is the Regression of B.11.5, keyed by finding `id`. A
  fix counts only when comparable (section 8.3).
- Review items are keyed by `check`, `kind`, `file` and `text`.
- It never says who authored a fix.
- Exit 0 whatever it finds. Exit 2 on an invalid invocation.

### 13.3 The `report` document

| Field | Meaning |
| --- | --- |
| `schema_version`, `command` | 1, `"report"` |
| `scope` | `{kind: session \| since, value, known}` |
| `counts` | `{caught, fixed, open, not_compared, set_aside, reviews, holes, not_measured, notices, advisory}` |
| `regressions` | `[{id, check, file, text, state: open \| fixed \| not-compared \| set-aside, first_seen, last_seen}]` |
| `reviews` | `[{check, kind, file, text, reason, last_seen}]` |
| `holes` | `[{check, reason, last_seen}]` |
| `not_measured` | `[{check, reason, file, last_seen}]` |
| `notices` | `[{time, message, delivered}]` |
| `advisory` | `[{time, reason}]` |
| `skipped_lines` | Lines of a schema this reader does not know. |

A regression's `state` is `fixed` also when the configuration changed before
the site went, and `config_changed` says so (section 8.3). The document also
carries `episodes`, `audit` and `activity`, the per-regression trail the
benchmark reads. They are outside the stable schema.

## 14. Performance

### 14.1 Budgets that carry forward

section B.13 carries forward, with these amendments:

- The source-line resource ceiling is not exit 2. A file the base measured
  becomes `measurement-lost`, a new file an opened gap, and an inherited one a
  coverage note (section 7.2).
- The "whole-tree `--strict` run" rows are whole-tree `klin check` rows with
  the same limits.
- The 2,000-file budgets, the large-repository budgets of ADR 0042 and the
  release rule for the controlled rows stay. These limits are release limits.

### 14.2 The vNext Stop admission envelope

On the controlled `structural_1m` workload with 20 changed files and a warm
cache, excluding the project's build:

- 1,500 ms is the admission envelope. A change MUST NOT be admitted when it
  moves this median above 1,500 ms. The 5-second release limit of ADR 0042
  still applies to every row. The two limits are read together: the release
  limit catches a regression of the shipped product, and the envelope decides
  admission of new work.
- Below 500 ms is the strategic target.
- The difference between the current median (1,218 ms at `748d01fc`) and
  1,500 ms is not a feature budget.
- The 100-changed-file row stays a scaling check. A change that raises its
  median by more than one third against the preceding controlled row MUST be
  explained before it is admitted.

The owner takes the 1M rows on the controlled machine. A contributor reports
the relative change of the same rows at 300k on their own machine, which is
enough to propose a change.

### 14.3 Stop rules

- No external process on the Stop quality path.
- No whole-tree source walk or second parse for a capability that can reuse
  the delta facts.
- `{check}`-placed work adds zero work to the Stop (section 6.2).
- Basis records and coverage counts at the Stop cover the changed scope only
  (section 8.1).
- Incremental cost bands for a new Stop capability (#358):

| Incremental median at 1M/20 | Reading |
| --- | --- |
| below 10 ms | trivial |
| 10 to 25 ms | normal |
| 25 to 75 ms | needs clear product value |
| 75 to 150 ms | exceptional |
| above 150 ms | `{check}` unless evidence strongly says otherwise |
| above 500 ms | not a Stop capability |

### 14.4 Host-event rules

- `pre_tool` within 50 ms, including the opt-in walk.
- `session`, `prompt` and `stop` do the history check of section 6.6: file
  reads of HEAD, its symbolic ref, its reflog and the default-branch ref on
  each event, and `git merge-base` on the first event after each commit.
  `pre_tool` does none of it.
- Each Stop that holds the state lock writes the `aborted` verdict when it
  starts, which is one more atomic state write.
- An advisory Stop takes a full stamp capture (B.6.5) on the Stop path.
  This is rare, and it falls under the release limit of ADR 0042, not under
  the admission envelope of section 14.2.
- No other new work. The cost of these on the 1M row is a requirement to
  verify before the implementation ships.
- No PostTool hook.

## 15. Failure and Recovery

Behavior is keyed by the path, Stop or `klin check`. klin never changes a
verdict because a run is in CI. It reads the GitHub variables only to choose
the base (section 6.5).

| Class | Stop | `klin check` |
| --- | --- | --- |
| Invalid configuration | No block. Verdict `unjudged`. A notice goes to the person (section 10.7). | ERROR, exit 2, naming the file and key. |
| A policy path whose files the change renamed or deleted | The policy follows a rename. A deletion is a note. | `moved-pin` review item. Exit unaffected. |
| A file moved out of a scope that still selects others | Measured under its base scope. A new finding is a FAIL. | The same. |
| A file moved under a directory every walk skips | A note. | `moved-skipped` review item. Exit unaffected. |
| Other capability-scope configuration or git error | No block. A notice. The other capabilities report, and a FAIL beside it spends its block. | That row `execution: error`. Exit 2. The other capabilities report. |
| No `klin.json` | No answer, no state (section 5.1). | Runs under `{}` and says so. |
| A present base candidate does not resolve, shallow history, or a missing `GITHUB_BASE_REF` | Not applicable. | ERROR, exit 2, naming `fetch-depth`. |
| A push `before` rewritten away in a full checkout | Not applicable. | Falls back to the merge-base with a note. |
| A base equal to HEAD hides unpushed commits | Not applicable. | Hole, `comparison-unproven`, exit 3. |
| Incoming commits from the default branch, a branch change, or lost history (section 6.6) | Advisory Stop, which takes a fresh stamp itself. Without `refs/remotes/*`, the 0.x branch fallback for a branch change. | Not applicable. |
| No base resolves | Not applicable. | ERROR, exit 2, naming what was tried. |
| Stamp missing, ref present | Restored from the ref, and a note says so (B.6.2). The Stop writes the verdict its gates gave. | Not applicable. |
| Stamp and ref missing | Advisory Stop, which takes a fresh stamp itself. Without `refs/remotes/*`, the 0.x branch fallback (section 6.6). | Not applicable. |
| Stamped commit outside HEAD history on the same branch | The turn window stays, judged against the stamp tree (section 6.6). | Not applicable. |
| A file the base measured and the change made unmeasurable | `measurement-lost` FAIL. Blocks like any FAIL. | FAIL, exit 1. |
| klin fails during a Stop | Verdict `aborted` stays (section 6.6). Section 10.10 sets the exit. | ERROR, exit 2. |
| Another gap the change opened | A note, told once per stamp. No block. | Review item with a pull-request annotation. Exit unaffected. |
| A limit the change did not open | Coverage note. | Coverage note, counted in `not_measured`. |
| A capability stopped at its work bound | A note. No block. | Hole, `work-limit`, exit 3, unless a FAIL gives exit 1. |
| Integration tool error | Not applicable: integrations never run at Stop. | Hole, `tool-error`, exit 3, unless a FAIL gives exit 1. |
| Deleted test | Section 9.2. | Review item. Exit unaffected. |
| Unmatched accepted entry | A note. | Review item. Exit unaffected. |
| Build fails | Build block per section 6.4. | Not applicable: no build. |
| Build tool absent (127) | A note. The gates still run. | Not applicable. |
| Host event unreadable, usage error, panic | Section 10.10. | Not applicable. |
| Unknown harness protocol version | Fails closed: refuses tool calls, blocks Stops, and names both versions (B.9.7). | Not applicable. |
| State directory unwritable | Reports, writes no verdict and no count, blocks nothing. On Cursor it tells nothing (B.9.1). | Not affected: `klin check` writes no state. |
| State lock not taken in time | Measures and reports, spends no block, writes no verdict. | Not affected. |
| Cache unreadable or incompatible | Rebuild. When the rebuild cannot read what it needs, a capability-scope `git` error. | The same. |
| klin internal failure | Section 10.10. | ERROR, exit 2. |

Recovery is automatic. No person command moves local state (section 6.6).

## 16. Trust and Security

### 16.1 Feedback

Local hooks and a local `klin check`. klin puts each failure in front of the
agent, blocks at most twice per prompt for gate failures, keeps the window
open until the failure is fixed or accepted, and refuses the agent's edits to
the guarded set. Holes, coverage notes and configuration errors are told and
recorded locally, and CI judges them where CI runs `klin check`. An agent
that merges incoming commits, switches branches or moves a remote-tracking
ref by hand makes one Stop advisory (section 6.6). Where HEAD has no reflog,
the agent's own push of the default branch does the same. The debt it left before
that Stop is then judged only where CI runs `klin check`, and goes unjudged
where no CI runs it. In a repository with no remote, the branch fallback
judges from the B.6.3 base, which on the default branch can be HEAD, so a
squash past the stamp's parent there leaves committed work unjudged when the
stamp's parent is no longer readable. Nothing stops a
PATH shim, a `chmod -x` or a tool call the host does not show. This level
makes no stronger claim.

### 16.2 Enforced

Feedback plus a CI run of `klin check` with all of:

- a checkout the agent never touched, against a protected branch;
- the klin job is a required status check;
- the job treats every non-zero exit as failure (section 12.1);
- history deep enough to resolve the base;
- a pinned klin version and Action version;
- `klin.json`, the workflow, the hook settings and CODEOWNERS under
  CODEOWNERS.

At this level a gate holds against an agent, and loosening it takes a
reviewed commit by a person.

Enforced means klin's quality policy. It does not mean the project's build,
tests or type checks. The project's CI owns those (section 12.2).

A deleted test and an unmatched accepted entry are review items at this
level. The Action shows them in the job summary and as annotations (section
12.3). They do not fail CI. A person's code review decides them.

The README MUST name both levels and say which one a setup reaches.

### 16.3 Secrets and safety

klin reads no secrets. No native check or recipe reads the network, and no
measurement sends anything anywhere. `klin update` uses the network to fetch
a release, and the guard denies it to an agent. A user-owned integration
command may use the network as the project's own choice (section 9.4). The
`build` and `run` commands execute a project's own shell lines, which the
configuration owner wrote, under bounded execution. A derived build command
comes from a fixed table and is printed before it runs. klin never invokes
`npx`, `npm exec` or a command that could fetch a tool.

`docs/THREAT_MODEL.md` carries the reader-facing form of this section.

## 17. Compatibility, Stability and Migration

### 17.1 Public command migration

klin is private before 1.0, so it keeps no 0.x spelling alive. A removed
public command is an unknown command and a removed flag is an unknown
argument. Both exit 2, with no alias and no migration error. The per-check
options with no vNext place are gone: `dead-symbols --report`,
`doc-size --file` and `--ceiling`, `doc-citations --file` and `--roots`,
`--list-languages`, `--only` and `--quiet`. `klin policy public-api` and
`klin policy conventions [NAME]` take over the two reports (section 11.6).
The 0.x hook spellings `klin radius`, `klin guard` and `klin gate --hook`
are unknown commands too. `klin setup` and the plugin write only
`klin __agent event` (section 10.1), at every scope.

### 17.2 Behavior migration

| 0.x behavior | vNext |
| --- | --- |
| Exit 2 for a file the change made unmeasurable | A `measurement-lost` FAIL, exit 1 |
| Exit 2 for a new file no grammar reads, a new unresolved form or a new resource ceiling | A review item with an annotation, exit unaffected |
| An unparseable file in the hook is a note | A `measurement-lost` FAIL when the base measured the file, otherwise a note |
| A tool error at Stop spends a gate block, and its text asks for a repair | No block, no repair text |
| A deleted test is a note in CI | A review item in CI, exit unaffected |
| An unmatched accepted entry fails under `--strict` | A review item, exit unaffected |
| `--strict` exits 2 for a lost file | A `measurement-lost` FAIL, or a `left-scope` review item or coverage note |
| `--strict` exits 2 for no source root | Exit 3 in a whole-tree run (`nothing-measured`) |
| A base equal to HEAD with hidden local commits is exit 2 | Exit 3 |
| A shallow CI clone could fall through to a vacuous pass | Exit 2 naming `fetch-depth` |
| A force-push `before` falls through to the merge-base | The same, with a note |
| A FAIL plus a hole outside the hook exits 2 | Exit 1 |
| One gate's configuration error is ERR beside the other gates | The same, as a capability-scope error, exit 2 |
| `sarif` runs at Stop | `sarif` runs at `klin check` only |
| `klin gate` without `klin.json` is exit 2 | `klin check` runs under `{}` |
| A `klin.json` below the repository root is read | Only the worktree root's file is read. The hooks tell a notice to move a nested file. |
| Cursor tells a red pass-through by `followup_message` | Journal, `klin status` and `klin report` |
| Stop text names `klin stats --turn` | Stop text names `klin report` |
| Action runs `klin gate --strict` | Action runs `klin check` |
| `turn reset` moves the stamp to the current tree | No command |
| A merge of the default branch makes its code new at the Stop | That Stop is advisory and takes a fresh stamp itself |
| The branch fallback after a history move judges the whole branch, red | With a remote: one advisory Stop that takes a fresh stamp. Without a remote: the branch fallback from the B.6.3 base, after a branch change or a missing stamp only |

### 17.3 Configuration migration

vNext retires no `klin.json` key. A configuration that 0.x accepted is valid
in vNext, provided it sits at the worktree root (section 5.1).

### 17.4 Inputs to the stability contract (#344)

#344 rewrites the stability contract against this document. The surfaces
are:

- the six public commands, their flags and their exit codes;
- the four JSON documents of section 11.7, their schema versions and their
  enums: hole reasons, error kinds, review kinds, integration states, report
  states;
- `klin.json` keys and their meaning;
- the hook-line spelling `klin __agent event [--host NAME]` and the exit
  rules of section 10.10;
- the harness protocol v1 schemas (section 10.9);
- the host decisions klin writes: block, allow, deny, ask, tell;
- the journal schema, if #344 covers readers outside klin.

Not covered: prose, `diagnostics`, state and cache files, the content of
`policy --reference` and `--schema`, and measurements under a semantics
version change (section 8.2).

#344 decides: whether a semantics version change is a minor or a major
release; the deprecation window after 1.0; how a klin reads a `klin.json` key
that a newer klin added; and how harness protocol v1 may change.

## 18. Admission and Extension

### 18.1 The admission loop

A capability earns a place by the whole loop, not by detector accuracy alone:

- detection: deterministic where it blocks, high precision, explicit
  before, after and change semantics;
- repair: actionable, behavior-preserving, resistant to cheap appeasement;
- AX: the agent understands the finding and repairs the underlying problem at
  low turn cost;
- DX: reproducible, explainable, low configuration burden, portable across
  first-class hosts and CI;
- UX: a person is interrupted only when their judgement is needed;
- evidence integrity: one coherent tree and basis per measurement, intended
  against observed coverage, no fabricated clean evidence;
- operations: bounded runtime and explicit holes.

### 18.2 Placement admission

- A blocking capability needs deterministic evidence at blocking strength,
  strong product value, a low rate of harmful intervention, a
  behavior-preserving repair path and resistance to cheap appeasement.
- A contextual capability is a review item or stays out.
- `{stop, check}` additionally needs the Stop rules of section 14.3 and a
  measured incremental cost at 1M/20. The proposal states why the capability
  belongs at Stop and not only at `klin check`.
- A review item that asks the agent at Stop needs agent-outcome evidence that
  the ask does not create repair pressure.
- A capability whose value does not justify its implementation,
  configuration and maintenance cost stays research evidence.
- A future admission uses a small targeted study, triggered by concrete
  product or user demand. The 29-phenotype study of #357 is not a standing
  prerequisite.

### 18.3 Extension rules for a native check

A new native check:

1. adds one catalogue row with its placement, activation, needs, semantics
   version and labels, and no other public surface;
2. adds no public command, no lifecycle flag and no host event;
3. reads facts through the shared engine and reuses the run's file list and
   parses where it can;
4. returns findings, review items, notes, holes, gaps and errors in the
   result model of section 7, with a measurement basis;
5. MAY declare a family identity under section 8.4;
6. MAY persist versioned derived base evidence under section 8.3, keyed by its
   inputs, and MUST treat missing, corrupt or incompatible evidence as a
   rebuild or a capability-scope error, never as a pass;
7. reports exhaustion of its own work bound as a hole with reason
   `work-limit`, and a file over a byte ceiling under section 7.2;
8. states its identity rule, ratcheted values, derivation rule, required
   measurements and remedy as B.8.6 and section 7.2 require;
9. pins its behavior through CLI tests (section 19).

### 18.4 The duplication boundary

#478 owns duplication's detector semantics. It MUST obey sections 6.2,
7, 8, 14.3 and 18.3. In particular:

- its identity is family-specific and versioned;
- its base evidence, such as a base index, is versioned derived evidence, and
  missing, corrupt or incompatible evidence is rebuilt or is an error, and
  never becomes a pass;
- its placement is `{stop, check}` or `{check}`;
- a Stop placement must meet the Stop admission and performance rules;
- contextual, check-only evidence may be a review item;
- resource exhaustion of its work is a hole, `work-limit` (INCOMPLETE);
- it adds no public lifecycle command.

This document does not freeze: token normalization, thresholds, region-family
identity, positional-hash or index layout, duplication cache payloads,
BLOCK against REVIEW for duplication, or exact-region against near-miss
algorithms.

#480 rejected the current exact-region Stop design and advanced a TypeScript
variant for a future non-blocking `klin check` review. That is #478's input,
not part of this contract.

### 18.5 The parser and cache seam

This document freezes only semantic requirements for caches and derived
evidence: the keys and comparability of section 8.3, rebuild-or-error, no
fabricated pass, and zero Stop work for `{check}` capabilities (section 6.2).

It does not freeze `ParsedFile` ownership, shared AST extraction, the
structural-cache format, base-index storage, or cache invalidation tied to
detector payloads. An implementation ticket that materially redesigns any of
these MUST synchronize with #480 step 1 and #489 before it freezes that seam.

CLI, result-model, lifecycle-routing, reporting and migration work do not
wait for that synchronization.

### 18.6 The bar to admit a pre-handoff phase

A readiness or deep-local phase returns only with all of (#484 section 12):

1. a named, admitted capability whose cost or nature keeps it off every
   Stop;
2. an agent-outcome comparison on at least two first-class hosts that shows
   more correct repairs or fewer harmful hand-backs when the capability runs
   before hand-back than when it runs at `klin check` in CI;
3. a cost account of the record, identity, sandbox and delivery machinery the
   phase needs, against that benefit.

Speculative infrastructure does not meet the bar.

### 18.7 Known limits of the local window

A public command that moves the stamp returns only with evidence from real
use that a red window, which the agent cannot close, costs people more than
the risk of silencing local feedback that CI would still fail. Under section
2.3 only agent-fixable FAILs keep a window red, so such a window SHOULD be
rare. These known cases are accepted with this document, and each roadmap
ticket that implements the area MUST measure them:

1. **Grammar lag.** A `measurement-lost` finding from a construct klin's
   grammar does not read yet blocks until a person holds the file (section
   7.2).
2. **Teammates' commits on the same branch.** A pull of a teammate's commits
   on the agent's own feature branch does not move the default-branch
   merge-base, so it is not advisory, and those commits read as new at the
   Stop.
3. **Stacked branches.** A merge of a branch other than the default branch
   does not move the default-branch merge-base either.
4. **Cross-file findings.** A finding in a file the agent changed that an
   earlier commit of the branch caused, without incoming commits.
5. **Reflog dependence.** Rule 1 of section 6.6 reads HEAD's reflog. A
   repository with reflogs turned off falls back to "a moved merge-base is
   advisory", which also fires on the agent's own push to the default branch.
6. **Feedback-level escapes.** An agent can make one Stop advisory by
   merging the default branch or switching branches. CI judges the branch
   where it runs `klin check`.
7. **Moves with rewrites.** A pinned path moved together with a heavy rewrite
   reads as a deletion (section 7.3).
8. **Rename scoring.** git's inexact rename scoring can differ between git
   versions (section 7.2).

Repeated real-use reports of any case are the trigger to revisit this
section.

## 19. Validation

### 19.1 The test seam

The test seam stays the binary's command line (AGENTS.md). Each vNext
contract below needs a CLI test that builds a throwaway tree, writes
`klin.json`, commits a base, runs the binary, and asserts the exit code and
the printed text or JSON.

### 19.2 Contracts to pin

Result model and exit codes:

- a clean run exits 0 with `pass` and `complete`;
- a review-only run exits 0 with `review`;
- a failing run exits 1;
- a run with a hole exits 3, and its JSON lists the hole;
- a failing run with a hole exits 1 and lists both;
- a file that the base parsed and the change made unparseable is a
  `measurement-lost` FAIL, exit 1, with the same `id` whatever gates a run
  selects;
- a line over the source-line ceiling added to a file the base measured is a
  `measurement-lost` FAIL;
- a manifest the change made invalid is a `measurement-lost` FAIL;
- an accepted entry of gate `measurement-lost` holds the file for every
  capability;
- a new file in a language klin reads, whose parse has an error node, is one
  `unmeasured` review item, exit 0, and `not_measured` counts it;
- a new file in a language klin does not read is counted in `not_read`, not
  as a review item;
- a file the base measured that the change re-encodes, makes binary, or
  replaces with a symbolic link is a `measurement-lost` FAIL;
- a `measurement-lost` accepted entry stays matched while the file still
  fails to parse;
- a file that left a scope narrowed in `klin.json` is a coverage note, and
  one that left a scope narrowed by a manifest is an `unmeasured` review
  item when it is clean and a FAIL when the base's scope finds a new
  finding in it;
- `* -text` in `.gitattributes` makes no file `measurement-lost`;
- a path with an inherited `filter` attribute is a coverage note, and a
  `filter` attribute that the change added to a base-measured path is a
  `measurement-lost` FAIL;
- a CRLF working tree does not trip the line ceiling;
- a case-only rename on a case-insensitive file system is not a lost file;
- rename detection ignores the person's `diff.renameLimit`;
- `measurement-lost` is a reserved name, and a Stop that does not block
  tells the person, never the agent, how to hold a lost file;
- a capability that stops at its work bound is a `work-limit` hole, exit 3.
  No shipped capability has a work bound yet, so this is a `#[cfg(test)]`
  pin under the AGENTS.md exception until one ships. The pins are
  `a_work_limit_hole_makes_the_gate_incomplete_and_the_run_exit_3` and
  `a_failing_gate_beside_a_work_limit_hole_exits_1_and_an_error_exits_2` in
  `src/gate.rs`;
- `klin check --changed` over a change with no measurable file exits 0;
- an invalid configuration exits 2 with `execution: error` and null axes;
- a capability-scope configuration error exits 2, and the other capabilities
  still report;
- an error and a hole together exit 2;
- an integration with a missing report exits 3 with a `tool-error` hole;
- an integration whose command is not found exits 3 with detail
  `command-not-found`;
- an integration with an empty, fresh report is complete;
- a selector that names an unknown or excluded gate exits 2;
- a `klin check` with a 0.x flag is an unknown argument and exits 2.

Windows:

- a present `GITHUB_BASE_REF` that does not resolve exits 2 and names
  `fetch-depth`;
- a push `before` that a force-push rewrote away falls back to the merge-base
  with a note in a full checkout, and exits 2 in a shallow one;
- no base exits 2;
- a local source with unpushed commits and a base equal to HEAD exits 3 with
  `comparison-unproven`;
- a repository with no remote and a base equal to HEAD passes with a note;
- `klin check` without `klin.json` runs under `{}` and prints the `config:`
  line.

Stop:

- a code FAIL spends gate block 1, a same-tree Stop passes through, and a
  changed-tree Stop spends gate block 2;
- an opened gap and a coverage note spend no gate block and leave the stamp
  green; a configuration error spends no gate block; none of their texts
  holds a source-repair instruction;
- a Stop that klin aborts leaves `aborted`, not an earlier `green`;
- a coverage note is told once per stamp, not at every Stop;
- a FAIL beside a capability-scope error still spends its block;
- a Stop with a FAIL and an unasked deleted test spends one block for both;
- a turn that changes no measurable file leaves the stamp green;
- after a merge, a fast-forward pull and a `pull --rebase` of the default
  branch, including after a fetch done before the stamp, the Stop is
  advisory: it blocks nothing for findings, tells them once, writes an
  `advisory` line, and takes a fresh stamp itself;
- a branch switch to a branch that does not hold the stamp's parent, and a
  missing stamp, are advisory; `git switch -c` at the same HEAD is not;
- a paused interactive rebase is not advisory;
- with HEAD's reflog turned off, a moved default-branch merge-base alone is
  advisory;
- a repository with no `refs/remotes/*` ref takes no advisory Stop: a local
  branch move changes nothing, and a branch switch takes the 0.x branch
  fallback from the stamp's parent;
- the agent's own `git push` and `git push -u` never make a Stop advisory;
- `git commit --amend`, `git reset --soft` and an interactive rebase of the
  turn's commits, pushed or not, keep the turn window, and the agent's red
  finding still blocks;
- a build failure at an advisory Stop still blocks under the build budget and
  takes no fresh stamp;
- an advisory Stop tells an unasked deleted test, and `klin check` shows it
  as a review item;
- an advisory Stop takes the fresh stamp itself, so a host continuation in
  the same prompt is an ordinary Stop;
- `* -text` in `.gitattributes` makes no file `measurement-lost`;
- a path with an inherited `filter` attribute is a coverage note, and a
  `filter` attribute the change added to a base-measured path is a
  `measurement-lost` FAIL;
- a CRLF working tree does not trip the line ceiling;
- a hook in a repository whose only `klin.json` is in a subdirectory tells
  one notice that names the file to move;
- an `unjudged` Stop after a red Stop keeps the stamp red;
- `klin check` run from a subdirectory with its own `klin.json` reads the
  worktree root's file and names the nested one as ignored;
- a Stop where neither HEAD nor the default branch moved starts no git
  process for the merge-base;
- a regression open at an advisory Stop is `set-aside` in `klin report`, not
  fixed;
- a nested `klin.json` below the worktree root is ignored by the hooks;
- a run-scope configuration error writes `unjudged`, which `klin status`
  shows as "nothing judged";
- renaming the files a pinned `in` names keeps the capability measuring them
  and adds a `moved-pin` review item; deleting them is a note and a review
  item; neither blocks;
- renaming a pinned `doc_size` document keeps it measured at its new path,
  against the base's copy at the old path, and adds a `moved-pin` review
  item; deleting it is a note and a review item, not an exit 2;
- renaming the files a convention's `in` names keeps the convention measuring
  them and adds a `moved-pin` review item; deleting them is a note and a
  review item, not an exit 2;
- a complex function in a file moved out of an `in` scope that still selects
  other files still fails;
- a file renamed under a directory every walk skips is a note at the Stop and
  a `moved-skipped` review item at `klin check`;
- a state directory deleted while the turn ref survives restores the stamp
  from the ref;
- a review item that was asked about leaves the stamp green;
- a deleted test with a block left asks once and is then a review item, at
  Stop and at `klin check`;
- a deleted test with no block left is asked about under a later prompt;
- an integration never runs at Stop;
- an unmatched accepted entry is a note at Stop and a review item at
  `klin check`;
- a Cursor non-blocking notice emits no `followup_message`, lands in the
  journal, `klin status` and `klin report`, and expires when its window
  closes;
- a Cursor block still uses `followup_message`, and a Cursor follow-up gains
  no fresh budget;
- a Codex continuation gains no fresh budget;
- an ask-tool flow keeps same-tree pass-through and the two-block cap;
- the harness protocol carries a non-blocking notice as `tell`.

Host protocol:

- with an invalid `klin.json`, `pre_tool`, `session` and `prompt` still answer
  correctly, which shows that they dispatch before configuration loads;
- without `klin.json`, every event exits 0 with no output and no state;
- a usage error, an unknown argument and an unreadable event under the
  ingress never exit 2;
- an unknown harness protocol version still fails closed;
- the opt-in walk stops at the first `.git` entry;
- the guard denies `klin setup`, `klin update` and `klin __agent` from an
  agent.

CLI:

- each removed command is an unknown command and exits 2;
- `status`, `report` and `policy` write nothing to the state directory;
- `status` reports each integration's state and the local window verdict;
- `report` with no session says so and suggests `--since 7d`;
- `setup` run twice changes nothing the second time;
- `check` writes no stamp, no build stamp and no journal line, runs no build,
  and leaves no base worktree, also beside a concurrent `check`;
- the Action annotates failing findings before review items and counts what
  it did not annotate;
- `policy --reference` exits 0 outside a repository and matches
  `docs/REFERENCE.md` byte for byte;
- each JSON document carries `schema_version` and `command`.

Evidence:

- an incompatible cache is rebuilt;
- a regression flagged under one semantics version and absent under another
  is "not compared" in `klin report`;
- a journal line of another schema counts in `skipped_lines`.

### 19.3 Performance evidence

- A change to the Stop path reports the 1M/20 median against section 14.2,
  taken by the owner on the controlled machine, and the 300k relative change.
- A `{check}`-placed capability shows zero added Stop work on the 300k row.
- `pre_tool` stays within 50 ms on the performance fixture.

## Appendix A. Answers to the #358 questions

| # | Question | Section |
| --- | --- | --- |
| 1 | Durable thesis | Purpose, 1 |
| 2 | What runs on every Stop | 6.2, 9.1 |
| 3 | What runs only at `check` | 6.2, 9.4 |
| 4 | What is authoritative only in CI | 16.2 |
| 5 | Stop performance and admission | 14, 18.2 |
| 6 | Result states | 7 |
| 7 | Completeness, tool and code distinction | 7.2, 7.3 |
| 8 | Comparability | 8.1 to 8.3 |
| 9 | Finding identity | 8.4 |
| 10 | Which families may block | 9.1, 18.2 |
| 11 | REVIEW, NOTE, UNKNOWN | 4.4, 7.2 (advisory unknown is a note, an opened gap is a review item, klin's inherited limits are coverage notes, broken configured evidence is a hole) |
| 12 | Required evidence | 7.2 |
| 13 | Native, recipe, SARIF boundary | 9.3, 9.4 |
| 14 | Rejected capabilities | 9.5 |
| 15 | Configuration philosophy | 5.2 |
| 16 | Trust boundary | 16 |
| 17 | UX, DX, AX | 2.1, 10.6, 11 |
| 18 | Surfaces kept and removed | 11, 17.1 |
| 19, 20 | Surviving and new tickets | derived after acceptance |
| 21 | Six commands and hidden ingress | 10.1, 11.1 |
| 22 | Build ownership | 6.4, 12.2 |
| 23 | JSON and exit codes | 7.4, 11.7 |
| 24 | Stop and `check` difference | 6.1, 6.3 |
| 25 | Tree and basis identity | 8.1, 11.7 |
| 26 | INCOMPLETE at Stop and `check` | 2.3, 7.2, 10.4, 15 |
| 27 | Event routing | 10.1 to 10.3 |
| 28 | `init --pin` | 5.4 |
| 29 | `turn reset` | 2.3, 6.6, 18.7 |
| 30 | Placement vocabulary | 4.3, 5.2, 6.2 |
| 31 | Dispatch with no extra work | 10.2 |

## Appendix B. The carried-forward 0.x rules

This appendix holds each section of the 0.x specification that section 0.3
carries forward, under its 0.x number with the prefix B: the 0.x section 6.3
is section B.6.3. The sections stay normative for vNext. Where a section of
the main body amends a rule here, as the table of section 0.3 records, the
main body wins.

Inside this appendix, a citation by bare number, such as "6.3" or "section
6.3", names the section B.6.3 of this appendix. A citation of a 0.x section
that this appendix does not hold names the section of the main body that the
table of section 0.3 lists as its replacement.

The 0.x command names in this appendix map to vNext as section 0.5 records:

| 0.x | vNext |
| --- | --- |
| `klin gate`, `klin gate --strict` | `klin check` (section 11.3) |
| `klin gate --hook`, `klin radius`, `klin guard` | `klin __agent event` (section 10.1) |
| `klin install`, `klin init` | `klin setup` (section 11.2) |
| `klin stats` | `klin report` (section 13.2) |
| `klin turn reset` | None. A history move makes one Stop advisory (section 6.6). |

### B.4 Core Domain Model

#### B.4.1 Tree

A tree is a set of files at one point. A run holds exactly two: `before` and
`after`. `after` is the working tree unless a command names a commit. `before`
is what the Window chose. A tree MUST be readable without a network.

#### B.4.2 Window

A window names the pair of trees a run compares and where each came from.
Three kinds exist.

| Kind | `before` | `after` | Who uses it |
|---|---|---|---|
| `turn` | the tree as it stood when the current turn's window opened | working tree | the Stop hook, `klin radius` |
| `branch` | the merge-base with the default branch, or the pull request base | working tree, or HEAD in CI | `klin gate` by hand, CI on a pull request |
| `push` | the push event's previous commit | HEAD | CI on a push |

Every run MUST print the window it used, as one line, before any gate output.
Every JSON report MUST carry it.

#### B.4.3 Derived

A value klin computed from the tree because the config did not pin it. Every
derived value MUST be printed with the word `derived` and the rule that
produced it, on the run that uses it. Two kinds exist.

A derived number, such as a ceiling, a radius percentile or a build command,
MUST be a pure function of one commit, the derivation commit of 6.6, and the
binary version, so it is cached by commit id. It is computed over the paths
that commit's own survey holds, never over a path found only in `after`.
Where its sample needs policy, it reads the policy recorded by that commit,
never the working-tree copy.

A derived path set such as the documents of `doc_size` or the manifests of
`build` is the union of the derivation commit's survey and a discovery walk
over `after`. Source roots and language lists are facts, not configuration
values: each source check selects every supported file from the run's one
repository walk, then applies its compact `in` / `except` policy. A site
present only in `after` matches nothing in `before` and is `new` (7.1).

#### B.4.4 Site

The identity a Finding is keyed by. A site is a file plus the text of its
declaration line, with the line number as a tie-breaker only (ADR 0008). A
check MAY define a different identity when a declaration line does not exist,
and MUST document it. Moving code within a file MUST NOT create a new site.
Renaming a file MUST NOT create new sites (ADR 0009). A finding MAY name a
site that `after` no longer holds, when the check ratchets existence (8.2).
Its `file` and `text` are then the `before` site's.

When one site has more than one finding or more than one entry, the findings
and the entries at that site are paired one to one, greedily, in this rank
order (16.5):

1. No ratcheted value rose against the entry.
2. The most ratcheted values exactly equal.
3. The smallest line distance. The matcher does not read a line from an
   accepted entry, so its distance is 0.
4. The finding's line. Findings arrive sorted by file then line (12).
5. Entry order: the accepted list in config order, then the `before` sites
   by line.

A finding matches at most one entry, and an entry at most one finding. A
finding is `worsened` only when no untaken entry at its site holds it. So a
stale accepted entry never fails a site that `before` holds (7.3), and a
value a person raised in the accepted list holds the site. The order also
lets a twin that moved keep its entry over a nearer twin whose values
changed, so moving code does not read as new debt, and it makes a twin
inserted between two twins the new one.

A function moved between files with its body unchanged SHOULD match its old
site. The RECOMMENDED second pass matches unmatched findings to unmatched
entries by a whitespace-normalized body hash, across files. This is additive
to ADR 0008 and does not change the primary key, so a move within a file and
a rename of a file still match on the primary key alone.

The body is everything below the first line, with every run of whitespace
collapsed. A declaration that fits on one line has nothing below it, so the
whole line is its key. A declaration that wraps keeps its later lines,
because the primary key is the declaration line and not the whole signature
(ADR 0008). Dropping the first line drops the name, so a rename whose body
did not change matches too, which is what `inventory` reads to tell a
renamed test from a deleted one (8.2). An edit to the body changes the hash,
so a move that also edits is `new`. This is an equality match on an
unchanged body and not a similarity match.

Only an entry whose site the `after` tree no longer holds is a move target.
From one site the second pass may take as many `before` entries as that site
lost, which is the count of its `before` entries less the count of its
findings, lowest line first. An entry an accepted one outranked is not lost,
so a copy is not a move: the original takes the primary match, and the copy
is `new`. klin drops a body hash from an accepted entry, so the accepted
list keeps the keys 4.8 names and no accepted entry is matched this way.

Pairing is one to one and ranks by 16.5 without the line distance, which
means nothing across files. A failure whose entry sits in another file names
that file after the values it was, so a function that moved and grew does
not read as a regression where it now sits.

#### B.4.5 Finding

One site with the values a check measured there. Fields:

- `file` (string) repository-relative path, REQUIRED
- `line` (integer) 1-based, REQUIRED where a line exists
- `text` (string) the declaration line, REQUIRED, part of the identity
- `values` (object) metric name to number or string, REQUIRED
- `outcome` (`new` | `worsened` | `held`) set by the engine, never by a check.
  A note about a site, such as an unmatched accepted entry or an unparsed
  file, is not a finding. It is a separate record under `notes` in the JSON
  (11.2), and its `outcome` names the kind of note rather than a verdict.

A value the check ratchets on is one where higher is worse. A check MUST name
those values. A value that is not ratcheted is carried for the report only.

#### B.4.6 Check and Gate

A check is one measurement and the judgement behind it. A gate is one
configured or derived instance of a check. A check declares:

- `name`, the command name and the `--gate` name
- `section`, the config key it reads
- `needs`, what the check needs of the base: nothing, the commit the window
  names, or that commit laid out as a tree beside the working one. Every
  Automatic check MUST judge against the base, so that a tree with no config is
  green (5.1). A check that judges one tree against a number exists only for a
  number a person pinned. A check that needs the commit or the tree is the
  kind `--strict` reaches, because it has a comparison or an accepted list to
  judge. `sarif` needs the commit and not the tree: it reads which lines the
  window changed and runs the scanner once, over the working tree only (8.3).
- `takes_scope`, whether a changed-file list narrows it. False says only that
  it does not, and a check sets it false for its own reason: `doc-size` and
  `lockfile` judge a set small enough that narrowing it buys nothing,
  `layering` and `sarif` read the whole tree by construction (8.3). The
  physical changed-file set is the default judgement boundary, and one of
  those reasons is that a check owns a broader bounded judgement unit
  instead, because a change elsewhere in the repository deterministically
  changes the meaning of evidence in a file the window did not touch.
  `public-api` judges the whole consumer-facing surface, `reachability` judges
  every member of each family, and `doc-citations` judges the whole derived
  root-document set. A check that owns such a unit MUST name it in its own
  contract in 8.2.1, and the runner never infers one: there is no dependency
  graph and no propagation rule above the catalogue. The turn window of ADR
  0014 says which work belongs to the turn. It does not require every finding
  of that turn to sit on a line the turn edited, and the two-tree ratchet, not
  the changed-file list, is what keeps old debt quiet.
- `gate_per_entry`, whether the section is a list of entries a person writes,
  each its own gate under its own `name`, rather than one section the whole
  check runs under. Only `sarif` sets it (8.3).
- `available`, whether the tree holds what an Automatic check applies to: a
  source root, a document at the tree root, an instruction file of 5.4 for
  `doc_size`, a test, a manifest klin reads. It
  is answered from the facts of 4.3 alone and never from a derived number
- `activation`, what the section's absence means. An Automatic check runs
  over the tree's facts where it is available and derives its own policy. A
  Policy check, such as `conventions`, runs only once a person writes the
  policy. An Integration check, such as `sarif`, runs only once a person names
  the external tool (ADR 0038). No component derives a section for a check:
  each check reads the facts, resolves its own policy, and prints the
  provenance of each value it used (ADR 0040).

A check declares no cost of its own. The catalogue is one ordered table,
written cheapest first, and a run executes its gates in the order the
catalogue declares them (ADR 0036). A check that plans several gates, one per
entry a person wrote, keeps them together in that position, in the order the
section lists them. The order is a property of the table, so adding a check
in the right place is the whole of the decision.

A Policy or Integration check runs only when its section is present. An
Automatic check runs unless its section is `false`.

A run loads and validates the configuration once and reads every tree's file
list once, however many gates run and however many roots their sections
name. The configuration is the policy a person wrote; what a tree holds is a
fact the run reads for itself; the runner composes the two and a check
borrows what it needs (ADR 0038).

#### B.4.7 Ceiling

The value a measure may reach before its gate fails. A ceiling is derived, a
pinned number, or a pinned dated schedule. Section 7.3.

#### B.4.8 Accepted entry

Debt a person allows, keyed like a site, with every value the gate ratchets
and the amount of each they allow. Only a person writes one, in a reviewed
commit (ADR 0009). An entry that does not give a number for each of those
values is a config error, because a value it leaves out would grow unjudged
at that site. The one exception is a value a site of that gate may carry
or not: `lines` and `test_lines` of `complexity`, of which a function carries
at most one (8.2.1). An entry may leave either out. It then holds only a
finding that does not carry the value it left out, because a value a finding
carries and its entry does not is a rise (7.1). An entry that names `lines`
for a function in test code, with `test_lines` pinned, therefore matches
nothing. Pinned by
`an_accepted_entry_that_names_test_lines_holds_a_test_function`,
`an_accepted_entry_without_lines_holds_a_test_function_on_its_cc`,
`an_accepted_entry_that_leaves_out_lines_does_not_hold_a_production_function`
and `an_accepted_entry_that_names_some_of_the_values_is_a_tool_error` in
`tests/complexity.rs`. An entry that matches nothing is a NOTE, and a failure
under `--strict`.

An entry may carry a `reason`, a string in which the person says why they
accept the debt. It is not a value: klin never ratchets it, never compares
it, and leaves it out of the `values` of the entry a finding matched (11.2)
and of the values `klin policy` prints. `klin policy --json` keeps it.
An entry whose `reason` is not a string is a config error. Pinned by
`an_accepted_entry_with_a_reason_keeps_the_reason_out_of_its_values` and
`an_accepted_entry_whose_reason_is_not_a_string_is_a_config_error` in
`tests/complexity.rs`, and by
`the_readme_accepted_example_fits_the_schema_and_the_binary` in
`tests/schema.rs`.

The accepted entry takes the match when the finding holds against it, unless
a `before` entry the finding also holds against shares more values with it
(4.4). When the finding rose against the accepted entry but not against
`before`, or when `before` shares more values, the `before` entry takes the
match and the accepted entry matches nothing. That is how `--strict` tells a
person to delete the line once the code has moved off the accepted value.

### B.5 Configuration Contract

#### B.5.1 The file is optional

`klin.json` at the repository root, when it exists. Discovery walks up from
the working directory. `--config PATH` overrides. Paths resolve against the
file's own directory. The file is under the guard and SHOULD be under
CODEOWNERS.

With no file, every Automatic check for which the tree holds applicable facts
runs. When
the two trees are the same, this MUST produce a green run, because nothing in
a tree is worse than itself. That is the first stop of the hook, whose first
stamp is the working tree as it stands. When the trees differ, as under `klin
gate` by hand against a merge-base, the run fails on new or worsened findings
only. A stale citation, a missing lockfile entry or a long document that the
base already holds is `held`, not `new`. A file klin could not measure at
the base either is a NOTE (8.6), so it keeps this promise too. A build that
already fails is judged under the failure model of section 15 and is not part
of this promise.

Under `--hook` the file is the marker that the repository opted in. When no
`klin.json` resolves, `klin gate --hook` MUST exit 0, print nothing and write
no state, before it surveys the tree. A `--config PATH` that names no file
reads the same way. `klin gate` without `--hook` keeps the exit 2 that names
the file and the sections that would fill it. ADR 0028.

One configuration per repository, at the root. Source checks discover every
supported package in a monorepo from that one root. Discovery walks up only to
find the file from a subdirectory. A configuration per package is not
supported.

#### B.5.2 Top-level keys

- `build` (string, list or `false`) OPTIONAL, ADR 0012. A build a person
  chose over the derived one: a command, a list of `{run, root}` entries, or
  `false` to build nothing. Absent, the hook derives one command per standard
  manifest when it builds (5.4). A derived command is never written into the
  file.
- `accepted` (list) OPTIONAL, section 4.8
- `radius` (object) OPTIONAL, ADR 0014, with `lines` and `directories` as whole
  numbers. Derived from history when absent.
- `journal` (object) OPTIONAL, with `prompt` a boolean. `false` turns off the
  prompt excerpt of 11.4; the excerpt is recorded by default. A configuration
  klin cannot read carries no excerpt either: the one case where klin cannot
  see this setting is the case where it MUST NOT record the text.
- one key per gate, named for its section, or `false` to exclude the gate.
  A check that runs as several gates holds them in its own section, as the
  named entries of `sarif` (8.3) and the named conventions (8.4) do. There
  is no top-level list of extra gates (ADR 0038).

`klin.json` is a person's policy over facts klin derives, and `{}` is a
complete configuration. It never describes the repository: roots, languages,
documents, manifests, test roots, families and build commands are facts.

A key klin does not know MUST be an error naming the key. `project` and
`version` are no longer keys, and a configuration that names either MUST be an
error saying to delete it: the repository's identity is a fact, and a binary
version freezes no semantics. A configuration schema epoch, should klin need
one, is a decision of its own (ADR 0040). A section with a `baseline` key MUST
be an error saying the key is gone (ADR 0009). Every section, and every
`radius`, `journal` and `build` entry, MUST reject every field its reader does
not read, including retired topology fields, with an actionable migration
error, and SHOULD name the field a person most likely meant. A section MAY pin
some keys and leave others to derivation. A pinned key MUST print as `pinned`
beside the derived ones.

#### B.5.3 Compact policy

`complexity`, `escapes`, `stubs`, `dead_symbols` and `reachability` are
Automatic. An absent section means use discovered facts and built-in language
support; `false` disables the check; an object holds only a decision a person
made. Every one reads `in` and `except`: a repository-relative path or
non-empty list, selecting that path and everything below it. They are paths,
not globs. An explicit `in` with no applicable file is exit 2.

`complexity` additionally reads `cc` and `lines`; either is a whole number or
a dated ceiling schedule and either may be omitted for derivation. It also
reads `test_lines`, of the same shape, which only a person pins, and which
holds the length of test code as `lines` holds the length of production code
(5.4, 8.2.1, ADR 0063). `escapes`
additionally reads `skip_test_idioms`, default `true`, which leaves `unwrap` and
`expect` out of Rust test code and `@ts-expect-error` out of TypeScript and
JavaScript test files, and nothing else (8.2, ADR 0049, ADR 0060). The key was
`skip_rust_tests`, and a section that still names it MUST get the migration
error of 5.2 naming `skip_test_idioms`. Pinned by
`the_retired_skip_rust_tests_key_names_the_key_that_replaced_it` in
`tests/escapes.rs`. `dead_symbols`
additionally reads name globs in `ignore`. `stubs` and `reachability` read no
other policy.

The retired `roots`, `languages`, `patterns`, `skip_dirs`, `exclude`,
`exclude_except` and `ceilings` fields, and a person-authored reachability
family list, MUST be rejected rather than ignored. Source discovery, supported
languages, marker patterns and reachability topology are properties of the
binary and tree, not knobs in `klin.json`.

`doc_size`, `doc_citations`, `inventory` and `lockfile` are Automatic too.
`doc_size` is a map of document path, from the configuration's directory, to
a ceiling, a whole number or a dated schedule (5.5). A document the map names
is judged under that ceiling, and the instruction files of 5.4 keep their
automatic ceilings where the map does not name them, so a pin
never takes an instruction file out of scrutiny. An empty map is exit 2.
`doc_citations` reads no policy: its section is absent or `false`.
`inventory` and `lockfile` read only `in` and `except`. The retired `doc_size` entry list of `file` and `ceiling`,
the `file`, `roots` and `extensions` of `doc_citations`, the `name`, `path` and
`pattern` of `inventory`, and the `manifests` and `exclude` of `lockfile` MUST
be rejected with the replacement named (ADR 0040).

#### B.5.4 Derivation rules

Each check documents its rule. The rules for the shipped checks:

- Source files: every supported, non-ignored file in the repository's one
  tree listing, less the built-in skip set and the section's `in` / `except`
  scope. Each check owns its supported language table. The before tree uses
  the compact scope its own commit records, and today's scope when it records
  none, so narrowing scope cannot silently erase coverage. A derived sample
  instead uses the compact scope recorded by the derivation commit, so both
  the files and the policy that select them come from that one commit.
- Source roots and test roots: start at the directory of each source file
  and merge upward while the directory above holds nothing but source. A
  source root is a directory this finds that no other one holds. A source
  file that sits directly in a directory holding something else, such as
  `build.rs` beside a crate's `Cargo.toml`, starts at that directory, so the
  crate directory is a source root and holds the ones beneath it. A test root
  is a directory this finds that a test directory segment names or whose
  every source file carries a test affix, that no other test root holds, and
  that no other directory this finds holds unless that one is, or holds, the
  nearest directory above the test root that directly holds a `Cargo.toml`,
  `go.mod`, `package.json` or `tsconfig.json`. So a crate's `build.rs`, or a
  `jest.config.js` beside a package's `package.json`, keeps the package
  directory a source root and leaves its `tests/` a test root. Nor does a
  source file in a directory above the package remove that test root, such
  as a root `install.sh` above `rust/Cargo.toml` or a `crates/check.sh`
  above a workspace member, with or without a `build.rs` in the crate.
  `src/spec/` beside `src/schema.sql` and `src/lib.rs` is no test root,
  because `src`, which holds `lib.rs`, is a directory this finds between it
  and the crate directory, and it stays in its crate's source root. The
  survey knows no other manifest, so `Tests/` beside a Swift package's
  `Package.swift`, or `tests/` beside a Python project's `setup.py` and
  `pyproject.toml`, is no test root. Pinned by
  `a_build_script_at_a_crate_root_keeps_its_tests_as_a_test_root`,
  `a_workspace_member_with_its_own_build_script_keeps_its_tests_as_a_test_root`,
  `a_crate_below_a_directory_that_holds_a_script_keeps_its_tests_as_a_test_root`,
  `a_config_file_beside_a_packages_manifest_keeps_its_tests_as_a_test_root`
  and `a_directory_another_test_root_holds_is_no_test_root_of_its_own` in
  `tests/survey.rs`, and by
  `unwrap_and_expect_in_the_tests_of_a_crate_with_a_build_script_are_left_out_by_default`,
  `a_workspace_members_build_script_and_src_are_judged_beside_its_test_root`,
  `a_script_added_between_a_workspace_member_and_its_root_keeps_the_members_tests_left_out`,
  `a_directory_inside_src_is_no_test_root_beside_a_non_source_file` and
  `removing_a_non_source_file_from_src_keeps_a_held_site_under_it_held` in
  `tests/escapes.rs`.
- `doc_size`: the agent instruction files `AGENTS.md` and `CLAUDE.md` at the
  tree root and `AGENTS.md` in any directory the survey reads, in the
  derivation commit and in `after`. The survey reads no hidden directory and
  nothing in the default skip set, such as `node_modules`, `target`, `dist`
  or `fixtures`, so an `AGENTS.md` there is not judged. Each file is judged on its own, and no
  budget sums them. The ceiling of a file the derivation commit holds is its
  word count there, rounded up to the next 50 and never below 50, so an empty
  document gets 50 rather than a ceiling its first word breaks. A file the
  derivation commit lacks takes a ceiling of 50 words, and its `derived:` line
  names that new-file default. The 50 is a fixed product value and reads
  nothing from `after`, so 4.3 holds. `init --pin` writes no such default:
  it pins only the ceilings derived from files the derivation commit holds.
  Whether a file is new comes from the derivation commit's survey. A held
  file whose ceiling git cannot read is exit 2, never the new-file default,
  and a cached ceiling set that misses a held file is derived again. A nested `CLAUDE.md`, a `CLAUDE.md`
  import and a host's own rule directory are not instruction files. The
  survey finds nested files in the one tree listing, and the derivation
  commit's copies are read in one batch (#435). A changed run reads only the
  automatic instruction files the change set touched: an unchanged one has
  the base's word count, so it is under its ceiling or held at the base. A
  changed run still reads every pinned document, so a pin that names a
  missing file stays exit 2. A base read git cannot finish is exit 2, never
  new debt. The base
  copies of the documents over their ceilings are read in one batch too. Every other document, such as a
  README or a changelog, grows by design and is judged only when the section
  pins it, because the gate exists for the instruction file that grows every
  turn (8.2, #343). A document the section pins takes its pinned ceiling
  instead, and is judged wherever it sits. With no section, `doc_size` runs
  only when the tree holds an instruction file, and a tree that holds only
  other documents needs a section a person writes. `--file`
  on such a document with no pin is exit 2. A cache that holds a ceiling for
  any other document is no derivation of that commit, so klin derives again
  and `init --pin` never writes that ceiling into policy. Pinned by
  `a_readme_that_grows_past_its_base_word_count_passes_with_no_pin`,
  `an_instruction_file_that_grows_past_its_derived_ceiling_fails_with_no_pin`,
  `a_pinned_readme_is_judged_under_its_pin`,
  `a_readme_alone_under_an_empty_config_leaves_doc_size_needing_a_section`,
  `file_on_a_readme_under_an_empty_config_is_a_tool_error_naming_the_instruction_files`,
  `each_instruction_file_takes_its_derived_ceiling_or_the_new_file_default`,
  `a_pin_overrides_the_new_file_default_and_a_nested_claude_md_is_not_judged`,
  `a_hundred_nested_instruction_files_are_each_judged`,
  `a_cache_that_misses_a_held_instruction_file_is_derived_again`,
  `a_changed_run_judges_only_the_instruction_files_that_changed`,
  `a_changed_run_still_reports_a_pinned_document_that_was_renamed` and
  `a_changed_run_reports_a_new_pin_that_names_a_missing_file`
  in `tests/doc_size.rs`, by
  `a_document_the_derivation_commit_lacks_is_judged_under_the_new_file_default`
  in `tests/survey.rs`, and by
  `pin_writes_no_readme_ceiling_a_cache_of_this_version_still_holds` in
  `tests/init.rs`.
- `doc_citations`: every Markdown file at the tree root in the union of 4.3,
  each read against the whole tree with the built-in extension list of 8.2.1.
  This set is the check's judgement unit on a changed run too (8.2.1). It is
  wider than the documents `doc_size` derives a ceiling for, so a README is
  read for its citations with no pin. Pinned by
  `a_root_readme_doc_size_does_not_judge_is_still_read_for_citations` in
  `tests/doc_citations.rs`.
- `inventory`: every file under a test root the survey finds, and every
  source file a test directory segment or a test affix of 8.2 marks wherever
  it sits, less the default skip set and hidden directories.
- `lockfile`: every manifest the survey finds that klin has a reader for,
  `Cargo.toml`, `package.json` and `go.mod`.
- `complexity.cc` and `complexity.lines`: the 95th percentile of each measure
  over every supported function selected by the compact scope recorded at the
  derivation commit, with the TypeScript and JavaScript suite callbacks in test
  files omitted as 8.2.1 states,
  rounded up to the next whole number, with a floor of `cc 10` and `lines 25`
  so a small clean tree is not held to a ceiling of 1. Below 50 functions the
  floor is the ceiling. A pinned `cc` wins over the floor, whatever its value
  (ADR 0059). Pinned by `a_percentile_below_ten_derives_a_cc_ceiling_of_ten`
  and `a_pinned_cc_below_ten_still_judges_at_the_pinned_value` in
  `tests/complexity.rs`. Test code of 8.2.1 stays in the sample of both
  measures, so no derived ceiling moves when tests are judged apart. Pinned
  by `test_code_stays_in_the_derived_sample_for_both_ceilings`. `lines`
  judges only production code. `complexity.test_lines` is never derived: a
  missing `test_lines` means test code is not judged on length, and not that
  a ceiling is derived for it (ADR 0063). A function found only in `after`
  never enters the percentile. No recorded section, or a recorded object with neither `in` nor
  `except`, selects the whole repository. A recorded scope that selects no
  supported function derives the floors and names its zero-function sample.
  A recorded configuration or complexity section that exists but cannot be
  read as compact scope falls back to the whole repository and emits a NOTE.
  A pre-compact section containing only retired fields has no compact scope
  and therefore selects the whole repository. Today's compact scope selects
  what is judged but never enters this sample; when it differs from the
  recorded scope, a NOTE names both. Two scopes are compared after each is
  sorted and loses every path another path of its list holds and every
  `except` path outside every `in`, so one selection written two ways is one
  scope. The fallback and the difference are `derivation` notes, which a stop
  nothing blocks still tells the person.
- `dead_symbols`: every Rust and TypeScript/TSX file selected by the compact
  scope. Its optional `ignore` list is a set of name globs.
- `reachability`: one family per directory of the derivation commit whose
  files share a basename prefix or suffix at a token boundary and one
  concrete extension, such as `src/commands/*_command.rs`, named for its
  root and pattern. A family is derived only when its complete cohort under
  that root holds at least three files, every one structurally measured,
  every one with an eligible declaration, and every one proven reached: an
  eligible declaration whose name has exactly one declaration under the
  index and a reference from another file. A member reached only through a
  name several files declare is not proof. A TypeScript destructuring
  declaration that binds the name is no declaration of it here, and one in
  another file counts as a reference from that file, as a lazy
  `const { default: Profile } = await import(…)` does for the file it loads.
  A shorthand binding counts too, so `const { Login } = await import(…)` in
  another file proves the member that declares `Login`. A destructuring
  inside a function body is no declaration, so a name only it binds proves
  nothing (8.2.1). Pinned by
  `a_destructuring_that_binds_a_member_name_is_no_second_declaration_of_it`,
  `a_member_only_a_destructuring_in_another_file_binds_still_proves_its_family`
  and `a_shorthand_binding_in_another_file_reaches_and_proves_a_member` in
  `tests/reachability.rs`. `*.rs`, `*.ts` and every other
  bare extension are never a family. The derivation reads the structural
  files under the derivation commit's source roots, less those under a
  source root that is also a test root, and a file under a test directory
  seeds no family. A test root inside a source root, such as a crate's
  `tests/` beside its `build.rs`, stays among the files read, so a reference
  from it can prove a member, and a test directory under a family's root
  stays in the cohort the family must prove. Pinned by
  `a_crates_integration_tests_beside_its_build_script_still_prove_a_family`
  and
  `a_test_directory_under_a_family_root_stays_in_the_cohort_beside_a_non_source_file`
  in `tests/reachability.rs`. A file under a test directory, or one whose
  basename carries a test affix of 8.2, is never a member in a tree a run
  judges.
  Of two candidates the
  broader wins where its whole cohort is proven, and a narrower one survives
  a broader one that is not. The policy is read from the derivation commit
  alone, never from the union with
  `after`, so the tree being judged cannot widen or weaken it, and it is
  cached under that commit. When nothing is proven the gate has no families
  to judge. A person may narrow the derived families only with `in` and
  `except`, or disable the check with `false`; a family list is invalid.
- `radius`: the 90th percentile over the last 200 non-merge commits, per
  ADR 0014, or no section below 50 commits.
- `build`: one entry per manifest, per ADR 0012, derived only by a hook run
  that runs the build. The `derived:` line names each command and the
  manifest it came from, and the hook prints it whether the build passes or
  fails, so a derived command never reaches the agent with no origin (ADR
  0040, ADR 0048). Manifests are a path set. A manifest the derivation commit
  lacks gets its entry from the fixed table on the turn that adds it.
  The derived value is the command the table names, such as `tsc --noEmit`,
  and it stays a function of the derivation commit. Which tool a checkout
  runs that command with is resolved when the build runs, per 9.3, and
  reaches neither the derived value nor the order the entries run in. A run
  that resolved a command against the checkout prints one `resolved:` line
  per such entry in its text report, beside the `derived:` line and above the
  gates, so the report names what ran. A `resolved:` line is a fact of the
  checkout and no derivation, so neither the journal nor the `--json` object
  carries it. klin never invokes `npx`, `npm exec` or any other command that
  could fetch a tool. A `build` a person wrote runs exactly as written.

A derived ceiling is not monotone. A percentile falls when simple functions
arrive and rises when simple functions leave. A tree of 96 simple functions
and four complex held ones has a ceiling at the floor. Delete 46 of the
simple ones and the next derivation puts the ceiling at the complex four,
though no surviving function got worse. klin keeps no history that could
prevent this, and a run prints the ceiling it used, so a person sees the
number move. The derived ceiling is the day-one default and nothing more. A
person who wants a ceiling that cannot loosen pins one. Section 7.3 names
what tightens.

#### B.5.5 Pinned ceiling shape

A pinned ceiling is either a number or an object of dated steps:

```json
"complexity": {
  "cc": 12,
  "lines": { "2026-09-08": 90, "2027-01-01": 70, "2027-07-01": 60 }
}
```

The run uses the lowest step whose date is on or before today. A schedule
with no step yet due MUST be an error. Today is the system date in UTC, so
two machines on one day agree on the step. A run MAY take
`KLIN_TODAY=YYYY-MM-DD` for tests. A run that uses a schedule MUST print the
date it used on the `derived:` line for that ceiling. This is the shape issue
#87 proposes for one gate, applied to every ceiling.

#### B.5.6 Exclusion

A gate is excluded by setting its section to `false`. Naming an excluded gate
on the command line is exit 2.

#### B.5.7 What `init` does now

`init` writes `{}` when no configuration exists, which is the repository's
opt-in marker (5.1, ADR 0028), and says so. It reads no tree and derives
nothing, because a run derives what the file leaves out. It MUST NOT change an
existing configuration.

`init --pin` writes today's suggested guardrails as policy a person reviews:
the complexity `cc` and `lines` the derivation commit gives, and no
`test_lines`, which nothing derives (pinned by
`pin_writes_no_test_lines_because_klin_never_derives_it` in `tests/init.rs`), a `doc_size`
ceiling for each instruction file of 5.4 that commit holds and
for no other document, because it pins what a run derives, and the `radius`
values history gives. The `doc_size` rule is pinned by
`pin_writes_a_document_ceiling_only_for_the_instruction_files` in
`tests/init.rs`. It writes a value only where the configuration states
none, so a pinned number, a dated schedule, a section set to `false`, the
`accepted` list and the `journal` preference stay as a person wrote them. It
creates the file when there is none. It MUST NOT write repository topology: no
roots, languages, document entries, manifests, test roots, reachability
families, build commands or package structure (ADR 0040). The snapshot flags
`--add` and `--force` are gone.

`init` MUST write only the config. The guard denies `init` in every form from
an agent. `init` MUST NOT edit `.gitignore`, because klin writes nothing that
git could see.

`init` carries no host integration. `klin install` owns it, and the `--hooks`,
`--host` and `--global` flags of `init` are gone (19.3, ADR 0046). A command
line that names one is a usage error.

`init` is a convenience, not a step. A tree with no `klin.json` is fully
gated.

#### B.5.8 The configuration reference

`klin reference` prints the configuration reference as Markdown on stdout and
exits 0. It reads no configuration and no tree, so it runs anywhere.

Every check declares its keys beside the code that reads them, and the
reference prints one table per section. Each row states the key, what it
holds, whether klin derives it when absent or only a person pins it, the
derivation rule of 5.4 where there is one, and the default where there is
one. A section that reads no key says so. The reference states the top-level keys of 5.2 the same way, and
the dated ceiling shape of 5.5.

Every key the reference names is read through its declaration, so a key
renamed in the declaration is renamed where it is read. A key inside one of them, such as the `run` of a `build`
entry, is stated in what the key above it holds and is not a row of its own.
The sections the reference prints, and the built-in language coverage it
prints beside them, come off the same table of checks a run gates from, so a
check cannot be gated and left out of the reference.

`schemas/klin.json` is generated from those same declarations by `klin
reference --schema`. SchemaStore registers the exact filename `klin.json`, so
editors can discover completion, hover text and structural validation without
an inline `$schema`, editor setting or extension. The schema is editor
guidance only: `klin` remains the authority for semantic validation, and
normal commands never read or validate against the schema artifact.

The schema accepts a file exactly when klin's structural rules accept it,
except for the disagreements below and the `measurement-lost` entry that #500
owns. A structural rule is a rule that `klin.json` alone decides.
A `remedy` with no character other than whitespace is refused. A step date
takes a month from 01 to 12 and a day from 01 to 31, and the schema adds no
calendar rule. `the_schema_accepts_a_configuration_exactly_when_klin_does` in
`tests/schema.rs` pins this with a valid and an invalid file for each shape
that the schema generates separately.

klin and the schema disagree on the files in the two lists below.
`every_known_disagreement_still_disagrees` in `tests/schema.rs` pins each
one. For the files in this first list, no decision says which side is
intended, so each one waits for an owner to decide:

- A convention whose name holds only whitespace: klin refuses it, and the
  schema accepts it.
- A `text`, `code` or `files` matcher that is empty or holds only
  whitespace: klin refuses it, and the schema accepts it.
- A `text` matcher with a line break: klin refuses it, and the schema
  accepts it.
- An accepted entry with an extra field that is not a number: klin accepts
  it, and the schema refuses it.
- A whole number written with a fraction, such as `5.0`: klin refuses it, and
  the schema accepts it as an `integer`.

The schema does not state these rules, which compare one value with another
entry, with a reserved name or with a parsed pattern. klin refuses each file,
and the schema accepts it:

- Two `sarif` entries with the same `name`.
- A `sarif` entry named `measurement-lost`, the name that section 7.2 reserves.
- A `files` glob that does not parse, such as `[`.

The reference MUST also state what the key tables alone do not say:

- the built-in source extensions each check discovers, printed from the
  tables in the binary, and that those tables are capability rather than
  configuration
- the shared default skip list and git-ignore behavior
- the `in` / `except` path shape and the retired topology keys that compact
  sections reject
- the path-to-ceiling shape of `doc_size` and the built-in citation extensions
  of `doc_citations`
- that `publish = false` and `"private": true` do not make a package not
  applicable to `public_api` (ADR 0050)
- that a file which leaves compact scope is reported as lost coverage under
  the base-era scope rule of 8.6

`docs/REFERENCE.md` holds the printed reference, and a CLI test fails when the
committed copy differs from what the binary prints, so CI fails on a reference
that drifted. A person regenerates it with `klin reference > docs/REFERENCE.md`.

### B.6 Window Selection

#### B.6.1 The hook uses the turn window

In `--hook` mode `before` is the turn stamp. The stamp is a git tree object of
the working tree, including uncommitted and untracked files that `.gitignore`
does not exclude. `after` is the working tree. Committing inside the turn does
not move either.

This reverses ADR 0009 for the hook only. The merge-base is the right window
for a pull request and the wrong one for a turn. On the default branch with a
remote it measures unpushed commits and then nothing. With no remote it
measures uncommitted work and then nothing. On a long branch it re-judges
every file the branch touched at every stop. The turn window has none of
these, and it is the window ADR 0014 already built for radius.

#### B.6.2 When the stamp moves

One rule, applied on session start and on every prompt submitted alike:

> The stamp moves to the current working tree on a first session, or when
> the last stop ended green. Otherwise the stamp stays where it is.

So debt an agent left behind stays `new` until it is fixed or a person accepts
it, across turns and across sessions. The stamp persists in the state
directory between sessions, and a session start that finds a red verdict
keeps the old stamp rather than photographing the mess. A first session is a
worktree whose state directory does not exist yet. Its first stamp is the
working tree as it stands, which treats a person's uncommitted work as prior,
and that is correct.

An acceptance does not move the stamp. An accepted entry is a `before` entry
under 7.1, so the site it names is `held` on the next stop, and the stamp
moves when that stop ends green. Accepting finding A while finding B is still
open leaves B `new`. A rule that moved the stamp on acceptance would make B
inherited debt, which is the route to green this section exists to close.

A person who abandons the work has a third route: `klin turn reset` moves the
stamp to the current working tree and prints that a person moved it. The guard
denies the command by name from an agent, the way it denies `init`. Without
this command a red window that nobody acts on degrades into a report that
everyone learns to ignore.

A stamp describes the current turn only while current HEAD history still holds
the commit it was taken over. That commit is the stamp's parent, HEAD at
stamping time, and not the stamp itself: the stamp is a synthetic sibling of
its parent and is never an ancestor of a later commit. So the rule is keyed to
commit history and not to branch names.

> A red stamp stays while the HEAD it was taken over remains in current HEAD
> history. Where current HEAD no longer holds that commit, the stamp cannot
> describe the current turn.

A commit made inside the turn keeps the parent an ancestor of HEAD, so the
turn window stays, and so does `git checkout -b` at the same HEAD and any
branch that descends from the parent. A switch to divergent history, a
detached checkout of an unrelated commit, a hard reset, and a rebase that
drops the parent each take the parent out of HEAD history. The stop then
prints a NOTE that names the reason, judges a branch window from the base of
6.3 for the current checkout, and writes that base as the stamp, red, keeping
the prompt counter and dropping the `asked` record of 8.2 and `intervened`,
because both belong to the turn the checkout left. The handoff records of
9.1 belong to a host session and not to the turn, so the fallback leaves
them: a report the host is about to submit is still recognized, and a
session's next prompt clears its record as always.
The journal records the stop as `branch-fallback` (11.4).

The recovery copies go with it. The stop deletes `refs/worktree/klin/turn`
and `refs/worktree/klin/mark`, because both are copies of a turn the checkout
left and a copy of that turn is the one thing a later stop must not read. A
stop that loses the `turn` file after this widens to the branch window, which
is the window the fallback already judged, so the deletion forgives nothing.
The ref MUST NOT instead be moved to the base: a stamp restored from the ref
reads its parent as `<commit>^`, which names the stamped HEAD for a synthetic
stamp and the commit before the base for an ordinary one, so a moved ref would
name a parent no stop ever took and would pass this section's test on history
that holds no base at all.

klin MUST tell a proven "not an ancestor" apart from a question git could not
answer. `git merge-base --is-ancestor <parent> HEAD` exits 0 for an ancestor,
1 for a proven divergence, and other codes when git refused the question. Only
the proven divergence takes the fallback above. A stamp that names no parent,
which is the shape a stamp taken over an unborn HEAD has, keeps the turn
window.

A stamp is missing when the state directory exists and holds no `turn` file.
A `turn` file that is gone while the ref of 6.5 remains is restored from the
ref with a RED verdict, and a NOTE says so. When the file and the ref are
both gone, the stamp was deleted. The rule above writes no fresh stamp then,
because a fresh stamp would photograph whatever the deletion was meant to
hide. The next stop prints a NOTE that names the missing stamp, judges a
branch window from the base of 6.3, and writes that base as the stamp with
the verdict the gates gave. So deleting a stamp widens the window to the
whole branch. When no base of 6.3 resolves either, the stop judges from HEAD
and says so.

The stamp holds the stamped commit id, the time, the verdict of the last
stop, a prompt counter, the prompt mark of 6.2.1, and the `asked` record of
8.2. `klin gate --hook` writes the verdict and adds to `asked`. A fresh stamp
holds an empty `asked`, so the record goes whenever the stamp moves. `klin radius` applies the rule above and raises the
counter by one on every session start and prompt submitted, whether or not
the stamp moved. The counter is what makes the per-turn block budget of 9.3
literal, because the stamp itself moves only after a green stop. The state directory
the stamp sits in is guarded (9.4).

##### B.6.2.1 The prompt mark

The stamp waits for a green stop, and the radius report cannot. Keyed to a
frozen stamp, the report measures one window that grows with every prompt,
calls it the turn that just ended, and prints on every prompt once that
window passes its value. ADR 0024.

So klin takes a second mark. The prompt mark is a commit over the same tree
the stamp commits, with HEAD as its parent, under `refs/worktree/klin/mark`.
It moves on every session start and on every prompt submitted, whatever
verdict the last stop left. It is held in the `turn` file beside the stamp and
written in the same atomic write.

The prompt mark is the window the radius report measures, and nothing else
reads it. A stop judges the stamp, `klin turn reset` moves the stamp, and the
derivation commit of 6.6 comes from the stamp. A mark the `turn` file has lost
is read from the ref. With neither, the report prints nothing, which is what
it does with any window it cannot measure. Deleting the mark costs a report
and no block, so the mark needs no recovery beyond its ref.

#### B.6.3 `klin gate` by hand and in CI

Candidates, in order:

1. `GITHUB_BASE_REF`: the pull request target, as
   `refs/remotes/origin/<target>` then as a local name. Kind `branch`.
2. The push event's `before` commit from `GITHUB_EVENT_PATH`, when it is not
   the null commit. Kind `push`.
3. The merge-base of HEAD with the default branch, trying the remote HEAD
   symbolic ref, then `origin/main`, `origin/master`, `main`, `master`. Kind
   `branch`.

The first candidate that resolves wins. None resolving is exit 2 with the list
of what was tried. No flag overrides the list.

#### B.6.5 Materialization

Under `--changed` the `before` version of each changed file comes from `git
show`, at its old path for a rename. Without `--changed` the `before` tree is
a detached worktree under a temporary directory, removed after the run. A root
the `before` tree lacks measures nothing there.

The turn stamp is a commit made with `git commit-tree` over a tree from a
temporary index, with HEAD at stamping time as its parent, so both paths above
work on it unchanged. Each capture MUST use a fresh private index path, so a
Git index lock left by an interrupted capture cannot keep later turns open.
The completed turn index is retained in the state directory only after a
usable stamp has been published. It records that a turn has been established;
an unsuccessful capture MUST NOT erase that record. An interrupted first
capture with no published stamp or ref MUST retry as a first session.
Captures for each canonical index MUST serialize explicitly and remove orphaned
capture directories before allocating a fresh one, so interrupted hooks cannot
accumulate private indexes. A read-only radius report retains no turn index.
`a_leftover_private_index_lock_does_not_keep_a_green_turn_open` in
`tests/turn.rs` pins recovery after a green stop;
`an_interrupted_first_stamp_retries_as_a_first_session` and
`the_next_capture_removes_an_interrupted_private_index` pin interruption recovery.
The RECOMMENDED stamping sequence is: allocate a fresh private path under the
state directory, `git add -A` with `GIT_INDEX_FILE` naming that path, then
`git write-tree`, `git commit-tree -p HEAD`, `git update-ref
refs/worktree/klin/turn <commit>`, publish the `turn` file, and atomically
promote the completed index to the retained canonical `index`. The ref keeps `git gc` from pruning the stamp,
makes it visible to `git log --all`, and is the copy a stop restores the
`turn` file from when that file is gone (6.2). The ref is never pushed. The `turn` file
in the state directory holds the time, the verdict, the prompt counter and
the `asked` record of 8.2 beside the commit id, and `intervened`, set once a
stop under the stamp spends a gate block (9.5). A fresh stamp holds neither.
What a host session was handed is not in the `turn` file: it lives in the
handoff records of 9.1, one file per session, so no write of the stamp can
replace it.
It MUST be written to a temporary name and renamed into place, so a hook that
dies mid-write leaves the previous stamp, not a torn one. Two sessions in one
worktree share one window and one `turn` file. Session-start and prompt events
MUST hold the same advisory state lock from before reading the stamp and counter
through publishing the mark, refs, stamp and retained index. They MUST capture
the tree and measure the radius before they take that lock, so a slow capture
cannot make a stop lose the lock and its blocks.
`a_slow_prompt_capture_does_not_cost_a_failing_stop_its_block` in
`tests/copies.rs` pins this. Distinct events
wait their turn; duplicate-event claims alone do not serialize this transaction.
If the lock cannot be acquired within 30 seconds, the event changes no turn
state and prints a NOTE. A manual reset holds this lock too.
`concurrent_distinct_sessions_serialize_the_prompt_counter` in `tests/copies.rs`
pins distinct sessions advancing the counter separately.
A stop MUST hold an advisory
lock on the state directory from before it measures until after it writes the
verdict, so stops in one worktree run in order and the last verdict describes
the last tree. Without the lock an old green stop that finishes after a new
red one would write green, and the next prompt would move the stamp over the
red debt. A stop that cannot take the lock within the hook budget writes no
verdict, spends no build block and no gate block, and says so. It still
measures and reports, but another stop may be writing the counts of 16.3,
so a block it spent could exceed the budget of 9.3. It reads its window
read-only: from the `turn` file, or else the ref, with no restore of a
missing file, no re-anchor of an abandoned stamp and no replacement written
(6.2, 16.1).

The ref is `refs/worktree/klin/turn`, not `refs/klin/turn`. Git shares
`refs/` across the worktrees of one repository, with `refs/worktree/`,
`refs/bisect/` and `refs/rewritten/` as the exceptions, so a stamp under
`refs/klin/` in one worktree would replace the stamp of another and leave it
for `git gc` to prune.

#### B.6.6 The derivation commit

Derived values come from one commit, the derivation commit. Under the turn
window it is the stamp's parent, which is HEAD at the time the stamp was
taken. Under the branch and push windows it is `before` itself. It is never
the stamp, because a stamp is a new commit on every turn and a cache keyed by
it would never hit.

The window names its derivation commit, and a run takes that commit from the
window before any fact of the tree reaches a check. A fact read before the
window is known is read again under it. A run by hand or in CI, whether
`klin gate` or one check's own command, derives from `before` whatever turn
stamp the checkout holds, so a change it judges never moves its own ceiling,
even when the change is committed. The hook derives from the stamp's parent
even when the state directory cannot be written, because the turn ref keeps
the stamp. A run by hand that resolves no base has no `before`, and it
derives from the stamp's parent, or from HEAD when no stamp is readable.
`klin init --pin` judges nothing and has no window, so every value it pins
comes from that same commit, the stamp's parent or HEAD, as `radius` does.
Pinned by `a_branch_run_derives_from_the_base_and_not_from_a_committed_change`,
`init_pin_with_no_turn_stamp_derives_from_head_and_not_from_the_base`,
`init_pin_derives_from_the_stamps_parent_and_not_from_the_base_or_head`,
`a_branch_run_by_hand_derives_from_the_base_and_not_from_a_turn_stamp`,
`a_push_run_derives_from_the_commit_the_push_started_from`,
`a_document_ceiling_comes_from_the_same_base_as_the_complexity_ceiling`,
`a_check_run_on_its_own_derives_from_the_same_base_as_the_gate`,
`doc_size_run_on_its_own_derives_from_the_base`,
`a_stop_whose_state_directory_is_unusable_still_derives_from_the_stamps_parent`
and `a_commit_inside_the_turn_does_not_recalibrate_until_the_stamp_moves` in
`tests/survey.rs`, and by
`reachability_on_its_own_derives_its_families_from_the_base_as_the_gate_does`
in `tests/reachability.rs`.

The survey MUST cache its result under `survey/<commit>.json` in the state
directory, because its numbers and its path sets at that commit are a pure
function of the derivation commit and the binary version. The `after` walk
of 4.3 is not cached and is unioned in at run time. The first stop after a
commit pays one whole-tree parse.
Every stop between two commits reads the cache. The cache is per worktree
like the rest of the state directory (7.4).

A commit inside a turn does not move the derivation commit. The stamp's parent
is fixed when the stamp is taken, and the stamp moves only under 6.2. So a
turn is judged against one set of derived values from start to end, whatever
the agent commits along the way. The derivation commit changes only when the
stamp moves, which is after a green stop, and a person sees the new ceiling on
the `derived:` line of the next run.
A compact-scope edit changes what is judged immediately, but changes a derived
complexity ceiling only when the derivation commit advances.

### B.7 Ratchet Semantics

#### B.7.1 Outcomes

Three outcomes (ADR 0009):

- `new`: over the ceiling in `after`, no matching site in `before` or in the
  accepted list. FAIL.
- `worsened`: matched, and a ratcheted value rose. FAIL. A value the finding
  carries and the matched entry does not counts as a rise, so a value that
  starts to be judged at a site is judged from nothing.
- `held`: matched, no ratcheted value rose. Pass.

A `complexity` site ratchets both `cc` and `lines`, so a function over its
`cc` ceiling is `worsened` when only its `lines` rise, even under the `lines`
ceiling. A function in test code with no `test_lines` pinned ratchets `cc`
alone, because nothing judges its length (8.2.1). Pinned by
`a_test_function_whose_length_grew_is_held_while_its_cc_holds_with_no_test_lines`.
A function over its `cc` ceiling that leaves test code, such as one whose
`#[cfg(test)]` is removed, carries `lines` again, and the base entry does
not, so it is `worsened`. Pinned by
`a_test_function_that_becomes_production_code_is_judged_on_its_length`. No `+N` or percentage tolerance applies (ADR 0062). Pinned by
`a_function_whose_length_grew_since_the_base_fails_too` in
`tests/complexity.rs`.

Below the ceiling nothing is judged. An accepted entry is a `before` entry. A
finding matches at most one entry, and an entry at most one finding.
Identical sites match in the rank order of 4.4.

A site under a path the derivation commit's survey did not hold matches
nothing in `before`, whatever `before` holds there, so when the check judges
it at all it is `new`. Whether the check judges it follows its rule for a
path without a number (4.3, 5.4). A path that was not measured was never
held, so a directory that becomes a root, or a file that becomes a known
language, cannot bring inherited debt with it.

A path under a root the derivation commit's survey held stays held when a
root that survey did not hold also contains it: a root `build.rs` that makes
the crate directory a root keeps the base's sites in `src` held, and only the
paths outside `src`, such as the `build.rs` itself, match nothing in `before`.

#### B.7.2 Scope

`--changed` restricts both findings and entries to the changed files, so an
untouched file's debt never reads as new or as fixed. It announces the file
count it judged. Under the turn window the changed files are the files the
window changed, and an empty set is an honest "nothing changed", not a hole,
because a commit does not empty it.

The runner also makes the full `Change` set available to each gate in a
changed run, separately from the list of files the gate judges. A `Change`
keeps its current path and, where applicable, its base path, so additions,
deletions, modifications and renames remain distinguishable. `Context.only`
continues to mean judgement and report scope; it is not a change or
invalidation model.

#### B.7.3 Tightening

The invariant that makes tightening safe:

> A ceiling change MUST NOT fail a site the base holds. A site over the
> ceiling in both trees with no ratcheted value risen is `held`, whatever the
> ceiling is.

Because of it, lowering a ceiling affects new code only. What tightens, in
order of how much work it does:

1. A person pins a dated schedule once (5.5). This is the mechanism. `init`
   MAY offer to write one from the derived ceiling to a target over a period,
   under a flag such as `--tighten 18m`, and MUST NOT write one unasked.
2. A person pins a lower number.

The derived ceiling tightens nothing. It can rise when simple code leaves the
tree (5.4). A pinned number or schedule is the only ceiling that cannot
loosen, and `init` writes one from today's derived value in one command.

Forced paydown, where a touched file must leave with less debt, is NOT
RECOMMENDED. Its only remedy is a refactor nobody asked for, which the radius
report exists to discourage.

#### B.7.4 What the tool writes, and where

Nothing into the working tree. `init` writes `klin.json` and hook files, and
only when a person runs it.

klin's own state is six things: the turn stamp with the prompt mark of
6.2.1, the build stamp, the handoff records of 9.1 under `handed/`, the
event claims of 9.8 under `claims/`, the cache, which holds the survey of 6.6
and the structural cache of 8.4, and the journal of 9.6. All are per working
tree. The cache is safe to delete. All
six are guarded, because the
guard guards the directory they share (9.4). Deleting the turn stamp buys
nothing, because a stop without one judges the whole branch (6.2). They live
in the state directory:

- By default, `klin/` under the directory `git rev-parse --git-dir` returns.
  Git never tracks it, never lists it as untracked, `git clean` never removes
  it, each worktree has its own, and it goes when the repository goes.
- When `KLIN_STATE_DIR` is set, under that directory, in a subdirectory named
  by a hash of the git common directory and the worktree path. Two clones or
  two worktrees MUST never share a stamp. `~/.cache/klin` is the intended
  value.

An implementation MUST print the state directory under `klin gate --list`.
`klin cache clean` MUST remove the cache for the current tree, and with
`--all` the cache under every entry of `KLIN_STATE_DIR` whose repository
no longer exists. It MUST NOT remove a stamp, and it MUST leave the journal
alone: the journal is history, not a cache. A repository path that does not
resolve on the machine running the command, such as a worktree a container
does not mount, is no proof that the tree is gone, and a stamp is the one piece
of state whose loss lets a block go unspent.

ADR 0004's argument for the location holds in both: an agent empties `target/`
as a matter of routine and does not empty `.git/` or a home cache. ADR 0015's
reason for `.klin/` was one `.gitignore` line, and neither location needs one.

Anyone already running klin has `.klin-build-blocked` or `.klin/` in
`.gitignore`. Those lines become inert. `init` MAY say so when it sees one.

### B.8 Check Catalogue

#### B.8.1 Criteria

A check earns a place when all four hold:

1. **Deterministic and offline.** Same inputs, same output, no network, no
   clock except a pinned dated ceiling.
2. **Additive remedy.** The fix adds or improves code. A check whose only
   remedy is undoing work is a report, not a gate (ADR 0014).
3. **Not a linter's job.** Either it needs git, a second tree, a second
   artifact such as a document or a lockfile, or knowledge across files. Or
   it ratchets a linter's own output.
4. **Names an agent failure.** Something an agent does when it wants green
   cheaply.

#### B.8.2 Tier 1: build these

Every check here except `sarif` compares two trees. "New against `before`"
means a site present in `after` and absent in `before` fails, and a site in
both is held. `inventory` runs the other way: a site in `before` and gone
from `after` fails, as a rise of its `missing` value from 0 to 1.

| Check | Agent failure it names | Identity | Judgement | Derivable | Status |
|---|---|---|---|---|---|
| `escapes` | silenced check, swallowed error, skipped test | file + line text | `count` rises | yes | shipped |
| `complexity` | tangled function written in a hurry | file + declaration | `cc`, `lines` rise | yes | shipped |
| `dead-symbols` | private declaration left unreferenced | file + declaration | `dead` rises from 0 to 1 | yes | shipped |
| `reachability` | implementation file nothing in the repository uses | file | `unreached` rises from 0 to 1 | yes | shipped |
| `doc-size` | instruction file that grows every turn | document | words over a ceiling derived from the derivation commit | yes | shipped |
| `doc-citations` | document that cites a file that moved | document + path | new against `before` | yes | shipped, needs the base comparison |
| `radius` | unprompted wide change | turn | report only | yes | #91 |
| `stubs` | placeholder left behind | file + line text, or file + kind for a comment marker | `count` rises | yes | **new** |
| `inventory` over tests | deleted test file, deleted test function | test file path, or test function site | `missing` rises | yes | shipped |
| `lockfile` | dependency added without a lockfile entry, pin removed, pin the lockfile does not record | manifest + name | `unlocked`, `unpinned`, `stale` rise | yes | shipped, Rust, npm and Go |
| `sarif`, `after` only | anything a linter reports, on a line the window changed | file + rule + message | new on a changed line | no | shipped, section 8.3 |

`inventory` has two identities. A test file is keyed by path. A test
function is keyed like a complexity site, file plus declaration line, and
only functions the language's test convention marks count, such as a
`#[test]` item, a `test_` function or an `it(` call. That table is fixed in
the binary and 8.2.1 states it. `inventory` ratchets one value, `missing`,
which is 0 for every test site in `before` and 1 for a site `after` no
longer holds, so a vanished site is `worsened` under the one judge of 16.4.
Both identities reach that judge together and count in one unit, `test
site(s)`. The second pass of 4.4 matches a renamed test by body hash before
it counts as missing, so a rename with the body unchanged is `held`.

Removing a test is ordinary work, and deleting a failing one is the
cheapest route to green that section 1 names. klin cannot tell which it was, so it
asks once and does not judge the answer (ADR 0031):

- Under `--hook`, a vanished site is `worsened` and blocks the stop, and the
  remedy asks the agent to restore the test and fix the code if the test
  failed, or to say why the removal is intended and stop again. A stop that
  blocks records the site id (11.2) of every finding it reported in the
  `turn` file, beside the stamp (6.5). A vanished site that record holds is
  let through: its `before` entry carries `missing: 1`, so the one judge
  holds it, and the run lists it in a NOTE. The report names every finding
  it records, so nothing reaches that record unasked. The record goes when
  the stamp moves, and the guard refuses an agent's write to the file that
  holds it (9.4). The rule keys on `--hook` and not on the window kind,
  because a stop whose stamp was deleted judges a branch window (6.2), and a
  stamp that went takes the record with it, so such a stop asks again.
- Everywhere else, every vanished site is let through the same way and is a
  NOTE, `--strict` included.

An accepted entry that names a vanished site still takes the match, by
entry order (16.5), so it neither fails nor turns into an entry that matched
nothing. Nothing klin prints asks for one.

A deletion that removed the subject too is a NOTE: for a file, the subject
file went in the same window, and for a function, the file that held it
went. The subject of a test file is
the file in `before` whose path equals the test's path with the test
affixes stripped: a `test_` or `spec_` prefix, a `_test`, `_spec`, `.test`
or `.spec` suffix before the extension, a `Test` or `Tests` suffix on the
basename as in `FooTest.java` or `FooTests.swift`, and a `tests/`, `test/`,
`spec/` or `__tests__/` directory segment. The table is fixed in the binary
and printed with the NOTE.


`lockfile` proves one thing: every dependency the manifest names has an entry
in the lockfile beside it, no pin the base held is gone, and the lockfile
records the version of each exact pin. It cannot prove
that a package exists in a registry, because it runs offline. A dependency
that does not exist fails the project's own install, when the `build` step
runs one. A derived build runs no install, so a declared dependency that was
never installed leaves the build's tool absent, and 9.3 makes that a NOTE
that lets this gate speak. Workspace members, path dependencies and optional dependencies are
implementation-defined and MUST be documented per manifest format, which
8.2.1 does for the five formats that ship.

The first version covers `Cargo.toml` against `Cargo.lock`, `package.json`
against `package-lock.json`, `pnpm-lock.yaml` and `yarn.lock` against
`package.json`, and `go.mod` against `go.sum`. The pnpm reader recognizes a
`lockfileVersion` major of 4 or later and reads package names from the direct
keys of its `packages` mapping. It accepts the path-shaped keys of older
lockfiles and the `name@version` keys of newer ones, including peer suffixes.
The Yarn v1 reader recognizes the `# yarn lockfile v1` marker and reads names
from its top-level, comma-separated selectors. The Yarn 2+ reader recognizes
the top-level `__metadata.version` value of 4 or later and reads names from
its top-level locators. These readers scan only the package-key shapes they
need and do not parse YAML values. A pnpm or Yarn file without its recognized
marker, section or key shape is one NOTE per run and no finding. `poetry.lock`
and `uv.lock` need a TOML reader and remain follow-ups. A manifest whose
lockfile format klin cannot read is one NOTE per run and no finding, so such a
manifest never reads as a pass.

Two more checks belong to this tier by the criteria and are not in the core
list of the 0.x implementation checklist, because each takes weeks and carries an unsolved problem:

| Check | Agent failure it names | Identity | Judgement | Status |
|---|---|---|---|---|
| `duplication` | copy instead of reuse | block in changed lines, and tree share | `share` rises | #48 |
| `sarif` with `compare` | a linter's findings compared across two trees | file + rule + message | `count` rises | #47, section 8.3 |

`stubs` is new. Its patterns per language are a fixed table the way escapes
are, and every pattern in the first table is an executable body that does
nothing: `todo!()`, `unimplemented!()`, `pass` as the sole body of a
function, `raise NotImplementedError`, `throw new Error("not implemented")`,
an empty body on a function the language's test convention marks, and an
elision comment such as `// ...` or `# rest of the code` as a sole body. The
sole-body patterns match function bodies only, because `pass` as the body of
a class or an exception is ordinary Python. The comment markers `TODO`,
`FIXME`, `XXX` and `HACK` are in the table too, as #106 decided. A project
that tracks work in such comments sees a new one fail once, and a person
accepts it or the agent moves the note to the tracker. An abstract
declaration whose body is meant to be empty, such as a trait method or a
protocol, MUST NOT match. Identity is file plus line text, ratcheted on
`count`, exactly like escapes, except for a comment marker. A comment is not
a declaration, so every comment marker in one file is one site, keyed by the
file and the row kind and ratcheted on its count (4.4, 8.2.1, ADR 0064). It
SHOULD share the escapes engine and differ only in the table. #106 shipped
the line patterns and #114 the body shapes, which the function walk reads.

The escapes table gains three rows for test-disabling constructs it lacks:
`fit(`, `fdescribe(` and `pytest.mark.xfail`. `skipif` is not a row, because a conditional skip states which platforms a test supports. The
other focus and skip markers, `.only`, `.skip`, `xit`, `#[ignore]`,
`@Disabled`, `t.Skip` and `XCTSkip`, are already there.

For the same reason a Rust `#[cfg_attr(P, ignore)]` or
`#[cfg_attr(P, ignore = "...")]` is a skipped test only where `P` is always
true under Rust's cfg rules, such as `all()`, `not(any())` or a nest of
these. Such a test never runs, as under a bare `#[ignore]`. A configuration
option, such as `windows` or `feature = "slow"`, may hold on one target and
not on another. So `any(windows, not(any()))` always holds and is a site, and
`all(windows, not(any()))` states where the test runs and is none. A
predicate that never holds, such as `any()` or `false`, skips nothing, so it
is no site either. `true` always holds, and so does `test`, because a test
runs only where `cfg(test)` is set. `ignore` counts at any place after the predicate, and inside a nested
`cfg_attr` whose predicate always holds too. The pattern finds `#[ignore` and
`#[cfg_attr` with any whitespace or comment between their tokens, and the
Rust grammar then reads the `cfg_attr` it found, so whitespace and comments
inside the attribute change nothing.

The remedies MUST name the legitimate repair (#433):

- `doc-size` tells the agent to keep in the file what the task asked for;
  only a person raises the ceiling, in a reviewed commit.
- `doc-citations` tells the agent to update the citation when a file moves
  or is renamed. It offers deleting the citing sentence only when the
  referenced content was intentionally removed and the sentence no longer
  applies.
- `lockfile` on an unlocked dependency tells the agent to report why an
  install cannot run and forbids writing lockfile entries by hand.
- `escapes` tells the agent to remove a skip or fix what made the test fail,
  and says that swallowing an error in place of the escape is not a fix.
- `public-api`, including its hook remedy, says that an added optional
  parameter or overload is still a changed contract (ADR 0054).

These are output requirements; measurements, ratchets, block policy,
accepted entries, journal, stats and CI semantics are unchanged.

##### B.8.2.1 Measurement rules of the shipped checks

The table above names what each shipped check measures. This section states
how, so a second implementation reproduces klin's own numbers and so a rule
that looks wrong is disputed as a rule, not rediscovered in source. Every
rule here is pinned by a CLI test under `tests/`, named beside it. Section 5.8
owns the compact policy keys and built-in language coverage, and links here.

**`doc-size` counts words.** A word is a maximal run of characters that are
not Unicode whitespace, as `White_Space` defines it. A no-break space and an
em space split words the way an ASCII space does. Nothing is stripped: a
heading marker, a fence, a backticked span, an emphasis marker and every
token inside a code block are words. A byte sequence that is not valid UTF-8
is decoded lossily, and each replacement character is part of a word, never
an error. The count compares to the ceiling with `words > ceiling` failing,
so a document exactly at its ceiling passes, and lands inside the two
percent margin that adds a WARN line. Pinned by
`a_word_is_a_run_of_non_whitespace_so_markup_counts_and_unicode_spaces_split`
and `a_byte_that_is_not_utf8_is_read_as_one_word_not_an_error` in
`tests/doc_size.rs`. Known limit: the count rewards terse markup and
punishes fenced examples equally. That is a design choice for a separate
ticket, not a defect of the rule. `doc-size` is a per-document size policy
(5.4). It does not bound the total context an agent reads, so many small
instruction files that together pass every ceiling are not a failure.

**`escapes` and `stubs` aggregate matches into sites.** A site is one file
plus the text of one line with leading and trailing whitespace trimmed, except
for a `stubs` comment marker, whose site is keyed by its file and kind. Every
match of every pattern in the language's table, on every line whose trimmed
text is equal, lands on that one site. Its `count` is the number of those
matches. Its label and remedy are the ones of the first pattern, in table
order, that matched a line with that text, and its line is the first line
that pattern matched. The built-in language table comes first. A line that
carries two kinds is one site labelled by
the earlier row, and a second copy of that line, indented differently, adds
its matches to the same site rather than opening another. `escapes` reads
the text as written, so a pattern inside a string literal is a match. Unless
`skip_test_idioms` is `false`, it leaves each language's test idioms out of
that language's test code and counts them on the coverage line as skipped in
tests. Each language's table names its test idioms and its test code, and a
language that names none has every row judged in a test. The Rust test idioms
are `unwrap` and `expect`, and Rust test code is an inline `#[cfg(test)]`
module or a `.rs` file under a test root the survey finds in the tree being
read (5.4). The TypeScript and JavaScript test idiom is `@ts-expect-error`, a
row of its own, and their test code is a test file of 5.4: one under a test
root, or one a test directory segment or a test affix marks. It needs no
description, the same way a reason on `#[ignore]` silences nothing. Each tree
is classified over its own files, so a root that stops being test-only has its
production sites judged. Every other row is judged in a test as anywhere else,
so a `#[ignore]`, an `#[allow(...)]`, an `unsafe { }`, a `@ts-ignore`, a
`@ts-nocheck` or a non-null `!` inside a test is a site, and a
`@ts-expect-error` in production code is a site (ADR 0049, ADR 0060). Pinned
by `a_site_inside_a_cfg_test_module_is_not_a_production_site`,
`a_ts_expect_error_in_a_typescript_test_file_is_left_out_by_default`,
`every_other_typescript_escape_is_still_judged_in_a_test_file`,
`skip_test_idioms_turned_off_judges_the_idioms_of_every_language`,
`skip_test_idioms_turned_off_judges_the_test_module_too`,
`unwrap_and_expect_in_a_file_under_a_test_root_are_left_out_by_default`,
`a_skipped_test_under_a_test_root_is_still_an_escape`,
`a_skipped_test_inside_an_inline_test_module_is_still_an_escape`,
`allow_and_unsafe_in_rust_tests_remain_escapes`,
`skip_test_idioms_turned_off_judges_a_file_under_a_test_root_too` and
`production_rust_beside_a_test_root_is_judged_as_before` and
`a_root_that_stops_being_test_only_has_its_new_production_unwrap_judged` in
`tests/escapes.rs`.
A Rust `cfg_attr` that carries `ignore` is a skipped test only where its
predicate always holds, as 8.2 states. Pinned by
`a_cfg_attr_whose_predicate_always_holds_is_a_skipped_test`,
`a_skipped_test_is_found_through_whitespace_comments_and_nesting`,
`a_cfg_attr_on_test_or_a_true_literal_is_a_skipped_test` and
`a_cfg_attr_whose_predicate_may_not_hold_is_no_skipped_test` in
`tests/escapes.rs`.
`stubs` throws away a match that lies wholly inside a quoted span on one
line, judges a test module like any other code, and refuses the key. Pinned by
`repeated_lines_of_two_kinds_fail_as_one_site_labelled_by_the_first_pattern_with_every_match_counted`,
`a_line_carrying_two_escape_kinds_counts_both_under_the_first` and
`the_same_line_twice_in_one_file_is_one_site_whose_count_ratchets` in
`tests/escapes.rs`. Known limit: the label hides the second kind on a mixed
line. A finding that says `unwrap x4` may hold two `expect` calls.

`stubs` keys a comment marker by its file and the row kind,
`comment marker`, in place of a line text (ADR 0064). Every marker match in
one file lands on that one site, its `count` is the number of matches, and
an accepted entry names `comment marker` as its `text`. So a typo fix inside
a marker, a change from `TODO` to `FIXME` and a move within the file hold
the count, and a new marker or one moved in from another file raises it. The
gate pairs each marker match with one base match of the same trimmed text in
the same file, and the matches left over are the lines the base file lacks.
The site's line is the first of them, and its `new_lines` value lists them
all as one string, such as `"1, 3"`, so a failure names each one. The first
named line may be an edited marker and not the new one. A line that holds a
marker and a code stub is two sites, and a code stub keeps its line text, so
an edited `todo!()` line is a new site. An accepted entry written before
ADR 0064 for a line that holds a marker and a code stub or a body shape keys
the site of that code stub or body shape. Where the base holds that site,
the base entry shares the exact count and takes the match (4.4), so the
accepted entry matches nothing: a NOTE, and a failure under `--strict`. The
marker is then held at the base. Where the base does not hold that site, the
accepted entry holds it, `--strict` does not name it, and the marker fails
as new. The pairing is `n log n` in the marks of a file, the line count is
linear in the file for each pattern, and the quoted-span lookup is a binary
search. Known limit: a reworded marker, or a marker deleted while another is
added in the same file, holds the count.
Pinned by `a_typo_fix_inside_an_existing_marker_is_held`,
`a_new_marker_in_a_file_that_holds_one_raises_its_count_and_names_the_new_line`,
`every_marker_line_the_base_file_lacks_is_named`,
`an_edited_not_implemented_line_is_a_new_site`,
`a_marker_moved_within_a_file_is_held_and_one_moved_to_another_file_is_new_there`,
`a_marker_and_a_body_shape_on_one_declaration_line_are_two_sites`,
`an_accepted_marker_entry_names_the_row_and_holds_at_its_count`,
`an_accepted_entry_for_a_line_that_held_a_marker_and_a_code_stub_holds_the_code_stub`,
`an_accepted_entry_for_a_mixed_line_the_base_holds_is_stale_and_the_base_holds_both_sites`
and `a_file_of_two_hundred_thousand_distinct_markers_is_judged_in_seconds`
in `tests/stubs.rs`.

**`stubs` judges three body shapes.** The function walk of `complexity`
reads them, over every grammar the built-in stubs table supports, and
`stubs` records each one at the declaration line of the function that holds
it, as one more match on the site of that line. A shape is read off a body that
holds a run of statements, so a concise arrow body such as `() => value` is
one expression and does the work of one. A function whose body holds one
`pass` statement is a `pass body`. A function whose body holds no statement,
and one comment that starts with `...` or with `rest of the`
case-insensitively once the comment markers are off it, is an `elided body`.
A function whose body holds no statement, and whose declaration the
language's test convention marks, is an `empty test`. A comment is not a
statement in any of the three.

A body meant to be empty is not a shape. A declaration that carries no body,
such as a trait method without a default or an interface method, has none. A
Python declaration under a decorator that names `abstractmethod`,
`abstractproperty` or `overload` has none, and neither does one in a class
whose bases name `Protocol` or `ABC`. Both are read as whole names, the last
segment of a dotted one, so `StoreABC` is not `ABC` and a route argument
that spells `overload` exempts nothing. `pass` in a class body, an exception
class included, is not a function body and never matches. A function that
starts on the declaration line of a function that holds it is not judged
either, because the line it would be reported at is not its own: a callback
written inside the call that declares a test carries the test's line and not
its shape. Pinned by
`a_pass_body_fails_and_the_same_declaration_with_a_body_stays_green`,
`an_elided_body_fails_and_a_comment_that_elides_nothing_stays_green`,
`an_empty_test_body_fails_and_a_test_rewritten_with_the_same_declaration_stays_green`,
`an_empty_body_under_a_multi_line_tokio_test_is_an_empty_test`,
`pass_on_an_exception_class_and_on_an_abstract_declaration_is_not_a_stub`,
`a_callback_on_the_line_of_a_test_declaration_is_not_an_empty_test` and
`a_decorator_or_a_base_whose_text_only_spells_a_marker_does_not_hide_a_pass_body`
in `tests/stubs.rs`. Known limit: another shape that stands in for work,
such as `return null` or `{}` on a function no test convention marks, is not
judged, and adding one is a spec change with its own legitimate-use fixture.
Second known limit: a text the grammar rejects keeps its line patterns and
loses its shapes, and the run says nothing about the loss, so a file that
does not parse can only under-report. A parser resource error is not a
grammar rejection: `stubs` MUST propagate the named error of section 13,
including in a direct `stubs` command or a run selecting only that gate.
Pinned by `an_oversized_source_preserves_the_named_resource_error_in_stubs`
in `tests/stubs.rs`.

**`dead-symbols` judges private declarations.** The structural index supplies
module-level functions, methods, types, constants and variables from Rust and
TypeScript, with TSX treated as TypeScript. A declaration is dead when no
reference with the same name exists outside its own declaration. A
TypeScript destructuring declaration is judged by the names it binds: it is
dead only when none of them has a reference outside the declaration. A
renamed binding such as `{ add: loaded }` is judged by its local name, and a
nested or defaulted pattern binds every name inside it, never a default value
or a computed key. A name a `const`, `let` or `var` destructuring declaration
binds is no reference to that name, at the top level and in a function body,
so such a destructuring keeps no other declaration of a name it binds alive.
The head of a C-style `for` is a declaration, so `for (let [first] = [0]; ; )`
binds `first` as a `let` statement does. A name a parameter, a `for…in` or
`for…of` head, a `catch` clause or a Rust `let` binds, or an assignment such
as `[first] = load()` writes, still reads as a reference, unless a pattern
writes it as a shorthand such as `{ name }`, which never reads as one. A
field name reads as a reference too (ADR 0035). A name
resolves to every same-name declaration, so ambiguity keeps each declaration
alive. Declarations marked externally visible, Rust `main`, and functions the
shared test convention recognizes are not judged. The `ignore` list adds name
globs. A glob is matched against each name a declaration binds, so a
destructuring declaration is left out only when every name it binds matches
one. The check is name-only: it does not resolve imports, types, reflection,
framework entry points or external callers. The index reads a string as text,
with two exceptions. Where a Rust attribute item holds `serde(...)`, as the
attribute itself or directly inside `cfg_attr`, the string value of `default`,
`skip_serializing_if`, `serialize_with`, `deserialize_with` or `getter`, plain
or raw, is the path of a function the derive calls. The string is read by its
value, every escape decoded, and a line continuation drops its newline and the
whitespace after it. A string with an escape klin cannot decode names nothing.
The Rust grammar reads the decoded value as one path expression, and the name
that path ends in is a reference, and no other segment is. So generic
arguments, a qualified-self prefix such as `<T as Trait>` and a const-generic
block, with any literal or comment inside it, never change which name that
is, and `Accessor::<u8>::get` references `get` alone. A value the grammar does
not read as exactly one path names nothing. It
is an ordinary reference, so a changed run widens on it and `reachability`
counts it. The same `serde` tokens inside a macro call are not an attribute,
so no path they spell is a reference. `with` names a
module, which no one reference stands for, so its string stays text, as do a
`default` with no value and the string of any other key, such as `rename`.
Inside a string literal that a Rust macro call receives or a `macro_rules!`
body holds, plain or raw and read by the same decoded value, the name of each
`{name}` or `{name:spec}` capture is a reference, and so is a width or a
precision the spec names, such as `WIDTH` and `PREC` in `{v:>WIDTH$.PREC$}`.
So a name only `format!("{name}")` or `format!(r#"{name}"#)` uses is alive. `{{` is a brace, and a position such as `{0}` or `{}` names
nothing. The macro is not resolved, so a string any macro receives is read
this way, and under the name-only rule an extra reference can only make a
declaration look used. A
declaration that becomes dead
after being referenced at the base is `worsened`; a dead declaration already
held at the base is one NOTE and never fails. When it can, a worsened finding
names the first base file that held a lost reference. `--report` prints the
complete current dead-symbol list. A changed run that is not strict builds
declaration state only for the files in its effective judgement scope, over
both trees and under the base's own path and rename semantics (6.5). That
scope is the run's physical scope plus the files that declare a name a changed
file references on one semantic side and not the other. A declaration that did not
move can still turn from referenced to dead when its last caller changed, so
the physical scope alone is not the semantic impact scope. A name a changed file references on both sides cannot flip
one, so it widens nothing. The names come from the same structural facts the
index is built from, never from a textual diff,
and a name with several declarations widens to all of them, which keeps the
ambiguity rule of ADR 0035: fail less, never more. A changed file whose
working-tree text the grammar could not read widens nothing from its base
reference names: that hole is reported as a hole, and missing evidence never
becomes proven deadness. One effective scope drives the state of both trees,
the ratchet, the base and accepted matching, `lost_reference`, the notes, the
counts and the coverage line, so no run reports a finding at a site it says it
did not judge. `Context.only` keeps its runner meaning: the check derives this
scope locally. Its evidence stays whole: both
trees keep the complete index of 8.4, so a judged declaration is alive on a
reference from any measured file, changed or not, and a lost reference in an
unchanged file still explains a worsened finding. A whole run, a strict run
and the check by hand build state for every eligible declaration. Pinned by
`a_new_private_unreferenced_rust_function_fails_as_new`,
`a_private_typescript_main_is_judged`,
`losing_the_last_reference_is_worsened_and_names_the_old_reference_file` and
`one_typescript_reference_keeps_duplicate_names_alive` in
`tests/dead_symbols.rs`; the destructuring rule by
`an_object_destructuring_declaration_whose_binding_is_used_passes`,
`an_array_destructuring_declaration_whose_binding_is_used_passes`,
`a_renamed_binding_used_by_its_local_name_passes`,
`a_nested_or_defaulted_binding_keeps_its_declaration_alive`,
`a_destructuring_declaration_whose_bindings_are_all_unused_still_fails`,
`a_destructuring_declaration_with_one_used_binding_passes`,
`a_property_key_a_computed_key_and_a_default_value_bind_nothing`,
`two_unused_destructurings_of_one_name_do_not_keep_each_other_alive`,
`a_destructuring_inside_a_function_keeps_no_declaration_of_its_names_alive`,
`ignore_globs_match_every_name_a_destructuring_binds`,
`a_changed_caller_that_drops_the_last_binding_reference_worsens_the_destructuring`,
which also requires a second run over the structural cache to print the same,
and `a_destructuring_that_loses_a_reference_in_two_callers_names_the_first_base_file`;
the `serde` strings by
`a_private_function_only_a_serde_default_names_passes`,
`a_private_function_only_a_serde_skip_serializing_if_names_passes`,
`a_private_function_no_serde_key_names_still_fails`,
`a_serde_attribute_inside_cfg_attr_names_its_function_too`,
`a_serde_path_references_only_its_last_segment`,
`a_serde_with_module_names_no_function`,
`serde_tokens_inside_a_macro_call_name_no_function`,
`a_raw_string_serde_path_names_its_function`,
`a_generic_qualified_serde_path_references_its_terminal_callable`,
`a_qualified_self_serde_path_references_its_terminal_callable`,
`an_escaped_serde_string_is_read_by_its_value`,
`a_const_generic_block_in_a_serde_path_keeps_its_terminal_callable`,
`a_brace_in_a_char_literal_of_a_const_generic_block_keeps_the_terminal_callable`,
`a_brace_in_a_string_or_a_comment_of_a_const_generic_block_keeps_the_terminal_callable`,
`a_continued_serde_string_is_read_without_the_whitespace_after_the_newline` and
`removing_a_serde_attribute_in_a_changed_file_worsens_an_unchanged_helper`,
with `a_member_a_serde_string_names_is_reached` in `tests/reachability.rs`;
the format captures by `a_private_const_only_a_format_capture_uses_passes`,
`a_private_const_only_a_raw_format_capture_uses_passes` and
`a_private_const_only_a_width_or_a_macro_rules_capture_uses_passes`; the test
convention by
`a_tokio_test_passes_inline_and_under_a_test_directory` and
`a_multi_line_test_attribute_marks_its_function`; the report cap
is covered by `report_lists_every_current_dead_symbol_without_the_note_cap`,
and the
judgement scope by
`a_changed_run_builds_no_state_for_the_declarations_it_does_not_judge` and
`unrelated_historical_debt_outside_the_changed_scope_stays_silent`, and the
semantic impact scope by
`removing_the_last_reference_in_a_changed_caller_worsens_an_unchanged_declaration`,
`a_reference_another_unchanged_caller_still_holds_is_no_regression`,
`deleting_the_only_caller_worsens_the_unchanged_declaration`,
`every_declaration_of_an_affected_name_stays_conservatively_in_scope`,
`a_changed_file_that_keeps_its_reference_names_widens_nothing` and
`a_changed_caller_the_grammar_cannot_read_guesses_no_deadness`.

In a changed run that is not strict, which includes the hook, the two trees
`dead-symbols` and `reachability` compare share one base extraction. The run's
change set (4.5) is the authority: a working-tree file it does not name, and
that the base lists under the same name, holds the base's bytes at the same
path. The working tree takes the base extraction's outcome for that file and
does not extract it again. An added, modified or renamed file, and a file the
base does not list under the same name, such as one renamed by case alone
where git does not see the rename, is extracted from the working tree. The
base is laid out whole with each renamed file at its current path, so its old
bytes are read under the grammar of that path, and an extension-changing
rename such as `.ts` to `.tsx` reads them as TSX. The TypeScript rule for a
declaration file (8.2) reads that path too, so a rename such as `.d.ts` to
`.ts` reads the old bytes as a file that is no declaration file, pinned by
`a_declaration_file_renamed_to_a_module_is_read_at_the_base_under_its_new_name`.
Each tree still selects its
own files under its own scope and keeps the facts it selected; a
name-resolving check builds its own index lazily from those facts, and no parse
tree outlives its extraction. A strict run, a run that
is not changed, and the check by hand extract both trees. Known limit: a
change git does not report, such as an edit to a file marked `assume-unchanged`
or `skip-worktree` or bytes a clean filter hides, reads as the base's bytes in
a changed run. That run's scope already leaves the file's own findings out,
and the names the file declares and references are the base's.

The same runs keep the base's structural outcomes between runs in the
structural cache, one file per base commit under `cache/structural/<commit>`
in the state directory (7.4). The file name is the full object id of the
commit the run compares against, so a branch name never keys it, and a red
turn reads the cache of its own stamp. For each path of the base tree that
the run's change set does not name, the file holds the outcome the base
extraction came to: facts without a parse tree, unsupported, unparsed or
foreign. A later run over the same commit takes those outcomes in place of
reading, parsing and extracting the base's copy of each file. It still
extracts every path its own change set names, at the path and under the
grammar the base tree gives it. A run that extracted an outcome the file did
not hold writes the whole file again through the atomic replacement of ADR
0041, and that write keeps the four newest files of the structural cache and
removes older ones, so an evicted commit costs a later run one extraction.
Each file carries an identity: an explicit schema epoch, the binary version,
a checksum of the extraction sources and `Cargo.lock` the binary was built
from, the commit, and a checksum of the configuration's root, git's
configuration and the attribute files git reads outside the tree, which decide
the bytes a checkout of the commit writes. A file that is missing,
carries another identity, fails its body checksum or does not decode to the
end reads as no file, and the run extracts the base as it would without one.
The cache changes what a run costs and never what it reports: the findings,
notes, coverage and exit codes are the ones a run without it gives. Strict
runs, whole runs and the check by hand neither read nor write it, and `klin
cache clean` removes it. Known limit: the identity does not name the
system-wide attributes file or the behavior of a filter program, so a change
to either between two runs over one commit, with git's configuration
unchanged, reads the outcomes the earlier checkout gave.
`tests/structural_cache.rs` pins the repeated run, a damaged file, a file
copied from another commit, a smudge filter added between two runs, the
four-file bound, and repeated red stops through a prompt and a branch switch. `tests/structural_views.rs` repeats each changed
caller over the cache and requires the same output.

A run that reads that cache lays the whole base out without checking the base
commit out. It registers the base commit as a linked worktree with no file on
disk, reads the commit into that worktree's index, and writes from the index
only the base files a gate may read: every file the index holds, less the
source files whose facts the cache already holds. The base tree's file list is
the index's, under the rules of 4.3, so no walk and no ignored-path discovery
runs in the base; a fresh checkout holds tracked files only, so the base's
ignored set is empty either way. A submodule entry is no file, as an
uninitialized submodule is an empty directory. A symbolic-link entry is no
file where git writes a link, and is a file where `core.symlinks` is false and
git writes the target path as a plain file. Every file the layout writes is
written before any rename moves it, so git converts each one under the base
commit's own attribute topology, which is the topology a checkout of the
commit reads. Renames then move files to today's paths, as they do for a base
checked out whole.

The layout changes what a run costs and never what it reports. A strict run, a
run that is not changed, and the check by hand check the base out whole. So
does a run with no state directory, with no cache to name, with a cache it
cannot read, whose index holds a sparse checkout or any other shape this
layout does not read, or whose git commands refused. klin asks git itself what
`core.sparseCheckout` and `core.symlinks` are worth, so every spelling git
reads as a boolean reads the same way here, and a value git will not read as a
boolean sends the run to the checkout rather than to a default. A layout that
could not be completed removes its worktree before the run checks the base out
on the same directory, so no half-laid base reaches a gate. `tests/structural_cache.rs`
and `tests/structural_views.rs` compare each warm run with the same run over a
removed cache and require the same findings, notes, coverage and exit code.

The shared structural view keeps imports and module declarations alongside
declarations and references, and keeps unparsed and unsupported outcomes as
coverage data. The project's Change data remains separate from the structural
scope, so a consumer can reuse facts without losing which paths changed.
`layering` and `public-api` consume those facts directly through the
module-resolution layer; neither builds a `SourceIndex`, because they resolve
modules and exported paths rather than declaration and reference names. `dead-symbols` and `reachability` request
their own name index only when they judge names, so each measurement builds at
most one index and the structural cache remains a cache of facts only. A
measurement holds its facts sorted by path, the one order its index also reads
them in, so the two views of a measurement never disagree on order.
`tests/structural_views.rs` requires a cached base of imports, module
declarations and an unparsed file to be read and parsed only for the changed
file, and the cache round-trip unit test pins the import and module fields.

The findings, notes, coverage and exit codes of a changed run are the ones
two independent extractions give. `tests/structural_views.rs` pins the
gate, changed and hook callers for edits, additions, deletions, both rename
classes, scope movement, name ambiguity, a lost reference, an unsupported
language, unparsed files and a case-only rename git does not see. With
`KLIN_DIFF_BIN` naming an earlier build, each of those tests also requires
that build to print the same normalized output for the gate, strict,
changed, hand `--report` and hook callers over the same trees.

**`reachability` judges files of a derived family.** The derivation commit
proves each family from a directory, concrete extension and basename pattern;
the report exposes its name, roots and pattern, while configuration may only
narrow all derived families with `in` and `except`. A member is reached when another
file holds a reference with the name of one of its eligible declarations:
functions, types, constants and module-level variables that are not entry
points. A TypeScript destructuring declaration in another file that binds
that name reaches the member too, in whatever form it binds it, shorthand
included, so `const { default: Profile } = await import(…)` reaches the file
it loads while `Profile` is still unused. A named TypeScript re-export in
another file, `export { x } from "./m"` or `export { x as y } from "./m"`,
is a reference to `x` from the file that holds it, under the same name-only
rule, so a name several files declare still reaches each of them. A re-export
names a declaration by the name the declaration exports it under: `default`
for `export default function Profile` or `export default class Profile`, and
its own name otherwise. So `export { default } from "./m"` and
`export { default as Profile } from "./m"` name that declaration, and
`export { Profile } from "./m"` does not. A re-export proves a member only
where one declaration under the index answers to that name, so a `default`
re-export proves nothing while several files export a default, and proves the
only default of the index wherever the re-export points. The rule
reads the export facts the structural extraction already holds and resolves
no module. A star re-export, `export * from "./m"` or
`export * as ns from "./m"`, names nothing and reaches no member (ADR 0061).
`dead-symbols` reads no re-export as a reference, because it judges private
declarations, which no re-export can name. Pinned by
`a_member_only_a_named_re_export_in_another_file_names_is_reached`,
`named_re_exports_in_another_file_prove_a_family`,
`a_default_member_a_bare_default_re_export_names_is_reached`,
`a_default_member_a_renamed_default_re_export_names_is_reached`,
`a_default_member_a_re_export_of_its_own_name_leaves_unreached`,
`a_re_export_of_the_only_default_proves_its_member`,
`a_default_re_export_proves_no_member_while_several_files_export_a_default` and
`a_member_only_a_star_re_export_names_stays_unreached` in
`tests/reachability.rs`, and by
`a_re_export_of_its_name_in_another_file_keeps_no_private_declaration_alive`
in `tests/dead_symbols.rs`. Methods are not eligible, because a
name such as `run` or `get` recurs across unrelated types and the name-only
rule would reach every file that declares one. Exported declarations are
eligible, unlike in `dead-symbols`, because a family says its files are
wired inside this repository. A reference from the file itself reaches
nothing. Resolution is
the structural index's name-only rule, so a name several files declare
reaches every one of them: ambiguity makes a file look reached and never
unreached. The index covers the whole tree in each family's language partition, so
every run resolves against unchanged callers. The check takes no
scope: a changed run judges every member of both trees, because the edit that
strands a member is an edit to its caller and not to the member, so a run
narrowed to the changed files would judge no member at all and report every
site as held at the base. A derived family is small by construction, and the
index is built over the whole partition either way, so the whole judgement
costs a lookup per member, and the check reads no second extraction of its
own. The physical changed-file set is therefore not this check's judgement
boundary, and the wider boundary raises no old debt: a member unreached in
both trees stays one NOTE under the ordinary two-tree ratchet, and never
fails because a run re-judged it. Identity is the
repository-relative path, so a file two families match is judged once,
under the first family in the list, and an accepted entry names the path. A
file under a test directory, or one a test affix marks, is no member in either
tree a run judges (5.4), so a test written beside a family's files is judged
by no family. A
measured member with no eligible declaration is measured and not judged,
and is neither unreached nor unsupported. A file that leaves the tree is
`inventory`'s and no finding here. A new unreached member fails as new, a
member that loses its last external reference is `worsened`, and one
unreached in both trees is one NOTE. The remedy names the first proven
reached sibling of the family in path order, and none when every sibling is
unreached or reached only through a shared name. The remedy also names the
conflict with a public-api break over what an unreached member held, on
every unreached finding because a check reads no other gate's findings, and
keeps the two decisions apart. The task decides the public contract: it is
restored where the task keeps it, and the break is left for a person to
accept where the task removes it. Whether the implementation is still needed
decides the file: it is kept and wired in, or deleted only if unused. A
public-api identity names no file, and an unreached member may still be live
through a caller this check does not resolve, so neither decision implies the
other. Pinned by
`an_unreached_file_that_held_a_public_api_break_names_the_conflict_and_not_a_bare_delete`.
The check does not resolve imports, `mod foo;`, side-effect imports,
star re-exports, string registries,
dependency injection, framework discovery by name or attribute, macro or
build-generated callers, or callers outside the tree, which belong to the
module graph of `layering` or to no V1 check; a family wired that way is narrowed by path or accepted by a
person. Two files that reference only each other read as reached. Pinned
by `a_new_command_file_nothing_references_fails_as_new`,
`losing_the_last_external_reference_is_worsened`,
`one_ambiguous_reference_reaches_every_file_that_declares_the_name`,
`a_file_with_only_entry_points_or_methods_is_measured_and_not_judged`,
`the_remedy_names_a_proven_sibling_and_not_one_reached_by_ambiguity`,
`a_family_the_base_proves_is_derived_and_judges_a_new_working_tree_member`,
`a_changed_run_judges_a_member_a_dispatch_edit_stopped_referencing`,
`the_stop_hook_blocks_a_turn_that_left_a_member_unreached`,
`a_changed_run_reports_one_surface_the_whole_run_reports_too`,
`legacy_unreached_debt_stays_a_note_in_a_turn_that_edits_another_file`,
`a_new_test_file_in_a_family_directory_is_no_member`,
`a_test_directory_under_a_family_root_stays_in_the_cohort_it_must_prove`,
`a_destructuring_in_another_file_that_binds_a_member_name_reaches_it` and
`a_plain_declaration_elsewhere_or_a_destructuring_in_the_same_file_reaches_no_member`
in `tests/reachability.rs`, and by
`a_caller_only_turn_judges_the_whole_family_off_the_shared_extraction` in
`tests/structural.rs`. Known limit: a destructuring inside a function body is
no declaration, so a name only it binds reaches no member. That is right for
`function boot() { const [Login] = list; }`, whose binding names a local
value, and wrong for an unused `const { default: Profile } = await import(…)`
inside a function, which leaves the file the import loads unreached. Second
known limit: a member's own destructuring declaration, such as
`export const { Profile } = factory;`, is judged by its pattern text, which
no reference spells, so a file that uses `Profile` does not reach it. Third
known limit: a declaration that a later statement exports as the default,
such as `function Reset() {}` with `export default Reset;` or
`export { Reset as default };`, keeps its own name, so a `default` re-export
does not reach it and `export { Reset } from "./m"` does.

**`doc-citations` reads backticked paths, not Markdown links.** On each line,
backticks pair from the left, and an unpaired trailing backtick opens
nothing. A span is a citation when, after trimming and dropping everything
from the first colon on, it holds no space and no `*`, it ends with one of the
built-in source, document and manifest extensions the reference prints, and it
holds a `/` or a `.`. So `` `src/a.rs:12` `` cites
`src/a.rs`, and `` `*.rs` ``, `` `a b.rs` `` and `[a](src/a.rs)` cite
nothing. A run's root is the tree root, and `--root` by hand names others.
Any citation resolves when a root holds a file at that path. A path with
a `/` resolves nowhere else. A bare filename no root holds directly
resolves when exactly one file under the roots has that basename, is
ambiguous when several do, and resolves nowhere when none does. Identity is
the document plus the cited path after trimming and the colon strip, so the same stale string on two lines is one site with `count` 2, and the
same string moved to another line is held. Pinned by
`a_span_that_is_not_a_path_is_not_read`,
`a_wildcard_a_link_and_a_line_suffix_are_read_as_the_syntax_says`,
`a_bare_filename_resolves_when_exactly_one_file_under_the_roots_has_that_name`,
`two_candidates_is_ambiguity_reported_with_both`,
`a_stale_citation_moved_to_another_line_is_held` and
`the_same_stale_string_cited_once_more_is_worsened_with_the_count` in
`tests/doc_citations.rs`. Known limit: a citation split across two lines, a
path in a Markdown link, and a path with a space are never read.

The check takes no scope: a changed run judges every derived root document of
both trees, because the edit that breaks a citation is a move, a rename or a
new basename clash in the source, and not an edit to the document that cites
it, so a run narrowed to the changed files would judge no document at all.
The derived root-document set is bounded by 5.2, and each document already
resolves against whole-tree path facts, so the whole judgement costs one
resolution per citation. The physical changed-file set is therefore not this
check's judgement boundary, and the wider boundary raises no old debt: a
citation broken in both trees stays held under the ordinary two-tree ratchet,
and never fails because a run re-judged it. One root-document set drives the
findings, the base and accepted matching, the coverage counts and the
moved-target remedy alike, so a changed run never reports a document that its
coverage says it did not measure. `--file` and `--root` by hand are
unchanged: a person naming a document still judges that document only. Pinned
by `under_changed_a_move_breaks_the_citation_of_a_document_the_window_did_not_touch`,
`the_stop_hook_blocks_on_a_move_that_breaks_an_untouched_documents_citation`,
`under_changed_a_stale_citation_of_an_untouched_document_stays_held`,
`under_changed_a_new_basename_clash_makes_an_untouched_citation_ambiguous`,
`under_changed_removing_a_basename_clash_leaves_an_untouched_citation_silent`
and `file_and_root_by_hand_judge_that_document_against_the_base` in
`tests/doc_citations.rs`.

**`complexity` measures each function on its own.** The unit is a function
node of the language's grammar, and an accessor or initializer body counts
as a function of its own. Cyclomatic complexity starts at one and adds one
for each decision node and each boolean operator in the language's tables,
walked through the function's body. A nested function is not walked: it is
excluded from the count of the function around it and measured as a site of
its own, at its own declaration line. A TypeScript or JavaScript callback
passed directly as an argument to a suite container call in a test file is not
a function site. The calls are direct `describe`, `context`, `suite`,
`fdescribe` and `xdescribe` calls; direct `.only` and `.skip` calls on
`describe`, `context` and `suite`; and curried `.each` calls on those same
three containers, in the form `describe.each(data)(name, callback)`. A callback
may be wrapped in parentheses or a TypeScript `as`, `satisfies`, non-null or
type assertion. Calling the value returned by `describe(...)` or
`describe.only(...)` does not make that call a suite container. The suite
callback is omitted from both trees' judged function population and from the
derived `cc` and `lines` sample.
Functions inside it remain measured, including `it` and `test` callbacks,
hooks and helpers. The same callback in a file that is not a test file is
measured as before. A test file is one that
5.4 marks under a test root or by a test directory segment or test affix. The
`default` arm of a Java or Swift
`switch`, the `else` arm of a Kotlin `when`, and a single unguarded
catch-all arm of a `match` or `case` add nothing. `lines` is the
count of source lines from the first line of the declaration to the last
line of its body, both inclusive, so a one-line function is 1.

Test code is judged on `cc` as production code is, and on length only
against `test_lines`. Test code is every function in a test file of 5.4, one
under a test root or one a test directory segment or a test affix marks, and
every function inside a Rust item marked `#[cfg(test)]`, such as an inline
test module or a lone helper function, helpers and fixtures included. Each
tree is classified over its own files, and a file the change renamed is
classified at the base under the path the base holds it at, so a test moved
into production code is new production code there. Pinned by
`a_function_marked_cfg_test_outside_a_module_is_test_code` and
`a_test_file_renamed_into_production_code_is_judged_as_production_code`. With
`test_lines` pinned, a function in test code over it is a finding, and
`lines` judges only the rest. A finding in test code carries its length as
`test_lines`, and one in production code as `lines`. A finding that moves
between the two carries a value its base entry does not, so it is
`worsened` (7.1). That holds for any such move of a site over its `cc`
ceiling, whichever length ceiling is stricter and even when its length is
under both. Pinned by
`a_test_over_cc_moved_into_production_code_is_worsened_with_both_ceilings_pinned`,
`production_code_over_cc_moved_into_a_test_is_worsened_with_both_ceilings_pinned`
and
`a_function_over_cc_that_changes_class_is_worsened_under_both_length_ceilings`.
With no `test_lines`, no function in test code fails on length, its finding
carries no length, and the `OK:` line says how many test functions were not
judged on length and names each file that holds one and that the change added
or renamed, since nothing else checks how long those tests are. The change
set says which files those are, so a file a wider scope brings in is not
named. Pinned by
`a_file_a_wider_scope_brings_in_is_not_named_as_added`. A failure names the ceilings in force, with `test_lines`
only where one is pinned. Pinned by
`with_no_test_lines_a_test_function_past_the_lines_ceiling_does_not_fail`,
`with_no_test_lines_a_test_function_past_the_cc_ceiling_still_fails`,
`with_test_lines_pinned_a_test_function_past_it_fails`,
`with_only_lines_pinned_a_production_function_is_judged_and_a_test_function_is_not`
and
`with_no_test_lines_the_coverage_line_says_test_code_was_not_judged_on_length`
in `tests/complexity.rs`.

The hand-checked numbers per language are the contract, in
`*_functions_carry_their_hand_checked_numbers` for Rust, Python, TypeScript,
Go, Java, Ruby, Swift and Kotlin, with
`a_nested_function_is_measured_on_its_own_not_folded_into_the_one_around_it`,
`a_fall_through_arm_is_not_a_decision`,
`a_guarded_catch_all_arm_is_still_a_decision` and
`an_accessor_or_initializer_body_is_measured_like_any_other_function`,
`a_growing_suite_callback_in_a_test_file_is_not_a_complexity_finding`,
`a_long_test_callback_inside_a_suite_is_still_measured`,
`suite_callbacks_do_not_raise_the_derived_lines_ceiling` and
`a_suite_callback_in_a_production_file_is_still_measured` in
`tests/complexity.rs`. Known limit: which grammar nodes are decisions is per
language and fixed in the binary. Two languages that express one construct
differently may count it differently, and the fixtures are the record of
which choice was made.

**`inventory` ratchets the existence of two things.** A test file is the
repository path of a file the base commit's tree listing holds that 5.4 calls
a test, and its `missing` is 1 when the working tree holds no file there. A
test function is a site of ADR 0008 inside such a file, in both trees, read
off each tree's file list and kept only where the
language's test convention marks it: a `fn`, `def` or `func` declaration
whose name starts with `test_`, a `func Test` declaration, an `it(` or a
`test(` call at the start of the declaration line, an `@Test` annotation on
the declaration line or on the run of marker lines directly above it, and a
Rust attribute whose path ends in the segment `test`, with or without
arguments, such as `#[test]`, `#[tokio::test]`,
`#[tokio::test(flavor = "multi_thread")]` or `#[async_std::test]`, among the
attributes and comments directly above the function. The Rust grammar reads
that attribute, so an attribute over several lines, or with whitespace or a
comment between its tokens, marks the same test. `#[rstest]`, `#[test_case(...)]`
and any other attribute whose last segment is not `test` mark nothing. The
same convention decides which functions `dead-symbols` leaves unjudged and
which empty body `stubs` calls an `empty test`. A marker an identifier runs
into matches nothing, so `myfunc Test` is not a declaration, and a call marker counts at
the start of the line only, so `def helper(test_arg): return it(test_arg)`
is not a test. Where the grammar holds the annotation inside the function's
own node, as it does for Java, the declaration line is the first line of
that node that carries more than an attribute or an annotation, so the site
names the method and the body hash of 4.4 leaves the name out. Its
`missing` is 1 when no function in the working tree takes it, by site first
and then by body hash, one to one on each pass. `in` and `except` narrow
both identities the same way, and both trees are read under the scope the
base commit records, and under today's when the base records none (8.6), so a
narrowing lets a deletion through only once it is committed. A file the working tree's grammar refuses holds no function site,
so the functions in it are not judged and the file is the unparsed refusal
of ADR 0003. A parser resource error MUST propagate as the named error of
section 13, including its file, line, measured byte count and ceiling; it is
not a grammar rejection. Pinned by
`deleting_a_test_function_from_a_file_that_stays_blocks_the_stop_and_asks_why`,
`a_deleted_test_function_is_a_note_that_fails_nothing_outside_the_hook`,
`the_stop_after_the_question_passes_and_leaves_a_green_verdict`,
`a_prompt_between_two_stops_does_not_ask_about_the_same_test_again`,
`a_stop_whose_stamp_was_deleted_still_asks_about_a_deleted_test`,
`a_test_function_renamed_and_moved_with_its_body_unchanged_is_held`,
`a_function_whose_name_only_holds_a_marker_is_not_a_test_site`,
`a_test_name_with_no_attribute_above_it_is_a_test_site`,
`deleting_a_tokio_test_from_a_file_that_stays_is_a_vanished_test_site`,
`an_in_that_names_one_file_judges_the_functions_in_it`,
`a_test_file_beside_its_source_is_judged_with_no_configuration`,
`an_except_added_only_in_the_working_tree_does_not_let_a_deletion_through` and
`a_test_file_no_grammar_reads_is_named_and_exits_two` in
`tests/inventory.rs`. Known limit: the convention table is fixed in the
binary, so a project whose tests carry another mark has no function
identity, and only its test files are ratcheted.

**`lockfile` reads the manifest and the lockfile beside it.** A site is the
manifest's repository path plus the dependency name, and it carries three
values, all higher is worse: `unlocked` is 1 when the lockfile holds no entry
for the name, `unpinned` is 1 when the manifest's specifier is a range or
absent, and `stale` is 1 when the manifest states an exact version, the
lockfile holds the name, and no version it records for this manifest is that
pin. A dependency that one table states as a range and another as an exact
version is `unpinned` and is still judged for staleness at its exact version.
A version is the pin when it equals it. A pin of one or two numbers, such as
Cargo's `=1.2` or npm's `1.2`, also holds every release that starts with it
and a dot, so `=1.2` holds `1.2.5` and `1.0.0-alpha` does not hold
`1.0.0-alpha.1`. A version that names a path, `link:` or `file:`, is a
workspace package with no registry release to compare, and it holds every pin.
A lockfile that records several versions of one name is stale only when none
of them is the pin. A range is never judged for staleness, because it has no
single version to compare. Exact means a Cargo requirement that
starts with `=`, an npm specifier that starts with a digit and holds no
operator and no wildcard segment, and every Go `require`, which states one
version. A path, git or
workspace dependency has no registry behind it and no version to pin, so it
carries 0 for every value and can never fail. The manifest tables read are
Cargo's `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]` and
`[workspace.dependencies]`, including the `[dependencies.name]` sub-table
form, the dotted-key form, an inline table on one line or several, and any
table whose last path segment is one of those names, so a target-specific
table is read too. A statement only ever takes a value away, so two tables
that name one dependency read the same in either order, and a `features` line
undoes no version another line stated. A Cargo `package` field gives the name
the lockfile is searched for, so a renamed dependency is found. A comment is
cut off before the line is read, so a brace inside one opens no inline table.
The npm tables are `dependencies`, `devDependencies`,
`optionalDependencies` and `peerDependencies`. Go reads both forms of the
`require` directive. A `replace` directive applies to the version it names,
or to every version when it names none. One that sends a module to a local
path gives it no registry, and one that sends it to another module or version
makes klin look up that module and judge that version, because `go.sum`
records the replacement. The lockfile is the nearest one at or
above the manifest's own directory, which is how a workspace member finds the
one lockfile its members share. `Cargo.lock` gives the `name` and `version` of
each `[[package]]` block, in either order, `package-lock.json` gives the keys of
`packages` with everything up to the last `node_modules/` stripped and the
keys of the nested `dependencies` tree that version 1 writes, and `go.sum`
gives the first field of each line, with the second field as its version once
a `/go.mod` suffix is cut off. So a Go module is stale when `go.sum` holds no
line for the version its `require` states. npm and pnpm resolve each name
for each manifest, so klin judges a manifest's pin against the entry for the
manifest's own directory, relative to the lockfile, and then against the
root's. npm writes that entry as `<directory>/node_modules/<name>`, or as a
top-level key of the version 1 `dependencies` tree, and a link takes the
version of the package it links. pnpm writes it under `importers`, or in the
top-level dependency tables of a lockfile that has no importers. The pnpm
`packages` mapping and the npm entries nested in another package's
`node_modules` count only toward `unlocked`, so a pinned name that the
lockfile holds only there is stale. Cargo and Yarn record no resolution per
manifest, and klin judges a pin against every version they record. A
dependency the base manifest did not name is a
site only when it is `unlocked` or `stale`, so a new dependency with a range
and a lockfile entry is not a finding, while a pin the base held and a
lockfile entry the base held are both `worsened` when they go. Staleness the
base already had is held, so a new one is `new` and one that grew is
`worsened`. `pnpm-lock.yaml` gives names from the direct
keys under its `packages` mapping: older versions use `/name/version` or
`/name@version`, and current versions use `name@version`, with a peer suffix
ignored after the version: the `(...)` of current versions and the `_...` of
version 5. The version in the key is the one it records.
`yarn.lock` v1 gives names from top-level selectors after
`# yarn lockfile v1`; Yarn 2 and later gives them from top-level locator keys
after `__metadata.version`. Both give the version from the `version` line
under each key. Selectors and locators are split at the
package-name separator, preserving scoped names. The line readers recognize
only those markers and key shapes; an unrecognized shape is a NOTE and no
finding rather than an empty lockfile. That NOTE is filed under the manifest
and names the lockfile. A lockfile only the base held unreadable is named at
its base path, followed by `at the base`. A JSON lockfile klin cannot parse
is a tool error naming the file. A manifest with no lockfile in either tree is a
NOTE and no finding, and a lockfile only the base held makes every
dependency of that manifest `unlocked`, so deleting a lockfile fails. A
manifest klin cannot parse now and that did not parse at the base is a NOTE
naming the manifest in every run, hook or not (8.6). It judges none of that
manifest's dependencies, and every other manifest is still judged, so a
fixture that is invalid on purpose does not turn the gate red. A manifest
klin cannot parse now that the base did not hold is a tool error outside the
hook, because the work added the hole: the error says to make it parse or to
add it to `except`. In the hook it is the same NOTE, because the agent cannot
edit `except`. Once the change is in the base, the manifest is a NOTE in
every run. A manifest that parsed at the base and does not parse now
is a tool error naming the file, because the work broke it and the agent can
fix it. A manifest that did not parse at the base and parses now is judged
against a base that named no dependency. Every manifest and lockfile is read
once per tree, the base's through one git process. The base judges a manifest
the window renamed at the path it had there, beside the lockfile it had there,
so a rename keeps its base. A manifest renamed from another format, such as
`Cargo.toml` to `package.json`, has no comparable base, because its base bytes
were written for another reader, so it is judged as one the base did not
hold. A lockfile several manifests share is parsed once. An accepted entry that gives no `stale` holds
a `stale` of 0, so an entry written before the value existed stays valid and
holds no staleness. The remedy has one part for each value that failed, in the
text and in each finding's `fix_advice`: every nonzero value of a new finding,
and each value that rose on a worsened one.
`unlocked` asks for the project's own install, so the lockfile records the
dependency, `unpinned` asks for the base's exact version back or an exact
version for a new dependency, and `stale` asks for an install again, so the
lockfile records the pinned version. The failure names a dependency that the
lockfile beside the manifest does not lock at an exact pin. Only the npm reader can
reject a manifest, because the Cargo and Go readers are line scans. The check
judges every manifest under `--changed` as well, because a lockfile change
judges a manifest whose own text did not change and the whole set is a
handful of files. Pinned by
`a_new_rust_dependency_with_no_lockfile_entry_fails_as_new`,
`a_rust_lockfile_entry_that_went_fails_as_worsened`,
`a_rust_pin_that_became_a_range_fails_as_worsened`,
`a_new_dependency_with_a_range_and_a_lockfile_entry_does_not_fail`,
`a_path_a_git_and_a_workspace_dependency_are_not_judged_for_unlocked`,
`a_deleted_lockfile_fails_every_dependency_of_its_manifest`,
`a_workspace_lockfile_above_the_member_manifest_is_found`,
`a_sub_table_and_a_target_table_are_read_like_any_dependency_table`,
`a_dotted_key_states_one_field_and_undoes_nothing_another_line_stated`,
`an_inline_table_written_over_several_lines_is_read_as_one_dependency`,
`a_brace_inside_a_comment_hides_no_dependency_below_it`,
`a_renamed_dependency_is_locked_by_the_package_the_lockfile_records`,
`both_npm_lockfile_versions_hold_a_dependency_in_the_base_state`,
`an_unrecognized_lockfile_format_is_a_note_and_judges_no_manifest`,
`pnpm_lockfile_key_styles_hold_and_missing_dependencies_fail`,
`both_yarn_lockfile_formats_hold_and_missing_dependencies_fail`,
`a_malformed_lockfile_is_a_tool_error_naming_the_file`,
`a_derived_manifest_klin_cannot_parse_is_a_note_and_every_other_manifest_is_judged`,
`a_derived_manifest_that_parsed_at_the_base_and_does_not_parse_now_is_a_tool_error`,
`a_derived_manifest_the_change_adds_and_klin_cannot_parse_is_a_tool_error_and_passes_the_hook`,
`a_derived_manifest_klin_cannot_parse_that_the_change_only_renamed_is_a_note`,
`a_renamed_manifest_is_judged_against_the_lockfile_beside_it_at_the_base`,
`a_lockfile_only_the_base_could_not_read_is_named_at_the_base`,
`a_manifest_renamed_to_another_format_has_no_base_to_hide_behind`,
`a_derived_manifest_that_did_not_parse_at_the_base_is_judged_once_it_parses`,
`a_manifest_klin_could_never_parse_is_a_note_and_no_tool_error`,
`two_manifests_that_share_one_lockfile_are_each_judged_against_it`,
`under_changed_a_changed_lockfile_with_an_unchanged_manifest_is_still_judged`,
`a_manifest_pin_the_lockfile_records_at_another_version_fails_as_worsened`,
`a_new_pin_the_lockfile_records_at_another_version_fails_as_new`,
`npm_judges_the_version_under_a_package_root_and_not_a_nested_one`,
`every_lockfile_reader_fails_a_pin_it_records_at_another_version`,
`a_range_with_a_lockfile_entry_at_any_version_passes`,
`staleness_the_base_already_had_is_held`,
`a_lockfile_with_several_versions_of_one_name_is_stale_only_when_none_equals_the_pin`,
`a_new_dependency_missing_from_the_lockfile_gets_a_remedy_that_names_the_install`,
`a_finding_with_several_values_prints_the_remedy_for_each`,
`a_pnpm_5_peer_suffix_is_no_part_of_the_name_or_the_version`,
`a_go_module_a_replace_sends_to_another_version_is_judged_at_that_version`,
`an_exact_pin_beside_a_range_of_the_same_dependency_is_still_judged_for_stale`,
`an_npm_pin_with_only_a_nested_lockfile_entry_is_stale`,
`an_npm_workspace_link_is_judged_at_the_version_of_the_package_it_links`,
`each_npm_workspace_manifest_is_judged_against_its_own_entry_and_then_the_root`,
`a_cargo_lock_block_is_read_whatever_the_order_of_its_fields`,
`only_a_pin_with_fewer_than_three_numbers_holds_the_versions_it_is_a_prefix_of`,
`a_go_replace_applies_only_to_the_version_it_names_and_judges_its_target`,
`a_pnpm_manifest_is_judged_against_its_own_importer_and_not_the_package_pool`,
`a_pnpm_5_root_dependency_section_is_its_importer`,
`a_worsened_finding_prints_only_the_remedy_of_the_value_that_rose`,
`a_stale_pin_fails_under_a_condition_that_names_it` and
`an_accepted_entry_that_gives_no_stale_holds_no_staleness` in
`tests/lockfile.rs`. Known limit: the Cargo and Go readers are line
scans, so a manifest that states a dependency in a shape the scan does not
know contributes no site rather than a wrong one. Known limit: a Yarn
lockfile records no resolution per manifest, so in a Yarn workspace a pin
passes when any workspace resolved the name to it. This can hide staleness,
and it cannot report staleness that is not there.

**`layering` judges resolved dependencies against a person's layers.** It is a
Policy check: with no section it does not run. The section holds `layers`, a
map of layer name to a layer's `in` and `can_use`, and the optional shared
`in`, `except` and `acyclic`. A layer's `in` is a path or a list of paths of
5.3, never a glob. `can_use` lists the layers a layer may depend on, `null`
lets it depend on every layer, and an absent `can_use` lets it depend only on
itself. A layer may always depend on itself. A `can_use` that names no layer,
a layer or shared `in` that holds no Rust or TypeScript file, and a file two
layers hold are config errors. `roots`, `languages`, `skip_dirs`, `name`,
globs, `allow_same` and package topology are refused.

The check builds the module graph of ADR 0043 for both trees from the
structural facts of 8.4 and each tree's own file list. A Rust target root
comes from a Cargo manifest's `lib` and `bin` targets, implicit ones included,
and, where no usable manifest sits above a file, from `src/lib.rs`,
`src/main.rs` or a file directly in `src/bin`. From each root the check follows
`mod` declarations to `name.rs` or `name/mod.rs`, and a literal `#[path]` from
the directory of the file or of the inline module that holds it. A file two
targets reach is a module of each. A dependency is a path a `use` tree or a
path outside an import writes from `crate`, `self` or `super`, resolved to the
deepest module it names. A path whose first segment names a module the same
file declares at that path's nesting, with `mod name;` or inline and directly
in a module rather than inside a block, resolves as if it started with
`self::`, so `pub use inner::X;` and `inner::f()` beside `mod inner;` depend on
`inner`. A module a block declares directly, a function body or a `const` or
`static` initializer alike, is an item of that block and is reached by no bare
path from outside it. A module that module declares is its child as usual. A
target's edition comes from its manifest, where Cargo's default is 2015, and a
conventional root is read as edition 2024. In a `use` tree of an edition 2015 target, a first segment
starts at the target root instead, and resolves only where the root declares a
module of that name. A Rust path that resolves to its own module
names that module's items and is no edge, so `use self::Kind::*` closes no
cycle. A path from another name may be another crate or a local item, so a
`use` tree that writes it is counted as external and not resolved, and a path
outside an import that writes it is not read. Every TypeScript file is a
module. A relative specifier resolves when exactly one of these files exists:
the specifier itself with a TypeScript extension, the `.ts` or `.tsx` file a
`.js` specifier stands for, or `.ts`, `.tsx`, `index.ts` or `index.tsx` after
it. A bare specifier with no recognized paths rule and a specifier that
names a file of another kind are counted as external. Explicit paths aliases
resolve or produce local incompleteness under the rule below. A `mod` declaration is containment and
never a dependency.

A module is its resolver's identity and holds one or more physical files, each
once and in no meaningful order (ADR 0058). A Rust or a TypeScript module holds
one; a resolver for a later language may group several. Structural facts stay
per file. Each file a resolver attaches counts on its own in the coverage
line, so a module of ten files is ten attached files. A dependency is a
*site*: the module that writes it, the module it reaches, and the exact file
and line that write it. A resolver or a surface derivation runs only over a
tree whose file list holds a path of its language: a Rust source or a
`Cargo.toml` for Rust, a TypeScript source or a `package.json` for
TypeScript. A source the grammar refuses still counts, so the resolver still
places it. The file list is scanned once while the topology is built.

Path policy is physical. Each file of a tree is placed once: whether the scope
selects it and which layer holds it. Each module is then folded once over its
files: its scope is inside where the scope selects every file, outside where it
selects none, and mixed otherwise; its layer is a layer where every file sits
in that one layer, none where no file sits in a layer, and mixed otherwise. A
dependency is judged where the scope selects the file that writes it and the
module it reaches is inside, by the layer of that file and the folded layer of
that module. A module that is mixed is never placed by one of its files. A
dependency on a module whose files straddle the scope is unresolved as below
and gets no verdict. A dependency from a file in a layer on a module whose
files straddle the layers is unresolved as below and gets no layer verdict,
but it is still judged for cycles, because only the scope decides that.

For every dependency the section judges, where the file that writes it and
the module it reaches sit in layers and the source layer may not use the
target layer, the edge is forbidden. With `acyclic` true, the strongly
connected components are found over modules, with every judged pair of
modules one edge however many sites write it. A judged dependency whose two
modules share a component is cyclic, and a module that imports itself is
cyclic. The check first judges *semantic edges*: the module that writes a
dependency, the module it reaches, and the edge's text, which names its kind,
the two layers for a forbidden edge, and the module it reaches by its file and
the inline modules after it. A module's semantic identity is its identity
under current paths and, for a Rust module, the kind and root of the target
that owns it, so a file two targets reach is a different module under each.
The working tree's semantic edges are paired with the base's before any
finding exists: a semantic edge the base holds is held wherever its sites now
sit, so evidence that moves between the files of one module, splits across
them or joins in one of them is held. Then each edge is reported where it is
written: one finding per file and text, which carries `edge` at 1 and is held
only where the base holds every semantic edge it merges. A line that starts to
reach another module, an inline module of the same file included, is new, and
so is a file two targets reach whose targets swap what each reaches. An
accepted entry is matched by the file and text a person wrote and never
follows a move, so under `--strict` an entry its edge moved away from is
stale. The check needs the commit
and not the runner's tree: it reads the whole base through the run's one
shared checkout, so a changed run lays out no partial tree for it. A new cyclic edge prints one shortest cycle through it,
which explains the finding and is no part of its key. Today's policy judges
both trees. The base places a file the window renamed under the path it had at
the base, and its finding names the current path, so a move into another layer
is new debt and a move inside a layer is held.

TypeScript explicit `compilerOptions.paths` aliases are local dependencies.
One conventional ancestor `tsconfig.json`, with no extends or references,
proves an exact or single-`*` alias only for a proven project root and when
its single target reduces to
exactly one held TypeScript module under the relative candidate rule. JSONC
comments, trailing commas and a UTF-8 BOM are supported. Exact rules precede
wildcards; the longest matching wildcard prefix wins. Equal-priority rules
are unresolved. Absolute specifiers stay outside V1 and are never remapped.
Targets are relative to a directly known `baseUrl`, or to the
config directory without one. The tree still holds every source module, but
ancestry alone does not prove that a source belongs to the alias config.
Root proof accepts an explicit `files` entry, regardless of `exclude`, or a
supported `include` match not filtered by `exclude`. Supported patterns are
literal file paths, directory paths, a directory followed by `/**/*`
(including `**/*` at the config root), and `**/*.ts` or `**/*.tsx`, alone or
after a directory. Paths are relative to the config. An unsupported include
entry adds no roots and leaves other include entries in force. An unsupported
exclude entry might remove any file, so it declines all include proof for
that config. Explicit files still prove roots. A malformed include/exclude
list declines include proof. With neither files nor include, the config directory is the default
include; with files but no include, include is empty. Dot paths, dependency
directories and outDir/declarationDir are conservatively outside include
proof, even when an explicit exclude list would admit them. Explicit files
can still prove these roots. An excluded or non-root file might enter a
TypeScript program through imports; V1 does not infer that reachability and
keeps its recognized aliases locally unresolved. Targets need only be held,
not roots themselves. V1 models configured roots, not direct-file compiler
invocations that ignore tsconfig.

Configs and rules are read once per tree; exact rules use a direct map and
wildcards are ordered once. Scope selection uses ancestor-directory map
lookups and root membership is selected once per source file, never per
import. A config-only edit can retarget an unchanged dependency or change
its root proof in a changed run.

A matching paths rule whose target cannot be proved produces located local
resolution incompleteness. One exception follows TypeScript, which tries
normal package lookup after the paths targets: when a rule's target names
no file the tree holds and the specifier is a valid package name, such as
`react` or `@scope/pkg`, the import is external. A specifier that cannot name
a package, such as `@/missing`, stays a hole. Multiple/nested or alternate tsconfigs, extends,
project references, multiple fallback targets and invalid anchors are not
used to guess edges. Held relative extends files are read once, with cycles
bounded, to recognize local names only. The nearest defined paths object
in a single local extends chain replaces the parent's entire object; an
empty or malformed object supplies no parent keys. Array-form extends
supplies no inherited names, though direct child paths remain recognized.
Package extends, unreadable configs
and bundler-specific configuration supply no unobserved names. Standalone
baseUrl lookup and Node/package resolution remain outside V1. Aliased targets
of another kind or ignored source retain the existing outside-V1 behavior.
Layering consumes every relevant local hole under the held/new semantics
below. Public-api consumes alias holes at re-export source sites only:
implementation imports alone do not prove a public contract incomplete.
Proven aliased re-exports traverse the shared graph via `reached_at`, which
names one import or re-export by its byte offset in the source, so two
statements on one line never share a target or a hole.
Pinned by `direct_typescript_aliases_close_cycles_and_forbidden_edges`,
`a_paths_mapping_retargets_an_unchanged_import_in_each_tree`,
`unsupported_local_paths_are_located_and_inherited_holes_stay_notes`,
`alias_targets_of_other_kinds_and_packages_stay_outside_the_graph`,
`nested_configs_and_ambiguous_alias_candidates_are_not_guessed`,
`jsonc_paths_use_exact_then_longest_wildcard_prefix`,
`local_extends_aliases_are_recognized_without_guessing_inheritance`,
`absolute_specifiers_are_not_reinterpreted_through_a_paths_wildcard`,
`sibling_typescript_configs_keep_each_files_aliases_and_rule_priority` and
`equally_specific_typescript_wildcards_remain_unproved`,
`a_paths_config_does_not_prove_aliases_for_a_file_outside_its_project_roots`,
`files_and_include_roots_do_not_turn_exclude_into_a_program_ban`,
`openstock_style_recursive_roots_prove_typescript_sources`,
`same_line_implementation_alias_hole_does_not_poison_external_re_export`,
`same_line_re_exports_follow_the_dependency_at_each_site`,
`same_line_alias_holes_stay_distinct_sites`,
`a_catch_all_paths_rule_without_a_local_target_leaves_package_imports_external`,
`imported_files_outside_project_roots_stay_unproved`,
`child_paths_replace_inherited_alias_names_instead_of_merging` and
`multiple_extends_do_not_supply_inherited_alias_names` and
`a_config_only_edit_changes_root_proof_for_an_unchanged_import`,
`output_and_dot_sources_need_explicit_files_for_root_proof` and
`local_extends_recognition_uses_the_nearest_paths_object` in
`tests/layering.rs`, and
`unresolved_implementation_aliases_do_not_poison_a_surface_but_re_exports_do`
and `a_proven_alias_re_export_measures_the_contract_behind_it` in
`tests/public_api.rs`. ADR 0043 and ADR 0044 amend the former paths exclusion.

A module that two files answer, a module no file answers, a path above the
crate root, and a TypeScript specifier with no candidate or with two are
unresolved. Where the scope selects the file that writes it, or where a manifest
writes it, each is a NOTE in the hook and exit 2 elsewhere, unless the base
holds it too, which is a NOTE in every run (8.6). A
file on disk that the file list leaves out, such as generated source git
ignores, is counted as external. The `OK:` line counts the dependency sites judged, the
files attached by a manifest and by a conventional root, the Rust files no
target reaches and the external dependencies. In a changed run that is not
strict, the working tree takes the base's facts for every unchanged file, as
`dead-symbols` does. Pinned by
`a_new_forbidden_dependency_fails_as_new`,
`a_forbidden_dependency_the_base_holds_is_held`,
`an_allowed_and_a_same_layer_dependency_pass`,
`can_use_null_lets_a_layer_use_every_layer`,
`a_file_in_two_layers_is_a_configuration_error_naming_both`,
`retired_topology_and_unknown_keys_are_refused`,
`a_super_path_inside_an_inline_module_resolves_from_that_module`,
`a_path_attribute_that_retargets_an_unchanged_file_is_new_debt_in_a_changed_run`,
`a_manifest_that_moves_the_library_root_changes_what_an_unchanged_file_reaches`,
`a_workspace_member_that_inherits_its_edition_is_attached_by_its_manifest`,
`a_file_two_targets_reach_is_judged_as_a_module_of_each`,
`a_module_two_files_answer_is_unresolved_by_hand_and_a_note_in_the_hook`,
`a_module_two_files_answered_at_the_base_too_is_a_note_by_hand_and_under_strict`,
`a_second_copy_of_a_form_the_base_could_not_resolve_is_new`,
`typescript_relative_imports_resolve_and_package_imports_are_counted_not_guessed`,
`two_typescript_files_one_specifier_names_are_unresolved`,
`a_new_cycle_fails_and_a_cycle_the_base_holds_is_held`,
`a_module_that_imports_itself_is_a_cycle`,
`a_rust_path_to_its_own_module_is_no_cycle`,
`a_retarget_to_an_inline_module_of_the_same_file_is_new`,
`a_typescript_re_export_is_a_dependency`,
`a_package_renamed_with_its_manifest_keeps_its_base_debt`,
`a_missing_target_root_a_manifest_names_is_unresolved_whatever_the_scope`,
`a_module_declaration_is_containment_and_not_a_dependency`,
`a_cycle_closed_through_a_bare_child_path_is_a_cycle`,
`a_cycle_the_base_held_through_a_bare_child_path_stays_held`,
`a_call_through_a_child_the_file_declares_is_a_dependency_on_it`,
`a_first_segment_that_names_no_declared_module_stays_external`,
`a_child_declared_inside_an_inline_module_is_not_reached_from_beside_it`,
`a_module_declared_inside_a_function_is_not_reached_by_a_bare_path_beside_it`,
`a_module_declared_inside_a_constant_initializer_is_not_reached_by_a_bare_path`,
`a_child_of_a_block_local_module_is_reached_from_that_module`,
`a_bare_use_path_in_edition_2015_starts_at_the_crate_root`,
`a_bare_use_path_from_edition_2018_starts_at_the_declared_child`,
`a_file_renamed_inside_its_layer_keeps_its_base_debt`,
`a_file_renamed_into_another_layer_is_placed_in_its_base_layer_at_the_base`,
`a_changed_run_beside_a_gate_that_lays_out_changed_files_judges_the_whole_base`,
`a_cached_changed_run_reads_and_parses_only_the_changed_file`,
`an_accepted_forbidden_edge_is_held`,
`a_tree_with_no_typescript_path_dispatches_only_the_rust_resolver`,
`rust_source_the_grammar_rejects_still_dispatches_the_rust_resolver`,
`typescript_source_the_grammar_rejects_still_dispatches_the_typescript_resolver`,
`a_file_two_targets_reach_that_swaps_what_each_target_reaches_is_new`,
`an_accepted_edge_does_not_follow_its_dependency_to_another_file` and
`without_a_section_the_gate_needs_one_a_person_writes` in
`tests/layering.rs`. A module of several files has no resolver yet, so the
unit tests
`multi_source_work_is_linear_in_files_modules_sites_and_unique_edges`,
`a_straddled_destination_is_ambiguous_and_never_judged`,
`a_site_is_its_file_and_line`,
`a_moved_semantic_edge_is_held_once` and
`a_semantic_edge_is_paired_before_its_findings_are_made` in
`src/layering.rs`, and
`a_module_of_many_files_attaches_each_file_and_names_each_site` in
`src/modules/mod.rs`, pin it over a graph built in memory. Known limit: a path
inside a macro's tokens, a bare Rust path that names no module its file
declares, a TypeScript `import()` or `require()`, standalone `baseUrl` lookup and package
exports are not dependencies in V1. A name a block or a function binds in the
type namespace, such as a local `use`, a local type alias or a generic type
parameter, shadows a declared child module of the same name in Rust, and klin
still reads a bare path through that name as a dependency on the child.

**`public-api` judges the consumer-facing contract a library or package
exposes.** Klin derives public API from standard Rust library and TypeScript
package entry points. You normally configure nothing: the section is absent,
or `false` to exclude the gate, and any object under it is a config error,
because surfaces, roots, languages and entry points are facts of the tree. It
is an Automatic check that needs the commit and takes no scope: a changed run
judges every file of both trees, because a manifest, an entry point or a
re-export can change what an unchanged file means to a consumer. It reads the
structural facts of 8.4 and the module graph of ADR 0043 for each tree under
that tree's own topology, and it builds no name index, no second resolver and
no parser of its own.

A *surface* is what a consumer addresses. For Rust it is a Cargo library
target, named by its crate name and matched between trees by that name and
its package manifest path. Two libraries with the same crate name retain
separate surfaces: unchanged items pass, and a removal or contract change is
judged only against the library that owned it. Adding or removing a namesake
library does not change the identity of an existing surface. Moving the
library root within its package keeps the surface's identity. Pinned by
`libraries_with_the_same_crate_name_keep_their_own_items`,
`a_namesake_library_does_not_hide_a_contract_change_or_removal` and
`removing_a_namesake_library_reports_only_its_surface`.
A binary target and a Rust directory no
manifest names are not surfaces. Implicit and custom library roots and every
library package of a workspace are found the way ADR 0043 finds targets. For
TypeScript a surface begins only at explicit package metadata that names a
checked-in TypeScript, TSX or declaration source file: an `exports` string,
each `exports` subpath whose target reduces to exactly one such file across
its conditions, and, without `exports`, the first of `types`, `typings`,
`main` and `module` that names one. Generated JavaScript is never mapped back
to source, `src/index.ts` is never guessed, and a package none of whose
entries names a supported source is not applicable and is said so on a
`NOTE:` line, not a hole. Publication metadata does not decide a surface: a
Cargo package with `publish = false` and an npm package with
`"private": true` keep their surfaces, because a workspace sibling, a path
dependency or a git dependency can still consume them (ADR 0050).

An *item* is what a consumer names under a surface, and its identity is the
surface, the exported path or name and the item's kind, never the file that
declares it. In this check, *external* means outside the crate or package
that declares the item, including a sibling in the same repository, and never
means published. The check judges only external items. From a Rust root the
check follows every plain `pub` declaration, every `pub mod`, every `pub use`
leaf and every `pub extern crate`, which re-exports the crate under its alias:
an alias renames the item, a glob exposes every public item of the module it
reaches less every name the globbing module binds itself at any visibility
in the namespace the name lives in (an item it declares, a child module or an
`extern crate` alias as a type, and a name a `use` outside a function binds
in both), a name a module neither declares nor re-exports by name comes from the one glob of it
that provides the name and is a hole where two do, a re-export of a module
exposes everything under it, and a plain
`pub` item inside a private module is external only where a `pub use` exposes
it.
`pub(crate)`, `pub(super)`, `pub(self)` and `pub(in ...)` are never external.
A Rust module already on the active surface traversal is a coverage hole:
its cyclic module re-export gives unbounded public paths. A named re-export
already being resolved is also a hole, because its cyclic name cannot be
resolved. A module-cycle hole names the file, line and source text of the
re-export that closes the cycle. Both end in a report rather than an abort;
separate finite aliases of one module are still followed independently.
Pinned by `a_module_re_export_cycle_is_a_named_hole_instead_of_unbounded_paths`
and `a_named_re_export_cycle_is_a_named_hole_instead_of_recursing_forever`;
`a_module_cycle_names_the_export_that_closes_it` pins the closing source site.
A re-export whose path starts, with or without a leading `::`, at a name of
the crate's extern prelude that reaches a library target the tree holds is
followed into that library's source, globs included, and each item it
reaches is measured there under the surface that re-exports it. Those names
are each normal dependency the target's manifest takes by path, target-specific
ones included, under its rename where the manifest writes `package =` and
under the library's own crate name otherwise, and each alias an
`extern crate`, public or private, at the top of the crate root gives one of
those dependencies, as `pub extern crate wgpu_types as wgt;` does. The path
names the manifest of the library, so a crate name two libraries of the tree share
reaches the one the dependency names. A dependency without a path, such as a
registry version, names a crate the tree does not hold even where a library of
the tree has its name, and so does a name a `use` binds, which never enters
the extern prelude. A first segment the module binds itself, by a `use`
outside a function, as a type it declares or, below the crate root, as an
`extern crate` alias, names a local item, so the path is opaque even where a
dependency has that name, and so is an alias the crate root gives a crate
that is no dependency of its manifest. An associated item of an `impl` or a
`trait` is no item of the module, so it hides no name. So an item moved into a sibling crate and
re-exported under its old name keeps its identity: an unchanged contract
passes and a changed one fails as changed. A re-export of a whole crate root,
such as that `pub extern crate` itself, is an opaque item, because the items
of a library the tree holds are judged under its own surface. Pinned by
`an_item_moved_into_a_workspace_sibling_and_re_exported_under_its_name_passes`,
`an_item_moved_into_a_workspace_sibling_with_a_changed_contract_fails_as_changed`,
`a_re_export_through_a_pub_extern_crate_alias_of_a_sibling_is_judged_the_same_way`,
`a_name_a_sibling_provides_through_a_glob_is_measured_where_the_glob_reaches`,
`a_re_export_after_a_leading_path_separator_reaches_an_extern_prelude_name_and_no_use_alias`,
`a_crate_the_manifest_takes_from_a_registry_stays_opaque_though_the_tree_holds_its_name`,
`a_crate_name_two_libraries_of_the_tree_share_reaches_the_one_the_manifest_names`,
`a_dependency_the_manifest_renames_is_followed_under_its_new_name`,
`a_dependency_inherited_from_the_workspace_is_followed_from_the_workspace_path`,
`a_dependency_inherited_from_a_workspace_below_the_tree_root_is_followed`,
`a_dependency_a_package_inherits_from_its_own_workspace_is_followed`,
`a_target_specific_path_dependency_is_followed`,
`a_private_extern_crate_alias_of_a_sibling_is_followed`,
`a_name_two_globs_provide_is_a_hole_where_a_re_export_names_it`,
`a_re_export_of_a_crate_the_tree_does_not_hold_stays_opaque`,
`a_local_use_named_like_a_dependency_keeps_its_path_opaque_though_its_item_changes`,
`a_private_item_or_import_hides_the_name_a_glob_of_a_sibling_provides`,
`an_extern_crate_alias_named_like_a_dependency_keeps_its_path_opaque`,
`only_a_module_level_binding_in_the_same_namespace_hides_a_name_a_glob_provides`,
`a_use_inside_a_function_named_like_a_dependency_leaves_the_dependency_followed` and
`a_glob_of_a_workspace_sibling_lists_its_items`.
A public inherent method, associated constant or associated type is an item
under its type, pinned by `an_item_of_an_inherent_impl_is_an_item_under_its_type`.
From a TypeScript entry
file the check follows exported declarations and namespaces, default exports,
local export clauses, and named, aliased,
type-only and star re-exports through the module graph's own edges. An
exported file no entry reaches is not package API. TSX is TypeScript.

An item is *measured* where its declared contract is canonical, and *opaque*
where klin proves it exists and no more. The canonical contract is written by
the language's structural adapter and never by the check: it drops bodies,
initializers, comments and decorators, one space stands between tokens, a
private member leaves except a private constructor, which stops a consumer
constructing the class, a private tuple position becomes `_`, and a binding
name that is not contract becomes `_`. The contract is complete: it shows
every fact whose own change can fail an item, so the `was` and `now` lines of
a failure always differ, and a construct the adapter cannot canonicalize is
opaque, never guessed (ADR 0054). A Rust type or variant with a directly
written `#[non_exhaustive]` renders it, and every other attribute leaves,
`#[cfg_attr(...)]` included. A Rust struct's private named field leaves and
its field list ends in `..`, whether or not the field sits under `#[cfg]`. A
Rust trait method with a default body renders `{ .. }` where one without
renders `;`, and a trait's associated `const` with a default renders `= ..`.
A TypeScript overload set keeps the source order of its
signatures inside one file, the groups of different files are ordered by their
text, so a renamed file never changes a contract, and an implementation
signature that follows overload signatures leaves the set. Inside a class or
interface body the overloads of one method, call signature or construct
signature likewise stay together in source order and lose their
implementation, though the public and protected properties a constructor
implementation declares through its parameters stay as members, and the
members keep one order whatever order the source wrote them in. An object
type literal keeps every member in source order, its overloads included, so
reordering any of its members fails. A `this` parameter shows as `this` with
its type. A TypeScript
parameter with a default carries `?` where no required parameter follows it,
and `= ..` where one does, because a caller passes `undefined` to reach that
default. A default on a constructor parameter that declares a property is
always `= ..`, because that property is never `undefined` the way an optional
one may be. Its initializer never shows. So
`#[non_exhaustive]` added to a type or a variant, a private field added to a
struct whose fields were all public, a default body or a default `const`
removed and a change that only reorders a TypeScript overload set each fail.
The `cfg` declarations of one Rust item are ordered by their text, so
reordering them passes. Rust covers
functions with qualifiers, generics, receiver and parameter types, return
type and `where` clause; structs, unions, enums with their variants, fields
and explicit discriminants; traits with their supertraits and associated-item
signatures; type aliases; and `const` and `static`
with their type alone. TypeScript covers functions and overload sets, classes
with their heritage and public and protected members, interfaces, type
aliases, enums and variables. A type the compiler would infer is written as
`?`, so an inferred contract is visibly partial and never fabricated from a
body. A re-export of a crate the tree does not hold or of another package, an
enum variant re-exported by path, a `* as ns` export and an anonymous default
export are opaque, and the
normalized clause that exposes them is the contract klin compares. A
TypeScript namespace an `export` statement declares, written `namespace`, or
`module` with a name that is no string, `declare` or not, is the item
`NAME (namespace)`, where `NAME` is the first name a dotted name such as `A.B`
writes, because that is the name the declaration binds. It is opaque, and its
clause is the contract klin compares: `declare` where written, unless a
declaration file or an ambient namespace around it makes the namespace ambient
already, `namespace` and its name as written, and in braces each member a
consumer can see, spelled by the rules above, so a body and an initializer
leave and an inferred type is `?`. A namespace is ambient where `declare`
makes it so, where an ambient namespace holds it, and in a declaration file: a
file whose name ends in `.d.ts`, `.d.mts` or `.d.cts`, or a `.ts` file whose
name holds `.d.`. So `declare` added in a declaration file passes. The body
of an ambient namespace that holds no export clause and no export assignment
exports every declaration it holds except an import alias written without
`export`. Any other body exports only what it writes `export` on and the names
its export clauses list. A name a clause lists is spelled as the member of the
body that binds it, or as the name itself where no member binds it, followed,
where the clause exposes it under another name or as a type only, by `as`,
`type` where `export type` or a `type` before the name makes it type-only, and
the name a consumer reaches it by. A variable that destructures binds each
name its pattern binds, never a default value or a computed key, and is
spelled whole for each of them, with each default its pattern writes shown as
`= ..`, because whether a default is there can change the type of the name it
binds. Its value never shows. The export clause itself is never spelled,
so reordering one passes, and adding `export {}` fails as changed only where it
hides a member. A statement that declares nothing leaves, a nested namespace is
spelled the same way, an import alias is spelled as written, the overloads of
one function stay together in source
order and lose their implementation, and the members keep one order whatever
order the source wrote them in. So a new namespace is a new item and passes, a
namespace the base exposed that is gone fails as removed, one whose clause
changed fails as changed, and an edit to a function body, an initializer or a
member the namespace does not export passes. A dotted name stays as written, so
rewriting `namespace A.B` as a namespace `B` inside `A` fails as changed. A
namespace declared without `export` and exposed by a later export clause or a
default export is the opaque item that clause exposes, judged by the clause,
except where the file also declares a function, class or other declaration of
that name: the clause then exposes that declaration alone, and the members of
the namespace go unjudged. A namespace an `export` statement declares hides the
name a star export provides. Pinned by
`a_new_exported_declare_namespace_in_an_entry_file_is_an_item_and_passes`,
`an_exported_namespace_the_working_tree_lacks_fails_as_removed`,
`an_exported_namespace_whose_declaration_changed_fails_as_changed`,
`a_plain_namespace_shows_what_it_exports_and_an_edit_no_consumer_sees_passes`,
`a_declare_namespace_shows_every_member_it_holds_and_dropping_declare_fails`,
`an_export_clause_added_to_a_declare_namespace_fails_as_changed`,
`under_an_export_clause_a_member_without_export_is_hidden_and_dropping_export_fails`,
`a_name_an_export_clause_lists_shows_under_the_name_the_clause_gives_it`,
`a_name_a_destructuring_declaration_binds_shows_as_that_declaration_where_a_clause_lists_it`,
`a_namespace_in_a_declaration_file_is_ambient_and_shows_every_member`,
`a_member_edit_inside_a_nested_namespace_or_module_fails_as_changed`,
`a_module_with_a_name_is_a_namespace_and_a_dotted_name_is_its_first_name`,
`a_dotted_name_stays_as_written_so_nesting_it_fails_as_changed`,
`a_namespace_a_later_clause_exports_is_the_opaque_item_of_that_clause`,
`a_clause_that_exports_a_function_and_a_namespace_of_one_name_exposes_the_function_alone`
and `an_exported_namespace_hides_the_name_a_star_export_provides`.

Base and working tree are derived independently. A base surface the working
tree lacks fails once, at the surface. For every item of a surface both hold,
an item gone fails, a measured contract that changed or is no longer declared
fails, an opaque clause that changed fails, and everything else passes: a new
surface, a new item, a widened visibility, an opaque item that became
measured. Each break carries `break` at 1 with the surface as its file and
`NAME (KIND)` as its text, so an intentional break is an accepted entry under
that identity, and the base holds no break by construction. For a Cargo library, the
finding's `file` is always `CRATE (MANIFEST)`, such as
`shared (a/Cargo.toml)`, while the consumer name in `--report` stays `shared`.
The owning manifest is part of accepted-entry matching, JSON site identity,
removed-module grouping and unresolved-hole evidence even when the crate
name is unique in either tree. An accepted break for one library cannot hold
the same item break in a namesake, and an unqualified crate-name entry for a
Cargo library matches nothing and follows the normal stale-entry rules;
klin never rewrites accepted entries.
Pinned by `accepting_a_namesake_break_cannot_hold_another_librarys_break`,
`identical_breaks_in_namesake_libraries_have_distinct_json_ids`,
`an_unqualified_rust_surface_acceptance_is_stale` and
`a_removed_module_of_two_surfaces_with_one_name_prints_each_item_once`.
Where one change removes a module and the items inside it, the text report
prints them as one group, the module's line and then the lines of the items it held, while the
11.2 object, the journal, the identities and the accepted entries keep one
finding per item. Pinned by
`a_removed_module_prints_as_one_group_and_json_keeps_each_item`. The remedy
keeps the base's contract only where the task allows it, says that for a
changed contract a new item beside the unchanged one keeps the base's contract
where that serves the task, proposes no name or design, and tells the agent
not to change what the task asked for only to satisfy the gate. In the hook
it adds that a stop blocked on a break the task intends is answered in the reply and
followed by another stop, which the block policy of 9.3 may let end, and
that the reply accepts nothing: a person accepts the break with an accepted
entry in a reviewed commit, and CI refuses it until then. Every stop prints
the same wording, so a stop that passes does not claim it blocked, and the
wording promises no end, because a build failure still blocks under 9.3.
Outside the hook the remedy names only the person's accepted entry and CI,
and no second stop. Pinned by
`a_break_in_the_hook_names_the_intended_change_route_and_leaves_acceptance_to_a_person`
and `a_break_by_hand_names_person_acceptance_and_no_second_stop`. A glob over
a crate the tree does not hold, a star export of another package, a name two
globs or two stars provide, an export form klin recognizes and cannot list,
such as TypeScript's `export =` or an exported `declare module` whose name is a
string, a
path through a module no file answers, and an unresolved module or specifier
inside a surface are holes: a `NOTE:` in the hook and exit 2 elsewhere, while
other findings still print, because a green run must not imply a surface it
claims to support was completely measured. A hole the base holds too is a
NOTE in every run (8.6). Unresolved-hole matching includes the same
owner-qualified Cargo surface identity as compatibility findings: a form
that moves from one namesake surface to another is new at its destination,
even when its source file, text and underlying reason are unchanged. Pinned
by `an_unresolved_form_moved_to_a_namesake_surface_is_new` and
`export_equals_and_an_ambient_module_are_still_holes`. The `OK:` line counts
the items and surfaces judged, how many are measured and opaque, the library
targets and entry points found, and the packages or targets with no supported
surface.
`klin public-api --report` prints the working tree's derived contract without
judging it: each surface with its discovery source, each item with its
identity, kind, origin, measured or opaque status and canonical signature,
each hole, and each package or target not applicable. A language's surface
derivation runs only over a tree that holds a path of that language, by the
same rule as its resolver in `layering`. Pinned by every test in
`tests/public_api.rs`. Known limits: a module bound by `use` and then
re-exported by its bare name, a macro, a trait implementation's semantics,
`cfg` evaluation, an attribute written through `#[cfg_attr(...)]`, a
registry dependency that `[patch]` or `[replace]` points into the tree,
`extern crate self as` an alias, a re-export by name through a glob of a
module that binds the name in either namespace, which is opaque,
`typesVersions`, conditional exports that do not reduce to one source file,
the top-level names a declaration file exports without writing `export`,
`tsconfig` paths and a package alias are outside V1, a declaration file
renamed to a name that is none, such as `index.d.ts` to `index.ts`, is read
at the base under its new name (8.2.1), so a member only the declaration file
exported goes unjudged, a declaration no surface
exposes is not judged, even where an exposed contract names it, and a generic
parameter renamed is a changed contract.

None of these rules asks another implementation to agree with klin. They
state what klin's own tests hold, per ADR 0025, so a change to one is a
change to the spec and to a test in the same commit.

#### B.8.3 The linter seam

The seam ships in two steps. The first needs no second tree and fits the hook
budget. The second is deferred until a user asks for the cases it catches.

**Step one, the `after` tree only.** A `sarif` entry names a command that
writes a SARIF report from the working tree, or a report that is already
there:

```json
"sarif": [
  { "name": "eslint", "run": "npx eslint -f sarif -o out/eslint.sarif .", "report": "out/eslint.sarif" },
  { "name": "semgrep", "report": "out/semgrep.sarif", "differential": true }
]
```

With `run`, klin deletes `report`, executes the command in the working tree the
way it runs `build`, under the same limit and deadline (9.3), then reads
`report`. A report that is missing after `run` is ERR, and so is a command
still running at the limit or the deadline, named with the one it reached. The
report path SHOULD be under `.gitignore`, so the stamp of 6.5 does not carry
it. The report is fresh by construction, because the only file at that path is
one the tool wrote over the tree klin is about to judge. The command's exit
status is not judged, because a linter exits non-zero when it finds something.
In the hook this is the RECOMMENDED form. Without `run`, klin reads the report
as it finds it, and that form belongs in CI, where the same job wrote the
report one step earlier. There, a report older than any file the window changed
is ERR, because a report that predates the change cannot describe it. Executing
the tool in the `after` tree has no dependency problem: the working tree has
its dependencies installed, or the build would fail first.

Each entry of the section is its own gate, named by its own `name`, because
nothing in a tree says which scanner an entry runs. The section MUST be a list,
and an entry with no `name` is a config error naming the key. A `sarif` gate
reads the base commit and never the base tree, so it runs one scanner over the
working tree and no `before` worktree is laid out for it.

Each result is keyed by file, rule id and message. A result fails when its
line falls inside a hunk the window changed. A result in a file the tree does
not track is one whole changed range, so every result in it is judged.

**Where a result sits.** A result's `physicalLocation.artifactLocation.uri`
reaches klin in four forms, because every scanner writes a location its own
way: a path relative to the repository, an absolute path, a `file://` URI, and
a path under an entry of `originalUriBaseIds`, which may name a further entry
in turn. klin percent-decodes each uri, drops a `file://` scheme, resolves a
base id through at most four entries, strips the tree's own directory off an
absolute path, following the real directories both name so that a tree reached
by a symlink still places, and resolves the `.` and `..` segments of what is
left. A location that lands outside the tree, in any of the four forms, and a
result with no physical location at all, is a NOTE naming the uri and is not
judged, because a path klin cannot resolve says nothing about which lines
changed. A location with no `region.startLine` starts at line 1, which is the
line a result about a whole file is judged on.

Several results of one rule in one file are one finding with a `count`, so an
accepted entry holds that site at the count a person accepted. An accepted
entry that matches nothing is a NOTE, and exit 1 under `--strict`, as for
every other gate (10).

A report klin did not write is judged fresh by modification time: klin
compares the report file's time against the time of each file the window
changed, and a report older than one of them is ERR.

A result on a line the window did not change is held, whatever the base held
there, and the gate's `OK:` line says how many results it judged and how many
it held. With `differential: true` the tool already reports only what is new,
so every result fails and that line says instead that it judged every result
the scanner reported. Scoping to changed lines, not changed files, is what
keeps an agent that touches a file with thirty old warnings green. A reformat
that moves every line is the known weakness, and the radius report already
names such a turn.

This step runs the tool once, needs no `before` worktree, and delivers most
of what the goal names: eslint, tsc, clippy and ruff findings become klin
failures exactly where the agent wrote the line. The same `run` and `report`
contract is the one a coverage reader or a test-result reader takes later.

**Step two, `run` in both trees.** An entry marked `compare: true` is
executed in both trees and its results are matched by site and ratcheted on
`count`. This catches a rule count that rose on an unchanged line, and a
finding that moved. It is deferred for a reason the review stated: the
`before` worktree has no installed dependencies, and a symlink of today's
dependencies runs today's tool over old source, which is not a pure function
of the `before` commit and breaks section 12. A design for step two MUST
solve that before it is written. A tool that compiles the tree, such as
clippy or `tsc`, is out of scope for `compare`: a bare `before` worktree
would fetch dependencies, which section 12 forbids, and a build is outside
the budget of section 13. The first candidate is a tool that is one binary
and reads no project dependencies, such as ruff.

#### B.8.4 Tier 2: build when tier 1 is green

`conventions` (#42, ADR 0037), `public-api` over the module graph and a
derived public surface (#46, ADR 0044),
`reachability` over one reference extractor (#49, #51), `layering` over one
module graph (#50, ADR 0043), `changed-coverage`
and `crap` over one coverage reader with a postflight run (#53, #54, #55,
#70), `hotspots` as a report (#60), SARIF output (#65).

The reference extractor is the `syntax` module (#49, ADR 0035). It owns the
grammars, the parser and the file no grammar read, and it hands a check
declarations, imports, module declarations and references, so no check holds
another language's node kinds. It resolves a reference by name to every
declaration of that name under the roots. That errs toward "referenced", so
`reachability` and other structural checks fail less, never more.

That conservative answer is evidence for judging a tree and not for writing
policy over it. `reachability` judges a file reached through an ambiguous
name, and derives a family only from members proven reached through a name
one declaration holds, per 5.4. `dead-symbols` and `reachability` are
shipped over this extractor; a third structural language is an adapter in
`syntax`, and neither check branches on a language.

Rust and TypeScript are the first structural languages, and TSX is TypeScript
rather than a language of its own. A file in a language no structural adapter
reads is counted as not measured on the coverage line of 8.6, so a green run
over such a tree is visibly a run over nothing. The module graph of ADR 0043
resolves an import or a Rust `mod foo;` to a file. The extractor keeps each
specifier as written for it, with the inline modules that hold an import, a
module declaration or a qualified path, every leaf path of a Rust use tree, and
every path outside an import that starts at `crate`, `self`, `super` or the
name of a module the file declares outside a block at that path's nesting.

A `test-hygiene` check, a count of habits across the test roots against a
dated ceiling, was considered and is not a check. A habit that rose is an
escapes `patterns` row with `roots` set to the test roots, and a schedule on
a whole-tree total fails a tree nobody changed, which is the forced paydown
7.3 refuses.

**`conventions` judges the project's own rules.** A convention names one
thing the project forbids, where it applies, and what to do instead:

```json
{
  "conventions": {
    "single-git-boundary": {
      "code": "Command::new(\"git\")",
      "except": ["src/project/git.rs", "tests"],
      "remedy": "Use the shared Git boundary."
    },
    "no-old-flags": {
      "text": "config::Flags",
      "remedy": "Use the explicit execution context."
    },
    "no-scratch-files": {
      "files": "**/scratch.*",
      "remedy": "Remove temporary scratch files."
    }
  }
}
```

The section is a flat object keyed by the convention's name, and the name is
the convention's identity. A convention MUST state `remedy` and exactly one of
`text`, `code` and `files`. It MAY state `in` and `except`, and a `code`
convention MAY state `language`. Any other key is a config error that names
the convention and the key. There is no regex and no example.

- `text` is literal text, matched line by line. No character in it has a
  special meaning. A comment or a string that holds the text is a match. A
  file that holds a NUL byte is not text and is not read.
- `code` is a code pattern. `$NAME` holds one piece of code and `$$$ARGS`
  holds a list. It matches code, so a comment or a string that reads like the
  pattern is not a match. Its languages are `rust` and `typescript`, and a
  `.tsx` file is TypeScript.

A `code` pattern MAY be a fragment: the piece of code a person means, with
nothing written around it. klin reads the fragment in each place its language
lists. For Rust, those places are code as written, an expression, a match
arm, a type and a field. For TypeScript, they are code as written and a type.
A reading counts when the grammar reads the fragment there with no error and
no token it had to supply, and one node is the whole fragment. A file matches
through every reading, and klin never chooses among them. A node that two
readings match is one match. A pattern that is only a hole is not a reading,
because it would match every node. A pattern that no place reads is a config
error that names every place klin tried.

| `code` | Reads as | Matches | Does not match |
|---|---|---|---|
| `_ => Ok(0)` | a match arm | `_ => Ok(0),` and `_=>Ok(0)` | `_ => Ok(1)`, the text in a comment |
| `RefCell<$T>` | a type | `records: RefCell<Records>`, `Rc<RefCell<Cache>>` | `Cell<u8>`, the text in a string |
| `Command::new("git")` | an expression | `Command::new("git").arg("status")` | `std::process::Command::new("git")` |
| `Array<Foo>` | code as written, a type | `let a: Array<Foo>`, `const n = Array<Foo>` | `Array<Bar>` |

A reading matches the code as the grammar reads it, so a path written in full
is a different piece of code from a path written short, and an element with
one more attribute is a different element. A name such as `config::Flags`
inside `use crate::config::Flags;` is part of a longer path, so a `text`
convention says it better.
- `files` is a glob over repository-relative paths. `**/` crosses any number
  of directories, `*` and `?` stay inside one, and `[...]` is a class. It
  reads no file.

`in` and `except` each take a repository-relative path or a non-empty list of
them. A path names itself and everything below it, and it is never a glob:
`src/syntax` holds `src/syntax/mod.rs`, and `src/main.rs` holds only itself.
With no `in`, a convention applies to the whole repository, and `except`
takes paths out of that scope. An absolute path, a path outside the
repository and a glob are config errors. An `in` path the working tree holds
nothing at leaves its convention measuring nothing there, so it is exit 2, and
a NOTE in the hook. An `except` path the working tree holds nothing at is a
NOTE.

A convention reads what the shared walk reaches. It never reads the default
skip set, a hidden directory, or what git ignores. It never reads the
configuration file either, because that file states every literal a convention
forbids.

A `code` convention with no `language` takes the one language that the files
in its scope are written in. Two languages, or none, is a config error that
lists them and asks for `language` or a narrower `in`. Which grammar reads the
pattern is never evidence of the language. A pattern that no grammar of its
language can read is a config error. A pattern that one grammar of the
language reads and another does not matches nothing in the files the other
reads: a JSX pattern matches no `.ts` file. A pattern that matches nothing in
the tree is a valid convention.

Each convention is judged as its own gate, `conventions/<name>`, through the
ratchet of section 7 and the accepted list of 4.8. Two conventions on one line
are two findings, each with its own remedy and its own accepted entry. A
`text` or `code` site is the file and the text of the line the match starts
on, and `count`, the matches at that site, is the ratcheted value. A `files`
site is the path, at line 0 and a count of 1.

A site that moves to another line of its file is held, and a site in a file
git renamed keeps its key, as 4.4 has it. A site that moves or is copied to
another file is `new`. No body hash pairs a site across files. The scope and a
`files` glob read the path each tree holds, so a file renamed out of `except`
brings its sites in as `new`, and a path renamed into a `files` glob is `new`.

An accepted entry whose gate names a convention the section defines no longer
is a config error. When the section is absent or `false`, no convention gate
runs, and its entries wait for it.

A failure names the convention and prints its `remedy` as the fix. A JSON
finding carries `convention`, `logical_gate`, `matcher`, `count` and `remedy`
in its values, and `language` for a `code` convention (11.2).

`klin conventions --report` reads the configuration and writes nothing. It
prints a summary with one row per convention. `--report <name>` explains one
convention.

```text
3 conventions: 2 new sites, 1 can't run

exhaustive-cli-dispatch    1 new                      src/main.rs:4
no-bad                     Can't read the pattern.
no-records-side-channel    1 new                      src/main.rs:7

For details, run: klin conventions --report <name>
```

The first line counts the conventions, the new sites, the sites that got
worse, and the conventions that cannot run. Each row names the first thing to
act on, in this order:

1. why the convention cannot run
2. its new and worse sites, with the place of the first one
3. an `in` path that matches nothing
4. the files klin cannot parse
5. its held and accepted sites
6. `Clear`, when it matches nothing

```text
window: branch — base ead8a4f, the merge-base with main

exhaustive-cli-dispatch

Forbids _ => Ok(0) in src/main.rs (1 file).
Reads as a match arm in Rust. The language is derived from the files in scope.

  src/main.rs:4   _=>Ok(0),   New

Fix: Handle every CLI command explicitly.
```

The detail prints the window line of 4.2 first. Then it prints:

- what the convention forbids, where, and how many files it reads
- for a `code` convention, what the pattern reads as, and whether the
  language is derived or pinned
- each `in` or `except` path that matches nothing, and each file klin cannot
  parse
- why the convention cannot run, when it cannot
- each site with its outcome against the base and the accepted list
- the remedy, after `Fix:`

The summary prints no window line, and it counts against the same base. When
no base resolves, both views say so and give no site an outcome. A convention
that cannot run makes the report exit 2. A name that the section does not
define is exit 2.

A run walks each tree once. It reads each file once for every `text`
convention, and it parses each file once for every `code` convention. ADR
0037.

#### B.8.5 Tier 3: defer with a reason

`guard-suites` (#43) and
`manifests` (#44): one stack each.
`db-migration-safety` (#57): deterministic for raw SQL only. `asset-path`
(#59): the ticket expects false positives, which fails criterion 1 in
spirit. `flaky-test-runner` and an MCP server: refused in ADR 0008.

#### B.8.6 Per-check contract

Every check MUST:

- state its identity rule, its ratcheted values and its derivation rule in
  its module docstring
- print one remedy per failure that names what to change
- print, per failure, the site it matched in `before` or the accepted entry it
  matched, and the value on each side, with the ceiling in force beside them,
  so an agent fixes the right thing and a person can dispute a wrong match. A
  `new` finding says that nothing matched. A check that prints a held site
  says the value the base holds it at, for the same reason. Section 11.1
  fixes those line shapes.
- print one `OK:` line with what it judged on success, plus any `NOTE:`
  lines, and nothing else. A ratcheting check writes the state it measured in
  its own vocabulary and its own counts, and the ratchet writes why those
  findings do not fail, because only the comparison knows that. The reasons
  are distinct and a green line MUST NOT claim more than the comparison
  proved, on the one line the ratchet writes for the whole gate:
  `, all held at the base` when a base site holds every finding,
  `, all on the accepted list` when a person-authored entry holds every one,
  `, N held at the base and M on the accepted list` when both hold some, and
  nothing at all when the run judged no finding. A run that measured no file
  judges no finding, so it claims no comparison either, and its coverage says
  what it measured. No check composes that qualifier for itself. A check that
  judges documents one by one and holds them against the base itself, which is
  `doc_size` alone, still says per document what the base holds that document
  at, because that line names a value and not the gate's pass reason.
  What it judged includes the coverage: how many
  files it found, measured, not measured, excluded and could not read, so a
  green run over an unexpectedly small scope is visible on its one line.
  Section 11.1 writes the boundary between those five counts down once, and
  every check uses it.
  A check that judges documents one by one, such as `doc_size`, says each on
  a line and the gate's coverage on one
  line for the gate.
- name every file that is present in both trees, was measured in `before`,
  and was not measured in `after`. Such a file left through compact scope, a
  file the grammar stopped reading, or a discovery rule the tree no longer meets.
  In the hook it is a NOTE. Under `--strict` it is exit 2, per the 0.x runner
  contract that sections 11.3 and 12 replace.
  The rule binds a check whose scope is a set of source files. A check whose
  scope is a list a person writes, a report, or the very set it ratchets,
  such as `inventory`, has nothing to lose this way that it does not already
  judge. A compact source check measures `before` under the `in` / `except`
  scope the base commit records, and under today's when the base records none:
  today's scope applied to both trees could never show a file that a narrowing
  this run took away. Every other rule is today's on both trees. A scoped run reports the
  loss among the files in its scope. The JSON carries the loss as a `lost`
  record under the gate's notes (11.2).
- name a file it could not measure. Outside the hook that is exit 2, with or
  without `--strict` (ADR 0021), when the base could measure the file or did
  not hold it, because the change opened that hole. A file the base held and
  could not measure either is a NOTE in every run. It is not a hole the work
  opened, and no run could ever end green around it, so a tree that holds one
  is green the day klin arrives (5.1). The rule covers a file no grammar reads
  and a form a resolver supports and could not resolve. The base's side is the
  check's own measurement of `before`: a file counts when that measurement
  could not read it, and a form counts when the base holds a form in the same
  file with the same text and reason, at any line, paired one to one. A check
  that reads the base's copy of a renamed file under today's path, which every
  check but `conventions` does, does not count a file the window renamed from
  a path another grammar reads, such as `.ts` to `.tsx`, because the base may
  have read it under its own path. So that file stays exit 2. A rename between
  two paths one grammar reads, such as `.js` to `.mjs`, counts. `conventions` reads the base's copy under
  the path the base holds it at, so its own measurement already decides. In
  the hook each of them is a NOTE, because the agent has no remedy. A file the
  base did not hold stays exit 2 outside the hook, a `lockfile` manifest
  included (8.2.1): the change that adds a fixture invalid on purpose also
  adds it to `except`, and once it is in the base it is a NOTE.
  Pinned by
  `a_file_no_grammar_read_at_the_base_either_is_a_note_by_hand_and_under_strict`,
  `a_file_the_base_parsed_and_the_change_broke_is_exit_two_by_hand`,
  `an_extension_changing_rename_reads_the_base_bytes_under_the_new_grammar`,
  `a_file_no_grammar_read_at_the_base_either_is_a_note_after_a_rename`,
  `a_rename_one_grammar_reads_at_both_paths_keeps_the_note`,
  `a_glob_the_base_holds_too_is_a_note_by_hand_and_under_strict` and
  `an_unparsed_file_is_named_by_each_caller_as_before`.
- run under `klin gate` and under its own subcommand with the same output
- carry tests through the binary only, on a throwaway tree with a base

### B.9 Hook Protocol

#### B.9.1 Host adapter

Hosts come in two kinds, and a port starts by naming which kind the host is.

A shell-hook host runs a command with a JSON event on stdin and reads a
decision from stdout or from the exit code. Claude Code, Codex CLI, Cursor
and Hermes are this kind. Each one is one module under the host adapter that
implements the adapter's trait: the name `--host` takes, the marker
directory, the hook file, the guard's tool matcher, the key its hook file
lists plugins under if it has one, how an unlabelled event is recognised as
this host's, how the event is read, and how a decision goes back. A port of a
shell-hook host adds that module and registers it in the adapter list. No
other module names a host. The trait MAY grow when a host needs a seam no host
had — the prompt event's name, the tree the event names, the stop channel — and
the existing adapters fill the new method with what they already did.

A program-hook host loads a module in its own process and has no shell hook.
OpenCode and Pi are this kind, and klin cannot be the hook. A shim outside
this crate translates the host's event into the harness protocol of 9.7,
spawns the same `klin` command, and translates the answer back. The shim is
the whole port, and klin gains no variant for it. A shim MUST NOT fabricate a
first-class host's payload to reach klin: the harness protocol exists so that
a harness klin does not maintain speaks as itself.

Two facts shape both kinds. The stop answer is the report text, and the
adapter chooses the channel it goes out on: the exit code, stderr, or a JSON
field. A host's guard event carries a list of paths, because one event may
name more than one file, and the adapter maps that list onto the record
below.

One adapter, chosen from the event's shape or from `--host`, maps a host
event to one internal record:

- `tool` (string), `file_paths` (list), `command` (string) for the guard
- `blocked_before` (bool), the host's flag that the hook already blocked this
  turn
- `session` (string) when the host sends one

And maps one internal decision to the host's output. For Claude Code:
pre-tool decisions go out as `hookSpecificOutput.permissionDecision` with
`allow`, `deny` or `ask` and a reason. Stop blocks are exit 2 with the report
on stderr. A stop that ends with something for the person, such as a note
about a file no grammar read or a deleted test the run let through, writes
it as a JSON `systemMessage` on stdout under exit 0. A stop whose host event
klin cannot read writes that note to stderr and exits 1 instead, by the rule
of 16.5, because klin does not know whose shape to tell it in. Prompt and
session-start text go to stdout on exit 0.
For Codex CLI, `allow` is exit 0 and both `deny` and `ask` are exit 2 with
the reason on stderr, because Codex rejects `permissionDecision: ask` on
`PreToolUse` as unsupported. Stop blocks also use exit 2, and Codex makes the
reason a continuation prompt inside the same turn, which runs no
`UserPromptSubmit`. A stop that tells the person uses `systemMessage` too,
because Codex rejects plain text on a stop that exits 0.

For Cursor, an event carrying `cursor_version` is Cursor's, and Cursor sends
that field on every request. The adapter is
tried after Codex and before Claude Code, because those events also carry
Claude Code's fields. `preToolUse` names `Write`, `Edit` and `Delete`.
`beforeShellExecution` carries the shell command at the top level of the
event rather than under `tool_input`; klin reads a top-level `command` on
that event alone, because `beforeMCPExecution` carries the MCP server's own
launch command there and the agent did not run it. `conversation_id` is the
session. Cursor sends no `stop_hook_active`, so `blocked_before` is always
false and klin's own gate block count and gate tree bound the blocks (9.3). `loop_count`
counts the follow-ups one conversation has already taken and MUST NOT be
read as `blocked_before`. Guard decisions that refuse go out as `permission`
on stdout: both a deny and an ask carry `deny` and the reason in
`agent_message`, and exit 2, because Cursor 3.20.21 accepted `ask` on a
shell event but did not enforce it. A question the host does not enforce
fails closed, as it does on Codex. An allow is exit 0 with no stdout, matching
Claude Code and Codex, so a user-scope plugin stays silent in a tree that
never wrote `klin.json`. Stop blocks print a JSON
`followup_message` on stdout and exit 0. Cursor 3.21.18 did not submit the
follow-up of a stop hook that exited 2, and did submit one from a hook that
exited 0 (measured 2026-09-23, `docs/cursor-compatibility.md`). Nothing
enforces an exit-0 block: a Cursor that ignored stdout would let the turn
end, which fails open. The journal still records the stop as blocked. A stop
that tells the
person writes a JSON `followup_message` on stdout under exit 0. Cursor submits
that follow-up as the next user prompt. Before delivery, klin records a hash
of the exact report or message in a handoff record, one file per session id
of the stop's event under `handed/` in the state directory, for a block and
a told stop alike. Only that session's own hooks write its file, and a host
runs one session's hooks in order, so no write for one session can replace
another session's record, sequentially or concurrently. A prompt of that
session with that hash consumes the record and moves neither the prompt
counter nor the mark. Every different prompt of that session, including one
that starts with `klin:`, clears the record and opens a turn normally. A
prose prefix is not a protocol marker. A block whose report klin could not
record is reported and not blocked, because the host would submit it as a
person's prompt and gain a fresh budget; the block its count already took
stays spent. ADR 0045, ADR 0052.

A told message is also recorded as told under the current prompt, less its
window line, whose age moves each minute and says nothing new. A later stop
under that prompt whose message is the same apart from that line tells
nothing, so a message Cursor submits and the agent answers without a change
cannot replay forever. A different message is told, and the session's next
prompt clears the record. A stop that tells nothing because of this carries
`told-before` in its `flags`. A host that submits a told message hears only
one klin recorded first: a stop that lost the state lock (6.5), or whose
handoff record would not write, tells it nothing, because an unrecorded
message would replay. Every such stop records an empty `told` (11.4). ADR
0052.

Cursor runs a project hook from the workspace root and a user hook from
`~/.cursor`, so the working directory is not the tree on a user-scope
install. Every Cursor request carries `workspace_roots`, and a tool event
also carries the `cwd` a relative path in a command stands on. The adapter
names the tree from `cwd`, then from the first `workspace_roots` entry, and
every command a hook runs measures that tree instead of the working
directory: the guard, the hook-mode gate, and the `radius` that moves the
stamp and the mark of 6.2.1. A host that runs its hooks in the tree names
none, and klin reads the working directory as before. `radius --report` is a
person's command with no event, so it reads the working directory.

Each host names the event a person's prompt raises: it is the last `radius`
line in that host's hook table (`UserPromptSubmit` for Claude Code and Codex,
`beforeSubmitPrompt` for Cursor). The spread report of 9.2 rides that event,
and no module outside the adapter names it. The adapter places the payload
once and the resulting event carries its host, named tree and whether it is
this prompt event; guard, gate and radius do not place or parse it again.

In hook mode the exit code is the host's protocol, not the verdict. On Claude
Code, Codex and the harness protocol, exit 2 means "block this stop", whatever
caused it. On Cursor a block is a `followup_message` under exit 0, as above.
The verdict of section 7 lives
in the report and in the `turn` file. Outside hook mode the exit code is the
verdict.

Codex CLI sends Claude Code's
event fields plus `turn_id` on every turn-scoped event. `turn_id` alone
places an event as Codex, and it is tried before Claude Code's fields.
`permission_mode` places nothing, because both hosts send it. A session
start is not turn-scoped, so Codex's is read in Claude Code's shape, which
runs the same `radius`. Codex's `Bash` tool carries a shell command in
`tool_input.command`. An MCP tool carries its own arguments, so the guard
reads a `command` there only when the tool has one. `apply_patch` carries
one or more file paths in its patch headers. The adapter is the only module
that reads a host's JSON.

#### B.9.3 The block policy

ADR 0004 and ADR 0012 hold in policy. A build failure blocks each stop until
the tree builds. In the current product, the Stop hook is the only klin path
that runs the build: `klin gate` outside the hook, `--strict` and CI included,
runs no build entry, so the project's own CI must run the build. This describes
the current CLI and does not constrain a future agent-readiness path from using
local build feedback; project CI remains the authoritative build, type-check
and test boundary. Each hook message that lets a stop end over a tree that does
not build, or that the hook could not build, says so. A gate failure blocks at
most two stops under one prompt, the second only over a changed tree (ADR 0052).
The build stamp in the state directory carries the facts between the processes.

ADR 0004 relies on the host's cap on consecutive blocks. That cap is not in
the current Claude Code documentation. klin MUST bound its own blocks (ADR
0022). The first failing stop spends gate block 1 of 2. After it, a failing
stop over the tree that block was taken over spends no block: the hook
reports and lets the turn end, so an agent that says a break is intended and
stops again without an edit ends the turn. A failing stop over a different
tree spends gate block 2 of 2, whether the failure is the same finding, a new
one, or a tool error. After gate block 2 no gate failure under that prompt
blocks, whatever the tree. Each gate block names its number in the turn.

klin MUST prove gate block 2 from its own record: the tree gate block 1 was
taken over, recorded in the build stamp, and a current tree that differs
from it. The host's `blocked_before` flag says a block happened and never
which tree it saw. Where klin's record holds no gate block and no build
block, the flag counts as a gate block klin never recorded, so the stop
spends none. It never authorizes a second. When klin cannot read that tree, cannot hash the
current tree, or cannot write the record of the block, the hook reports and
spends no block. That holds for the first gate block too: a block klin cannot
record would read as unspent at the next stop, and a host that sends no
prior-block flag would take it again at every stop, so it blocks nothing, by
the rule of 14 that a state klin cannot keep blocks nothing.

A host that submits the block report as another prompt records that exact
report in the session's handoff record of 9.1 before delivery; the matching
prompt consumes it
without opening another turn, so it raises no prompt counter and brings no
fresh gate budget. The match is by exact text, and a host can submit text
klin did not hand off: Cursor merges every stop hook's answer, and another
hook's `followup_message` can win (9.1). So a stop whose host says it follows
a message the host submitted by itself keeps the build stamp of the prompt
that opened the chain, whatever the prompt counter says, and writes it back
under the current counter. The build stamp names the host session that took
it, and a chain keeps only its own session's stamp. A stop that follows no
automatic message opens its prompt's budget: where its session's stamp names
an earlier prompt, the stop writes a fresh stamp for the current prompt, even
if it spends no block. So a chain that a clean stop opened never inherits the
cap of an earlier prompt. Cursor says so with a `loop_count` above 0, and
Cursor 3.21.18 returns the count to 0 for a person's message
(`docs/cursor-compatibility.md`). That rule never adds a block: where a host
does not reset the count for a person's message, it only withholds that
prompt's fresh budget. A genuine
later prompt brings a fresh budget of two gate blocks and eight build
blocks. A deleted test is the one gate failure that
does not stay red: the stop that blocks on it records the question beside
the stamp, and the next stop lets it through as a NOTE and ends green (8.2,
ADR 0031). A deletion already asked about fails nothing, so it is no reason
for gate block 2. A new deletion may spend a gate block that remains, and
once both are spent it is reported and asked about under a later prompt. A
build failure blocks at each
stop that changed the tree since the last build block, until the tree
builds, up to eight in one turn, and then the hook reports, says that it
stopped blocking, and lets the turn end. A stop over a tree the last build
block already saw spends no block: the hook reports the failure, says the
tree did not change, and lets the turn end, because a block over a tree the
agent did not touch teaches it nothing (ADR 0048). The build
stamp holds the count, the prompt counter of 6.2 the count was taken
under, and the tree of 6.5 the last block was taken over, apart from the
gate block count and the tree the last gate block was taken over. A build
block changes only the build fields and a gate block only the gate fields.
A count taken under an earlier prompt reads as zero, so every turn
has eight build blocks and two gate blocks, and only the stop writes the
build stamp. A build-failure
stop writes a RED verdict before it blocks, so the next prompt does not move
the turn stamp over a tree that does not build. Each block names its number
in the turn, and a failing build's report opens with the `derived:` line of
5.4 when the command was derived.

A derived build for a JavaScript project runs the tool that project
installed. From the directory the entry runs in, and then each directory
above it up to the root klin measures and never above it, the nearest
`node_modules/.bin` that holds the tool names it, written relative to the
directory the entry runs in. The tool on `PATH` runs when the checkout
installed none.

A tool the project installed is there even when it cannot run, so a broken
install, such as one whose link points nowhere or one the host cannot
execute, fails its own build and klin does not quietly compile with another:
the 127 of ADR 0048 is an absent tool only for a command klin did not resolve
to a binary in the checkout. klin never invokes `npx`, `npm exec` or any
other command that could fetch a tool, and it starts no package manager. This
applies to a derived JavaScript command alone: a derived `cargo` or `go`
command, and every command a person wrote, run exactly as they read, in the
environment the hook itself was given. A tool a project installed but did not
put on `PATH` is therefore found, and the NOTE below is not told for it.

Each build command runs for at most 300 seconds, and klin stops every build and
`run` command of one run at 600 seconds after klin started, so a later command
gets only what is left before that deadline, and a command with no time left
does not start. The deadline counts klin's own work between commands too. The
host gives the Stop hook 900 seconds, so when klin starts as the stop begins,
no command is still running when the host's time runs out, and whatever klin
does after its last command has at least 300 seconds. Time the host spends
before klin starts, such as a `cargo run` that compiles klin first, is not
counted. When a command reaches its limit or the deadline, klin stops it and
the build fails with a message that names the command and the limit or deadline
it reached. `klin.json` cannot change either: a person whose build takes longer
sets `build` to `false` and has the project's own CI build. A run MAY take
`KLIN_COMMAND_LIMIT` in seconds for tests. It sets the limit of each command,
from 1 to 300 seconds, and the deadline is twice it. Any other value is an error, so the override can
shorten the limit and the deadline and never raise them.

Each command runs in a process group of its own. When the command's shell
exits, when it reaches its limit or the deadline, and when a hangup, an
interrupt or a terminate signal ends klin, klin sends a kill signal to every
process left in that group before it goes on. So a command that starts a
process in the background leaves nothing running that could change the tree
after klin judged it. A process that makes a group of its own leaves klin's
reach, and so does every process when klin is killed with a signal it cannot
catch.

A build whose shell exits 127 is not a build failure. The shell could not
find the command, so the tool is absent and the code is unjudged. The hook
records the build as unmeasured: one NOTE names the command, quotes the
shell, and says that klin judged the source as it stands, that `klin gate`
outside the hook runs no build, so the project's own CI must run it, and
that the action left is to install the project's dependencies or for a
person to set `build` to `false`. The gates then run over the tree,
so a `lockfile` finding for the dependency that was declared and never
installed reaches the agent, and the NOTE is told at a stop nothing blocks.
The exit code is the whole test: klin reads no shell message and guesses no
tool name. An absent tool skips its own entry and no other, so a later entry
that fails still blocks, and the NOTE stands only for a build in which every
entry that ran passed (ADR 0048).

Two facts in ADR 0014 about where hook output goes on exit 0 need one more
check against the current documentation before #91 lands. The documentation
read for this draft says Stop-hook stdout on exit 0 reaches the model. If that
holds, the last turn of a session can carry a radius report at its stop.

#### B.9.4 The guard's three decisions

The guarded set is this tree's `klin.json` and this tree's own state
directory. The guard resolves both: the configuration beside the tree root
`git rev-parse --show-toplevel` names, and the state directory of 7.4. A
host's hook file, CODEOWNERS, `refs/worktree/klin`, every verification file,
another project's `klin.json` inside the tree, and another worktree's state
directory are ordinary files, and an edit to one of them is `allow`. ADR
0027, ADR 0032 and ADR 0033 record why.

The guard answers only where it can prove that a tool call writes a guarded
path. Everything else is `allow`, and a write the guard misses costs nothing,
because ADR 0009 makes CI authoritative. ADR 0033 records the reversal.

- `deny`: an edit tool whose path is the configuration or reaches inside the
  state directory, a redirect onto either, `init` in any form, `install` in
  any form, and `turn reset`. The reason names the file and says a person changes it in a
  reviewed commit, or names the command a person runs instead: `klin turn
  reset` for the stamp, and `klin cache clean`, which stays open to an agent,
  for the cache. Nothing else denies.
- `ask`: a command that writes every argument it takes, with a guarded path
  among its arguments. The writers are `rm`, `rmdir`, `unlink`, `shred`,
  `mv`, `truncate` and `tee`, and `sed` or `perl` carrying `-i`. `cp` and
  `install` are not writers here, because each reads its first argument. The
  reason quotes the token that matched. The person decides.
- `allow`: everything else, including every command that only names a guarded
  path.

A path resolves from the directory the guard runs in, with `.` and `..` taken
out and every symbolic link its existing part carries followed. A path the
guard cannot resolve is `allow`. A token holding a shell wildcard proves
nothing about the file it stands for, so the guard matches no path against
it, and `rm *.json` in the tree root passes.

The guard matches no path in a command that holds shell it does not read: an
unbalanced quote, `$(`, a backtick, `${`, `<<`, a backslash, or a `cd`
command word. An unbalanced quote allows the whole command, because the
command then does not say where its arguments end. The other six suppress
path matching alone. `init`, `install` and `turn reset` name no path, so their `deny`
stands behind any of them, and a command substitution in command position is
a prefix in front of klin's own name.

A command splits into segments at `;`, `&`, `|` and a newline, and the split
MUST honor single and double quotes (issue #90). A redirect is an unquoted
`>`, so a `>` inside an argument is a character of that argument and not a
redirect. A writer and klin's own name are read by the basename of the
command word, and each is found behind the same prefixes: an assignment,
`env`, `npx`, `pnpm`, `bunx`, `time`, `nice` and `sudo`. So a script of the
tree's own named `rm` is read as `rm`, and `sudo rm klin.json` asks.

For Codex CLI, an `apply_patch` call is judged by every path in its `*** Add
File:`, `*** Delete File:`, `*** Update File:` or `*** Move to:` headers.
Patch body text is data. A deny for any path wins; otherwise an ask wins over
allow.

The guard MUST NOT read the configuration. It runs before the config loads.
It MAY read `KLIN_STATE_DIR` and run `git rev-parse` to resolve the tree root
and the state directory, and it reaches for git only when a path needs
proving. It MUST finish in under 50 milliseconds, because it runs on every
tool call.

#### B.9.7 The harness protocol

A harness klin does not maintain speaks klin's own event shape rather than
another host's, so no port has to fabricate a first-class host's payload.
`klin_protocol` names the version, and it is the field that places the event:
no host klin maintains sends it, so the protocol's adapter is tried before all
of them. `--host harness` overrides detection, and refuses a payload that
carries no version of this protocol. The adapter's name, and so the `host` of
a journal line (11.4), is `harness`. Before 1.0 the protocol was called
`generic`, and that name names no host now.

Version 1 carries `event`, one of `session`, `prompt`, `pre_tool` and `stop`,
and then `root`, `session`, `prompt`, `tool`, `file_paths`, `command` and
`blocked_before`. `harness-protocol/event.schema.json` is the checked-in
schema. The adapter maps these onto the one internal record of 9.1 and nothing
further: klin gains no second turn, guard or gate engine for a custom harness.

The protocol carries only evidence the harness proves. `file_paths` holds the
paths the harness can prove the call will touch, and `command` holds a command
only where the harness knows the one the agent is about to run. Missing
evidence stays missing: klin MUST NOT read a file write out of a tool name or
out of opaque tool arguments, and a call that proves neither a path nor a
command is allowed.

The decision is one JSON object on stdout, under
`harness-protocol/response.schema.json`: `allow`, `deny` with a `reason`,
`block` with the report as its `message`, or `tell` with a note as its
`message`. Exit 0 carries `allow` and `tell`, exit 2 carries `deny` and
`block`, and a refusal also goes to stderr so it holds where stdout goes
unread. At most one decision is printed, on a line of its own, beside the
report text a blocked stop also prints. A stop that passes prints no decision,
as it does on every other host, so exit 0 with no decision ends the turn. No host-specific field of Claude Code, Codex CLI or Cursor appears in
this protocol. There is no `ask`: nothing here proves a question the harness
enforces, so the ambiguous class of 9.4 fails closed as it does on Codex and
Cursor, and 19.4 has the integrator record that difference.

A `klin_protocol` klin does not speak fails clearly and closed. klin names the
version the event sent and the version it speaks, refuses every tool call and
blocks every stop while that mismatch stands, and MUST NOT read the event as
another host's.

#### B.9.8 One copy per host event

A host may run several copies of klin's hooks for one event on one machine: a
native plugin beside committed project hooks or user-scope hooks, and Cursor,
which runs the hooks in Claude Code's settings files beside its own by
default. Exactly one copy takes effect per event. klin compares no hook
line and knows nothing about the other copies. Each copy reads the same
payload, because the host sends one event to all of them.

**The event's identity.** An event's identity is the values of the fields that
name it, as each host sends them (`docs/HOST_COMPATIBILITY.md`):

- Claude Code and Codex CLI: `session_id`, `hook_event_name`, `prompt_id`,
  `turn_id`, `tool_use_id`, `source`, `stop_hook_active` and
  `last_assistant_message`, where the payload holds them.
- Cursor: `conversation_id`, `generation_id`, `session_id`,
  `hook_event_name`, `tool_use_id`, `tool_name`, `tool_input`, `command`,
  `cwd`, `status` and `loop_count`. Cursor sends every copy the same payload
  in its own shape, including a copy it imported from Claude Code's settings
  or a Claude Code plugin. A shell call is the one exception: Cursor's own
  hook receives it as `beforeShellExecution` and an imported one as
  `preToolUse` on the `Shell` tool. So a shell call is named by
  `conversation_id`, `generation_id` and its command alone.
- A custom harness (9.7): none. It runs one copy per event.

An event has an identity only when its payload names a session and one field
that scopes the event inside the session: `prompt_id`, `turn_id`,
`generation_id`, `tool_use_id` or `source`. A payload without both cannot tell
two copies of one prompt from two prompts, or two sessions from each other.
Such an event is run by every copy, as before this section.

**The claim.** Each copy that acts on an event first claims its identity in
`claims/` in the state directory (7.4). The claim is held while the copy acts.
A copy yields when another copy holds the claim, or when that copy let the
claim go less than two seconds before. The host starts every copy of one event
together and starts the next event only after all of them answered. So a late
copy arrives within a process start of the first one, and two different
events with one identity are a model turn apart. A claim older than the window
belongs to an earlier event, and the copy that finds it takes it. A copy that
cannot write the state directory acts, because an event run twice is the
older failure and an event run by no copy would drop a block.

**What a copy that yields does.**

- `klin radius` prints nothing and exits 0. It moves no stamp, no mark and no
  prompt counter, writes no journal line and consumes no follow-up of 9.1.
  The claim comes before the follow-up is consumed.
- `klin gate --hook` prints nothing and exits 0, and runs no gate. It cannot
  weaken the other copy's block: Claude Code and Codex apply the most
  restrictive answer across hooks.
- `klin guard` gives its answer as every copy does, and writes no journal
  line. The guard's answer is the same in every copy, and a copy that let a
  call through because another call had the same identity would open that
  call. So only the journal line depends on the claim.

Two different events, and events from two sessions on one worktree, never
have one identity. A late copy whose wrapper took longer than the window to
start, such as a plugin wrapper that downloads its binary on first run, acts
on the event again, so for that one event the prompt counter moves twice.

### B.11 Output Contract

#### B.11.1 Text

Stable, tested line shapes. `ok    NAME`, `FAIL  NAME`, `ERR   NAME` for
rows. `OK:` for a passing gate's one line, printed under its row by the
runner and on its own by the gate's subcommand, because 8.6 asks for the same
output from both. `FAIL:` for a failure with its remedy under it. `NOTE:` for
a note. One `window:` line first. One `derived:` or `pinned:` line per value
a gate used, above that gate's row, and the hook's build lines above every
row.

Every `OK:` line ends in the gate's coverage, as `(N file(s) found, N
measured, N not measured, N excluded, N unreadable)` when the check has
unmeasured files; otherwise it keeps the four-count shape
`(N file(s) found, N measured, N excluded, N unreadable)`. The boundary is
the same for every check: `found` counts every file the check's own discovery
rule reached under its roots, before anything dropped one; `excluded` counts
the ones an exclusion dropped; `unreadable` counts the ones it reached and
could not read or parse; `not measured` counts known-language files for
which this check has no structural adapter; `measured` counts the ones it
judged. A scoped run counts only the files in its scope. A check whose scope
is not a set of files counts the thing it discovers — a document, a manifest,
a test file the base holds — and its module docstring names that thing.

Every failure line ends in what the ratchet judged it against: `— matched the
base site at FILE:LINE`, `— matched the accepted entry for FILE`, or `—
nothing matched` for a `new` finding, with `, ceiling C` after it wherever
the gate has a ceiling of its own. So an agent fixes the site the ratchet
compared, and a person can dispute a wrong match.

#### B.11.4 The journal record

One JSON line per event of 9.6. Every kind shares four fields:

- `schema` integer, 1 for this record. A reader MUST skip a line whose
  `schema` it does not know, and MUST count what it skipped so a report can
  say so. A schema bump without an upgrade in the reader MUST NOT ship.
- `version`, the klin version that wrote the line, and `time`, seconds since
  the epoch.
- `kind`, `"stop"`, `"prompt"`, `"guard"` or `"reset"`.
- `session`, the host's id for its grouping of turns, null where the event
  carried none or named no host, and always null on a `reset` line, which
  `klin turn reset` writes over no host event at all.

The `stop` line is the 11.2 object the stop's run built — the gates, a build
failure, or an error alike — plus what only the hook knew:

- `host`, the adapter's name, null where the event was unreadable.
- `prompt`, the counter of 6.2 the stop ran under.
- `hook` `{blocked, delivery, gate_spent, gate_blocks, gate_block,
  build_blocks, blocked_before, continued}`. `continued` is whether the host
  said this stop follows a message it submitted by itself (9.3). `blocked` is
  whether this stop blocked, whatever exit code its host takes
  for a block (2 on Claude Code, Codex and the harness protocol, 0 on
  Cursor, 9.1).
  `delivery` is `block` or `none`; `follow-up` and `report` are reserved for
  a host whose stop cannot block (#67). `gate_blocks` and `build_blocks` are
  the build stamp of 16.3 as this stop left it, and `gate_spent` is whether
  `gate_blocks` is above zero. All three are cumulative, so a later stop sees
  them too. `gate_block` is the number of the gate block this stop itself
  spent, 1 or 2, and null on every other stop: an intervention is a failing
  gate on a line whose `gate_block` is set (ADR 0034, ADR 0052).
  `blocked_before` is the host's flag.
- `verdict`, `green`, `red`, or `none` for a stop that wrote no verdict, with
  a `why` string beside `none` that names the reason this stop had and no
  other: the lock timed out, the state directory could not be readied, it held
  no stamp klin could read, or the stamp klin read could not be written back.
- `timing` `{total_ms, build_ms, lock_ms, base_remove_ms, base_prune_ms,
  klin_ms}`, where `klin_ms` is the total less the build, so the budget of 13
  reads straight off it. A stop that laid the whole base out removes its
  linked worktree before it writes this line, so `total_ms` holds the removal,
  and `base_remove_ms` and `base_prune_ms` are the parts `git worktree remove
  --force` and `git worktree prune` took, zero for a stop that laid out no
  whole base.
- `asked`, the site ids this stop asked about, as the turn stamp records
  them (8.2).
- `flags`, the unusual paths this stop took, empty on a clean stop:
  `turn-restored` (16.1), `branch-fallback` (a stop that judged a branch
  window because no stamp resolved, or because the commit the stamp was taken
  over is outside current HEAD history, 6.2), `count-unwritable` (a build
  stamp that would not write, 14), `told-before` (9.1, a message this prompt
  already told a host that submits it as a prompt, told nothing this time),
  `no-prompt-event` (16.3, a prompt that
  spent a gate block found no prompt line for the stop's session).
- `told`, the parts of the `systemMessage` this stop printed for the person,
  empty where it printed none: `note` (8.2, 14, 16.3), `turn` and `weekly`
  (9.5). A reader finds the last weekly line from it.
- `window`, the window the stop judged, which the line carries even where its
  run compared nothing against a base, so a reader knows which turn stamp
  each stop judged against (11.5).
- `config_hash`, a hash of the config file in force, so a later reader can
  tell a fix from a config change without a schema bump. Recorded and not
  read.

The `prompt` line is the `UserPromptSubmit` event `klin radius` ran on:

- `prompt`, the counter this event raised it to.
- `text`, the prompt's first line cut at 80 characters. Absent where
  `journal.prompt` (5.2) is `false`, where the configuration would not load,
  or where the event carried no prompt text.
- `radius` `{lines, formatting, moved, directories, wide}`, the facts `klin
  radius` measures (ADR 0014), present only where radius could measure them:
  a first prompt, with no mark yet to measure from, carries none. `wide` is
  whether the turn spread past the project's usual change, the same test
  that decides whether `klin radius` prints its note.

The `guard` line is one `ask` or one `deny`, never an `allow`: the guard
runs on every tool call under its 50 millisecond budget (13), and an allow
tells a reader nothing.

- `decision`, `"ask"` or `"deny"`: the answer the host delivered, not the one
  the guard reached. A host with no question to ask refuses instead (9.1), so
  an ask that reached the agent as a refusal is recorded as `"deny"`.
- `reason`, a hyphenated tag naming why the guard decided as it did:
  `config-write` and `state-write` for a proven write an edit tool's path or
  a redirect names, `config-mention` and `state-mention` for a command that
  only names a guarded path as a writer's argument, which klin cannot prove
  the way it proves the first two (ADR 0033), and `init` and `turn-reset`
  for one of klin's own subcommands that only a person runs.

The `reset` line is `klin turn reset`:

- `prompt`, the counter the stamp carried over. A reset does not end the
  turn, so the counter is unchanged by it (6.2).

A reader on the stop path MUST NOT read the whole file, whose length is the
worktree's whole lifetime. It reads backwards from the end and stops at the
first line older than the history the stop's decision needs: the seven-day
cutoff the weekly line of 9.5 reads, or the second before the turn stamp was
taken where the turn reaches further back. The stamp's own second cannot be
the bound, because the `klin radius` run that appends a `prompt` line takes
the stamp after appending it. That stopping line is also how the stop
tells a journal reaching further back from one beginning inside the window,
which the weekly line of 9.5 asks about. One bounded reader answers for the
whole stop: the turn end and the `no-prompt-event` check of 16.3 read one
tail and not one each.

The bounded reader MUST NOT stop at a line it could not parse or whose
schema it does not know: a truncated last write is not an older line, and a
line from a newer klin is skipped and counted like any other. It MUST NOT
report a `prompt` line absent because a byte or line limit was reached. A
worktree holding no readable stamp bounds nothing and reads the whole file.
`klin stats` (11.5) keeps the whole-file reader, because its windows ask for
history the stop path never needs.

The file is a public surface. `klin stats --json` (11.5) is what a harness
reads, and the two readers of the design read nothing else.

#### B.11.5 The counted unit of `klin stats`

A **regression** is one finding site that a blocked stop put in front of the
agent because it was new or worse than the base. The blocked stop is one whose
line records a `gate_block` (11.4), and a build block is none. A line an older
klin wrote records no `gate_block`, and its `blocked` stands in (ADR 0052). It
is the person's unit, and
the word `shortcut` MUST NOT appear in any text `klin stats` or a turn end
prints.

The same site over several blocked stops in the window is one regression, with
its latest outcome. A regression's identity is the finding `id` of 11.2 where
the record carries one. That id hashes the gate, the path and the declaration
text, so a rename produces a different id. The reader MUST stay conservative
and MUST NOT merge two ids because their text resembles each other. A record
carrying no usable id — `doc-size` is one such shape — is keyed by the smallest
conservative key its recorded fields allow, which is its gate, file, line and
text. Such a record MUST NOT be dropped, and two distinguishable findings MUST
NOT be merged. A reader MUST NOT deduplicate by line number alone.

The **outcome** is a relation between the lines, which the writer never stores:

- `fixed-next`, the site was absent from the first stop after it was flagged
  that measured its gate
- `fixed-later`, the site was absent from a later measuring stop, and the
  record says how many measured stops it took
- `config-changed`, the site went from a measurement taken under a
  `config_hash` (11.4) other than the one in force when it was flagged. The
  report MUST NOT call that a code fix
- `set-aside`, a person moved the turn stamp before the site went. The report
  MUST NOT call it fixed, MUST NOT call it currently open, and MUST NOT say it
  is still in the tree, which the report cannot see
- `open`, no stop in the window measured its gate without it
- `asked-once`, a later stop recorded the site, by its gate, file and line, as
  one klin let through after it asked (8.2), or as a deleted test function
  whose file went too. The code is as the agent left it and the fix is the
  person's to make, so it is a question and not a regression: it stays out of
  the regression count and keeps its own audit entry. Pinned by
  `a_deleted_test_klin_let_through_after_asking_counts_only_as_asked_once`,
  `the_stop_that_lets_a_deleted_test_through_says_no_regression_was_fixed`,
  `a_deleted_test_restored_after_the_block_counts_as_caught_and_fixed_next` and
  `a_deleted_test_whose_file_went_after_klin_asked_is_not_a_fixed_regression`
  in `tests/stats.rs`

A gate the stop's `gates` list of 11.2 carries no row for did not run, and a
row that says `ERR` measured nothing. Neither ends a regression, and neither
counts as a measured try. Two sites under one failing gate end apart from each
other: a site absent from a stop that measured its gate went, whether or not
another site kept that gate red.

The report MUST NOT say who authored a fix. The journal proves a site was
present and later absent from a measurement, and nothing about who edited the
code. `The agent fixed all 12.` is therefore forbidden, and `All 12 were fixed
after klin flagged them.` is the form.

### B.12 Determinism

- Every `git diff` klin runs MUST pin `--diff-algorithm=histogram` and pass
  `-M` or `--no-renames` by name (ADR 0014).
- Findings MUST be sorted by file then line before matching and before
  printing.
- Grammars are compiled into the binary. A grammar version change is a klin
  version change, and the survey cache and the structural cache both key on
  the version.
- No check MAY read the network.
- The only clock a judgment reads is a pinned dated ceiling (5.5), read in UTC,
  and `KLIN_TODAY` overrides it. The report age check of 8.3 compares file
  times, and the command limit and deadline of 9.3 stop a build or `run`
  command that outlives them. These are the two other places time enters a
  verdict, and in both two machines can judge one tree differently: file times
  can differ between checkouts, and a command that ends just under its limit or
  the deadline on one machine and just over it on another passes on the first
  and fails on the second. Every other field a verdict depends on is a pure
  function of the trees. The `ms` of 11.2 and the `time` and `timing` of 11.4
  are measurements about the run, recorded and never judged, so they do not
  break determinism.
- A `run` entry in 8.3 is deterministic only when the tool it runs is. klin
  MUST record the command it ran beside the results.
- A derived number or reachability family is a pure function of the derivation
  commit and the binary version. Other derived path sets are the union of that
  commit's survey and `after`; source checks discover their own facts (4.3).

### B.13 Performance Budget

Before parsing grammar-backed source, klin MUST refuse any source line longer
than 65,536 UTF-8 bytes, excluding its line terminator. This deterministic
resource ceiling applies equally to both trees and every language read by the
shared parser. The error MUST name the file, one-based line, measured byte
count and ceiling as a `source-line resource ceiling exceeded` error (exit 2),
rather than silently exclude the file or truncate its site identity. Survey
and tolerant readers that already omit refused parses omit this source too.
The source-line ceiling is defense in depth, not a bound on total retained
site text or process memory. Complexity's survey MUST collect numerical
metrics without materializing function site text or body hashes. Its retained
function sites MUST share one text per source-row and holder-row pair within
a file; an owned finding text is materialized only for a function over its
ceilings. Dense lines below the source-line ceiling MUST retain every
supported function in both trees and the survey. The CLI pins are
`an_oversized_source_line_reports_a_named_resource_error`,
`the_source_line_resource_ceiling_is_inclusive_and_counts_utf8_bytes`,
`dense_sub_ceiling_lines_keep_every_function_in_both_trees_and_the_survey`,
`a_minified_bundle_reports_a_resource_error_in_json` and
`an_oversized_test_source_preserves_the_named_resource_error` and
`an_oversized_source_preserves_the_named_resource_error_in_stubs`.

The Stop hook, excluding the project's own build, SHOULD finish within 5
seconds on a tree of 2,000 source files when scoped with `--changed` to 20
files and the survey cache is warm. The first stop on a new base MAY take the
whole-tree survey and SHOULD finish within 30 seconds on the same tree. A
whole-tree `--strict` run in CI SHOULD finish within 60 seconds. An
implementation MUST measure all three on a fixture and record the numbers in
the release notes when they move by more than a third.

The guard MUST finish within 50 milliseconds.

The dense structural rows have a separate large-repository product budget. The
`structural_300k` row holds exactly 325,077 source lines in 10,000 source
files, and `structural_1m` holds exactly 1,033,827 source lines in the same
10,000-file split. Excluding the project's own build, the 20-file warm Stop
row, the first-stop cold survey and the whole-tree strict run SHOULD finish
within these limits:

| Workload | `structural_300k` | `structural_1m` |
| --- | ---: | ---: |
| Warm Stop hook, 20 changed files and a warm structural cache | 5 seconds | 5 seconds |
| Cold survey | 30 seconds | 60 seconds |
| Whole-tree `--strict` run | 20 seconds | 45 seconds |

These are product requirements, not the current implementation's medians.
They are rounded limits with operating headroom, chosen from the final
controlled measurements in #193: 2,388 ms and 2,733 ms for the warm rows,
22,834 ms and 47,507 ms for the cold rows, and 12,168 ms and 30,724 ms for
the strict rows. The 100-file warm row remains a fixed-repository scaling
check, not a second product budget. The existing 2,000-file budgets above are
unchanged.

Before a release, the dense rows MUST be rerun on the controlled reference
machine and compared with the preceding controlled release. A median that
misses one of the limits above, or rises by more than one third against its
preceding controlled row, MUST be explained in the release notes before the
release proceeds. This is a release checklist rule: ordinary contributor
tests continue to verify fixture shape and semantics, and do not use
machine-specific wall-clock or RSS values as an oracle.

The normative measurement fixture is the ignored `performance_fixture` CLI
test, selected by `cargo test -- --ignored perf`. It generates a deterministic
temporary repository with `rust/` and `web/` projects: the 2k row holds 1,000
`.rs` and 1,000 `.ts`/`.tsx` files, and the 10k row holds 5,000 of each. A
small one-percent TypeScript subset is `.tsx`; both rows include Rust and
TypeScript manifests, tests, imports and references, and held escape/stub
sites. Each row changes 10 files in each language. The test runs warm hook,
cold survey and whole-tree strict five times and prints the median; the hook
row explicitly excludes the project build. It also sends 1,000 deterministic
read, write, shell, quoted-path, glob, heredoc, configuration-name and
multi-path patch events through the real guard binary path five times, as a
separate row. The test records the klin version, fixture counts, cache state,
changed-file counts, iteration count and median milliseconds; it does not
enforce the budgets on contributor hardware. The dense rows additionally
change 40 more files in each language and print a 100-file warm row. The dense
fixture asserts that structural counters grow with that delta: changed files
are extracted from both trees, while unchanged base files remain cached and
shared.

The same test also measures two source-dense structural rows. The
`KLIN_PERF_ROW` environment variable selects one row:
`KLIN_PERF_ROW=structural_300k cargo test --release --test performance --
--ignored perf` or `KLIN_PERF_ROW=structural_1m`. Without the variable the
test runs the 2k, 10k and guard rows as before. `structural_300k` holds 10,000
source files and exactly 325,077 source lines. `structural_1m` holds the same
5,000 Rust plus 5,000 TypeScript split and exactly 1,033,827 source lines.
Each dense row reports the primary 20-file delta and the additional 100-file
warm delta; the latter is a fixed-repository scaling check, not a product
budget. Both rows keep 50 `.tsx` files, tests, entry points and held escape
and stub sites.

Each dense file repeats one structural unit: two constants, a struct or
interface, a type alias, an `impl` or class with a method, a function in one of
256 duplicate-name buckets, an exported value function that imports and calls
its neighbour, and a branching function. `structural_300k` writes one unit per
file and `structural_1m` writes three or four. The rows differ in source volume
and keep the same shape, so fixture generation asserts that both hold 270 to
290 declaration lines per 1,000 source lines. Generation also asserts file
counts, the language split, the TSX count, exact LoC, the exact declaration
count, an FNV-1a digest of every generated path and byte, and representative
structure. Configured Rust and TypeScript module families exercise
`complexity`, `dead-symbols` and `reachability` through the real binary. The
dense rows also write a `layering` section with
one layer per language and `acyclic` set, so every row builds both module
graphs and finds their cycles, and each gate row prints `graph_modules`,
`graph_sources`, `graph_dependencies`, `graph_edges`, `graph_ms` and
`graph_dispatches_<language>`, so a row tells semantic modules from the
physical files they hold, and dependency sites from the distinct module pairs
those sites join. The `public-api` row also prints
`surface_dispatches_<language>`. The warm hook asserts that `layering`
reads and parses no source of its own, because it takes every structural
outcome an earlier gate of the stop already held. `KLIN_PERF_LAYERING`
chooses the section: `on`, the default, writes it, and `off` leaves it out,
for a binary before #50 that reads none. The choice does not depend on
`KLIN_BIN`, so a row taken with and without it judges the same gates. The
fixture line prints `layering=on` or `layering=off`. It prints `off` for the
2k and 10k rows and for the `legacy` configuration, which never hold the
section.
The fixture's `web/package.json` names `./src/index.ts` under `exports`, and
its `rust/src/lib.rs` is the implicit library root, so every row derives one
Rust and one TypeScript public surface, and the `public-api` gate row prints
`surface_surfaces`, `surface_items`, `surface_measured`, `surface_opaque`,
`surface_holes` and `surface_ms` beside its `graph` counters. The warm hook
asserts that `public-api` reads and parses no source of its own, for the same
reason as `layering`. A binary before #46 has no `public-api` row, and its
rows leave the counters out.

Each dense row runs warm hook, cold survey and whole-tree strict five times
and prints the median total and every gate's median `ms`. Where available it
also prints deterministic content-work and structural-fact counters:
`work_reads`, `work_parses`, `facts_reads`, `facts_parses`, `extracted`,
`shared`, `cached`, `cache_read_ms` and `cache_write_ms`. The `dead-symbols`
and `reachability` rows also print the `names` group of 11.2 as
`names_base_ms`, `names_lost_ms`, and each tree's values under
`names_before_` and `names_after_`, so a warm row separates the base layout,
each tree's measurement, index build and name queries, and the lost
references from the rest of the gate's time. The row that laid the base out
also prints the parts of `names.layout` as `names_layout_<name>`, and a warm
hook row prints the stop's `timing` values as `stop_<name>`, so the worktree
checkout, the base file list, the cache and the worktree removal are told
apart. A targeted warm row also times, on the fixture's repository and apart
from every stop, the git commands a lighter base layout would use, five times
each with the median printed as `worktree_<name>_ms`: a whole detached
`worktree add`, its `worktree remove --force`, a `worktree add --no-checkout`,
a `read-tree` into it, an `ls-files --stage` listing of it, a `checkout-index`
of the changed files' base paths into it, its removal, and `worktree prune`.
Those medians are an estimate of a candidate, not a measurement of klin. The `dead-symbols` row also
prints the `footprint` group of 11.2, each counter as `footprint_<name>` and
each type size as `footprint_size_<name>`, so one row carries the population,
sparsity and byte proxies of the facts that run held. `KLIN_PERF_CASE=warm20` or `warm100` runs
one targeted warm scenario of either dense row, so a 1M warm row needs no cold
or strict run. A row that runs one
targeted warm scenario prints the structural cache's file count and bytes
after its stops and the peak RSS of one more warm hook, where a full row
prints them beside the peak memory. A second warm hook
row removes the structural cache of 8.4 before each stop, so it measures a
stop that extracts the base and writes the cache, beside the first row's
stop that reads it. After the rows, one untimed stop writes the structural
cache again, because the cold rows' `cache clean` removed it, and the output
prints the file count and the bytes on disk of that cache. The warm hook rows
read each
gate's values from the journal line of the stop it timed. The cold and strict
rows read them from `--json`. A gate's `ms` covers its whole run:
reading, parsing and extracting the files that no earlier gate of the run
extracted, building a structural index where name resolution needs one, and
its own algorithm. Its `facts.ms` is the first part, so `ms` less `facts.ms`
is the time of its index and its algorithm. A run extracts each structural
file of a tree once, so the
first gate that reads a file pays for the extraction, and a later gate counts
that file in `facts.shared` (11.2). In the warm hook, `dead-symbols` and
`reachability` take the base's facts for every unchanged working-tree file
(8.4), so their `extracted` is the base's structural files plus the changed
ones, and their `shared` is the unchanged ones. Where the structural cache of
the turn's base holds the base's outcomes, `extracted` is the changed files of
both trees and `cached` is the unchanged ones. `complexity` walks a parse of
its own,
which no extracted fact replaces, so its `ms` still covers its parsing. A row
taken with an earlier binary through `KLIN_BIN` prints no fact counters when
that binary records no `facts`. The output records source LoC, declarations,
digest, file and language counts, cache state, changed files, iteration count,
version and host platform, and excludes project build time from hook timing.
On a platform with `/usr/bin/time`, it also prints the controlled peak RSS of
one warm hook, one warm hook without the structural cache, one
`gate --changed --json --gate dead-symbols` and one strict run;
missing resource reporting is not a test failure. These rows record
measurements and add no wall-clock or RSS budget.

`KLIN_PERF_ROW=source_areas` selects the root-count rows: the same 2,000
source files, 1,000 Rust and 1,000 TypeScript, split over 2, 100 and 500
directories that hold nothing but source under a manifest directory that holds
more, so each is a source area the tree listing discovers. Each
row is the whole-tree strict run, five times, median. A run reads a tree's
file list once and asks git once what it ignores, whatever the root count, so
the three rows MUST read alike; a row that grows with the root count is a walk
or a process per root coming back (ADR 0038). `KLIN_BIN` names another klin
binary for the harness to run, so a row can be taken under an earlier release
beside the current one. Such a run does not assert that every dense gate
reported a time, because an earlier release may lack one.
`KLIN_PERF_SCOPE=rust` gives the 2k, 10k and dense rows a `complexity` section
of `"in": "rust"`, recorded at the base, so the derived ceilings sample one
source area. Without it, or with `whole`, the section is absent and the
sample is the whole repository.

`KLIN_PERF_CONFIG` chooses the configuration of the 2k, 10k and dense rows.
`build-off`, the default, writes `{"build": []}`. `empty` writes `{}`, so the
hook derives the build, and puts stand-in `cargo` and `tsc` commands first on
the path, so the row measures the build's preparation and no compiler.
`legacy` pins the configuration the running binary's own `init --force`
writes after the base and commits it, which only a binary before #180 reads,
so it is taken with `KLIN_BIN`. Comparing it with `build-off` under the
current binary measures what discovering the facts costs (ADR 0040).

A check that cannot take scope, such as a whole-tree duplication share, MUST
say so in `gate --list` and MAY be skipped by the hook under a `hook: false`
key on its section.

### B.16 Reference Algorithms

#### B.16.1 The hook's window

```
state = KLIN_STATE_DIR/hash(common_dir, worktree) if set else git_dir()/klin

read_stamp():
  stamp = read(state/turn)
  if stamp is not None: return stamp
  commit = resolve("refs/worktree/klin/turn")
  if commit is None: return None
  note("turn file missing, restored from the ref")
  stamp = Stamp(commit, parent=parent(commit), time=None, last_verdict=RED)
  # parent(commit) is commit^, which names the stamped HEAD only because the ref holds a
  # synthetic stamp. 6.2 is why no other commit may be written to that ref.
  write_atomic(state/turn, stamp)
  return stamp

holds_head(commit):                    # 6.2, the three answers stay apart
  status = run("git merge-base --is-ancestor " + commit + " HEAD")
  if status == 0: return True                              # ancestor of HEAD, or HEAD
  if status == 1: return False                             # proven divergent
  return None                                              # git could not answer

left_behind(stamp):
  return stamp.parent is not None and holds_head(stamp.parent) is False

hook_window():
  stamp = read_stamp()
  if stamp is not None and left_behind(stamp):
    note("the turn started from a commit HEAD no longer holds, judging the branch")
    delete_ref("refs/worktree/klin/turn")                  # no restore of what HEAD left
    delete_ref("refs/worktree/klin/mark")                  # 6.2.1
    before = branch_stamp(stamp, mark=None)
    return Window(BRANCH, before, WORKING, "the turn HEAD left behind")
  if stamp is None:
    note("stamp deleted, judging the branch")
    return Window(BRANCH, branch_stamp(stamp, mark=mark_of(stamp)), WORKING, "stamp missing")
  return Window(TURN, stamp.commit, WORKING, "since " + stamp.time)

branch_stamp(stamp, mark):             # the base this stop judges, written back red
  before = choose_window(strict=False).before or HEAD      # 16.2
  write_atomic(state/turn, commit=before, parent=before, time=now, last_verdict=RED,
               prompt=prompt_of(stamp), mark=mark,
               asked=[], intervened=False, followup=None)
  return before

on_session_start_or_prompt():          # one rule for both events
  stamp = read_stamp()
  tree  = write_tree(index=state/index, add_all=True)
  if event == PROMPT: report_radius(read_mark(), tree)   # 6.2.1, never on a session start
  if stamp is None and not exists(state): move_stamp(tree)   # first session here
  elif stamp is not None and stamp.last_verdict == GREEN: move_stamp(tree)
  elif stamp is None: note("stamp deleted, the next stop judges the branch")
  move_mark(tree)                                  # 6.2.1, on both events, whatever the verdict
  if exists(state/turn): bump_prompt(state/turn)   # prompt += 1, whether or not the stamp moved

turn_reset():                          # a person's command, denied by the guard
  tree = write_tree(index=state/index, add_all=True)
  move_stamp(tree); move_mark(tree); print("a person moved the window")

move_stamp(tree):
  commit = commit_tree(tree, parent=HEAD)
  update_ref("refs/worktree/klin/turn", commit)
  write_atomic(state/turn, commit=commit, parent=HEAD, time=now, last_verdict=None,
               prompt=current_prompt(state/turn), mark=current_mark(state/turn))

move_mark(tree):                       # 6.2.1, the window the radius report measures
  commit = commit_tree(tree, parent=HEAD)
  update_ref("refs/worktree/klin/mark", commit)
  write_atomic(state/turn, mark=commit)

read_mark():
  return mark_of(state/turn) or resolve("refs/worktree/klin/mark")

derivation_commit(window):
  return stamp.parent if window.kind == TURN else window.before

roots(window):
  return survey(derivation_commit(window)).roots | survey_roots(window.after)
```

#### B.16.2 `klin gate` and CI window

```
choose_window(strict):
  for (ref, kind, source) in candidates():
    before = resolve(ref)
    if before is None: continue
    if before == HEAD and tree_is_clean():
      if source == Remote: return Window(kind, before, WORKING, "same, remote agrees")
      tip = remote_default_tip()
      if tip is None:
        if strict: fail("cannot tell whether history is hidden")
        return Window(kind, before, WORKING, "same, no remote to ask")
      if commits_between(tip, HEAD) > 0: fail("unpushed commits hidden")
      return Window(kind, before, WORKING, "same, remote holds HEAD")
    return Window(kind, before, WORKING)
  fail("no base resolves")
```

#### B.16.3 The hook run

```
hook(event):
  host = Host.detect(event)
  window = hook_window()
  at = derivation_commit(window)
  survey = cached_survey(at) or survey(at)
  count = read(state/build-blocked)
  mine = count is not None and count.session == event.session
  if count is None or (count.prompt != turn.prompt and not (host.continued(event) and mine)):
    count = Count(prompt=turn.prompt, session=event.session, builds=0, gate_blocks=0)
    if mine and not host.continued(event) and not lost_the_lock:
      write_atomic(state/build-blocked, count)     # a person's prompt opens its budget (9.3)
  count.prompt = turn.prompt                       # a continued chain keeps its record (9.3)
  failure = build(config_or(survey), changed_files(window))
  unbuilt = None
  if failure and failure.exit == 127:
    unbuilt = note("unbuilt", failure.command, failure.shell_said); failure = None
  if failure:
    write_verdict_atomic(state/turn, RED)
    if lost_the_lock: report(failure); return 0         # 6.5: another stop may be counting
    tree = tree_of(working_directory)
    if count.builds > 0 and tree == count.build_tree: report(failure, "the tree did not change"); return 0
    count.builds += 1; count.build_tree = tree; write_atomic(state/build-blocked, count)
    if count.builds > 8: report(failure, "stopped blocking after eight"); return 0
    block(derived_lines + failure + "block N of 8")
  (failed, errored, reported, told) = run_gates(config_or(survey), window, scope=changed, unbuilt)
  if failed == 0 and errored == 0:
    write_verdict_atomic(state/turn, GREEN)
    if told: tell(report)                          # systemMessage on stdout, exit 0 (9.1)
    return 0                                       # see tell() below
  write_verdict_atomic(state/turn, RED)
  if lost_the_lock: pass_through("another klin event held the state directory"); return 0
  if no_state_directory: pass_through("could not record a gate block"); return 0
  if count.gate_blocks >= 2: pass_through("the gate has blocked 2 stops"); return 0
  flagged = count.builds == 0 and host.blocked_before(event)
  if count.gate_blocks == 0 and not flagged:
    number = 1
    tree = tree_of(working_directory)              # None when git cannot hash it
  else:
    if count.gate_tree is None: pass_through("no record of the last gate tree"); return 0
    tree = tree_of(working_directory)
    if tree is None: pass_through("no record of the last gate tree"); return 0
    if tree == count.gate_tree: pass_through("the tree did not change"); return 0
    number = count.gate_blocks + 1
  count.gate_blocks = number; count.gate_tree = tree
  if not write_atomic(state/build-blocked, count):
    flags += "count-unwritable"
    pass_through("could not record a gate block"); return 0
  add_asked_atomic(state/turn, reported)           # 8.2, cleared when the stamp moves
  if host.follows_up(event) and not record_atomic(state/handed/hash(event.session), followup=hash(report)):
    report(); return 0                             # 9.1: the echo would open a turn
  block(report + "gate block " + number + " of 2")

tell(message):                                   # 9.1, ADR 0052
  if host.follows_up(event):
    handed = state/handed/hash(event.session)
    if lost_the_lock: return                       # an unrecorded message would replay
    if handed.told == hash(without_window_line(message)): flags += "told-before"; return
    if not record_atomic(handed, followup=hash(message), told=hash(without_window_line(message))): return
  deliver(message)

pass_through(why):
  report(); say(why)
  if count.gate_blocks > 0 and event.session and no_prompt_line(event.session):
    systemMessage("klin: no prompt event reached this session; klin grants no fresh gate blocks until `klin radius` runs on session start and on prompt submitted.")
    flags += "no-prompt-event"
```

`reported` is the site id (11.2) of every finding the run printed, and `told`
counts the notes a person needs even when nothing blocks: a file no grammar
read, a file measured in `before` only, and a deleted test the run let
through. Only a stop that blocks on a gate adds to `asked`, so a question the
agent never saw is asked again at the next stop that blocks. `inventory`
reads `asked` under `--hook` (8.2).

The build stamp is one record per prompt: the prompt counter it belongs to,
the number of build blocks and the tree the last build block was taken over,
and the number of gate blocks and the tree the last gate block was taken
over. The two pairs never share a field (ADR 0052). A record an older klin
wrote holds `tree` for the build tree and `gate_spent` for one gate block,
and no gate tree, so it never proves a second gate block.
A passing build does not reset the build count, so a tree that builds, breaks
and builds again inside one turn still gets eight blocks in that turn and no
more. The tree is the one 6.5 hashes for the stamp, read through an index of
the build stamp's own, `build-index`, so a build block costs one hash of the
working tree and leaves the turn stamp's first-session marker alone. `unbuilt` is one
note the runner adds to the run's notes and counts as told, so a stop nothing
blocks still tells it. The host's `blocked_before` flag is a second opinion for the first gate
block only, because after a build block that flag is true while no gate
block is spent (ADR 0004). It never proves a second gate block (ADR 0052).

#### B.16.4 Evaluate one gate

```
evaluate(check, section, window, scope):
  before = check.measure(window.before, section) if check.needs is the tree else []
  after  = check.measure(window.after,  section, before)   # only inventory reads `before`
  entries = accepted(config, check.name) + before   # accepted first: entry order breaks ties (16.5)
  (after, entries) = restrict(after, entries, scope)
  ceiling = section.ceiling            # derived from the derivation commit, pinned, dated, or none

  after = [f for f in after if ceiling is None or over(f, ceiling)]
  before_over = [e for e in entries if ceiling is None or over(e, ceiling) or e.accepted]
  return judge(after, before_over, check.ratcheted)
```

`before_over` is what makes 7.3 hold: an entry below today's ceiling is not
judged, and an entry above it is matched, so a lower ceiling never turns a
held site red. A check with no ceiling, such as `inventory`, judges every
finding and every entry, so its `missing: 0` entries stay in and a vanished
site matches its `before` entry.


`inventory` fits the same judge by ratcheting existence. Its measure of
`after` emits a finding for every site the `before` measure holds, with
`missing: 1` where `after` has no match by site and then by body hash, and
`missing: 0` where it has one. The `before` entries carry `missing: 0`, so a
vanished site is `worsened` under the one `judge` and nothing else changes.
A vanished site the run lets through, outside the hook or in a stop's
`asked` record (8.2, 16.3), has a `before` entry of `missing: 1` instead, so
the same judge holds it and the run lists it in a NOTE. A site the accepted
list names is matched like any other, ahead of the `before` entry by entry
order. A vanished site whose subject went in the same window is a NOTE, not
a finding.

#### B.16.5 Match one site

```
match_site(findings, entries, ratcheted):
  # findings are sorted by line; entries are the accepted list in config
  # order, then the `before` sites by line
  candidates = []
  for i, f in enumerate(findings):
    for j, e in enumerate(entries):
      rose     = any(f[m] > e[m] for m in ratcheted)
      shared   = count(m for m in ratcheted if e[m] == f[m])
      distance = 0 if e.accepted else abs(e.line - f.line)
      candidates.append((rose, -shared, distance, i, j))
  pairs = []
  for (_, _, _, i, j) in sorted(candidates):
    if taken(findings[i]) or taken(entries[j]): continue
    take(findings[i]); take(entries[j])
    pairs.append((findings[i], entries[j]))
  return pairs, untaken(findings), untaken(entries)
```

The judge runs this once per file and declaration text (4.4). An untaken
finding is `new`. An untaken `before` entry has no outcome. An untaken
accepted entry matched nothing (4.8). The second pass of 4.4 takes the
untaken findings and entries as its input, grouped by body hash rather than
by site, with `distance` fixed at 0. Each group sorts both sides by file and
line before it runs, so what is left of the rank order, `i` and `j`, is the
finding's file and line, then the entry's. Only an entry whose site the
`after` tree lost reaches this input (4.4).

Inserted twin. `before` holds `fn f() {}` at lines 1 and 5. `after` holds it
at lines 1, 3 and 6, all with equal values. Every pair shares every value,
so distance decides: line 1 takes line 1 at distance 0, then line 6 takes
line 5 at distance 1. Line 3 is untaken and `new`. Pairing by line order
would pair line 3 with line 5 and name line 6, which nobody wrote, as new.

Moved twin. `before` holds `fn twin()` with cc 2 at line 3 and cc 1 at line
20. `after` holds cc 2 at lines 19 and 50. Both findings share both values
with the cc 2 entry. Line 19 is nearer, so it takes that entry and is held.
Line 50 takes the cc 1 entry and is `worsened`, cc 1 to 2. Pairing by
nearest line would pair 19 with the cc 1 entry at 20 and report a function
that only moved as worse.

Stale accepted entry. `before` holds `fn f()` with cc 3 and 5 lines. The
accepted list holds it at cc 2 and 4 lines. `after` holds it at cc 2 and 5
lines. The finding shares one value with each entry. It holds against the
`before` entry and rose against the accepted one, so the `before` entry takes
the match and the site is held (7.3). The accepted entry matched nothing.
Without the first rank, entry order would give the match to the accepted
entry and report a site that got no worse as `worsened`.

### B.19 Installation and Distribution

#### B.19.0 Support status, delivery, and the words for a scope

Two separate axes place a host. Support status says who keeps an integration
working. Delivery says how klin reaches the host. A first-class host is
reached by either of two delivery mechanisms, so the axes do not collapse into
one list of integration types.

Support status:

1. **First-class integration.** A host klin maintains as part of its
   compatibility promise: Claude Code, Codex CLI and Cursor. Each has a
   built-in adapter, compatibility evidence klin owns
   (`docs/HOST_COMPATIBILITY.md`), and the native plugin and standalone routes
   klin supports for it.
2. **Custom harness integration.** A harness-specific integration maintained
   outside klin's compatibility promise, which maps its harness into the
   harness protocol (9.7, 19.4). Using the protocol does not make a host
   first-class.

Delivery and interoperability:

1. **Native plugin.** The native plugin klin ships for a first-class
   host. It is the native alternative to the standalone route: Claude Code and
   Codex install and update it, and Cursor's verified route is a local copy a
   person makes from a release tag. It carries
   the host hooks, the klin skill and a pinned wrapper that fetches a pinned
   runtime. Section 19.2.
2. **Standalone route.** The klin binary and `klin install`, the route
   documents lead with on every first-class host, because it alone gives the
   person the `klin` command. It owns explicit
   hook files a repository or a person commits or keeps, standalone skill
   placement, managed and manual installations, and every first-class host
   surface that has no native plugin. Sections 19.1 and 19.3.
3. **Harness protocol.** klin's versioned event and decision contract of 9.7,
   which a custom harness integration implements. klin states the protocol and
   ships a reference adapter for it in `harness-protocol/`; it ships no adapter
   for any particular harness.

"Standalone integration" is not a category: the standalone route delivers a
first-class integration and is not a tier of support.

A person on a first-class host takes either route, and documents lead with the
standalone route: the installer, then `klin install` (ADR 0053). A plugin is
self-sufficient. The wrapper it carries fetches the runtime, so a plugin user
installs no binary for the hooks to work. It gives the person no `klin`
command, and it MUST NOT install one: the wrapper names the command that does,
once (19.2). A custom harness integration uses the binary of 19.1 and not this
route (19.4).

No route gates a repository by itself. Under `--hook` a `klin.json` at the
repository root is the marker that the repository opted in, and a tree without
one stays silent (5.1, ADR 0028). Installing a plugin makes klin available to
every repository the host opens. It opts none of them in.

The contract uses five scope words, and no others:

- **project** — integration the repository owns, committed with it. It is
  portable with the repository wherever the host loads project configuration.
- **user** — integration for one person on one machine. It is not portable
  with the repository, and it MUST NOT be described as reaching a cloud or
  remote agent.
- **plugin** — a first-class integration the host manages and klin maintains.
- **managed** — host policy or configuration an organization controls.
- **custom** — a harness that is not first-class and implements the harness
  protocol of 9.7.

`global` is too broad a word for a user-scope install, so this contract does
not use it for one. The CLI uses the same word: `klin install --user` writes
the user scope, and no flag is called `--global`.

#### B.19.1 The binary

A protected release pull request prepares version `X.Y.Z`. It MUST pass the
required `quality / gates` check and merge to `main` before `vX.Y.Z` is
created on that exact merged commit. The tag builds the binary for macOS and
Linux, on x86_64 and arm64, and attaches the four archives, a `.sha256`
beside each one, a `sha256.sum` over all of them, and the install script to a
GitHub release. cargo-dist creates that release and makes it Latest after the
tag build succeeds; there is no separate prerelease/promotion phase.
`releases/latest` therefore names the last successfully published release.
`dist` runs that pipeline, so its artifact names and its install script are
what a route consumes. ADR 0026 and ADR 0029 record that choice.
`klin --version` prints the version the binary was built from, which is the
tag without its `v`. Every route below downloads from that release and MUST
verify the checksum.

Two routes ship:

1. The install script the release carries, `klin-installer.sh`, run through
   `sh`. It detects the platform, verifies the checksum it was generated
   with, and puts `klin` and `klin-update` in `~/.local/bin`, or in the
   directory `KLIN_INSTALL_DIR` names. Each release carries the script that
   installs that release, so a URL under a tag pins a version.
2. A GitHub Action, `action.yml` at the root of this repository, that
   installs a pinned version and runs `klin gate --strict`. For CI only. A
   workflow pins it by the release tag, `uses: brajevicm/klin@vX.Y.Z`, so one
   tag names the binary, the plugin and the Action.

The plugin wrapper of 19.2 is not a third route into this list. It fetches the
same release for the plugin alone, into the plugin's own cache.

No other channel ships. There is no Homebrew tap, no npm package and no
published crate, and this document MUST NOT print an install command for one.
Each of them is a distribution channel with its own release obligations, and
none is required to make klin work on a supported platform, so each stays
deferred (#64). #315 ships Homebrew and npm once releases are public. A future
channel is added here only once it ships.

The supported binary targets are macOS and Linux, on x86_64 and arm64. The
plugin wrapper resolves that same set, so the public contract and the release
targets stay one list. On Linux the install script refuses a glibc older than
the one on the runner that built the binary, which is 2.35 on `ubuntu-22.04`.
klin ships no musl build. Native Windows is not a supported target: klin
builds no Windows binary and ships none. WSL is not a documented supported route
either, because klin has no compatibility evidence for it. A person on Windows
has no shipped klin install today.

#### B.19.2 The first-class plugins: Claude Code, Codex CLI and Cursor

One plugin directory serves all three hosts. It holds `hooks.json` with the
three hooks of 9.2, Cursor's flat `hooks/cursor.json` with the same three
commands on Cursor's event names, one skill that tells the agent how to read a
failure and what it may not touch, two slash commands that run the gates and
list them, and a `bin/klin` wrapper. Each host's manifest names its hooks
file, so none of them has to find it by convention. Cursor's manifest MUST
name `./hooks/cursor.json`, because the default `hooks/hooks.json` is Claude
Code's nested shape. ADR 0045.

The plugin is self-sufficient. It carries the hooks, the skill and the pinned
wrapper, and the wrapper fetches the pinned runtime on first run. A plugin
user MUST NOT be told to install the standalone binary of 19.1 to make the
plugin work. The one exception is the managed case below, where the host gives
the plugin no usable `bin/`.

**Claude Code.** klin maintains a native Claude Code plugin, the host-managed
route. The project route of 19.3, under `.claude/`, stays the portable and
manual alternative, and it is what a person takes where user settings are not
available. A plugin a person installed, and the settings that enable it, live
on that person's machine. This document MUST NOT claim that they exist in a
remote or cloud environment.

**Codex CLI.** klin maintains a native Codex plugin for the Codex surfaces
that load plugins. Codex CLI reads the same manifest and the same `hooks.json`
shape. It substitutes the literal `${CLAUDE_PLUGIN_ROOT}` into a plugin's hook
line, and the hook lines do not rely on the variable being exported, because a
shell default form such as `${CLAUDE_PLUGIN_ROOT:-}` was left unsubstituted
and expanded to nothing. Claude Code exports the variable and reads the bare
form the same way, so the hook lines name the plugin root in that form and no
other, and the same lines run on both hosts. Codex finds the plugin through a
marketplace file of its own at `.agents/plugins/marketplace.json`, which names
the same plugin at the same release tag as Claude Code's
`.claude-plugin/marketplace.json`. The install is `codex plugin marketplace
add brajevicm/klin` and `codex plugin add klin@klin`. Installing a plugin does
not trust its hooks: Codex skips an untrusted plugin's hooks until the person
reviews and trusts the current hook definition through the CLI `/hooks`
surface, and a fresh session then runs them, so the install documentation
names both steps. The Codex IDE extension's contract loads no plugins, so
klin's plugin support MUST NOT be described as covering it; that surface takes
the standalone route of 19.3.

**Cursor.** klin maintains a native Cursor plugin. Cursor finds it through
`.cursor-plugin/marketplace.json` at the repository root, which points at the
same directory. Cursor documents only a path source, so this entry names the
directory in the marketplace's own tree and no tag. Cursor Teams import that
repository under Dashboard → Plugins → Team Marketplaces. A person without a
team marketplace copies `plugins/klin` from the release tag the manifests pin
to `~/.cursor/plugins/local/klin` and
reloads the window, which is a user-scope install for that machine alone. A
copy made again from that tag takes a released plugin and never a wrapper from
an unreleased branch (ADR 0029). The copy instructions a document gives MUST
be idempotent: a second run leaves one usable copy and never nests one plugin
inside another. The Team Marketplace import has no recorded verification
(`docs/cursor-compatibility.md`), so a document MUST label it as such rather
than present it as a verified route. Cursor skips a symlink whose target sits
outside that folder. The Cursor hook lines name
`${CURSOR_PLUGIN_ROOT}/bin/klin` in that form and no other, the way Claude
Code and Codex name `${CLAUDE_PLUGIN_ROOT}`. Cursor expands both variables. A
project `.cursor/hooks.json` (19.3) stays the portable and manual route, and
it is the route for an environment that loads a repository's hooks but not a
person's own. A user-scope Cursor hook or skill is local to that machine. This
document MUST NOT call it global, and MUST NOT imply that it reaches Cursor
Cloud Agents.

**The release a marketplace installs.** Claude Code and Codex read a
marketplace from the repository's default branch, and a relative path there
copies the plugin as `main` holds it. Their two entries MUST therefore name
`plugins/klin` at the release tag `vX.Y.Z` that the manifests pin, through the
`git-subdir` source both hosts document. Each entry MUST spell the path as its
host documents it: `plugins/klin` for Claude Code, `./plugins/klin` for Codex.
A plugin installed from either marketplace then holds the files of the release
its manifest names, as long as the tag does not move, and a commit to `main`
that changes `plugins/klin` reaches a plugin user only with the next release.
The generated release pull request rewrites the `ref` with the manifests.
It MUST differ from its `main` base only by the configured release-version
substitutions, including the crate/lock versions, the two manifest versions,
the two marketplace refs and the two README pins; file modes and all other
bytes MUST remain unchanged. The required `quality / gates` check MUST prove
that exact transformation and MUST run `dist plan` before the release PR may
merge. After merge, `publish-release` MUST revalidate the transformation,
MUST create `vX.Y.Z` only on that exact merged commit, and MUST NOT move an
existing tag. cargo-dist then creates the GitHub Release and makes it Latest
after the tag build succeeds.

The host compatibility evidence in `docs/HOST_COMPATIBILITY.md` is
independent of this ordinary release ceremony. Manual Claude Code, Codex or
Cursor verification is required only for host-sensitive changes or when that
evidence is stale, red or inconclusive; a version-only generated release PR
does not require a manual host smoke. CLI/static release tests fail when a
marketplace `ref` disagrees with the crate version, when the configured
release rewrites anything outside the intended version fields, when exact
release validation can ignore non-version bytes or file modes, or when
`cargo-release` can push or tag. ADR 0029 records the decision.

The pre-tool matcher names the union
`Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*` of the tools
Claude Code and Codex CLI emit, so the guard reads edits and MCP calls on both
hosts. A name missing from this matcher leaves that host's corresponding tool
call unguarded; a name the host never emits is dead text. `klin install` uses
the same union for both hosts, keeping the standalone route aligned with the
plugin. ADR 0030 records the decision.

The wrapper is a shell script. It reads the version from the plugin manifest
beside it, so the plugin carries one pin. On first run it downloads that
release into `~/.cache/klin/bin/<version>/klin`, verifies the checksum, and
executes it. That install removes every other version that no session ran for
seven days: another host's plugin may pin another version into the same cache,
and removing it at once would make the two fetch in turn. A `radius` run
touches its version to mark it used. Every later run executes the cached
binary with no network call. When the download fails, the wrapper runs a
`klin` that PATH resolves if there is one, and otherwise prints one line
saying so and exits 0, so a turn is never blocked by a missing network. This
is the one place klin touches the network, and it is install, not measurement.

The plugin never installs a `klin` command for the person (19.0). Once per
machine, at the first `radius` run that printed nothing, the wrapper says in
one `systemMessage` that the CLI exists and names the command that installs
it. The hint is never a `followup_message`: Cursor submits a stop's
`followup_message` as the next prompt, which would hand the installer to the
agent. It says nothing where PATH resolves a `klin` other than the wrapper
itself, and nothing under Cursor, which shows no message at a prompt. A file
beside the cache records that the hint was given.

Every line the wrapper or a hook prints on exit 0 is a JSON object with a
`systemMessage`. A notice carries `followup_message` with the same text too,
except a notice that names an install command, which carries `systemMessage`
alone for the reason above. Claude Code and Codex show `systemMessage`.
Cursor's native stop shows `followup_message`. Codex rejects plain text on a
Stop that exits 0, and Claude Code writes it to the debug log alone.

Each hook line runs `${CLAUDE_PLUGIN_ROOT}/bin/klin` when that file is
executable, and otherwise the `klin` that PATH resolves. Claude Code appends
every installed plugin's `bin/` to the end of PATH, for hooks as for the Bash
tool, so a binary an installer left in `~/.local/bin` would win over the
wrapper if the hooks called `klin` by name. Calling the wrapper by its path
makes the version the plugin pins the one Claude Code runs. Where `bin/` is
unavailable, which is the managed case of a plugin distributed through
organization settings, the hooks find `klin` on PATH from a route in 19.1, and
the skill says which command installs it. When neither is present the Stop
hook says so once and lets the turn end.

The hook lines resolve the binary before they run it, because a person may
install the plugin where no binary resolves yet. With neither the wrapper nor
a `klin` on PATH the session start, the prompt and the pre-tool events say
nothing and block nothing. The Stop hook names the install command in a
`systemMessage` on stdout, and only where a `klin.json` resolves at the
project root, so a tree that never opted in stays silent (ADR 0028). Nothing
blocks, so the turn ends at that stop and the line appears once. Cursor shows
no `systemMessage` at a stop and submits a `followup_message` as a prompt, so
the Cursor line's notice is written and not shown.

The wrapper reads two overrides, `KLIN_RELEASE_BASE_URL` and `KLIN_CACHE_DIR`.
They exist so a CLI test can fetch a release of its own over `file://` and
prove the two paths that a real release cannot: the first run that installs,
and the failure that installs nothing.

Installing the plugin is the whole install of the hooks for the host. It is
not an install of the `klin` command, and it is not the install for a
repository: no `init` runs, and the repository opts in through its own
`klin.json` (5.1). Once it has one, the first stop is gated.

This reverses ADR 0002. Its first reason, a version pin beside committed
baselines, went with ADR 0009. Its second reason is handled by the PATH
fallback above. A version difference between the wrapper's binary and a CI
binary is a NOTE per 5.2, not a failure.

#### B.19.3 The standalone route: `klin install`

The standalone route is the one documents lead with (19.0, ADR 0053, ADR 0055,
ADR 0056). On this route the binary comes from 19.1, and `klin install` is the
one command that installs and repairs the integration. It serves the person who wants the
`klin` command, a team that wants hooks committed and covered by CODEOWNERS,
and a host surface that loads no plugin. A plugin user may take it later for
the command, and the hooks it commits serve a teammate without the plugin. ADR
0046 records the command.

`klin install` does three things in one run: it opts the repository in, it
selects the hosts to serve, and it reconciles the explicit hook files klin
owns.

**The repository opt-in.** Inside a git repository, `klin install` writes the
`klin.json` marker at the **repository root** that `git rev-parse
--show-toplevel` names, and not at the directory the command was run from. So
a run inside `repo/apps/web` writes `repo/klin.json`. A marker that already
exists is a person's, and its content MUST be kept whole. Outside a repository
the project form is refused and names the `--user` form; `--user` outside a
repository opts no repository in and says so. `--user` MUST NOT write a
`klin.json` beside the home directory.

**Host selection.** A supported host is a candidate when `--host NAME` names
it, when the scope holds that host's own configuration directory, or when klin
can prove that host's native plugin is enabled for that scope. A marker
directory is evidence of the host and never of the install. Where several
hosts are provable, every one of them is reconciled unless `--host` narrows
the run. Where a repository holds no host's configuration directory and no
`--host` is given, every first-class host is reconciled, and the run says so
and names `--host`: a repository serves a team whose hosts klin cannot see,
and a hook file for a host nobody runs does nothing. A plugin the person's
home enables does not narrow this, so what the repository gets does not
depend on who runs the install (ADR 0056). Under `--user` a home that proves
no host is refused, and the refusal names the supported `--host` values. `--host` may be named again for a second host.

**A plugin beside the committed hooks.** A selected host whose native plugin
already supplies klin's hooks still receives its explicit entries and the
skill, and the run names the file that proves the plugin and says that on this
machine the committed copy yields on each event the plugin took first. The
plugin registers the same host events, so the host runs both copies, and 9.8
makes one of them take effect. A teammate without the plugin gets the
committed hooks, and a person who removes the plugin keeps them with no second
install. Each host's adapter knows where that host lists its enabled plugins.
Claude Code lists them under `enabledPlugins`
in its settings files: for a repository write klin reads the repository's, the
local ones beside them and the user's, and for a user write the user's alone,
because a plugin one repository enables gates that repository and not the
machine. Codex CLI lists them as `[plugins."klin@<marketplace>"]` tables in
`config.toml`, on unless the table says `enabled = false`, and klin reads the
repository's and the user's the same way. Cursor's documented local layout is
`.cursor/plugins/local/<name>`, and Cursor 3.20.21's observed marketplace
cache is `.cursor/plugins/cache/<marketplace>/<plugin>/<revision>`. klin
reads `.cursor-plugin/plugin.json` named `klin` at exactly those two depths,
under the project and the user's home, and a klin manifest anywhere else
under `plugins`, such as a marketplace's own source, is not an installed
plugin. Cursor records no enabled state klin can read, so the run says that
Cursor may load the copy it names. A repository write beside a user file that
already holds klin's entries is written the same way, and the run names that
file.

**Reconciliation.** The files klin writes are:

- Claude Code's `.claude/settings.json`, with `SessionStart`,
  `UserPromptSubmit`, `PreToolUse` and `Stop`. `PreToolUse` is the one entry
  that carries a matcher, the union of 19.2.
- Codex CLI's `.codex/hooks.json`, in the same nested shape and on the same
  event names, at turn scope.
- Cursor's `.cursor/hooks.json` at schema version 1, with `sessionStart`,
  `beforeSubmitPrompt`, `preToolUse`, `beforeShellExecution`,
  `beforeMCPExecution` and `stop`. `preToolUse` is the one entry that carries
  a matcher, `Write|Edit|Delete`. A shell or MCP event names no tool, so a
  matcher there MUST NOT be written: it would match nothing and gate nothing.

For each selected host the run MUST bring klin's own entries to the canonical
contract above, and not merely add what is missing:

- a command klin owns is one that runs the klin binary on `radius`, `guard`
  or `gate`. A command that runs another klin command, and a command that only
  mentions klin, is a person's. Claude Code and Codex nest several commands
  under one entry, so ownership is judged per command and never per entry: a
  person's command that shares an entry with klin's MUST survive, with its own
  text, when klin's command is taken out of that entry.
- a missing canonical entry is added;
- a klin-owned entry whose command, matcher or event is stale is replaced,
  including a matcher from before #214;
- a second klin-owned entry on one event is removed, because two copies run
  the lifecycle twice;
- a klin-owned entry on an event klin no longer writes is removed, and an
  event that removal empties goes with it;
- every command that is not klin's keeps its text and its order, and an entry
  left holding no command at all goes;
- one klin hook in the file MUST NOT be read as a complete install: a partial
  install is repaired.

This is how a person on the binary route receives a later fix without deleting
a hook file by hand.

Each line klin writes resolves `klin` on PATH before it runs it and ends the
hook when none resolves, the way the plugin's own lines do (19.2). After PATH
it looks in `~/.local/bin`, where the install script of 19.1 puts the binary,
because a host started from the terminal that ran the installer has no such
PATH yet (ADR 0056). A `klin` that PATH resolves still wins. A person
who never installed the binary, or who removed it, sees nothing rather than a
failed hook on every event. The one exception is the stop of a repository that
holds a `klin.json` at its Git root, which the line resolves with `git
rev-parse --show-toplevel` because a session may start below the root: there
the line names the install command in a `systemMessage` alone, so a teammate
who cloned the committed hooks learns what they are for. Cursor shows the
person no stop field that is not also a prompt, so on Cursor the notice is
written and not shown (`docs/cursor-compatibility.md`). A host whose klin
plugin is enabled gets the committed hooks and the skill all the same, and the
run says that the committed copy yields on this machine (above, 9.8). Cursor
runs the hooks in Claude Code's settings files by default, beside its own
(`docs/HOST_COMPATIBILITY.md`), so a repository that commits klin's hooks for
both hosts runs two copies in Cursor, and 9.8 makes one of them take effect
there too. Codex
skips a project hook file's hooks until the person trusts them through
`/hooks`, as it does a plugin's (19.2), so a document that gives the
standalone route for Codex names that step.

**Standalone skill.** The standalone route writes the exact text authored at `plugins/klin/skills/klin/SKILL.md`; the binary embeds that source so the plugin and standalone copies cannot drift.

At project scope the selected hosts receive:

- Claude Code: `.claude/skills/klin/SKILL.md`
- Codex and Cursor: `.agents/skills/klin/SKILL.md`

At user scope, `klin install --user` writes the corresponding paths under the person's home directory: `~/.claude/skills/klin/SKILL.md` for Claude Code and `~/.agents/skills/klin/SKILL.md` for Codex and Cursor. Codex and Cursor sharing a path produce one planned write and one output line. A native plugin that serves the selected host and scope carries the skill too, and klin writes the standalone copy all the same, so a teammate without the plugin has it.

Skill targets participate in the same preflight as hooks. A missing file is written, a byte-identical file is already current, and a different existing file is an explicit conflict that is never overwritten. A later binary may reconcile an older standalone file only when klin can prove it owns that file; without that proof, the different file is preserved and refused. The conflict is found before the marker or any host integration is written. Rerunning `klin install` is the reconciliation step after a binary update.

The standalone route copies the skill only. Slash commands and other host-specific command surfaces remain plugin-owned. User scope is local to one machine and does not reach a cloud or remote agent.

**Preflight and partial failure.** One run may touch several files. It MUST
resolve the repository root, the selected hosts, the plugins beside them, every
target path and every host file's shape before it writes anything, so a
deterministic error leaves every file as it was. Each owned file is written
whole, through a neighbour and a rename, so a run that dies partway leaves the
file it found. It follows a path that is a link, so a settings file kept in a
dotfiles tree stays a link, and it keeps the permissions the file had. Where a
filesystem failure still happens after the first write, the output MUST name
what was written and what was not, and the run MUST NOT print a plain success.

**Scope.** The default is project scope: the files are committed, a teammate
who clones gets the hooks, and CODEOWNERS SHOULD cover them. `--user` writes
the host's user-level file instead — `~/.claude/settings.json`,
`~/.codex/hooks.json`, `~/.cursor/hooks.json`. That write covers every
repository the person opens on that machine, it is not committed, it does not
travel with the repository, and this document MUST NOT claim it reaches a
cloud or remote agent. A person who chooses it accepts that an agent can
remove the lines. No flag is named `--global` (19.0).

**Output.** A run prints one line per component: the repository marker, and
each host klin knows with what happened to it — reconciled, already current,
or not requested — and, where a plugin or a user file also serves the host,
that the committed copy yields on this machine. The marker's own line says to commit
it, because the repository's opt-in travels with the repository at either
scope. One closing line follows, and it speaks of the host files alone: a run
that wrote none of them MUST NOT tell a person to commit hooks, and MUST say
the integration is already current. So a second run over a complete
installation writes no file and says exactly that. Raw host configuration is
not printed unless there is an error.

**Guard.** The hook files are not guarded (9.4), but `klin install` writes a
person's configuration and integration files, so the guard refuses the command
from an agent exactly as it refuses `klin init` and `klin turn reset`.

#### B.19.4 A custom harness

A harness klin does not maintain integrates through the harness protocol of
9.7: klin's own versioned event on stdin, klin's own decision on stdout, and
the same three commands. It supplies the binary from 19.1 and translates its
own lifecycle into that protocol. klin ships no adapter, no hook file and no
skill placement for such a harness, `klin install` connects none, and a
custom integration is not part of the compatibility promise that covers the
first-class plugins. The protocol is a
portability seam, not a second engine: a protocol event normalizes into the one
internal event of 9.1 and runs the same turn, guard and gate loop.

Claude Code, Codex CLI and Cursor are first-class and MUST NOT route through
the harness protocol in production. Each keeps its built-in adapter, its native
plugin, its compatibility tests and its own documentation. A custom harness
integration stays custom until a separate ticket promotes the harness: that
ticket proves the host's current official semantics, adds a built-in adapter or
native plugin, adds adversarial compatibility fixtures, defines install, update
and trust behavior, and moves the harness into the first-class matrix.
Speaking the harness protocol alone MUST NOT be described as first-class
support.

**The protocol directory and the guide.** `harness-protocol/` carries the two
schemas, one fixture per event kind and `reference-adapter.sh`, a minimal
reference of the protocol boundary that integrates no host;
`docs/HARNESS_INTEGRATION.md` carries the worksheet, the lifecycle mapping and
the conformance levels. The two MUST be enough to port a harness without
reading klin's Rust adapters. The reference adapter stays small and
dependency-light: it demonstrates the translation and is not a second
supported host runtime. `harness-protocol/` MUST NOT hold a copy of the skill.
klin's canonical skill is the one authored text of 19.3, and the guide points a
custom integration at it.

**Conformance levels.** A custom integration states its level, and states what
is missing rather than claiming equivalence with a native one.

- **Full** — session or prompt lifecycle, pre-tool interception with proven
  evidence, an end-of-turn hook, and a block that returns the report to the
  agent. One difference from a first-class host remains: klin's `ask` has no
  channel this protocol can prove, so the ambiguous class of 9.4 is refused
  (9.7).
- **Gate** — an end-of-turn check that reports a failure, missing one or more
  of the pre-tool and turn-feedback capabilities. The missing ones are named.
- **Manual / CI** — no reliable lifecycle hook. A person runs `klin gate`, and
  CI runs `klin gate --strict`. There is no same-turn feedback contract, and
  none is claimed.

#### B.19.5 CI

The Action of section 12.3 installs the pinned version and runs `klin check`
over the workflow's own checkout, which needs `fetch-depth: 0` for the base
commit to resolve. An `args` input appends selectors or flags such as
`--changed`. The version comes from the `version` input, then the tag the
workflow pinned the Action at, then the latest release. The Action never reads
`klin.json`, which holds no version (ADR 0040). The Action runs the install
script under that release's tag URL, and that script verifies the checksum, so
a mismatch fails the job before any gate runs. A workflow without the Action
runs the same script and the same command.

#### B.19.6 Upgrades

A new klin version may change a measurement. Under the base commit model both
trees are measured by one binary, so an upgrade changes nothing about any
verdict except where a new check applies. A new derivable check runs on the
first stop after the upgrade, against a derived ceiling from the base tree,
so it is green on arrival. The plugin pins its own version and upgrades when
the plugin does, through `/plugin marketplace update` or the host's
auto-update. Every other route upgrades when the person asks. `klin update`
runs the `klin-update` beside the binary, or the one PATH resolves, which
installs the Latest release of 19.1 over the current one, and its exit code is
the updater's. Where no updater is found, `klin update` says so, names the
installer, and exits 2. ADR 0029 records that one tag names every route.
