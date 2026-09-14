# The structural hot path after one shared extraction (#183)

#176 left one extraction per tree. This note records what a profile of the
path after it showed, what changed, and what stayed as it was because the
profile showed no material cost. The findings, coverage, exit codes and
report text of every structural gate are unchanged.

## Profile

The profile is macOS `sample` at 1 ms over a release build with line tables,
on copies of the dense 1M and 300k fixtures of spec 13. Counts are samples of
the 1M warm hook at `2f719b9`. Sampling makes a run slower, so the counts show
shares and not wall-clock time.

| Cost | Samples | Verdict |
| --- | ---: | --- |
| `dead-symbols` in total | 20,156 | |
| Extraction: parse, query, reference walk, test walk | 11,645 | material |
| Of that, the parse (`ts_parser_parse`) | 4,153 | not changed |
| Of that, parser construction and grammar setup | about 7 | not material |
| `finding_with_lost_reference`: a scan of every base state for each dead finding | 6,802 | material |
| `SourceIndex::of`, two trees | 928 | small |
| `reachability::states`: one lookup per reference, each with a fresh `String` and a path set insert | 9,102 | material |
| Line vectors | under 20 | not material |

## What changed

- `SourceIndex` holds each name's declaration and reference sites in one map
  per logical language, keyed by the name. A lookup borrows the `&str` it is
  given, and the index copies a name once, the first time it meets it. Each
  site list is built in file and line order. A lookup reads one entry, so the
  map's order reaches no consumer. `SourceIndex::file` finds a file's facts by
  path in the sorted file list.
- `dead-symbols` finds the base state of a dead finding by halving the sorted
  base states, and it finds the file's language through `SourceIndex::file`.
  Both were a linear scan for each dead finding.
- `reachability` decides whether a member is reached from the member's own
  eligible declarations, by asking whether another file references one of
  their names. It used to look up every reference of the tree and collect
  every declaring file in a set, which reaches the same files.
- `syntax::walk` moves one cursor over the whole tree, where it used to make a
  cursor for each node. Every reader that walks a tree gets the change.

## What stayed

- The parser is still built for each file. Its construction was about 7 of
  the 4,153 parse samples.
- The reference walk and the test walk stay two walks. The test walk was
  about 1,000 of the 9,471 extraction samples after the cursor change.
- The derived reachability families keep their candidates and cohorts. In
  the cold 1M run, the derivation spent about 5,700 samples reading blobs from
  `git cat-file` and about 6,900 extracting them. Candidates and cohorts were
  under 30 samples, and the index about 200.
- klin adds no parallelism, no persistent cache and no pool.

## Map choice

The new algorithm was built twice, once with `HashMap` and once with
`BTreeMap`. The two binaries ran alternately, three times each, over the 1M
strict run of the two structural gates:

| Map | `dead-symbols` less extraction | `reachability` |
| --- | ---: | ---: |
| `HashMap` | 798 to 816 ms | 240 to 242 ms |
| `BTreeMap` | 1,034 to 1,077 ms | 506 to 512 ms |

## Same output

On both dense fixtures, the binary at `2f719b9` and the new binary printed the
same bytes for `gate --strict --json` with the timings removed, `gate
--strict`, `dead-symbols --report`, `reachability`, `dead-symbols --only` and
the warm hook. At 1M the `dead-symbols --report` text is 1,300,664 bytes. The
journal lines differ only in their timings. The fixtures of #52 and #51 are
the CLI tests of `dead-symbols`, `reachability` and the structural gates run
together, and they pass unchanged. A new CLI test pins the order at the public
boundary: references lost from two files name the first file in path order.

## Measurements

Measured 2026-09-14 on the 0.1.1 baseline machine (MacBook Pro 18,3, Apple M1
Pro, macOS 26.6.2), release build, median of five iterations of spec 13's
dense rows. The before columns are `2f719b9`, run through `KLIN_BIN` in the
same session as the after columns.

| Row  | Warm hook before | Warm hook after | Cold survey before | Cold survey after | Strict before | Strict after |
| ---- | ---------------: | --------------: | -----------------: | ----------------: | ------------: | -----------: |
| 300k |        11,049 ms |        9,348 ms |          27,879 ms |         24,584 ms |     14,533 ms |    12,607 ms |
| 1M   |        44,707 ms |       22,838 ms |          69,422 ms |         50,009 ms |     53,923 ms |    31,884 ms |

Per-gate medians, before → after. `dead-symbols` extracts every file, so its
time less its `facts.ms` is its index and its own judgment. `reachability`
extracts nothing in these rows, and in the cold rows its time also covers the
derived families.

| Row / mode | Complexity | Dead symbols | Dead-symbol extraction | Dead symbols less extraction | Reachability |
| --- | ---: | ---: | ---: | ---: | ---: |
| 300k warm hook | 1,492 → 1,559 ms | 5,690 → 4,663 ms | 4,098 → 3,600 ms | 1,592 → 1,063 ms | 835 → 103 ms |
| 300k cold survey | 9,064 → 8,671 ms | 4,954 → 4,036 ms | 4,188 → 3,809 ms | 766 → 227 ms | 7,381 → 6,271 ms |
| 300k strict | 3,070 → 3,108 ms | 4,947 → 3,877 ms | 4,194 → 3,638 ms | 753 → 239 ms | 820 → 106 ms |
| 1M warm hook | 4,495 → 4,342 ms | 23,320 → 12,171 ms | 12,965 → 10,670 ms | 10,355 → 1,501 ms | 10,083 → 245 ms |
| 1M cold survey | 17,366 → 17,169 ms | 20,576 → 11,492 ms | 12,168 → 10,748 ms | 8,408 → 744 ms | 19,386 → 9,915 ms |
| 1M strict | 9,237 → 8,731 ms | 21,072 → 11,295 ms | 12,387 → 10,577 ms | 8,685 → 718 ms | 10,005 → 243 ms |

What the rows show:

- The 1M warm hook is 49% faster and the 1M strict run is 41% faster. At
  300k the warm hook is 15% faster and strict is 13% faster.
- At 1M, `dead-symbols` less its extraction went from 10,355 ms to 1,501 ms
  in the warm hook, and `reachability` went from 10,083 ms to 245 ms.
- Extraction is 10,670 ms of the 12,171 ms that `dead-symbols` takes in the
  1M warm hook, and most of it is the parse and the query. Reading files in
  parallel would be a separate decision.
- In the cold rows `reachability` still derives its families from the
  derivation commit. Its cold time less its warm time is about 9.7 s at 1M.

Peak resident memory, from single runs of each binary on a copy of the 1M
fixture, as `/usr/bin/time -l` reports it:

| Run | Before | After |
| --- | ---: | ---: |
| Warm hook | 1,033 MB | 955 MB |
| Strict | 1,134 MB | 1,067 MB |

Budget: SPEC 13's budgets are unchanged, and the large-repository budget stays
with #182.
