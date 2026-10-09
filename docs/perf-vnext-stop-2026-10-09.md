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

These are the rows of the `748d01fc` baseline. Add `KLIN_PERF_ORIGIN=on` to
include the default-branch reads of the history check (see "The history
check" below).

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

| Run, in order | Binary | Changed files | Hook median (ms) | `stop_total_ms` | Outside the gates (ms) | Peak RSS (KB) |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | vNext | 20 | 765 | 736 | 190 | 148,848 |
| 2 | baseline | 20 | 766 | 736 | 174 | 151,968 |
| 3 | vNext | 20 | 775 | 744 | 192 | 148,912 |
| 4 | baseline | 20 | 777 | 747 | 181 | 157,024 |
| 5 | vNext | 100 | 950 | 920 | 231 | 148,576 |
| 6 | baseline | 100 | 960 | 928 | 236 | 156,224 |

"Outside the gates" is `stop_total_ms` minus the sum of the gate medians. The
gates run one after the other, so this is the Stop's own work: the lock, the
base layout, the journal and, on vNext, the work of section 14.4. A sum of
medians is an estimate, not a measured value.

The 300k relative change of the vNext Stop is within noise: 765 and 775 ms
against 766 and 777 ms at 20 files, and 950 ms against 960 ms at 100 files.
Most gate times agree to within 2 ms in each pair. Two gates are slightly
faster on vNext: `dead-symbols` is 219 and 220 ms against 231 and 232 ms, and
`reachability` is 128 and 129 ms against 133 and 135 ms. The vNext Stop spends
about 11 to 16 ms more outside the gates at 20 files, which the faster gates
balance. In the bands of section 14.3, that added work is normal (10 to
25 ms). At 100 files, the difference outside the gates is within noise.

## The history check

The fixture of runs 1 to 6 has no remote, so it has no default-branch ref
(`refs/remotes/origin/HEAD`, `origin/main` or `origin/master`). Without one,
the history check of section 6.6 reads HEAD, its symbolic ref and its reflog,
and then stops: no default-branch read, no merge-base cache read and no
`git merge-base`. The 1M rows of the `748d01fc` baseline have the same shape.

`KLIN_PERF_ORIGIN=on` points `refs/remotes/origin/main` at the base, so each
Stop also reads the default-branch ref and the merge-base cache file. The
`git merge-base` runs when HEAD or the default branch moves, which is at the
prime in these rows, and the warm Stops read its cached result.

```sh
KLIN_PERF_ORIGIN=on KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture
```

| Run, in order | `origin` | Hook median (ms) | `stop_total_ms` | Outside the gates (ms) |
| --- | --- | --- | --- | --- |
| 10 | on | 746 | 718 | 187 |
| 11 | off | 748 | 719 | 189 |
| 12 | on | 751 | 723 | 186 |
| 13 | off | 749 | 720 | 189 |

The default-branch reads add no time that these rows can show. The rows do
not time a `git merge-base` itself, because no warm Stop follows a commit.
The baseline binary was not run with `KLIN_PERF_ORIGIN=on`, because a remote
ref may change how the 0.x Stop chooses its base.

## A `{check}`-placed capability at the Stop

`sarif` is the one `{check}`-placed capability (section 9.1).
`KLIN_PERF_SARIF=on` adds a `sarif` entry named `scanner` to the fixture's
`klin.json`, whose `run` command writes a marker file. On every timed Stop the
row asserts that the marker file does not exist and that the journal line has
no `scanner` gate row. A Stop that started the command or reported the entry
fails the row. The Stop still reads the entry in `klin.json` when it plans
the gates, and the engine drops it by placement before any gate runs. The
assertions cover processes and gate rows. They do not detect a file read,
such as a check that `scanner.sarif` exists.

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
reported the entry. The assertions are the evidence. The timing shows nothing
either way: runs 7 and 8 differ by 1 ms, run 9 is 47 ms faster than run 8,
and the machine drifted by about 70 ms between run 1 and run 8.

## `pre_tool`

The guard row of the default rows sends 1,000 `pre_tool` events to the
2,000-file fixture, 5 times. Each event is one process, timed from the test.
The row now prints the 50th and 99th percentiles and the maximum of the 5,000
single events.

```sh
cargo test --release --test performance -- --ignored perf --nocapture
```

Section 14.4 sets the limit: `pre_tool` within 50 ms.

| Run | `per_event_ms` | p50 (ms) | p99 (ms) | Max (ms) | Machine |
| --- | --- | --- | --- | --- | --- |
| A | 6.41 | 7.83 | 16.80 | 24.60 | quiet |
| B | 11.68 | 9.59 | 19.85 | 67.26 | load average 16 |

`per_event_ms` is the median of the 5 iteration totals divided by 1,000. The
percentiles pool the 5,000 single events. On the quiet machine, all 5,000 events completed within 50 ms. Run B ran while
other processes loaded the machine: its 10,000-file cold survey took 30,709 ms,
against 15,302 ms on a quieter run. The p99 of run B stays within 50 ms, and
its maximum does not. The time includes the process start, which is outside
klin.

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
  binary without the agent ingress, as the 300k baseline rows need. The
  guard row still sends `klin __agent event`, so the default rows do not run
  with such a binary.
- `KLIN_PERF_SARIF=on`, `KLIN_PERF_ORIGIN=on` and the guard percentiles are
  new. Each option applies to the source-dense rows only.
