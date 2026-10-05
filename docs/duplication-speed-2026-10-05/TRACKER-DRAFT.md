# Historical pre-optimization tracker draft

Superseded by [PR-REPORT.md](PR-REPORT.md), the current report prepared for PR #490.

Corrected region-only research rejects the current prototype for production
promotion. Sorted production minimizers, conservative unsafe-unit exclusions,
explicit capped/work INCOMPLETE states and exact-text full-check confirmation
are now covered by CLI regressions and an independent exhaustive text oracle.

Five-run warm measurements include the prescribed corpora plus largest-file,
100k-token and multiplicity stress. 1M spread meets the sampled warm limits, but
100k changed tokens take 35.56 ms and klin's largest 20 files take 58.43 ms,
exceeding Stop's 15/25 ms limits. Shared-tree A/B cold measurements are recorded
in FINAL.md and results-regions/integrated.json: median paired cold regression
is 13.77%, above the 5% budget before index and lineage. Candidate-index size is 10.04% with paths at 64 bits,
9.22% with reusable paths omitted, before lineage metadata.

The owner authorized check-only calibration despite the step-1 failure. The
selection rule was committed beforehand. Blind model source review labeled all
1,923 match pairs; no candidate T=60/80/100/150 has all-copy labels in either
language. At T150, Rust has 28/42 non-copy pairs across 2 repositories and TS has
12/13 across 1. No T is selected and no 0/n precision claim is made. Original
#480 Stop-first acceptance criteria are not claimed complete.

Recommendation: reject this candidate for production integration. Contextual
check REVIEW could be a separately scoped follow-up. Generic region lineage
(#489), bounded full-check reporting and a changed precision/scope policy remain
prerequisites for such a proposal; none are hidden behind fast incomplete runs.
See docs/duplication-speed-2026-10-05/FINAL.md for all evidence and limitations.
