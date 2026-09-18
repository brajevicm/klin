# shadow-active-v1

The treatment-independent design of the publishable Shadow/Active round, as it
stood before any outcome existed. Issue #211 requires it here, committed, so a
reader can audit the design without trusting the run directory: `benchmark/runs`
is ephemeral and is written by the same operator who reads the results, while
this file is dated and reviewed by the history.

`protocol.json` holds the record protocol number, the frozen seed, the sample
plan, the analysis that was predeclared, the fixture identities and the whole
36-block, 72-row run order with its 18/18 first-arm balance.

It holds nothing of the machine. No binary digest, host version, model, flag or
machine fact is here, because a plan reads those and they belong to the run
directory's `manifest.json`. What is here is a function of the fixtures and the
seed alone, so any checkout of this commit gives it again:

```sh
node benchmark/src/cli.ts protocol          # does the catalogue still give this?
node benchmark/src/cli.ts protocol --write  # rewrite it, then review and commit
```

`plan` and `execute` both refuse a round that departs from this file, so a
changed prompt, fixture, seed, sample plan or schedule stops the round before a
session is paid for. `execute` reads the whole frozen environment again before
every block, and a hand edit to this file leaves the harness tree dirty, which is
itself a refusal.

The decisions that are prose rather than data are not here. The two arms, the
treatment-integrity procedure, the infrastructure-invalid criteria and the
outside-workspace validity rule are in `benchmark/README.md`. The Gate A and
Gate B checklists are in issue #211, and the rubric that reads the result is in
issue #115.
