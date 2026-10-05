# O3: reuse derived artifacts for identical inputs within one invocation

Status: isolated implementation compiled; six-call CLI identity probe passed.
Five alternating pairs for each workload are complete; all paired semantic counters
and checksums agree. No production change. This experiment must not infer reuse from the
20,096 callback count: two callbacks with the same path can contain different
base/current bytes.

The existing structural extractor already retains outcomes once per tree using
`Extracted::held`, but the base and working tree own different caches. Its cache
identity and change-set rules must remain intact. Source: [`Extracted::outcome` and
`Extracted::keep`](../../src/syntax/structural/mod.rs), and the shared callback
experiment in [`integrate.py`](integrate.py).

## Smallest experiment

The isolated archive at `/tmp/klin490-opt-reuse` adds a process-lifetime map from
path to input versions. Reuse requires **full byte equality** at the same path.
The language and normalization/minimizer basis are fixed by that binary; changing
path or bytes misses. Same bytes at a different path never reuse because relative
import provenance depends on the importing path. Returning to an earlier version
of the same path may safely reuse that version. No source read or parse is added:
lookup receives the bytes and parsed tree already owned by structural extraction.

Each retained artifact owns the actual normalized token hashes, rows, safety
flags, unsafe-unit/error result and selected `(key, position)` fingerprint vector.
It is not a count-only memoization benchmark. Both no-reuse and reuse runs consume
all selected fingerprints into an observable checksum. Identical checksums are a
measurement sanity check, not a collision-free equality proof. The safety of cache
reuse follows from full input equality, fixed basis and same path.

The isolated CLI probe exercises same input twice, changed source, a renamed path,
TS versus TSX path, then reversion. It asserts actual selected fingerprints exist
and that path-dependent imports and changed bytes produce different normalized
artifacts. It does not prove a persisted cache contract, arbitrary lineage or
eligibility after configuration changes.

## Required measurement

Use the existing `tests/performance.rs` DENSE_1M fixture: five alternating
no-reuse/reuse pairs for warm20, warm100 and cold, same binary with the normalizer
always enabled. Record actual calls, hits, derived input count, paths with multiple
versions, total retained artifact/input payload and end-to-end wall time.
A cache consumes memory even when it misses; the report must charge that cost.
`retained_payload_bytes` counts owned vector capacity, input bytes, path bytes and
artifact/version structures; it excludes HashMap bucket allocation, allocator
headers and synchronization metadata. RSS is required before a total-memory claim.

Source snapshot and raw results belong in `results-optimizations/reuse*`.
No speed claim is made before these runs. Even perfect reuse cannot remove changed
input work, index construction, exact verification or lineage. This standalone
per-run map is an experiment: shipping should attach the artifact to an existing
shared owner if that can supply identical path/bytes/basis lifetime guarantees,
rather than maintain a second long-lived cache.

## Measured result

| Workload | No reuse median | Reuse median | Paired median delta | Actual hits | Retained payload |
|---|---:|---:|---:|---:|---:|
| Warm20 | 1,329.97 ms | 1,322.91 ms | -1.10 ms | 0 | 594,998 B |
| Warm100 | 1,744.29 ms | 1,747.29 ms | +10.11 ms | 0 | 2,944,728 B |
| Cold | 36,973.20 ms | 34,778.80 ms | -2,323.83 ms | 9,998 | 148,456,596 B |

Cold derived inputs fall from 20,096 to 10,098; normalization median falls from
4,406.73 to 2,239.27 ms. Warm base/current bytes differ, so retaining both versions
provides no hits. These runs compare reuse with normalization always enabled,
using the frozen pre-O1 normalizer. They do not measure a combined O1/O2/O3 design
or compare against an uninstrumented hook. Index/verification/lineage remain absent.

Decision: retain reuse as a cold-path opportunity, reject this duplicate
process-lifetime cache as a shipping design. Investigate an existing artifact
owner, lifetime-based eviction and lossless equality evidence before implementing
persistent reuse. The 2.24 s derivation alone still exceeds the previous ~1.61 s
cold allowance; the measured saving cannot establish the 5% budget.
Raw evidence: [reuse.json](results-optimizations/reuse.json),
[probe](results-optimizations/reuse-probe.log), and
[isolated patch](results-optimizations/reuse.patch). Payload excludes container and
allocator overhead; no total RSS or memory-feasibility claim is made.

## Combined O1 follow-up

The same-binary callback-off/on experiment uses O1 (without O2) and reuse. Five
cold pairs give a 3.37% paired median increase, but two pairs exceed 5% (max 5.94%).
Normalization/reuse median falls to 1,447.18 ms; retained payload remains
148,456,596 B. No index or lineage costs are included. See the
[combined ledger](OPTIMIZATIONS.md) and [raw evidence](results-optimizations/combined.json).

The existing `ParsedFile` owns its syntax tree but borrows source text;
`Extracted` retains structural outcomes separately in each tree. It does not
currently own cross-tree full input bytes that this experiment could borrow.
Attaching a second map to it would preserve the copies and memory cost. A real
shared source/artifact lifecycle is required; reuse based only on a hash or an
assumed unchanged path is not an acceptable substitute for correct input identity.

## Raw input identity obligation and guarded probe

Structural `Extracted::outcome` converts bytes through `String::from_utf8_lossy`
before the callback. The reuse experiment compares every byte **it receives**;
that is not necessarily the original file's byte identity. Two TS string
literals containing respectively raw FF and FE become the same replacement
character in structural input, although the frozen raw normalizer produces
unequal canonical text with safe flags. [Counterexample](results-optimizations/reuse-raw-input.json).
Thus the UTF-8 fixture A/B cannot certify integration correctness for all inputs.

A separate isolated guard detects the existing lossy conversion's owned result,
marks that path's duplication evidence incomplete, and prevents it from entering
normalization/reuse. It adds no extra parse or source read. Its CLI probe follows
real `Extracted::outcome` for two invalid files and one valid replacement-character
file: two invalid paths, INCOMPLETE, one valid derived artifact, zero reuse hits.
The original six-input identity probe still passes. This conservatively excludes
an invalid path for the invocation; it does not certify absence of duplication
there. A real integration must forward exact raw bytes with their matching parse,
or propagate this explicit incomplete state to its duplication result.

[Runnable guard probe](results-optimizations/reuse-raw-probe.py),
[result](results-optimizations/reuse-raw-guard.json),
[guard overlay patch](results-optimizations/reuse-raw-guard.patch),
[guard provenance](results-optimizations/reuse-raw-guard-provenance.json).
The overlay applies after combined.patch. It is **not** the measured binary;
archived timing results remain pinned to the earlier combined implementation.
No shipped check or existing structural contract changed.
