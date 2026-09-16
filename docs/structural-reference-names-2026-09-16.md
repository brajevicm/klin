# Compact repeated reference names

This records the deterministic evidence for #201. The before run uses
untouched `9f80742`; the after run is the implementation in this checkout. Both use
`KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20` with the release
performance test and the same `source-dense-300k` fixture (10,000 files,
digest `f62c3dae3eaff7ab`).

| measurement | before | after |
| --- | ---: | ---: |
| `Reference` size | 32 bytes | 16 bytes |
| `Name` size | — | 8 bytes |
| references | 330,874 | 330,874 |
| reference occurrence text bytes | 2,261,484 | 2,261,484 |
| semantic distinct reference names | 35,272 | 35,272 |
| physical canonical name allocations | 330,874 | 35,576 |
| canonical name bytes | 2,261,484 | 366,109 |
| canonical allocations / distinct names | 9.381x | 1.008x |
| estimated reference representation bytes | 10,587,968 | 5,293,984 |
| SourceIndex distinct names | 65,590 | 65,590 |
| SourceIndex index time (before + after) | 39 ms | 54 ms |
| cache bytes | 11,850,967 | 11,850,966 |
| cache read/decode | 33 ms | 39 ms |
| warm/20 median | 714 ms | 765 ms |

The before representation estimate is `references * 32`; the after
estimate is `references * 16`. The after-only allocation counters come from
the new footprint report. The before distinct-name, allocation, canonical-byte
and ratio values are deterministic derivations from the old one-`String`-per-
reference representation; the old binary had no equivalent counters. The
warm median increased by 51 ms (7.1%) in these runs, while the per-reference
representation estimate halved. The timing is a single noisy warm sample; no
cache format or epoch change was needed.

The manual RSS follow-up remains out of scope for this change, as allowed by
the issue; the existing 300k warm/20 run is the reproducible owner check.
