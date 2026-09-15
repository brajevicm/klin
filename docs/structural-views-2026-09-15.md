# Dead symbols over one base extraction (#190)

This is the first architecture checkpoint of #187. `dead-symbols` measured
both trees independently, so a warm Stop extracted every structural file
twice even when it changed 20 files. In a changed run that is not strict,
which includes the hook, the working tree now takes the base extraction's
outcome for every file that the run's change set does not name and that the
base lists under the same name. Only the changed files are extracted from the
working tree. Strict runs, whole runs and the check by hand still extract
both trees. SPEC 8.4 and the #190 follow-up of ADR 0038 record the rule and
its known limit.

## Same output

`tests/structural_views.rs` builds ten scenarios through the real binary:

- an edit
- an addition
- a deletion
- a same-extension rename
- an extension-changing rename
- movement into and out of scope
- name ambiguity
- an unsupported language
- unparsed files
- a case-only rename that git does not see

Each scenario asserts literal findings and notes. With `KLIN_DIFF_BIN`
naming another build, each scenario also builds a second copy of the trees
and requires that build to print the same normalized output. The compared
callers are `gate --json`, `gate --strict --json`, `gate --changed --json`,
`dead-symbols --report` and `gate --hook --changed --json`. The
normalization removes only the base commit, `ms` and `facts`. All ten
scenarios passed with `KLIN_DIFF_BIN` set to a release build of `17f3c64`.

The case-only rename scenario came from review. The first version of the rule
asked whether the base copy was a file. On a case-insensitive disk,
`src/caller.rs` found the base's `src/Caller.rs`, so the changed run read the
old reference and passed. The earlier build reports a worsened finding there. The rule now asks the base's
file list for the exact name, and the scenario pins that.

## Measurements

Measured 2026-09-15 on the 0.1.1 baseline machine (MacBook Pro 18,3, Apple M1
Pro, macOS 26.6.2), release build, median of five iterations of spec 13's
dense rows, each row changing 10 Rust and 10 TypeScript files. The before
columns are `17f3c64`, and the after columns are this change. Both ran
through `KLIN_BIN` in the same session. A Stop hook of this session ran klin
on the klin repository while the 300k before row started. That row's warm,
cold and strict medians differ from an undisturbed run of the same binary
earlier in the session by 2%, 5% and 8%, in both directions.

| Row  | Warm hook before | Warm hook after | Cold survey before | Cold survey after | Strict before | Strict after |
| ---- | ---------------: | --------------: | -----------------: | ----------------: | ------------: | -----------: |
| 300k |         5,818 ms |        5,546 ms |          22,530 ms |         22,420 ms |     11,955 ms |    11,901 ms |
| 1M   |        13,183 ms |       12,763 ms |          48,289 ms |         48,114 ms |     30,967 ms |    30,825 ms |

The structural gates in the warm hook, before → after:

| Row | Dead symbols | Dead-symbol extraction | Files extracted / shared | Reachability | Reachability extraction | Files extracted / shared |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 300k | 4,757 → 2,807 ms | 3,599 → 1,743 ms | 20,000 / 0 → 10,020 / 9,980 | 94 → 1,886 ms | 0 → 1,786 ms | 0 / 20,000 → 9,980 / 10,020 |
| 1M | 12,055 → 6,511 ms | 10,629 → 5,096 ms | 20,000 / 0 → 10,020 / 9,980 | 216 → 5,344 ms | 0 → 5,120 ms | 0 / 20,000 → 9,980 / 10,020 |

The cold and strict rows extract both trees, as before. Their structural
counters are unchanged, and their gate medians are within 5%.

Peak resident memory, from one run of each binary after the timed rows, as
`/usr/bin/time -l` reports it:

| Run | 300k before | 300k after | 1M before | 1M after |
| --- | ---: | ---: | ---: | ---: |
| Warm hook | 312 MB | 310 MB | 900 MB | 823 MB |
| `gate --changed --gate dead-symbols` | 309 MB | 267 MB | 896 MB | 781 MB |
| Strict | 350 MB | 350 MB | 992 MB | 991 MB |

## What the rows show

- The hypothesis holds for `dead-symbols`. At 1M it extracts 10,020 files in
  place of 20,000, its extraction time falls from 10.6 s to 5.1 s, and the
  gate is 46% faster. A changed `dead-symbols` run alone peaks 115 MB lower
  (13%).
- In the same 1M hook, `reachability` still measures the working tree from
  that tree's own extraction. It now extracts the 9,980 unchanged files that
  `dead-symbols` took from the base, and they cost it 5.1 s. The two gates
  together still extract 20,000 files, so the warm hook is 3% faster at 1M
  and 5% faster at 300k. The 1M hook peaks 77 MB lower (9%). #191 moves
  `reachability` onto the same view, which removes the second extraction.
- The file-local gates of #189 already read only the changed contents, so
  structural extraction is now most of the 1M warm hook.
- The strict and cold rows are unchanged, because strict and whole runs keep
  reading every byte of both trees.

The performance fixture printed `unavailable` for peak RSS on macOS before
this change. `/usr/bin/time -l` writes the number first, and the fixture read
the last word of the line. It now reads the first number on the line, and it
also prints the warm hook and the changed `dead-symbols` run beside strict.

Budget: SPEC 13's budgets are unchanged, and the large-repository budget stays
with #182.

## Reachability over the shared base extraction (#191)

### Same output

The 16 structural-view scenarios passed on 2026-09-15 with `KLIN_DIFF_BIN`
set to a release build of `df546d3`, the commit before `7760bf7`. There were
no normalized output differences. The new scope-movement scenario and the
changed-caller scenario both produced the same literal findings under both
builds.

### Measurements

Measured 2026-09-15 with the release binary of `7760bf7` through `KLIN_BIN`,
five iterations per row. The first timing in each cell is the #190 after
column above; the second is this run.

| Row | Warm hook | Cold survey | Strict |
| --- | ---: | ---: | ---: |
| 300k | 5,546 → 4,480 ms | 22,420 → 23,116 ms | 11,901 → 12,290 ms |
| 1M | 12,763 → 8,225 ms | 48,114 → 49,796 ms | 30,825 → 32,012 ms |

The warm structural counters, #190 after → #191:

| Row | Dead symbols | Reachability |
| --- | ---: | ---: |
| 300k | 2,807 ms, facts 1,743, 10,020 / 9,980 → 3,364 ms, facts 2,020, 10,020 / 9,980 | 1,886 ms, facts 1,786, 9,980 / 10,020 → 99 ms, facts 0, 0 / 20,000 |
| 1M | 6,511 ms, facts 5,096, 10,020 / 9,980 → 7,044 ms, facts 5,403, 10,020 / 9,980 | 5,344 ms, facts 5,120, 9,980 / 10,020 → 219 ms, facts 0, 0 / 20,000 |

The two numbers after each facts value are `extracted / shared`. The warm
reachability run now extracts no files of its own; it shares the structural
facts already extracted by `dead-symbols`. Peak RSS was unavailable in this
run.

## The base's facts kept between runs (#192)

A changed run that is not strict now keeps the base commit's structural
outcomes in the structural cache, one file per commit under
`cache/structural/` in the state directory. The next run over the same commit
reads that file in place of parsing the base again. SPEC 8.4 and the #192
follow-up of ADR 0038 record the rule.

### Same output

`tests/structural_cache.rs` runs the real binary for six cases:

- A second changed run reads the base's outcomes from the cache and prints
  the same verdict, findings, notes and coverage as the first run and as a
  run after the cache is removed. Its `dead-symbols` row reads and parses 2
  files where the first run reads and parses 6.
- An empty, truncated, extended, bit-flipped or foreign cache file reads as
  no cache. The run judges the same, and it writes the same bytes again.
- A cache file of another commit, copied under this commit's name, is not
  read. Trusting it would pass a new dead declaration, and the run fails on
  that declaration.
- A smudge filter added between two runs changes the base's bytes, and the
  run over the old cache judges the same as a run without it.
- After six bases, the cache holds the files of the four newest.
- Repeated red Stops keep the same turn base and the same failure through a
  prompt and a branch switch, and the Stops after the first read the cache.

Every scenario in `tests/structural_views.rs` now runs `gate --changed` a
second time over the cache and requires the same normalized output. Where the
build records `cached`, the second run's `extracted` plus `cached` must equal
the first run's `extracted`, so every base outcome that the change set does
not name came from the cache. All 17 scenarios passed. Unit tests in `src/syntax/structural/cache.rs` cut the file
at every byte, change every byte, forge a body with a matching checksum, and
change the epoch, version, commit and root, and each reads as no cache.

### Measurements

Measured 2026-09-15 on the 0.1.1 baseline machine (MacBook Pro 18,3, Apple M1
Pro, macOS 26.6.2), release build of this change on `8dfda03`, median of five
iterations of spec 13's dense rows, each row changing 10 Rust and 10
TypeScript files. The "no cache" column removes the structural cache before
each Stop, so that Stop extracts the base and writes the cache. The Stop hook
of this session ran klin on the klin repository several times while the rows
ran, so single rows may carry that load.

| Row | Warm hook, cache read | Warm hook, no cache | #191 warm hook | Cold survey | Strict |
| --- | ---: | ---: | ---: | ---: | ---: |
| 300k | 1,941 ms | 3,736 ms | 4,480 ms | 22,558 ms | 11,996 ms |
| 1M | 2,711 ms | 7,939 ms | 8,225 ms | 47,925 ms | 30,782 ms |

The `dead-symbols` row of the warm hook, cache read → no cache:

| Row | Gate | Extracted / cached / shared | Extraction | Cache read | Cache write | Cache file |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 300k | 1,061 → 2,821 ms | 40 / 9,980 / 9,980 → 10,020 / 0 / 9,980 | 11 → 1,752 ms | 27 → 0 ms | 0 → 8 ms | 8,346,561 bytes |
| 1M | 1,567 → 6,788 ms | 40 / 9,980 / 9,980 → 10,020 / 0 / 9,980 | 26 → 5,298 ms | 78 → 0 ms | 0 → 25 ms | 25,793,587 bytes |

`reachability` extracts nothing and reads no cache in either warm row. It
shares the 20,000 outcomes `dead-symbols` took, in 247 ms at 1M.

Peak resident memory, from one run of each after the timed rows, in kB as
`/usr/bin/time -l` reports it:

| Run | 300k | 1M |
| --- | ---: | ---: |
| Warm hook, cache read | 281,168 | 806,304 |
| Warm hook, no cache | 287,136 | 833,648 |
| `gate --changed --gate dead-symbols` | 282,608 | 826,768 |
| Strict | 353,824 | 1,019,936 |

The first 1M row printed its cache size and peak memory after `cache clean`
had removed the cache, so the fixture now writes the cache again with one
untimed Stop before it prints them. The 1M cache size and peak memory above
come from a second 1M row taken with that correction. Its warm hook medians
were 2,625 ms with the cache and 7,783 ms without it, within 3% of the first
row.

### What the rows show

- At 1M, a Stop that reads the cache takes 2,711 ms against 7,939 ms for a
  Stop that extracts the base, 66% less, and below the roughly 5 s milestone
  of #187. At 300k it takes 48% less.
- `dead-symbols` reads and parses the 40 changed file versions and nothing
  else. Its extraction time falls from 5.3 s to 26 ms at 1M.
- Reading the whole cache costs 78 ms at 1M, 3% of the warm hook, so no lazy
  per-file or per-symbol format is added.
- Writing the 25.8 MB cache costs 25 ms at 1M. Only a Stop that extracted an
  outcome the cache lacked writes it.
- Peak memory of a Stop that reads the cache is 2% below a Stop without it at
  300k and 3% below at 1M. The facts a Stop holds are the same either way.
- The cold survey and strict rows are 2% to 4% below the #191 rows, which
  another build took in another session. Strict and whole runs neither read
  nor write the cache. The no-cache warm hook is 4% below #191 at 1M and 17%
  below at 300k, although that Stop also writes the cache. These rows do not
  show the cause of the 300k difference.

### #193 completion

The persistent policy is four newest base-commit cache files. The cache writes
the current file atomically, then evicts older files by modification
time with a path tie-breaker. Six-base CLI coverage proves that an evicted
base falls back to a cold run with the same verdict; missing, damaged,
foreign, incompatible and checkout-stale files have the same fallback
property. The red-turn lifecycle test also revisits the exact stamped base
after a prompt and branch switch.

Measured 2026-09-15 on the same baseline machine, release build of 0.1.1
from implementation commit 9909b9f, with five iterations per row:
Machine: MacBook Pro 18,3, Apple M1 Pro, macOS 26.6.2 / Darwin 25.6.0,
macOS aarch64.

| Source row | Warm, 20 changed | Warm, 100 changed | Warm, no cache | Cold | Strict | Cache bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 300k | 2,220 ms | 2,559 ms | 3,979 ms | 23,854 ms | 12,592 ms | 8,416,407 |
| 1M | 2,860 ms | 3,220 ms | 8,090 ms | 48,177 ms | 30,724 ms | 25,863,432 |

The full output records every gate for every row. The structural counters are
the important fixed-repository scaling result:

| Source row | Changed | Warm, 20: extracted / cached / shared | Changed | Warm, 100: extracted / cached / shared |
| --- | ---: | ---: | ---: | ---: |
| 300k | 20 | 40 / 9,980 / 9,980 | 100 | 200 / 9,900 / 9,900 |
| 1M | 20 | 40 / 9,980 / 9,980 | 100 | 200 / 9,900 / 9,900 |

Structural gate medians, in milliseconds:

| Source row | Gate | Warm, 20 | Warm, 100 | Warm, no cache | Cold | Strict |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 300k | complexity | 16 | 41 | 16 | 8,452 | 2,983 |
| 300k | dead-symbols | 1,188 | 1,272 | 2,969 | 3,889 | 3,875 |
| 300k | reachability | 97 | 103 | 97 | 5,916 | 99 |
| 1M | complexity | 28 | 98 | 28 | 16,598 | 8,180 |
| 1M | dead-symbols | 1,657 | 1,820 | 6,862 | 10,941 | 10,822 |
| 1M | reachability | 222 | 225 | 220 | 9,358 | 206 |

The work counters for the changed-file-local gates scale with the same delta:

| Changed files | complexity reads / parses | dead facts reads / parses / extracted | escapes reads / parses | stubs reads / parses |
| ---: | ---: | ---: | ---: | ---: |
| 20 | 40 / 40 | 40 / 40 / 40 | 40 / 20 | 40 / 40 |
| 100 | 200 / 200 | 200 / 200 / 200 | 200 / 100 | 200 / 200 |

The current sandbox did not expose peak RSS to the fixture, so its resource
rows say unavailable rather than inventing a number. The controlled reference
rows above still report 806,304 kB warm-cache and 833,648 kB without the cache
at 1M, against the post-#183 roughly 955 MB comparison point; strict
whole-tree work is the separate 1,019,936 kB path.

The 1M cache file is 25,863,432 bytes and the 300k file is 8,416,407 bytes.
Four equally sized retained files therefore occupy at most 103,453,728
bytes and 33,665,628 bytes respectively for these workloads. This is a
measured retention envelope, not a byte-LRU policy. The 1M warm 20-file row is
2.86 seconds, below the roughly 5 second milestone of #187.

The boundary test in tests/structural_views.rs combines a cached Rust import
and module declaration with an unparsed file and compares coverage and
verdicts across runs. The structural cache round-trip tests cover the
corresponding fact fields directly. Change remains run data, separate from
the reusable structural facts, so this proves the #50 input boundary without
adding module resolution, layering or cycle analysis.

The dense fixture now reports the original 20-file warm row and a second
100-file warm row for each source volume. Its assertions require changed-file
fact extraction to scale with that delta while unchanged base outcomes stay
cached and shared. The rows provide evidence for #182; they do not establish
a final product performance or memory budget.
