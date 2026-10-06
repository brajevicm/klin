# Rust duplication policy for check REVIEW (#494)

This research follows #480 under #478. It asks whether a Rust rule can
qualify for a future non-blocking `klin check` REVIEW signal. If it fails,
Rust stays out of the review signal.

## Criteria

All of these must hold on the fresh sample:

1. Weighted precision is at least 80% for each of two independent reviews.
2. Weighted recall is at least 40% for copies of 60 or more tokens, for each review.
3. The sample is new and comes from at least two repositories.
4. The rule is frozen in a commit before any new label exists.
5. Every pair has a counterpart that existed in the parent commit.

Only the `copy` label is a copy. `mixed` and every other label are non-copies,
as in `holdout.py`.

## Frozen rule: Rust v5

The rule is `v5` in [`design.py`](design.py). A pair is kept when:

- its two spans do not overlap in one file;
- its two spans are not both inside Rust trait impls;
- its token count after the import and attribute trim is at least 60; and
- the trimmed count is at least 100, or both spans hold at least two complete
  statements and at least two control-flow nodes (`if`, `match`, `for`,
  `while`, `loop`, `?` or `.await`).

The first three conditions and the 100-token branch are v2. The fragment
branch is new. `unitclass` gained the Rust node kinds that it needs.

## Design screen

`python3 design.py screen` scores the candidates on the #480 design sets.
The four fully labeled sets are scored directly. The v3 blind sample is scored
with its weights, once per reviewer.

| Candidate | calibration | holdout | holdout2 | introduced | pooled | v3 A | v3 B |
|---|---:|---:|---:|---:|---:|---:|---:|
| v2 | 84.6% / 22.0% | 89.8% / 22.7% | 100.0% / 24.8% | 67.5% / 11.9% | 84.8% / 19.2% | 93.9% / 35.8% | 91.9% / 35.3% |
| v5 | 77.8% / 28.0% | 93.2% / 35.2% | 100.0% / 44.8% | 68.3% / 18.9% | 85.5% / 30.1% | 50.8% / 35.8% | 49.8% / 35.3% |

Cells are precision / recall. The v5 fragment branch raises pooled recall
from 19.2% to 30.1% at the same pooled precision. In the v3 sample, v5 keeps
one dropped-stratum pair with a weight of 50. Both reviewers labeled it
`boilerplate`, so this one pair halves the v3 precision estimate. That
sample has no other change in decisions.

Two other designs were rejected on this data:

- A statement floor on the 100-token branch dropped real copies whose
  function body is one `match` tail expression with zero statements.
- A branch for trait-impl pairs lowered pooled precision below 85%.

No candidate reached 40% pooled recall at 85% pooled precision. The fresh
sample is therefore a real test, and it can fail.

## Fresh sample plan

The repositories are new to the #480 work:

| Repository | Pinned head | Window |
|---|---|---|
| starship | `d4d0459c5c24ba8f64663af8714f7858d7fb357b` | 200 newest non-merge commits |
| helix | `ba40e547426b0f9896c8bdc699a4ab11f2b37dbc` | 200 newest non-merge commits |

[`scan.py`](scan.py) finds regions that a commit introduces: at least half of
one span's lines are added by the commit. It keeps a pair only when the
counterpart's exact normalized tokens occur in the same path in the parent
commit. It counts the pairs that fail this check.

The sample takes every v5-kept pair, up to 60 per repository, and a seeded
random sample of the dropped pairs, stratified by repository, for a total of
at most 150 pairs. `sampling.json` records each stratum's population and
weight. Two model reviewers label the packet independently with the #480
review prompt. They see no rule, stratum, or token count.
