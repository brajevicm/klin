# Corrected exact-region research verdict

**Final #480 recommendation: reject this blocking Stop design and advance
TypeScript v3 only as the leading candidate for a future check-only REVIEW
detector.** Rust remains unqualified because its blind recall is below the
owner's 40% target. The current Stop architecture remains unqualified on cache
size, integrated performance and generic introduced-only lineage. This is a
negative Stop-feasibility result, not evidence that klin should abandon its
already decided goal of detecting duplication.

The [checklist](CHECKLIST.md) records the work. Historical results in README
remain historical. Current evidence is in `results-regions/`, `verification.json`,
`calibration/`, `appeasement.json`, and `lineage-cases.json`.

This report freezes the pre-optimization candidate. Subsequent implementation
and measurements are tracked separately in [OPTIMIZATIONS.md](OPTIMIZATIONS.md);
they do not replace these archived measurements or the calibration verdict.

## Final policy decision after the fresh-family review

The later TypeScript v4 review supplies a fresh-family population of 139 pairs
with no pair/family overlap with earlier labels. V4 itself scores 96.1%
precision with 49.1% / 56.2% recall under the two model reviewers.

The already-frozen v3 rule can also be evaluated on those committed labels.
That secondary cross-score is reproducible from the v4 `sampling.json`,
`rules.json` and raw reviewer labels: reviewer A gives 97.7% precision /
64.7% recall and reviewer B gives 90.8% / 68.7%. It was not the preregistered
candidate for that packet, so it does not rewrite v3's earlier failed blind
confirmation. However, 61 of the 66 v3-kept pairs in the full fresh-family
population were directly labeled. Even if all five unlabeled v3 keeps are
non-copies, precision remains at least 87.9% for A and 84.8% for B. Even if
every unlabeled v3 drop is a copy, recall remains at least 53.2% / 54.4%.

For a non-blocking REVIEW signal, that materially higher recall at still-strong
precision makes v3 the policy to advance. V4 remains the more conservative
TypeScript candidate if a future blocking design needs extra precision margin.
Neither policy is admitted as a Stop blocker by this research.

Rust is unchanged by v3/v4. Its blind result is 93.9% / 35.8% for reviewer A
and 91.9% / 35.3% for reviewer B (precision / recall), so it does not meet the
owner's >=40% per-language recall gate. The next policy experiment should
therefore be Rust-only, fresh-family and parent-provenance-verified.

## Correctness and safety

The short-function bypass is gone. Names and T apply to every region. Production
mod-minimizer positions are sorted and unique. Stop enumerates bounded posting
prefixes, bounds hit/bridge work, and retains INCOMPLETE for capped postings,
local truncation, work truncation, unresolved exact edges, or T below the
fingerprint guarantee. An anchor cannot certify an omitted partner absent.
The decisive 64-decoy fixture has no unanchored key, yet must remain INCOMPLETE;
full check recovers its omitted true partner. Deleting `chain.bin` does not
prevent a Stop-only query.

Normalization now uses the pinned member/statement terminator rules, Rust test
ranges, ERROR/missing exclusions and the known risky multiline-comment class.
LF, CR and Unicode line breaks are covered. Unsafe top-level units and exclusion
boundaries cannot contribute equality evidence. This conservative unit policy
can also exclude valid neighboring code; it is not a general proof that the
parser has no undiscovered silent misparse. Per-language unsafe counts accompany
builds/queries. The parser probes and frozen rules are recorded in
[the normalization note](../duplication-normalization-2026-10-05.md).

`verify.py` compares small fixtures to an exhaustive diagonal scan over actual
normalized token text, independent of hashes, fingerprints, candidate selection
and detector truth counters. It also exercises threshold edges, edits, names,
TS/TSX, cross-language separation, repetitive streams and parser risks. The
standalone crate runs this oracle through its real CLI integration suite.
The full check confirms normalized token text for every candidate, including
Stop's hash-proven candidates. Hash equality is not the check's equality contract.

## Five-run standalone measurements

These are Stop candidate-index timings at k=41, w=20, minimizer t=5, cap=64,
64-bit keys. They include index load, normalization/fingerprinting and matching;
changed-file reading/parsing is reported separately as shared work. They exclude
full check, validation and lineage. They do not represent full hook overhead.
Each row has five runs; actual changed counts are retained when fewer files exist.

| Workload | Actual changed | Changed tokens | Median ms | Max ms | Result |
|---|---:|---:|---:|---:|---|
| 1M fixture, spread | 20 | 11,766 | 5.96 | 6.00 | Sample meets 15/25 ms |
| 1M fixture, spread | 100 | 59,223 | 26.35 | 26.56 | Sample meets 50 ms median |
| 1M fixture, largest | 20 | 16,129 | 8.27 | 8.58 | Sample meets 15/25 ms |
| 100k changed-token stress | 20 | 100,240 | 35.56 | 35.92 | Misses 15/25 ms |
| klin largest | 20 | 130,184 | 58.43 | 58.87 | Misses 15/25 ms; INCOMPLETE |
| GlareDB spread | 100 | 92,430 | 53.39 | 53.61 | Misses 50 ms; INCOMPLETE |
| Multiplicity 1,000 | 20 | 3,440 | 1.88 | 1.90 | Bounded work; INCOMPLETE |
| Multiplicity 1,000 | 100 | 17,200 | 13.71 | 13.96 | Bounded work; INCOMPLETE |

The full matrix includes 10k, 300k, all three pinned real trees, spread/largest
selection, multiplicity 2/10/40/63/64/65/100/1000 and separate 32/48/64-bit key
runs. `summary.txt` gives cold build cost, cache bytes, longest postings, memory
and all query rows. At multiplicity 64 the posting fits; at 65 INCOMPLETE is
retained. A fast incomplete run cannot establish complete recall or Stop fitness.
The 1M fixture has no true >=60-token region at this basis; it measures cost.
The separate long-region multiplicity and decoy fixtures measure common copies.

## Equality, cache and actual shared-tree cost

At 1M the candidate index occupies 2,184,542 / 2,965,110 / 3,745,678 bytes for
32/48/64-bit keys. The 64-bit region-only subtotal is 3,439,975 bytes. Including
the path table gives 10.04% of the 37,301,283-byte structural cache, already
slightly above 10%; omitting reusable paths gives 9.22%. Neither includes lineage.
The validation chain is another 47,440,064 bytes and is not a Stop index.

The chosen prototype contract is **64-bit candidate keys, exact normalized text
confirmation at full check, no hash-only production BLOCK**. Both token FNV and
rolling k-gram hashes are noncryptographic. Widening the key reduces accidental
collisions; it cannot prove equality or resist adversarial inputs. 32/48-bit rows
are space/cost alternatives for candidate selection, not permission to block on
those hashes. A fixed 32-bit collision fixture (`collision_47066` / `collision_48240`,
k=w=T=1) produces a false hash-proven candidate; full check rejects it and
retains only the truly equal semicolon. This is a diagnostic stress setting,
not the proposed T. Calibration also confirms actual token text before recording
a match.

`integrate.py` puts this normalizer/fingerprinter in an isolated archive of klin
at c1805539, inside the existing structural measurement's shared parsed-tree
callback. The production repository's `src/` is untouched. The existing
`tests/performance.rs` harness runs five alternating A/B pairs for warm20,
warm100 and cold, one binary with the callback disabled/enabled. It charges
normalization/fingerprinting in every invocation of that shared callback.

Final A/B statistics and the exact source snapshot are in
[`integrated.json`](results-regions/integrated.json); raw pairs are in
`integrated.log`. **This comparison is a lower bound: it excludes index build,
index query and lineage.** Counters account for the instrumented path (no extra
parse, unchanged-source read, whole-tree walk or external process introduced by
the callback); they are not an independent syscall audit. The final five-pair medians are 32,258.59 ms baseline and 36,821.81 ms
enabled for cold; the median paired delta is 4,437.49 ms, or 13.77%, against
the 5% limit. Warm20/warm100 paired deltas are 9.85/49.30 ms. Normalization
alone consumes 4,361.56 ms at cold. These numbers are observed lower-bound
results, not a full-index/ratchet overhead estimate. No summed standalone
normalization time is represented as an end-to-end cold measurement.

## Blind check-only threshold calibration

The rule was committed in `13655f91` before labels. Stop failed step 1; the owner
then explicitly authorized continuing calibration for a check-only detector.
This exception does not satisfy #480's original Stop-first acceptance criterion.

All eligible files in klin `src/` at c1805539, GlareDB at 8001afa4 and karakeep at
f8ae9866 were surveyed: 80/618/493 files, 233,276/573,537/263,888 tokens and
0/1/2 unsafe top-level units respectively. Test exclusions and the normalization
basis match the prototype. Generated production files and documentation build
inputs have no specified exclusion, so they remain eligible. Even disregarding
generated labels leaves non-copy matches at every candidate threshold.

542 exact-content families produced 1,923 maximal match pairs. Blind packets hide
T and token length. Three source reviewers inspected their assigned unique spans
and every pair location, using pinned source context and citations; only then
were labels joined to lengths. Labels are **model judgments**, not human labels,
proof of author intent, or independent statistical trials. Overlapping regions
and high-multiplicity families produce correlated pair counts.

| Language | T | Non-copy / all pairs | Non-copy / all families | Repositories |
|---|---:|---:|---:|---:|
| Rust | 60 | 1,677 / 1,777 | 350 / 437 | 2 |
| Rust | 80 | 891 / 948 | 171 / 220 | 2 |
| Rust | 100 | 378 / 417 | 86 / 122 | 2 |
| Rust | 150 | 28 / 42 | 15 / 29 | 2 |
| TypeScript | 60 | 88 / 146 | 68 / 105 | 1 |
| TypeScript | 80 | 45 / 79 | 28 / 47 | 1 |
| TypeScript | 100 | 25 / 51 | 16 / 28 | 1 |
| TypeScript | 150 | 12 / 13 | 7 / 8 | 1 |

**No T is selected for either language.** No row establishes 0/n. T +/-10/20
around a selected threshold is therefore inapplicable, rather than fabricated.
The label audit includes 158 copy, 713 boilerplate, 677 required-shape, 353
generated and 22 distinct pair judgments. See `calibration/summary.json`, blind
packets, labels and the three source-review notes. `mixed` follows #480's exact
family definition, not an uncertainty bucket.

The survey verifies text after minimizer selection; its scalable enumeration
still uses the frozen selector's >=60 guarantee. It does not replace the independent
small exhaustive oracle. Label disagreements could change individual counts;
the presence of long import, trait/protocol, generated and intentional-duplicate
counterexamples independently prevents accepting an all-copy threshold here.

## Lineage, repair pressure and disposition

[LINEAGE.md](LINEAGE.md) answers all seven #489 questions, with explicit slot
states, extension/split/merge/scope-exit examples, ambiguity and metadata needs.
Nine supplied-certificate accounting cases exercise conservation lower bounds.
They do not infer certificates from offsets or implement arbitrary-region ancestry.
The current postings index has no persisted family/role/exit certificates.
The measured full chain cannot fit the budget; the illustrative per-anchor
metadata cost is not a measured compact lineage design. **#489 remains unresolved
for a generic blocking ratchet.** Implementing such a ratchet after this candidate
fails both feasibility and threshold selection would invent an unadmitted contract.

`appeasement.py` executes 18 cheap-edit fixtures and five intentional-duplicate
fixtures. Comments/layout survive, but local renames, neutral expression edits,
insertion, statement reordering and splitting can evade exact-region detection.
Those are limitations of exact copying, not a renamed/near-miss implementation.
Intentional protocol, adapter, migration, independent-evolution and generated
fixtures are detected; automatic extraction can create harmful coupling.
These are constructed scenarios, not measured agent repair outcomes or prevalence.
Feedback must offer contextual review, with no extraction demand based only on
text equality. Test-scope exits still need lineage accounting before any ratchet.

The full research check enumerates pair candidates without a work/memory budget;
high multiplicity can grow quadratically. Its oracle/full-check path is appropriate
for bounded research fixtures, not shipped use. A future check needs cohort/span
reporting and explicit cancellation/work budgets. No quadratic full check has been
smuggled into the Stop timings.

The completed evidence rejects this candidate's production promotion. A future
check REVIEW proposal needs an explicit precision/scope policy, bounded reporting,
shared-tree full-check integration and, if ratcheted, admitted region lineage.
No production code, klin.json, accepted debt or hooks changed; no issue/PR update
was posted. `TRACKER-DRAFT.md` provides the concrete proposed tracker update.
