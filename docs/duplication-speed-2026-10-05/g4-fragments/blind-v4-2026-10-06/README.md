# G4 v4 fresh-family TypeScript review, 2026-10-06

This is a new blind test of the 70-token fragment candidate. The completed v3 evidence remains final as recorded in the v3 results: its two independent TypeScript estimates disagree across the precision floor, so it does not pass G4. This packet evaluates a separate candidate and does not revise those labels.

The candidate preserves every v2 whole-unit keep. It also keeps v3 fragment matches only when their minimum non-JSX count across both spans is at least 70. The threshold was screened on existing design sets before this fresh scan; the screen did not demonstrate a pooled precision gain over 60 and reduced pooled recall. This review checks whether the candidate generalizes on new historical windows.

The introduced-region scan found 276 TypeScript pairs: 77 in actual and 199 in documenso. After applying the candidate and excluding content families already present in earlier labeled sets, the fresh-family population has 51 kept and 88 dropped pairs. The 100-pair sample censuses all 51 kept pairs and all 19 actual dropped pairs, then samples 30 of 69 documenso dropped pairs. Weights in sampling.json recover estimates for this fresh-family population. Reviewers saw only the files under review/.

Family exclusions use canonical repository plus family hash and cover calibration, calibration-holdout, calibration-holdout2, calibration-introduced, and the reviewed v3 sample. The selected packet has no pair-id or family overlap with those labeled sets. The target population is therefore the remaining fresh-family portion of these windows, not all 276 raw pairs.

The scanner marks a region when at least half of one span's lines were added by the commit and finds a counterpart in the post-commit tree. It does not prove that the counterpart existed in the parent commit. See provenance.json.

Reviewers A (`gpt-6.1-sol`) and B (`gpt-6-astra`) each received only the prompt and packet, with no prior conversation context. Both returned all 100 labels. They agree on 92% of full labels and copy/non-copy decisions (κ = 0.80).

| Reviewer | TypeScript precision / recall | Result |
|---|---:|---|
| A | 96.1% / 49.1% | Passes |
| B | 96.1% / 56.2% | Passes |

Both independent estimates clear G4's 80% precision and above-30% recall criteria on this fresh-family population. The earlier v3 labels remain final and unchanged. The introduced-region scanner still cannot prove that a counterpart existed in the parent commit.

To reproduce the weighted estimates and agreement:

~~~sh
python3 docs/duplication-speed-2026-10-05/g4-fragments/blind-v4-2026-10-06/score_sample.py \\
  docs/duplication-speed-2026-10-05/g4-fragments/blind-v4-2026-10-06/review/results/reviewer-a.json \\
  docs/duplication-speed-2026-10-05/g4-fragments/blind-v4-2026-10-06/review/results/reviewer-b.json
~~~
