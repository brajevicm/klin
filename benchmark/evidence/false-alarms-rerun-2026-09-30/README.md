# Rerun of the #343 replay, 2026-09-30

The evidence for #412. The section "Rerun after #389, 2026-09-30" of
`docs/false-alarms-2026-09-29.md` states the results.

- `selection.json`: a copy of the first run's selection, the same 100 changes.
- `klin.provenance.json`: the commit and SHA-256 of the binary that ran.
- `runs/`: one record per change, as in the first run.

The rows and labels are those of `../false-alarms-2026-09-29/`. To build
the records again, with the clones in a directory of your choice:

```sh
KLIN_BIN=PATH node benchmark/src/cli.ts replay-run benchmark/evidence/false-alarms-rerun-2026-09-30 --clones DIR
```
