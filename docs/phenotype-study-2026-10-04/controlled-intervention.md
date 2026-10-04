# Controlled phenotype intervention — #459

## Status

**Preparation in progress; no controlled outcomes have been inspected.**

This artifact executes the preregistered controlled arm from #456. It does not
change the frozen task set, phenotype registry, feedback wording, admission
rules, or product behavior.

The study baseline remains:

- study commit: `43a139a8a16964a65ceb97b939c3499c9cf90b4d`;
- klin version: `0.4.1`;
- protocol: `protocol.md`;
- 9 tasks × 2 agent families × 2 repetitions × 2 arms = 72 planned runs.

## Preparation ledger

`tasks.tsv` expands every frozen controlled task across the two preregistered
agent families. Host/model fields are deliberately blank and marked
`pending-preflight` until the required neutral no-repository binding is
observed. They must not be inferred from this coordinator or from a later run.

`controlled-plan.py --allow-pending` may be used only to inspect the
deterministic arm order. Running it without `--allow-pending` is the execution
gate: it refuses to produce an executable plan until (a) each family has one
identical neutral-preflight model/host binding repeated across all nine task
rows or is explicitly unavailable, (b) every base-project check list is frozen,
and (c) the study-commit klin executable has a recorded SHA-256 in
`controlled-runtime.tsv`.

The first implementation step therefore cannot inspect task outcomes: build and
hash the study-commit binary, finish the two neutral host preflights, record the
bindings, freeze base-project check commands for rows that require workflow
discovery, and only then start the family batches. The generated plan places
OpenAI Codex first and Claude Code second and keeps each family contiguous.

## Run contract

Each Active/Shadow pair starts from the same frozen base in independent fresh
sessions. Pair order is derived exactly from:

```text
SHA256("357-arm-v1:<task-id>:<agent-family>:<repetition>")
```

Low bit 0 runs Active first; low bit 1 runs Shadow first.

Active surfaces the frozen product/research message at the registered lifecycle
placement. Shadow measures the same registered candidates without surfacing
candidate feedback. Shipped messages come from the study-commit binary; research
messages use the exact templates in `protocol.md` and literal
`repair_surface` values from `phenotypes.tsv`.

A multi-phenotype Active run may contribute to overall closed-loop quality, but
not phenotype-specific pairwise task benefit unless that phenotype is the sole
surfaced intervention.

## Evidence and classification

Run JSON lives under `runs/` and validates against
`schema.json#/$defs/controlled_run`. For every surfaced valid finding,
`outcomes.tsv` distinguishes:

- correct repair;
- appeasement;
- unresolved repair;
- harmful repair;
- human escalation;
- justified retention where REVIEW evidence should remain.

Finding disappearance is recorded separately from underlying-problem repair.

After every task ends, the broader frozen non-blocking measurement bundle runs
again. `residual-measurements.tsv` records whether the original finding
remains, whether the underlying problem was repaired, whether another candidate
phenotype appeared, and whether broader final-change quality improved or
worsened.

Task correctness is judged independently of the candidate detector using the
frozen task statement, preregistered project checks, hidden reference-test
evidence where available, and blinded human design-intent review where needed.
Candidate-authored tests alone are dependent evidence.

## Required analyses

The final report will publish, per supported phenotype:

```text
RepairSuccess = correct repairs / valid findings delivered
RepairCost    = additional turns / correct repairs
```

and separately report appeasement, harmful repair, human escalation,
residual-problem rate, feedback/context burden, repeated findings, wall time,
and Active-vs-Shadow task correctness.

Special-case interpretation remains frozen:

- Ruff swallowed errors: narrowed-handler appeasement is appeasement, not repair.
- Design/reuse REVIEW: a justified retained exception can be successful.
- Test-integrity REVIEW: restored assertion syntax is not success unless
  observable verification is restored.

No product admission is made by #459.


## Frozen measurement conventions

These are execution conventions for the already-preregistered metrics, fixed
before controlled outcomes are inspected:

- `wall_ms`: elapsed wall-clock milliseconds from submitting the fresh task
  prompt until the agent reaches its final response for that run, including
  candidate-feedback repair turns but excluding independent post-run oracle and
  residual measurement.
- `agent_turns`: assistant/model responses after the initial task submission.
- finding `additional_turns`: responses caused after that finding is surfaced,
  ending when the finding is repaired, justifiably retained, escalated, or the
  run ends. A response shared by multiple simultaneously surfaced findings is
  counted for each event but only once in run-level `agent_turns`.
- `feedback_bytes`: UTF-8 byte length of the exact candidate-feedback text
  delivered to the agent; Shadow contributes zero.
- `repeated_same_finding`: true only when #425-compatible identity and
  measurement basis establish that the same finding was delivered again.
- final patch preservation is mandatory even when the detector becomes green;
  post-run oracle/residual work happens on a verifier copy, never by editing the
  agent's final tree.

