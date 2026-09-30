# The relaxations #389 rejects

> Records decisions. It changes no rule. Spec 7.1, 8.2 and ADR 0008 stand.

#389 asked which of the relaxations the replay of #343
(`docs/false-alarms-2026-09-29.md`) suggested klin should make. ADR 0059,
ADR 0060 and ADR 0061 record the three it makes. This record keeps the rest,
with the reason each one fails. The labels are the replay's: an agent drafted
them and agents reviewed them.

## Q2: a longer function over its `cc` ceiling is worse

A site over its `cc` ceiling still worsens when only its `lines` rise, even
under the `lines` ceiling. No `+N` or percentage tolerance applies.

SlopCodeBench measures erosion as the share of `CC × √SLOC` that functions
over CC 10 hold, and that share rose in 77% of agent trajectories. A complex
function that grows longer is the erosion the measure counts, so the ratchet
names it.

The same holds for a site over only its `lines` ceiling whose `cc` rises under
the `cc` ceiling. A new branch almost always adds lines, so a separate rule
for that case would clear no labeled row.

## Q3: the declaration-line key stays

`complexity` keeps the declaration-line key of ADR 0008. A function whose
declaration line changed stays `new`. R002, R027 and R045 changed their bodies
too, and grew from a base already over the ceiling. A name-based key would
turn them from `new` into `worsened`, and they would still fail.

## Q4: cyclomatic complexity stays classic

Each arm of a `match` or `switch` still counts. A lookup table such as
`try_name` (R019, R029) or the dispatch match of R016 stays far over the
`lines` ceiling, and each new arm adds a line. Counting the construct once
would clear none of those rows, and it would loosen every existing pin. JSX
components and module wrappers get no exemption either.

## Q5: `lines` stays physical lines

`lines` counts physical lines, nested functions included. A `cargo fmt`
reflow (R033) and the wrapper in R071 add code lines that SLOC counts too, and
a change to what klin measures would move every pin.

## Q8: a production non-null `!` stays an escape

No local-proof exemption applies, such as for `match[1]!` after a successful
match or `child.stdout!` after a piped spawn (R047, R067, R074). `escapes`
matches lines. To see that a local check proves the value present, it would
need a flow analysis per language, and a narrow one would pass the same `!`
where no check proves anything. A person who judges one site safe accepts it
in the `accepted` list, in a reviewed commit.

## Q9: `|| true` stays an escape

No list of commands that are safe to ignore applies, such as best-effort
cleanup (R034, R041). Whether a failure is safe to ignore depends on the
script, and a list in the binary would hold that policy for every tree. A
person accepts the site instead.

## Q10: `#[ignore]` and `#[allow(...)]` stay escapes

`#[ignore]` stays an escape with or without a reason, such as a manual or
network test (R018), and so does `#[allow(...)]`, such as
`#[allow(clippy::too_many_arguments)]` (R018, R024, R028). No tiers apply. A
reason is text that nothing checks, so it silences nothing, as ADR 0060
decided for `@ts-expect-error`. A tier would let the reason decide what the
gate judges.
