# O4: verified families and occurrences

Status: runnable isolated diagnostic prepared in [families.py](families.py).
The implementation consumes the frozen executable's normalized **token text**
and safe flags. Text includes the language/provenance prefix. It never uses a
fingerprint to establish equality. Current-content identity remains separate
from ancestry, which this experiment leaves unresolved.

## Experiment

Group identical complete normalized streams and safety masks with Python tuple
equality. Hashing accelerates lookup; dictionary equality compares the actual
tuple, so hash collisions cannot merge distinct streams. For each stream
class, calculate its self-overlapping maximal regions once. If multiple files
share the class, add the eligible diagonal-zero runs and expand their physical
intervals to all member files. For every distinct pair of classes, retain the
existing exhaustive diagonal text matcher as a fallback, then map its regions
to each member's physical intervals. Store families by their complete matched
text, with sets of `(file, start, end)` occurrences. No pair-edge list exists.

For bounded cases, a second calculation enumerates every original file pair
and every diagonal using [verify.py](verify.py). Equality is required for the
entire family-to-occurrence mapping, not merely a finding count. Cases cover
renamed declaration/interior regions, unsafe boundaries, languages,
self-overlap, overlapping XY/YZ families, and 64 short decoys before a real
partner. The large case has 1,000 identical already-normalized records and
requires all 1,000 whole-stream occurrences with one representative comparison.

Run from this directory:

```
python3 families.py results-optimizations/families.json
```

## Limits and decision

This is a concrete compression experiment, **not a replacement generic region
index**. Distinct whole streams can share arbitrary interior regions: they
still use an exhaustive fallback. With G distinct stream classes of L tokens,
fallback remains O(G² L²); diverse repositories receive little benefit. Highly
repetitive streams may have many distinct maximal subregion families, so
output itself remains large. Text-based family storage also needs measured
lossless encoding before any cache-budget claim.

Overlapping XY and YZ are separate keys; there is no transitive XYZ family.
Every occurrence is a physical interval, never a guessed origin certificate.
The requirements in [LINEAGE.md](LINEAGE.md), especially changed edges,
split/merge, eligibility exit and competing descendants, therefore remain
unresolved. Compressing pair edges cannot resolve them.

The diagnostic times only grouping and region reporting on already-normalized
records. It excludes file reads, parsing, normalization, index persistence and
lineage. Such timings cannot establish the Stop or cold budgets. A production
candidate would use verified region-cohort content and occurrence metadata,
preserve the generic interior matcher, and charge all retained evidence.

Primary contracts: [LINEAGE.md](LINEAGE.md) family/occurrence identity and
required stored evidence; [FINAL.md](FINAL.md) current feasibility result;
[verify.py](verify.py) independent exhaustive text oracle. Research follows
these repository-owned contracts without changing T, names or eligibility.

## Measured result

All seven bounded cases agree with the exhaustive full family/occurrence oracle.
For 1,000 identical normalized records, all 1,000 whole-stream occurrences remain,
with one representative comparison and 499,500 cross-file pair comparisons avoided.
Five grouping-only runs have a median of 2.58 ms (2.52–2.60 ms). This includes no
normalization or persisted-index work. [Raw results](results-optimizations/families.json).
Decision: retain the model, continue generic region-cohort and encoding research;
the exhaustive distinct-stream fallback is not a shipping algorithm.

## Generic exact seed diagnostic

[region_cohorts.py](region_cohorts.py) replaces the exhaustive distinct-stream
fallback in a separate diagnostic with postings for every eligible T-token
**text tuple**. Every maximal eligible exact run of length at least T contains
its first T-token tuple. The posting pair at that first position has no equal
eligible predecessor; extending it right therefore produces the entire maximal
run. Other seeds inside the same run have an equal predecessor and are skipped.
Tuple equality confirms content even if Python hashes collide. Safety masks
prevent grams or extensions crossing unsafe tokens; language and import
provenance remain part of token text. Identical stream classes retain the
existing diagonal-zero physical-occurrence expansion.

This finds interior regions between distinct streams without comparing every
possible token diagonal. Self-overlaps, overlapping families and all physical
occurrences remain eligible. It does not infer ancestry. The tuple-key prototype
uses O(NT) seed material for N eligible gram positions; extension work depends on
maximal regions. A repeated seed with M positions still visits M(M−1)/2 pairs,
and distinct maximal output can itself be large. There is no cap or silent
truncation. This deliberately simple generic index establishes correctness and
candidate selectivity, not a production memory or Stop-budget result.

Prepared validation: 246 exhaustive oracle comparisons plus one exact expected-mapping check (247 cases), including arbitrary interiors in 12 distinct
streams, unsafe boundaries, self-overlap, XY/YZ, language/import provenance,
1,000 identical records, 64 short decoys and 240 deterministic random cases.
The exhaustive diagonal oracle independently compares full family-to-occurrence
maps in bounded cases; the 1,000-record case asserts its complete known map.
All checks passed; results are recorded below.

```
python3 region_cohorts.py > results-optimizations/region-cohorts.json
```

## Suffix/LCP cohort alternative

[suffix_cohorts.py](suffix_cohorts.py) prepares a second generic diagnostic that
avoids seed witness-pair enumeration. Exact token text receives shared integer
IDs. Each eligible safe run ends in its own unique terminator, so common prefixes
cannot pass a safety boundary. A prefix-doubling suffix array and exact LCP scan
identify common-prefix intervals. At an interval of depth d, its children group
suffixes sharing the next token. Two suffixes have a maximal right extension of
d exactly when they are in different children. They are left-maximal when their
eligible predecessor tokens differ; safe-run starts have unique predecessors.

For an occurrence with child c and predecessor p, the number of maximal partners
is `total - count(c) - count(p) + count(c,p)`. A positive count adds that physical
occurrence to the exact length-d text family. This preserves every occurrence
that had a maximal partner without storing or visiting each witness pair. The
whole-stream diagonal-zero expansion across identical classes remains separate.
No transitive overlap merging or origin inference occurs.

This stdlib diagnostic uses O(N log²N) suffix-array construction. It visits all
leaves of each qualifying LCP interval; repetitive inputs can still make that
work quadratic, and reporting all distinct maximal families can require
quadratic output. Thus removing witness pairs does not imply bounded Stop work.
The same 246 exhaustive-oracle and one expected-map checks passed.

## Generic diagnostic results

Both alternatives pass all 246 bounded exhaustive full-map comparisons and the
1,000-record exact expected-map assertion. The arbitrary-interior fixture has
12 distinct streams with distinct prefixes/suffixes and one shared 30-token
interior: both retain its complete 12-occurrence family. The exhaustive oracle
visits 4,530 candidate diagonals; seed postings visit 1,254 seed pairs with 66
maximal starts; suffix cohorts visit 228 interval leaves with zero witness pairs.
These operation units differ; they are evidence of the work each implementation
performs, not equivalent timing units. On the repeated-token self-overlap case,
seed postings visit 105 pairs and suffix cohorts visit 119 interval leaves,
retaining the same 14 families and 28 occurrences. Removing witness pairs does
not remove repetitive output or traversal costs.

[Raw seed cases](results-optimizations/region-cohorts.json),
[raw suffix cases](results-optimizations/suffix-cohorts.json).

[region_corpus.py](region_corpus.py) normalizes the pinned klin production `src`
corpus once using the frozen executable, then compares both complete output maps.
The source revision is `c1805539c4d2743975915ab8452d731bac47e207`; eligibility is
the existing compression experiment's predicate. At T60, 80 files contain
233,276 normalized tokens. Both algorithms produce exactly 20 families with
41 occurrences; their complete sorted-map SHA-256 is
`46394a584153d4ea8f63a1794f1e30c5722ca5d605538d2b7a42aeab324f9c7f`.

The seed implementation has 228,483 grams, 227,963 distinct keys, 541 candidate
pairs and 21 maximal starts. The suffix implementation has 233,356 suffixes,
520 qualifying LCP intervals, 1,061 interval-leaf visits and zero witness pairs.
One diagnostic run takes 716 ms for normalization, 250 ms for seed indexing and
reporting, and 1,405 ms for suffix indexing and reporting. Combined Python peak
RSS is 300,220,416 bytes, measured over the whole process retaining both maps;
it excludes normalizer subprocess memory and is not attributed to either
algorithm. [Raw corpus result](results-optimizations/region-corpus.json).

These timings are not repeated benchmarks and are not a Stop, cold, Rust or
cache-budget qualification. There is no exhaustive corpus oracle; equivalence
on that corpus is between these two implementations, backed by the small
independent oracle checks and the maximality argument above. The suffix design
is a concrete generic interior candidate with no witness-pair enumeration.
Next qualification requires bounded-output handling, efficient Rust storage,
full charged evidence and lineage, plus repeated end-to-end measurements.

```
python3 suffix_cohorts.py > results-optimizations/suffix-cohorts.json
python3 region_corpus.py /tmp/klin490-corpus/klin/src > results-optimizations/region-corpus.json
```
