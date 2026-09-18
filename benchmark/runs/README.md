# Run records

This is the control plane. Nothing here is visible to a coding agent: the
host's sandbox refuses the subject's shell every path under it, the host's file
tools refuse it the same paths, no path from a subject workspace leads to it,
and `src/integrity.ts` checks the layout for every trial.

`node benchmark/src/cli.ts probe` is what proves the first two, through one
real session that is told where this directory is and asked to read it. It
leaves `probe/<id>/probe.json`.

`calibrate` and `execute` write one directory per run, named by the trial id. That directory
is the trial's plane: the harness runs the trial out of it, and everything the
subject must not reach is in it.

```text
<stamp>/manifest.json          the protocol, the seed and the scheduled order; for a
                               publishable round, every round-wide frozen value too
<stamp>/scorecard.json         the unclassified mechanical package of a publishable round
<stamp>/<id>/crash.json        an attempt that crashed before a record existed, beside
                               whatever the trial had written
<stamp>/<trial>/record.json    the machine run record, against record.schema.json
<stamp>/<trial>/agent.json     the host's own result, with no hidden reasoning
<stamp>/<trial>/behaviour.json the hidden oracle's answer and its output
<stamp>/<trial>/hook           the wrapper, as it ran, under its settled name
<stamp>/<trial>/settings.json  the host wiring, which carries the arm
<stamp>/<trial>/state/         klin's own state and journal for the trial
<stamp>/<trial>/hooks/         every wrapped hook call, with its real status
<stamp>/<trial>/fixtures/base/    the tree the agent started from
<stamp>/<trial>/fixtures/final/   the tree the agent left
<stamp>/<trial>/fixtures/scoring/ the copy the hidden test ran over
```

Nothing under here is committed except this note. A set is evidence for a
published claim, not source. After the benchmark verifier accepts a completed
set, package it with:

```sh
node benchmark/src/cli.ts evidence-prepare benchmark/runs/<set> \
  --into benchmark/evidence/<set> --archive /path/to/<set>-raw.tar.gz
node benchmark/src/cli.ts evidence-verify benchmark/evidence/<set> \
  --archive /path/to/<set>-raw.tar.gz
```

Commit only the slim evidence and `evidence.json`; publish the exact raw
archive as an immutable release asset. Every record states the harness commit
and the klin source commit, so a reader can check out the tree that produced
it.

`benchmark/test/live-record.json` is one real record kept in the repository,
because two tests read a record and a hand-written one would drift from the
schema.

The subject's own repository is not here. It sits under the system temporary
directory, alone in its parent, so no relative path leads from it to this tree.

The tree copies sit under `fixtures/`, which is in klin's built-in skip set, so
records kept in this repository are never measured as klin's own source.

A calibration record says `publishable: false`, and `verify` refuses a set
that says otherwise. A publishable record says `publishable: true` and states
its `repetition` and, for a replacement, the trial id it `replaces`. The
original attempt's directory stays beside it; nothing here is deleted or
overwritten once written. Issue #115 excludes calibration from the product
scorecard.

`benchmark/test/live-record.json` is one live trial from 2026-09-17. It is a
protocol 1 record, so it holds neither `infrastructure.terms` nor
`isolation.outside`. The tests that read it write today's protocol over it. It
is evidence of what ran, not a set to verify.
