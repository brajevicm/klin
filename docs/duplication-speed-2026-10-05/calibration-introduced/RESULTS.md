# G1: introduced-copy corpus results

Version 2 was applied unchanged to 809 unique introduced-region candidates
from 118 commits across klin, atuin, yazi, documenso and actual. At least half
of one matched span's lines were added by the introducing commit. The
counterpart was found in the post-commit tree; the scanner did not verify that
it existed in the parent commit. These candidates therefore measure matches
involving introduced lines, but do not establish parent-tree provenance. The
corpus contains 463 Rust and 346 TypeScript pairs.

The rules were frozen before labels were produced. Two blind model reviewers
labeled one shard each (409 and 410 unique pair IDs). Ten IDs occur in both
shards; the reviewers agree on all ten. `labels.json` retains each pair's label,
rationale, source references and reviewer shard. These are model judgments,
not human labels or evidence of author intent.

| Language | All pairs | Labeled copies | Version 2 kept | Copies kept | Precision | Recall |
|---|---:|---:|---:|---:|---:|---:|
| Rust | 463 | 227 | 40 | 27 | 27/40 = 67.5% | 27/227 = 11.9% |
| TypeScript | 346 | 155 | 17 | 15 | 15/17 = 88.2% | 15/155 = 9.7% |

Version 2 fails the 80% precision criterion in Rust and passes it in TypeScript.
Both recall rates are below the 40% goal. The unfiltered T baseline has
precision 47/93 = 50.5% in Rust and 155/346 = 44.8% in TypeScript.

Compared with the previous blind confirmation in
[`calibration-holdout2/RESULTS.md`](../calibration-holdout2/RESULTS.md),
candidate-corpus Rust precision is 32.5 points lower (67.5% versus 100%), and
recall is 12.9 points lower (11.9% versus 24.8%). TypeScript differs by 3.2
points in precision and 4.9 points in recall. The more-than-10-point Rust
difference meets G1's precommitted trigger to use this corpus for further rule
work, with earlier surveys retained as comparative evidence. Treat that choice
as provisional until parent-tree provenance is verified; the strict historical
corpus criterion remains open.

Reproduce scoring from the repository root with:

```sh
python3 docs/duplication-speed-2026-10-05/holdout.py measure docs/duplication-speed-2026-10-05/calibration-introduced
```
