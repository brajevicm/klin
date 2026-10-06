# O5: candidate keys and lossless equality evidence

Status: all four pinned corpus roundtrips and 32/48/64-bit collision probes pass.
The initial lossless format fails the 1M cache budget.

Candidate keys may be compact because they only select possible partners.
They cannot establish equality. The current prototype's `chain.bin` stores
64-bit token hashes for candidate validation; full check reopens source and
compares canonical text. A lossless cache could supply that text evidence
without reopening unchanged files. Source: [build/query implementation](proto/src/main.rs),
specifically `Stream::emit`, `Stream::leaf`, `build`, and `query`.

The experiment uses the frozen binary's `normalize` CLI to obtain canonical
token bytes, including the language prefix and import provenance. Each distinct
safe token receives a dictionary ID; chains store unsigned varint IDs and zero
for unsafe boundaries. Dictionary records retain token bytes with length
prefixes. No token hash substitutes for these bytes. Excluded test units remain
excluded, and unsafe tokens cannot provide an equality witness. Unsafe lexemes
are intentionally not retained: zero preserves their exact boundary positions.
Rows are measured separately as delta varints for reporting; relative paths
and per-file token counts are charged separately.

The runnable stdlib experiment is
[compression.py](results-optimizations/compression.py). Its self-check proves
varint roundtrips and the important two-tree constraint: equal local dictionary
IDs can denote different token text. A shared dictionary, or a text-confirmed
mapping between dictionaries, is necessary before comparing chains. Unsafe
boundaries remain distinct from safe text, and excluded units emit no IDs.

This format does not implement a lineage model. Any total reported here excludes
unimplemented lineage and cannot establish the complete ratchet's cache budget.
The size experiment includes candidate indexes at 32/48/64 bits, lossless
equality dictionary/chain, rows and paths; it makes no speed claim. The initial
linear varint format also needs per-file or sparse offsets before random region
access is efficient. Those offsets must be charged when implemented.

## Acceptance checklist

- [x] Canonical text dictionary and varint-chain self-check.
- [x] Demonstrate unsound local-ID comparison and correct shared-dictionary mapping.
- [x] Roundtrip every eligible token on pinned synthetic and real corpora.
- [ ] Account candidate keys, equality data, rows, paths and required access offsets.
- [x] Demonstrate 32/48/64-bit full-check outcome equivalence on collision fixtures.
- [x] Record feasibility verdict with unimplemented lineage explicitly excluded.

## Measured size

| Corpus | Files | Tokens | Equality bytes | Equality + rows + paths |
|---|---:|---:|---:|---:|
| 1m | 9,998 | 5,930,008 | 16,608,635 | 22,794,324 |
| klin | 80 | 233,276 | 414,057 | 648,623 |
| glaredb | 618 | 573,537 | 1,005,399 | 1,611,156 |
| karakeep | 493 | 263,888 | 619,026 | 903,668 |

At 1M, even the 32-bit candidate index plus equality evidence totals 18,793,177 B,
versus a 3,730,128 B allowance (10% of the frozen 37,301,283 B structural cache).
Rows increase that to 24,723,185 B; paths are already in the candidate index and
are not charged twice. Random-access offsets and lineage are still absent.
Therefore this encoding fails before those additional costs. It is not a proof
that stronger lossless compression cannot fit. Next measure standard block
compression and access costs on these exact bytes before writing another format.

The intentional 32-bit collision adds a false candidate, while all three key
widths preserve the text-confirmed single true region with zero check false
positives or misses. See [collision rows](results-optimizations/compression-collisions.json).

## Standard block compression follow-up

The runnable [compression-blocks.py](results-optimizations/compression-blocks.py)
uses the same frozen normalizer and shared exact-text dictionary. It measures
stdlib zlib and LZMA independently over 64 KiB blocks, retaining raw blocks when
compression expands them. Every block is decoded and compared byte for byte;
canonical chains are separately decoded to exact safe token text and unsafe
boundaries, and row deltas are reconstructed before compression.

Accounting includes compressed dictionary, dictionary-ID offsets, token chains,
row deltas, paths, fixed per-file directories, and sparse token/row offsets every
256 tokens (including the preceding absolute row). Each block also charges its
24-byte access record and one-byte codec selector; the artifact header charges
component offsets and lengths. Candidate indexes are charged separately without
subtracting their existing path storage. The sparse layout is intended to avoid scanning an entire file to reach a region.
Random-access lookup is not implemented or timed; only the stored access metadata
and byte roundtrips are measured. Each decoded block contains at most 64 KiB.

All four pinned corpora passed every canonical-text, unsafe-boundary, row and
compressed-byte roundtrip. Raw results are in
[compression-blocks-1m.json](results-optimizations/compression-blocks-1m.json),
[compression-blocks-klin.json](results-optimizations/compression-blocks-klin.json),
[compression-blocks-glaredb.json](results-optimizations/compression-blocks-glaredb.json),
and [compression-blocks-karakeep.json](results-optimizations/compression-blocks-karakeep.json).

| Corpus | zlib evidence bytes | LZMA evidence bytes | 64-bit candidate + LZMA evidence |
|---|---:|---:|---:|
| 1m | 3,217,015 | 1,594,479 | 5,340,157 |
| klin | 235,036 | 202,627 | 344,857 |
| glaredb | 469,381 | 391,203 | 761,807 |
| karakeep | 294,236 | 249,257 | 421,166 |

Evidence totals include every charged access directory and header. At 1M, LZMA
plus the 32-bit candidate index is 3,779,021 B: 48,893 B above the 3,730,128 B
allowance, before lineage. The 48-bit combination is 4,559,589 B. zlib plus
32-bit candidates is 5,401,557 B. Thus none of these measured combinations
qualifies. Sharing paths with the candidate index would remove only 5,840 B
from the LZMA case (plus one 16-byte component header entry), which still fails.
This is a large reduction from the initial varint encoding, not a proof that
another layout cannot fit. No runtime, memory, random-access, or complete-ratchet
feasibility claim follows from compressed size alone; lineage remains excluded.

### Compressed candidate index follow-up

The same script also compresses the frozen REG2 candidate bytes directly:

```
python3 results-optimizations/compression-blocks.py \
  --indexes /tmp/klin490-measured/1m-32 /tmp/klin490-measured/1m-48 /tmp/klin490-measured/1m-64 \
  --evidence-result results-optimizations/compression-blocks-1m.json
```

[Raw candidate results](results-optimizations/compression-blocks-candidates-1m.json)
include every block directory, selector, and a 32-byte candidate container header.
All original index bytes roundtrip exactly; there is no compressed-index lookup
implementation or speed qualification.

| Candidate width | zlib candidate + evidence | LZMA candidate + evidence |
|---|---:|---:|
| 32 bits | 5,117,407 | 3,472,699 |
| 48 bits | 5,897,938 | 4,253,439 |
| 64 bits | 6,679,311 | 5,034,339 |

The fully block-compressed 32-bit/LZMA combination is below the size allowance
by 257,429 B, before lineage. This changes the size-only verdict: a compact exact
text witness plus 32-bit candidate data can fit this corpus's allowance. It does
not establish the complete ratchet's feasibility: lineage storage, lookup and
update costs, decompression memory, and collision-amplified candidate work remain
unqualified. The 48/64-bit combinations and all zlib combinations still exceed
the allowance. These totals preserve duplicate path metadata rather than assume
a shared unimplemented directory.

### Codec stage qualification

The [five-iteration Python codec profile](results-optimizations/compression-blocks-profile-1m.json)
builds the exact 1M raw components once, checks their frozen digest, and runs
separate worker processes for each codec. Compression timing includes block
encoding and directory construction; decompression timing uses the retained
compressed payloads and directories, with every decoded byte checked against
its original component. Normalization, file preparation, disk I/O, indexed
lookup, lineage, and Rust integration are excluded from these stage timings.

| Codec | Candidate32 + evidence bytes | Median compression ms | Median decompression ms | Worker process high-water bytes |
|---|---:|---:|---:|---:|
| zlib default | 5,117,407 | 399.68 | 17.82 | 149,782,528 |
| LZMA preset 0 | 3,994,651 | 447.07 | 155.96 | 150,290,432 |
| LZMA preset 6 | 3,472,699 | 3,319.77 | 141.33 | 166,592,512 |

Process high-water comes from `getrusage(RUSAGE_SELF).ru_maxrss`, converted to
bytes for the host platform. It includes raw component storage, temporary codec
buffers and interpreter memory. macOS process-launch high-water may also include
inherited parent memory; these values do not establish isolated codec allocation
or incremental production RSS. Random-access lookup remains unimplemented.

Only preset 6 fits the measured size allowance. Its Python compression stage
alone takes about 3.32 seconds, exceeding the combined experiment's entire cold
5% allowance of 1.70 seconds. Faster zlib and preset 0 miss the size allowance.
The combined lower-bound normalization/reuse experiment leaves approximately
254 ms by stage subtraction or 585 ms by paired median wall delta; neither is
an allocated codec budget, because canonical-text capture, index work and
lineage remain absent and two cold pairs already fail the wall-time criterion.
These results therefore do not qualify synchronous compressed evidence for the
cold path. Python timings cannot establish Rust costs, but they give no evidence
that the size-fitting codec will fit the remaining cold work. A production
integration would need a separately measured reader/writer and a complete
accounting of the currently omitted work.

Reproduce with:

```
python3 results-optimizations/compression-blocks.py /tmp/klin490-corpus/1m \
  --indexes /tmp/klin490-measured/1m-32 --profile
```
