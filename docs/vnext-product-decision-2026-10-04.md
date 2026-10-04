# vNext product decision (#475)

Date: 2026-10-04

This note is the human product synthesis required by #358. It turns the
completed research in #352–#357, #361–#364 and #425, plus the accepted CLI
decision in #452, into one product contract for the end-state SPEC.

It authorizes no implementation by itself.

## 1. Decision

klin remains **deterministic quality control for coding agents**.

Its durable job is to ratchet repository-grounded invariants on the change,
give the coding agent concise repair feedback while it works, and let an
independent CI checkout enforce the same quality policy.

The product does **not** become a general-purpose "AI slop" classifier.

That means:

- optimize the loop `detect -> explain -> repair -> verify repair`;
- prefer a small number of precise, delta-aware checks to broad detector
  coverage;
- hold existing debt instead of making the repository newly red;
- keep ambiguity, missing measurement and tool failure explicit;
- do not add an AI-authorship detector, global slop/oracle score, generic
  SOLID/design-pattern judge or speculative repository-wide semantic index;
- do not admit a new user-facing phenotype capability from #357.

#357 ended without meeting the evidence bars for any new user-facing
phenotype. That is a failure to admit, not proof that every researched
phenotype is useless. Future additions need a small targeted admission study
triggered by concrete product/user demand; the 29-phenotype study is not a
standing prerequisite to rerun.

## 2. Experience contract

### UX

Normal first-class-host use requires no routine human CLI after setup.

A person should be interrupted only for:

- a concrete blocking regression;
- a genuine contextual REVIEW decision;
- an explicit measurement/integration problem that prevents required evidence.

`status`, `report` and `policy` are read-only. REVIEW must never masquerade
as FAIL, and missing evidence must never masquerade as PASS.

### DX

The public CLI is the six-command surface accepted in #452:

```text
klin setup
klin check [CHECK...]
klin status
klin report
klin policy [SECTION]
klin update
```

There is one measurement engine. Stop, readiness and public check select work
from it; they are not three implementations.

`{}` remains a meaningful complete configuration. A disputed result must expose
enough policy, measurement basis and site evidence to reproduce what klin
claimed.

### AX

Agent feedback leads with:

1. site;
2. measured fact;
3. why that fact matters;
4. repair direction.

The repair direction describes the underlying problem, not how to make a
metric disappear.

A REVIEW does not force the agent to edit until the evidence vanishes.
INCOMPLETE/UNKNOWN/tool failure does not tell the agent to invent a source-code
repair. Repeated compatible evidence is deduplicated where the finding family
has safe identity.

## 3. Execution and trust model

The internal lifecycle has three monotonically deeper placements:

```text
stop  -> ready -> check
```

A check assigned to `stop` also runs at readiness and check. A check assigned
to `ready` also runs at check. A check assigned to `check` runs only on
explicit/public check.

These are internal execution placements, not public command modes.

### 3.1 Ordinary Stop

Stop is fast, bounded, native feedback inside the coding-agent loop.

- It runs only product-owned native quality work whose placement is `stop`.
- The quality-measurement path does not launch arbitrary external analyzers.
- Existing configured/derived project-build feedback may still run in the host Stop
  loop as a separate repair signal; it is not part of klin's quality judgement
  and does not become part of the public `check` contract.
- Readiness-only work adds zero work to a Stop where readiness was not declared.
- The controlled 1M / 20-changed acceptable envelope remains <=1500 ms.
- <500 ms remains the strategic target.
- A future Stop capability needs exceptional precision, a behavior-preserving
  repair path, resistance to cheap appeasement and measured near-free
  incremental cost.

Existing host build feedback remains separate from klin's quality judgement.
Project build/test/typecheck are not part of public `klin check` and are not
part of the reusable readiness verdict defined below.

### 3.2 Explicit readiness

The hidden protocol operation is:

```text
klin __agent ready
```

It means only:

> I believe this exact working tree is ready to hand back.

It is deliberately invokable by the model but hidden from the normal human CLI.

Readiness:

- runs `stop` and `ready` product work through the shared measurement engine;
- may run bounded external evidence only when an integration is explicitly
  eligible for readiness;
- returns findings to the same agent context for repair;
- remains local Feedback, never the security/enforcement boundary;
- writes a tree-bound readiness record only under the rules in section 7.

A clarification/yield Stop does not imply readiness. Stop text, commits, tests
or model wording are never inferred as readiness.

### 3.3 Public check and CI

`klin check [CHECK...]` is the one public quality-measurement command.

- It runs all selected `stop`, `ready` and `check` klin evidence.
- It does not declare readiness and does not mutate readiness state.
- Local execution is Feedback.
- An independent required CI checkout running the same command is Enforced.
- CI uses plain `klin check`; there is no public `--ci`, `--strict`,
  `--hook`, `--postflight`, `--finalize` or `--ready` lifecycle mode.
- Project CI separately owns build/test/typecheck/install. The klin Action does
  not imply that those project commands ran.

## 4. Result model

#354's separation is normative: **judgement**, **measurement completeness** and
**execution** are independent axes.

### 4.1 Judgement

```text
pass
review
fail
```

- `pass`: no blocking finding and no REVIEW item.
- `review`: no blocking finding, but at least one item requires person
  judgement.
- `fail`: one or more blocking policy findings.

REVIEW is successful execution and does not fail CI by itself. It must remain
visible in human and machine output so repository review policy can act on it.

NOTE is an informational evidence item. It does not change judgement.

### 4.2 Measurement

```text
complete
incomplete
```

A measurement is **required** when an active native check selected for the
current placement needs it, or when a configured integration is selected for
that placement. Product-defined advisory evidence may remain optional; a
repository does not get to relabel a blocking native requirement as advisory.

A required measurement is incomplete when the requested claim could not be
established completely: unsupported scope, partial coverage, ambiguity that the
claim requires resolved, resource exhaustion, missing required tool/report, or
another explicit measurement hole.

`unknown` is an evidence/claim state, not a fourth top-level judgement.
Required UNKNOWN evidence contributes to `measurement=incomplete`. Advisory
UNKNOWN evidence remains visible as NOTE/UNKNOWN without fabricating a failure.

### 4.3 Execution

```text
ok
error
```

`error` is reserved for invalid invocation/configuration or a klin execution
failure that prevents a trustworthy run. A code-quality defect is not an
execution error.

### 4.4 Exit codes

`klin check` and `klin __agent ready` use the same category mapping:

| Exit | Meaning |
| ---: | --- |
| 0 | execution OK, required measurement complete, judgement PASS or REVIEW |
| 1 | execution OK, required measurement complete, judgement FAIL |
| 2 | invocation/configuration/klin execution ERROR |
| 3 | execution returned usable output but required measurement is INCOMPLETE |

If more than one non-success condition exists, the exit category precedence is:

```text
ERROR (2) > INCOMPLETE (3) > FAIL (1) > success (0)
```

The JSON still reports every known FAIL/REVIEW/NOTE and every measurement hole.
The single process exit code never erases the orthogonal axes.

## 5. Evidence and comparison contract

Every material claim records a measurement basis sufficient to answer what was
measured and whether two observations mean the same thing.

At minimum the basis identifies, where relevant:

- producer/check identity and measurement-semantics version;
- effective policy/rule basis;
- parser/extractor or external tool + version;
- external configuration/rules/adapter identity;
- selected scope and intended/observed coverage;
- comparison/base/against identity;
- measured working-tree identity;
- explicit incomplete/unsupported/tool-failure reasons.

Two measurements are compared as ordinary held/new/resolved/worsened evidence
only when their required basis is compatible.

A basis change is not ordinary source churn. Non-comparable evidence is
reported as such; it is never silently interpreted as a new regression,
resolution or clean measurement.

Candidate-authored tests, coverage, mutation results, external analyzers and
counterfactual runs stay claim-local evidence. None earns generic "oracle
strength" credit.

## 6. Finding identity

#425's recommendation is adopted as an extension rule, not as a universal key.

- Identity is optional and family-specific.
- Every identity contract has a version.
- A version is compared only when the implementation knows its semantics.
- Duplicate occurrences are a multiset and match one-to-one.
- Ambiguous identity never guesses a pairing that could hide a new regression.
- A family without proven structural identity keeps its conservative existing
  matching behavior.
- Identity never contains the metric value merely to make matching easier.
- Person-owned accepted entries keep their person-authored semantics; the
  product does not require users to author opaque structural ids.

A measurement-basis compatibility check happens before finding-identity
matching. Stable identity cannot make incompatible measurements comparable.

## 7. Readiness record and exact-tree invalidation

The readiness identity is the Git tree id of the working tree produced through
klin's private kept index, covering everything in the repository working tree
that Git does not ignore. `klin.json` is therefore naturally part of the tree.

A readiness run:

1. takes the existing state lock;
2. computes tree identity T0 using the kept private index;
3. runs the selected `stop` + `ready` evidence;
4. recomputes T1 and compares both the tree id and the private-index stat data;
5. records no reusable verdict if the tree drifted during the interval.

The persisted readiness record is versioned and contains at least:

```text
record_schema
tree
klin_build_identity / measurement-semantics identity
measurement_basis_digest / referenced basis records
judgement
measurement
review_count
prompt/session position sufficient for reporting
time
```

The derived readiness state is:

- **CURRENT**: same tree, same compatible measurement basis, execution was OK,
  measurement complete, and the stored judgement was PASS or REVIEW.
- **FINDINGS**: same tree/basis, complete measurement, stored judgement FAIL.
- **INCOMPLETE**: the last attempt on this tree could not complete required
  measurement.
- **STALE**: a complete record exists but tree, klin measurement semantics or
  required basis no longer match.
- **MISSING**: no applicable record exists.

Reuse rules:

- complete PASS/REVIEW/FAIL results may be reused for the identical tree and
  compatible basis **only when every contributing evidence source declares that
  its recorded basis is sufficient for reuse**;
- native deterministic evidence is reusable when its tree/policy/producer basis
  matches;
- external evidence that depends on ignored files, ambient environment,
  project-installed dependencies or another unrecorded input is non-cacheable by
  default and must rerun even when the source tree is unchanged;
- INCOMPLETE is never reused, because the missing tool/environment may have
  changed outside the Git tree;
- ERROR and drift write no reusable verdict;
- any edit that changes the tree invalidates the record immediately;
- an external-tool version/configuration/basis change invalidates reuse even
  when the source tree is unchanged.

`klin status` exposes this state without measuring anything.

## 8. Host readiness delivery

First-class integrations must make the readiness obligation agent-visible
without requiring the person to remember a command.

Claude Code and Codex may use their installed session/instruction surfaces.
For Cursor Cloud, where session-start context injection is unavailable, setup
must install/use a repository-scoped Cursor-readable instruction or skill
surface that tells the model to invoke `klin __agent ready` before the final
hand-back.

The exact host artifact path is an adapter implementation detail, but the
product rule is not optional:

- `setup` must be able to state whether readiness instruction delivery is
  installed for that host;
- `status` must report it;
- if a host cannot reliably receive the readiness obligation, klin reports the
  integration as incomplete/unsupported rather than inferring readiness from
  Stop.

A future genuine host completion event may invoke the same hidden operation. It
does not change the product contract.

## 9. Configuration philosophy and placement

`{}` remains complete. Native check placement is product-owned catalogue
metadata, not repository configuration.

Repositories cannot move native checks onto or off ordinary Stop merely by
editing a lifecycle knob.

External/integration evidence has one bounded placement choice:

```text
ready | check
```

- `check` is the default.
- `ready` means run at readiness and again at public/CI check.
- External/integration configuration cannot select `stop`.

This is the only lifecycle placement vocabulary exposed in configuration.
There are no corresponding public `check --ready` or `check --ci` flags.

Configuration expresses policy and explicit integration requirements, not klin
internal architecture. Implementation tuning knobs stay internal unless a
real user policy decision requires them.

## 10. Capability ownership boundary

### 10.1 Native klin-owned checks

Native checks are appropriate when klin can own deterministic semantics,
repository-grounded delta judgement, bounded operation and a truthful repair
contract.

No new #357 phenotype check is admitted by this decision.

### 10.2 Named klin-owned external recipes

The architecture may support a future named recipe only after that recipe
passes a targeted product-admission decision.

For a named recipe, klin would own:

- supported tool/version policy;
- fixed invocation/environment;
- adapter/normalization;
- rule/configuration basis;
- completeness and tree-binding contract;
- bounded execution/failure behavior.

The external tool still owns its detector semantics.

No Ruff, Semgrep, Gitleaks or other new named recipe ships from #475.
A future klin-owned recipe must remain offline at measurement time; network
resolution/install is not part of a klin-owned check.

### 10.3 User-owned SARIF/integration evidence

The project owns the analyzer command, analyzer version and analyzer
configuration.

klin owns:

- controlled execution boundary when it launches the command;
- report freshness/tree binding;
- completeness/UNKNOWN representation;
- changed/delta judgement;
- result rendering.

User-owned arbitrary commands do not inherit the reproducibility claim of a
named klin-owned recipe. They may use network or ambient project state only as
an explicit project-owned trust choice; that dependency becomes part of the
reported limitations and prevents klin from claiming deterministic recipe
semantics.

In vNext, user-owned external execution never runs on automatic Stop. Existing
pre-1.0 configurations that relied on that behavior receive a targeted
migration error/instruction to use readiness or check placement.

## 11. Shipped capability dispositions

#475 does not silently re-certify every shipped gate. The current product
research leads to these decisions.

### complexity

Retain the shipped gate and the post-#389 defaults, including the derived
`cc` floor of 10 and the explicit test-length policy.

The #343 replay still shows substantial residual noise, but the investigated
relaxations did not provide a principled safer rule. Do not broaden complexity
or spend Stop latency on speculative sophistication. Future changes require
targeted agent-work evidence.

### escapes

Retain the shipped deterministic spellings and the test-idiom corrections
already made. Do not pretend the gate detects every equivalent escape.
Known rewordings remain documented limitations unless a narrow deterministic
shape independently earns admission.

### stubs

Retain the narrow marker/code-stub gate and the file+kind marker-count identity
from #415. #362 does not authorize a broad unfinished/error-masking expansion.

### reachability / dead-symbols / public-api / layering

Retain their current product roles subject to their existing documented
language/measurement holes. The named re-export correction is part of the
current contract. Ambiguous graph or resolution evidence stays explicit rather
than guessed.

### doc-size

Retain the per-instruction-file policy from #435. It is not a total-context
budget. The remedy must not tell an agent to move requested narrative merely
to make the metric disappear.

### inventory / intentional test deletion

Change the vNext presentation semantics.

The first local Stop may interrupt on a newly deleted test to obtain the
agent's reason, but an intentional deletion is not made "clean" by reply-only
clearance. After acknowledgement, the deletion remains a **REVIEW** item in
readiness/check/report output; the agent's reply does not erase the evidence.

The REVIEW itself does not require a new klin acceptance workflow and does not
change exit status. The repository's normal human code-review process owns the
decision. It is not a permanent FAIL merely because the test was intentionally
removed, and the agent is not forced to recreate a test solely to clear the
REVIEW.

### build

Project build/test/typecheck remain project-owned. The current truthfulness fix
from #453 stands. klin may surface local build feedback through the host loop,
but public/CI `klin check` neither runs nor claims to have run the project
build.

### external SARIF

Keep the integration seam, but move external execution out of automatic Stop
under section 9. Existing report judgement remains delta-aware; absence,
failure or unproved report scope never reads as clean.

## 12. Public CLI and migration

The #452 six-command decision is confirmed.

### setup

`klin setup` is repository setup, not CLI-binary installation. It subsumes
current init/install repository-integration intent and is safe/idempotent to
rerun for integration repair.

The current pinning use case survives as:

```text
klin setup --pin
```

It is explicit and non-default: it freezes currently derived policy into
person-owned configuration where the underlying policy supports pinning.

### check

One public measurement command. Named checks are selectors:

```text
klin check
klin check complexity public-api
```

`--changed` may narrow scope and `--json` changes representation. Neither
changes the trust model. No lifecycle flags.

### status

Read-only integration/readiness state. It never silently checks the repository.

### report

Read-only history/outcome view: findings, repairs/resolutions, REVIEW items,
set-aside/person decisions and measurement gaps.

Default scope is the current/newest agent session. If no session is known,
report that explicitly and suggest `--since 7d`; do not silently widen scope.
Keep `--since Nd`, `--details` and `--json`. Retire public `--turn` and
ambiguous `--all`.

### policy

Read-only effective policy + provenance. It replaces policy inspection spread
across current reference/list surfaces.

### update

Retained.

### Old command migration

Before 1.0, removed public names hard-break with targeted migration errors
rather than long-lived human aliases unless external adoption evidence later
proves an alias necessary.

Generated hook/plugin commands are the exception: an updated binary must not
silently strand an already-installed integration. Time-bounded hidden legacy
dispatch compatibility may survive long enough for `klin setup` to reconcile
the installation, but it is absent from normal help and is not a public API.

The migration is:

| Current concept | vNext |
| --- | --- |
| `gate` | `check` |
| individual top-level checks | `check CHECK...` |
| `init` + `install` | `setup` |
| `init --pin` | `setup --pin` |
| `stats` | `report` |
| `reference` / `gate --list` | `policy` |
| `radius`, `guard`, hook-specific gate modes | hidden `__agent event` |
| research/finalization concept | hidden `__agent ready` |
| `turn` | no public replacement |
| `cache` | no stable public replacement unless support evidence later proves a need |

There is no public person-owned `turn reset` in the end-state product.
Prompt/session/window recovery is automatic or internal support machinery.
If an implementation cannot recover a corrupted local state safely, `setup`
may repair/recreate integration state; that does not justify a stable
turn-state vocabulary.

## 13. Machine-readable public contract

At 1.0, `check`, `status`, `report` and `policy` each expose a
schema-versioned JSON document.

The stable contract is the documented envelope and fields, not every internal
diagnostic.

The `check` envelope includes at least:

```json
{
  "schema_version": 1,
  "command": "check",
  "tree": {},
  "judgement": "pass|review|fail",
  "measurement": "complete|incomplete",
  "execution": "ok|error",
  "findings": [],
  "reviews": [],
  "notes": [],
  "measurements": [],
  "errors": []
}
```

`status` exposes integration state plus readiness
CURRENT/FINDINGS/INCOMPLETE/STALE/MISSING.

`report` exposes versioned outcome/journal records. `policy` exposes
effective values and their provenance.

Additive fields are allowed within a schema version and consumers must ignore
unknown fields. Removing/changing documented field meaning requires the
compatibility policy that #344 will define. Clearly marked debug/diagnostic
payloads are outside the stable schema.

## 14. Performance and future admission

Performance is part of product admission, not cleanup after implementation.

Ordinary Stop:

- <=1500 ms remains the current controlled 1M/20 acceptable envelope;
- <500 ms remains the strategic target;
- no external process;
- no whole-tree source walk or second parser for a candidate that can reuse
  existing delta facts;
- no new capability spends unused latency merely because the envelope permits
  it.

A new blocker needs:

- deterministic evidence at blocking strength;
- strong natural product value;
- low harmful/undesired intervention;
- a behavior-preserving repair path;
- resistance to cheap appeasement;
- bounded near-free Stop cost if placed on Stop.

A contextual capability belongs at REVIEW or stays out. A capability whose
value cannot justify its implementation/configuration/maintenance cost stays
benchmark/research evidence.

## 15. Existing candidate tickets

### #48 canonical duplication

**Defer.**

The current narrow multiplicity/fingerprint design is compatible with klin's
philosophy, but #48 was not part of the completed admission study and has no
product-value result that overrides #357's no-new-admission outcome.

Do not put it on the first vNext implementation roadmap. Preserve the design.
If real demand or repeated production evidence makes duplication important,
run a small targeted admission/calibration study and implement only if it
passes.

### #53 streaming coverage reader

**Close as not planned.**

It is infrastructure for an unadmitted changed-coverage product capability and
has little independent product value. Build the smallest reader later only if
an admitted consumer needs it.

### #54 changed coverage

**Close as not planned.**

Coverage may be useful claim-local evidence, but current research does not
justify a new coverage gate/threshold surface. Coverage is not behavioral
correctness credit.

### #70 postflight execution phase

**Close as superseded.**

Do not build the public `gate --postflight` phase or a speculative generic
postflight substrate.

Carry forward only its proven design constraints:

- phase/placement filtering before expensive requirements and I/O;
- skipped/unmeasured is not PASS;
- bounded child-process output and cleanup;
- freshness/tree binding;
- external time separate from klin time;
- no duplicate Git/diff work.

The end-state readiness/check placement model supersedes its public phase.

### #344 stability contract

**Keep, then rewrite after the end-state SPEC.**

1.0 still requires a written stability/compatibility contract. #344 must
consume the final vNext public CLI, config, JSON/exit and hidden host protocol
rather than freeze today's transitional surface.

## 16. Rejected/deferred capability conclusions

The end-state SPEC must not imply that research results below are product
commitments.

- broad test-integrity expansion: no new blocker admitted;
- generic unfinished/error-masking detection: no new capability admitted;
- design/reuse conformance: research evidence only, no product implementation;
- counterfactual simplification: experimental/optional research only;
- Ruff injection/swallowed-error named recipe: technically researched, not
  product-admitted;
- new dependency/undeclared-import review: not admitted;
- `uv.lock` and Cargo lock-entry shape: technically promising, not admitted;
- registry-existence/age checks: rejected/deferred;
- generic similarity, SOLID/pattern review and broad semantic-index expansion:
  rejected as product direction.

## 17. Answers to #358's final synthesis

1. **Durable thesis:** deterministic, repository-grounded quality control and
   repair feedback for coding agents, independently enforced in CI.
2. **Every Stop:** only product-owned native `stop` evidence.
3. **Readiness:** explicit hidden `__agent ready`, running `stop+ready`.
4. **CI authority:** independent `klin check`; project CI separately owns
   build/test/typecheck.
5. **Stop performance:** <=1500 ms current envelope, <500 ms strategic target;
   admission is evidence- and cost-gated.
6. **Result states:** orthogonal judgement, measurement and execution axes.
7. **Completeness/tool/code distinction:** section 4.
8. **Comparability:** compatible measurement basis before ordinary delta
   comparison.
9. **Finding identity:** optional, versioned, family-specific and conservative.
10. **BLOCK families:** current/native deterministic product checks only; no new
    #357 phenotype.
11. **REVIEW/NOTE/UNKNOWN:** contextual judgement, informational evidence and
    claim uncertainty respectively; none silently becomes FAIL/PASS.
12. **Required ready/check evidence:** required measurement holes produce
    INCOMPLETE, not a fake quality failure.
13. **Native/recipe/SARIF boundary:** section 10.
14. **Rejected/benchmark-only:** section 16.
15. **Configuration:** `{}` complete, policy not internals; placement only for
    integrations as `ready|check`.
16. **Trust:** local Feedback, independent CI Enforced.
17. **UX/DX/AX:** section 2.
18. **Surfaces retained/removed:** six public commands; lifecycle machinery
    hidden.
19. **Existing tickets:** section 15.
20. **New implementation work:** derive only after the end-state SPEC.
21. **CLI:** #452 six-command contract is confirmed.
22. **Build ownership:** project CI owns build/test/typecheck; klin check does
    not imply compilation.
23. **JSON/exit:** sections 4 and 13.
24. **Stop/ready/check difference:** placement and side-effect contract in
    section 3 over one engine.
25. **Readiness identity:** section 7.
26. **INCOMPLETE:** measurement axis + exit 3.
27. **Host readiness injection:** section 8, including Cursor Cloud.
28. **`init --pin`:** becomes `setup --pin`.
29. **`turn reset`:** no public replacement.
30. **Placement vocabulary:** integrations only, `ready|check`; native
    placement is catalogue-owned.
31. **`__agent event` fast dispatch:** required to dispatch before config,
    Project, base resolution or tree scanning; pre-tool remains <50 ms.

## 18. What the end-state SPEC may still choose

The SPEC may choose implementation details that do not reopen this product
decision, including:

- exact Rust types/module boundaries;
- readiness-record file name and serialization format;
- private-index file layout;
- precise JSON field shapes beneath the stable envelopes;
- exact host adapter artifact paths;
- internal catalogue representation;
- exact bounded child-process implementation;
- cache representation and optimization.

It may not silently reverse the product decisions above. A reversal requires a
new explicit product decision with evidence.

## 19. Next step

Write and adversarially review the Symphony-style end-state vNext SPEC from
this contract.

Only after that SPEC lands:

1. reconcile/close/rewrite stale candidate implementation tickets;
2. create the minimum new implementation tickets;
3. establish dependencies and milestones;
4. rewrite #344 against the stable surfaces;
5. derive public/GTM claims from the new contract rather than the historical
   roadmap.
