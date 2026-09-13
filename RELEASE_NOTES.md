# Release notes

## Unreleased

### Cold survey reads the derivation commit through one git process

The complexity sample used to read each sampled file with its own `git show`.
It now reads every sampled file through one `git cat-file --batch` process.
Measured 2026-09-13 on the 0.1.1 baseline machine (MacBook Pro 18,3, Apple
M1 Pro, macOS 26.6.2), release build, median of five iterations, before and
after the change in the same session:

| Row | Warm hook before | Warm hook after | Cold survey before | Cold survey after | Strict before | Strict after |
| --- | ---------------: | --------------: | -----------------: | ----------------: | ------------: | ------------: |
| 2k  |         1,703 ms |        1,602 ms |          15,363 ms |          2,034 ms |      1,745 ms |      1,716 ms |
| 10k |         5,451 ms |        5,406 ms |          74,465 ms |         10,755 ms |      6,787 ms |      6,760 ms |

The guard row was 9,025 ms before and 8,992 ms after for 1,000 events. The
warm rows sit above the 0.1.1 table because the machine carried other load
during this session; the before and after columns share that load.

## 0.1.1

### Performance baseline

Recorded 2026-09-12 on a MacBook Pro 18,3 with an Apple M1 Pro (8 cores),
macOS 26.6.2, Darwin 25.6.0 arm64, using klin 0.1.1. Each timing is the
median of five iterations; project builds are excluded from hook timings.

| Row                                 | Warm hook | Cold survey |   Strict |
| ----------------------------------- | --------: | ----------: | -------: |
| 2k (1,000 Rust + 1,000 TypeScript)  |    934 ms |   14,250 ms | 1,406 ms |
| 10k (5,000 Rust + 5,000 TypeScript) |  2,393 ms |   72,828 ms | 5,835 ms |

The guard row processed 1,000 deterministic events per iteration: 8,974 ms
median total, or 8.974 ms/event. Both rows changed 10 Rust and 10 TypeScript
files; the TypeScript counts include 10 and 50 `.tsx` files respectively.
