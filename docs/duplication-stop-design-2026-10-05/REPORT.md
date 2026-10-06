# Exact duplication at Stop: final design experiment

**Verdict: this implementation does not qualify as klin's ideal Stop design.**
Sharing traversal is feasible and preserves the tested semantics. Exact cohort
matching also passes its independent oracle. The tested lossless representations
miss the cache allowance, and full matcher construction is expensive. Generic
lineage remains unresolved; a separate TypeScript policy candidate later clears
its blind accuracy criteria, as recorded below.

These are research artifacts. No shipped check, hook, accepted debt or
configuration changed. The requested location remains Stop.

## What worked

The structural reference callback can run inside canonical normalization's
traversal. Canonical exclusions leave structural callbacks intact. The isolated
prototype passes **1,319 exact normalization comparisons**, all **12 existing
prototype tests** including the 69-case text oracle, and **203 structural CLI
checks** with the real reference callback fused. This establishes a useful
implementation seam, not its performance or complete source coverage.
See [shared traversal](SHARED-WALK.md).

The Rust suffix/LCP matcher retains exact canonical bytes, arbitrary interior
regions, self-overlap and every participating occurrence. It enumerates no
witness pairs and imposes no detection cap. **247 complete-map cases pass**,
including 1,000 identical copies. On klin/src its **20 families / 41 occurrences**
have the same complete-map digest as the previous independent diagnostics.
See [cohorts](COHORTS.md) and [raw measurements](cohorts-results.json).

## What failed

The cache stores exact dictionary bytes, token chains, unsafe boundaries, rows,
paths and extraction metadata. Five full corpus roundtrips per tested codec
preserve all of those fields. A bounded search for better compression matches
changes encoding choices only; it does not cap duplication detection or change
decoded content.

| 1M lossless encoding | Evidence cache | Plus existing candidate32 reference | Median encoding |
|---|---:|---:|---:|
| Last matching triple | 9,329,778 B | 11,514,320 B | 1,429.70 ms |
| Four previous candidates | 8,693,510 B | 10,878,052 B | 2,339.24 ms |
| Sixteen previous candidates | 8,693,502 B | 10,878,044 B | 1,922.96 ms |

The allowance is **3,730,128 B**. Even the evidence cache alone exceeds it.
The candidate number is the frozen prior REG2 index, not a measured persistent
index for the new suffix matcher. Replacing that index would not rescue these
evidence sizes. Lineage and indexed random access are absent.

The four/sixteen-candidate cache breakdown is about **1.11 MB dictionary,
4.14 MB chains, 3.16 MB rows and 0.28 MB metadata**. Searching more candidates
barely improves the four-candidate size and does not qualify encoding cost.
The prior ~254 ms encoding headroom was a subtraction from an incomplete cold
experiment, not an independently enforceable production budget. None of these
measurements establish the complete cold 5% criterion.
See [storage](STORAGE.md) and its source/provenance records.

| Exact matcher full rebuild | Files / tokens | Matcher time |
|---|---:|---:|
| klin/src | 80 / 233,276 | 126.80 ms |
| 1M fixture | 9,998 / 5,930,008 | 4,838.76 ms |

These are single complete index/report builds with normalization and output
serialization excluded. They are **not warm incremental Stop timings**. Warm
lookup against a persisted base index is unimplemented. The comparison-sort
suffix construction, nested interval visits and explicit output can be costly;
removing witness pairs does not make all remaining work bounded.

## Remaining correctness and policy boundaries

The fused integration currently computes transient complete canonical evidence
and discards it. It does not compose storage, persisted lookup or a ratchet.
Structural lossy UTF-8 conversion and whole-file parse rejection need explicit
coverage handling before a complete duplication gate can claim success.

Equal content does not establish base/current ancestry. The seven previously
documented lineage cases still need an admitted identity rule and measured
metadata. Restricting the detector to whole functions would sacrifice arbitrary
region coverage and is not used to make this experiment pass.

The 2026-10-05 matcher experiment preserves its region definition, so its speed
results alone provide no evidence of improved actionable precision. A separate
TypeScript v4 candidate passed two blind weighted reviews on a fresh-family
sample; this later policy evidence is summarized below. See
[lineage and policy](LINEAGE-AND-POLICY.md).

## Subsequent G4 policy evidence (2026-10-06)

The v4 candidate preserves v2 whole-unit keeps and requires at least 70
non-JSX tokens for additional v3 fragment keeps. Two independent reviews of a
100-pair, family-disjoint TypeScript sample from `actual` and `documenso` both
clear G4: 96.1% precision, with 49.1% and 56.2% recall. This updates the
earlier negative precision evidence for that candidate and fresh-family
population. It does not resolve parent provenance, lossless storage, or the
measured cache allowance failure; the overall Stop design remains unqualified.
See the [G4 results](../duplication-speed-2026-10-05/g4-fragments/RESULTS.md).

## Disposition

Keep the shared-traversal seam and exact matcher as research results. Do not
promote this combination to a blocking gate. The cache miss is decisive, so a
full integrated cold/warm qualification would not establish a passing design
and was not undertaken. This rejects the tested design, not every possible
lossless algorithm.

The [checklist](PLAN.md), individual reports and raw JSON retain the positive
proofs, negative results and unimplemented requirements separately. No original
acceptance criterion is considered satisfied by documenting its failure.
