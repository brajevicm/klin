# klin vNext Core Specification

Status: accepted by the owner on 2026-10-06 (#493), after seven adversarial
reviews and the frictionless revision, with the known limits of section 18.7.
This document is the authoritative target contract for klin vNext. The
shipped 0.x binary keeps `docs/SPEC-0.x.md` as its contract until the
roadmap migrates each part (section 0.3, ADR 0066).
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

vNext does not restate every measurement rule of the 0.x specification. The
sections below of `docs/SPEC-0.x.md` stay normative for vNext, except where a
section of this document amends them. A row that names no amendment carries
the 0.x section unchanged.

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
| 11.1, 11.2 | Text and JSON | Replaced by sections 11.3 and 11.7. |
| 11.3 | SARIF output | Not shipped. A future additive output. |
| 11.4 | Journal record | Replaced by section 13.1, which keeps schema 1 lines readable. |
| 11.5 | `klin stats` | Replaced by section 13.2. The counted unit carries forward. |
| 12 | Determinism | Carried forward. |
| 13 | Performance budget | Carried forward. Section 14 amends it. |
| 14 | Failure model | Replaced by section 15. |
| 15 | Trust model | Replaced by section 16. |
| 16.1 | The hook's window | Carried forward. Section 6.6 amends it: no `turn_reset`, no forced RED, the verdict table, advisory windows, and the fallback kept only without a remote. |
| 16.4, 16.5 | Reference algorithms | Carried forward. Section 8.4 amends 16.5. |
| 16.2 | `klin gate` and CI window | Carried forward. Section 6.5 amends it. |
| 16.3 | The hook run | Carried forward. Section 10.4 amends it. |
| 17, 18 | Test matrix, checklist | Replaced by section 19. |
| 19.0 to 19.5 | Installation and distribution | Carried forward. Section 17 amends the command names. |
| 19.6 | Upgrades | Carried forward. Section 8.3 amends it. |

The final roadmap step folds every carried-forward section into this
document and retires `docs/SPEC-0.x.md`.

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

Each row is an ADR that this document changes. ADR 0066 records the split
between this document and the 0.x text. The other rows take an ADR amendment
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
   every capability (0.x 4.3, ADR 0038).
3. **Window** chooses the two trees that a run compares and names the
   derivation commit (0.x section 6, section 6.5).
4. **Catalogue** is the one ordered table of capabilities. Each row declares
   its placement, activation, needs, semantics version and labels (section
   8.2).
5. **Engine** runs the selected capabilities over the window. It returns one
   semantic result: findings, review items, notes, measurement records and
   errors (section 7). The engine is the only place that measures code.
6. **Ratchet** matches findings against the base and the accepted list and
   sorts them into new, worsened and held (0.x section 7, section 8.4).
7. **Host protocol** reads a host event through `klin __agent event`, applies
   the Stop policy, and writes the decision in the host's shape (section 10).
8. **Guard** answers pre-tool events about writes to the guarded set (0.x 9.4,
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
  0.x sections 4.1 to 4.3 define them.
- The Stop uses the `turn` window. `klin check` uses the `branch` or `push`
  window (0.x section 6, section 6.5).

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

- **Finding**: one violation at one site, as `CONTEXT.md` and 0.x 4.5 define
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
- **Turn stamp** and **prompt mark**: 0.x 6.2 and 6.2.1.
- **Build stamp**: the per-prompt block counts and the trees the last blocks
  were taken over (0.x 9.3).
- **Handoff record**: the hash of a message that a host may submit as a
  prompt (0.x 9.1).
- **Advisory Stop** (*new*): a Stop that measures and blocks nothing,
  because the turn's history moved under the stamp (section 6.6).

### 4.9 Trust levels

- **Feedback**: local hooks and a local `klin check`.
- **Enforced**: `klin check` in an independent CI checkout under the
  conditions of section 16.2.

## 5. Configuration Contract

### 5.1 The file

The rules of 0.x 5.1 to 5.6 carry forward: `--config PATH`, compact policy,
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
ignored. A `--config PATH` that names no file is exit 2.

The hooks' walk passes every directory between the event's tree and the
worktree root, and tests each for a `klin.json` without parsing it. When it
finds one below the root and none at the root, the hooks stay silent as an
opt-out, and tell one person notice per session that names the file and
says to move it to the worktree root. This replaces the 0.x support for a
configuration below the repository root, which
`a_config_below_the_repository_root_holds_the_debt_the_base_holds` and
`a_config_below_the_repository_root_scopes_a_changed_run_the_same_way` in
`tests/base.rs` pin. The roadmap ticket that implements discovery rewrites
those tests.

The file is the repository's opt-in marker for the host protocol (ADR 0028).
When the worktree root holds no `klin.json`, `klin __agent event` answers
every event with no decision, prints nothing, writes no state and exits 0. It
does not read or parse the file for this test.

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

The keys of 0.x 5.2 carry forward: `build`, `accepted`, `radius`, `journal`,
and one key per gate section. vNext adds no key and retires no key. A
configuration that 0.x accepted is valid in vNext.

- `build` is local Stop feedback policy (section 6.4). It never affects
  `klin check`.
- `accepted` holds person-authored debt (0.x 4.8). Only a person writes it,
  in a reviewed commit.
- A `sarif` entry whose `name` equals a capability name, or equals another
  entry's name, is a configuration error.

### 5.4 Pinning

`klin setup --pin` writes today's derived guardrails as policy a person
reviews, with the rules of 0.x 5.7 for `init --pin`: complexity `cc` and
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
- A full run lays the base out as a temporary worktree of its own (0.x 6.5).
  It removes the worktree and prunes its registration before it exits. A
  later run removes only a worktree whose owning process is gone, so two
  concurrent runs never remove each other's.

### 6.4 Build feedback

The build is local Stop feedback that is separate from the quality
judgement. The rules of 0.x 9.3 carry forward:

- The Stop runs the configured or derived build before it measures, so the
  agent does not receive measurements over code that does not build.
- A build failure blocks each Stop that changed the tree since the last build
  block, up to eight per prompt, and writes a red verdict first.
- A build whose shell exits 127 is an absent tool. It is a note, not a build
  failure (ADR 0048).
- Command limits, deadlines and process-group cleanup are as 0.x 9.3 states.
- 0.x 9.3's sentence that the build rule "does not constrain a future
  agent-readiness path" does not carry forward. vNext has no readiness path
  (section 18.6).

`klin check` MUST NOT run the build, MUST NOT read the build stamp, and MUST
NOT claim that the project builds. The project's own CI owns build, test,
type check and install (section 12.2). The build has no Stop time budget of
its own. Section 14 excludes it from every Stop measurement.

### 6.5 Windows of `klin check`

The base candidates of 0.x 6.3 carry forward, with these rules:

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
3. **A base equal to HEAD, with a clean working tree** (0.x 6.4):
   - a remote source passes and prints that the trees are the same;
   - a local source passes when the remote default branch holds HEAD;
   - a local source whose HEAD the remote default branch does not hold is a
     hole, `comparison-unproven`, exit 3, which names the unpushed commits;
   - with no remote at all, the run passes and prints a note that no remote
     proves what to compare. This is the 0.x behavior without `--strict`.

The "same tree MUST be green" promise of 0.x 5.1 holds with one exception: a
local source that hides unpushed commits.

### 6.6 The stamp and automatic recovery

The stamp rules of 0.x 6.2 carry forward, with these changes.

**The stamp verdict.** A Stop writes exactly one verdict. When more than one
row applies, the first row in the table wins.

| Verdict | When | Effect |
| --- | --- | --- |
| `aborted` | klin failed during the Stop. | The stamp stays until a Stop measures. It neither blocks nor shows as red. `klin status` shows since when. |
| `red` | The build failed (section 6.4), or the result holds a failing finding, or a deleted test that klin has not yet asked about (section 9.2). An advisory Stop writes `red` only for a build failure. | The stamp stays. |
| `unjudged` | A run-scope configuration error stopped the Stop before it measured. | When the stamp was `green` before, the next session or prompt moves it, so the first Stop after a fix never judges days of work. When the stamp was `red`, it stays red. `klin status` and `klin report` show "nothing judged", never green. An advisory window stays pending until a Stop measures. |
| `advisory` | The Stop measured in an advisory window (below) and the build did not fail. | The Stop itself takes a fresh stamp. |
| `green` | The engine measured, and nothing above holds. | The next session or prompt moves the stamp (0.x 6.2). |

A Stop that holds the state lock writes `aborted` before it measures and
replaces it with its final verdict, so a Stop that crashes never leaves an
earlier `green` in place. A Stop that cannot take the lock writes no verdict,
as 0.x 6.5 says.

Review items, notes, coverage notes and errors the base had too do not keep
the stamp.

**Advisory windows.** Some events bring other people's commits into the turn
or take the turn's history away, so a precise local judgement is no longer
possible. klin then degrades to reporting for one Stop. The stamp records
the merge-base of HEAD with the default branch, HEAD's symbolic ref (an
in-progress rebase counts as its `head-name` branch), and the position of
HEAD's reflog. A Stop is advisory when, since the stamp was taken, one of
these happened:

1. **Incoming commits.** The default-branch merge-base moved, and HEAD's
   reflog since the recorded position holds a `merge`, `pull`, `rebase` or
   `reset` entry. The default branch is found by candidate 3 of 0.x 6.3
   without the GitHub variables, and only a remote-tracking ref
   (`refs/remotes/...`) counts. The reflog is compared by entry position,
   never by time. When HEAD has no reflog, a moved default-branch merge-base
   alone is advisory.
2. **A branch change.** HEAD's symbolic ref differs from the recorded one,
   and the stamp's parent is no longer an ancestor of HEAD. `git switch -c`
   at the same HEAD keeps the turn window, as 0.x 6.2 did.
3. **Lost history.** The recorded default-branch merge-base is no longer an
   ancestor of HEAD, for example after the default branch was rewritten and
   the agent reset onto it.
4. **Missing state.** The stamp and its ref are both missing (0.x 6.2).

The agent's own work never triggers these rules: a commit, a push, an amend,
a `reset --soft` or an interactive rebase of the turn's commits on the same
branch leaves the default-branch merge-base and the branch as they were. The
turn window then stays precise, because `before` is the stamp tree. When such
a rewrite drops the stamp's parent from HEAD history, the stamp keeps its
tree, and the derivation commit stays the stamp's parent, which the stamp
commit keeps readable.

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
  lock, because the verdict and the stamp must describe the same tree;
- writes one `advisory` journal line with its reason: `incoming-commits`,
  `branch-changed`, `history-lost` or `stamp-missing`.

Because the advisory Stop takes the fresh stamp itself, the next Stop is
ordinary, also within the same prompt, in a host continuation, and on a
host whose prompt hook does not run. The prompt counter carries on.

**Repositories with no remote.** When no `refs/remotes/*` ref exists, no CI
can judge what an advisory Stop would skip, so such a repository takes no
advisory Stop. Rule 1 cannot fire. For rules 2 and 4 it keeps the 0.x 6.2
branch fallback: the Stop judges a branch window, blocks as any Stop does,
writes the verdict its gates gave, and keeps `asked`, `told` and
`intervened`. The fallback's base is the stamp's recorded parent while that
commit is still readable, and the 0.x 6.3 base otherwise. Its journal
outcome stays `branch-fallback`. When remote refs exist but the default
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
- A stamp with no recorded merge-base or reflog position, such as a 0.x stamp
  after an upgrade or a stamp restored from its ref, records them at its first
  Stop. That Stop is advisory when the default-branch merge-base is not an
  ancestor of the stamp's parent, because the turn may already hold incoming
  commits.
- Section 18.7 lists the cases these rules do not cover.

**Cost.** klin caches the merge-base under the pair (HEAD commit,
default-branch commit). `session`, `prompt` and `stop` read HEAD, HEAD's
symbolic ref, the default-branch ref and HEAD's reflog as files, or with one
git process where the repository stores refs in a reftable. `pre_tool` reads
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
the stamp from the ref (0.x 6.2). A person who deletes the state directory
loses the journal too. `klin status` says so beside the state directory
path.

### 6.7 State

The state directory and its contents carry forward from 0.x 7.4, with these
changes:

- The journal `reset` kind is not written. A reader still reads old `reset`
  lines. A Stop line can carry the verdict `advisory` (section 13.1).
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
  source-line ceiling of 0.x section 13 and the working tree's copy holds a
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
  and the first 10,000 lines of each `.gitattributes` file and ignores a
  pattern longer than 256 bytes. What klin ignores changes only how a form is
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
  the host's person channel, and never on a host that submits a told message
  to the agent as its next prompt. The entry is `{"gate": "measurement-lost", "file":
  PATH}`, and it matches by file alone, for every capability (an amendment
  of 0.x 4.8, which otherwise requires `text` and values). The entry counts
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

**The named 0.x holes.** Each case that 0.x section 8 names as a file or form
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
- Every "exit 2" case of the carried-forward 0.x sections 5 and 8 is one of
  the kinds above. A case that names a file klin could not measure is a
  `measurement-lost` finding, an opened gap or a coverage note of section
  7.2, not an error.
- **Moved and deleted policy paths.** A path that the policy names (an `in`
  path, a pinned document, a convention's `in`) and that selects files in the
  base and none in the working tree is decided by what happened to those
  files, by a deterministic test on both trees:
  - **Renamed.** When git detects every selected file as renamed (`-M50%`),
    the policy follows the rename for this run. The capability measures the
    new paths under the pinned values, so it stays switched on. `klin check`
    adds a review item of kind `moved-pin` that names the old and new path, so
    a person updates the pin in the pull request. No block and no error.
  - **Deleted.** When the selected files are gone and not renamed, it is a
    note at the Stop and a review item of kind `moved-pin` at `klin check`.
    No error.
  - **Mixed.** When some selected files were renamed and the rest deleted,
    the renamed files are measured at their new paths under the pin, and one
    `moved-pin` review item names the path.
  - **Selects nothing in either tree.** A pinned path that selected nothing in
    the base either, such as a pin left behind after an earlier move merged,
    is a `moved-pin` review item, not an error, so an ignored review item
    never turns into a later exit 2.
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
  move.
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

Every measurement record names its basis. The basis holds, where relevant:

- the producer: the capability name and its semantics version (section 8.2);
- the klin version that measured;
- the effective policy that the capability used, with provenance (derived,
  pinned, built-in) and the derivation commit;
- for an integration: the configured entry name, the command or report path,
  and whatever tool identity the report states;
- the selected scope and the observed coverage counts;
- the window: kind, `before` and `after`;
- the holes, each with its reason.

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
when either differs. They are **unknown** when either side does not record a
semantics version, such as a journal line that 0.x wrote. A policy change,
such as a derived ceiling that moved, is reported, and it does not decide
comparability.

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
   flagged it, no longer holds it. Not comparable and unknown are reported as
   "measurement changed; not compared". They are never read as a fix or a new
   regression. A 0.x line is never upgraded by inference.
3. **The accepted list**: an accepted entry keeps its person-authored
   meaning. An entry that stops matching after a semantics change is a review
   item (section 7.6), never a failure. The release notes of the version
   change name the capability.

This amends 0.x 19.6: an upgrade can change a verdict where a semantics
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

Among eligible pairs, the rank and the one-to-one greedy pass of 0.x 16.5
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
  holds (0.x 7.3).
- A site under a path the derivation commit did not hold matches nothing in
  `before` (0.x 7.1).
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

0.x section 12 carries forward. In addition:

- A measurement record and its basis are a pure function of the trees, the
  policy and the binary, apart from the time-dependent cases 0.x 12 names
  (dated ceilings, report age, command limits).
- An integration's result is deterministic only when its tool is. klin
  records the command it ran beside the results.

## 9. Capabilities

### 9.1 The native catalogue

The shipped checks stay, with the dispositions of #475 section 11. Their
measurement rules are 0.x section 8, amended below and by section 7.

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

The 0.x 8.2 rules for what counts as a deleted test carry forward, including
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

- the bounded execution when it launches the command (0.x 9.3 limits);
- report freshness, by deletion before `run` or by file age (0.x 8.3);
- the delta judgement on changed lines (0.x 8.3);
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
  the event's own fields (0.x 9.1, 9.7). `--host NAME` overrides host
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
- A `session` or `prompt` event adds no work beyond 0.x 6.2 and 6.5, apart
  from the merge-base that section 6.6 records with a new stamp.
- A `stop` event loads configuration only after the opt-in walk.
- The ingress adds no git process, no configuration load and no tree walk to
  any kind, compared with the 0.x command that served the same event, except
  the history check of section 6.6. At `session`, `prompt` and `stop` it reads
  HEAD, HEAD's symbolic ref, the default-branch ref and HEAD's reflog as files
  (or with one git process in a reftable repository). It starts
  `git merge-base` only when the cached pair changed, and `--is-ancestor`
  only to test rules 2 and 3 after a branch change or a moved merge-base.
  `pre_tool` does none of this.

### 10.3 Events

| Kind | Does | Blocks | Writes |
| --- | --- | --- | --- |
| `session` | moves the stamp per 0.x 6.2, raises the prompt counter, moves the prompt mark | never | stamp, mark, counter |
| `prompt` | the same as `session`, consumes a matching handoff record, and tells the radius note when the window passed its value (0.x 6.2.1, ADR 0024) | never | stamp, mark, counter, journal `prompt` line |
| `pre_tool` | the guard (section 10.8) | `deny` or `ask` | journal line for `ask` and `deny` |
| `stop` | build feedback, then the engine at placement `stop`, then the Stop policy | per section 10.4 | verdict, build stamp, handoff record, journal `stop` line |

The adapters, the event identity, the claims that let one copy act per event,
and the handoff records carry forward from 0.x 9.1 and 9.8.

### 10.4 Stop block policy

The block budget of 0.x 9.3 and ADR 0052 carries forward. In each prompt:

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

0.x 16.3 is amended to match: the hook blocks for a code FAIL and for the
ask of section 9.2, and for nothing else that the engine returns.

klin proves gate block 2 from its own record (0.x 9.3). When klin cannot read
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

The Stop keeps these 0.x 9.5 shapes, with vNext command names:

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
  expires when the window it belongs to closes, so notices never pile up.
- On the harness protocol, the notice is a `tell` decision.
- A blocking Stop on Cursor still uses `followup_message`.

### 10.8 Guard

The guard of 0.x 9.4 carries forward: `deny`, `ask` and `allow`, the proof
rule, path resolution, shell parsing and the 50 ms budget. The guarded set
stays this tree's `klin.json` and this tree's state directory. In a tree that
holds no `klin.json`, the guard answers nothing (section 5.1).

The `deny` list of commands is:

- `klin setup` in any form, because it writes `klin.json` and host files;
- `klin update`, because it replaces the binary that judges the agent;
- `klin __agent` in any form from an agent's shell, because a fabricated
  event could refresh a block budget. The host runs the hook lines itself, so
  the guard never sees them;
- the 0.x spellings `init`, `install` and `turn reset`, because an older
  binary on PATH may still run them.

A deny reason says that a person changes the file in a reviewed commit, or
names the command a person runs instead. It never names a command that
accepts debt. The guard's journal line names the matched command.

### 10.9 Harness protocol

Protocol v1 (0.x 9.7, ADR 0047) carries forward unchanged in its event and
response schemas. A custom harness invokes `klin __agent event` instead of
the 0.x commands. The `event` field supplies the kind. The `tell` decision
carries non-blocking notices (section 10.7). The fail-closed rule for an
unknown protocol version stays (section 0.4).

### 10.10 Exit codes of the ingress

On Claude Code, Codex and the harness protocol, exit 2 blocks a Stop or
denies a tool call. So:

- `klin __agent event`, and every hook spelling that legacy dispatch accepts
  (section 17.3), exit 2 only to block or to deny.
- A usage error, an unknown argument, a host event klin cannot read, a
  run-scope internal failure and a panic exit 0 with no decision at
  `pre_tool`, `session` and `prompt`, and exit 1 at `stop`. Each writes a
  notice to stderr and, where the state directory allows, a journal note. A
  capability-scope error is not one of these (section 7.3).
- A well-formed harness event with an unknown `klin_protocol` version is not
  "an event klin cannot read". It keeps the fail-closed rule of section 10.9.
- A hook spelling that a later release removes keeps these rules. It exits 0
  and tells a notice that names `klin setup` through the host's person
  channel (`systemMessage`, or `tell` on the harness protocol). It never
  falls through to a migration error with exit 2.

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
  the hosts that `--host NAME` names, with the host selection of 0.x 19.3
  (ADR 0056). It writes the hook lines of section 17.3, the skill files and
  the slash-command text, all with vNext command names.
- A klin-owned file that a person changed is a conflict. `setup` reports it
  and leaves it, as 0.x 19.3 does for `install`.
- Flags: `--host NAME` (repeatable), `--user` for one person's host files on
  this machine, `--pin` (section 5.4), `--config PATH`.
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
  `ERR:` lines under the row, as 0.x 11.1 shapes `FAIL:` and `NOTE:`. A
  coverage note is a `NOTE:` line that names the file, its reason and the
  words `not measured`;
- the `OK:` line and its coverage counts of 0.x 11.1;
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
| `legacy` | The host files hold 0.x hook lines that legacy dispatch serves. |
| `missing` | The repository proves the host, and no copy of klin's hooks is installed for it. |
| `conflict` | A klin-owned file was changed, or two copies disagree in a way 0.x 9.8 cannot settle. |

- `--json` prints the document of section 11.7.
- Exit 0 when it could read what it reports, whatever it found. Exit 2 on an
  invalid invocation, or outside a git repository.

### 11.5 `klin report`

Section 13.2.

### 11.6 `klin policy [SECTION]`

- Read-only. It derives values as a run would, and it runs no check.
- Prints the effective policy of every capability, or of the one `SECTION`
  names: activation, placement, state (`active`, `excluded`, `needs-policy`,
  `not-applicable`), and each value with its provenance (`derived` with its
  rule and derivation commit, `pinned`, dated with the step in force, or
  `built-in`). It also prints the build policy, the accepted list and the
  integration limitations of section 9.4.
- A capability whose derived policy is more than a value per key explains it
  in place of those lines. `public-api` lists each derived surface with its
  items, measured or opaque, and the packages with no supported surface (ADR
  0044). `conventions` explains each convention, or the one that
  `klin policy conventions NAME` names: what it forbids and where, what its
  code pattern reads as and how its language was settled, any `in` or
  `except` path that matches nothing, and its remedy (ADR 0037). It counts no
  match, because a count is a measurement. An unknown `NAME` exits 2.
- `--reference` prints the configuration reference of 0.x 5.8 as Markdown,
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
  compatibility policy of #344 (section 17.5).
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
`matched` (`{file, line, text, accepted, values}` or null), `remedy`
(string), and `identity` (section 8.4) when the family has one.

A review item: `check`, `kind` (`deleted-test`, `unmatched-accepted`,
`unmeasured`, `moved-pin`), `file`, `line`, `text`, `reason` (the agent's
reply for a deleted test, the gap reason for `unmeasured`, the old and new
path for `moved-pin`).

A note: `check` (or null), `kind`, `coverage` (boolean, true for a coverage
note), optional `file`, `line` and `text`, and `message`. A coverage note's
`kind` is its reason of section 7.2. Other kinds include the
0.x note outcomes that stay notes: `unmatched` at the Stop, `derivation`,
`config`.

A measurement record: `check` (null for the run), `basis` (section 8.1),
`state` (`complete`, `incomplete`), `holes` (list of `{reason, detail}`;
every hole's site is the run or the gate).

An error: `kind` (section 7.3), `check` (null for the run), `message`. A
file or form klin could not measure is never an error: section 7.2 sorts it
into a `measurement-lost` finding, a review item or a coverage note.

#### The `status` document

`schema_version`, `command`, `klin {version}`, `config {path, present,
valid, error}`, `integrations [{host, scope, route, state, detail}]`,
`state_dir`, `cache_dir`, `window {verdict, age_seconds, open [finding
ids], unasked [deleted-test sites], error, aborted_since,
default_branch, last_advisory {time, reason} or null, notices [...]}` or
null,
and `last_stop {time, verdict, historical: true}` or null.

#### The `report` document

Section 13.3.

#### The `policy` document

`schema_version`, `command`, `config {path, present}`, `derivation
{commit}`, `capabilities [{name, section, kind, activation, placement,
state, values [{key, value, provenance, rule}], limitations}]`, `build`,
`accepted`, `state_dir`.

### 11.8 `klin update`

Unchanged from 0.x 19.6. It uses the network to fetch the release. When
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

- The Action runs `klin check $ARGS` with `fetch-depth` deep enough for the
  base, and maps the exit code to the job result.
- It writes review items, holes, failing findings and the count of files
  not measured to the job summary. It writes failing findings and review
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
  file when it has none.
- It refuses a pinned klin version older than the first release that ships
  `klin check`, with a message that names that version.

## 13. Reporting

### 13.1 The journal

The journal of 0.x 9.6 stays: one JSON line per Stop, prompt and
guard `ask` or `deny`, best-effort, never pruned. A Stop that ends on a
configuration error writes a line too, so `klin status` and `klin report`
can show the error.

vNext writes journal `schema` 2:

- Every line carries `schema`, `version`, `time`, `kind` and `session`, as
  0.x 11.4 defines them.
- A `stop` line holds the `check` document of section 11.7 under `result`,
  built by the Stop's engine run. In it, `command` is `"stop"`, `window.kind`
  is `turn`, and `exit` is null, because the host's exit code is not a
  verdict. Beside `result`, the line keeps the 0.x 11.4 fields `host`,
  `prompt`, `verdict` with its `why`, `asked`, `hook`, `told`, `flags`,
  `config_hash` and `timing`, and adds `notice`: the non-blocking person
  notice, and whether a host channel delivered it or only the journal holds
  it.
- An advisory Stop's line carries `verdict: advisory` and its reason:
  `incoming-commits`, `branch-changed`, `history-lost` or `stamp-missing`.
  A repository with no remote keeps the 0.x `branch-fallback` outcome for its
  fallback Stop.
- `prompt` and `guard` lines keep their schema 1 fields.

A reader reads schema 1 and schema 2 lines. A schema 1 `stop` line keeps the
0.x reading of 11.4 and 11.5. A schema 1 `reset` line still sets aside the
regressions before it. A regression flagged on a schema 1 line and absent on
a schema 2 line is "not compared", because the schema 1 line records no
semantics version (section 8.3).

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
- A regression still open at an advisory Stop, or when an `unjudged` stamp
  moves, is `set-aside`, as after an old `reset` line, because the fresh
  stamp no longer judges it. It is never counted as fixed.
- The counted unit is the Regression of 0.x 11.5, keyed by finding `id`. A
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

## 14. Performance

### 14.1 Budgets that carry forward

0.x section 13 carries forward, with these amendments:

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
- An advisory Stop takes a full stamp capture (0.x 6.5) on the Stop path.
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
| Other capability-scope configuration or git error | No block. A notice. The other capabilities report, and a FAIL beside it spends its block. | That row `execution: error`. Exit 2. The other capabilities report. |
| No `klin.json` | No answer, no state (section 5.1). | Runs under `{}` and says so. |
| A present base candidate does not resolve, shallow history, or a missing `GITHUB_BASE_REF` | Not applicable. | ERROR, exit 2, naming `fetch-depth`. |
| A push `before` rewritten away in a full checkout | Not applicable. | Falls back to the merge-base with a note. |
| A base equal to HEAD hides unpushed commits | Not applicable. | Hole, `comparison-unproven`, exit 3. |
| Incoming commits from the default branch, a branch change, or lost history (section 6.6) | Advisory Stop, which takes a fresh stamp itself. Without `refs/remotes/*`, the 0.x branch fallback for a branch change. | Not applicable. |
| No base resolves | Not applicable. | ERROR, exit 2, naming what was tried. |
| Stamp missing, ref present | Restored from the ref, and a note says so (0.x 6.2). The Stop writes the verdict its gates gave. | Not applicable. |
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
| Unknown harness protocol version | Fails closed: refuses tool calls, blocks Stops, and names both versions (0.x 9.7). | Not applicable. |
| State directory unwritable | Reports, writes no verdict and no count, blocks nothing. On Cursor it tells nothing (0.x 9.1). | Not affected: `klin check` writes no state. |
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
ref by hand makes one Stop advisory (section 6.6). The debt it left before
that Stop is then judged only where CI runs `klin check`, and goes unjudged
where no CI runs it. In a repository with no remote, the branch fallback
judges from the 0.x 6.3 base, which on the default branch can be HEAD, so a
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
Section 17.3 covers the hook spellings.

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
| A merge of the default branch makes its code new at the Stop | That Stop is advisory, and the next prompt takes a fresh stamp |
| The branch fallback after a history move judges the whole branch, red | With a remote: one advisory Stop that takes a fresh stamp. Without a remote: the branch fallback from the stamp's parent, with the gates' verdict |

### 17.3 Generated host integrations

An updated binary MUST NOT strand an installed integration. The invariants:

1. The release that ships `klin __agent event` keeps hidden legacy dispatch
   for the generated 0.x hook lines: `klin radius`, `klin guard` and
   `klin gate --hook --changed`, each with an optional `--host`. Legacy
   dispatch routes into the same event path, under section 10.10. It is
   absent from help.
2. Plugin hooks switch to `klin __agent event` in that release, because a
   plugin pins its own binary.
3. `klin setup` writes `klin __agent event` lines into user-scope files from
   that release on, because one person's machine holds one binary.
4. `klin setup` keeps writing legacy lines into project-scope files, which a
   team commits and whose members may run older binaries, until the release
   that removes legacy dispatch. That release raises the minimum version and
   names it.
5. Legacy dispatch is removed before 1.0. After its removal, a legacy hook
   spelling still exits 0 with a notice (section 10.10).
6. `klin status` reports legacy lines as `legacy` and names the minimum
   version that reads the new lines.

The roadmap sets the release numbers.

### 17.4 Configuration migration

vNext retires no `klin.json` key. A configuration that 0.x accepted is valid
in vNext, provided it sits at the worktree root (section 5.1).

### 17.5 Inputs to the stability contract (#344)

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
   measurements and remedy as 0.x 8.6 and section 7.2 require;
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
- a complex function in a file moved out of an `in` scope that still selects
  other files still fails;
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
- legacy dispatch reaches the same behavior as `klin __agent event`;
- a usage error, an unknown argument and an unreadable event under the
  ingress never exit 2;
- an unknown harness protocol version still fails closed;
- the opt-in walk stops at the first `.git` entry;
- the guard denies `klin setup`, `klin update` and `klin __agent` from an
  agent.

CLI:

- each removed command is an unknown command and exits 2;
- `status`, `report` and `policy` write nothing to the state directory;
- `status` reports a `legacy` integration and the local window verdict;
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
- a schema 1 journal line reads as 0.x did.

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
