# Structural representation pressure and the C/D decision (#200)

#197 asks two questions about the facts a run holds:

- **C.** A compact representation, with FileId and declaration layout judged
  apart from SymbolId.
- **D.** A packed or zero-copy representation backed by the structural cache.

This note records the measured footprint, the estimates built on it, and a
verdict for each part. It implements no representation change and no cache
format change. It required no process-RSS measurement.

## Method

Every counter comes from the `footprint` group of the `dead-symbols` gate row
(11.2). Two samples:

- The dense 300k row, at commit `51681bd`, five iterations, median, run by the
  repository owner on MacBookPro18,3 (Apple M1 Pro, macOS 26.6.2), rustc
  1.98.1, release build:

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture
```

- klin's own repository, one whole run of `klin gate --json --gate
  dead-symbols`, which extracts both trees. It is a real-code sample beside
  the generated one, because the fixture writes no inline module and no
  default export.

A counter is a measurement. A total, a share and a saving is an estimate,
marked as such, and each estimate names its assumption.

## Measured type sizes

The size of one value, without the bytes its strings and lists own:

| Type | Bytes |
| --- | ---: |
| `FileFacts` | 176 |
| `Declaration` | 168 |
| `Import` | 128 |
| `ModuleDecl` | 112 |
| `Export` | 112 |
| `ExportLeaf` | 48 |
| `Reference` | 32 |

`Declaration` holds four fields that each cost 24 bytes in every value:
`nesting`, `exported_as`, `owner` and `signature`. `Reference` holds a name
and a line.

## Measured populations and bytes

Each file counts once, however many trees hold one extraction of it.

| Counter | 300k fixture | klin repository |
| --- | ---: | ---: |
| `files` | 10,020 | 199 |
| `declarations` | 90,237 | 6,843 |
| `references` | 330,874 | 163,059 |
| `imports` | 10,022 | 1,086 |
| `module_declarations` | 4 | 219 |
| `exports` / `export_leaves` | 1 / 1 | 2 / 2 |
| `qualified_paths` | 0 | 55 |
| `path_bytes` | 226,186 | 3,306 |
| `declaration_name_bytes` | 898,555 | 144,634 |
| `declaration_text_bytes` | 2,953,370 | 353,472 |
| `reference_name_bytes` | 2,261,484 | 862,155 |
| `signatures` / `signature_bytes` | 80,217 / 2,637,592 | 6,843 / 343,884 |
| `owners` / `owner_bytes` | 5,010 / 55,110 | 638 / 4,278 |
| `exported_aliases` | 0 | 0 |
| `nestings` / `nesting_entries` / `nesting_bytes` | 0 / 0 / 0 | 196 / 196 / 1,232 |
| `import_text_bytes` | 856,842 | 113,589 |
| `module_text_bytes` | 78 | 3,973 |

The structural cache of the 300k row holds one file of 11,850,967 bytes. The
row reads and decodes it in 60 ms of a 2,230 ms Stop, which is 2.7%.

### Sparsity

| Optional field | 300k fixture | klin repository |
| --- | ---: | ---: |
| `signature` present | 88.9% | 100% |
| `owner` present | 5.55% | 9.32% |
| `nesting` non-empty | 0% | 2.86% |
| `exported_as` present | 0% | 0% |

Both samples are Rust-heavy, and only TypeScript writes a default export, so
the `exported_as` rate of a TypeScript-heavy tree is unmeasured.

## Derived totals (estimate)

Structure bytes are the population times the type size. String bytes are the
measured payload. The totals exclude the per-allocation overhead of each
`String` and `Vec`, their capacity slack, and the name index, so the real
memory is larger.

| Estimate | 300k fixture | klin repository |
| --- | ---: | ---: |
| Structure bytes | 27.46 MiB | 6.26 MiB |
| String payload | 9.43 MiB | 1.75 MiB |
| Total facts | 36.89 MiB | 8.01 MiB |
| `Declaration` values' share | 39.2% | 13.7% |
| `Reference` values' share | 27.4% | 62.1% |
| References per declaration | 3.7 | 23.8 |

The shape of real code differs from the fixture. klin's own tree holds 24
references per declaration, and its `Reference` values alone carry 62% of its
facts bytes.

For historical context, #196 recorded a 300k warm hook peak of 140,432 kB on
this machine. The 36.89 MiB estimate is about 27% of that figure. No RSS was
measured for this note.

## Verdicts

### FileId: rejected for footprint

`path_bytes` is 226,186 in the fixture, which is 0.6% of the facts bytes, and
3,306 in klin. The other path copies a run holds, such as each tree's
extraction map and each measurement's coverage lists, are of the same order.
Compact file identity cannot move memory.

Its other claim was comparison cost. #199 measured the name queries of both
gates at 0 ms to 13 ms per tree, so the path comparisons inside them have no
measured cost to remove either. No implementation ticket.

### Declaration layout: deferred

`nesting`, `exported_as` and `owner` cost 72 bytes in every declaration and
are almost always empty. Splitting them out saves an estimated 6.2 MiB in the
fixture (16.8% of its facts bytes) and 0.47 MiB in klin (5.9%), because
klin's declarations are only 13.7% of its facts.

`signature` is present in 88.9% and 100% of declarations, so it is not a cold
field and must stay in the hot value.

The saving is real but small where real code was measured, and the split adds
a second lookup to every consumer of a nesting, an owner or an alias. No
ticket of its own. A later compact-representation ticket may take it as part
of a wider change.

### SymbolId: recommended

This is the one part with an implementation case, and the case is measured,
not assumed from the idea that names repeat.

| Evidence | 300k fixture | klin repository |
| --- | ---: | ---: |
| Name occurrences (declarations and references) | 421,111 | 169,902 |
| Distinct names | 65,590 | 7,326 |
| Repetition | 6.4× | 23.2× |
| Name payload | 3.01 MiB | 0.96 MiB |

Estimated saving, on the assumptions that an id is 4 bytes, that a
`Reference` of an id and a line packs to 16 bytes, that a declaration keeps
one id in place of one `String`, and that the index reuses the same ids:

| Estimated saving | 300k fixture | klin repository |
| --- | ---: | ---: |
| Name payload not copied | 2.54 MiB | 0.92 MiB |
| 16 bytes per site | 6.43 MiB | 2.59 MiB |
| Together | 8.97 MiB (24.3%) | 3.51 MiB (43.8%) |

It also removes about 356,000 string allocations in the fixture, which the
totals above do not count.

An implementation must keep the name-only resolution rule of 8.4, the Rust
and TypeScript partitions, the deterministic order of sites, and the cache's
identity and fallback behaviour. The cache format decides whether ids are
written or rebuilt on read; this note does not decide that.

### D: deferred

The cache holds 11.3 MiB where the facts estimate is 36.89 MiB, and reading
and decoding it costs 60 ms of a 2,230 ms Stop. A packed or zero-copy
representation can remove at most that 60 ms of CPU, and it must still read
every byte, because the file carries a whole-body checksum. It must also keep
the corruption and truncation fallback, the atomic replacement, the four-file
eviction and the cross-platform behaviour of 8.4.

C reduces the same payload without touching the format, so D stays deferred
until a measured row shows that owned materialization is still dominant after
a compact name identity lands.

## What should receive an implementation ticket

One ticket: compact name identity over the shared structural owner, with the
name index reusing the same ids. Estimated 24% of the fixture's facts bytes
and 44% of klin's.

No ticket: FileId, the declaration-layout split, and D.

## Manual RSS boundary

A manual RSS check is worth running only where it can change a decision:

1. After compact name identity lands, one 300k warm/20 RSS check by the
   repository owner. The estimate predicts about 9 MiB less of facts against
   the 137 MiB warm peak recorded in #196, and the check decides whether the
   estimate held.
2. One 1M timing and RSS check at the final validation of the whole x10
   round, and only where a claim about 1M performance or the product contract
   changes.

No RSS run belongs to an investigation ticket, and none was run for this one.
