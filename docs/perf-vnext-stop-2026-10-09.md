# vNext Stop performance evidence (2026-10-09)

This document records the performance evidence for the vNext Stop (#506),
against `docs/SPEC.md` sections 14.2, 14.4 and 19.3. It compares the vNext
Stop with the baseline of `docs/perf-baseline-748d01fc-2026-10-02.md`.

- Current: the binary built from `src/` at `ff94578d` (main after #503).
  The Stop runs through `klin __agent event`, with the history check of
  section 6.6, the `aborted` write and the told-once record.
- Baseline: the binary built from `748d01fc`, through `KLIN_BIN`. Its Stop
  is the 0.x `klin gate --hook --changed`.
- Machine: macOS, aarch64 (Apple Silicon), developer laptop. This is a
  contributor machine, not the controlled machine.
- Each value is the median of 5 iterations. The times exclude the project
  build command.

## 1M rows (owner)

The owner takes these rows on the controlled machine (section 14.2). They are
not recorded yet.

```sh
KLIN_PERF_ROW=structural_1m KLIN_PERF_CASE=warm20  cargo test --release --test performance -- --ignored perf --nocapture
KLIN_PERF_ROW=structural_1m KLIN_PERF_CASE=warm100 cargo test --release --test performance -- --ignored perf --nocapture
```

| Row | Baseline at `748d01fc` | vNext | Limit |
| --- | --- | --- | --- |
| warm 1M/20 hook median (ms) | 1218 | not taken | 1,500 ms admission envelope |
| warm 1M/100 hook median (ms) | 1567 | not taken | a rise above one third needs an explanation |

## 300k rows (contributor)

Fixture: `source-dense-300k`, 10,000 files, 325,077 LOC, 90,007
declarations, `complexity_scope=whole`, `config=build-off`, `layering=on`.
The rows ran one after the other in the foreground, with the binaries
alternated. Each run builds its own fixture.

```sh
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm100 cargo test --release --test performance -- --ignored perf --nocapture
```

The baseline runs are the same commands with `KLIN_BIN` set to the
`748d01fc` release binary.

| Run, in order | Binary | Changed files | Hook median (ms) | `stop_total_ms` | Peak RSS (KB) |
| --- | --- | --- | --- | --- | --- |
| 1 | vNext | 20 | 765 | 736 | 148,848 |
| 2 | baseline | 20 | 766 | 736 | 151,968 |
| 3 | vNext | 20 | 775 | 744 | 148,912 |
| 4 | baseline | 20 | 777 | 747 | 157,024 |
| 5 | vNext | 100 | 950 | 920 | 148,576 |
| 6 | baseline | 100 | 960 | 928 | 156,224 |

The 300k relative change of the vNext Stop is within noise: 765 and 775 ms
against 766 and 777 ms at 20 files, and 950 ms against 960 ms at 100 files.
Most gate times agree to within 2 ms in each pair. Two gates are slightly
faster on vNext: `dead-symbols` is 219 and 220 ms against 231 and 232 ms, and
`reachability` is 128 and 129 ms against 133 and 135 ms.

The warm rows make no commit between Stops. The `git merge-base` of section
6.6 runs on the first event after a commit, which is the session event that
primes the fixture, so these rows do not time it. They do time the file reads
of the history check, the `aborted` write and the told-once record on each
Stop.

## A `{check}`-placed capability at the Stop

`sarif` is the one `{check}`-placed capability (section 9.1).
`KLIN_PERF_SARIF=on` adds a `sarif` entry named `scanner` to the fixture's
`klin.json`, whose `run` command writes a marker file. On every timed Stop the
row asserts that the marker file does not exist and that the journal line has
no `scanner` gate row. A Stop that started the command or planned the entry
fails the row.

```sh
KLIN_PERF_SARIF=on KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture
```

All three runs use the vNext binary.

| Run, in order | `sarif` | Hook median (ms) | `stop_total_ms` |
| --- | --- | --- | --- |
| 7 | on | 844 | 812 |
| 8 | off | 843 | 809 |
| 9 | on | 796 | 766 |

Both `sarif=on` runs passed the assertions, so no Stop ran the command or
planned the entry. The medians of runs 7 and 8, taken one after the other,
differ by 1 ms. The machine drifted by about 70 ms between run 1 and run 8,
which is larger than any difference these rows can show.

## `pre_tool`

The guard row of the default rows sends 1,000 `pre_tool` events to the
2,000-file fixture, 5 times. Each event is one process, timed from the test.
The row now prints the 50th and 99th percentiles and the maximum of the 5,000
single events.

```sh
cargo test --release --test performance -- --ignored perf --nocapture
```

Section 14.4 sets the limit: `pre_tool` within 50 ms.

| Run | Mean per event (ms) | p50 (ms) | p99 (ms) | Max (ms) | Machine |
| --- | --- | --- | --- | --- | --- |
| A | 6.41 | 7.83 | 16.80 | 24.60 | quiet |
| B | 11.68 | 9.59 | 19.85 | 67.26 | load average 16 |

On the quiet machine, all 5,000 events completed within 50 ms. Run B ran while
other processes loaded the machine: its 10,000-file cold survey took 30,709 ms,
against 15,302 ms on a quieter run. The p99 of run B stays within 50 ms, and
its maximum does not. The time includes the process start, which is outside
klin, and the first event of the row starts a binary that macOS had not run
before.

Run A took its p99 at rank `len * 99 / 100`. Run B used the final harness,
which takes the nearest rank, `ceil(0.99 * len) - 1`. Two runs are left out:
one from an intermediate harness that printed only the maximum, and one that a
Stop hook of this repository disturbed while it ran.

## Harness changes

- The Codex events of the guard row did not carry `hook_event_name`, which
  the ingress of #498 reads to find the event kind. The ingress let them
  through, and the row failed on its first Codex deny. The events now carry
  `"hook_event_name": "PreToolUse"`, as Codex sends them.
- The fixture primes its state with `klin radius` when `KLIN_BIN` names a
  binary without the agent ingress, as the baseline row needs.
- `KLIN_PERF_SARIF=on` and the guard percentiles are new.
