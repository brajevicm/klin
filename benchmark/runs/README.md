# Run records

This is the control plane. Nothing here is visible to a coding agent: no path
from a subject workspace leads to it, and `src/integrity.ts` checks that for
every trial.

`calibrate` writes one directory per run, named by the trial id. That directory
is the trial's plane: the harness runs the trial out of it, and everything the
subject must not reach is in it.

```text
<stamp>/manifest.json          the protocol, the seed and the scheduled order
<stamp>/<trial>/record.json    the machine run record, against record.schema.json
<stamp>/<trial>/agent.json     the host's own result, with no hidden reasoning
<stamp>/<trial>/behaviour.json the hidden oracle's answer and its output
<stamp>/<trial>/hook           the wrapper, as it ran
<stamp>/<trial>/settings.json  the host wiring, which carries the arm
<stamp>/<trial>/state/         klin's own state and journal for the trial
<stamp>/<trial>/hooks/         every wrapped hook call, with its real status
<stamp>/<trial>/fixtures/base/    the tree the agent started from
<stamp>/<trial>/fixtures/final/   the tree the agent left
<stamp>/<trial>/fixtures/scoring/ the copy the hidden test ran over
```

The subject's own repository is not here. It sits under the system temporary
directory, alone in its parent, so no relative path leads from it to this tree.

The tree copies sit under `fixtures/`, which is in klin's built-in skip set, so
records kept in this repository are never measured as klin's own source.

A calibration record says `publishable: false`, and `verify` refuses a set
that says otherwise. Issue #115 excludes calibration from the product
scorecard.

`ad-hoc/0ce3ce1cb732` is one live trial from 2026-09-17, kept as raw evidence.
It is a protocol 1 record, so it holds neither `infrastructure.terms` nor
`isolation.outside`, and `verify` names it as a protocol mismatch. It is
evidence of what ran, not a set to verify.
