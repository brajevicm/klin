# Performance baseline at 748d01fc (2026-10-02)

This document records the baseline that vNext work (#358) compares against.
All values come from one run on one machine.

- Commit: `748d01fc`, klin 0.4.1, release profile.
- Machine: macOS, aarch64 (Apple Silicon), developer laptop.
- The perf rows ran after the test suites, one after the other.

## Test suites

| Suite | Tests | Time |
|---|---|---|
| Unit tests (`cargo test --release --bins`) | 75 passed | 1.75 s in the test binary, 1.84 s wall |
| CLI tests (`tests/*.rs`) | 1390 passed, 1 ignored | 93.3 s summed over the binaries |
| All tests (`cargo test --release --tests`) | 1465 passed, 1 ignored | 110.2 s wall, test binaries already built |

## 1M warm Stop rows

Fixture: `source-dense-1m`, 10,000 files (5,000 Rust, 5,000 TypeScript, 50
TSX), 1,033,827 LOC, 292,507 declarations, digest `4606c5ecf1f80e3b`,
`complexity_scope=whole`, `config=build-off`. Each value is the median of 5
iterations. The times exclude the project build command.

Commands:

```sh
KLIN_PERF_ROW=structural_1m KLIN_PERF_CASE=warm20  cargo test --release --test performance -- --ignored perf --nocapture
KLIN_PERF_ROW=structural_1m KLIN_PERF_CASE=warm100 cargo test --release --test performance -- --ignored perf --nocapture
```

### Time and memory

| Measure | 20 changed | 100 changed |
|---|---|---|
| Hook median (ms) | 1218 | 1567 |
| `stop_total_ms` | 1176 | 1526 |
| Peak RSS (KB) | 324,864 | 331,808 |
| Structural cache | 1 file, 37,301,283 bytes | 1 file, 37,301,283 bytes |

### Gate time (ms)

| Gate | 20 changed | 100 changed |
|---|---|---|
| dead-symbols | 486 | 623 |
| reachability | 285 | 281 |
| complexity | 27 | 92 |
| escapes | 33 | 91 |
| stubs | 20 | 71 |
| layering | 62 | 63 |
| public-api | 35 | 37 |
| doc-citations | 27 | 27 |
| inventory | 20 | 19 |
| lockfile | 7 | 7 |

### Structural reads and parses

| Counter | 20 changed | 100 changed |
|---|---|---|
| complexity reads / parses | 40 / 40 | 200 / 200 |
| escapes reads / parses | 40 / 20 | 200 / 100 |
| stubs reads / parses | 40 / 40 | 200 / 200 |
| dead-symbols facts reads / parses (extracted) | 40 / 40 | 200 / 200 |
| dead-symbols facts from cache | 9980 | 9900 |
| dead-symbols facts cache read (ms) | 104 | 102 |
| dead-symbols facts (ms) | 37 | 165 |
| layering, public-api, reachability facts shared | 20000 each, 0 reads | 20000 each, 0 reads |

### File listings and base layout

| Counter | 20 changed | 100 changed |
|---|---|---|
| `names_base_ms` (dead-symbols) | 188 | 191 |
| `layout_worktree_add_ms` | 58 | 64 |
| `layout_cache_name_ms` | 16 | 16 |
| `layout_written` (files) | 27 | 107 |
| `layout_walk_ms`, `changes_ms`, `renames_ms`, `ignored_ms` | 0 | 0 |
| `stop_base_prune_ms` / `stop_base_remove_ms` | 5 / 8 | 5 / 12 |
| `stop_lock_ms` | 5 | 5 |
| git `ls-files --stage` (experiment) | 8 | 8 |
| git `worktree add` whole / no checkout (experiment) | 884 / 9 | 898 / 9 |

### Graph and name-index work

| Counter | 20 changed | 100 changed |
|---|---|---|
| layering graph: modules / edges / ms | 10010 / 9906 / 28 | 10010 / 9906 / 28 |
| public-api graph: modules / edges / ms | 10010 / 9906 / 17 | 10010 / 9906 / 18 |
| dead-symbols name index before / after (ms) | 73 / 76 | 75 / 75 |
| reachability name index before / after (ms) | 75 / 76 | 74 / 74 |
| reachability query before / after (ms) | 28 / 28 | 28 / 27 |
| name index: files / declarations / references | 10000 / 292557 / 1040114 | same |

The full output line of each row holds all other counters. Run the commands
above to print them again.
