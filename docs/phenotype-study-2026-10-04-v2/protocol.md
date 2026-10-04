# Final phenotype-admission study protocol, 2026-10-04 — study v2

This directory is the fresh study-v2 preregistration for the final #357
phenotype-admission study. Study v1 remains preserved at
`docs/phenotype-study-2026-10-04/`; it was superseded before any natural-study
outcome, blind label, Active/Shadow result, or product disposition was inspected.

The reset changes the executable baseline from klin 0.4.1 to the released
klin 0.4.2 at current `main`. The shipped detector source is unchanged between
the two study commits; the intervening tree changes are the #456 preregistration
artifacts and release/version metadata. This is therefore a new preregistration
version, not a claim that the v1 baseline was 0.4.2.

No study-v2 outcome may be inspected before this protocol is committed.

## 1. Frozen measurement basis

- **study commit:** `138dc8d0a927c60df289bd485627f472488cf2ba`
- **klin version:** `0.4.2`
- **baseline branch:** `main`
- **baseline date:** 2026-10-04
- **natural-population source:** AIDev v4, `hao-li/AIDev`, revision
  `c63c8a57a2de34fc03fa83722412824af4d8753b`
- **AIDev pull_request.parquet SHA-256:**
  `c0b8e81e1d099905ef9ea420bf907d45179771ff5972afce6c219d8cafcef3e8`
- **AIDev repository.parquet SHA-256:**
  `a08e34be4921c708be88a4ebd9e275b32f37fd442bb2770c0b69c94834dc6aa7`

Every replay records the exact executable SHA-256. A run whose binary is not
built from `study_commit` is a protocol deviation, not a comparable row.

### Research artifacts frozen as inputs

| Input | Artifact | Blob SHA |
| --- | --- | --- |
| #352 | `docs/finalization-lifecycle-2026-10-02.md` | `9920619c5f31abf4b5165936ca6eb19bf28e5948` |
| #353 | `docs/test-integrity-2026-10-02.md` | `6fad87b90ae9156893e7b5fffaf02abf5ec521c5` |
| #354 | `docs/evidence-provenance-2026-10-02.md` | `92f28789ea1bf618ddd9f1d37482ed0e77e8aecf` |
| #355 | `docs/design-conformance-2026-10-02.md` | `36ee25f2e7d835d377291abba7387d87f93e84ed` |
| #356 | `docs/counterfactual-2026-10-03.md` | `91a30fed0836b6654db6cdd2457fcd459c247188` |
| #361 | `docs/appeasement-audit-2026-10-02.md` | `a0234a495cfe5dfbb438ddc425de22e63cf4957b` |
| #362 | `docs/unfinished-code-2026-10-02.md` | `9077c4c7e8ab27ccd188e6ef02d06588d2decbb8` |
| #363 | `docs/analyzer-recipes-2026-10-03.md` | `6268901b4f8d39cc1d2d18f3d6fa1451123cd6e1` |
| #364 | `docs/dependency-evidence-2026-10-02.md` | `f9971260e64f2c61eaa0879e21d4040dee035683` |
| #425 | `docs/finding-identity-2026-10-03.md` | `74b4a0fe7da4072620669148b3d6f8eb627626f9` |
| pilot | `docs/phenotype-pilot-2026-10-02.md` | `5340b165cda3b0ed36352ac07cec088d476a7730` |

All research prototypes are the copies held by `study_commit`; their
measurement basis is the artifact path named in `phenotypes.tsv`. A prototype
is not silently replaced with a later implementation.

### External analyzer basis

The only named external-analyzer candidate in this study is the #363 Ruff
recipe:

```text
Ruff 0.16.10
S102,S307,S602,S604,S605,S608
BLE001,S110,S112
--isolated --no-cache --ignore-noqa
```

It runs only at readiness/Finalize and CI strength as REVIEW evidence, never as
ordinary Stop work. A different Ruff version is a different measurement basis.

## 2. Prerequisite dispositions

#435 and #445 are closed and landed on `main` before `study_commit`.

- #435 closes the demonstrated root-new-`CLAUDE.md` and nested-`AGENTS.md`
  doc-size routes. Its issue still has the many-nested-file performance checkbox
  unticked; that is recorded as a known performance-evidence hole and may not be
  rewritten as a measured result.
- #445 makes recognized local TypeScript aliases either a proved ModuleGraph
  edge or located local-resolution incompleteness. Architecture rows must
  distinguish **complete zero findings** from **incomplete because a known-local
  dependency could not be proved**.

### #356 decision

**Defer #356 counterfactual/removable implementation signals out of this vNext
admission round.**

The frozen #356 conclusion is “Keep counterfactual analysis
experimental/optional,” with authentic multi-repository tasks, faithful
minimal-feedback AX trials, real supersession history, multiple model families,
and real build/test cost still missing. #357 may cite #356 as a benchmark/research
result, but #457–#460 do not create a counterfactual phenotype row and do not
collect new trajectory solely for it.

### #48 decision

#48 canonical duplication is not in this registry. At preregistration it is an
open implementation ticket with no completed calibration/production detector.
Adding it later would be a new study version, not an amendment to these results.

Other source-ticket candidates that their frozen research explicitly rejected
are not revived here. #364's Go lock-entry candidate is also deferred rather
than treated as rejected: study v1 has no Go population, and #364 explicitly
states that no real `go.sum` entry sample was measured.

## 3. Study phases and data flow

```text
this preregistration (#456)
        |
        +--> natural measurement (#457)
        |       |
        |       +--> blinded packets --> labeling (#458)
        |
        +--> controlled Active/Shadow intervention (#459)
        |
        +--> product-cost ledger (#460)
                        |
                        v
                  #357 synthesis
```

#457, #459 and #460 may begin only after this preregistration commit lands.
#458 consumes the frozen packets from #457. #357 alone makes final product
dispositions.

## 4. Natural population

The exact selection and matching rules are in `populations.tsv`. In summary:

- languages: Rust, TypeScript/TSX, Python;
- source: the frozen AIDev revision above;
- date universe: AIDev v4's curated 2024-12-24 through 2025-10-24 population;
- closed PRs are eligible whether merged or not; merge state is preserved;
- one agent change per repository, across all three language strata. Strata
  are filled in the order Rust, TypeScript, Python; a repository taken by an
  earlier stratum is not eligible in a later one. A change belongs only to the
  stratum that took it and counts once in combined-language rates. It is
  measured for every registered phenotype whose language and eligibility rule
  it meets;
- a candidate whose head SHA equals the head of a change already taken is a
  duplicate and is skipped. A superseded or superseding PR in the same
  repository needs no other rule, because the repository cap takes at most one;
- repositories used by the 2026-10-02 pilot agent arm are excluded entirely;
- repository size must be <=150,000 KB and the PR must touch <=100 files;
- the PR must touch at least one file of the study language;
- a root `klin.json` in the head excludes the change;
- base/head must be fetchable and have a merge base;
- agent candidates are ordered without outcome inspection by SHA-256 of
  `357-final-v1:<aiddev-pr-id>`;
- agents are walked in the fixed pilot order
  `Claude_Code, OpenAI_Codex, Cursor, Copilot, Devin, Google_Jules`;
- target: first **40** eligible changes per language. This is four times the
  pilot stratum and makes a zero-observation stratum's rule-of-three upper bound
  about 7.5%, instead of 15% at n=20. If fewer than 40 exist, take all eligible
  and do not relax any rule after measurement begins.

The matched-human arm uses the same repository and a +/-60-day creation window.
It excludes a candidate when any of these holds:

- its PR id appears in the frozen AIDev pull-request table;
- its author's GitHub type is `Bot`, its login ends in `[bot]`, or its login is
  `Copilot`;
- its head branch begins with `codex/`, `cursor/`, `copilot/`, `devin/`,
  `claude/`, `jules/` or `jules-`;
- its body or any commit message contains, case-insensitively, `claude code`,
  `co-authored-by: claude`, `codex`, `cursor agent`, `cursoragent`,
  `devin`, `jules`, `copilot`, `generated with` or `🤖`.

This is “not attributed to an agent,” not proof of human-only authorship. A
candidate also requires the same study language and changed-file count within
[ceil(agent_files/2), min(100, 2*agent_files)], and must pass every
agent-arm eligibility rule except one change per repository. A candidate that
fails one is skipped and the next candidate in the order below is taken.
Choose deterministically by:

```text
same merge state first
then smallest |log2(human_files / agent_files)|
then smallest creation-time distance
then lower PR number
```

If no candidate qualifies, the agent change has no human match. Do not widen the
window or scale rule after seeing outcomes.

## 5. Controlled population

`populations.tsv` freezes nine real repository tasks, three per language, and
their base/reference commits. Two agent families are used:

- OpenAI Codex;
- Claude Code.

The agent family and model-binding procedure are frozen. Before any controlled
task is opened, measured, or run, the coordinator resolves each host once in a
neutral no-repository preflight and records the exact exposed model identifier
and host version in `tasks.tsv`. That exact model identifier is then explicitly
selected for every run in that family. If the host cannot expose and reselect
that exact model identifier, the family is recorded unavailable; do not
substitute another model or family. All runs for one agent family execute in one
contiguous study batch. If a run exposes a different model identifier from the
preflight binding, that run is invalid and the family stops until a protocol
amendment is committed. The preflight binding happens before any controlled
outcome is inspected. Each repetition uses a fresh session with no memory or
transcript from another run; a seed is recorded when the host exposes one and
otherwise recorded as `not-exposed`. `schema.json` requires that value as
`session_seed`. For every surfaced finding event, the run record preserves the
exact feedback text plus its SHA-256; Shadow events record null feedback
text/hash and no evidence-packet id.

For each task x agent family x repetition (two repetitions), run two independent
fresh sessions from the same base:

- **Shadow:** all registered candidates measure, but candidate feedback is hidden.
- **Active:** a registered finding is surfaced at its registered lifecycle
  placement and with the frozen product/research message.

That is 9 x 2 x 2 x 2 = **72 planned runs**. Pair order is deterministic from
SHA-256 of `357-arm-v1:<task-id>:<agent-family>:<repetition>`: low bit 0 means
Active first, 1 means Shadow first. Both sessions start from a clean copy of the frozen base; neither sees the
other result.

Active behaves as the intended product bundle: every registered finding that
would be surfaced at that lifecycle placement may be shown. Event-level repair
outcomes are recorded for every finding. If more than one phenotype is surfaced
in an Active run, the pair may contribute to overall closed-loop quality but
**not** to phenotype-specific Active-vs-Shadow task-benefit counts for any of
those phenotypes. A phenotype-specific pairwise benefit is countable only when
that phenotype is the sole surfaced intervention in the pair.

### Feedback construction

Feedback is frozen from the registry rather than written ad hoc during #459.

- shipped rows use the exact message/fix advice emitted by the
  `study_commit` binary;
- a `research-block-candidate` uses:
  `FAIL <id>\n  <site>  <detector observation>\n\n  <repair_surface>`;
- a `research-review` uses:
  `REVIEW <id>\n  <site>  <detector observation>\n\n  <repair_surface>\n  This is review evidence, not a compulsory repair; keep a justified exception when it preserves the task and repository contract.`;
- a `research-note` is the same shape headed `NOTE` and never blocks;
- Ruff observations use the exact Ruff 0.16.10 rule id and message as
  `<detector observation>`.

`repair_surface` is the literal field of `phenotypes.tsv`; dynamic facts are
limited to id, site and detector observation. #459 may not improve wording after
seeing a repair outcome. REVIEW delivery follows #452's hidden readiness
semantics (`klin __agent ready`); the study adds no public command.

Task correctness is independent of the candidate detector. Evidence consists of:

1. the frozen task statement and acceptance facts in `populations.tsv`;
2. the base repository's project checks. Commands written explicitly in
   `populations.tsv` are used as written. For a row that says
   `base project checks`, the coordinator freezes the command list before any
   arm runs by reading `.github/workflows/*` at the frozen base and taking, in
   workflow-path/job/step order, shell `run:` steps from `pull_request` jobs
   whose job or step name contains `test`, `check`, `type`, `lint` or
   `build` (case-insensitive). Setup/install/cache/upload/deploy steps are not
   project checks. If that produces no command, use the language fallback:
   `cargo test --workspace` for Rust; `pnpm test` when `packageManager`
   names pnpm, otherwise `npm test`, for TypeScript; `python -m pytest` for
   Python. The frozen commands are stored before Active/Shadow execution;
3. where the historical reference change contains test-only edits, a hidden
   acceptance patch made only from those test edits and applied in a separate
   verifier copy after the run. Test-only edits are test files and test-only
   hunks in other files, such as a Rust `#[cfg(test)]` module;
4. human design-intent review where executable evidence cannot prove an
   acceptance fact. The reviewer sees both final patches of a pair without
   arm, agent family or repair events.

Candidate-authored tests in the new run are recorded as dependent evidence and
cannot alone establish task correctness.

## 6. Phenotype registry and denominators

`phenotypes.tsv` is normative. A detector not present there is exploratory and
cannot influence #357 admission.

Each phenotype has one explicit **semantic** eligibility denominator. Eligibility is decided from the frozen registry and diff/tree facts before measurement success is known. Every selected natural change gets one phenotype measurement row per phenotype in `measurements.tsv`, keyed by `(change_id, phenotype_id)`. Each finding is one row in `findings.tsv`, keyed back to that measurement row. Finding multiplicity therefore never repeats a denominator. `semantic_eligible` records whether the change belongs to that phenotype's population; `eligibility_count` records its semantic units; `measured_count` records only units for which the frozen detector/prototype produced claim-complete evidence.

The core prevalence and coverage outputs are always:

```text
affected changes / semantic-eligible changes
affected sites / semantic-eligible units
fully measured semantic-eligible changes / semantic-eligible changes
sum(measured_count) / sum(eligibility_count)
```

Unsupported, partial, unavailable, parse-failed, tool-error, or local-resolution-incomplete measurements remain in the semantic denominator and are not zero findings. They retain their state under #354. Missing measurement therefore lowers coverage rather than inflating prevalence.

For TypeScript architecture candidates, a zero is countable only when all
recognized local dependency sites relevant to that claim are proved or the
candidate explicitly defines a narrower complete scope. A located alias/config
hole is `partial`/incomplete, not clean.

For `shipped-module-cycle`, derive study configuration without outcome
inspection: collect the source roots that the frozen project discovery reports,
sort/deduplicate them, place all of them in one layer named `study-all` with
`can_use: null`, and set `acyclic: true`. If that configuration cannot
represent the discovered roots, the row is unsupported rather than silently
measured under a different policy.

For the named Ruff recipe, every changed Python file passed to the recipe is semantically eligible. A Ruff parse failure is retained as `invalid-syntax` recipe evidence as #363 specifies, is not a clean zero for either family, and contributes zero measured units for that file. Tool failures and invalid syntax therefore reduce coverage; they never remove the file from the eligibility denominator.

### Frozen hard-negative population

`hard-negatives.tsv` is normative. It freezes the hard-negative case sets and deterministic inclusion rules before final-study outcomes are inspected.

- Every row selected by an applicable case-set selector is evaluated; there is no later subsampling or hand-picking.
- Frozen planted corpora (#353, #355, #362, #364 and #363) are replayed from their registered bases/routes. The pilot rows are frozen previously labeled contexts and are used only as classification controls, never as controlled-task benefit evidence.
- A phenotype with an applicable case-set mapping whose selector yields zero executable cases cannot satisfy a new user-facing admission bar in study v1; it is benchmark/defer rather than receiving a later hand-selected control.
- Hard-negative outcomes are written to `hard-negative-results.tsv` with case-set id, case id/route, phenotype, measurement state, finding ids and label/intervention outcome.

## 7. Finding identity

Use #425's frozen semantics where the family supports them.

- complexity: versioned optional structural identity;
- dead-symbols: versioned optional structural identity only for the languages
  covered by its frozen replay; otherwise current matching;
- reachability and public-api: keep their current family-specific identities;
- prototype candidates: use the exact identity described by the frozen research
  artifact;
- ambiguous or duplicate structural identities are conservative and never
  guessed;
- multiplicity is preserved.

A repeated finding count is calculated only when two records have compatible
measurement basis and compatible identity semantics.

## 8. Blind evidence packets

#457/#459 and the frozen hard-negative replay share one blind-packet namespace.
Before any packet exists, the coordinator draws a 32-byte random `salt`. Every
labelable finding gets one unique `source_finding_id` before packet ordering:

```text
natural finding:
  natural:<finding_row_id>

controlled Active finding:
  controlled:<run_id>:<finding_event_id>

hard-negative finding:
  hard-negative:<case_set_id>:<case_id>:<phenotype_id>:<finding_id>
```

`finding_event_id` is unique within a controlled run. Hard-negative
`finding_id` values are unique within one case-set/case/phenotype result.
Packets are sorted by
`(SHA256("357-label-v1:" + salt + ":" + source_finding_id), source_finding_id)`
and then numbered in that order. `packet-manifest.tsv` maps
`source_finding_id -> packet_id` plus the source-kind/join keys needed to
recover the unblinded record. The salt and source-to-packet map remain hidden
from labelers until primary labeling is complete.

Where judging remains possible, a label packet hides:

- detector/phenotype id;
- source research ticket;
- source candidate disposition/strength;
- Active/Shadow status;
- agent versus human provenance;
- agent/model identity.

The packet may reveal language, repository-relative paths, diff/context,
before/after facts, task/review context needed to judge intent, and measurement
holes. If hiding a field makes the case unjudgeable, the packet may reveal it
and must record that blindness limitation.

The packet schema is in `schema.json`.

## 9. Label contract

`label-rules.md` is normative. Each natural finding, each finding surfaced
in an Active run, and each finding emitted by a frozen hard-negative case gets
exactly one primary label and therefore one section-8 `source_finding_id`.
#459 packs a surfaced finding before its repair outcome is joined; the labeler
sees the base and the change the finding was raised on, never the later repair.
Shadow findings are measured but not surfaced or labeled and have no
evidence-packet id. A delivery is valid when its final label is
`valid-regression` or `valid-review`:

- `valid-regression`
- `valid-review`
- `undesired`
- `unresolved`

Task correctness, new/worsened status, behavior preservation, and whether human
judgment is needed are separate fields, not aliases for the four-way label.

## 10. Product decision contract

`decision-rules.md` freezes the necessary evidence bars for blocker,
Finalize/REVIEW, benchmark-only and reject/defer outcomes. No scalar quality
score exists. Prevalence, safety, repair, appeasement, attention, performance,
configuration and implementation complexity remain separate axes.

A source ticket's earlier “BLOCK candidate” or “REVIEW candidate” is a study
input, not an automatic #357 outcome.

## 11. Amendments

After this commit, a protocol change is allowed only when execution is
impossible or the frozen rule is internally inconsistent.

Every amendment must:

1. be committed before inspecting affected outcomes;
2. name the broken rule and why execution could not follow it;
3. keep old and amended rows separately recoverable;
4. bump `study_version` from `1`;
5. never loosen a rule merely because observed results are inconvenient.

Detector logic, candidate definitions, label boundaries, denominators and
decision thresholds are otherwise frozen.

## 12. Required raw artifacts

The child tickets add data below this directory without rewriting the
preregistration:

```text
natural-sample.tsv
measurements.tsv
findings.tsv
evidence-packets/
packet-manifest.tsv
labels-primary.tsv
labels-secondary.tsv
adjudication.tsv
tasks.tsv
runs/
outcomes.tsv
residual-measurements.tsv
hard-negative-results.tsv
product-cost.tsv
```

Derived reports must be reproducible from those artifacts. Ephemeral model
transcripts are not required; final patches, feedback shown, command outcomes,
evidence provenance, timing and classifications are.
