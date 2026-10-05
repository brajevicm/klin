# Exact copied regions: speed prototype, 2026-10-05

This note is step 1 of #480 under #478. It measures whether a detector of
exact copied regions can run inside the #478 Stop limits. Step 2 (threshold)
did not start.

Status: sections 1 to 6 measure designs A and B (data in `results-ab/`).
Design A misses the cache limit. Design B meets the limits but can block a
region that is not an exact copy. Section 7 is the design that replaces both:
design C, the proven chain. It gave no false positive in any run and meets
every #478 limit, at 1M too. Section 8 narrows design C's band from 58 to
38 tokens with a smaller index scheme. That is now the default. Its 1M row
waits for the owner.

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
  `results-ab/*.jsonl` holds every run. `results-ab/summary.txt` holds the summary.
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

## 7. Design C: the proven chain

### Idea

Design B joins hits on one diagonal that are up to w tokens apart, but a
k-gram covers only k tokens. So the tokens between two hits are not proven
equal, and B can block a region that is not one exact copy.

Design C sets w ≤ k. Winnowing selects at least one k-gram in every w
consecutive k-grams. So two neighbor hits inside one copy are at most w ≤ k
apart, and their k-grams overlap or touch. A chain of such hits on one
diagonal proves that every token from the first hit to the end of the last
k-gram is equal on both sides. The proof needs no base tokens, so the Stop
reads no unchanged source.

- **Regions (copied lines).** k = w = 30. A copy of at least
  k + w − 1 = 59 tokens shares a fingerprint. The proven length is
  `last − first + k`. The Stop blocks when the proven length is at least T.
- **Whole functions.** The chain loses up to w − 1 tokens at each edge. So a
  copied function of fewer than T + 2(w − 1) = 118 tokens may not be proven
  as a region. A second index holds a 64-bit hash of the token stream of each
  function from 10 to 117 tokens. The hash covers the parameters, return type
  and body, but not the name. Equal hashes block. Functions of 118 tokens or
  more are always proven by the region chain when they are copied whole.
- **Storage.** Both indexes are sorted. They use blocks of 64 entries, delta
  varint keys, and token offsets bit-packed to the width of the tree's token
  count. A function key keeps 48 bits of the hash. Keys that occur more than
  64 times are capped, and a hit on one makes the result INCOMPLETE.
  `proto/src/postings.rs` holds the format.

### Guarantee

For a threshold T:

1. **No false positive.** A blocked region is an exact copy of at least T
   tokens, and a blocked function is an exact copy of a function of at least
   10 tokens. The only exception is a hash collision. A false region needs a
   32-bit key collision on the same diagonal, next to a chain. A false
   function needs a 48-bit collision.
2. **Every long copy is found.** Every exact copy of at least
   T + 2(w − 1) = T + 58 tokens is blocked, unless a capped key breaks its
   chain. That case is reported as INCOMPLETE, not as a pass.
3. **Every whole-function copy is found.** Every function of at least 10
   tokens that is copied whole is blocked, by the function index or by the
   region chain.
4. **Deterministic.** The proven length depends only on the copied tokens. So
   the same copy always gets the same result, in both trees.

A copy of T to T + 57 tokens that is not a whole function may or may not be
blocked. That depends on where winnowing selects in it. `klin check` has no
Stop limit, so it can read both files and find the exact edges of these
copies.

### Validation

The query also reads the base token chain, for validation only. The cache
figures below leave the chain out. For each region the Stop blocks, the
validation finds the exact match. For each function hit, it compares the
tokens. It also finds every exact copy of at least T tokens among the hits,
which is the truth set. With k = w = 30, every copy of 59 tokens or more
shares a fingerprint, so this set is complete for T = 60.

| Tree | Changed | Blocked regions | False regions | Function hits | False functions | True regions ≥ 60 | Missed | Longest missed |
|---|---|---|---|---|---|---|---|---|
| 10k | 20 / 100 | 0 / 0 | 0 / 0 | 498 / 2,480 | 0 / 0 | 0 / 0 | 0 / 0 | — |
| 300k | 20 / 100 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | — |
| GlareDB | 20 / 100 | 40 / 256 | 0 / 0 | 54 / 291 | 0 / 0 | 49 / 404 | 9 / 148 | 84 / 99 |
| karakeep | 20 / 100 | 8 / 22 | 0 / 0 | 27 / 72 | 0 / 0 | 19 / 50 | 11 / 28 | 87 / 89 |
| klin `src/` | 20 / 80 | 6 / 8 | 0 / 0 | 19 / 36 | 0 / 0 | 13 / 21 | 7 / 13 | 78 / 78 |

There were no false positives in 50 queries (5 runs of each row). Every
missed copy is shorter than the bound of 118 tokens. The fixtures block no
region because every query hits a capped key and is INCOMPLETE. Their
function hits are the fixture's repeated small functions.

Other values of k = w, at 20 changed files, show the trade-off between the
band and the index size:

| k = w | Band (T + …) | GlareDB missed | karakeep missed | Index / cache: 10k, 300k, GlareDB, karakeep |
|---|---|---|---|---|
| 30 | 58 | 9 of 49 | 11 of 19 | 3.8%, 6.2%, 5.0%, 7.7% |
| 20 | 38 | 7 of 49 | 5 of 19 | 4.9%, 8.2%, 7.0%, 10.9% |
| 16 | 30 | 7 of 49 | 4 of 19 | 5.0%, 9.3%, 8.5%, 13.2% |

Only k = w = 30 keeps every tree under 10%, so design C uses it.

### Cost

Data is in `results-c30/`, with its summary. The time is the
median of 5 runs (max), with reading and parsing left out.

| Tree | Index (no path table) | / klin cache | Cold extra | / klin cold hook | Warm 20 | Warm 100 | Warm peak RSS |
|---|---|---|---|---|---|---|---|
| 10k | 112,031 | 3.8% | 105 ms | 0.9% | 0.75 (0.77) | 1.65 (1.89) | 6.9 MB |
| 300k | 748,015 | 6.2% | 405 ms | 2.4% | 1.55 (1.59) | 5.26 (5.35) | 13.6 MB |
| GlareDB | 195,606 | 5.0% | 139 ms | 2.7% | 3.12 (3.14) | 23.14 (23.36) | 10.9 MB |
| karakeep | 94,463 | 7.7% | 70 ms | 3.4% | 3.08 (3.08) | 13.81 (13.91) | 9.0 MB |
| klin `src/` | 83,901 | — | 53 ms | — | 10.62 (10.72) | 55.09 (56.19), 80 files | 13.5 MB |

The index file also holds a path table. In klin, the structural cache already
holds the file list, so the path table is not counted. With the path table,
the 10k fixture is at 14.2%, because its 10,000 paths take 305,671 bytes.

The longest postings (up to 1,788 at 300k) are on capped keys only. The
longest posting kept is 64.

### 1M rows (owner)

```sh
cargo build --release --manifest-path docs/duplication-speed-2026-10-05/proto/Cargo.toml
WORK=/tmp/dup-speed docs/duplication-speed-2026-10-05/run.sh 1m /tmp/dup-1m
python3 docs/duplication-speed-2026-10-05/summary.py
```

The owner ran these commands on the fixture from section 6.

| #478 Stop limit | Design C at 1M | Result |
|---|---|---|
| 20 changed: median ≤ 15 ms, max ≤ 25 ms | 4.02 ms, 4.06 ms | met |
| 100 changed: median ≤ 50 ms | 15.81 ms | met |
| Cache growth ≤ 10% of 37,301,283 bytes | 2,710,514 bytes (regions 1,687,707, functions 1,022,795, capped keys 12), 7.3% | met |
| Cold ≤ 5% slower | 1,352 ms on the 31.76 s cold hook, 4.3% | met |
| No extra parses, no reads of unchanged source, no whole-tree walks, no external processes | the query reads the index and the changed files only | met by design |

- The path table adds 305,671 bytes. klin would use its own file list, so it
  is not counted. With it, the index is 8.1%.
- The 1M index holds 130,030 of 146,278 functions.
- Every query was INCOMPLETE (19 and 128 capped hits), as at 10k and 300k.
  So these times leave out the work for capped postings.
- The warm peak RSS of about 32 MB includes the validation, which reads the
  whole 23.6 MB token chain. The Stop path does not read the chain.
- There were no false positives and no missed copies at 1M. There were also
  no true regions of 60 tokens or more, because the capped keys hide the
  fixture's repeated bodies.

### Open points

- The cap of 64 makes every fixture query INCOMPLETE. The real trees never
  reach it.
- The function index blocks short exact copies from 10 tokens. Step 2 sets
  the minimum function size together with T.
- Warm time follows the size of the changed files, at about 155 ns for each
  token. 80 large klin files take 55 ms.

## 8. Design C with mod-minimizers (k = 41, w = 20)

### Change

Section 7 used k = w = 30. Its band was 58 tokens, and it missed 11 of 19
true copies on karakeep. A smaller w narrows the band, but with winnowing
k = w = 20 put karakeep at 10.4% of the cache.

Two changes make the index smaller:

1. **Elias-Fano keys.** The sorted keys are stored as fixed low bits plus
   unary high bits, with a select sample every 256 zeros
   (`proto/src/postings.rs`). This saved only about 4%. A random 32-bit key
   costs about log2(U/n) + 2 bits, and the token offset costs about
   log2(tokens) bits. Neither can get much smaller.
2. **Fewer entries.** The edge loss depends only on w, and the chain proof
   needs only k ≥ w. So k can grow until k + w − 1 = T. The mod-minimizer
   (Groot Koerkamp and Pibiri, 2024) uses a large k to select fewer k-grams,
   and it keeps one selection in every window of w k-grams. It finds the
   smallest t-mer in the window and selects the k-gram at that position
   mod w. With k = 41, w = 20 and t = 5, it selects 0.063 of the karakeep
   positions, against 0.091 for winnowing with k = w = 20. A unit test checks
   the window property and the shared-fingerprint guarantee.

The guarantee from section 7 holds with w = 20:
- every copy of k + w − 1 = 60 tokens shares a fingerprint;
- the band is T + 2(w − 1) = T + 38;
- the function index covers 10 to 97 tokens.

### Choosing k, w and t

Index size, with the path table left out, as a share of klin's structural
cache:

| k | w | t | Band | 10k | 300k | GlareDB | karakeep |
|---|---|---|---|---|---|---|---|
| 30 | 30 | winnowing | 58 | 3.5% | 5.8% | 4.7% | 7.4% |
| 20 | 20 | winnowing | 38 | 4.5% | 7.7% | 7.0% | 10.4% |
| 41 | 20 | 21 | 38 | 2.9% | 6.4% | 5.5% | 8.1% |
| **41** | **20** | **5** | **38** | **2.8%** | **6.3%** | **5.2%** | **7.8%** |
| 45 | 16 | 13 | 30 | 2.8% | 7.0% | 6.0% | 8.9% |
| 45 | 16 | 5 | 30 | 2.9% | 7.1% | 6.3% | 9.3% |
| 49 | 12 | 5 | 22 | 3.0% | 7.8% | 7.3% | 10.8% |
| 51 | 10 | 5 | 18 | 3.1% | 8.8% | 8.4% | 12.3% |

The first row includes the Elias-Fano change, so it is smaller than in
section 7. Two settings fit on every tree with a band under 58: w = 20 and
w = 16. Their misses over the 6 real-tree queries:

| Setting | Total missed (of 556 true copies) |
|---|---|
| k = w = 30, winnowing (section 7) | 216 |
| k = 41, w = 20, t = 5 | 125 |
| k = 45, w = 16, t = 13 | 165 |

w = 16 has the narrower band, but it missed more copies here. A copy is
missed when its two edge losses add up to more than L − T, so the count
depends on where the selections fall. k = 41, w = 20, t = 5 missed the
fewest and has more room under the cache limit. It is the default.

### Results

Data is in `results/`, and the summary is in `summary.txt`. The time is the
median of 5 runs (max), with reading and parsing left out.

| Tree | Changed | Time | Blocked regions | False | Function hits | False | True ≥ 60 | Missed | Longest missed |
|---|---|---|---|---|---|---|---|---|---|
| 10k | 20 | 0.73 (0.74) | 0 | 0 | 498 | 0 | 0 | 0 | — |
| 10k | 100 | 1.61 (1.62) | 0 | 0 | 2,480 | 0 | 0 | 0 | — |
| 300k | 20 | 1.51 (1.54) | 0 | 0 | 0 | 0 | 0 | 0 | — |
| 300k | 100 | 5.00 (5.02) | 0 | 0 | 0 | 0 | 0 | 0 | — |
| GlareDB | 20 | 3.04 (3.13) | 43 | 0 | 54 | 0 | 49 | 6 | 66 |
| GlareDB | 100 | 22.09 (22.62) | 320 | 0 | 290 | 0 | 404 | 84 | 79 |
| karakeep | 20 | 2.90 (3.18) | 11 | 0 | 23 | 0 | 19 | 8 | 87 |
| karakeep | 100 | 13.34 (13.67) | 36 | 0 | 68 | 0 | 50 | 14 | 74 |
| klin `src/` | 20 | 10.04 (10.15) | 9 | 0 | 19 | 0 | 13 | 4 | 63 |
| klin `src/` | 80 | 52.25 (52.57) | 12 | 0 | 36 | 0 | 21 | 9 | 74 |

- There were no false positives in 50 queries.
- Every missed copy is under the bound of 98 tokens.
- With k = 41, the 300k fixture has no capped key, and its queries are
  complete. The 10k fixture still hits 11 capped keys.

| Tree | Index (no path table) | / klin cache | Cold extra | / klin cold hook |
|---|---|---|---|---|
| 10k | 82,597 | 2.8% | 110 ms | 1.0% |
| 300k | 756,537 | 6.3% | 414 ms | 2.5% |
| GlareDB | 205,308 | 5.2% | 139 ms | 2.7% |
| karakeep | 95,095 | 7.8% | 71 ms | 3.5% |

### Lines

klin counts tokens, not lines. `dup-speed lines ROOT T FMIN` reports how many
source lines a run of T tokens spans, at every start position, and how many
lines each function of at least FMIN tokens spans:

| Tree | 60 tokens: min / p10 / median / p90 | 98 tokens: min / p10 / median / p90 | Functions ≥ 10 tokens: min / median, on one line |
|---|---|---|---|
| GlareDB | 1 / 7 / 11 / 18 | 1 / 11 / 18 / 27 | 3 / 7, none |
| karakeep | 3 / 8 / 12 / 16 | 6 / 14 / 19 / 25 | 1 / 8, 251 of 2,410 |
| klin `src/` | 1 / 5 / 9 / 15 | 2 / 10 / 15 / 22 | 3 / 10, none |

So at T = 60:
- A copied block can block when it is a single long line, but most blocked
  copies span about 9 to 12 lines.
- Every copy of 98 tokens or more blocks. That is about 15 to 19 lines in
  the median, and one or two lines when the lines are very long.
- A copied whole function blocks from 10 tokens. In karakeep that includes
  one-line arrow functions.

T and the minimum function size come from step 2, so these line counts change
with them.

### 1M row (owner)

```sh
cargo build --release --manifest-path docs/duplication-speed-2026-10-05/proto/Cargo.toml
WORK=/tmp/dup-speed docs/duplication-speed-2026-10-05/run.sh 1m /tmp/dup-1m
python3 docs/duplication-speed-2026-10-05/summary.py
```
