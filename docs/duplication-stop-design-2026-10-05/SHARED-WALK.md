# Shared structural and canonical traversal

The shared traversal seam is feasible. No new event state machine is needed:
retain the corrected canonical normalizer's recursive traversal and invoke the
existing structural reference callback before each node. A subtree excluded
from canonical capture is still traversed for structural references. Thus tests,
comments, import bindings and suppressed TypeScript member separators remain
visible to structural extraction, with exactly the original preorder.

Source authority: [`Reading::uses`](../../src/syntax/structural/mod.rs) follows
`harvest`'s query pass and calls the full-tree `syntax::walk`. Its reference
closure is reused unchanged in the isolated integration. The existing corrected
[`Stream::walk`](../duplication-speed-2026-10-05/proto/src/main.rs) owns canonical
ordering, exclusions and exit-time synthetic terminators. The prototype adds a
callback and a small iterative descendant visitor only for excluded branches.
Import provenance still requires the existing direct-root-child import scan;
this is not an additional full-tree traversal. Declaration/query extraction and
its contract-spelling traversals remain unchanged. This does not claim that
structural extraction has only one traversal in total.

## Evidence

- 1,319 exact CLI normalization comparisons pass: 80 klin files, 618 glaredb
  files, 493 karakeep files and 128 synthetic parameter cases. Equality includes
  canonical bytes, rows, safe flags, parse/unsafe counts and selected positions.
- Each normalized case also asserts that the fused callback receives precisely
  the iterative traversal's sequence of `(node.id, start, end)`.
  The expected traversal uses the same descendant helper as skipped branches,
  and a different algorithm from the canonical recursion on retained branches.
  This covers descendants canonical capture skips, not just captured leaves.
- All 12 existing prototype tests pass, including the independent 69-case
  canonical-text region oracle.
- The actual `Reading::uses` callback is fused in a c1805539 isolated checkout.
  With `KLIN_DUP_SHARED_WALK=1`, all 203 existing CLI checks pass:
  dead-symbols 69, layering 65, reachability 52 and structural-sharing 17.
  These cover reference-dependent reports and retained/cache sharing. Callback
  preorder identity plus unchanged closure establishes the structural facts
  seam; passing gate checks alone would not establish every field's equality.
- Strict prototype Clippy passes. No timed measurements were taken.

Raw differential provenance is in
[`shared-walk-equivalence.json`](shared-walk-equivalence.json).

## Reproduction and application order

1. Copy the existing `docs/duplication-speed-2026-10-05/proto` crate to an isolated
   directory. Copy its parent `verify.py` beside it for the CLI oracle.
2. Apply [`shared-walk-proto.patch`](shared-walk-proto.patch) in that crate with
   `patch -p1`. Build/test normally. The normalization CLI intentionally retains
   its preorder proof allocation; **do not time that CLI as the production seam**.
3. Extract `git archive c1805539` into a different isolated checkout. Copy the
   patched crate's `src` to that checkout's `src/dup_research`.
4. Apply [`shared-walk-integration.patch`](shared-walk-integration.patch) in that
   checkout. Run the real binary with `KLIN_DUP_SHARED_WALK=1` to fuse callbacks;
   omit the variable to retain the original structural walker.
5. CLI checks: `KLIN_DUP_SHARED_WALK=1 cargo test --test structural
   --test dead_symbols --test reachability --test layering`.

Prepared locations: `/tmp/klin-stop-shared/proto` and
`/tmp/klin-stop-shared/integrated`. The integrated path uses
`consume_shared_visit` → `normalized_root_with`, avoiding the CLI proof vectors.
It currently computes transient full canonical bytes, token hashes and safety/row evidence,
black-boxing the resulting artifact to retain capture work under optimization.
It discards them after the reference pass, so any wall measurement is only a
shared-capture lower bound. Storage, index, exact matching, lineage and persistent
base/current reuse still need to be charged in the full design.

## Coverage conditions before a complete gate claim

The current structural owner converts raw bytes using `String::from_utf8_lossy`.
The shared callback must not treat converted bytes as original canonical bytes.
Either preserve original bytes and parse from them, or use the existing
[raw-input guarded experiment](../duplication-speed-2026-10-05/results-optimizations/reuse-raw-guard.patch)
to skip affected capture and explicitly mark duplication coverage incomplete.
The latter is a diagnostic limitation, not permission to pass a complete gate.

Likewise structural parsing rejects an entire tree containing syntax errors,
whereas the corrected duplication normalizer can retain safe neighboring units.
A rejected file must be marked incomplete until the design provides equivalent
canonical capture for that file. Unsupported languages, declaration-only/test
exclusions and cache-hit files need explicit coverage accounting. The isolated
integration is a traversal proof, **not a successful complete Stop gate**.

The raw guard patch depends on the old combined accounting experiment and does
not apply directly to this smaller integration. Port its input-boundary check
when composing the full design; do not stack the old combined patch, which would
restore the second normalization traversal.
