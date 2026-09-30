# False alarms of the default configuration, 2026-09-29

The evidence for #343. `docs/false-alarms-2026-09-29.md` states the rules and the results.

- `search-rust.json` and `search-typescript.json`: the two GitHub search responses.
- `selection.json`: the ten repositories, their ten changes each, and the one skipped repository.
- `klin.provenance.json`: the commit and SHA-256 of the binary that ran.
- `runs/`: one record per change, with the exit status and the JSON report of `klin gate --json`.
- `worksheet.md` and `worksheet.json`: the 79 replay rows.
- `journal-worksheet.md` and `journal-worksheet.json`: the 138 rows from klin's own journal.
- `labels.json`: one entry per row. A person sets `label` to `appropriate` or `not-appropriate` and may add a `note`.

To build it again, with the clones in a directory of your choice:

```sh
node benchmark/src/cli.ts replay-select benchmark/evidence/false-alarms-2026-09-29 --clones DIR
KLIN_BIN=PATH node benchmark/src/cli.ts replay-run benchmark/evidence/false-alarms-2026-09-29 --clones DIR
node benchmark/src/cli.ts replay-worksheet benchmark/evidence/false-alarms-2026-09-29 --clones DIR --journal FILE
```

`replay-select` reuses the saved search responses, and `replay-run` skips a change whose record exists.
