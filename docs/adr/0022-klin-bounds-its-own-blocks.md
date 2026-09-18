# klin bounds its own blocks

> ADR 0048 amends this: the count rises only at a stop whose tree changed
> since the last block, and the stamp records that tree.
>
> Amends ADR 0004 and ADR 0012. The block-once policy stands. What changes is
> who bounds a build failure that never clears.

ADR 0004 let a build failure block every stop until the tree compiles, and
relied on Claude Code overriding a Stop hook after eight consecutive blocks
without progress. The current Claude Code hooks documentation does not state
that cap. A bound that lives in another program's undocumented behaviour is
not a bound klin can promise.

## The decision

klin bounds its own blocks. A gate failure blocks once per turn, as before. A
build failure blocks at each stop until the tree builds, and after eight
build blocks in one turn klin stops blocking, says so, and lets the turn end
with the failure in the report. The build stamp in the state directory counts
the blocks. The turn stamp of ADR 0017 resets the count when it moves.

## Consequences

A tree that never compiles cannot trap the agent, whether or not the host caps
anything. If the host does cap, the two bounds agree at eight.

Two stops of one turn are two processes, so the count crosses them through
the build stamp, which ADR 0012 already writes for the fact that the build
blocked at all. The stamp gains a counter and nothing else.

Amended 2026-09-09. The decision above said the turn stamp resets the count
when it moves. The stamp moves only after a green stop, and a build failure
never ends green, so after eight build blocks the count never reset and the
build never blocked again in that window. The count now sits beside the
prompt counter it was taken under. `klin radius` raises that counter in the
`turn` file on every session start and prompt submitted, whether or not the
stamp moves, and a count taken under an earlier prompt reads as zero. The
block budget is per prompt, which is what a turn is, and the window stays
per stamp. A build-failure stop also writes a red verdict before it blocks,
so the next prompt does not move the stamp over a tree that does not build.
`docs/SPEC.md` 6.2, 9.3 and 16.3 carry the rule.

The build stamp is a record now, not a marker, so it retires two details of
ADR 0004. No run deletes it: a record from an earlier prompt reads as zero,
which is what ends its life. And a stop that passes no longer ends it either,
so the gate's one block per turn is spent where the record says it is, apart
from the build blocks. That is what ADR 0004 wanted and could not have while
the only state was a file's existence.
