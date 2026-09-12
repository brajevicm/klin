# Release notes

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
