# An unreadable file is a note in the hook, and exit 2 in CI

> Amends ADR 0003. The CI behaviour it chose stands. The hook behaviour
> changes.
>
> Spec 8.6 narrows this record for one case (#133): a `lockfile` manifest the
> survey derived that did not parse at either commit is a NOTE in every run.

ADR 0003 made a file the grammar cannot read a named tool error, exit 2, with
every other finding still printed. Its reason holds: a file klin cannot
measure is a hole in the ratchet, not debt anyone accepted, and a green run
over a hole is the worst thing the tool can do.

In the hook that exit 2 blocks the stop. A Flow-typed `.js` file is ordinary
in a React Native tree, and the JavaScript grammar refuses it on every stop.
The agent has no remedy. Only a person can exclude the file or update the
grammar. So the block costs a turn per stop and fixes nothing.

## The decision

In `--hook` mode a file no grammar reads is a NOTE naming the file and the
grammar that rejected it. It does not block. Under `--strict`, which is how CI
runs, it stays exit 2 as ADR 0003 chose. A person sees the hole where a person
can act on it, and the agent is not asked to fix what it cannot.

## Consequences

The hook's report is not silent about the hole. The note prints on every stop
until a person acts, which is the pressure ADR 0003 wanted, aimed at the party
that can respond to it.

`klin gate` by hand, without `--strict`, follows ADR 0003 and exits 2. Only
the hook is softened.
