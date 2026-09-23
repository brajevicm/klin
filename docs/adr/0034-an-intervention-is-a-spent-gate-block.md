# An intervention is a spent gate block, not a followed finding id

> ADR 0052 gives a prompt two gate blocks. An intervention is one gate
> failure on a stop that itself spent gate block 1 or 2, which the stop's
> journal line records as `hook.gate_block`.

The journal of spec 9.6 exists so two readers — `klin stats` and the
benchmark — can count what klin caught. Both need one unit to count, and the
obvious unit is the finding: record each finding's `id` (11.2), follow it
across stops, and call it an intervention when it appears and later goes.

That unit does not hold up. Three facts break it:

- The `id` follows the path. A file rename changes every id in the file while
  the site of 4.4 survives, so a rename reads as one set of findings fixed
  and another set introduced.
- `doc-size` findings carry no `id` at all, so a finding-keyed count would
  silently drop one whole gate.
- ADR 0031 makes a deleted test green on the next stop by rule. A
  finding-keyed reader would count that as a fix the agent made, when the
  rule ended the episode.

## The decision

An intervention is one gate failure on a stop where klin spent the prompt's
gate block. One blocked stop with three failing gates is one blocked stop and
three interventions, and a reader reports both counts.

The definition uses only what ADR 0004 and ADR 0022 already made observable:
the stop blocked, and the build stamp records that the block was the turn's
gate block. The journal therefore stores no outcome and follows no id. Each
stop's line records what that stop saw — the 11.2 object, the `hook` facts,
the verdict — and every outcome is a relation between two lines, computed by
the reader. A reader can change its rule without rewriting history.

Finding `id`s stay in the record, inside the 11.2 object, because a reader
that lists what was caught still names sites. They key nothing.

## What this gives up

An intervention does not say which finding the agent went on to fix. A stop
that blocked on three gates and a next stop that is green is three
interventions resolved, with no per-finding attribution. The stories a
report tells come from the findings in the lines, not from followed ids, so
a renamed file can appear twice in a list of examples.

## What was ruled out

- Following finding ids across stops. The three facts above.
- Storing the outcome in the line as it is written. The writer would have to
  know the future, or rewrite earlier lines, and the hook never prunes or
  edits the journal.
- Counting blocked stops alone. One stop that catches three shortcuts would
  count as one, and the gate that blocks most would be unaskable — the
  question the journal exists to answer.

## Amended on 2026-09-16 by #172

The decision above stands for the hook. An intervention is still one gate
failure on a stop that spent the prompt's gate block, the journal still stores
no outcome, and every outcome is still a relation the reader computes.

Two parts of it no longer hold for the person's report.

**The person's counted unit is the Regression, not the intervention.** One
gate failure on a blocked stop was the right unit for the maintainer's
question, "which gate blocks most". It is the wrong unit for the person's
question, "what needs me". A stop that blocks on one gate with four sites is
one intervention and four things to fix, and the same four sites over four
blocked stops are four interventions and still four things to fix. `klin stats`
now counts one regression per finding site per window, with its latest outcome.

**Finding ids key the regression.** This ADR said ids key nothing, for three
reasons. Two of them are answered rather than gone:

- The id follows the path, so a rename produces a different id. The reader
  stays conservative: it never merges two ids because their text resembles each
  other, and a renamed site reads as a second regression. Over-counting is the
  honest failure here, and merging is not.
- A shape that carries no id — `doc-size` is one — falls back to the smallest
  conservative key its recorded fields allow: gate, file, line and text. The
  gate is neither dropped nor merged with the finding beside it.
- ADR 0031 stands untouched. A deleted test klin lets through is an
  `asked-once` question, excluded from the regression count, and never a fix.

Two rules were added with the unit, and both read the record this ADR already
required. A site absent from a stop that measured its gate went, whether or not
another site kept that gate red, so two sites under one gate resolve apart. And
the `config_hash` this ADR recorded and did not read is now read: a site that
went from a measurement taken under a different hash is reported as resolved
after the config changed, never as a code fix.

Spec 11.5 carries the result.
