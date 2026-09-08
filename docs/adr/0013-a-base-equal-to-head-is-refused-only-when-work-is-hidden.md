# A base equal to HEAD is refused only when work is hidden

> ADR 0017 takes `--hook` mode out of this record. The hook compares against
> the turn stamp and never reaches the rule below. `klin gate` by hand and CI
> still follow it.

ADR 0009 made a base equal to HEAD a tool error, flat, with no exception. The
rule fired on an honest case. A session works on the default branch, changes
nothing, and HEAD already sits at the remote tip. Nothing is hidden there, and
the stop hook has nothing to block, yet klin exited 2 and told the operator to
work on a branch.

The refusal now sorts by where the base came from, and only when the working
tree is clean. A dirty tree never reaches the rule, as before.

A base from a remote reference or from a push event's commit passes. The
remote holds what HEAD holds, so a later run against a real diff will measure
the same code.

A base from a local reference makes klin resolve the tip of the remote default
branch, and the answer decides:

- The tip resolves and commits sit between it and HEAD. Exit 2, naming those
  commits. They are on the default branch and the remote does not have them.
- The tip resolves and nothing sits between it and HEAD. Pass.
- No tip resolves. Exit 2 under `--strict`, pass otherwise.

Each branch carries its own message. The old one gave a CI operator the wrong
advice for a missing fetch.

## Why the split falls here

The failure ADR 0009 named is real: a run that reports green while measuring
nothing is the worst thing this tool can do. The hole that mattered most, a
shallow CI clone, stays shut. Such a clone resolves no remote tip, and under
`--strict`, which is how CI runs, that is still exit 2.

The agent that commits `wip` on the default branch never reaches this rule at
all, and did not need the flat rule either. Once a remote tip exists and HEAD
has moved past it, the merge-base with that tip is not HEAD, so the run
measures the commit the ordinary way. The rule here catches the case after
that one: the tip is unreachable, or the base came from a local reference,
and commits still sit above what the remote holds.

What the flat rule cost was a false refusal on every clean local run whose
HEAD matched the remote. A remote reference agreeing with HEAD is evidence,
not an assumption, and it is the evidence the flat rule threw away.

`--strict` decides the case klin cannot answer. Without a remote tip there is
no way to tell a fetched tree that is up to date from a shallow clone hiding
history. CI asks for the strict answer and gets the refusal. A local run asks
for feedback and gets its gates, which is the same bargain ADR 0009 struck for
the local merge-base.

## What this supersedes

ADR 0009's "Which two trees" paragraph, in its claim that a base equal to HEAD
is a tool error, never a pass. Everything else in that ADR holds: the base
candidates and their order, the merge-base locally, the host's event data in
CI, and the rename handling.

## Consequences

`--strict` gains a fourth failure, next to the unaccounted gate, the config
error, and the accepted entry that matches nothing.

A clean tree on the default branch at the remote tip now runs every gate and
compares each tree against itself. Every finding is held, so the run passes
and prints the gates it ran. That costs one whole-tree parse of a base that is
HEAD, which is the price of a report over a refusal.

klin now reads `refs/remotes/origin/*` for a second purpose. Before this, the
remote refs were base candidates only. Now they are also the answer to whether
the local branch is pushed.
