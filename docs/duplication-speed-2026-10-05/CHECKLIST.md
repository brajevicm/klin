# Completing the exact-region research

This checklist continues PR #490 at `815455dc`. Completion means a supported
placement decision, including a documented check-only outcome if Stop fails.
Historical measurements remain unchanged. New measurements identify the code,
normalization, equality basis, machine, workload and incomplete-state reasons.

- [x] 1. Restore region-only semantics: remove the short-function detector;
  keep names and T; exercise the binary in Rust and TypeScript CLI regressions.
- [x] 2. Correct candidate completeness and ordering. Sorted unique production
  minimizers; decoys before a true capped partner; excluded postings; bounded
  changed-to-changed work; truncation never silently reports complete.
- [x] 3. Freeze normalization safety. Pinned terminator/member rules, klin
  test ranges, ERROR/missing/silent-misparse eligibility, per-language counts,
  and remaining parser probes.
- [x] 4. Independent exhaustive oracle. Compare normalized token text rather
  than detector hashes/candidates on small fixtures; threshold/edge boundaries,
  edits, repetitive streams, cross-language separation and TS/TSX equivalence.
- [x] 5. Adversarial performance evidence. Five warm runs for 20/100 files,
  largest files, 100k changed tokens in 20 files, multiplicity
  2/10/40/63/64/65/100/1000, capped-only copies and decoys. Charge all work.
- [x] 6. Equality and cold evidence. Measure 32/48/64-bit region keys, choose
  an explicit collision contract, and interleave repeated cold A/B runs.
- [x] 7. Region lineage assessment (#489). Record family/occurrence identity,
  seven worked cases, slot states and required persisted evidence; test supplied-
  certificate accounting; measure the full-chain alternative and state the
  unmeasured compact-design requirements. Outcome: generic ratchet feasibility
  remains unproven; #489 is not closed and no ancestry implementation is claimed.
- [x] 8. Threshold calibration (#480 step 2). Commit the rule below before
  any labels, generate blind match packets, label all eligible matches at
  60/80/100/150, select per-language T with 0/n and repository counts, and
  measure +/-10/20. Owner explicitly authorized check-only calibration after step-1 failure;
  no Stop eligibility or original #480 acceptance claim follows.
- [x] 9. Integration decision (#481/#482/#478). Run the cheap-evasion and
  intentional-duplicate cases against the frozen candidate; measure actual
  shared-tree end-to-end marginal cost in the existing performance harness;
  conclude Stop BLOCK/REVIEW, check REVIEW, or reject, with residual limits.

## Threshold selection rule (before labels)

For each language, select the lowest candidate T in 60/80/100/150 at which
every labeled eligible production match is `copy`. Labels are `copy`,
`boilerplate`, `required-shape`, `generated`, `distinct`, or `mixed`, with the
definitions in #480. A zero-match sample cannot establish precision. Label
packets conceal T and selected length; the evidence retains an auditable
mapping. Report non-copy/total as 0/n and the number of repositories. Include
T +/-10 and +/-20 after selection. If no candidate qualifies, report no
selected T. Do not manufacture labels or select a threshold from synthetic
fixtures. Calibration follows corrected speed evidence and frozen safety.

## Execution record

- Rule committed as `13655f91` before labels; #1 completed first.
- Twelve Rust tests pass, including the 69-case independent CLI text oracle;
  strict Clippy passes. Eighteen edit and five intentional-duplicate fixtures
  pass; nine supplied-certificate accounting scenarios pass.
- Five-run standalone matrix completed against the frozen candidate. Large-
  change Stop budgets fail; capped/deferred queries remain INCOMPLETE.
- All 1,923 blind pairs reviewed with pinned-source citations. No T qualifies
  in either language. Labels are model judgments, not human calibration.
- Final frozen shared-tree A/B: five pairs per phase passed. Cold paired
  regression is 13.77% before index/lineage, exceeding the 5% limit.
  Recommendation: reject production promotion. PR update is recorded in [PR-REPORT.md](PR-REPORT.md); original #480 acceptance criteria are not claimed complete.
- The research outcome is a negative feasibility/precision verdict. Generic
  lineage, a selected product threshold and shipped integration are not
  fabricated to complete the checklist. See [FINAL.md](FINAL.md).

- Follow-up optimization work: [five tracked experiments and results](OPTIMIZATIONS.md).
  O1/O2 preserve measured outputs; large-file and full-integration qualification
  remain open. Baseline evidence above is unchanged.
