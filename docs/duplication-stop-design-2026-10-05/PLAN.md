# One final exact Stop design experiment

Status: research attempt complete, negative verdict. See [REPORT.md](REPORT.md).
This is a new experiment, not a replacement for the frozen
[PR #490 report](../duplication-speed-2026-10-05/PR-REPORT.md).
The duplication gate must complete its blocking decision at Stop. An expensive
check-only verifier does not satisfy this experiment.

## Checklist

- [x] Preserve canonical token bytes, language/import provenance, rows, test
  exclusions and unsafe boundaries in a shared structural reference traversal.
  Compare against the corrected normalizer before measuring speed.
- [x] Measure fast lossless dictionary/chain storage, including construction,
  decoding, metadata and candidate-index bytes. Pin roundtrip and corrupt-input
  rejection through a runnable CLI check.
- [x] Implement uncapped exact occurrence cohorts without witness-pair enumeration.
  Compare the entire maximal-region/occurrence map against the independent oracle.
- [ ] Measure cold construction and warm changed-file matching on the existing
  pinned corpora, including large-file and high-multiplicity cases. Preserve raw
  rows, source identity and executable identity.
- [ ] Establish what base/current ancestry the design actually certifies and
  charge its persisted metadata and computation. Ambiguity is incomplete evidence.
- [x] Apply the precision results: exact equality alone did not qualify a
  blocking threshold. The v4 TypeScript candidate later passed two blind
  weighted reviews on a fresh-family sample; this does not settle the other
  design requirements.
- [x] Record a verdict for the complete design, including failed criteria.

Storage fails the allowance even without candidate bytes. Full matcher
construction was measured, but integrated cold/warm qualification and generic
ancestry remain unchecked above: they were not implemented after that decisive
failure. Checked experiments are not passing production acceptance criteria.
The shared integration still needs explicit raw-byte/parse coverage handling.

## Fixed criteria

Warm20: median incremental cost at most 15 ms and maximum at most 25 ms.
Warm100: median at most 50 ms. Cold: at most 5% incremental whole-hook cost.
The 1M fixture's candidate plus evidence cache allowance is 3,730,128 bytes;
the existing 32-bit candidate index alone costs 2,184,542 bytes. Its hashes may
select candidates, but complete canonical bytes must establish equality.
Do not silently cap matching, discard occurrences or relax normalization to fit.

The prior ~254 ms encoding headroom is a diagnostic subtraction from an
incomplete cold experiment, not a guaranteed budget for this implementation.
Full decoding memory, index construction, lookup, exact verification, lineage
and source freshness remain part of the design's cost.

## Smallest design being tested

Reuse the full reference traversal in `Reading::uses` rather than add another
whole-tree token walk. Structural extraction first runs its query captures;
duplication capture must leave all existing structural observations intact.
Its own excluded nodes must not prune the shared traversal.

Store exact sorted dictionary bytes and lossless token-ID chains with rows and
boundary metadata. Dictionary IDs from separate caches are not compatible
identities unless remapped by exact bytes. No new general-purpose codec is needed.

Group matching occurrences by exact regions rather than build all partner edges.
The prior suffix/LCP diagnostic supplies an oracle-tested starting point;
it does not prove bounded runtime or generic lineage.

## Decision boundary

Matching, ancestry and actionable policy are three different requirements.
If any required stage fails, report that failure. A speed win or exact roundtrip
does not qualify the full gate, and a partial ancestry rule must not be presented
as arbitrary-region lineage. The earlier negative precision calibration
remains part of the baseline; the v4 TypeScript candidate later passed two blind
weighted reviews on a fresh-family sample. This updates policy evidence but
does not resolve storage or lineage requirements. See the
[G4 results](../duplication-speed-2026-10-05/g4-fragments/RESULTS.md).
