# Exact Stop evidence: fast lossless storage experiment

`proto/src/storage.rs` provides `pack` and `unpack` over canonical `FileTokens`.
The format retains exact safe token bytes, unsafe boundary positions, rows,
paths, parser-error status and unsafe-unit counts. Token bytes already contain
language and import provenance. Excluded tokens are not reintroduced.

Dictionary IDs are assigned by sorting complete bytes, with no hash equality.
A prefix-coded dictionary stores every distinct byte string. Chains use literal
IDs or backward references to already decoded IDs, including overlap. The
writer finds one previous occurrence per triple and compares the complete ID
sequence before emitting a reference. Missing a better compression match only
increases bytes; it cannot change decoded content. Rows use runs of equal
unsigned deltas. Both files and tokens retain their original order.

Separate caches must compare decoded exact bytes or explicitly remap their
complete dictionaries. Their numerical IDs are not a cross-tree identity.

The container has a version header and an FNV-64 accidental-corruption checksum.
The reader also checks dictionary order/prefix bounds, integer overflow, token
IDs, reference bounds, row overflow, run sizes, flags and trailing bytes. This
checksum is not authentication and does not promise detection of maliciously
chosen collisions. Cached evidence must remain tied to authoritative source
identity by its caller; this module does not establish source freshness.

The initial reader expands all chains and token bytes. Its decoded memory,
allocation cost and lack of random access are explicit limitations. A per-file
100-million-token ceiling prevents a single unbounded run but is not a whole
application resource policy. Cold cache accounting must include dictionary
construction, encoding and writing, not merely the compression loop. Candidate
index and lineage bytes are separate and must be charged in the final total.

Initial standalone Rust validation passed empty streams, unsafe boundaries,
repeated overlapping sequences, rows, metadata, every truncated cache prefix
and every single-byte mutation in a 1,259-byte fixture. The root prototype CLI
will pin the format roundtrip through its public command seam. Corpus size and
runtime qualification are pending; no budget-fit claim is made.

The format deliberately avoids general-purpose compression and new dependencies.
Its greedy lookup is one candidate per token triple; expand that search only if
measurements show the compression budget misses and the CPU budget has room.

## Serial Rust measurements

The public `storage ROOT` CLI includes the frozen prior normalizer as a source
module and captures exact eligible token bytes once before timing. Each of five
iterations times complete `pack` dictionary construction/encoding and complete
`unpack` separately, then asserts full `FileTokens` equality. No parse, disk I/O,
candidate generation, matching or lineage costs are included in these timings.
Commands and SHA-256 provenance are in `storage-provenance.json`.

| Corpus | Files / tokens | Cache bytes | Median pack ms | Median unpack ms |
|---|---:|---:|---:|---:|
| klin/src | 80 / 233,276 | 437,003 | 33.843 | 6.480 |
| 1m | 9,998 / 5,930,008 | 9,329,778 | 1,429.697 | 165.207 |

All ten corpus roundtrips match every path, exact token, unsafe boundary, row,
unsafe-unit count and parser-error flag. The 1m cache plus frozen 32-bit
candidate index totals **11,514,320 B**, exceeding the **3,730,128 B** allowance
before lineage. Median encoding alone exceeds the provisional 254 ms remaining
stage target. Encoding ranged from 1,413.415 to 3,248.612 ms; memory pressure
was not measured, so that variation has no assigned cause. Full decoded memory
and indexed region access remain unqualified.

This first fast lossless format **fails size and encoding targets**. It is not
proof that every lossless representation fails; it supplies no evidence for
shipping this format or calling the full Stop design qualified.

Durable self-check:

```
cargo run --release --manifest-path proto/Cargo.toml -- storage-check
cargo run --release --manifest-path proto/Cargo.toml -- storage /tmp/klin490-corpus/1m
```

The CLI labels its added candidate cost `candidate32_1m_reference_plus_evidence_bytes`:
that fixed candidate reference is meaningful only for the pinned 1m corpus, not
an actual candidate size for arbitrary roots. Each newly emitted measurement
also reports dictionary, chain, row, metadata and container byte components.
Compiler version and source hashes are in `storage-provenance.json`; normalization
is included from `../duplication-speed-2026-10-05/proto/src/main.rs` relative to
this document's directory. The storage-check is a public CLI command.

## Bounded compression-search follow-up

The measured size miss justified the documented upgrade: retain the last four
or sixteen occurrences per token triple and compare complete IDs to select the
longest lossless reference. This bounds compression search only, never detector
occurrences or equality verification. Both variants are isolated source archives;
the default storage implementation retains its initial one-candidate policy.
Every variant passes the public storage-check and all five whole-corpus exact
roundtrips. Timings were serial, after the cohort matcher's measurements ended.

| Search candidates | Evidence bytes | Candidate32 + evidence | Median pack ms | Median unpack ms |
|---|---:|---:|---:|---:|
| 1 | 9,329,778 | 11,514,320 | 1,429.697 | 165.207 |
| 4 | 8,693,510 | 10,878,052 | 2,339.238 | 171.009 |
| 16 | 8,693,502 | 10,878,044 | 1,922.965 | 171.855 |

The sixteen-candidate layout consists of 1,114,188 B dictionary, 4,143,951 B
chains, 3,159,672 B rows, 275,679 B file metadata and 12 B container. Four
candidates differs only by eight chain bytes. The default's chain size can be
inferred as 4,780,227 B because its dictionary, row and metadata encodings are
unchanged; that component count was not recorded in the original timed run.
Candidate capacity sixteen does not materially improve size over four. Runtime
variation is visible, so these separate five-run samples do not establish a
monotonic CPU ranking. All variants still fail both target budgets before
lineage, normalization, disk I/O or matching. No further refinement was attempted.

`storage-bounded{4,16}-source.tar.gz` and matching provenance JSON preserve
actual variant sources and their executable SHA-256 before mutation. Included
normalizer source is separately identified by SHA-256. The initial default
measurement's exact original main bytes were not archived before formatting
and later CLI additions; its historical executable SHA is retained, while
current source hashes identify the updated CLI rather than the timed source.
Initial result files retain all numeric measurements, but were reformatted and
annotated; the klin candidate total was nulled to remove the misleading fixed
1m cost. Treat the initial timings as diagnostic, with this provenance limit.
