# Completing the exact-region research

This checklist continues PR #490 at `815455dc`. Completion means a supported
placement decision, including a documented check-only outcome if Stop fails.
Historical measurements remain unchanged. New measurements identify the code,
normalization, equality basis, machine, workload and incomplete-state reasons.

- [x] 1. Restore region-only semantics: remove the short-function detector;
  keep names and T; exercise the binary in Rust and TypeScript CLI regressions.
- [ ] 2. Correct candidate completeness and ordering. Sorted unique production
  minimizers; decoys before a true capped partner; excluded postings; bounded
  changed-to-changed work; truncation never silently reports complete.
- [ ] 3. Freeze normalization safety. Pinned terminator/member rules, klin
  test ranges, ERROR/missing/silent-misparse eligibility, per-language counts,
  and remaining parser probes.
- [ ] 4. Independent exhaustive oracle. Compare normalized token text rather
  than detector hashes/candidates on small fixtures; threshold/edge boundaries,
  edits, repetitive streams, cross-language separation and TS/TSX equivalence.
- [ ] 5. Adversarial performance evidence. Five warm runs for 20/100 files,
  largest files, 100k changed tokens in 20 files, multiplicity
  2/10/40/63/64/65/100/1000, capped-only copies and decoys. Charge all work.
- [ ] 6. Equality and cold evidence. Measure 32/48/64-bit region keys, choose
  an explicit collision contract, and interleave repeated cold A/B runs.
- [ ] 7. Region lineage (#489). Define family/occurrence identity, seven worked
  cases, slot states and persisted evidence; test conservation and ambiguity;
  state and measure the additional index requirements.
- [ ] 8. Threshold calibration (#480 step 2). Commit the rule below before
  any labels, generate blind match packets, label all eligible matches at
  60/80/100/150, select per-language T with 0/n and repository counts, and
  measure +/-10/20. Do not calibrate a Stop candidate that failed step 1.
- [ ] 9. Integration decision (#481/#482/#478). Run the cheap-evasion and
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

- Region-only CLI suite: seven existing tests and four integration tests pass.
- Strict Clippy currently has four inherited warnings outside this fix.
- Remaining work is in progress; no performance or threshold claim is accepted
  solely because a checklist exists.
