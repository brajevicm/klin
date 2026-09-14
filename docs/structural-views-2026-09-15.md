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
