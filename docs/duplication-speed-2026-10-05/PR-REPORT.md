# PR #490: corrected research and optimization qualification

**Verdict: do not promote this candidate to a production blocking ratchet.**
Correctness defects have concrete fixes and regression coverage, and several
optimizations help, but the combined design remains unqualified. Large-file Stop
cost, artifact memory, compressed-cache runtime, generic lineage and actionable
precision remain unresolved. No original #480 acceptance criterion is claimed
complete merely because the research checklist records a negative result.

## Findings and correctness

- The short-function bypass is removed: all regions retain names and obey T.
- Production minimizer positions are sorted/unique. Capped postings, local/hit/work
  truncation and unresolved exact edges remain INCOMPLETE. The 64-decoy fixture
  demonstrates that an anchored capped key cannot certify an omitted partner.
- Full check confirms canonical token **text**; hashes only select candidates.
  The deliberate 32-bit collision adds a candidate without adding a text-confirmed
  false positive or losing the true region. Rust and TS remain separate.
- Safety coverage includes parser ERROR/missing nodes, risky TS comments,
  terminator/member handling, test exclusions and discontinuities.
- A new integration counterexample shows that structural extraction's lossy UTF-8
  conversion merges different raw string literals before the shared callback.
  A separate isolated guard marks those paths INCOMPLETE and excludes them from
  reuse; the real CLI probe passes. This guard is not the timed combined binary.
  A shipped integration must preserve raw bytes or propagate incomplete evidence.

Validation: 12 Rust tests pass, including the 69-case independent text oracle;
strict Clippy passes. O1 and O1+O2 each pass 1,319 exact differential CLI cases.
Rebuilt candidate indexes and diagnostic chains are byte-identical to the frozen
baseline on the three real corpora and synthetic 1M fixture. The combined O1/reuse
build also passes the differential/oracle checks and six-input identity probe.
Eighteen cheap-edit cases, five intentional-copy cases and nine supplied-lineage-
certificate accounting scenarios pass; the latter do not implement generic ancestry.

## Warm benchmarks

Five interleaved runs per candidate, same frozen REG2 indexes, k41/w20/t5/cap64,
query T60. Values are median incremental Stop `design_c_ms`; source reading/parsing
is recorded separately. These are standalone stage costs, not whole-hook latency.

| Workload | Corrected reference | O1 extraction | O1 + O2 streaming |
|---|---:|---:|---:|
| 1M spread, 20 changed | 6.14 ms | 4.51 ms | 4.52 ms |
| 1M spread, 100 changed | 26.61 ms | 17.06 ms | 17.23 ms |
| klin largest, 20 changed | 57.59 ms | 31.94 ms | 32.08 ms |
| klin requested100, actual80 | 101.76 ms | 58.03 ms | 58.79 ms |
| 100k changed tokens, 20 files | 36.56 ms | 23.12 ms | 23.90 ms |
| GlareDB, 100 changed | 54.11 ms | 26.07 ms | 26.20 ms |

O1 improves every sampled workload. O2 adds no measured warm speed gain, but
five-run whole-query peak-RSS medians are about 3–6% lower. Largest20 and changed100k20
still exceed the 15 ms median Stop budget; klin actual80 exceeds the 50 ms budget.
Capped/deferred work remains incomplete rather than disappearing from accounting.
The frozen pre-optimization stress/multiplicity matrix is preserved separately.

## Shared-tree cold and reuse

All hook experiments use the existing dense fixture and real binary, five
alternating pairs per phase. They consume actual fingerprints, share the existing
parsed tree and add no source reads/parses. Index/verification/lineage costs are absent.

- Corrected pre-optimization callback-off/on cold paired regression: **13.77%**.
- Original-normalizer reuse/no-reuse: **2,323.83 ms** paired median cold saving,
  with **9,998 hits**, but **148,456,596 B** retained artifact/input payload.
  Warm20/100 have zero hits because base/current changed-file bytes differ.
- O1 + reuse, same-binary callback-off/on: **1,116.34 ms / 3.37%** paired median
  cold increase. **Two of five pairs exceed 5%, maximum 5.94%.** Normalization/reuse
  median is **1,447.18 ms**; retained payload is unchanged. Warm20's negative
  whole-hook delta is noise, not a speed-gain claim.

The cold lower bound improved substantially but is not a robust whole-design pass.
The current structural owner borrows short-lived source and caches outcomes per
individual tree; attaching another map does not remove the copied bytes or memory.
Payload accounting excludes map buckets and allocator overhead, so it is not total RSS.

## Families and compact evidence

Two generic interior-region diagnostics, exact T-token seeds and suffix/LCP
cohorts, each pass **246 exhaustive full-map oracle comparisons** plus a complete
expected-map check for **1,000 identical streams**. On pinned klin/src, both produce
exactly **20 families / 41 occurrences** at diagnostic T60. Seeds visit 541 pairs;
suffix cohorts visit 1,061 interval leaves and **zero witness pairs**. Their single
Python index/report runs take about 250 / 1,405 ms, excluding normalization.
Neither proves bounded Stop work; repetitive traversal/output can remain quadratic.
No origin or lineage is inferred from equal content.

All four pinned corpora roundtrip exact lossless evidence, unsafe boundaries, rows
and compressed bytes. Dictionary, file/block directories, sparse access offsets,
paths and headers are charged. Five codec iterations on the 1M artifact give:

| Codec | Candidate32 + evidence | Compression median | Decompression median |
|---|---:|---:|---:|
| zlib default | 5,117,407 B | 399.68 ms | 17.82 ms |
| LZMA preset0 | 3,994,651 B | 447.07 ms | 155.96 ms |
| LZMA preset6 | 3,472,699 B | 3,319.77 ms | 141.33 ms |

Only preset6 fits the **3,730,128 B** size allowance, with **257,429 B** headroom
before lineage. Its Python compression stage exceeds the entire ~1,701 ms cold
allowance. Faster measured codecs fail size. These are diagnostic stage measurements,
not Rust integration; indexed random access remains unimplemented. Process RSS
includes interpreter/raw buffers and possible inherited macOS launch high-water.
No complete cache/runtime feasibility claim follows.

## Precision and next decision

The owner authorized check-only calibration after Stop failed. The rule was committed
as `13655f91` before labels. Blind model review covers **1,923 pairs / 542 families**
with pinned-source evidence; these are model judgments, not human calibration.
**No T=60/80/100/150 qualifies in either language.** Even at T150, non-copy labels
are Rust **28/42** across two repositories and TS **12/13** across one.
Faster exact matching does not make benign equality actionable copying.

Recommendation: validate a contextual check-only REVIEW report with human review
before further Stop integration. Read source when needed rather than assume the
synchronous lossless cache is viable. Require conservative inherited/new/ambiguous
lineage and explicit incomplete handling; do not claim a blocking threshold.

## Reports and reproducibility

- [Corrected baseline verdict](FINAL.md), [nine-point checklist](CHECKLIST.md),
  [normalization contract](../duplication-normalization-2026-10-05.md), [lineage](LINEAGE.md).
- [Five-track optimization ledger](OPTIMIZATIONS.md), [reuse](OPT-REUSE.md),
  [families](OPT-FAMILIES.md), [compression](OPT-COMPRESSION.md).
- [Warm benchmark rows](results-optimizations/warm.jsonl),
  [summary and executable hashes](results-optimizations/warm-summary.json),
  [memory](results-optimizations/memory.json),
  [combined hook data](results-optimizations/combined.json),
  [codec profile](results-optimizations/compression-blocks-profile-1m.json),
  [generic corpus result](results-optimizations/region-corpus.json).
- [Frozen baseline provenance](results-regions/provenance.json),
  [baseline source archive](results-optimizations/reference-source.tar.gz),
  [combined source patch/provenance](results-optimizations/combined-provenance.json),
  [calibration verdict](calibration/README.md).

Everything remains research code and documentation; shipped klin, klin.json,
hooks and accepted debt are unchanged. The documented rerun uses fresh output/index
storage, preserving the frozen measurements. Five optimization tracks have measured
outcomes; the combined production design is not complete.
