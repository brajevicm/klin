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

## Amendment: `klin check` windows without `--strict` (#501)

vNext has no `--strict`, so the third case above no longer splits by flag.
Spec 6.5 decides every case of a base equal to HEAD with a clean tree:

- A remote source passes, with a note that the trees are the same.
- A local source passes when the remote default branch holds HEAD.
- A local source with commits the remote default branch does not hold is a
  hole, `comparison-unproven`, exit 3. It names those commits. It is no
  longer an error: klin found nothing wrong with the run, it only cannot
  prove the comparison.
- With no remote default branch at all, the run passes with a note that no
  remote proves what to compare.

The shallow CI clone this ADR kept shut through `--strict` is now shut by
spec 6.5 rule 1. A present `GITHUB_BASE_REF` that does not resolve, a push
`before` missing from a shallow clone, or a merge-base a shallow clone cannot
compute is exit 2, and the message names `fetch-depth: 0`. A push `before`
that a force-push rewrote away in a full clone falls back to the merge-base
with a note, because no fetch brings it back.
