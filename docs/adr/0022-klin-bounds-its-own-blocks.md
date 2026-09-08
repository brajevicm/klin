# klin bounds its own blocks

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
