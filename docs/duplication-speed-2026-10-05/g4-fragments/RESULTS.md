# G4: TypeScript fragment design screen

Version 3 preserves version 2's whole-unit TypeScript rule and adds a fragment
branch. A fragment must have at least 60 non-JSX tokens in both spans, at least
one call and either control flow or `await` in both spans, and at least two
complete executable statements in both spans. `holdout.py` records v2 and v3
decisions separately; v3 leaves Rust unchanged.

| Design set | V2 copy / kept | V2 precision / recall | V3 copy / kept | V3 precision / recall |
|---|---:|---:|---:|---:|
| `calibration/` | 23 / 23 | 100.0% / 39.7% | 41 / 49 | 83.7% / 70.7% |
| `calibration-holdout/` | 89 / 93 | 95.7% / 17.2% | 360 / 371 | 97.0% / 69.6% |
| `calibration-holdout2/` | 128 / 140 | 91.4% / 14.5% | 537 / 557 | 96.4% / 61.0% |
| `calibration-introduced/` | 15 / 17 | 88.2% / 9.7% | 86 / 96 | 89.6% / 55.5% |
| Combined | 255 / 273 | 93.4% / 15.8% | 1,024 / 1,073 | 95.4% / 63.6% |

All four design sets clear the 80% TypeScript precision floor. V3 increases
recall on each set and on the pooled design data (1,024 of 1,610 labeled
copies, up from 255). The introduced-region set is still provisional: its
scanner did not verify that the counterpart existed in the introducing
commit's parent. These scores are design evidence, not blind confirmation.

The selected fragment branch requires both a behavior signal and two complete
statements. The behavior-only branch was 79.2% precise on `calibration/`, below
the design floor; the two-statement condition raises it to 83.7% while retaining
70.7% recall there.

### Blind confirmation (2026-10-06; two-model review)

The frozen population has 3,085 pairs from six repositories. Version 3 keeps
59 Rust pairs across four repositories and 130 TypeScript pairs across two.
The blind review sample has 200 pairs: 50 kept and 50 dropped per language,
stratified by repository and rule result. Its private sampling metadata records
the weights needed to estimate TypeScript precision and recall.

The review packet was separate from the selection rules and sample strata; it
contained only pair IDs, source locations, code context and pinned links.
Reviewers A (`gpt-6.1-sol`) and B (`gpt-6-astra`) each saw only the review prompt
and packet, with no prior conversation context. Both returned all 200 labels.
Their full labels agree on 91% of pairs; copy/non-copy agreement is 91.5%
(κ = 0.83).

| Reviewer | Rust precision / recall | TypeScript precision / recall | TypeScript result |
|---|---:|---:|---|
| A | 93.9% / 35.8% | 77.9% / 64.9% | Fails precision |
| B | 91.9% / 35.3% | 88.5% / 58.7% | Passes |

Reviewer A is below the 80% TypeScript precision floor, so the v3 candidate
failed the two-review criterion. The reviewers disagreed on the
TypeScript copy/non-copy decision for 12 pairs (nine kept and three dropped).
Adjudicator C
(`gpt-5.6-sol`) saw only the prompt and packet, then labeled those 12 pairs
without seeing A or B's labels. Three-model majority resolves three as copies
and nine as non-copies. Applying those decisions and retaining the A/B agreed
copy status on the other TypeScript pairs gives 80.0% precision and 65.5%
recall. This secondary consensus estimate sits exactly at the precision floor;
it does not replace either independent estimate, so the v3 candidate remained
open under the two-review criterion. At that point, a revised candidate and a
fresh blind sample were still needed; the v4 follow-up is below. Of the 200
sampled pairs, 93 share a
content-family hash with an earlier labeled set in the same repository.
Commit windows are disjoint from G1 where it covered a repository; exact
family-disjoint selection would leave only 28 kept Rust families, below the
breadth target. The introduced-region method still cannot prove that the
counterpart existed in the parent commit. Raw labels, adjudication, and scores
are in [review results](blind-v3-2026-10-06/review/results/); the
[scorer](blind-v3-2026-10-06/score_sample.py) checks coverage and computes
weighted estimates and reviewer agreement.

To reproduce the design-set facts, then score both versions:

```sh
python3 docs/duplication-speed-2026-10-05/holdout.py rules CORPUS OUT
python3 docs/duplication-speed-2026-10-05/holdout.py measure OUT v2
python3 docs/duplication-speed-2026-10-05/holdout.py measure OUT v3
```

The blind labels and scores are preserved in `blind-v3-2026-10-06/review/results/`.
That v3 candidate failed the two-review criterion because reviewer A fell
below the precision floor. Its labels and adjudication remain final and are not
revised by the separate v4 test.

### V4 candidate screen and fresh-family confirmation

V4 preserves every v2 whole-unit TypeScript keep and retains additional v3
fragment candidates only when both spans have at least 70 non-JSX tokens. The
70-token cutoff was screened on existing design data before the fresh scan; it
did not improve pooled precision over 60 and reduced pooled recall. It was
tested as a stricter candidate, not selected on the blind labels.

| Design set | 60-token precision / recall | 70-token precision / recall |
|---|---:|---:|
| `calibration/` | 83.7% / 70.7% | 90.7% / 67.2% |
| `calibration-holdout/` | 97.0% / 69.6% | 96.5% / 47.8% |
| `calibration-holdout2/` | 96.4% / 61.0% | 96.4% / 49.2% |
| `calibration-introduced/` | 89.6% / 55.5% | 87.8% / 41.9% |
| Combined | 95.4% / 63.6% | 95.4% / 48.7% |

The fresh scan found 276 pairs in 200-commit windows from `actual` and
`documenso`. Removing families present in any previously labeled set leaves a
fresh-family population of 139 pairs: 51 candidate-kept and 88 dropped. The
100-pair sample censuses all 51 kept pairs and all 19 `actual` dropped pairs,
then samples 30 of 69 `documenso` dropped pairs. The registered weights recover
estimates for that population; the sample has no prior pair-ID or family
overlap.

| Reviewer | TypeScript precision / recall | Result |
|---|---:|---|
| A (`gpt-6.1-sol`) | 96.1% / 49.1% | Passes |
| B (`gpt-6-astra`) | 96.1% / 56.2% | Passes |

The reviewers agree on 92% of full labels and copy/non-copy decisions
(κ = 0.80). Both independent estimates clear the 80% precision and above-30%
recall criteria, so the v4 candidate passes G4 on the fresh-family TypeScript
test. The parent-provenance limitation remains open; see the
[v4 packet and results](blind-v4-2026-10-06/README.md).
