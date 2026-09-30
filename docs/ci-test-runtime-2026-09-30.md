# CI test-suite runtime, #367

This record holds the baseline, the changes, the investigated items that were
rejected with the measurement for each, and the items that still need CI runs.

## Runner

The `gates` job of the `quality` workflow runs on `ubuntu-latest`. The repository is private, so the
runner has 2 vCPUs. `cargo nextest run --locked` sets no thread count, so
nextest runs 2 tests at a time. The summed test time divided by 2 is close to
the execution time (236.5 s / 2 against 119.5 s in attempt 4), so the 2 slots
stay full. One test-second removed saves about 0.5 s of wall time.

## Baseline

The sample is attempts 2, 3 and 4 of `quality` run 36752827135, commit
`52c87800`, 2026-09-30. `main` at `2ba7cf63` holds the same `src/`, `tests/`,
`Cargo.lock` and `quality.yml`.

| Step | Attempt 2 | Attempt 3 | Attempt 4 | Median |
| --- | ---: | ---: | ---: | ---: |
| `gates` job | 302 s | 261 s | 285 s | 285 s |
| `cargo nextest run --locked` step | 195 s | 162 s | 182 s | 182 s |
| compile and list (`Finished` line) | 68 s | 60 s | 62 s | 62 s |
| test execution (`Summary` line) | 125.9 s | 100.9 s | 119.5 s | 119.5 s |
| summed test time | 249.2 s | 199.9 s | 236.5 s | 236.5 s |
| `cargo clippy` | 48 s | 46 s | 42 s | 46 s |
| `./target/debug/klin gate --strict` | 43 s | 30 s | 43 s | 43 s |

1411 tests ran across 46 binaries.

The slowest tests in attempt 4:

| Test | Time |
| --- | ---: |
| `sarif::the_build_draws_on_the_limit_the_sarif_commands_share` | 4.06 s |
| `radius::the_sample_stops_at_two_hundred_commits` | 3.02 s |
| `copies::the_same_stop_again_after_the_copies_settled_is_gated` | 2.67 s |
| `sarif::the_commands_of_one_run_share_twice_the_limit` | 2.06 s |
| `structural_cache::a_sparse_checkout_the_light_layout_does_not_read_checks_the_base_out_whole` | 1.70 s |
| `radius::the_percentile_is_the_nearest_rank` | 1.47 s |
| `journal::a_stop_that_wrote_no_verdict_still_writes_a_line_that_says_why` | 1.37 s |
| `window::a_stop_that_cannot_take_the_lock_writes_no_verdict_and_says_so` | 1.37 s |
| `host::a_cursor_stop_without_the_state_lock_tells_nothing_and_writes_no_stamp` | 1.35 s |
| `state::a_stop_that_cannot_take_the_state_directory_says_so_and_writes_no_verdict` | 1.31 s |

The slowest binaries by summed test time in attempt 4 are `gate` (18.6 s),
`survey` (18.0 s), `radius` (15.0 s), `public_api` (14.6 s) and `build`
(12.4 s).

## Changes

- `copies::the_same_stop_again_after_the_copies_settled_is_gated` no longer
  sleeps 2.5 s. It sets the modification time of each claim file 3 s into the
  past, and production `settled_within` reads that time as before. With an age
  of 0 s, the test fails (1 journal line, not 2).
- `harness::history` and `harness::history_from` write their commits through
  one `git fast-import` stream and then check the tree out. Before, each commit
  cost one `git add` and one `git commit`. `Tree::commit()` is unchanged. The
  radius and init tests that use these histories see the same commits, paths
  and file contents. All commits carry one fixed committer time, no message
  ends in a newline, and no test reads either.
- `Tree::new()` runs `git init` and one `git fast-import`: 2 processes where
  it ran 4. `tree.base()` writes its two commits through one
  `git fast-import` when the tree is on `work` and `work` holds the files of
  `main`. It then runs 4 processes where it ran 6. In that state, the old
  checkout of `main` changed no file. In every other state, `base()` runs
  the old commands. The branches, parents, messages and trees are the same as
  before, and the commits are dated now, as before. fast-import writes no
  reflog, and no klin code or test reads one.
- The sparse-checkout test keeps 6 spellings: `TRUE`, `on`, `1`, `off`,
  `no` and `0`. It dropped `true`, `yes`, `false` and `FALSE`. klin reads the
  value through `git config --type=bool`, so git itself normalizes case and
  words. The kept set still has an uppercase word, alternate words and
  numbers.

No test was removed, and no production code changed.

## After, local

These numbers come from a MacBook with 8 cores, at `-j2`, 3 runs each, over
five binaries: `radius`, `init`, `copies` and `structural_cache`, which the
changes touch, and `journal`, which they do not. Before is `2ba7cf63`. After
is `2af51163`, whose sparse-checkout test held `true` and `false` where the
final branch holds `TRUE` and `off`.

| | Summed test time | Wall time |
| --- | ---: | ---: |
| Before | 78.3, 78.7, 78.9 s | 39.8, 40.0, 40.1 s |
| After | 47.9, 48.2, 48.0 s | 24.4, 24.5, 24.4 s |

The cheaper `Tree::new()` and `base()` were measured separately over five
light binaries (`config`, `doc_size`, `lockfile`, `escapes`, `stats`, 216
tests), at `-j2`, 3 runs each. These tests mostly build a tree, run klin once
and assert, so the setup is a large part of each test.

| | Summed test time | Wall time |
| --- | ---: | ---: |
| Before, `d4f07584` | 62.1, 62.4, 62.3 s | 31.3, 31.4, 31.4 s |
| After | 48.3, 51.4, 49.0 s | 24.3, 25.9, 24.7 s |

That is about 62 ms less for each test locally. Most of the 1336 integration
tests start from `Tree::new()`. If 1200 of them do, at the CI-to-local ratio
of about 0.45, the change saves about 33 s of summed CI time, about 17 s of
wall time. This is an
estimate from a sample of light tests. Heavier tests spend a smaller share on
setup, so the real saving may be lower.

Single tests, locally:

| Test | Before | After |
| --- | ---: | ---: |
| `the_sample_stops_at_two_hundred_commits` | 6.71 s | 0.35 s |
| `the_same_stop_again_after_the_copies_settled_is_gated` | 3.86 s | 1.34 s |
| sparse-checkout test | 4.99 s | 3.32 s |

On CI, the same five binaries sum to 39.5, 29.4 and 37.3 s, about 0.45 of the
local time. The local saving of 30.6 s therefore scales to about 14 s of summed
CI test time, which is about 7 s of wall time. This is an estimate. It is not
a CI measurement. The job then moves from about 285 s to about 278 s, which
still bills 5 minutes.

## After, CI

Not measured yet. A `quality` run of this branch gives the numbers for the
baseline table.

## Rejected

- **Lock-budget waits.** Seven tests each spend the real 1 s stop lock budget:
  2 in `state.rs`, 3 in `window.rs`, 1 in `journal.rs`, 1 in `host.rs`. Each
  pins a different branch after a lost lock: the verdict, the build block, the
  turn file restore, the abandoned stamp, the journal line and the Cursor
  followup. `src/gate.rs` passes one `lost` flag to these branches, but the
  only seam is the command line, so a shorter wait needs a test-only budget
  override. #367 forbids that. The seven tests hold about 9 s of summed CI
  time, about 4.5 s of wall time.
- **#341 timeout tests.** The nine build and SARIF limit, deadline,
  descendant and signal tests hold 9.3 s of summed CI time in attempt 4. Only
  `the_build_draws_on_the_limit_the_sarif_commands_share` (4.06 s) can get
  shorter: a `sleep 0.5` build with a 1 s limit takes about 2 s. That change
  also cuts the margin of each timing assertion from 1 s to 0.5 s on a shared
  2-vCPU runner. The saving is about 1 s of wall time, so the risk of a flake
  is not worth it. The three signal tests take under 0.2 s each on CI. The
  other tests already use the 1 s limit, which is the shortest that
  `KLIN_COMMAND_LIMIT` accepts.
## Not benchmarked, to save CI minutes

These comparisons need repeated runs on the CI runner class, because local
runs on 8 cores do not predict a 2-vCPU runner. The Actions budget is tight,
so this PR runs neither. Each item names the reason and the method, for a
later run.

- **nextest concurrency.** The 2 slots stay full with CPU-bound tests, so a
  higher `-j` can overlap only the tests that sleep or wait on a lock. Those
  hold about 20 s of summed CI time, so the best case is about 10 s of 119.5 s,
  below the 10 % bar of #367. Method: build once, then run
  `cargo nextest run --locked -j N` for N = 2 and 4, twice each, in one job.
- **nextest against libtest.** Rejected without a benchmark. libtest runs the
  tests of one binary as threads of one process, and the signal,
  process-group and lock tests depend on one process for each test. #367 also
  says not to switch runners for a small gain. Method: after one build,
  compare `cargo nextest run --locked` with
  `cargo test --locked -- --test-threads=2`.

## Deferred

- **Consolidation of the integration-test binaries.** Deferred, not done. A
  cold local `cargo test --locked --no-run --timings` took 20.1 s wall and
  130.4 CPU-s. The 125 dependency units took 75.2 CPU-s, the `klin` binary
  8.9 s, its unit tests 6.3 s, and the 45 integration-test targets 40.0 CPU-s
  together (0.5 to 1.6 s each). Cargo reports no link section for these
  units, so the link share of the 40.0 s is not known. On 2 vCPUs, the test
  targets may account for up to about 20 s of the 62 s compile. Take the same
  timings on CI before a decision. Consolidation changes the layout of
  `tests/`, so it is a separate ticket.
