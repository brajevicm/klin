# klin blocks new real defects

> Amends spec 2.2, spec 18.2, spec B.8.1 rules 2 and 3, and ADR 0014 for
> rows whose remedy deletes code. Recorded for #571, 2026-10-11, from the
> owner's stance of 2026-10-10.

The research after 2026-10-02 judged each candidate row on a strict bar: N of
1 or less for a block, no cheap rewording, and "not a linter's job". No
candidate passed it. The owner's count in #571 is 0 valid regressions under
`{}` on the 30 real agent pull requests of the pilot
(`docs/phenotype-pilot-2026-10-02.md`).

That bar was never applied to the gates klin ships. By the owner's count in
#571, on the replay of #343,
`complexity` measured about 25 not-appropriate findings per 100 changes, and
`escapes` about 9. The old bar would have rejected every shipped check.

## The decision

**klin may be nosy about real issues, and it improves the code a little over
time.** These rules carry this:

1. A check may duplicate a linter rule when that rule is off by default in
   its ecosystem. Under `{}` no linter runs, so today nothing catches a
   secret or a swallowed error. This reverses SPEC B.8.1 rule 3 for that
   case.
2. A row that catches the reflexive form of a shortcut may block, even when
   a cheap rewording passes it. ADR 0031 already accepts this for a deleted
   test. This replaces the "no cheap rewording" bar of #362, #353 and #355,
   and the "resistance to cheap appeasement" bar of SPEC 18.2.
3. klin may tell about held debt in code the turn touched. It still never
   blocks on held debt alone. This narrows the non-goal of SPEC 2.2 from
   forced paydown to blocking for paydown.
4. A row in a tier that allows it may have a remedy that deletes code or
   changes behavior, such as deleting an orphan file. This reverses SPEC
   B.8.1 rule 2 ("additive remedy"), the "behavior-preserving repair path"
   bar of SPEC 18.2, and the matching part of ADR 0014, for those rows only.
5. Every row passes the admission bar below. Its N bar replaces the "low
   rate of harmful intervention" of SPEC 18.2.

## The admission bar

N is the count of not-appropriate findings per 100 changes on the replay of
#572.

| Tier | Bar | Outcome |
|---|---|---|
| Blocks, with a safe remedy (a type, a comment, a marker) | N of 3 or less | Blocks the Stop |
| Blocks, with a remedy that deletes code or changes behavior (orphan files, error handlers) | N of 1 or less | Blocks the Stop |
| Rides a spent block, with a safe remedy | N above 3 and at most 10 | Appears only in a Stop that already blocks for another finding, through the mechanism of #584 |
| Reports only | N above 10, and a person still wants the data | A note at `klin check`. The Stop never measures it |
| Rejected | N above 10 and no person wants the data, or above 1 for a remedy that deletes code or changes behavior | No row |

A row whose remedy deletes code or changes behavior never rides a spent
block. The stricter bar exists because a wrong deletion can remove work the
person wanted, and a ride-along finding still asks the agent to delete. The
agent is already stopped, so it is likely to act on it. Decided for #580,
2026-10-13.

Two more conditions hold for every row:

- The row catches its planted case in the recall suite of #572.
- Before a row may block, a person spot-checks its replay labels. Agents
  drafted the earlier labels.

#591 builds the ride-along tier and states what it means at `klin check`.

## The report-only tier

Decided for #696, 2026-10-19. #585 measured the lines-only findings of
`complexity` at N 99 per 100 changes
(`docs/complexity-calibration-2026-10-16.md`). That is far into the rejected
band, but the owner still wants the data, so a fourth tier keeps it.

- A report-only row runs at `klin check` only (SPEC 6.2, 9.7). The Stop never
  measures it, so it costs the Stop nothing and never reaches the agent, the
  journal, `klin report` or `klin status`.
- At `klin check`, each of its findings is a note. It changes no judgement and
  no exit code, and CI annotates none of them.
- The tier holds a ceiling klin derives. A ceiling in `klin.json` is policy and
  blocks, whoever wrote it: a person, `klin setup --pin` or a suggestion of
  `klin policy --suggest` the person picked. The bar judges klin's defaults,
  and a pin is a person's choice to enforce one.
- The derived `lines` ceiling of `complexity` is the first row in this tier.
  `test_lines` is never derived, so it always blocks.

## `complexity` has no blocking row

Decided for #707, 2026-10-20, from #697. #697 measured the derived `cc` row
at N 44.5 per 100 agent pull requests, and the best ride-along rule at N
18.2 (`docs/complexity-ride-along-2026-10-17.md`). No candidate reached the
ride-along bar of 10.

- `complexity` has no blocking row and no ride-along row. The derived `cc`
  ceiling joins the derived `lines` ceiling in the report-only tier.
- The interim exception that let the derived `cc` block at N 19.6
  (`docs/complexity-calibration-2026-10-16.md`) ended.
- A pinned `cc` still blocks, as every pinned ceiling does. Under `{}` no
  complexity ceiling blocks, so the Stop does no complexity work.

## What stays with each check

A reversal that belongs to one check lives in that check's ticket:

- ADR 0061 in #580.
- ADR 0050 in #574.
- SPEC 16.3 in #582.
- ADR 0020 in #576.

## What this gives up

A blocking row may now fire on code that a person would accept. Up to 3 of
100 changes, or 1 of 100 for a row whose remedy deletes code. The spot-check
of labels is the only guard against labels that understate N.

A remedy that deletes code can remove work the person wanted. The stricter
bar for that tier is the whole defense.
