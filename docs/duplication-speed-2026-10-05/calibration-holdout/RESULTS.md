# Held-out precision of the per-language rules (#480)

The owner set the acceptance limit at 80% precision. This note measures the
two frozen rules on new labels from four repositories that the rules never saw.

## Method

1. The owner chose gitui and yazi for Rust, and actual and linkwarden for
   TypeScript. `provenance.json` records the pinned commits.
2. `calibrate.py collect` surveyed the four repositories with the same
   eligibility, test exclusion, k = 41 and T >= 60 as `calibration/`. The
   survey found 1,914 matches, which are 1,793 pair ids. A pair id names two
   line spans, and 121 matches share their spans with another match.
3. `holdout.py rules` wrote `rules.json` before any label existed. Commit
   f0a7cd13 froze it. Commit 3159b8dc counted a repeated pair id once, which
   changed the kept state of one id. Nobody read a label before that commit.
4. Three blind model reviewers labeled the three shards (`blind-N.json.gz`).
   They saw no mapping, length, threshold or rule result. `review-N.md` holds
   their reports. No pair id received two different labels.
5. `holdout.py measure` joined the labels to `rules.json` and wrote
   `summary.json`.

The frozen rules:

- Rust: E2 (imports and attributes trimmed) and E3 (no pair with both spans in
  trait impls), T = 100.
- TypeScript: at least 50% of each span inside complete functions, T = 60.

## Result

| Language | Kept | Copy | Precision | Limit 80% | Copies kept / all copies |
|---|---:|---:|---:|---|---:|
| Rust | 65 | 53 | 81.5% | met | 53 / 233 |
| TypeScript | 126 | 89 | 70.6% | **missed** | 89 / 517 |

By repository:

| Repository | Kept | Copy | Precision |
|---|---:|---:|---:|
| gitui | 31 | 29 | 93.5% |
| yazi | 34 | 24 | 70.6% |
| actual | 109 | 72 | 66.1% |
| linkwarden | 17 | 17 | 100% |

Non-copies that the rules kept:

- Rust: 7 distinct, 4 required-shape, 1 boilerplate. Six of the 7 distinct
  pairs are spans that overlap inside yazi's `UTF8_CHAR_WIDTH` table.
- TypeScript: 27 generated, 10 boilerplate. All 27 generated pairs are svgr
  icon components in actual's `packages/component-library/src/icons/`. Each
  icon is one function that holds only JSX, so it passes the whole-unit rule.

## Post-hoc variants

These variants were chosen after the held-out labels were read. They are not
acceptance evidence. A third label set must confirm them.

| Variant | calibration/ | calibration-holdout/ |
|---|---:|---:|
| Rust frozen | 22/26 = 84.6% | 53/65 = 81.5% |
| Rust, no pair whose two spans overlap in one file | 22/26 = 84.6% | 53/59 = 89.8% |
| TypeScript frozen | 23/27 = 85.2% | 89/126 = 70.6% |
| TypeScript, plus >= 40 non-JSX tokens | 23/24 = 95.8% | 89/99 = 89.9% |
| TypeScript, plus >= 60 non-JSX tokens | 23/23 = 100% | 89/93 = 95.7% |

- An overlapping self-match does not show two copies. The matcher can reject
  it without labels, so this variant corrects the matcher. It does not tune
  the rule.
- The non-JSX minimum removes every icon pair and loses no copy in either set.
  It joins options 1 and 3 of the earlier TypeScript measurement.

## Limits of this evidence

- The labels are model judgments, not human labels.
- The share of copies differs between the label sets. At T >= 60, copies are
  233 of 751 Rust pairs here and 100 of 1,777 in `calibration/`. The new
  repositories may hold more real copies, or the reviewers may judge
  differently. This note does not separate the two causes.
- The reviewer for shard 0 first read a scratch file that another reviewer
  overwrote with a different shard. It discarded those labels and labeled
  shard 0 again from its own copy.
- One Playwright page model in actual's `e2e/page-models` passed the test
  exclusion. A reviewer labeled it boilerplate. The rules kept it.
- Recall is low: the rules keep 23% of the Rust copies and 17% of the
  TypeScript copies.
