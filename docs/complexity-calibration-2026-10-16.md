# Complexity calibration for TypeScript apps, 2026-10-16

This note belongs to #585. #585 asks which of five changes to `complexity`
cut its noise on TypeScript apps without losing the findings a person wants.
It measures each change on the precision replay of #572.

It is research only. It changes no shipped behavior and no SPEC semantics.
The decisions at the end name the follow-up work. Each change of behavior is
a separate ticket.

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
  the same 134 findings, outcomes, ceilings and matches as the `main` binary.
- The cap also runs on the sample of #343
  (`benchmark/evidence/false-alarms-2026-09-29/selection.json`), because no
  derived `cc` ceiling of the Next.js sample is above the floor.

### Labels

- The owner set the label rule before any finding was labeled:
  `complexity-calibration-2026-10-16/label-rule.md`. A finding is
  `appropriate` when a careful reviewer of the change would ask to simplify
  the function, about what this change did to it. The rule names no measure.
- Agents drafted the labels in two passes. Each pass labeled one finding at a
  time, blind to which variants keep it.
- Rule 6 (an early-return guard or a try/catch that adds no nesting does not
  tangle a function) came after the first pass, because the labelers split on
  guard clauses. The owner chose it on 2026-10-16.
- An adversarial review of the first pass found that 12 packets showed an
  empty "before" for functions in renamed files, and that packets of moved
  functions did not show their source. The second pass relabeled all 136
  findings with corrected packets. They show the real "before" or the most
  similar function in the parent commit, and they do not show klin's numbers.
  The rule's history is at the end of `label-rule.md`.
- The second pass changed 4 of the 136 first-pass labels: F024, F079, F117
  and F127, each from `appropriate` to `not-appropriate`. On the 69 packets
  that did not change between the passes, the passes agree on 67 (kappa
  0.82).
- Every number below uses the second-pass labels. No person has spot-checked
  them yet, so every N below is provisional. ADR 0073 asks for a person's
  spot-check before a row may block, and `replay.py` checks 30.

### N

- N is the count of `not-appropriate` findings per 100 changes, the unit of
  ADR 0073. Each N comes with an exact Poisson 95% interval over the 102
  changes.
- Three commits hold 72 of the 134 findings: two squash-merged feature
  commits (12,194 and 13,155 lines added and removed) and one repository-wide
  formatting commit (4,177 lines). The column "N, small" leaves these three
  out and counts 99 changes. It is for information only.

## The variants

| Variant | Definition in the prototype |
|---|---|
| JSX threshold | A function that holds a JSX element gets a `lines` ceiling of at least 120 (CodeScene's React default). |
| JSX logic lines | A function that holds a JSX element counts only the lines that no JSX element covers, against the same derived `lines` ceiling. Not one of the five. It is a second form of the first. |
| Nesting depth | The deepest nesting of `if`, loops, `switch`, `catch` and `?:`, with `else if` at the depth of its `if`. Ceiling: p95 or 4, whichever is higher (CodeScene's Deep Nested Complexity is 4). Depth is ratcheted like `cc` and `lines`. |
| Bumpy Road | The count of runs of top-level statements that nest 2 or deeper. A function with 2 or more bumps fails (CodeScene's rule). |
| Cognitive complexity | Sonar's rules, with nested functions measured on their own as `cc` does. It replaces `cc`, with a ceiling of p95 or 15, whichever is higher (Sonar's default). |
| Cap | `min(p95, cap)` for a derived ceiling, never below the floor. |

Nesting and Bumpy Road also ran in a second form: a function over `lines`
fails only when it nests 3 deep, or only when it has a bump. In that form they
can remove findings, not only add them.

The prototype's nesting, Bumpy Road and cognitive measures use the node kinds
of the TypeScript grammar. Every finding of this sample is TypeScript.

## Result on the Next.js sample

Today `complexity` reports 134 findings in 27 of the 102 changes. 13 are
`appropriate`, in 11 distinct places in the code: F001 is inside F050, and
F130 is inside F066.

| Variant | Findings | `appropriate` | N | N, 95% | N, small | `appropriate` lost |
|---|---|---|---|---|---|---|
| Today | 134 | 13 | 118.6 | 98–142 | 59.6 | – |
| JSX threshold 120 | 76 | 11 | 63.7 | 49–81 | 31.3 | F001, F088 |
| JSX logic lines | 71 | 12 | 57.8 | 44–75 | 24.2 | F050 (F001, inside it, stays) |
| Nesting depth | 135 | 13 | 119.6 | 99–143 | 59.6 | none |
| Bumpy Road | 134 | 13 | 118.6 | 98–142 | 59.6 | none, no change |
| Cognitive in place of `cc` | 132 | 13 | 116.7 | 97–140 | 59.6 | none |
| Cognitive beside `cc` (both must fail) | 132 | 13 | 116.7 | 97–140 | 59.6 | none |
| Cap | 134 | 13 | 118.6 | 98–142 | 59.6 | none, deduced (see below) |
| `lines` fails only at depth 3 | 30 | 6 | 23.5 | 15–35 | 11.1 | 7 |
| `lines` fails only with a bump | 49 | 8 | 40.2 | 29–55 | 18.2 | 5 |

- **Cap.** Every derived `cc` ceiling of the sample is the floor of 10, and
  the derived `lines` ceilings run from 50 to 66. A cap at or above the floors
  cannot change a ceiling here, so the cap row is deduced, not run.
- **Nesting depth.** No function in a finding nests deeper than 3, so the
  depth ceiling of 4 adds no finding. The one added finding, F076, comes from
  ratcheting depth: its depth rose from 0 to 1 while its `lines` fell, so a
  held site became `worsened`.
- **Bumpy Road.** As an extra measure, it adds nothing. A function with 2
  bumps is already over `lines` in this sample.

## Result of the cap on the sample of #343

Of the ten repositories of #343, a derived `cc` ceiling is above 10 only in
`louis-e/arnis` (15), `firecrawl/pdf-inspector` (13) and `gfx-rs/wgpu` (15). A
derived `lines` ceiling is above 100 only in `apollographql/apollo-client`
(106 to 108) and `gfx-rs/wgpu` (107). The cap ran on the 40 changes of those
four repositories (`scripts/replay/samples/fa343cap.json`). The case that
#585 names, a base at `cc` 19, does not occur.

| Cap | Findings added | `appropriate` | `not-appropriate` |
|---|---|---|---|
| `cc` at 10 | 35 (34 in arnis) | 4 | 31 |
| `lines` at 80 | 1 | 1 | 0 |

The `cc` cap adds 31 to N on that sample of 100 changes, 30 of them in arnis.
The `lines` cap rests on one finding. These labels are first-pass labels,
`findings-cap-343.tsv`.

## Where the noise comes from

The second-pass reasons of the 121 `not-appropriate` findings fall into these
groups. The grouping is by keywords in the reasons, so each count is
approximate:

| What the change did to the function | Findings |
|---|---|
| Moved, renamed, extracted or regenerated it, or rewrote an earlier version | 51 |
| Formatting or lint autofix: wrapped lines, sorted keys, handlers moved into `useCallback` | 23 |
| Styling, text or markup | 15 |
| Added only a guard or a try/catch (rule 6) | 12 |
| Types or API renames | 2 |
| Other: a small addition the labeler judged readable | 18 |

The label asks what the change did to the function. A measure of the function
as it stands cannot see the change, so this result follows partly from the
rule. Two checks that do look at the change remove little:

- **A name-based key.** 32 of the 53 `new` findings had a function of the
  same name at the base: 20 in the same file and 12 in another file. klin
  calls them new because its key is the declaration line (ADR 0008). Matched
  by name, 5 `not-appropriate` findings and 1 `appropriate` finding (F017)
  become `held`, and the rest still fail. This agrees with ADR 0062 Q3.
- **A token filter.** A `worsened` finding whose tokens are the same before
  and after, in any order, clears 1 finding (F119).

A third measure looks at the change and does separate the labels: the growth
of `lines` since the matched base entry. All 55 `worsened` findings that grew
by fewer than 10 lines are `not-appropriate`. The 26 that grew by 10 or more
hold all 9 `appropriate` `worsened` findings and 17 `not-appropriate` ones.
This was found after the labels, so it is post hoc.

## Splitting the check into tiers

ADR 0073 lets a row with N of 3 or less block, a row with N above 3 and at
most 10 ride a spent block, and rejects a row above 10. The rules below the
first three rows came after the labels, so they are post hoc and in-sample.
"Rose" and "grew" compare with the matched base entry.

| Rule | Findings | `appropriate` | N | N, 95% | N, small |
|---|---|---|---|---|---|
| Over `cc` (blocks today) | 26 | 6 | 19.6 | 12–30 | 11.1 |
| Over `lines` only | 108 | 7 | 99.0 | 81–120 | 48.5 |
| Over `lines` only, JSX logic lines | 46 | 6 | 39.2 | 28–53 | 13.1 |
| `cc` over 10 and (new, or `cc` rose) | 13 | 4 | 8.8 | 4–17 | 4.0 |
| Cognitive over 15 and (new, or cognitive rose) | 6 | 2 | 3.9 | 1–10 | 3.0 |
| `cc` over 10 and (new, or grew by 10 or more lines) | 14 | 6 | 7.8 | 3–16 | 3.0 |
| New and `cc` over 10, or `worsened` and grew by 20 or more lines | 18 | 11 | 6.9 | 3–14 | 1.0 |

- No rule reaches the blocking bar of N 3 or less. Each interval also reaches
  above 10, so this sample cannot place any rule in a tier with confidence.
- Outside the three large commits, only 3 findings are `appropriate` (F043,
  F066, F130). The `cc`-rise and cognitive-rise rules catch none of them, and
  the two growth rules catch F066.
- The cognitive rule misses the planted case of the recall suite (next
  section), and its `not-appropriate` findings are the guard cases of rule 6.
- Of the 20 `not-appropriate` findings over `cc`, 10 kept their `cc`: only
  their `lines` rose, which ADR 0062 Q2 counts as worse. 3 are new, 1 fell,
  and in 6 `cc` rose.

## The recall suite

The planted long component of `tests/recall.rs` is 80 lines at `cc` 13,
cognitive complexity 11, and 25 lines that no JSX element covers. Its row,
"a long copied component", counts a catch on any `complexity` finding that
carries `lines`, and every production finding carries `lines`. The row is
caught today because the component is over `cc`, not because it is long.

- A lines-only tier that leaves the Stop keeps this catch, because the
  component is new and over `cc`.
- The cognitive rule loses it: 11 is under the ceiling of 15.
- The row does not test length. A planted component over `lines` and at or
  under `cc` 10 would test it.

## Decisions

The owner decided these on 2026-10-16, after the adversarial review:

1. **None of the five variants is adopted.**
   - The JSX threshold and JSX logic lines halve N, but each loses an
     `appropriate` finding, and the lines-only tier stays above 10 either
     way.
   - Nesting and Bumpy Road add nothing as measures, and lose 5 to 7
     `appropriate` findings as conditions on `lines`.
   - Cognitive complexity removes 2 findings over the whole check. As a rule
     over `cc` findings alone, it is at N 3.9 and misses the planted case.
   - The `cc` cap adds 31 `not-appropriate` findings per 100 changes on #343.
     The `lines` cap rests on one finding.
2. **A lines-only finding moves to a new report-only tier.** It never
   reaches the Stop, it is not a FAIL at `klin check` or in CI, and `klin
   report` and `klin status` show it as information. This amends ADR 0073
   with a fourth tier, for a row above N 10 whose data a person still wants
   to see. Proposed for that ticket and not yet decided: the tier applies to
   derived ceilings only, so a pinned `lines` or `test_lines`, which a person
   set, stays a FAIL.
3. **The `cc` tier keeps blocking until a validated rule replaces it.** At N
   19.6 it is in the rejected band of ADR 0073. The owner records this as an
   interim exception that ends when the validation ticket closes.
4. **A validation ticket tests two ride-along candidates.**
   - "New and `cc` over 10, or `worsened` and grew by 20 or more lines".
   - "`cc` over 10 and (new, or `cc` rose)".
   - It runs on a new frozen sample that holds agent turns, Rust and
     TypeScript, labeled under the same rule.
   - A rule with N of 10 or less there becomes the ride-along row of
     `complexity`. `complexity` blocks only if a rule reaches N of 3 or less
     there.

### Decisions that change an ADR or SPEC

| Record | What changes | Why |
|---|---|---|
| ADR 0073, tier table | A fourth tier: report-only, for a row above N 10 | Lines-only findings are at N 99, and a person still wants their data in the report |
| ADR 0073, admission bar | Interim exception: `cc` blocks at N 19.6 until the validation ticket closes | No validated replacement exists yet |
| ADR 0062 Q2 | A lines-only rise of a function over `cc` no longer fails at the Stop, if a candidate rule is adopted | 10 of the 20 `not-appropriate` findings over `cc` are this case |
| ADR 0062 Q2, "no +N tolerance" | A growth threshold of 20 lines, if that candidate is adopted | Growth separates the labels; fewer than 10 lines of growth was never `appropriate` |
| ADR 0062 Q4 | Unchanged: no JSX exemption | Neither JSX variant is adopted |
| SPEC B.7.1 and the check table | `complexity` placement and the lines-only tier | Decision 2 |
| `tests/recall.rs` | A row that tests length on its own | The current row is caught on `cc` |

### Follow-up tickets

- #696: the report-only tier for lines-only findings, with the ADR 0073
  amendment, the SPEC change and CLI tests.
- #697: the validation of the two ride-along candidates on a new frozen
  sample, which ends the interim exception.
- #698: a recall row that tests length on its own, with `recall()` counting
  a ride-along finding apart from a blocking one.

## Limits

- Agents drafted every label, and no person has spot-checked them yet.
- 13 `appropriate` findings in 11 places is a small base, and 9 of them come
  from one commit, f9652b45. One label moves N by 1.
- The sample holds human commits to three Next.js apps. It holds no agent
  turns. Most findings are in `vercel/ai-chatbot`, and 6 findings are in
  vendored `components/ai-elements` code.
- Each finding has one label per pass. The two passes are the only measure of
  agreement between labelers.
- The prototype's measures miscount some shapes:
  - Bumpy Road counts a comment between two deep blocks as a break between
    bumps.
  - Bumpy Road counts a body wrapped in `try` as one statement.
  - Cognitive complexity gives no increment for a labeled `break` or
    `continue`.
  - Cognitive complexity counts a parenthesized logical operator sequence
    once less than Sonar may.

  None of these changed a result in this sample.

## Reproduce

The replay script caches a run by the binary's hash, so the prototype run
needs `EXP585` in that cache key. With `replay.py` changed to add
`"-" + os.environ.get("EXP585", "base")` to `version`:

```sh
git apply docs/complexity-calibration-2026-10-16/prototype.patch
cargo build --release
for v in base jsx120 jsxlogic depth bumpy cog cogand lendepth lenbumpy; do
  EXP585=$v python3 scripts/replay/replay.py run nextjs complexity
done
EXP585=cap:10 python3 scripts/replay/replay.py run fa343cap complexity
EXP585=cap:999:80 python3 scripts/replay/replay.py run fa343cap complexity
```

- `findings-nextjs.tsv` holds every finding of every variant with its
  measures, the variants that keep it, the rename and similar function the
  packet showed, and both passes' labels.
- `scripts/replay/labels/complexity--nextjs.tsv` holds the second-pass labels
  of all 136 findings in the form `replay.py` reads. Its `reviewed` column is
  empty, for the owner's spot-check.
- `fa343cap` has no committed labels file. Its labels are in
  `findings-cap-343.tsv`.
