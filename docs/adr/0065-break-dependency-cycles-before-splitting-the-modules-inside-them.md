# Break dependency cycles before splitting the modules inside them

> Governs the architecture foundation of #370. The inventory that applies it
> is `docs/architecture-cycles-2026-10-01.md`.

At `bffa48cd`, `klin layering` on klin's own tree holds 282 cyclic edges.
Two strongly connected components hold them: one of 46 of the 64 Rust
modules, and one of the 5 modules under the host directory. The base holds
these edges, so the gate passes. A module split inside a component gives new
module identities, and each edge from a new module back into the component is
a new cyclic edge. The `acyclic` gate fails on it. So the order of the work
decides whether the work can land.

## The decision

**Break dependency cycles before broadly splitting modules inside them. Push
shared contracts down into leaf modules, lift composition up, and retire the
held cycle debt monotonically until `klin layering` reports zero.**

During cycle retirement:

1. A split that only makes a file smaller is not architectural progress. A
   split inside a component is progress only when the split itself removes
   cyclic edges.
2. Every cyclic edge is assigned to one named implementation ticket before
   implementation starts. An edge that stops being cyclic when another ticket
   removes its path back is *passive*, and that ticket owns it.
3. Compatibility re-exports are forbidden. A moved item is moved only when
   every caller imports it from its new owner. Spec 8.2.1 resolves a Rust path
   to the deepest module it names and does not follow a `pub use`, so a
   re-export in the old owner keeps the old edge. A parent module that holds
   `mod` declarations re-exports nothing that a cycle-breaking move took out
   of it.
4. Until the check contract and the catalogue are split (#376), no new
   module imports `crate::check`. The `check` module holds `CATALOGUE`, so a
   new module that imports it, and that a check imports, sits inside the
   component. Code that needs `Context`, `Sink` or `Records` stays in an
   existing high module or in the check that calls it.
5. The held cyclic count may never rise. No new cyclic edge is accepted to
   make the migration easier.
6. Each completed cycle-breaking ticket lowers the held cyclic count. Its PR
   records the count before and after.
7. `klin layering` is authoritative. A one-off import graph or SCC script is
   diagnostic only.
8. Broad decomposition and final directory normalization wait until the
   count is zero.
9. After zero, `acyclic: true` in `klin.json` is the permanent architecture
   ratchet. Every later split or move stays acyclic, because the gate fails
   on any new cycle it can resolve.

## Known costs

- A cycle between a parent module and its own child files counts the same as
  any other cycle (spec 8.2.1). An idiomatic Rust dispatch, in which a parent
  names its children and the children use the parent's items, is cyclic.
  The inventory finds 24 such edges, and #427 owns them. To retire them, the
  shared items move to a sibling module that the parent and the children
  both import.
- Without re-exports, a move changes every caller's import. The diffs are
  wider than a move with a re-export.
- A ticket that removes no cyclic edge cannot land in this foundation, even
  when it improves the code.

## Rejected

- **Split the large modules first.** Each new module inside the component
  adds cyclic edges, and the `acyclic` gate fails on each of them.
- **Accept new cyclic debt during the migration.** The held count then stops
  being a monotonic measure of progress.
- **Keep the old paths with re-exports.** klin would still count the old
  edges, and the count would not fall.
