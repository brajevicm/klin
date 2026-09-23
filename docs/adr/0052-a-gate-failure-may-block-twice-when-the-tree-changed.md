# A gate failure may block twice when the tree changed

> Amends ADR 0004 and ADR 0022. Cites ADR 0048 for the identity of a working
> tree, ADR 0031 for a deleted test and ADR 0034 for the intervention. The
> build budget of eight blocks stands unchanged.

ADR 0004 and ADR 0022 let a gate failure block one stop per prompt. The next
stop reported and let the turn end, even when the agent had edited the tree in
between and the gate still failed.

Two benchmark trials showed the cost. In dea248b3aa13 the agent rewrote only
the root block of a v1 `package-lock.json` after the block. `lockfile` still
failed, klin printed that it would not block a second time, and the agent's
reply said the change "should satisfy the `lockfile` gate". In b647fbfa8993 the
agent made `height` optional on the exported `Point` after a `public-api`
block. The second stop reported the break again and did not block. In both
runs the report went to stderr under exit 0, and the agent's final reply
claimed the problem was fixed. Each failure needed exactly one more round of
feedback.

## The decision

**A gate failure may block two stops under one genuine prompt.**

- The first failing stop may spend gate block 1 of 2, as before.
- After that block, a failing stop over the same working tree spends no block.
  It reports and lets the turn end. This keeps the route #287 added: the agent
  says a break is intended, stops again without an edit, and the turn ends.
- After that block, a failing stop over a different working tree may spend
  gate block 2 of 2. Any gate failure or tool error may spend it: the same
  finding as before, or a new failure the repair introduced.
- After gate block 2, no gate failure under that prompt blocks again, whether
  or not the tree changes.
- Each spent gate block names its number, as build blocks already do.

**The tree is the one ADR 0048 hashes.** A gate block records the working tree
it was taken over, hashed the way a build block hashes it.

**Build and gate state are separate.** The build stamp records `builds` and
`build_tree` for build blocks, and `gate_blocks` and `gate_tree` for gate
blocks. A build block changes only the build fields, and a gate block changes
only the gate fields. So a build block between gate block 1 and a later gate
failure cannot replace the tree gate block 1 saw. Shared state would make the
second-block decision depend on which kind of block came last.

**klin proves the second block from its own record.** Gate block 2 needs a
recorded `gate_tree` and a current tree that differs from it. A host's
`blocked_before` flag says that a block happened, never which tree it saw.
Where klin's record holds no gate block and no build block, it counts as a
gate block klin never recorded, so the stop spends none, as before. It cannot
authorize a second. When klin cannot read the previous gate tree, cannot hash the
current tree, or cannot write the record of a block, it reports and spends no
block. That includes the first gate block. A first block klin could not record
would read as unspent at the next stop, and Cursor sends no prior-block flag,
so it would be taken again at every stop and the cap of two would bound
nothing. A host flag does not rescue it either, because after a build block
the flag no longer says which kind of block happened. This is the
conservative rule of ADR 0022 and spec 14: local state that cannot bound a
loop blocks nothing. It reverses the older rule that an unrecorded first gate
block still blocked.

**A stop that lost the state lock spends nothing.** The count is read and
written by the stop that holds the lock. A stop that waited out the hook's
lock budget runs beside another stop, so both could read the same count and
both deliver the same numbered block. It measures and reports, and spends
neither a build block nor a gate block (spec 6.5).

A record an older klin wrote names one `gate_spent` flag and no gate tree. It
reads as one gate block spent over an unknown tree, so it can never prove a
second block.

**A host-generated follow-up refreshes nothing.** Cursor submits a block
report, and any message a stop tells the person, as the next prompt (spec 9.1,
9.3, ADR 0045). klin records both before delivery, so that prompt is consumed
without raising the prompt counter, and the stop after it still holds the
budget the report came from. A told message was not recorded before this
decision, so a red turn on Cursor gained a fresh gate block from each turn-end
message it told.

A consumed follow-up also opens no turn for the agent to end, so a stop over
the same state would tell the same message, Cursor would submit it, and the
loop would repeat. klin therefore records the last message it told a
follow-up host under the current prompt, and a later stop under that prompt
tells nothing when its message is the same. The comparison leaves out the
window line, whose age moves each minute and says nothing new. A different
message is told, and the session's next prompt clears the record.

Both records are kept per host session, in a file of that session's own under
`handed/` in the state directory, because two sessions can share one
worktree. With one shared record in the `turn` file, a second session's told
message replaced the first session's pending block report, the first
session's echo then opened a turn, and its budget refreshed. A shared file
with a record per session still lost updates: `klin radius` consumes a
follow-up without the stop's lock, so it could read the file, let a stop of
another session write that session's record, and then write its older copy
over it. A session's file is written only by that session's hooks, which a
host runs in order, so neither the sequential nor the concurrent case can
replace another session's record.

A follow-up host hears only what klin recorded first. A stop that lost the
state lock, or whose handoff record would not write, tells it nothing,
because an unrecorded message would replay. A block whose report klin could
not record is reported and not blocked, because the host would submit it as
a person's prompt and gain a fresh budget. The count that block already took
stays spent, so the budget only shrinks.

A stop that lost the state lock also reads its window read-only. It restores
no missing `turn` file, re-anchors no abandoned stamp and writes no
replacement, because the stop holding the lock may be writing the same
stamp (spec 6.5). A genuine later prompt raises the counter and gets a
fresh budget of two gate blocks and eight build blocks.

**ADR 0031 keeps its precedence.** A deleted test already asked about stays
ask-once. It is a NOTE, it fails no gate, and it is no reason for gate block 2.
A newly deleted test is a gate failure like any other and may spend a gate
block that remains. The cap of two is absolute: once both gate blocks are
spent, a deletion cannot make a third. The stop reports it and records no
question. If the deletion remains under a later genuine prompt, ADR 0031 asks
about it then.

**The intervention is read from the stop that spent the block.** ADR 0034
defined an intervention as one gate failure on the stop that spent the
prompt's gate block. With two blocks:

- one failing gate on the stop that spent gate block 1 is one intervention;
- the same gate failing on the stop that spent gate block 2 is another;
- one blocked stop with three failing gates is three interventions;
- a build block after a gate block is no gate intervention, even though the
  build stamp already records a spent gate block.

The journal's cumulative `gate_spent` and `gate_blocks` cannot tell those
stops apart, because a later stop observes them too. Each stop line therefore
also records `gate_block`, the number of the gate block that stop itself
spent, or null. A reader counts interventions from the lines whose
`gate_block` is set. The Regression reader of spec 11.5 opens a Regression
only on such a line, so a build block opens none. A line an older klin wrote
records no `gate_block`, and its `blocked` stands in. The reader still keys
on finding sites, so one site delivered on both blocked stops is one
Regression.

## Why two and not more

Both observed failures needed one more round of feedback, and nothing in the
evidence supports a third. The changed-tree condition keeps a block off a tree
the agent did not touch. The cap keeps a gate from trapping the agent in
repeated attempts to satisfy it. CI stays authoritative for a failure the turn
leaves unresolved (ADR 0009).

## Consequences

- A second block adds pressure toward a repair that satisfies the gate without
  fixing the problem. v1 85964f1abb8c replaced `unwrap()` with indexing "to
  clear the escape-site check", and the optional `height` of b647fbfa8993 is
  the same kind of move. The seeded confirmation round measures it.
- A prompt whose gate fails twice costs the agent one more round.
- A gate failure hashes the working tree once, through the build stamp's own
  index, at a stop that may spend a gate block. A stop that finds both blocks
  spent hashes nothing.
- `docs/SPEC.md` 9.3, 9.5, 11.4, 11.5 and 16.3 carry the rule.
