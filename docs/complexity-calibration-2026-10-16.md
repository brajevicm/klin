# Complexity calibration for TypeScript apps, 2026-10-16

This note belongs to #585. #585 asks which of five changes to `complexity`
cut its noise on TypeScript apps without losing the findings a person wants.
It measures each change on the precision replay of #572.

It is research only. It changes no shipped behavior and no SPEC semantics.
The follow-up tickets at the end carry the changes the owner decided.

## Protocol

### Sample and binary

- The sample is the frozen `scripts/replay/samples/nextjs.json` of #572: 102
  non-merge commits, 34 each from `vercel/ai-chatbot`, `vercel/commerce` and
  `shadcn-ui/taxonomy`. A change is one commit against its first parent, with
  `klin.json` set to `{}`.
- The binary is a release build of `main` at `09ea85a0`.
- The variants run on a throwaway prototype of that binary,
  `complexity-calibration-2026-10-16/prototype.patch`. The environment
  variable `EXP585` selects a variant. With `EXP585=base`, the prototype gives
  the same 134 findings as the `main` binary.
- The cap also runs on the sample of #343
  (`benchmark/evidence/false-alarms-2026-09-29/selection.json`), because no
  derived `cc` ceiling of the Next.js sample is above the floor.

### Labels

- The owner set the label rule before any finding was labeled:
  `complexity-calibration-2026-10-16/label-rule.md`. A finding is
  `appropriate` when a careful reviewer of the change would ask to simplify
  the function, about what this change did to it. The rule names no measure,
  so no variant wins by its definition.
- Four agents drafted the labels, one finding at a time. They saw the commit,
  the function and its diff. They did not see which variants keep a finding,
  or any measure other than `cc` and `lines`.
- Rule 6 (guard clauses) came after the first pass, because the labelers
  split on four findings of one shape. The owner chose it on 2026-10-16. It
  changed one label, F069.
- No person has spot-checked the labels yet. Every N below is provisional
  until the owner checks 30 of them, as ADR 0073 requires.

### N

N is the count of `not-appropriate` findings per 100 changes, the unit of ADR
0073. Three squash-merged commits of 2,000 to 13,000 changed lines hold 72 of
the 134 findings. The column "N, small" leaves them out (99 changes), for
information only. An agent turn is much smaller than those commits.

## The variants

| Variant | Definition in the prototype |
|---|---|
| JSX threshold | A function that holds a JSX element gets a `lines` ceiling of at least 120 (CodeScene's React default). |
| JSX logic lines | A function that holds a JSX element counts only the lines that no JSX element covers. Not one of the five. It is a second form of the first. |
| Nesting depth | The deepest nesting of `if`, loops, `switch`, `catch` and `?:`, with `else if` at the depth of its `if`. Ceiling: p95 or 4, whichever is higher (CodeScene's Deep Nested Complexity is 4). |
| Bumpy Road | The count of runs of top-level statements that nest 2 or deeper. A function with 2 or more bumps fails (CodeScene's rule). |
| Cognitive complexity | Sonar's rules, with nested functions measured on their own as `cc` does. It replaces `cc`, with a ceiling of p95 or 15, whichever is higher (Sonar's default). |
| Cap | `min(p95, cap)` for a derived ceiling, never below the floor. |

Nesting and Bumpy Road also ran in a second form: a function over `lines`
fails only when it nests 3 deep, or only when it has a bump. In that form they
can remove findings, not only add them.

The prototype implements nesting, Bumpy Road and cognitive complexity for the
TypeScript grammar only.

## Result on the Next.js sample

Today `complexity` reports 134 findings in 27 of the 102 changes. 17 are
`appropriate`, and N is 114.7.

| Variant | Findings | `appropriate` | N | N, small | `appropriate` lost |
|---|---|---|---|---|---|
| Today | 134 | 17 | 114.7 | 57.6 | – |
| JSX threshold 120 | 76 | 14 | 60.8 | 29.3 | F001, F024, F088 |
| JSX logic lines | 71 | 15 | 54.9 | 22.2 | F024, F050 |
| Nesting depth, extra measure | 135 | 17 | 115.7 | 57.6 | none, adds 1 `not-appropriate` |
| Bumpy Road, extra measure | 134 | 17 | 114.7 | 57.6 | none, no change |
| Cognitive in place of `cc` | 132 | 17 | 112.7 | 57.6 | none |
| Cognitive beside `cc` (both must fail) | 132 | 17 | 112.7 | 57.6 | none |
| Cap | 134 | 17 | 114.7 | 57.6 | none, no change |
| `lines` fails only at depth 3 | 30 | 9 | 20.6 | 9.1 | 8 |
| `lines` fails only with a bump | 49 | 11 | 37.3 | 16.2 | 6 |

Every derived `cc` ceiling of the sample is the floor of 10, and every derived
`lines` ceiling is between 52 and 66. That is why the cap changes nothing
here.

As extra measures, nesting and Bumpy Road add almost nothing. Almost every
function that nests deep is already over `lines`.

## Result of the cap on the sample of #343

Of the ten repositories of #343, a derived `cc` ceiling is above 10 only in
`louis-e/arnis` (15), `firecrawl/pdf-inspector` (13) and `gfx-rs/wgpu` (15). A
derived `lines` ceiling is above 100 only in `apollographql/apollo-client`
(106 to 108) and `gfx-rs/wgpu` (107). The cap ran on the 40 changes of those
four repositories. The case that #585 names, a base at `cc` 19, does not
occur.

| Cap | Findings added | `appropriate` | `not-appropriate` |
|---|---|---|---|
| `cc` at 10 | 35 (34 in arnis) | 4 | 31 |
| `lines` at 80 | 1 | 1 | 0 |

The `cc` cap adds about 31 to N on that sample. The `lines` cap rests on one
finding. The labels are in `findings-cap-343.tsv`.

## Where the noise comes from

The labelers' reasons for the 117 `not-appropriate` findings fall into these
groups. The grouping is by keywords in the reasons, so each count is
approximate:

| What the change did to the function | Findings |
|---|---|
| Formatting or lint autofix: wrapped lines, sorted keys, handlers moved into `useCallback` | 37 |
| Moved, renamed, copied or regenerated it | 36 |
| Styling, text or markup | 10 |
| Types or API renames | 3 |
| Other: a small addition, a guard, or code the labeler judged readable | 31 |

The cause is what the change did, not how klin measures the function. No
variant of #585 can tell a reformatted function from one that grew logic.
Two checks after the run confirm this:

- **A name-based key.** 32 of the 53 `new` findings had a function of the
  same name at the base: 20 in the same file and 12 in another file. klin
  calls them new because its key is the declaration line (ADR 0008). Matched
  by name, 5 `not-appropriate` findings become `held`, 1 `appropriate`
  finding becomes `held`, and the rest still fail. This agrees with ADR 0062
  Q3.
- **A token filter.** A `worsened` finding whose tokens are the same before
  and after, in any order, clears 1 finding.

## Splitting the check into tiers

ADR 0073 lets a row with N of 10 or less ride a spent block, and a row with N
of 3 or less block. The owner asked how `complexity` fares when it is split.
The last two rows of this table came after the labels, so they are post hoc
and in-sample:

| Split | Findings | `appropriate` | N | N, small |
|---|---|---|---|---|
| A function over `cc` | 26 | 9 | 16.7 | 9.1 |
| A function over `lines` only | 108 | 8 | 98.0 | 48.5 |
| A function over `lines` only, JSX logic lines | 46 | 6 | 39.2 | 13.1 |
| Post hoc: `cc` over 10 and new, or `cc` rose | 13 | 5 | 7.8 | 4.0 |
| Post hoc: cognitive over 15 and new, or cognitive rose | 6 | 3 | 2.9 | 3.0 |

A `lines`-only finding is above the ride-along bar in every form measured.
Most `not-appropriate` findings over `cc` are functions whose `cc` did not
move: only their `lines` rose, which ADR 0062 Q2 counts as worse.

## The recall suite

The planted long component of `tests/recall.rs` is 80 lines at `cc` 13. Both
JSX variants keep it, because its `cc` is over 10. Cognitive complexity with
JSX logic lines would miss it: its cognitive complexity is 11 and it has 25
logic lines, and both are at or under their ceilings. A rule that blocks only
on `cc` keeps the catch, because the planted file is new.

## Decisions

The owner decided these on 2026-10-16:

1. **None of the five variants is adopted.**
   - The JSX threshold and JSX logic lines cut the most noise, but each loses
     `appropriate` findings, and the `lines` tier stays above the ride-along
     bar either way.
   - Nesting and Bumpy Road add nothing as measures and lose up to 8
     `appropriate` findings as conditions on `lines`.
   - Cognitive complexity removes 2 findings. That is too little to replace
     `cc` and its eight grammar tables.
   - The `cc` cap adds 31 `not-appropriate` findings per 100 changes on #343.
   - The `lines` cap rests on one finding.
2. **A `lines`-only finding leaves the Stop.** `klin check` and `klin report`
   still show it, and the held-debt line of #584 names it when the turn
   edited that function. This applies the ride-along bar of ADR 0073 to
   `complexity`.
3. **A narrower blocking rule is validated before it ships.** "Blocks when
   `cc` is new over the ceiling or rose" and the same rule over cognitive
   complexity run on a new frozen sample, labeled under the same rule. A rule
   is adopted only if its N there is 10 or less.

## Limits

- An agent drafted every label, and no person has spot-checked them yet.
- 17 `appropriate` findings is a small base. One label moves N by 1.
- One repository, `vercel/ai-chatbot`, holds most findings, and three
  squash-merged commits hold 72 of them.
- The sample holds human commits to three Next.js apps. It holds no agent
  turns.
- Nesting, Bumpy Road and cognitive complexity exist only for TypeScript in
  the prototype.

## Reproduce

The replay script caches a run by the binary's hash, so the prototype run
needs `EXP585` in that cache key. With `replay.py` changed to add
`os.environ.get("EXP585", "base")` to `version`:

```sh
git apply docs/complexity-calibration-2026-10-16/prototype.patch
cargo build --release
for v in base jsx120 jsxlogic depth bumpy cog cogand lendepth lenbumpy; do
  EXP585=$v python3 scripts/replay/replay.py run nextjs complexity
done
EXP585=cap:10 python3 scripts/replay/replay.py run fa343cap complexity
EXP585=cap:999:80 python3 scripts/replay/replay.py run fa343cap complexity
```

`fa343cap` is the 40 changes of arnis, pdf-inspector, wgpu and apollo-client
from the #343 selection, each with its head commit. `findings-nextjs.tsv`
holds every finding of every variant, with its measures, the variants that
keep it, and its label. `scripts/replay/labels/complexity--nextjs.tsv` holds
the labels of today's findings in the form `replay.py` reads, with the
`reviewed` column empty for the owner's spot-check.
