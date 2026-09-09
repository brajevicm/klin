# The report measures a prompt mark, not the turn stamp

> Amends ADR 0014. The report and its rules stand. What changes is which
> mark it measures from.

ADR 0014 gave the radius report the turn stamp as its window, because at the
time the stamp was the only mark klin took at a prompt. ADR 0017 then made the
stamp the hook's base as well, and spec 6.2 froze it: the stamp moves on a
first session or after a green stop, and otherwise it stays where it is.

One mark cannot serve both. The ratchet needs a mark that waits, so debt an
agent left behind stays `new` until a person fixes or accepts it. The report
needs a mark that always moves, because it describes the turn that just ended.

Keyed to the frozen stamp, the report fails in two ways after a red stop. The
window it measures grows with every prompt and never falls back, so a total
that several turns built is announced as one turn. And once that total passes
the value, the report prints again on every prompt until a green stop or a
`klin turn reset`. ADR 0014 made the report rare on purpose, because its
rarity is the signal. A message that always arrives is one an agent learns to
skip.

## The decision

klin takes a second mark, the prompt mark, under `refs/worktree/klin/mark`. It
is a commit over the same tree the stamp commits, with HEAD as its parent, and
it moves on every session start and every prompt, whatever verdict the last
stop left. The turn stamp keeps its own rule and its own ref, untouched.

The spread report measures from the prompt mark. Nothing else does. The stamp
remains the window a stop judges, the base of the derivation commit, and the
one mark a person moves with `klin turn reset`.

The `turn` file carries the mark beside the stamp, in the same atomic write.
A mark the file has lost is read from the ref, and a mark neither holds means
the report prints nothing, which is what ADR 0014 already says of a window it
cannot measure.

## Consequences

Each event writes one more commit than before. The tree is hashed once per
event and both marks commit that one tree, so the cost is one `commit-tree`
and one `update-ref`, not a second walk of the working tree.

The ref keeps `git gc` from pruning the mark, the same argument ADR 0017 makes
for the stamp. It sits under `refs/worktree/klin`, which section 9.4 already
guards as a namespace, so the guarded set does not grow.

A session start moves the mark, so work a person did between two sessions is
not reported as the agent's turn. That is the reading the report wants.

`klin radius --report` measures from the mark as well, so a person and the
hook see the same number. It still moves neither mark.

Deleting the mark costs a report and nothing else. Unlike the stamp, no block
and no window depend on it, so it needs no recovery beyond the ref.
