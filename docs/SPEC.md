# klin vNext Core Specification

Status: **draft for #493**, 2026-10-06.

This document specifies the intended core vNext product. It replaces the historical
end-state design in this file once #493 is accepted. Until then, `main` continues to
ship the pre-vNext command surface and behavior pinned by its tests and accepted ADRs.

The specification deliberately separates the stable core contract from detector-specific
research. In particular, code-duplication semantics remain owned by #478 and its child
research. A future duplication capability must plug into the contracts defined here; it
does not get to redefine the public CLI, lifecycle, trust model, result algebra, or
generic evidence rules.

The structure follows the Symphony-style product specification requested by #358.

## Normative language

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, RECOMMENDED, MAY and
OPTIONAL are to be read as RFC 2119 describes them.

"Core" means a semantic product contract this specification freezes. A physical cache
layout, parser ownership graph, private-index algorithm, lock implementation, or other
implementation mechanism is not core merely because the current binary uses it.

When this specification differs from the current shipped implementation or an older ADR,
this specification is the vNext target. Migration and implementation work may land in
steps, but a partially migrated binary MUST NOT claim the vNext contract until its
observable behavior satisfies the relevant sections.

`CONTEXT.md` and historical ADR vocabulary describe the current/pre-vNext product until
their migration work lands. Where their old public terms (`gate`, `turn`, `radius`,
`stats`, lifecycle flags) conflict with this document's vNext surface, this document is
the vNext authority. The implementation roadmap must reconcile those companion docs
rather than carrying both vocabularies forward indefinitely.

## 1. Problem statement

Coding agents can produce locally plausible changes while taking cheap routes around the
repository's quality intent: increasing existing complexity, suppressing safety checks,
skipping or deleting tests, leaving placeholders, breaking public contracts, introducing
dead or unreachable code, or bypassing repository architecture.

klin exists to make a narrow class of those regressions deterministic and repairable.

The product is **deterministic quality control for coding agents**:

> klin ratchets repository-grounded invariants on the change, gives the coding agent
> concise repair feedback while it works, and lets an independent CI checkout enforce
> the same quality policy.

klin is not a general "AI slop" classifier. It does not try to infer authorship,
aesthetic quality, generic SOLID/design-pattern compliance, or one universal measure of
code quality. Research in #352–#357 and #361–#364 showed that broad phenotype coverage
does not justify the precision, repair, attention, operational, and implementation cost
required for a product guarantee.

The optimization target is the complete loop:

```text
detect
  -> explain
  -> repair
  -> verify repair
```

A detector that classifies code well offline but causes harmful repairs, cheap
appeasement, repeated agent turns, or unnecessary human interruption is not automatically
a useful klin capability.

## 2. Product goals and non-goals

### 2.1 Goals

klin vNext MUST:

1. compare a before tree with an after tree and make existing debt held rather than newly
   red;
2. provide fast, bounded, repair-oriented feedback at the agent Stop boundary;
3. provide one explicit human/CI quality command, `klin check`;
4. make an independent CI checkout the enforcement boundary for klin policy;
5. keep project build, type-check, test, and install ownership with the project's own CI;
6. use one semantic measurement/result engine for Stop and `check`;
7. keep code judgment, measurement completeness, and execution state distinct;
8. keep missing, unsupported, exhausted, stale, or failed measurement visible rather than
   presenting it as clean;
9. compare evidence only when its semantic measurement basis is compatible;
10. allow finding identity only where a family has a proven, versioned identity contract;
11. keep `{}` a meaningful complete policy configuration;
12. keep the public CLI at the six intent-level commands in section 5;
13. hide host lifecycle machinery behind one `klin __agent event` ingress;
14. support Claude Code, Codex CLI, and Cursor as first-class integrations without
    requiring humans to invoke routine commands after setup;
15. remain useful offline for klin-owned native measurement;
16. keep ordinary Stop within the performance contract in section 16;
17. expose machine-readable, schema-versioned output for `check`, `status`, `report`, and
    `policy`;
18. give a developer enough evidence to reproduce a disputed result without exposing the
    entire internal cache/state model as public API; and
19. admit future capabilities only through the extension and product-evidence rules in
    section 19.

### 2.2 Non-goals

klin vNext MUST NOT require or imply:

- AI-authorship detection;
- a universal AI-slop, oracle-strength, maintainability, architecture, or quality score;
- generic GoF/SOLID classification;
- a universal CodeGraph or repository-wide semantic index as a prerequisite;
- executing the project's authoritative build/test/typecheck as part of `klin check`;
- a model-invoked readiness/finalization phase;
- PostTool quality hooks;
- a web service, dashboard, telemetry backend, or network service for native checks;
- perfect static knowledge of dynamic/framework behavior;
- making contextual REVIEW evidence disappear before an agent can hand control back;
- forcing agents to pay down inherited debt they did not worsen; or
- making local hooks tamper-proof.

Counterfactual simplification, broad test-strength judgment, design/reuse conformance,
named analyzer recipes, and broad unfinished/error-masking detection remain deferred or
research-only unless a later targeted admission decision satisfies section 19.

## 3. System overview

The core product has seven semantic components.

### 3.1 Public command surface

Humans and CI use exactly these public intents:

```text
klin setup
klin check [CHECK...]
klin status
klin report
klin policy [SECTION]
klin update
```

Flags may narrow scope or representation; they MUST NOT turn one command into another
lifecycle phase.

### 3.2 Hidden host protocol

Installed host integrations invoke one hidden entry point:

```text
klin __agent event
```

It normalizes host `session`, `prompt`, `pre_tool`, and `stop` events. It is not shown in
ordinary `klin --help` and is not a human workflow.

There is no `klin __agent ready` operation in this version of the lifecycle.

### 3.3 Measurement engine

The measurement engine evaluates selected capabilities over one Window, under one
effective policy and one coherent measurement basis. Stop and `check` use this same
semantic engine. They differ in placement, window/source, rendering, and authority; they
do not have separate definitions of a finding.

### 3.4 Native capabilities

Native capabilities are product-owned deterministic measurements implemented by klin.
Each capability declares product-owned placement (`stop` or `check`), policy shape,
selection semantics, measurement-basis semantics, and result identity.

Repository configuration MAY enable, disable, or parameterize policy where a capability
contract allows it. Repository configuration MUST NOT move a native capability between
Stop and `check`.

### 3.5 External integration evidence

User-owned external integrations, including SARIF producers, run only under explicit
`klin check` in the core lifecycle. Their command, tool version, analyzer configuration,
and detector semantics are project-owned unless a future named klin-owned recipe is
separately admitted.

No external integration process runs automatically at ordinary Stop.

### 3.6 State and journal

Local integration state exists to make bounded feedback and reporting work. It is not an
enforcement database. State loss, corruption, or lock contention MUST NOT create extra
quality blocks or fabricate a clean verdict.

The journal records enough semantic outcomes for `klin report` and for truthful
pass-through notices. Its physical file format is internal unless explicitly surfaced
through a versioned public JSON command.

### 3.7 Independent CI

An independent CI checkout running `klin check` is the Enforced boundary for klin's
quality policy. The project's separate CI steps remain the authority for build,
type-check, test, install, packaging, and other project-owned verification.

## 4. Core domain model

### 4.1 Tree

A **Tree** is one repository file state used as a subject of measurement.

A tree identity MUST be sufficient for the specific evidence that claims to bind to it.
Git tree identity is preferred when it covers the relevant source/configuration inputs.
Ignored files, ambient dependencies, generated reports, or external configuration that
affect a measurement belong in that measurement's basis rather than being silently
treated as part of the Git tree.

Tree identity and measurement-method identity are different concepts.

### 4.2 Window

A **Window** is the before/after pair one run compares, plus the reason those two trees
were selected.

At minimum the structured representation names:

```text
kind
before
after
selection
```

For automatic Stop, `before` is the prompt/turn mark maintained by host bookkeeping and
`after` is the then-current working tree.

For ordinary local `klin check`, `before` is the deterministic branch comparison base
chosen by the repository/Git context and `after` is the current tree.

For CI, the action or CI adapter SHOULD bind the comparison to the proposed change's base
and committed after revision. The selected base MUST be reported. A run MUST NOT silently
switch to a different before tree merely to obtain a measurable result.

`--changed` narrows the judgment scope within the same Window; it does not create a new
lifecycle or trust mode.

### 4.3 Policy

**Policy** is the combination of:

- product-owned capability semantics;
- values derived deterministically from repository facts;
- person-owned pins in `klin.json`;
- person-owned accepted debt where that capability supports it; and
- configured external integrations.

A derived fact is not a silent default. `klin policy` MUST say where an effective value
came from.

Policy configuration MUST express user policy, not internal lifecycle plumbing.

### 4.4 Capability

A **Capability** is one named measurement/judgment family. Its core descriptor contains
at least:

```text
name
owner: native | named-recipe | user-integration
placement: stop | check
policy section / activation
scope semantics
measurement semantics version
requiredness at its placement
```

`stop` means the capability runs at ordinary Stop and at explicit `check`.
`check` means it runs only at explicit `check`.

There is no third `ready` placement.

Native placement is product-owned metadata. User configuration cannot promote an external
process to Stop.

### 4.5 Claim and evidence

A **Claim** states what one result is about. Evidence supporting or contradicting the
claim is claim-local; heterogeneous evidence is not converted into a universal score.

Evidence records, where relevant:

```text
claim
scope
producer
origin/provenance
tree
measurement basis
intended obligation
observed obligation
observations
holes / limitations
```

Candidate-authored tests, pre-existing tests, hidden/acceptance tests, coverage,
mutation, external analyzer output, and counterfactual experiments are not
interchangeable. Provenance is part of what the evidence can truthfully support.

### 4.6 Result axes

Every material result keeps three independent axes.

**Judgment**

```text
pass
review
fail
```

- `pass`: this claim has no policy failure or contextual review item.
- `review`: evidence is useful but contextual judgment remains.
- `fail`: deterministic policy evidence requires repair before an authoritative green
  result.

**Measurement**

```text
complete
partial
unavailable
unsupported
```

`complete` means the declared intended obligation was measured under the producer's
declared semantics. It never means "behavior is complete."

`partial`, `unavailable`, and `unsupported` are explicit non-complete states. Exhaustion
is represented as a reason for `partial` or `unavailable`, never as zero findings.

**Execution**

```text
ok
tool-error
configuration-error
invocation-error
internal-error
```

Execution describes whether the producer/run could execute as intended. A code-quality
failure is not an execution error.

A result MAY have useful positive observations and still be partial. Those observations
remain visible; their absence claims do not extend over unmeasured scope.

### 4.7 Run status

A public run derives one short status without discarding the axes:

```text
PASS
REVIEW
FAIL
INCOMPLETE
ERROR
```

The derivation is:

1. `ERROR` when invocation, configuration, or klin-internal execution makes the requested
   run invalid;
2. otherwise `INCOMPLETE` when any required measurement is not complete or a required
   producer has `tool-error`;
3. otherwise `FAIL` when at least one required policy result has judgment `fail`;
4. otherwise `REVIEW` when at least one result has judgment `review`;
5. otherwise `PASS`.

If FAIL evidence and an incomplete required measurement coexist, the short status is
`INCOMPLETE`, but the FAIL remains present in `findings`. A one-word status MUST NOT erase
known evidence.

### 4.8 Finding

A **Finding** is one policy result at one site or scope.

A finding carries at least:

```text
id
capability
claim
site/scope
judgment
before/after evidence where applicable
measurement basis reference
remedy or next action where applicable
optional identity
```

A FAIL remedy MUST describe a behavior-preserving repair direction rather than the
spelling needed to appease the detector.

A REVIEW MUST explain why judgment remains contextual and MUST NOT instruct the agent to
edit repeatedly merely to clear the observation.

An INCOMPLETE/tool/configuration diagnostic MUST describe the measurement or operational
problem and MUST NOT prescribe an unrelated source-code repair.

### 4.9 Optional finding identity

Finding identity is optional, versioned, and family-owned.

The shared envelope is conceptually:

```text
identity:
  version
  state: identified | ambiguous
  key | reason
```

Rules:

- an implementation compares an identity only when it knows that version's semantics;
- differing versions do not compare;
- an unknown version does not compare even when both strings are equal;
- duplicate occurrences form a multiset and pair one-to-one;
- ambiguous identity never guesses a pairing that could hide a new occurrence;
- the measured metric value MUST NOT be part of identity merely to improve matching;
- a family without proven structural identity keeps its conservative legacy matching;
- person-authored accepted entries do not require opaque structural ids;
- measurement-basis compatibility is checked before identity matching.

The initial proven structural identity remains the #425 `complexity/1` family in Rust,
TypeScript, TSX, and Python only. Other languages retain conservative matching until
their own fixtures justify identity semantics. `reachability` and `public-api` retain
their domain identities. This section does not create a universal `SiteId`.

### 4.10 Ratchet outcome

For comparable before/after evidence, a capability may classify occurrences as:

```text
held
new
worsened
resolved
```

Existing debt that did not worsen MUST NOT make the repository newly red.

A capability MUST define what "worsened" means for its own values. Identity chooses which
site is compared; it does not define the metric.

If evidence is not comparable, klin reports the basis incompatibility instead of
manufacturing `new`, `resolved`, or `worsened` churn.

### 4.11 Accepted debt

`accepted` is person-owned policy. An agent MUST NOT be instructed to add an accepted
entry as a normal repair.

Accepted entries retain their documented, human-usable identity. vNext MUST NOT require
people to copy opaque parser ids into configuration.

`klin check` validates person-owned accepted entries closely enough to expose stale policy
rather than silently preserving configuration that no longer corresponds to debt.

## 5. Public CLI

The vNext public CLI is fixed to six intent-level commands unless a later explicit product
decision amends this specification.

### 5.1 `klin setup`

Contract:

> Set up or repair klin integration for this repository.

`setup` is repository setup, not binary installation.

It MUST:

- be safe and idempotent to rerun;
- reconcile first-class host integration artifacts that klin owns;
- create `klin.json` containing `{}` when no policy file exists, preserving current
  repository opt-in semantics;
- preserve an existing person-owned policy rather than rewriting it merely to reinstall
  hooks;
- when the repository proves no first-class host, install/reconcile repository artifacts
  for Claude Code, Codex CLI, and Cursor rather than refusing to guess;
- report what it changed and what remains unsupported;
- avoid running repository quality measurement as a side effect.

`klin setup --pin` additionally writes supported currently-derived policy values into
person-owned configuration. Pinning is explicit and non-default.

`setup` MUST NOT expose turn stamps, block counters, private indexes, or cache layouts as
user concepts.

### 5.2 `klin check [CHECK...]`

Contract:

> Explicitly measure the repository against klin's quality policy, optionally selecting
> named capabilities.

With no selectors, `check` runs every applicable native `stop` and `check` capability and
every configured required integration eligible for `check`.

Named arguments select capabilities through the same runner; they are not separately
implemented top-level programs.

Public scope/output flags:

```text
--changed
--json
```

`--changed` narrows judgment to the change set under the same Window.
`--json` changes representation only.

There is no public `--hook`, `--strict`, `--ci`, `--ready`, `--finalize`, or
`--postflight` lifecycle mode.

Local `check` is Feedback. An independent required CI invocation of the same command is
Enforced.

`check` MUST NOT run or claim to have run the project's authoritative build, type-check,
test, or install commands.

### 5.3 `klin status`

Contract:

> Read repository setup and integration state without running quality checks.

`status` is read-only. It MUST NOT measure source quality, advance prompt/window state, or
run external integrations.

The core status model has no readiness state. It reports setup/integration health, such
as whether expected first-class host integration artifacts are installed and whether the
repository policy can be discovered.

If a future implementation shows a prior check result, it MUST label that result
historical; it MUST NOT present it as current repository truth.

Public output flag:

```text
--json
```

`status --json` returns the versioned schema in section 13.

### 5.4 `klin report`

Contract:

> Show what klin caught, what was resolved, what still needs attention, and what could not
> be measured.

`report` is read-only and reads recorded history; it does not remeasure the repository.

Default scope is the current/newest known agent session. When no session is known, it
MUST say so and SHOULD suggest an explicit broader scope such as `--since 7d`; it MUST NOT
silently widen the time range.

Public options include:

```text
--since Nd
--details
--json
```

Public `--turn` and ambiguous `--all` are retired.

Report semantics include open findings, resolved findings, REVIEW items, person decisions
where recorded, and measurement/execution gaps. A non-blocking red Stop state that cannot
be person-visible on a host without creating another agent turn is persisted for this
surface.

### 5.5 `klin policy [SECTION]`

Contract:

> Explain the effective quality policy and the provenance of each decision.

`policy` is read-only. It replaces policy inspection spread across historical
`reference` and runner-list surfaces.

For each requested section it SHOULD show:

- activation;
- effective values;
- whether each value is product-owned, derived, pinned, accepted, or configured;
- supported language/scope limits;
- placement (`stop` or `check`);
- known measurement limitations that materially affect the claim.

`policy --json` returns the versioned schema in section 13.

### 5.6 `klin update`

Contract:

> Update the installed klin CLI through the supported release mechanism.

`update` changes the binary installation, not repository quality state. Repository
integration reconciliation remains `klin setup`.

## 6. Configuration contract

### 6.1 Optional file and `{}`

The repository policy file remains `klin.json`.

For an explicit `klin check`, a missing `klin.json` MUST behave as the empty derived
policy. `klin setup` MUST create an explicit `{}` when the file is absent because the
file remains the repository opt-in marker for automatic host feedback.

`{}` is a complete meaningful policy. A user does not need to copy derived topology into
configuration merely to activate klin.

### 6.2 Policy, not lifecycle

Configuration may express:

- capability policy values;
- path scopes/exclusions where the capability contract defines them;
- person-owned accepted debt;
- local project-build feedback configuration;
- journal/privacy preferences;
- explicit external integrations.

Configuration MUST NOT expose generic lifecycle placement for native capabilities.

There is no `stop | ready | check` placement key. Native placement is owned by the
capability catalogue. External integrations are `check`-only in this core contract.

### 6.3 Derived values and pins

When a value can be derived safely, absence means "derive", not a hidden constant.

A derived value MUST carry provenance sufficient for `klin policy` and structured output
to explain:

```text
section
key
value
derivation rule
derivation subject/basis where material
```

`setup --pin` turns supported derived policy into person-owned values. A pin is a policy
decision; it is not a cached repository fact.

### 6.4 Accepted entries

Accepted debt is edited only by a person in a reviewed policy change. Agent feedback may
say that a person can decide an intentional exception, but MUST NOT make "add an accepted
entry" the automatic repair path.

### 6.5 Local project-build feedback

A `build` policy entry, where supported, belongs to the **local Stop feedback** path. It
does not make build part of `klin check`.

A missing build tool, build command failure, or build timeout therefore has its own local
feedback semantics in section 8. It MUST NOT be reinterpreted as a native quality
finding.

### 6.6 External integrations

A user-owned integration configuration owns its tool command, tool version, analyzer
configuration, and producer semantics.

klin owns:

- bounded process execution;
- report freshness/tree association rules it can prove;
- normalization into the core result algebra;
- native/external identifier namespaces;
- delta judgment that klin explicitly documents;
- honest representation of observed coverage and holes.

A successful process exit or empty report is not sufficient proof of complete
measurement.

External integrations run at `check`, never automatic Stop.

### 6.7 Unknown configuration

Before the #344 1.0 stability contract is implemented, unknown or malformed policy MAY
remain a hard configuration error. The 1.0 compatibility rule for newer keys is owned by
#344 after this SPEC freezes the final public surface.

Regardless of version, ignored unknown policy MUST NOT silently weaken a required check.

## 7. Execution model

### 7.1 One measurement engine

Stop and `check` MUST select work from one capability/result engine.

The engine takes, conceptually:

```text
Window
effective Policy
capability selection
placement
measurement context
```

and returns structured semantic results. Agent text, human text, CI annotations, journal
records, and JSON are renderings of those results.

A capability MUST NOT implement one claim for Stop and a materially different claim for
CI under the same identity.

### 7.2 Placement

There are exactly two core placements:

```text
stop
check
```

A native `stop` capability runs at Stop and `check`.
A native `check` capability runs only at `check`.
All user-owned external integrations run at `check`.

Check-only work adds no work to ordinary Stop.

### 7.3 Initial native capability placement

The first vNext migration retains the narrow shipped native capabilities rather than
using #493 to admit new phenotype detectors.

Unless a separate roadmap ticket explicitly changes one under existing evidence, these
run at `stop` and `check`:

```text
doc-size
doc-citations
lockfile
escapes
stubs
inventory
complexity
dead-symbols
reachability
layering          (when policy activates it)
public-api
conventions       (when policy activates it)
```

`sarif` is an external integration and is `check`-only.

This table does not silently re-certify known detector limitations. Exact per-capability
measurement rules remain governed by their accepted behavior/tests and any explicit
migration ticket. Section 18 records the product dispositions vNext must preserve.

### 7.4 Explicit `check`

`klin check`:

1. discovers repository/policy state;
2. selects its Window and prints/records it;
3. resolves applicable capabilities and optional selectors;
4. constructs the before/after measurement context;
5. evaluates all selected native capabilities;
6. runs selected external integrations under their bounded execution contracts;
7. aggregates the orthogonal results;
8. renders text or one JSON document;
9. exits according to section 12.

The run MUST remain truthful when a later capability cannot execute. Earlier known
findings remain in output.

### 7.5 Project CI ownership

`klin check` does not compile, type-check, test, install, package, or benchmark the
project merely because a local Stop build entry exists.

A repository claiming Enforced vNext therefore has at least:

```text
required klin check
required project verification appropriate to that project
protected/reviewed policy surface
```

The project chooses the project-verification commands. klin does not call those commands
part of its own successful `check`.

## 8. Agent/harness protocol

### 8.1 One hidden ingress

All first-class host lifecycle traffic enters through:

```text
klin __agent event
```

The event kind MUST be determined before expensive initialization.

Before dispatching a cheap `session`, `prompt`, or `pre_tool` event, the binary MUST NOT
perform work whose only purpose is Stop/check measurement, including:

- loading the whole project;
- resolving the quality base;
- scanning the source tree;
- building a module graph;
- launching an external process.

The implementation MAY read the minimum state/configuration required by the event itself.

### 8.2 Event kinds

The normalized semantic event kinds are:

```text
session
prompt
pre_tool
stop
```

Host adapters may have different native names. The adapter maps them into this vocabulary.

Unknown/malformed local events fail open: klin MUST NOT block an agent based on an event
it could not place or parse. A concise diagnostic MAY be recorded.

### 8.3 Session and prompt bookkeeping

Session/prompt events maintain the local Window and genuine-prompt budget required for
bounded Stop feedback.

Multiple sessions in one worktree are supported. State publication MUST be
concurrency-safe: one session MUST NOT inherit another session's quality-block budget
because two prompt updates raced.

The core specification does not prescribe private Git index names, lock files, or a
transaction algorithm. Those are implementation mechanisms and may change as long as the
observable invariants hold.

Automatic host continuations generated by klin do **not** count as a new genuine person
prompt and MUST NOT refresh the quality-block budget.

### 8.4 Pre-tool guard

The pre-tool event provides best-effort local protection of person-owned klin policy/state
through host-routed tool calls.

It is Feedback, not a security boundary.

The pre-tool path retains the **<50 ms** budget. It MUST dispatch before project
measurement work and MUST NOT run quality capabilities.

When klin cannot prove that a tool call writes a guarded target, it follows the
guard-specific conservative contract; the guard MUST NOT pretend to sandbox arbitrary
shell execution.

### 8.5 Stop purpose

Stop is bounded local feedback. It is not a declaration that the model is semantically
finished.

On Stop, klin may:

1. run the separate local project-build feedback path;
2. run native capabilities placed at `stop`;
3. give actionable code FAIL feedback to the agent;
4. record REVIEW, incomplete, execution, and pass-through evidence;
5. spend at most the quality-block budget below.

No external analyzer/integration process runs because of automatic Stop.

### 8.6 Quality-block budget

A code-quality FAIL may spend:

- quality block #1 under a genuine prompt;
- quality block #2 only after the tree changed since the first blocked Stop.

A same-tree red Stop after block #1 does not spend block #2.

After block #2, further code FAIL results under that genuine prompt are reported but pass
through. There is no third quality block.

A changed tree may expose a different finding at block #2. The cap is still two total
quality blocks, not two per finding.

When local state cannot safely establish or persist the budget, Stop fails open rather
than inventing another block.

### 8.7 Incomplete/tool/configuration behavior at Stop

The following consume **zero quality blocks**:

```text
measurement partial
measurement unavailable
measurement unsupported
tool-error
configuration-error
invocation/internal execution failure in the local hook path
```

They MUST NOT be rendered as code-quality FAIL and MUST NOT tell the agent to change
source merely to fix measurement.

Stop gives a concise diagnostic/limitation, records it when possible, and passes through.

This rule does not turn the result green. A later explicit/CI `klin check` remains
non-green when required authoritative measurement cannot be produced.

### 8.8 Separate project-build feedback

Project build feedback remains separate from the quality-block budget and from klin
judgment.

The existing bounded build-feedback mechanism MAY block local Stop when the tree changed
and the configured/derived project build fails. Its current cap remains **eight build
blocks under one genuine prompt**. That counter is independent of the two quality blocks;
vNext does not reinterpret a build failure as a native finding.

A missing build tool is unmeasured local build feedback, not a code-quality FAIL.
The message directs the agent/person toward installing dependencies or correcting
configuration, not toward deleting the manifest or hand-editing generated lock evidence.

Because public `klin check` does not run the build, pass-through text MUST NOT say
"klin CI will catch the build." It says that the project's own CI owns that verification.

### 8.9 REVIEW at Stop

REVIEW does not consume a generic quality block merely because it is visible.

A capability MAY have one specifically designed human-intent interaction, such as
`inventory` asking once about a newly deleted test. That interaction is a separate
ask-once human-decision budget, not a generic code-FAIL quality block. It MUST occur at
most once for the same deletion evidence under the prompt/window and must not convert a
contextual observation into reply-cleared clean evidence.

Once acknowledged, an intentional deleted test remains REVIEW in later check/report
output until the underlying change or repository review resolves it.

### 8.10 Cursor delivery

Cursor blocking feedback MAY use the host's automatic follow-up transport because the
agent needs to act.

A **non-blocking red/pass-through person notice MUST NOT be emitted as
`followup_message`** on Cursor, because Cursor converts that transport into a new agent
prompt.

Instead klin records the unresolved status in the journal and exposes it through
`klin report`. Independent CI remains the enforcement boundary.

Claude Code and Codex may use person-visible non-blocking rendering where their host
surface supports it, but the semantic rule is the same: do not create a repair turn only
to deliver a person-facing unresolved-status notice.

### 8.11 Ask tools after partial edits

`AskQuestion` / `AskUserQuestion` style host tools may keep a turn open.

The accepted behavior is:

```text
agent edits partial tree
-> agent asks person
-> person answers or skips
-> turn ends
-> Stop judges the then-current tree
```

klin MUST NOT add PostTool quality hooks or special interception merely to judge the
partial tree before the person's answer.

After Stop runs, the same-tree pass-through and two-block cap apply normally.

### 8.12 No readiness phase

There is no core readiness record, readiness cache, readiness status, or
model-invoked finalization operation.

A future deep-local pre-hand-back phase can be re-admitted only when all of these exist:

1. a named, admitted capability whose cost/nature keeps it off every Stop;
2. agent-outcome evidence on at least two first-class hosts showing materially better
   repairs or fewer harmful hand-backs than check/CI-only placement; and
3. a cost account showing that record/tree identity/sandbox/delivery machinery is
   justified by that benefit.

Speculative future need is insufficient.

## 9. Ratchet and measurement semantics

### 9.1 Existing debt

For a ratcheted capability, a base occurrence that still exists without worsening is
held debt. It does not fail merely because it exceeds today's derived/pinned ceiling.

A new occurrence or a proven worsening may fail according to the capability contract.

Tightening policy MUST NOT retroactively make unchanged base debt newly red unless a
person explicitly chose a contract that says otherwise.

### 9.2 Comparable basis first

Before matching sites or computing ordinary deltas, klin checks measurement-basis
compatibility.

Compatibility is:

```text
comparable
not-comparable
unknown
```

with reasons.

Only comparable evidence enters ordinary held/new/resolved/worsened comparison.

When bases differ, the preferred repair is to measure both trees under one compatible
basis. If that cannot be done, klin reports the basis change; it does not claim source
churn.

### 9.3 Measurement basis

The minimum semantic basis contains only components that can change the meaning of the
measurement, including where relevant:

- native capability/producer identity;
- measurement-semantics version;
- external producer/tool and exact or compatible version;
- rule-set/configuration identity;
- klin adapter version;
- parser/extractor semantic identity;
- normalized measurement-relevant options;
- selection/scope semantics and selected obligation set;
- feature/platform/test-harness inputs when they change the claim.

Machine path, timestamp, PID, scratch directory, and duration are execution metadata,
not method identity by themselves.

Source contents are normally the subject being compared, not a different measurement
method.

### 9.4 Intended and observed obligation

A measurement declares what it intended to observe and what its producer proves it
actually observed.

`complete` requires proof that the intended obligation was observed under the declared
semantics.

Zero results do not prove that the selected scope was analyzed.

A producer that cannot prove per-file coverage may still provide advisory evidence, but
the result MUST limit its completeness claim accordingly.

### 9.5 Resource exhaustion

A deterministic work bound, timeout, output bound, or memory/resource bound that stops
analysis early never produces a fabricated clean measurement.

Record:

```text
bound type
configured bound
observed work when known
unmeasured remainder / limitation
```

Validated positive findings obtained before exhaustion may remain useful. Absence claims
do not extend across the remainder.

Ordinary Stop does not silently escalate a bounded capability into an unbounded scan.

### 9.6 Persistent/known measurement holes

A capability may have documented language or construct limits. A hole is not a finding.

A hole that lies outside the capability's declared supported obligation may be
not-applicable, but the supported boundary MUST be visible through policy/structured
output.

A supported file that newly becomes unreadable/unmeasurable is not clean. When a base
held the same explicitly known unmeasurable state, a ratchet may preserve it as a known
inherited hole rather than newly failing; the output still MUST NOT claim that file was
measured.

### 9.7 External identifier ownership

Native and external results have collision-proof ownership.

Conceptually:

```text
native:
  ["native", "klin", capability, family-owned-id]

external:
  ["external", configured-entry-id, producer-id, producer-owned-id]
```

An external report cannot impersonate a native finding by choosing a rule id such as
`klin/complexity`.

This is a namespace rule, not a generic plugin/provider registry.

## 10. Capability ownership and extension boundary

### 10.1 Native klin-owned

A native capability is appropriate when klin can own:

- deterministic semantics at the strength claimed;
- repository-grounded before/after judgment;
- bounded operation;
- clear supported/unsupported scope;
- behavior-preserving repair guidance; and
- the product experience cost of its placement.

No new #357 phenotype capability is admitted by this specification.

### 10.2 Future named klin-owned recipe

A named external recipe is a possible future capability type, not a currently admitted
recipe.

Before one ships, a separate targeted decision must define and validate:

- exact supported tool/version policy;
- fixed invocation and bounded environment;
- adapter/normalization;
- rule/configuration basis;
- observed-scope/completeness proof;
- offline/no-network measurement expectation;
- tree binding/freshness;
- licensing/distribution/platform support;
- failure semantics; and
- product value/repair/attention burden.

#363's Ruff research establishes technical feasibility only; #357 did not admit that
recipe. Ruff, Semgrep, Gitleaks, and ESLint therefore do not become named vNext features
through #493.

Any future named recipe is `check`-only unless a later lifecycle decision meets section
8.12's evidence bar.

### 10.3 User-owned SARIF/integration

User-owned integration remains the extension seam for project analyzers.

The project owns the command, analyzer version, rule/configuration, and any network or
environment assumptions of that command.

klin does not give arbitrary user shell execution the reproducibility claims of a named
recipe.

The integration MUST still map absence/failure/stale/partial report evidence into the
core result axes honestly.

### 10.4 Duplication boundary

#478 and its child research own duplication-specific:

- normalization/token rules;
- minimum region thresholds;
- occurrence/region-family identity;
- lineage semantics;
- exact versus near-miss algorithms;
- detector-specific BLOCK versus REVIEW disposition;
- base-index payloads and performance design.

A future duplication capability consumes this SPEC's:

- placement;
- result axes;
- measurement basis;
- finding identity envelope;
- ratchet semantics;
- repair rendering;
- JSON envelope;
- Stop admission and performance rules.

Duplication research does not reopen those core contracts unless it meets the explicit
evidence bar for a core product change.

### 10.5 Shared parse/cache seam remains physical, not normative

The core SPEC permits capabilities to share parsed facts, indexes, and cached base
evidence. It requires semantic cache invalidation to be truthful.

It deliberately does **not** freeze:

- which module owns the parser;
- the serialized cache format;
- base-index payload shape;
- detector index storage;
- cache file names;
- physical invalidation implementation.

Implementation work that would materially freeze those seams must synchronize with
#480 Step 1 and #489 as required by #358.

A cache is an optimization. Stale/corrupt/incompatible cache data MUST cause safe
recomputation or explicit incompleteness; it MUST NOT fabricate a PASS.

## 11. Initial capability dispositions

The core migration preserves the product decisions already made.

### 11.1 `complexity`

Retain the shipped gate's post-#389 direction, including the derived cyclomatic-complexity
floor of 10 and the explicit test-length policy.

Do not broaden complexity or spend Stop latency on speculative semantic sophistication.
Known residual noise is a reason for targeted future evidence, not for a generic score.

### 11.2 `escapes`

Retain narrow deterministic spellings and the established test-idiom corrections.

The public claim MUST NOT imply equivalent rewordings are all detected. Known rewording
routes remain documented limitations unless a narrow shape independently earns admission.

### 11.3 `stubs`

Retain the narrow marker/code-stub capability and the marker-count identity already
adopted for comment markers.

#362 does not authorize broad unfinished/error-masking blocking semantics.

### 11.4 `reachability`, `dead-symbols`, `public-api`, `layering`

Retain their product roles and documented language/measurement holes.

Ambiguous graph, resolution, public-surface, or dynamic-framework evidence remains
explicit rather than guessed.

### 11.5 `doc-size`

Retain the per-instruction-file policy. It is not a total-context budget.

A remedy MUST preserve requested content. It MUST NOT tell an agent to move narrative out
of the file when the user's task requires that narrative there merely to satisfy the
metric. Only a person changes a pinned ceiling.

### 11.6 `inventory` and intentional test deletion

A newly deleted test may trigger one bounded local intent interaction.

A reply acknowledging intentional deletion does not erase the evidence. Once intentional,
the deletion remains a REVIEW item for explicit check/report and normal human code review.

REVIEW does not require a special acceptance workflow and does not force the agent to
recreate an intentionally removed test merely to obtain exit 0.

### 11.7 `lockfile`

Retain the deterministic repository/lock consistency role.

Repair guidance MUST NOT invite an agent to hand-write lockfile entries when the package
manager cannot run. It tells the agent to use the project's package manager and to report
when the required install cannot be performed.

Registry existence/age is not inferred offline and is not added by this SPEC.

### 11.8 `build`

Build remains local project feedback plus project-owned CI verification.

It is not a `check` capability and not part of klin's authoritative quality result.

### 11.9 `sarif`

Retain the user-owned external integration seam, but run external execution only at
explicit `check`.

Missing, failed, stale, partial, or unproved-scope reports never appear as clean
measurement.

## 12. Public result and exit contract

### 12.1 `klin check` exit codes

vNext has three public process-exit categories:

| Exit | Meaning |
| ---: | --- |
| 0 | requested run executed; required measurement is complete; judgment is PASS or REVIEW |
| 1 | requested run executed; required measurement is complete; at least one blocking FAIL exists |
| 2 | authoritative result could not be produced: ERROR or INCOMPLETE |

Exit 2 includes invocation/configuration/internal errors and required
partial/unavailable/unsupported/tool-error measurement.

This deliberately supersedes the earlier #475 draft's separate exit 3. #492 froze one
non-green incomplete/tool-error category for authoritative public/CI `check`; the
structured axes preserve the distinction that a single process exit cannot.

When exit 2 coexists with known FAIL evidence, output still includes that FAIL.

### 12.2 Stop has no authoritative exit claim

The hidden host adapter translates local results into host decisions. Its process exit is
an integration transport detail, not the public quality status contract.

In particular, local INCOMPLETE/tool/configuration failures pass through and consume no
quality block even though an explicit `klin check` over the same required obligation
would exit 2.

### 12.3 REVIEW is successful but visible

REVIEW maps to exit 0 when required measurement is complete and execution is valid.

That does not mean REVIEW is PASS. Human text and JSON MUST make REVIEW visible so normal
repository review can act on it.

### 12.4 NOTE

A NOTE is informational evidence. It does not by itself change judgment or exit.

A NOTE MUST NOT be used to hide required incomplete measurement; required holes affect the
measurement axis and therefore exit 2.

## 13. Machine-readable public contract

At the vNext stability boundary, `check`, `status`, `report`, and `policy` each expose a
schema-versioned JSON document.

The stable contract is the documented envelope and documented field meanings, not every
debug/performance field.

Additive fields are allowed within a schema version. Consumers MUST ignore unknown fields.
Removing a documented field or changing its meaning is governed by the #344 stability
contract after this SPEC is accepted.

### 13.1 Common envelope

Every public JSON document contains:

```json
{
  "schema_version": 1,
  "command": "check|status|report|policy",
  "klin_version": "...",
  "repository": {}
}
```

`repository` contains stable repository/worktree identity needed to interpret the command,
not private cache paths.

### 13.2 `check --json`

Minimum envelope:

```json
{
  "schema_version": 1,
  "command": "check",
  "klin_version": "...",
  "repository": {},
  "window": {
    "kind": "...",
    "before": "...",
    "after": "...",
    "selection": "..."
  },
  "result": {
    "status": "PASS|REVIEW|FAIL|INCOMPLETE|ERROR",
    "judgment": "pass|review|fail",
    "measurement": "complete|partial|unavailable|unsupported",
    "execution": "ok|tool-error|configuration-error|invocation-error|internal-error",
    "exit": 0
  },
  "capabilities": [],
  "findings": [],
  "reviews": [],
  "notes": [],
  "measurements": [],
  "errors": []
}
```

For a mixed run, top-level `measurement` is `complete` only when every required
measurement is complete. Otherwise it is the strongest truthful non-complete aggregate;
per-measurement rows retain exact reasons.

An axis that could not be reached because an earlier invocation/configuration failure
prevented measurement is represented as JSON `null`, not guessed `pass` or `complete`.
The string unions shown above describe non-null values.

Top-level `execution` summarizes the cause that controls short status while per-capability
rows retain all execution states.

Each material finding/review/measurement row carries or references enough basis to
reproduce its claim. Large debug timing/counter payloads MAY be placed under an explicitly
unstable `diagnostics` object.

### 13.3 `status --json`

Minimum envelope:

```json
{
  "schema_version": 1,
  "command": "status",
  "klin_version": "...",
  "repository": {},
  "configured": true,
  "integrations": [],
  "diagnostics": []
}
```

There is no readiness CURRENT/FINDINGS/STALE state.

An integration row reports host, installation scope/source, expected/observed state, and
whether action is needed. `status` does not claim repository quality.

### 13.4 `report --json`

Minimum envelope:

```json
{
  "schema_version": 1,
  "command": "report",
  "klin_version": "...",
  "repository": {},
  "scope": {},
  "summary": {},
  "records": []
}
```

The stable semantic record exposes findings/reviews/resolutions and measurement gaps.
Internal journal line layout, lock records, block counters, and cache paths are not public
merely because `report` derives from them.

### 13.5 `policy --json`

Minimum envelope:

```json
{
  "schema_version": 1,
  "command": "policy",
  "klin_version": "...",
  "repository": {},
  "sections": []
}
```

Each section exposes effective values and provenance (`product`, `derived`, `pinned`,
`accepted`, or `configured` where applicable), activation, and placement. It MAY expose
documented capability limitations.

### 13.6 Finding/result identifiers

Public finding ids are deterministic under the capability's documented semantics.

The optional identity envelope MAY be exposed beside a finding. Its presence does not
change the finding id unless the capability's separately versioned public contract says
so.

External ids remain in the external namespace and cannot collide with native ids.

## 14. Reporting and progressive disclosure

### 14.1 Agent Stop output

Default agent feedback is intentionally short.

For each repairable FAIL it leads with:

```text
site
measured fact
why it matters
repair direction
```

It does not dump measurement-basis matrices, cache statistics, or all inherited debt into
the agent context.

When multiple results exist, prioritize:

1. build/execution conditions that prevent useful local work, rendered as operational
   feedback rather than source-quality blame;
2. deterministic repairable FAIL findings;
3. high-value contextual REVIEW evidence only when action/judgment is genuinely needed;
4. incomplete/diagnostic notes concisely;
5. detailed provenance only on explicit inspection.

The quality-block path contains a bounded repair queue, not an unbounded wall of findings.

### 14.2 Human `check`

Human text starts with the short authoritative outcome, then blocking findings, REVIEW
items, and required incomplete/error reasons.

Successful inherited/held debt MAY be summarized rather than printed site by site.

### 14.3 `report`

`report` is the progressive-disclosure surface for historical attention and value:

- what was put in front of the agent;
- what was later resolved;
- what remains REVIEW/open;
- what passed through because the block budget was spent;
- what could not be measured/executed.

It MUST distinguish observation from intervention when local state can prove that
distinction.

### 14.4 Explainability

The core product does not require a public `klin explain` command in this SPEC.

Nevertheless, stored/structured result design MUST make it possible for a future
explanation surface to reconstruct:

- Window/tree identity;
- effective policy;
- measurement basis;
- before/after evidence;
- completeness/holes;
- provenance;
- why the result received its judgment; and
- what verifies the repair.

Legacy records missing these facts are not retroactively upgraded by inference.

## 15. Determinism and reproducibility

Native deterministic capabilities MUST give the same semantic result for the same:

```text
before/after subject
effective policy
measurement basis
selected scope
klin measurement semantics
```

on supported platforms, excluding explicitly documented platform-sensitive claims.

Ordering of findings and structured sets MUST be deterministic.

An external integration can be useful without satisfying native reproducibility, but its
limitations must be represented in its basis/completeness. A user-owned arbitrary command
does not inherit product-owned reproducibility by being wrapped in klin.

Runtime timestamps, duration, process ids, and scratch paths MUST NOT alter finding
identity or basis compatibility unless a capability explicitly makes them part of the
claim.

## 16. Performance contract

Performance is an admission property.

### 16.1 Ordinary Stop

Controlled 1M / 20-changed workload:

```text
<= 1500 ms    current acceptable envelope
<  500 ms     strategic target
```

The envelope is not a feature budget to spend casually.

A candidate Stop capability MUST report incremental wall time and why Stop placement is
necessary. Guidance:

```text
< 10 ms       trivial incremental cost
10–25 ms      normal
25–75 ms      needs clear product value
75–150 ms     exceptional
> 150 ms      presume check placement
> 500 ms      not an ordinary Stop capability
```

These bands guide product review; they do not override precision/repair requirements.

Ordinary Stop MUST avoid:

- a new external process for quality measurement;
- a second parser for source already represented by shared facts where practical;
- a whole-tree source walk solely for a new changed-file capability;
- speculative precomputation for a future capability.

### 16.2 Pre-tool

The pre-tool guard retains a <50 ms target/budget and does no quality measurement.

### 16.3 Check-only work

Check-only capabilities may spend more time when product value justifies it, but every
operation remains bounded. External commands have time, output, and process-tree cleanup
bounds.

Check-only work MUST add zero work to a Stop where it is not selected.

### 16.4 Resource accounting

Performance diagnostics SHOULD distinguish:

- klin analysis time;
- project-build feedback time;
- external integration time;
- cache/layout work.

A capability does not hide external command latency inside "klin time" when product
admission depends on that distinction.

## 17. Failure, recovery, and concurrency

### 17.1 No fabricated green

When klin cannot prove a required measurement, it says so.

These are never synonyms:

```text
unmeasured == clean
unknown == unused
unobserved == unnecessary
tool error == code failure
```

### 17.2 Local fail-open boundary

Local hooks are Feedback. When hook state is unsafe, unreadable, concurrently unavailable,
or impossible to update safely, the integration fails open rather than blocking the agent
on an invented budget/state.

Fail-open MUST record a diagnostic when safe to do so.

Fail-open local behavior does not alter explicit `klin check` authority.

### 17.3 State concurrency

Concurrent sessions in one worktree MUST NOT:

- collapse two genuine prompts into one budget;
- transfer a quality-block spend between sessions;
- let a long prompt transaction cause a normal failing Stop to lose its intended
  bounded-feedback opportunity; or
- publish a partially constructed state record as authoritative.

The implementation MAY use locks, compare-and-swap, short transactions, private Git
indexes, or another mechanism. Those mechanisms are internal.

### 17.4 Crash consistency

A crash during local bookkeeping MUST leave either the previous coherent state or a
detectably incomplete/recoverable state.

A durable marker MUST NOT claim that a prompt/tree capture completed when only an
intermediate private index/cache write completed.

Temporary state created for capture MUST be bounded or recoverable so repeated
interruptions cannot grow unbounded repository-local storage.

### 17.5 Configuration failure

At Stop: diagnostic, zero quality block, pass through.

At explicit `check`: `ERROR`, exit 2.

A configuration message may name the policy action required. It must not masquerade as a
source-code remedy.

### 17.6 Tool/integration failure

At Stop, no external integration runs. Native internal errors and local tool/config
limitations consume zero quality blocks.

At `check`, failure of a required external producer yields tool-error/incomplete and exit
2. Optional advisory evidence may remain unavailable without vetoing unrelated complete
required measurement, but it cannot support a claim it did not measure.

### 17.7 Resource exhaustion

Resource exhaustion follows section 9.5. A timeout or bounded stop does not become a zero
result.

### 17.8 Recovery surfaces

Routine recovery is automatic or owned by `setup`.

There is no stable public `turn reset` or `cache` command in vNext. Advanced/private
support tooling MAY exist, but it is not part of the public compatibility surface.

## 18. Trust and security model

### 18.1 Feedback versus Enforced

**Feedback** is local:

- `__agent event`;
- local build feedback;
- Stop capabilities;
- pre-tool guard;
- local explicit `klin check`.

**Enforced** requires:

- an independent CI checkout;
- a pinned/approved klin version;
- `klin check` as a required check;
- protected/reviewed repository policy and CI configuration.

Local Feedback is useful but not tamper-proof.

### 18.2 Project verification is separate

Enforced klin policy does not imply that the project built or passed tests.

Repository policy SHOULD make appropriate project build/typecheck/test steps separately
required.

### 18.3 Agent-controlled worktree

The agent can potentially edit source, tests, project hooks, policy, workflows, or other
files available to its host. The local guard is best-effort feedback over routed tool
calls, not a sandbox.

Independent CI does not trust local journal/verdict state.

### 18.4 No telemetry and native network reads

klin-owned native checks read repository/local process state and do not send source or
telemetry to a service.

Native measurement MUST NOT require a network request.

A project-owned external command invoked by explicit `check` is project-controlled and
may have its own network behavior; klin MUST NOT describe that command as a native
offline guarantee.

### 18.5 External command safety

External processes run with explicit bounds on duration/output and process-tree cleanup.

A named future klin-owned recipe would additionally require a fixed/minimal invocation
contract and offline measurement policy.

### 18.6 Secrets

klin MUST NOT intentionally collect, transmit, or persist repository secrets as product
telemetry.

Diagnostic/output capture from project-owned commands SHOULD be bounded and should avoid
persisting more than is needed to explain execution.

## 19. Future capability admission

A future capability is not admitted because a paper describes the phenotype or because a
prototype has high offline precision.

The admission packet considers all of:

### Detection

- deterministic evidence at any blocking strength;
- precision and hard negatives;
- explicit before/after/change semantics;
- supported/unsupported scope;
- measurement-basis identity.

### Repair

- actionable remedy;
- behavior-preserving repair;
- resistance to cheap appeasement;
- re-verification step.

### Agent experience

- correct repair rate;
- appeasement;
- unresolved/harmful repair;
- additional repair turns/context size;
- repeated-finding behavior.

### Developer experience

- reproducibility/explainability;
- configuration/setup burden;
- portability across first-class hosts and CI;
- operational dependencies.

### User experience

- unnecessary human interruption;
- genuine human-judgment cases;
- understandable default message;
- attention burden.

### Evidence integrity

- coherent tree binding;
- intended/observed scope;
- compatible measurement basis;
- explicit resource/tool holes.

### Operations

- bounded runtime;
- cache/memory cost;
- implementation and maintenance cost;
- Stop performance impact.

A contextual capability belongs at REVIEW or stays out. A capability whose value does not
justify implementation/configuration/maintenance cost stays research/benchmark evidence.

Future studies SHOULD be targeted to a concrete product/user need. The terminated
29-phenotype #357 study is not a standing program to rerun.

## 20. Public compatibility and migration

### 20.1 Migration from pre-vNext public names

Before 1.0, old human-facing names hard-break with targeted migration errors rather than
permanent aliases unless real external adoption evidence proves an alias necessary.

Mapping:

| Historical surface | vNext |
| --- | --- |
| `gate` | `check` |
| individual top-level checks | `check CHECK...` |
| `init` + `install` | `setup` |
| `init --pin` | `setup --pin` |
| `stats` | `report` |
| `reference` / `gate --list` | `policy` |
| `radius`, `guard`, hook-specific gate modes | hidden `__agent event` |
| research/finalization `__agent ready` | no operation; readiness is deferred |
| `turn` / `turn reset` | no public replacement |
| `cache` | no stable public replacement |

Migration errors SHOULD name the exact new command when one exists.

### 20.2 Installed-integration compatibility

Generated hook/plugin commands are the exception to an immediate hard break.

An updated binary MUST NOT silently strand an already-installed integration. A
time-bounded hidden legacy dispatch path MAY accept old generated hook invocations long
enough for `klin setup` to reconcile them.

Such compatibility:

- is absent from ordinary help;
- is not a public human API;
- emits a migration diagnostic where useful;
- has a documented removal boundary in the implementation roadmap/stability contract.

### 20.3 State/cache compatibility

Private state/cache formats are not stable public API.

A new binary may invalidate/rebuild compatible caches safely. It MUST NOT reinterpret
unknown old state as authoritative current evidence.

Journal semantics exposed through `report` are stable at the report schema, not at the
private file layout.

### 20.4 1.0 stability contract

#344 follows this SPEC.

It must define semver/deprecation compatibility for at least:

- six public commands;
- documented public flags;
- process exit codes;
- `klin.json` keys and forward-key behavior;
- JSON schema envelopes/fields;
- custom harness event/answer protocol;
- measurement-semantic changes where they affect held/new/worsened identity.

This SPEC freezes the surfaces #344 reasons about; #344 defines release compatibility,
not product strategy.

## 21. Custom harness and first-class host contract

### 21.1 First-class hosts

Claude Code, Codex CLI, and Cursor are first-class integrations.

klin maintains adapters/tests for their supported event/decision contracts.

First-class status is a compatibility promise, not a statement that the three hosts expose
identical transports.

### 21.2 Generic/custom harness

A custom harness may implement klin's versioned harness event/decision protocol.

The semantic lifecycle remains:

```text
session
prompt
pre_tool
stop
```

A custom harness MUST NOT require a separate readiness/finalize event to satisfy the
current core protocol.

The generic protocol is versioned. #344 defines its additive/backward compatibility
window after this SPEC is accepted.

### 21.3 Host-specific rendering

Host transport may differ:

- blocking Stop feedback can use each host's supported blocking/follow-up mechanism;
- person-visible non-blocking diagnostics SHOULD avoid agent-turn creation;
- Cursor specifically follows section 8.10;
- unknown host events fail open.

Transport differences MUST NOT change the semantic judgment of one result.

## 22. Validation and conformance

Implementation of this SPEC is not complete until behavior is pinned at public/host seams.

### 22.1 CLI contract tests

At minimum test:

- the six-command help surface;
- targeted migration errors for old public names;
- `setup` idempotency and `setup --pin`;
- `check` selectors, `--changed`, and `--json`;
- `check` does not run project build/test/typecheck;
- status/report/policy are read-only;
- public exits 0/1/2 including mixed FAIL + INCOMPLETE;
- REVIEW exits 0 but remains visible;
- JSON required envelope fields.

### 22.2 Result/evidence tests

Pin:

- basis-compatible comparison;
- changed/unknown basis does not create false regressions/resolutions;
- complete versus partial/unavailable/unsupported;
- empty external report does not prove completeness;
- resource exhaustion cannot produce clean absence;
- external/native identifier namespaces cannot collide;
- ambiguous/duplicate optional identities preserve multiplicity;
- accepted entries remain human-authored and usable.

### 22.3 Stop lifecycle tests

Pin:

- quality block #1;
- changed-tree quality block #2;
- same-tree no-reblock;
- no third block;
- automatic continuation gains no fresh prompt budget;
- INCOMPLETE/tool/config/internal error consumes zero quality blocks;
- incomplete diagnostics prescribe no source repair;
- REVIEW creates no generic clear-the-warning loop;
- local state loss/lock failure is fail-open;
- concurrent sessions do not share prompt/block state;
- ask-tool-after-partial-edit behavior;
- no PostTool quality hook;
- Cursor blocking feedback reaches the agent;
- Cursor non-blocking pass-through sends no `followup_message`.

### 22.4 Build-boundary tests

Pin:

- local Stop may run configured/derived build feedback;
- public `check` never runs that build;
- messages name project CI as build/typecheck/test owner;
- missing local build tool is not a native quality FAIL.

### 22.5 Host tests

Claude Code, Codex, and Cursor compatibility evidence MUST cover the current supported
event formats and generated integration.

A host canary or equivalent must detect contract drift without silently turning a missing
integration into a quality PASS.

### 22.6 Performance tests

Preserve controlled 300k/1M fixtures and report at least the 1M/20-changed Stop path.

A new Stop implementation must show it did not add check-only work or duplicate parsing.

### 22.7 Security/trust tests

Pin that:

- local missing/malformed hook events fail open;
- independent `check` ignores local verdict state;
- external processes are bounded and cleaned up;
- config/tool errors do not become source findings.

## 23. Implementation architecture constraints

This section constrains behavior-preserving architecture without freezing the physical
design #480/#489 are still researching.

### 23.1 One catalogue / capability descriptor

The implementation SHOULD extend the existing central check catalogue rather than create
parallel Stop/check registries.

The descriptor is the natural home for:

- capability name;
- policy section;
- native/integration ownership;
- placement;
- activation;
- measurement semantics version;
- supported language/scope metadata.

### 23.2 One explicit execution context and result sink

Checks should continue receiving explicit immutable run context and writing to a shared
semantic result sink rather than reading hidden global lifecycle state.

The current `Context`/`Sink`/`Records` architecture is a migration foundation, not the
final public schema. vNext should evolve it so result axes and evidence basis are first
class rather than encoded indirectly as strings/notes.

### 23.3 Dispatch before project construction

`__agent event` dispatches its event kind before constructing a full `Project` or quality
Window. This is a hard architecture invariant because it protects pre-tool/cheap-event
latency.

### 23.4 Shared structural facts

Where multiple native capabilities need the same source parse/facts, they should share
one extraction. A capability should not introduce a second parser pipeline merely for
convenience.

This is a semantic ownership preference, not permission to freeze a new persistent
repository-wide index format before the #480/#489 rendezvous.

### 23.5 Cache is not authority

Persistent cache is an optimization over reproducible source facts.

Cache version/basis incompatibility invalidates reuse. It cannot turn unknown evidence
into held/clean evidence.

## 24. Superseded and deferred decisions

This SPEC explicitly reconciles the major research contradictions.

### 24.1 Explicit readiness

#352 proved explicit readiness could be built. #484 later tested whether vNext needed it
and chose Stop feedback + check enforcement.

Therefore the readiness command/state/tree-cache sections of #475/PR #476 are superseded.
Their useful evidence/basis/tree-coherence principles survive in sections 4 and 9.

### 24.2 Exit 3 for INCOMPLETE

PR #476 proposed exit 3 to separate INCOMPLETE from execution/configuration ERROR.

#492 later froze the authoritative `check` behavior: required incomplete/tool/config
failure is non-green in the same public failure category, while JSON preserves semantic
distinctions.

Therefore vNext uses exits 0/1/2 and the structured axes in section 13.

### 24.3 Analyzer recipes

#363 showed one Ruff recipe could be technically reproducible. #357 did not establish its
product value. No named recipe ships from that technical result.

### 24.4 Broad phenotype program

#357 was intentionally terminated without admitting a new user-facing phenotype.
No research candidate becomes a vNext feature by being mentioned in this SPEC.

### 24.5 Counterfactual simplification

#356 remains experimental/optional. It is not a core lifecycle reason to restore readiness.

### 24.6 Coverage infrastructure

#53 and #54 remain closed/not admitted. Coverage may be future claim-local evidence, not a
generic quality oracle or infrastructure commitment.

### 24.7 Postflight

#70's public postflight phase is superseded. Its useful bounded-execution/freshness
principles live in check-only external evidence; there is no public or hidden Postflight
lifecycle.

### 24.8 Duplication

#48 is superseded by #478/#480. Duplication remains an intended parallel product
workstream, but its detector contract is not part of the core SPEC until that research
closes.

## 25. Acceptance criteria for the core vNext contract

A vNext implementation conforms to this specification only when all of these are true:

- the public CLI contains the six intent-level commands and no old lifecycle vocabulary;
- installed hosts route lifecycle through hidden `__agent event`;
- no readiness phase exists;
- Stop and `check` use one semantic measurement/result engine;
- project build/test/typecheck ownership is truthful and separate;
- result judgment, measurement, and execution remain distinct;
- exits 0/1/2 match section 12;
- incomplete/tool/config failures consume no quality blocks and prescribe no source repair;
- same-tree pass-through and changed-tree block #2 remain bounded;
- Cursor non-blocking notices do not create agent follow-up turns;
- ask-tool partial-edit behavior needs no PostTool hook;
- evidence basis incompatibility cannot manufacture ordinary ratchet churn;
- optional finding identity is conservative under ambiguity/duplicates/version mismatch;
- `{}` is a complete meaningful policy;
- external integration work is check-only;
- native Stop remains inside the performance envelope;
- status/report/policy are read-only and schema-versioned;
- migration errors and hidden installed-hook compatibility follow section 20;
- shared parser/cache/index physical architecture remains open until its research
  rendezvous;
- #344 can be rewritten against these frozen surfaces without reopening product strategy.

## 26. Roadmap derivation rule

This SPEC does **not** itself create the implementation backlog.

After #493 is reviewed and accepted:

1. derive the minimum core implementation roadmap from the behavioral deltas between
   `main` and this document;
2. rewrite #344 against the frozen public/machine surfaces;
3. migrate CLI and host protocol without creating speculative detector infrastructure;
4. synchronize any work that would freeze shared parser/cache/index physical architecture
   with #480 Step 1 and #489;
5. continue #478 as a parallel duplication capability stream;
6. when #478 closes, create duplication implementation work from its final contract and
   integrate it through the core interfaces defined here;
7. rerun end-to-end performance evidence before shipping the combined product.

No implementation ticket may use this section to smuggle in a capability that #357 or
later targeted evidence did not admit.
