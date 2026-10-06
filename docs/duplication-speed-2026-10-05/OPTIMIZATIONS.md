# Exact-region optimization ledger

Baseline: corrected candidate documented in FINAL.md; frozen executable/source
under /tmp/klin490-opt-reference for this experiment, with hashes and a durable source archive in results-optimizations/reference.json
and reference-source.tar.gz.
These experiments preserve the region-only contract, token text, safety ranges,
import provenance, T and minimizer parameters. Performance improvements cannot
turn benign equal code into actionable copying. The existing no-selected-T and
unresolved-lineage conclusions stand until separate evidence changes them.

| ID | Work | Status | Correctness obligation | Acceptance evidence |
|---|---|---|---|---|
| O1 | Combine exclusion/safety/token extraction; remove repeated prefix/range scans | Implemented; equivalent; warm timings improved | Identical text, rows, safe flags and unsafe counts across baseline/candidate | CLI oracle + exhaustive output comparison on pinned corpora and safety stress; stage timings |
| O2 | Bounded streaming minimizers; hash selected large grams only; ordered output without sort | Implemented; equivalent; no measured warm speed gain | Identical selected positions/keys for ties, short/repetitive/random inputs; no lost >=T candidate | Differential CLI output, oracle, warm/cold stage timing and buffer bounds |
| O3 | Reuse same-input derived artifacts within a run | Isolated; validated; five-pair A/B complete | Full byte equality plus path/language/basis; changed inputs invalidate reuse | Actual repeated-input counts, retained artifact size, reuse experiment and A/B |
| O4 | Verified families/occurrences instead of all pair edges | Two generic diagnostics verified; corpus maps agree | Every witness/occurrence preserved; overlaps separate; no inferred lineage | Multiplicity/decoy/overlap cases, grouped-output cost, remaining proof obligations |
| O5 | Compact candidate keys separately from lossless equality evidence | Compressed32 format fits size-only; runtime/lineage pending | Collisions only add candidates; text confirms; truncation remains incomplete | 32/48/64-bit tradeoffs, collision checks, total verification/lineage bytes |

## Measurement protocol

Keep the original results-regions untouched. New evidence goes in
results-optimizations. Attribute each change separately before combining them.
Run differential correctness before timing. Time serially on the same prepared
roots and binary settings; include reading/parsing separately and charge all
retained artifacts and work. No performance claim from count-only mock reuse or
from dropping postings, changing T, ignoring unsafe code, or discarding lineage.
At cold the 5% budget is about1.61s versus4.36s normalization alone; largest-file
warm cost58.43ms requires about3.9x to meet15ms. Preserve INCOMPLETE and contextual
review; no hash-only BLOCK. Whole production feasibility requires full index,
verification and lineage costs, not these partial experiments.

## Execution record

- O1 combines extraction/exclusion/safety traversal, caches exclusion rows, and
  applies merged unsafe ranges with a forward scan. O2 uses bounded rolling
  windows and ordered selection, and scans unsafe tokens without a prefix array.
- O1 and O1+O2 each pass 1,319 exact differential CLI cases. The candidate passes
  12 Rust tests, including the 69-case independent text oracle, and strict Clippy.
  Source patches and executable hashes accompany those results.
- O3 retains actual artifacts keyed by full bytes and path. Its six-call CLI
  identity probe passes (two hits, four derived inputs); A/B measures actual reuse.
- O4 matches the full oracle family/occurrence mapping in seven bounded cases.
  Its 1,000-identical-stream case preserves every occurrence with one
  representative comparison. Diverse stream classes still use quadratic fallback.
- O5's 1M dictionary/varint chain needs 16,608,635 equality bytes. Even 32-bit
  candidates plus equality need 18,793,177 bytes versus the 3,730,128-byte budget,
  before lineage and access offsets. This simple format fails; it does not prove
  all possible lossless encodings fail.

Detailed obligations and limits: [O3](OPT-REUSE.md), [O4](OPT-FAMILIES.md),
[O5](OPT-COMPRESSION.md).

## Remaining qualification checklist

- [x] Freeze baseline source/binary and keep baseline results unchanged.
- [x] Implement O1 and O2 without changing normalization or candidate semantics.
- [x] Differentially compare both stages on real corpora and adversarial inputs.
- [x] Compare serialized candidate keys/chains byte-for-byte with frozen indexes.
- [x] Attribute O1 and O2 warm costs with five interleaved runs per workload.
- [x] Implement and validate full-input/path reuse using actual retained artifacts.
- [x] Complete five alternating reuse/no-reuse pairs for all three hook workloads.
- [x] Validate exact family/occurrence mapping and 1,000-stream multiplicity case.
- [x] Record family grouping microtimings, with normalization explicitly excluded.
- [x] Measure lossless equality encoding on the 1M fixture.
- [x] Roundtrip the three real corpora and run candidate-width collision probes.
- [x] Record individual decisions and next work after measurements.
- [ ] Qualify a combined design against full Stop/cold/cache budgets, including
      index construction, lossless verification, access metadata and lineage.
- [ ] Resolve false-positive/selection policy and generic region lineage before
      claiming a complete blocking research result.

## Serial warm attribution

Five interleaved runs per candidate/workload, using the same frozen indexes,
k41/w20/t5/cap64 and query T60. All non-timing query fields agree within every
comparison. `design_c_ms` measures incremental Stop work; reading/parsing is
reported separately in the raw rows and is not included in this column.

| Workload | Reference median | O1 median | O1+O2 median |
|---|---:|---:|---:|
| 1m-64-100 | 26.61 ms | 17.06 ms | 17.23 ms |
| 1m-64-20 | 6.14 ms | 4.51 ms | 4.52 ms |
| changed-100k-20 | 36.56 ms | 23.12 ms | 23.90 ms |
| glaredb-64-100 | 54.11 ms | 26.07 ms | 26.20 ms |
| klin-largest-64-100 | 101.76 ms | 58.03 ms | 58.79 ms |
| klin-largest-64-20 | 57.59 ms | 31.94 ms | 32.08 ms |

[Raw warm rows](results-optimizations/warm.jsonl),
[summary and binary hashes](results-optimizations/warm-summary.json),
[runnable measurement](opt_measure.py). The three real-corpus and 1M rebuilt
`index.bin` and `chain.bin` files are byte-identical to frozen baseline indexes;
[digests](results-optimizations/index-equivalence.json) cover candidate keys as
well as the differential diagnostic's selected positions.

## Decisions and next work

- **O1: retain.** It removes redundant traversal/range work and improves each
  measured workload. Largest20 still exceeds both the 15 ms median and 25 ms
  maximum budgets; klin's requested100 workload (80 actual files) still exceeds
  50 ms. Changed100k20 also remains above the 15 ms median budget.
- **O2: experimental.** Bounded selector buffers and no unsafe prefix array are
  useful structural properties, but five runs show no additional warm speed
  gain. Compare peak memory and stage costs before choosing this extra streaming
  complexity over O1 alone. The prototype currently includes both stages; their
  patches are separately archived. Neither changes token or candidate semantics.
- **O3: useful cold opportunity, unsuitable standalone cache.** The paired
  median cold saving is 2,323.83 ms, with 9,998 actual hits and 148,456,596 bytes
  retained payload. Warm queries have zero hits. Prefer an existing artifact
  lifetime/owner; measure O1 plus shared-owner reuse rather than adding this map.
- **O4: retain the family/occurrence model.** Seven oracle mappings agree;
  1,000 identical streams need one representative comparison, preserving all
  occurrences. Grouping-only median is about 2.58 ms. A generic region-cohort
  algorithm and lossless storage must replace the distinct-stream quadratic
  fallback before shipping. No ancestry is inferred.
- **O5: reject the initial varint format for the 10% budget.** All pinned corpus
  roundtrips and collision probes pass. Equality evidence alone is too large at
  1M. Measure block/dictionary compression on the actual token corpus next, with
  random-access costs and metadata charged; no need to implement a new codec yet.

The next qualification milestone is an O1 + existing-owner reuse experiment,
with full index, verification and lineage costs included. These optimization
results do not change the failed blind threshold calibration or provide a
false-positive policy. There is no production feasibility claim.

## Second qualification pass

- O1 + O3 lower-bound hook experiment: frozen O1 (without O2) replaces the old
  normalizer inside the isolated reuse build. The callback is disabled/enabled
  in the same binary, with reuse enabled whenever derivation runs. Five
  alternating pairs cover warm20, warm100 and cold. This compares against no
  duplication derivation, unlike the previous reuse/no-reuse experiment.
- Combined CLI: the six-input cache-identity probe, all 1,319 differential
  normalization cases and all 69 independent oracle/probe cases pass.
- Shared-owner investigation: `ParsedFile` borrows source, and `Extracted`
  caches structural `Outcome`s separately for each tree. Neither currently
  retains cross-tree full-byte identity with the lifetime this reuse map needs.
  Merely moving the map under `Extracted` would not remove the copied source or
  retained artifact cost. Existing-owner integration is therefore still open.
- O2: measure OS peak RSS on the large-file workloads before selecting the
  streaming implementation; bounded selector buffers do not alone prove lower
  total memory.
- O4: validate exact T-token seed cohorts as a generic interior-region candidate
  path; no candidate caps or guessed lineage.
- O5: standard block compression experiment includes random-access directories
  and sparse token/row offsets, beyond the initial varint-size diagnostic.

Source pin: [combined provenance](results-optimizations/combined-provenance.json)
and [isolated patch](results-optimizations/combined.patch). These changes remain
outside shipped klin. Measurements exclude full index persistence and lineage;
even a passing lower bound cannot qualify the whole design.

### O1 + O3 same-binary hook result

Five alternating off/on pairs per phase, 30 real hook runs. The callback is
always off for the reference and derives/reuses actual artifacts for the enabled
run. All enabled counters/checksums are stable within each phase; the integrated
normalizer also passes 1,319 differential and 69 text-oracle/probe cases.

| Phase | Off median | Enabled median | Paired median delta | Paired median delta % |
|---|---:|---:|---:|---:|
| warm20 | 1454.10 ms | 1417.31 ms | -50.27 ms | -3.46% |
| warm100 | 1762.72 ms | 1774.19 ms | 11.76 ms | 0.67% |
| cold | 34028.56 ms | 34731.50 ms | 1116.34 ms | 3.37% |

Cold normalization/reuse median is 1,447.18 ms with 9,998 hits. The paired median
cold increase is below 5%, **but two of five cold pairs exceed 5%** (maximum
5.94%). This is promising lower-bound evidence, not a robust whole-design pass.
The 148,456,596 B payload remains. Index construction/persistence, lossless
verification and lineage are excluded. Warm20's negative whole-hook difference
is noise/order variability, not an optimization claim: it still adds about
6.14 ms of measured derivation and has zero reuse hits. Warm100 adds about
27.70 ms derivation, also with zero hits.

[Combined raw data](results-optimizations/combined.json),
[oracle](results-optimizations/combined-oracle.json),
[differential](results-optimizations/combined-differential.json).

### O2 memory result

Five interleaved runs per candidate/workload; an isolated child provides each
OS peak-RSS sample (`getrusage`, macOS byte units). These are whole query-process
peaks, not allocated-buffer accounting. O1+O2 median peak is about 3–6% lower:

| Workload | O1 peak median | O1+O2 peak median |
|---|---:|---:|
| changed-100k-20 | 6,324,224 B | 5,947,392 B |
| klin-largest-64-100 | 12,386,304 B | 11,829,248 B |
| klin-largest-64-20 | 9,371,648 B | 9,060,352 B |

[Memory rows](results-optimizations/memory.json), [runnable check](opt_memory.py).
Decision: keep O2 as the verified bounded-memory alternative; its measured RSS
benefit is modest, and it does not solve the large-file Stop-time failure.

### O4 generic cohort qualification

Two new diagnostics replace exhaustive distinct-stream matching:
[exact T-token seeds](region_cohorts.py) and [suffix/LCP cohorts](suffix_cohorts.py).
Each passes 246 exhaustive full-map oracle comparisons plus the 1,000-identical
stream exact expected-map check. On pinned klin/src (80 files, 233,276 tokens),
they produce identical complete maps: 20 families and 41 occurrences at diagnostic
T60. The seed index visits 541 seed pairs and 21 maximal starts; suffix cohorts
visit 1,061 interval leaves with zero witness-pair enumeration.

This resolves the identical-stream-only limitation of the first experiment.
The Python seed index/report takes about 250 ms; suffix construction/report about
1,405 ms. Normalization is measured separately (~716 ms). The combined Python
process peaks at 300,220,416 B, excluding child processes; it is not a per-algorithm
or Rust memory estimate. [Corpus evidence](results-optimizations/region-corpus.json)
asserts full map equality. Suffix traversal and output can still be quadratic;
seed multiplicity can still be quadratic. Neither establishes bounded Stop
processing or lineage. T60 here is a diagnostic setting, not selected product policy.

### O5 size follow-up

LZMA block encoding charges dictionary and ID access offsets, token chains, rows,
paths, file directories, sparse token/row offsets, block directories and headers.
All four pinned corpora roundtrip exact evidence. Compressing the frozen candidate
bytes too gives 3,472,699 B at 1M for 32-bit candidates plus evidence: 257,429 B
under the 3,730,128 B allowance. 48/64-bit combinations and zlib still fail.
[Size results](results-optimizations/compression-blocks-candidates-1m.json).

This replaces the initial negative size-only verdict for that specific 32-bit
combination. It excludes lineage, and does not yet qualify lookup/update costs,
codec memory, cold compression work or collision-amplified Stop work. The 32-bit
intentional-collision fixture's full text confirmation still has no false positive
or missed true region. No hash-only BLOCK is introduced.

### Raw-source correctness pin

The shared callback receives structural extraction's lossy UTF-8 source. A new
raw-byte counterexample shows that callback byte identity alone cannot certify
all original inputs. The guarded isolated variant detects the existing lossy
conversion, marks duplication evidence incomplete and skips invalid paths.
Two invalid files and one valid replacement-character file pass the real CLI
probe; the six-call ordinary reuse identity probe still passes. Details and pinned
patch are in [OPT-REUSE.md](OPT-REUSE.md). The guarded variant is not the measured
combined binary. Source forwarding or explicit incomplete propagation is a
required integration condition, not an optional future optimization.

### O5 codec qualification and current decision

Five codec iterations over the exact frozen 1M evidence plus candidate32:

| Codec | Total bytes | Compression median | Decompression median |
|---|---:|---:|---:|
| zlib default | 5,117,407 | 399.68 ms | 17.82 ms |
| LZMA preset0 | 3,994,651 | 447.07 ms | 155.96 ms |
| LZMA preset6 | 3,472,699 | 3,319.77 ms | 141.33 ms |

Every decoded buffer equals its original. Normalization, canonical-text capture,
file preparation, I/O, indexed lookup and lineage are excluded. These are Python
stage measurements, not a measured Rust integration. Only preset6 fits size,
and its compression exceeds the entire ~1,701 ms cold allowance. Faster measured
codecs fail size. Process high-water includes interpreter/raw buffers and may
include macOS inherited launch high-water; it is not codec allocation accounting.
[Profile](results-optimizations/compression-blocks-profile-1m.json).

Decision: these measured synchronous evidence-cache formats do not qualify. Next
consider larger blocks/fast presets or an evidence lifecycle that avoids rebuilding
all verification bytes synchronously; measure lookup/update and collision costs
before adopting either. No custom codec is justified by this experiment.

The five optimization tracks now have correctness checks and concrete measured
outcomes, including generic interior matching and codec costs. **The combined
production design is still unqualified:** large-file Stop time fails, cold
headroom is not robust, retained reuse memory is excessive, lossless cache runtime
is unresolved, lineage is unimplemented and blind calibration selected no T.
Research can record this negative feasibility result; it cannot mark those product
requirements complete. Follow-up changes remain in isolated diagnostics.

To preserve evidence, run.sh/measure.sh now default to results-rerun and a README
rerun uses fresh WORK storage. RESULTS selects another destination; old archived
results are not rewritten by the documented command.
