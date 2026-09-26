# Multi-source ModuleGraph admission on the dense fixture (#221)

#220 changed `Module.file` to `Module.sources`, gave each `Dependency` a
source position, and paired layering edges by semantic identity before the
physical findings. This note measures that change on the existing
`structural_300k` and `structural_1m` rows. It adds no language, no
benchmark framework and no fixture tree.

Verdict: **PASS.** #220 adds no syntax work and no repository-sized work.
Its one repeatable cost is 9 to 13 ms of `layering_ms` in seven of eight
A/B pairs, and 20 ms in one. It comes from the semantic pairing of the
fixture's held cycles, and it scales with reported semantic edges. One local
revision, `a09664d9`, removes the naming work for sites that report nothing.

## Method

- **A (before):** `178643d8`, `main` immediately before the #220 merge.
- **B (after):** `5c3e404b`, the #220 merge.
- **B′ (fixed):** `5c3e404b` with `a09664d9` applied.

Each side ran from its own worktree with the same harness patch, `b13eecf3`.
That patch lets `structural_1m` run the targeted warm cases, prints the peak
RSS of one warm hook on a targeted row, and prints the dispatch counters.
`KLIN_BIN` is not usable for this A/B, because a binary it names gets no
`layering` section, and #220 changed that gate.

The machine is MacBookPro18,3, Apple M1 Pro, macOS 26.6.2, Darwin 25.6.0,
rustc 1.98.1, release builds. Every row is five iterations, median. The
sides ran interleaved within each scenario, with no turn ending during a
chain. Each row ran this command in its worktree:

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --locked --test performance -- --ignored perf --nocapture
```

The other rows change `KLIN_PERF_ROW` to `structural_1m` and
`KLIN_PERF_CASE` to `warm100`. Every row passed its fixture assertions,
including the assertion that `layering` and `public-api` read and parse no
source of their own.

## Rows

Pass 1, A → B:

| Scenario | Warm median ms | `layering_ms` | `layering_graph_ms` | `public-api_graph_ms` | Peak RSS kB |
| --- | ---: | ---: | ---: | ---: | ---: |
| 300k / 20 | 816 → 884 | 56 → 67 | 22 → 29 | 16 → 17 | 140,304 → 147,888 |
| 300k / 100 | 931 → 976 | 55 → 65 | 22 → 29 | 16 → 16 | 144,576 → 156,688 |
| 1M / 20 | 1,319 → 1,341 | 58 → 67 | 24 → 30 | 17 → 17 | 325,008 → 323,040 |
| 1M / 100 | 1,938 → 1,816 | 58 → 71 | 22 → 30 | 17 → 18 | 330,544 → 327,376 |

Pass 2, A / B / B′:

| Scenario | Warm median ms | `layering_ms` | `layering_graph_ms` | Peak RSS kB |
| --- | ---: | ---: | ---: | ---: |
| 300k / 20 | 844 / 1,027 / 987 | 57 / 77 / 70 | 22 / 34 / 30 | 144,672 / 149,632 / 155,648 |
| 300k / 100 | 998 / 1,019 / 1,009 | 57 / 66 / 66 | 22 / 29 / 29 | 144,896 / 160,272 / 153,072 |
| 1M / 20 | 1,341 / 1,360 / 1,372 | 58 / 69 / 69 | 23 / 30 / 31 | 323,408 / 325,856 / 321,968 |
| 1M / 100 | 1,813 / 1,701 / 1,754 | 59 / 69 / 70 | 24 / 30 / 29 | 330,800 / 330,544 / 329,328 |

The warm medians move in both directions by up to 183 ms. `dead-symbols`,
which #220 did not change, moves by up to 115 ms between the same pairs, so
most of the total movement is noise. The layering movement is the only one
that repeats in all eight A/B pairs.

The 1M warm/20 median stays under the 5-second budget of SPEC 13. No row
rises by one third. The cold and strict 1M rows were not rerun: #220 changed
no cache encoding and no whole-tree extraction. The structural cache holds
36,939,969 bytes at 1M on every side, within 2 bytes between runs of one
binary.

## Work counters

Every deterministic counter that A records is equal on B and B′, in every
scenario: reads, parses, extractions, cache hits and shares for every gate,
`graph_modules` and `graph_dependencies`. For the 20-file delta, each gate
that extracts facts reads and parses 40 files, and the other 9,980 files come
from the cache. For the 100-file delta the counts follow the delta. So #220
adds zero structural reads, parses and extractions, and the expensive work
still scales with the changed set.

The counters that B adds, per gate, over both trees:

| Counter | Value | Meaning |
| --- | ---: | --- |
| `graph_modules` | 10,010 | semantic modules |
| `graph_sources` | 10,010 | physical source memberships |
| `graph_dependencies` | 9,906 | physical dependency sites |
| `graph_edges` | 9,906 | distinct module pairs those sites join |
| `graph_dispatches_rust`, `_typescript` | 2, 2 | one resolver run per tree |
| `surface_dispatches_rust`, `_typescript` | 2, 2 | one surface derivation per tree |

Rust and TypeScript make one-source modules, so sources equal modules. The
fixture writes one site per module pair, so sites equal edges. The unit test
`multi_source_work_is_linear_in_files_modules_sites_and_unique_edges` pins
the multi-source case: each file is placed once, each module is folded and
named once, each site is judged once, and the SCC input is the number of
distinct module pairs.

## Attribution

Layering's `graph_ms` is the resolution time of its two graphs plus its own
judgement: placement, cycles and edges. `public-api` resolves the same graphs
and its `graph_ms` did not move, so the cost is in the judgement.

A scratch build timed each step of that judgement. It ran seven runs of
`gate --changed --json --gate layering` per binary on one saved 300k tree
after its 20-file change, interleaved. Medians, in microseconds:

| Step | A | B′ |
| --- | ---: | ---: |
| placement | not a step | 970 |
| cycles | 767 | 697 |
| edges | 4,030 | 10,025 |

The fixture holds 4,946 cyclic edges on each side, and the base holds all of
them. For each such site, B builds the report name and the semantic identity
of the destination module. It then groups the site under its module pair,
and re-keys that pair under two semantic identities and the edge text. That
re-keying is the pairing that #219 and ADR 0047 require. A built one name and
one physical key per site. The added time is about 0.6 µs for each held
cyclic edge on each side. It grows with the reported semantic edges, not
with the repository.

With `acyclic` switched off on a copy of the same tree, no site reports
anything:

| Step | A | B | B′ |
| --- | ---: | ---: | ---: |
| edges | 1,526 | 2,334 | 270 |

B named every destination before it looked at the verdict. `a09664d9` makes
a site that reports nothing skip the naming. That is the common case on a
real repository, where most dependencies are allowed. The unit test
`an_allowed_site_names_no_module` pins it. The revision does not change the
held-cycle case, because each of those sites reports a cycle.

## Memory

#220 changes the owned representation as follows:

- `Module.file: String` became `Module.sources: Vec<String>`. The inline
  size is 24 bytes both ways. Each module owns one more heap buffer of one
  `String`.
- `Dependency` grew from 24 to 32 bytes with `source: u32`. A site holds the
  position of its file among its module's sources. It holds no owned path.

Estimate: 10,010 modules × 32 bytes plus 9,906 sites × 8 bytes, over the four
graphs one stop builds, is about 1.6 MB. The held-cycle case adds transient
edge maps in layering. By an estimate from the key and node sizes, those maps
hold a few MB while both sides' edges are alive.

The warm hook RSS at 300k is higher on B and B′ in all six samples: 147.9 to
160.3 MB, against 140.3 to 144.9 MB on A. At 1M it is flat, at −3 to +2 MB.
One single-command test on the saved 300k tree, with the order reversed,
moved the same binary by 10 to 15 MB with its place in the sequence. Both
hook-row passes ran A first. These single-run values therefore cannot show a
10 MB effect. The 300k rise is consistent with the transient layering maps
at a peak that layering sets at 300k and `dead-symbols` sets at 1M. That is
an inference, not a measurement. SPEC 13 and ADR 0042 set no RSS budget, and
1M RSS did not move.

## Absent languages

`tests/layering.rs` `a_tree_with_no_typescript_path_dispatches_only_the_rust_resolver`
and `tests/public_api.rs` `a_tree_with_no_typescript_path_derives_only_rust_surfaces`
assert `"typescript": 0` dispatches on a tree that holds only Rust. A resolver
or surface derivation that does not dispatch reads, parses and scans
nothing. The dispatch decision is one pass over the file list the run already
holds. It stops when it has found every registered language, and it reads
no file.

## Decision rule

1. Semantic characterization is unchanged. On the saved 300k tree, A, B
   and B′ report the same outcome: 0 findings, 4,946 held cycles and the
   same coverage. The only difference is the `OK:` line's word "site", which
   #220 changed on purpose. The surface counters are equal in every row. The
   `layering` and `public_api` CLI suites pass on `main` with `a09664d9`.
2. No additional Rust or TypeScript read, parse or extraction path exists.
   The counters are equal.
3. The 20- and 100-file syntax work follows the delta.
4. Modules equal sources, and sites equal edges, on today's languages. The
   unit test pins the multi-source shape.
5. Layering places each file once and folds each module once. The unit test
   counts it.
6. The SCC input is the distinct module pairs: `graph_edges` counts them,
   and the unit test pins them.
7. An absent language dispatches nothing.
8. The layering cost is explained, and it scales with reported semantic
   edges. The edges step takes about 6 ms more on the saved tree. The rows
   show 9 to 13 ms more `layering_ms`, which is about 1 to 1.5% of the 300k
   warm hook and under 1% at 1M. It pays for the semantic pairing that
   ADR 0047 requires. The RSS movement is within this machine's
   single-run noise, and 1M RSS did not move.
9. SPEC 13 and ADR 0042 are unchanged.
