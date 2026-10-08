# An unreadable file is a note in the hook, and exit 2 in CI

> Amends ADR 0003. The CI behaviour it chose stands. The hook behaviour
> changes.
>
> Spec 8.6 narrows this record for one case (#133): a `lockfile` manifest the
> survey derived that did not parse at either commit is a NOTE in every run.
>
> The amendment below (#342) extends that narrowing to every check: a file the
> base held and klin could not measure there either is a NOTE in every run. It
> also ends the part of #133 that noted a manifest the base did not hold.
>
> The second amendment below (#500) ends the exit 2 outside the hook and the
> NOTE in it for a file the change made unmeasurable.

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

## Amendment: a hole the base holds too is a note (#342)

Outside the hook this record kept exit 2 for every file klin could not
measure. A repository that already holds such a file is then red the day klin
arrives, and it stays red for every change that does not touch the file. A
review on 2026-09-25 found this on public repositories: files the Rust and
TypeScript grammars reject, cfg-gated modules, and `export =`. For
`public-api` the only way back to green was `"public_api": false`, which
measures nothing.

A file the base held and could not measure either is a NOTE in every run,
with or without `--strict`. The change opened no hole there, and a person
decides whether to update the grammar or exclude the file. The base's side is
the check's own measurement of `before`, so the two trees are judged by one
binary under one rule (ADR 0001). A form a resolver could not resolve counts
as held when the base holds a form in the same file with the same text and
reason, paired one to one, so a second copy of a held form is new.

A file that the base could measure and the change makes unmeasurable stays
exit 2 outside the hook. That includes a rename from a path another grammar
reads, because the base's bytes were read under a grammar the base path does
not select. A rename between two paths one grammar reads keeps the NOTE. A file the base did not hold stays exit 2 too, a `lockfile` manifest
included. #133 noted such a manifest so that a fixture invalid on purpose
could never keep the gate red. The base rule now covers that: only the change
that adds the fixture fails outside the hook, a person closes it with
`except`, and every later run notes it. In the hook it is a NOTE. A manifest
the change only renamed is one the base held, so it keeps the NOTE. A manifest
renamed to another format has no comparable base, so it is judged as one the
base did not hold.

## Amendment: the hook blocks on a lost file, and nothing is exit 2 (#500)

Spec 7.2 sorts each file klin could not measure once per run, the same way at
the Stop and in `klin check`.

A file that the base measured and the change made unmeasurable is a
`measurement-lost` FAIL in every run. At the Stop it blocks like any other
FAIL. This reverses the NOTE above for that case: the agent can repair a file
it broke, and a NOTE let it hide a finding by breaking the file. A rename is
read on the base side with the base path's reader, so a rename from a path
another grammar reads is lost only when that reader read the base's bytes.

A file the change made unmeasurable with no clear agent cause, such as a new
file the grammar rejects or a new manifest that does not parse, is an
`unmeasured` review item at `klin check` and a note the agent sees at the
Stop. It is no longer exit 2 outside the hook.

A file the base could not measure either stays klin's own limit, now a
coverage note. The Stop says nothing about it.

Only a person can hold a lost file whose grammar lags, with the accepted entry
`{"gate": "measurement-lost", "file": PATH}`. The Stop tells the person how
only when it does not block, through the host's person channel, and never on
a host that hands a told message to the agent.
