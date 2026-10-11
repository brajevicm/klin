# klin blocks new real defects

> Amends SPEC 2.2, SPEC 18.2, SPEC B.8.1 rules 2 and 3, and ADR 0014 for
> rows whose remedy deletes code. Recorded for #571, 2026-10-11, from the
> owner's stance of 2026-10-10.

The research after 2026-10-02 judged each candidate row on a strict bar: N of
1 or less for a block, no cheap rewording, and "not a linter's job". No
candidate passed it. Under `{}`, klin found 0 valid regressions on 30 real
agent pull requests (`docs/phenotype-pilot-2026-10-02.md`).

That bar was never applied to the gates klin ships. On the replay of #343,
`complexity` measured about 25 not-appropriate findings per 100 changes, and
`escapes` about 9. The old bar would have rejected every shipped check.

## The decision

**klin may be nosy about real issues, and it improves the code a little over
time.** Five rules carry this:

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
5. Every row passes the admission bar below.

## The admission bar

N is the count of not-appropriate findings per 100 changes on the replay of
#572.

| Tier | Bar | Outcome |
|---|---|---|
| Blocks, with a safe remedy (a type, a comment, a marker) | N of 3 or less | Blocks the Stop |
| Blocks, with a remedy that deletes code or changes behavior (orphan files, error handlers) | N of 1 or less | Blocks the Stop |
| Rides a spent block | N above 3 and at most 10 | Appears only in a Stop that already blocks for another finding, through the mechanism of #584 |
| Rejected | N above 10 | No row |

Two more conditions hold for every row:

- The row catches its planted case in the recall suite of #572.
- Before a row may block, a person spot-checks its replay labels. Agents
  drafted the earlier labels.

#591 builds the ride-along tier and states what it means at `klin check`.

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
