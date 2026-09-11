# An intervention is a spent gate block, not a followed finding id

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
