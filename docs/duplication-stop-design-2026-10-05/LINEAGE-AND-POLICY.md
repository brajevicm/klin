# What this design still has to prove

The new traversal, storage and cohort experiments change how exact current
content is computed. They do not change the
[seven lineage cases](../duplication-speed-2026-10-05/LINEAGE.md) or establish
that equal code is actionable duplication.

## Ancestry

Unchanged source under the same measurement basis can reuse base evidence.
This also requires unchanged provenance inputs: equal file bytes alone do not
prove equal normalized tokens when the import environment changes.

For edited paths, matching canonical bytes establishes family membership,
not which old physical occurrence became which current occurrence. Cohort
counts cannot safely turn a removed occurrence into a free allowance: it may
have a descendant whose content or eligibility changed. The cases involving
extensions, splits, merges and competing descendants remain unresolved.

The narrow previously proposed certificate is an entire supported structural
role inside a same-path, uniquely identified declaration. It does not cover
every arbitrary interior region. Restricting matching to those roles would
reduce detection coverage, so it is not an acceptable shortcut in this attempt.

Consequently this prototype must not claim a complete inherited/new ratchet.
It would need a separately specified, validated identity rule and its measured
storage and execution costs. An unknown ancestor must remain unknown; equality,
line proximity and a greedy assignment are insufficient certificates.

## Blocking policy

The previous blind model calibration found no qualifying threshold among
T=60/80/100/150 in either Rust or TypeScript. At T150, non-copy labels were
28/42 in Rust and 12/13 in TypeScript. These were model judgments rather than
human validation; see the [calibration report](../duplication-speed-2026-10-05/calibration/README.md).

The new implementation keeps the same token equality and region definition.
It therefore provides no evidence that those precision results improve.
Selecting longer regions, reporting fewer matches or suppressing popular
cohorts would require a new policy evaluation and could lose wanted copies.

The requested placement remains Stop. This note does not move verification
elsewhere. It records two requirements that a complete Stop gate still lacks,
even if every storage and matching benchmark were to pass.
