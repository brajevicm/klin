# CI topology after #470

This record holds the final CI topology, the policy for `main`, the cache
design, the release verification and the measurements. It follows
`docs/ci-cost-2026-09-30.md` (#368) and `docs/ci-test-runtime-2026-09-30.md`
(#367). GitHub bills each job in whole minutes, rounded up.

## Topology

| Workflow | Trigger | Work |
| --- | --- | --- |
| `quality` | each pull request push | fmt, clippy, nextest, debug build, `klin gate --strict` |
| `rust cache` | a push to `main` that changes a cache key input, weekly, manual dispatch | clippy and the test build, to save the dependency cache |
| `release plan` | a pull request that touches a release input | `dist plan` |
| `Release` | a pushed version tag | the dist builds, with the `quality` work on the exact tag in the x86_64 Linux build, then host and announce |
| `benchmark` | the `benchmark` label on a pull request | release builds of base and head, perf rows |
| `prepare-release`, `host compatibility` | manual dispatch | release PR preparation, host canaries |
| `publish-release` | a generated release PR merges to `main` | validate the release-only diff/checks and push the immutable tag |

## The policy for `main`

On 2026-09-30, `main` has no branch protection and no rulesets. The
repository allows merge commits, squash and rebase, and a person can push to
`main` directly. No check is required, and no merge must be up to date with
`main`.

On 2026-09-30, the repository owner decided to remove the `quality` run on
each push to `main` before the protection exists. The owner plans to protect
`main` soon. Until then, nothing checks the exact tree that `main` holds
after a merge. Two pull requests that each pass `quality` can merge into a
`main` that fails, and the next pull request run shows the failure.

The protection that makes a push run on `main` redundant has all of these:

- `main` accepts changes only through pull requests, with no bypass.
- `quality / gates` is a required check.
- A merge must be up to date with `main`, or a merge queue tests the merged
  tree.

The protection is a decision for the repository owner, and #369 does not
make it.

## Rust cache

`quality` uses `Swatinem/rust-cache`. It caches `~/.cargo` and the
dependency artifacts under `target`, and it drops the artifacts of the klin
crate itself before it saves. Both workflows set `shared-key: gates`, so they
compute the same key.

- `quality` restores the cache and saves nothing (`save-if: false`). GitHub
  scopes a cache that a pull request saves to that pull request's merge ref,
  so a sibling pull request cannot restore it. A pull request can restore
  the cache of `main`.
- `rust cache` saves the cache on `main`. It runs clippy and the test build,
  which compile the same dependencies as `quality`, and it runs no tests.
- GitHub scopes each cache to the ref that saved it, so `main` never reads a
  cache from a pull request.
- The key holds the rustc version and a hash of `Cargo.toml`, `Cargo.lock`
  and `rust-toolchain.toml`. With an exact hit, the action does not save
  again. `rust cache` therefore runs on `main` only when one of those files,
  or its own workflow, changes.
- GitHub evicts a cache that no run reads for 7 days. The weekly run reads
  the cache, and it saves a new one when GitHub evicted the old one.

The `Release` workflow uses no cache. It runs a few times each month, on a
different runner image, and a tag run is the last check before a publish.

## Release verification

`dist-workspace.toml` sets `github-build-setup` to `.github/verify-tag.yml`.
dist puts those steps into each `build-local-artifacts` job of `release.yml`,
and each step runs only in the `x86_64-unknown-linux-gnu` job. The steps
check out the tag with its full history and run fmt, clippy, nextest, the
debug build and `klin gate --strict`. The dist build in the same job is the
release-profile build.

A failed step fails `build-local-artifacts`. `host` runs only when that job
succeeded or was skipped, and `announce` needs `host`, so nothing publishes.

dist also offers `plan-jobs`, a custom job that the build jobs need. A failed
plan job skips the build jobs, and `host` accepts skipped build jobs. That
path would publish a release with no binaries, so klin does not use it.

A release tag now names the exact commit produced by merging the generated release PR, so that commit is already on `origin/main`. The PR's required `quality / gates` run is therefore the authoritative diff gate. The tag workflow still re-runs fmt, clippy, nextest, the debug build and `klin gate --strict` on the exact commit before cargo-dist publishes artifacts.

The release jobs run on `ubuntu-22.04`, and `quality` runs on
`ubuntu-latest`. The tests have not run on `ubuntu-22.04` yet, so the first
tag may fail for a reason in the runner image.

## Action pins

Every workflow names each Action by a commit SHA. The hand-written files
give the release tag in a comment. `github-action-commits` in
`dist-workspace.toml` pins the Actions of the generated `release.yml`, and
the release tags are comments there, because dist writes bare SHAs. `dist
plan` still finds no drift. nextest, cargo-release and dist keep their pinned
versions. No bot updates the pins. A person moves a pin to a newer release
tag by hand.

## Measurements

### Before, no cache

Pushes to `main` and pull request runs with the #367 tests, 2026-09-30:

| Run | Event | Job | nextest step | clippy | gate |
| --- | --- | ---: | ---: | ---: | ---: |
| 36761062965 | pull request | 271 s | 164 s | 49 s | 41 s |
| 36762476528 | push to `main` | 270 s | 165 s | 47 s | 42 s |

Each run bills 5 minutes. In run 36762476528, clippy finished in 47 s, the
test build in 66 s and the test execution in 98.4 s. The #368 record has the
older baselines: 328 s before #368 and 285 s after it.

The last release, `v0.4.0` (run 36600255871), took these job times:

| Job | Runner | Time | Billed |
| --- | --- | ---: | ---: |
| `plan` | ubuntu-22.04 | 23 s | 1 min |
| x86_64 Linux build | ubuntu-22.04 | 160 s | 3 min |
| aarch64 Linux build | ubuntu-22.04-arm | 124 s | 3 min |
| aarch64 macOS build | macos-14 | 94 s | 2 min |
| x86_64 macOS build | macos-15-intel | 252 s | 5 min |
| `build-global-artifacts`, `host`, `announce` | ubuntu-22.04 | 67 s | 3 min |

The 7 macOS minutes cost more than the Linux minutes. GitHub counts a macOS
minute as about 10 included minutes on a private repository.

### After, to measure

These values need CI runs of this topology, and none has run yet:

- The median `quality` job on pull requests, over the first 20 runs.
- Cache hits and misses on pull requests, from the `rust-cache` step output.
- The job time of `rust cache` with a cold cache and with an exact hit.
- The added time of the verification steps in the x86_64 Linux release job.

The estimate: clippy and the test build spend most of their 113 s on
dependencies. If the cache removes about 60 s of that, a `quality` run takes
about 210 s and bills 4 minutes. The verification steps add about 4 to 5
minutes to the x86_64 Linux release job, because the release job has no
cache.

### Monthly usage at the #368 cadence

The cadence from 2026-09-16 to 2026-09-30 was 254 pull request pushes, at
most 115 pushes to `main`, about 15 `release plan` runs and 3 releases in 15
days. A month holds about twice that. A newer push cancels an older pull
request run, so the real counts may be lower. `rust cache` runs about 4 times
a month on its schedule, and once for each change to a cache key input. The
estimate is 8 runs of 3 minutes.

| | `quality` bills 5 min | `quality` bills 4 min |
| --- | ---: | ---: |
| Pull requests, 508 runs | 2,540 | 2,032 |
| `rust cache`, about 8 runs | 24 | 24 |
| `release plan`, 30 runs | 30 | 30 |
| 6 releases, with the verification | 90 | 90 |
| Linux minutes in a month | 2,684 | 2,176 |

The 6 releases also use about 42 macOS minutes. The GitHub Free plan
includes 2,000 minutes each month, and GitHub Pro includes 3,000. At the
cadence of the second half of September, the month fits GitHub Pro with
about 300 to 800 minutes of headroom, and it does not fit GitHub Free. The
push run on `main` that this topology removed cost about 920 to 1,150
minutes a month.
