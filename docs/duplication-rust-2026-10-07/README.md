# Rust duplication policy for check REVIEW (#494)

**Result: Rust v5 fails. Rust stays out of the review signal.** Both
independent reviews estimate 30% precision and 50% recall on the fresh
sample. The criterion is at least 80% precision and at least 40% recall.

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
branch is new. `unitclass` gained the Rust node kinds that it needs. The
new kinds (`macro_invocation`, `let_declaration` and the Rust control-flow
expressions) do not occur in the TypeScript grammar. The #480 TypeScript
counts therefore do not change.

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
| starship | `d4d0459c5c24ba8f64663af8714f7858d7fb357b` | 1,000 newest non-merge commits |
| helix | `ba40e547426b0f9896c8bdc699a4ab11f2b37dbc` | 1,000 newest non-merge commits |
| nushell | `88bf737d8df2f3aab12cdf4c2b3c676dd7abf58f` | 300 newest non-merge commits |

The first plan used 200 commits each from starship and helix. That scan found
57 pairs in starship and 1 pair in helix, so one repository could not supply a
sample. The windows were widened and nushell was added before any label existed.
The rule did not change.

[`scan.py`](scan.py) finds regions that a commit introduces: at least half of
one span's lines are added by the commit. It keeps a pair only when the
counterpart's exact normalized tokens occur in the same path in the parent
commit. It counts the pairs that fail this check. The check finds the same
tokens anywhere in the parent version of that path, not at the span's own
position. A counterpart in a file that the commit renamed fails the check,
so it is excluded. The cause of the 277 nushell exclusions was not examined.

The sample takes every v5-kept pair, up to 60 per repository, and a seeded
random sample of the dropped pairs, stratified by repository, for a total of
at most 150 pairs. `sampling.json` records each stratum's population and
weight. Two model reviewers label the packet independently with the #480
review prompt. They see no rule, stratum, or token count.

## Population and sample

| Repository | Pairs | v5 kept | Counterparts not in parent (excluded) |
|---|---:|---:|---:|
| starship | 645 | 14 | 6 |
| helix | 22 | 4 | 29 |
| nushell | 41 | 2 | 277 |

starship supplies 91% of the population, so the recall estimate depends
mostly on starship. `sample.py` drew the packet with seed 20261007: all 20
kept pairs and 129 dropped pairs (starship 119 of 631, nushell 7 of 39,
helix 3 of 18). Its context lines come from the stored corpus with the
#480 window of three lines before and after each span.

## Result

Reviewer A (`gpt-6.1-sol`) and reviewer B (`gpt-6-astra`) each ran through
`codex exec` in a read-only sandbox. Each directory held only
`REVIEW-PROMPT.md` and `review-packet.json`. Both returned all 149 labels.
The log headers in `review/results/` record the model and the session id.

| Reviewer | Precision | Recall | Passes |
|---|---:|---:|---|
| A | 6 / 20 = 30.0% | 6.0 / 12.0 = 50.0% | no |
| B | 6 / 20 = 30.0% | 6.0 / 12.0 = 50.0% | no |

The reviewers agree on 98.7% of full labels and on every copy decision
(κ = 1.0). The labels are model judgments, not human labels.

Twelve of the 14 starship pairs that v5 keeps are module scaffolding: a
project-availability guard followed by formatter symbol and style wiring.
The token count is 96 in most of them, so they pass the 100-token branch
only after the trim. Both reviewers labeled them `boilerplate`.

As a secondary figure only, v2 keeps 3 pairs in this sample, all copies:
100% precision and 3 / 12.0 = 25% recall for each reviewer. v2 was not the
frozen candidate, and its recall is also below 40%.

The population depends mostly on one repository (starship, 91% of pairs),
and it has an estimated 12 copies. These estimates therefore have wide
uncertainty. The precision failure is not marginal: 14 of the 20 kept pairs
are non-copies for both reviewers.

To reproduce the scores:

```sh
python3 docs/duplication-rust-2026-10-07/score_sample.py \
  docs/duplication-rust-2026-10-07/review/results/reviewer-a.json \
  docs/duplication-rust-2026-10-07/review/results/reviewer-b.json
```

`sampling.json`, committed in d56e3182 before any label, records the v5
decision of each sampled pair through its stratum. `rules.json` holds the frozen v5 facts and decision for each scanned pair.
`population-mapping.json` holds every pair with its introducing commit.
The scan reproduces with `scan.py` at the pinned heads, then
`design.py rules` and `sample.py`.
