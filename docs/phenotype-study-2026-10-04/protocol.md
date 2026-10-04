# Final phenotype-admission study protocol, 2026-10-04

This directory preregisters the final #357 phenotype-admission study. It is the
deliverable of #456. No natural-study outcome, blind label, Active/Shadow result,
or product disposition may be inspected before this protocol is committed.

## 1. Frozen measurement basis

- **study commit:** `43a139a8a16964a65ceb97b939c3499c9cf90b4d`
- **klin version:** `0.4.1`
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
- one agent change per repository;
- repositories used by the 2026-10-02 pilot agent arm are excluded entirely;
- repository size must be <=150,000 KB and the PR must touch <=100 files;
- the PR must touch at least one file of the study language;
- a root `klin.json` in the head excludes the change;
- base/head must be fetchable and have a merge base;
- agent candidates are ordered without outcome inspection by SHA-256 of
  `357-final-v1:<aiddev-pr-id>`;
- agents are walked in the fixed pilot order
  `Claude_Code, OpenAI_Codex, Cursor, Copilot, Devin, Google_Jules`;
- target: first 20 eligible changes per language. If fewer exist, take all
  eligible and do not relax any rule after measurement begins.

The matched-human arm uses the same repository and a +/-60-day creation window.
It excludes known/attributed agent PRs, requires the same language, and requires
changed-file count within [ceil(agent_files/2), min(100, 2*agent_files)].
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

`populations.tsv` freezes six real repository tasks, two per language, and
their base/reference commits. Two agent families are used:

- OpenAI Codex;
- Claude Code.

The family is frozen, not a mutable marketing model name. Every run records the
exact host version and model identifier exposed at execution. If a family is
unavailable, its row is `unavailable`; do not substitute another family.

For each task x agent family x repetition (two repetitions), run two independent
fresh sessions from the same base:

- **Shadow:** all registered candidates measure, but candidate feedback is hidden.
- **Active:** a registered finding is surfaced at its registered lifecycle
  placement and with the frozen product/research message.

That is 6 x 2 x 2 x 2 = **48 planned runs**. Pair order is deterministic from
SHA-256 of `357-arm-v1:<task-id>:<agent-family>:<repetition>`: low bit 0 means
Active first, 1 means Shadow first. Both sessions start from a clean copy of the
frozen base; neither sees the other result.

Task correctness is independent of the candidate detector. Evidence consists of:

1. the frozen task statement and acceptance facts in `populations.tsv`;
2. the base repository's project checks, discovered and recorded before either
   arm for that task executes;
3. where the historical reference change contains test-only edits, a hidden
   acceptance patch made only from those test edits and applied in a separate
   verifier copy after the run;
4. human design-intent review where executable evidence cannot prove an
   acceptance fact.

Candidate-authored tests in the new run are recorded as dependent evidence and
cannot alone establish task correctness.

## 6. Phenotype registry and denominators

`phenotypes.tsv` is normative. A detector not present there is exploratory and
cannot influence #357 admission.

Each phenotype has one explicit eligibility denominator. The core prevalence
outputs are always:

```text
affected changes / eligible changes
affected sites / eligible units
```

Unsupported, partial, unavailable, parse-failed, or local-resolution-incomplete
measurements are not zero findings. They retain their state under #354.

For TypeScript architecture candidates, a zero is countable only when all
recognized local dependency sites relevant to that claim are proved or the
candidate explicitly defines a narrower complete scope. A located alias/config
hole is `partial`/incomplete, not clean.

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

#457 generates packet IDs independently of phenotype and population, using a
random permutation seeded from SHA-256 of
`357-label-v1:<study_commit>:<sorted measurement-row ids>`. The seed and
permutation are stored only in the unblinded manifest until primary labeling is
complete.

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

`label-rules.md` is normative. Each natural finding gets exactly one primary
label:

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
evidence-packets/
packet-manifest.tsv
labels-primary.tsv
labels-secondary.tsv
adjudication.tsv
tasks.tsv
runs/
outcomes.tsv
residual-measurements.tsv
product-cost.tsv
```

Derived reports must be reproducible from those artifacts. Ephemeral model
transcripts are not required; final patches, feedback shown, command outcomes,
evidence provenance, timing and classifications are.
