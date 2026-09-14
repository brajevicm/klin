# Structural gates share one extraction per tree (#176)

`dead-symbols` and `reachability` each used to read, parse and extract every
structural file of both trees, and under `--changed` each of them checked out
the whole base again. A run now extracts each file of a tree once. The first
structural gate that reads a file pays for the extraction, and a later gate
reuses the facts. Each gate still selects its own files under its own scope
and builds its own index, so its findings and coverage match a run of that
gate alone. Under `--changed` the two gates share one checkout of the whole
base. ADR 0038 records the ownership.

Each gate row in `--json` and in the journal has a new `facts` field,
`{extracted, shared, ms}`, for the gates that read structural facts (spec
11.2). The dense performance rows print the extraction time as
`dead-symbols_facts_ms` and `reachability_facts_ms`, and the rest of a gate's
`ms` is its index and its own algorithm (spec 13).

## Measurements

Measured 2026-09-14 on the 0.1.1 baseline machine (MacBook Pro 18,3, Apple M1
Pro, macOS 26.6.2), release build, median of five iterations. The before
columns are `ea10c0e`, run through `KLIN_BIN` in the same session as the after
columns.

| Row  | Warm hook before | Warm hook after | Cold survey before | Cold survey after | Strict before | Strict after |
| ---- | ---------------: | --------------: | -----------------: | ----------------: | ------------: | -----------: |
| 300k |        16,249 ms |       10,561 ms |          28,694 ms |         24,732 ms |     17,735 ms |    13,784 ms |
| 1M   |        53,076 ms |       40,068 ms |          79,675 ms |         67,433 ms |     60,273 ms |    49,348 ms |

Per-gate medians, before → after. Extraction is the `facts.ms` of the after
binary.

| Row / mode | Complexity | Dead symbols | Dead-symbol extraction | Reachability | Reachability extraction |
| --- | ---: | ---: | ---: | ---: | ---: |
| 300k warm hook | 1,508 → 1,449 ms | 6,147 → 5,430 ms | 3,886 ms | 6,212 → 817 ms | 0 ms |
| 300k cold survey | 8,102 → 8,032 ms | 4,704 → 4,622 ms | 3,906 ms | 10,430 → 6,574 ms | 0 ms |
| 300k strict | 2,953 → 2,923 ms | 4,665 → 4,643 ms | 3,929 ms | 4,641 → 806 ms | 0 ms |
| 1M warm hook | 4,304 → 4,179 ms | 21,258 → 20,227 ms | 11,449 ms | 21,883 → 9,108 ms | 0 ms |
| 1M cold survey | 16,481 → 16,247 ms | 19,701 → 19,760 ms | 11,528 ms | 30,974 → 19,192 ms | 0 ms |
| 1M strict | 8,481 → 8,339 ms | 19,961 → 19,617 ms | 11,452 ms | 20,030 → 9,191 ms | 0 ms |

What the rows show:

- The 300k warm hook moved by more than a third (35% faster). The 1M warm hook
  is 25% faster, and strict is 22% faster at 300k and 18% faster at 1M.
- `reachability` extracts nothing in the warm and strict rows. In the cold
  rows its time still includes the derivation commit, which it reads on its
  own to derive its families.
- At 1M, extraction is 11,449 ms of the 20,227 ms `dead-symbols` takes in the
  warm hook. The other 8.8 s, and the 9.1 s `reachability` takes with no
  extraction, are index construction and each gate's own algorithm. #183
  profiles that path.
- Peak resident memory, from single runs on a copy of the 1M fixture: strict
  went from 1,113 MB to 1,125 MB, and the warm hook went from 1,022 MB to
  913 MB, because the hook now checks the base out once instead of twice.
- `complexity`, `stubs`, `escapes` and conventions code rules still parse on
  their own, because they walk a Tree-sitter tree that no extracted fact
  replaces. Keeping the parse of every file in both trees raised the peak
  memory of a strict `complexity` run at 1M from 63 MB to 1,842 MB, so klin
  does not keep them. In a single strict run at 1M after the change, `escapes`
  took 4,211 ms and `stubs` 7,025 ms.

Budget: SPEC 13's budgets are unchanged, and the large-repository budget stays
with #182.
