# Dead-symbol judgement state, scoped to what a run judges (#196)

`dead-symbols` measured complete structural evidence over both trees and then
built an owned `State` for every eligible declaration in every file, though
only the declarations in the run's judgement scope could become a finding, a
prior entry or a note. A changed run over a large repository built tens of
thousands of states to report on twenty files.

The evidence boundary is unchanged. Both trees still extract and index every
file the gate's selection holds, so a declaration in a changed file is alive
on a reference from any measured file, changed or not. What moved is the
judgement boundary: in a changed run that is not strict, both trees build
declaration state only for the files the run judges, which is the same
`Context.only` scope the ratchet already restricted findings and prior entries
to. The base tree decides paths, so a renamed file is judged under the path it
has today, as before. Whole runs, `--strict` runs and the check by hand judge
every eligible declaration.

`gate --json` now carries `states` in the `dead-symbols` row's `facts`: how
many declaration states the gate built over both trees. No other gate carries
the key. `tests/dead_symbols.rs` pins the boundary with
`a_changed_run_builds_no_state_for_the_declarations_it_does_not_judge`:
declarations added to unchanged files outside the judgement scope leave a
changed run's `states` where it was, while a whole run's grows.

## Same output

`tests/structural_views.rs` passed with `KLIN_DIFF_BIN` set to a release build
of `3520b04`, over all seventeen scenarios and the five compared callers. That
harness already removes `facts` from each gate row, which is where the new
counter sits, so nothing else needed normalizing. `reachability` and
`layering` are untouched.

## Measurements

Measured 2026-09-15 on the 0.1.1 baseline machine (MacBook Pro 18,3, Apple M1
Pro, macOS 26.6.2), release builds, median of five iterations of spec 13's
dense rows. Both columns ran through `KLIN_BIN` in the same session, the
before column at `3520b04` and the after column at this change. The 20-file
rows are the dense warm hook; the 100-file rows are its `changed_files=100`
delta row.

| Row              | Whole hook before | Whole hook after | `dead-symbols` before | `dead-symbols` after |
| ---------------- | ----------------: | ---------------: | --------------------: | -------------------: |
| 300k, 20 files   |          3,369 ms |         2,317 ms |              1,837 ms |             1,209 ms |
| 300k, 100 files  |          2,493 ms |         2,192 ms |              1,302 ms |             1,146 ms |
| 1M, 20 files     |          2,735 ms |         2,392 ms |              1,580 ms |             1,261 ms |
| 1M, 100 files    |          3,134 ms |         2,772 ms |              1,776 ms |             1,414 ms |

Peak RSS, from the same rows' `resource:` line:

| Row  | Warm hook before | Warm hook after | `dead-symbols --changed` before | after      |
| ---- | ---------------: | --------------: | ------------------------------: | ---------: |
| 300k |       286,944 kB |      140,432 kB |                      281,696 kB | 125,488 kB |
| 1M   |       810,560 kB |      312,400 kB |                      831,776 kB | 328,416 kB |

The rows that judge everything moved too: the 1M strict row reads 35,348 ms
before and 30,563 ms after, and the 1M cold survey 51,414 ms and 48,339 ms.
Those runs build the same state before and after this change, so the
difference is run-to-run variation on an uncontrolled machine and not an
effect of it. The warm rows' `facts` counters are identical in both columns,
which is the check that extraction was not narrowed.

## Follow-up, 2026-09-17 (#237)

The optimization above stands, but the sentence that a changed run's judgement
state is *only* `Context.only` was too narrow. Liveness is a property of a
name, not of a file, so a turn that changes only the caller can turn an
unchanged declaration from referenced to dead. Under the scope written here,
the Stop hook missed exactly that refactor.

The changed run now judges its physical scope plus the declarations indexed
under the reference names the changed files contribute on either semantic
side. Both parts come from the same complete before and after `SourceIndex`
this note already keeps, so the cost model is unchanged in shape: state grows
with the changed files' name fan-out, not with the repository. A changed file
the grammar could not read widens nothing, so a coverage hole stays a hole.
