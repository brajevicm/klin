# Exact copied regions: speed prototype, 2026-10-05

This note is step 1 of #480 under #478. It measures whether a detector of
exact copied regions can run inside the #478 Stop limits. Step 2 (threshold)
did not start.

Status: measured on the 10k, 300k and 1M fixtures and on three real trees.
The owner ran the 1M rows. Section 6 compares them with the #478 Stop limits.

## 1. Prototype

`proto/` is a standalone crate. It uses the grammar versions that klin pins
(`tree-sitter` 0.27.0, `tree-sitter-rust` 0.24.2, `tree-sitter-typescript`
0.23.2). `proto/src/fixture.rs` is a verbatim copy of the perf fixture
generator in `tests/performance.rs` at `c1805539`. The 300k digest check
passed, so the copy makes the same tree.

- **Token stream.** One parse per file. The tokens come from that tree. The
  stream follows `docs/duplication-normalization-2026-10-05.md`: no comments
  or layout, the TS terminator rule (15 terminated kinds, taken from the
  pinned `grammar.json`), import provenance, test paths and Rust
  `#[test]`/`#[cfg(test)]` items left out. Rust and TS never match.
- **Fingerprints.** Rolling k-gram hash, robust winnowing, k = 20, w = 41.
  Every exact match of at least k + w − 1 = 60 tokens shares a fingerprint.
  A unit test checks this guarantee.
- **Index.** One sorted array of (32-bit key, file, position), 12 bytes for
  each entry, built cold. A key with more than 64 postings is capped. The
  index keeps only its key.
- **Warm query.** Reads the index file and the changed files only. It ignores
  base postings of the changed files and matches the changed files against
  each other in memory. A changed fingerprint on a capped key makes the
  result INCOMPLETE.
- **Design A (exact edges at Stop).** A second file holds a 32-bit hash for
  each base token. The query reads only the slices of the files it hits, and
  extends each hit to its exact region.
- **Design B (approximate edges at Stop).** The query chains the hits on one
  diagonal. The span gives a lower bound. A region with a lower bound of at
  least T blocks. A region whose bound is within 2(w − 1) tokens of T is
  unclear and goes to `klin check` for exact edges.

`proto/` has 4 unit tests (`cargo test --release`): the ASI rule, comment and
test-item removal, import provenance, and the winnowing guarantee.

## 2. Method

- Machine: macOS, aarch64, developer laptop. Release profile. One process for
  each run, so each run starts cold in memory, like a hook.
- `run.sh NAME ROOT` builds the index cold once, then runs 5 warm queries for
  20 and for 100 changed files at T = 60. `summary.py` prints medians.
  `results/*.jsonl` holds every run. `summary.txt` holds the summary.
- The changed files are spread evenly over the sorted file list. Their content
  is the base content, so every match against other files is found.
- "Extra" time leaves out reading and parsing. klin's single shared parse
  already does that work. The read and parse time is listed apart.
- Peak memory is the maximum resident set size of the whole process, from
  `/usr/bin/time -l`.
- klin's structural cache and cold hook time come from klin `c1805539`
  (`gate --hook --changed`, `{"build":[]}`, cache removed first, 3 runs).
- Trees: the 10k fixture is the `file-count` profile at 5,000 files for each
  language (10,000 files, 84k lines). The 300k fixture is
  `source-dense-300k`. The real trees are klin's `src/` at `c1805539`, GlareDB
  at `8001afa4` and karakeep at `f8ae9866`.

## 3. Results

### Cold build

| Tree | Files | Lines | Tokens | Extra ms | klin cold hook | Extra / cold | Peak RSS |
|---|---|---|---|---|---|---|---|
| 10k | 9,998 | 84,093 | 402,961 | 129 | 11.4 s | 1.1% | 7.8 MB |
| 300k | 9,998 | 325,003 | 1,875,781 | 413 | 16.8 s | 2.5% | 18.0 MB |
| GlareDB | 618 | 128,763 | 573,546 | 159 | 5.1 s | 3.1% | 14.7 MB |
| karakeep | 493 | 50,612 | 264,422 | 69 | 2.1 s | 3.3% | 8.7 MB |
| klin `src/` | 80 | 37,195 | 233,276 | 51 | not measured | — | 7.8 MB |

The extra time includes the token walk, winnowing, the sort and the writes.

### Index size

| Tree | Index (B) | Token chain (A only) | klin structural cache | B / cache | A + B / cache |
|---|---|---|---|---|---|
| 10k | 436,139 | 1,611,844 | 2,944,623 | 14.8% | 69.6% |
| 300k | 1,084,747 | 7,503,124 | 11,986,034 | 9.0% | 71.6% |
| GlareDB | 349,666 | 2,294,184 | 3,945,673 | 8.9% | 67.0% |
| karakeep | 164,831 | 1,057,688 | 1,221,545 | 13.5% | 100.1% |
| klin `src/` | 132,732 | 933,104 | not comparable | — | — |

klin's own structural cache covers `src/` and `tests/` (9,745,251 bytes), so
the ratio for klin `src/` is left out.

### Postings

| Tree | Longest posting | Capped keys (> 64) |
|---|---|---|
| 10k | 499 | 7 |
| 300k | 4,313 | 3 |
| GlareDB | 52 | 0 |
| karakeep | 25 | 0 |
| klin `src/` | 11 | 0 |

### Warm query, median of 5 (max), ms

| Tree | Changed | Design B | Design A | Token walk + winnow | Shared read + parse | INCOMPLETE | Peak RSS |
|---|---|---|---|---|---|---|---|
| 10k | 20 | 0.36 (0.38) | 0.37 (0.40) | 0.20 | 0.87 | yes | 4.9 MB |
| 10k | 100 | 1.01 (1.11) | 1.03 (1.14) | 0.83 | 3.28 | yes | 5.1 MB |
| 300k | 20 | 1.10 (1.24) | 1.12 (1.26) | 0.79 | 2.15 | yes | 5.9 MB |
| 300k | 100 | 3.90 (3.98) | 3.93 (4.03) | 3.49 | 8.89 | yes | 6.4 MB |
| GlareDB | 20 | 2.46 (2.47) | 2.78 (2.83) | 2.23 | 6.81 | no | 6.0 MB |
| GlareDB | 100 | 17.92 (18.12) | 18.74 (18.81) | 16.84 | 48.73 | no | 9.3 MB |
| karakeep | 20 | 2.34 (2.45) | 2.47 (2.60) | 2.18 | 4.82 | no | 5.8 MB |
| karakeep | 100 | 10.87 (11.17) | 11.08 (11.39) | 10.42 | 22.01 | no | 7.8 MB |
| klin `src/` | 20 | 8.44 (8.83) | 8.78 (9.40) | 7.91 | 18.33 | no | 6.5 MB |
| klin `src/` | 80 (all) | 42.73 (42.99) | 42.96 (43.18) | 40.91 | 96.31 | no | 10.8 MB |

Index load, lookup and region work stay under 2 ms in every row. The token
walk is the cost: about 155 ns for each token (`dup-speed walk`, klin `src/`,
30 passes). So the warm time follows the size of the changed files, not the
size of the tree.

### Regions at T = 60, 20 changed files

| Tree | Regions | B blocks | B unclear | A blocks |
|---|---|---|---|---|
| GlareDB | 310 | 35 | 275 | 32 |
| karakeep | 335 | 12 | 323 | 18 |
| klin `src/` | 152 | 4 | 148 | 13 |

These counts include old copies. The lineage (#479, #489) decides which are
new, so they are not a measure of findings.

## 4. Findings

1. **Design A does not fit the cache limit.** The token chain costs 4 bytes
   for each token. It adds 55% to 87% to klin's structural cache on every
   tree. The #478 limit is 10%. Design A is out unless the chain gets much
   smaller.
2. **Design B is near the cache limit.** Its index is 9.0% of the cache at
   300k and 8.9% at GlareDB, but 14.8% at the 10k fixture and 13.5% at
   karakeep. The 1M ratio is not measured. 12 bytes for each entry is
   not compact: a delta-coded key and a 16-bit position could make it smaller.
   This is not tried.
3. **Design B leaves most regions unclear.** With w = 41, the unclear band is
   80 tokens wide. At 20 changed files, 275 of 310 GlareDB regions are unclear.
   They go to `klin check`, so the Stop result names few regions.
4. **Design B can over-block.** The chain joins hits on one diagonal that are
   at most w apart. Two exact regions with a short edit between them become one
   long region. On GlareDB, B blocks 35 regions and A blocks 32. The prototype
   did not check which regions cause the difference. Design A also has a gap:
   when its interior check finds a mismatch, it drops the region and does not
   split it.
5. **The generated fixtures are always INCOMPLETE.** Their repeated bodies put
   up to 4,313 postings on one key. Every warm query on the 10k and 300k
   fixtures hit a capped key. The real trees hit none at cap 64 (longest
   posting 52).
6. **The token walk sets the warm time.** On the fixtures, whose files are
   small, 20 changed files cost about 1 ms. On klin `src/`, whose files are
   large, 20 changed files cost 8.4 ms. The #478 limit is 15 ms at 1M with
   20 changed files. A Stop where an agent changes large files may come close
   to it. The 1M fixture has small files, so its row may not show this.
7. **Cold cost is small.** The extra cold work is 1.1% to 3.3% of klin's cold
   hook time on these trees. The #478 limit is 5%.

## 5. Not measured, and simplifications

- The 1M rows: owner run.
- The prototype is not inside klin. In klin, the token walk would use the
  shared parse. The prototype parses on its own, and lists that time apart.
- The prototype reads the whole index file for each query. An index at 1M
  may make load time grow. The 300k load was 0.23 ms.
- Test exclusion uses path rules and Rust test attributes only. It does not
  use klin's `cfg_test_ranges` function.
- Units with an `ERROR` node are counted (1 file in GlareDB, 2 in karakeep)
  but they are not kept from blocking.
- The TS terminator rule does not cover class members and interface members.
  Their `;` belongs to `class_body` and `object_type` in the grammar.
- Keys are 32 bits. A key collision can make a false hit. Design A drops it
  when it compares tokens. Design B does not see it.

## 6. 1M rows (owner)

The owner ran these commands. The generator gave 1,033,827 lines and digest
`5045938053977738811`, the values in `tests/performance.rs`.

```sh
cargo build --release --manifest-path docs/duplication-speed-2026-10-05/proto/Cargo.toml
docs/duplication-speed-2026-10-05/proto/target/release/dup-speed gen /tmp/dup-1m 1m
WORK=/tmp/dup-speed docs/duplication-speed-2026-10-05/run.sh 1m /tmp/dup-1m
python3 docs/duplication-speed-2026-10-05/summary.py
```

| Measure | Value |
|---|---|
| Files, tokens | 9,998, 5,913,813 |
| Cold extra ms (read + parse apart) | 1,243 (read + parse 2,458) |
| Index (B) | 3,152,763 bytes |
| Token chain (A only) | 23,655,252 bytes |
| Longest posting, capped keys | 15,485, 5 |
| Build peak RSS | 60.5 MB |
| Warm 20: B median (max), A median (max) | 2.86 (3.04), 2.89 (3.07) ms |
| Warm 100: B median (max), A median (max) | 11.73 (11.87), 11.83 (11.94) ms |
| Warm peak RSS | 8.3 MB (20), 9.5 MB (100) |
| INCOMPLETE | yes, every query (43 and 252 capped hits) |

| #478 Stop limit | Design B | Design A |
|---|---|---|
| 20 changed: median ≤ 15 ms, max ≤ 25 ms | met: 2.86, 3.04 | met: 2.89, 3.07 |
| 100 changed: median ≤ 50 ms | met: 11.73 | met: 11.83 |
| Cache growth ≤ 10% of 37,301,283 bytes (`748d01fc`) | met: 8.5% | missed: 71.9% |
| Cold ≤ 5% slower | met: 1,243 ms extra on a 31.76 s cold hook, 3.9% | same |
| No extra parses, no reads of unchanged source, no whole-tree walks, no external processes | met by design: the query reads the index and the changed files only | met by design; it also reads chain slices of hit files |

Every 1M query was INCOMPLETE. A capped key ends its lookup early, so these
times do not include the work that the capped postings would cost. The
20-changed time is about one fifth of the limit, so a Stop with no capped key
probably also fits. The prototype did not measure that.

The cold hook time is one owner run of klin `c1805539`
(`gate --hook --changed`, `{"build":[]}`, `.git/klin` removed first). It is
one sample, not a median.

Design A misses the cache limit at 1M, as on every other tree. Design B meets
every #478 Stop limit.
