# G4 v3 blind confirmation, 2026-10-06

The rule version and 200-pair sample were frozen before review. Two independent model reviews and a targeted adjudication are complete.

The population contains 3,085 introduced-region pairs from six repositories. Version 3 keeps 59 Rust pairs across four repositories and 130 TypeScript pairs across two repositories. The review packet is a stratified random sample with 50 v3-kept and 50 dropped pairs per language. Reviewers received only the files under `review/`; `rules.json`, `sampling.json`, and `population-mapping.json` contain the private selection data.

For TypeScript, calculate precision and recall with the weights in `sampling.json`, not raw sample fractions. The sample oversamples v3-kept pairs and weights each (language, repository, v3 status) stratum by its population/sample ratio. Only the `copy` label counts as a copy; `mixed` is non-copy, matching `holdout.py`.

Some family hashes overlap earlier labeled sets in the same repository. The candidate pair IDs use disjoint historical commits where G1 covered a repository; exact family-disjoint sampling would leave too few kept Rust pairs. This is recorded in `sampling.json`. The candidate scanner also cannot prove the counterpart existed in the parent commit; see `provenance.json`.

Reviewer A (`gpt-6.1-sol`) and reviewer B (`gpt-6-astra`) each returned all 200 labels. They agree on 91% of full labels and 91.5% of copy/non-copy decisions (κ = 0.83). Weighted estimates:

| Reviewer | Rust precision / recall | TypeScript precision / recall | TypeScript result |
|---|---:|---:|---|
| A | 93.9% / 35.8% | 77.9% / 64.9% | Fails precision |
| B | 91.9% / 35.3% | 88.5% / 58.7% | Passes |

G4 requires at least 80% TypeScript precision and recall above 30% for both reviews. Because reviewer A falls below the precision floor, G4 remains open. Reviewer C (`gpt-5.6-sol`) independently labeled the 12 TypeScript pairs where A and B differed on copy/non-copy status. Three-model majority resolves three as copies and nine as non-copies. Applied to these pairs while retaining A/B agreement on the rest, the secondary estimate is exactly 80.0% precision and 65.5% recall. This does not replace the two independent estimates. The labels, original scores, and adjudication record are in [`review/results/`](review/results/). The family-overlap and parent-provenance caveats above still apply.

To reproduce the score, run:

```sh
python3 docs/duplication-speed-2026-10-05/g4-fragments/blind-v3-2026-10-06/score_sample.py \
  docs/duplication-speed-2026-10-05/g4-fragments/blind-v3-2026-10-06/review/results/reviewer-a.json \
  docs/duplication-speed-2026-10-05/g4-fragments/blind-v3-2026-10-06/review/results/reviewer-b.json
```

The scorer checks exact ID coverage, reports each original reviewer's weighted estimates and pairwise agreement, and marks the TypeScript G4 criteria. `review/results/adjudication.json` records the targeted vote and secondary estimate.
