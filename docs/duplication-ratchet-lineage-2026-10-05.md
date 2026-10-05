# Duplication ratchet semantics and occurrence lineage, 2026-10-05

This note belongs to #479 under #478. It defines the duplication ratchet before
normalization or implementation is frozen.

The baseline is `5f6f333526d5e41b92191a87eb9ad5852fd76a47`.

The initial draft was produced from issue bodies and repository documents
without using discussion comments. Subsequent adversarial review explicitly
cross-checked the #479 discussion plus #425/#354 and revised the lineage
evidence rules where those counterexamples exposed unsafe inference.

## Decision

Use **lineage-bundle multiplicity** (the same semantic unit the issue discussion calls a **lineage unit / origin component**).

A current canonical clone group is not judged only by its current fingerprint
multiplicity. Its accounting separates:

- certified descendants of base canonical cohorts;
- a definitely-new **count**;
- lineage-unknown evidence;
- and, where available, proven-new physical sites.

Within one current canonical group, all descendants of the **same** base cohort
form one lineage bundle. Each definitely-new occurrence contributes one
additional bundle whether or not its exact physical site is historically
provable. The first bundle is allowed. Every additional bundle is one new
duplication regression.

For one current canonical fingerprint `H`, when lineage is complete:

```text
sources(H)   = distinct base cohorts with >=1 certified descendant in H
new_count(H) = definitely-new occurrence count in H

regressions(H) = max(0, |sources(H)| + new_count(H) - 1)
```

A base cohort is the set of base occurrences that had one compatible canonical
fingerprint. Its legacy duplicate debt is internal to that cohort and may follow
conservatively attributed descendants of that cohort through edits, moves and
splits. It may not be pooled with another cohort.

Every base occurrence contributes one **lineage slot**. A slot may be consumed
at most once by one current occurrence. This slot-conservation rule is what
prevents a diverged legacy member from leaving its old allowance behind for a
new copy to reuse.

This is the selected ratchet model for #478.

## Why canonical multiplicity alone is insufficient

The #48 equation is correct only while canonical identity itself does not move
between fingerprints.

For a fingerprint `H`, #48 uses:

```text
after(H) = B(H) - R(H) + A(H)
excess(H) = max(0, after(H) - max(B(H), 1))
```

That handles same-fingerprint additions, removals and replacements well, but it
cannot distinguish these two transitions:

```text
base:             after:
A = H1            A = H2
B = H1            B = H2
```

and:

```text
base:             after:
A = H1            A = H3
B = H2            B = H3
```

The first is normally one old duplicate cohort changing consistently. The
second is two independent implementations converging and therefore creates one
new unit of duplicate debt.

Current fingerprint counts see only a new group of multiplicity two in both
cases. The missing information is **where the current occurrences came from**.

## Terms

### Canonical fingerprint

The detector-owned, versioned semantic fingerprint used to decide whether two
current implementations are duplicates.

This note does not choose normalization. #480 owns that.

### Base cohort

For a compatible measurement basis, a base cohort is:

```text
C_h = { base occurrence b | fingerprint(b) = h }
```

The cohort id is conceptually:

```text
(detector/normalization version, language, canonical fingerprint)
```

A cohort of size one has no legacy duplicate debt. A cohort of size `k` has
`k - 1` units of held legacy debt.

### Conservative cohort attribution

A current occurrence may be attributed to a base cohort only through evidence
that conserves one base occurrence slot. The ratchet never invents ancestry to
make a current multiplicity convenient.

There are exactly two admitted lineage mechanisms.

1. **Version-compatible same-file structural site identity**

   A before and current occurrence may pair across fingerprint changes only
   when both carry the same supported family-specific structural identity, that
   identity is unambiguous on both sides, and both occurrences use the same
   repository path string on both sides.

   This is the bridge needed for synchronized edits such as `H1 -> H2`.
   It deliberately does **not** extend #425's proven contract across a file
   rename/move. #425's admitted identity is file-scoped; a cross-file move plus
   implementation change therefore remains lineage-unknown until a separate
   identity contract proves that case.

2. **Same-fingerprint replacement, only when competing ancestry is resolved**

   After proven site lineage is consumed, an unmatched removed base occurrence
   of cohort `H` may conserve one unmatched current occurrence whose fingerprint
   is also `H` **only when no unresolved cross-fingerprint ancestry could make
   that current occurrence a descendant of another base cohort**.

   The useful case is a semantically unchanged relocation/replacement:

   ```text
   base:   one H occurrence disappears
   after:  one H occurrence appears
   ```

   When lineage coverage is complete enough to rule out a competing origin, no
   exact historical site needs to be guessed; one `H` slot disappeared and one
   `H` occurrence appeared.

   This fallback is **not** safe merely because the fingerprints match. For
   example:

   ```text
   base:   A=H1  C=H1     B=H2
   after:          C=H1   B=H1
           A removed
   ```

   If `B: H2 -> H1` is unresolved, the removed `A:H1` slot must not be assigned
   to `B` as an exact replacement. Doing so would hide a real convergence of
   independent cohorts. The `H1` group is incomplete unless other evidence
   resolves B's ancestry.

A base occurrence slot can supply at most one current occurrence. No
cross-fingerprint cohort continuation is inferred from "sole successor",
cardinality, line proximity, rename similarity, or another heuristic.

### Definitely-new count versus definitely-new site

The ratchet distinguishes two claims:

- **definitely-new count**: evidence proves that at least `q` occurrences in
  a current clone group cannot be supplied by any compatible base cohort;
- **proven-new site**: evidence additionally proves which physical current
  occurrence is one of those new occurrences.

Absence of a certified ancestor is **not** positive evidence of newness when
the applicable identity is unsupported, ambiguous, incompatible, or otherwise
unable to exclude a predecessor.

Cardinality may prove a definitely-new **count** without proving a definitely-new
**site**. For example, a complete comparable base/current count can establish
that one extra `H` occurrence exists even when two equivalent current sites
cannot be ordered historically. The product may fail the clone group while
using a deterministic representative for presentation, but it must not claim
that representative is certainly "the copy".

### Lineage-unknown

A current occurrence is lineage-unknown when deciding whether it descended
from an old cohort would require guessing an ambiguous, unsupported or
incompatible identity, or when a nominal same-fingerprint replacement has a
competing unresolved cross-fingerprint ancestry.

Unknown lineage is not silently treated as either new or held.

## The equation

Let a current clone group for fingerprint `H` contain:

- `S_H`: the set of distinct base cohorts represented by certified
  descendants in the group;
- `new_count(H)`: the definitely-new occurrence count in the group.

When there are no lineage-unknown occurrences that can affect the judgement:

```text
bundle_count(H) = |S_H| + new_count(H)

regressions(H) = max(0, bundle_count(H) - 1)
```

Equivalent allowance form:

```text
inherited(H) =
    sum over source cohorts C represented in H of (descendants(C,H) - 1)

regressions(H) =
    max(0, occurrences_after(H) - 1 - inherited(H))
```

The bundle form is preferred because it exposes the product semantics directly:
old duplicate debt is internal to one source cohort; a convergence between
cohorts creates new debt.

## Deterministic algorithm

The algorithm operates only when the base/current measurement basis is
compatible.

### 1. Build the delta occurrence sets

For changed paths, obtain before/current function occurrences from the existing
parsed structural path.

For every occurrence retain:

```text
language
canonical_fingerprint
path
range / report site
optional structural identity { version, identified(key) | ambiguous(reason) }
```

Unchanged files are represented by the persisted base counts; they are not
read or parsed.

### 2. Establish structural lineage first

Within each same-path before/current file pair:

1. consider only identities whose version is known and equal;
2. an identity participates only when it is uniquely identified on both sides;
3. pair equal identity keys one-to-one;
4. record the current occurrence's source cohort as the base occurrence's
   canonical fingerprint cohort.

This pairing runs before replacement accounting so a newly converged
implementation cannot steal an old same-fingerprint occurrence from its real
site.

A Git rename/similarity mapping is not sufficient to carry cross-fingerprint
structural lineage. A cross-file `H1 -> H2` transition is lineage-unknown
unless a future separately admitted identity rule proves it.

### 3. Account for implicit unchanged survivors

For each base cohort `C_h`, determine how many base occurrences were outside
the changed paths.

If at least one remains, current group `h` contains source cohort `C_h`
without reading that file.

Changed-path structural pairs that remain at fingerprint `h` also establish
the same source cohort.

Only presence of the source cohort is needed for the regression equation; its
full unchanged occurrence list is not.

### 4. Apply safe same-fingerprint replacement accounting

For each fingerprint `h`, first identify the unmatched removed slots from base
cohort `C_h` and unmatched current `h` occurrences.

A removed `C_h` slot may conserve a current `h` occurrence only when the
available lineage evidence is complete enough to rule out that occurrence being
an unresolved descendant of another base cohort.

When that condition holds, consume replacement capacity one-for-one:

```text
safe_replacements(h) =
    min(unused_removed_slots(C_h), replacement-safe current h occurrences)
```

No site-to-site historical claim is required for those safe replacements.

When the condition does not hold, do not greedily spend `C_h` slots. Preserve
the known source cohorts/new-count evidence and mark the unresolved part of the
group incomplete.

There is **no unique-changed-successor fallback**. A removed `H1` slot does not
follow an `H2` group merely because `H2` is the only observed successor.
That would turn legacy multiplicity into a transferable coupon.

### 5. Classify certain new count, certain sites, and unknown lineage

For each current clone group determine separately:

- distinct certified source cohorts;
- safe same-fingerprint replacement capacity;
- a lower-bound definitely-new occurrence count;
- the subset of those new occurrences whose physical sites are positively
  proven;
- lineage-unknown occurrences or counts whose ancestry remains unresolved.

Do not use "no match found" as proof of newness.

A conservative lower bound is mechanical and does not need guessed matching.
After certified lineage and any independently proven-new physical sites are
removed from the unmatched sets, let:

```text
Q_H = remaining unmatched current occurrences at fingerprint H
S_H = unused removed base slots from the base H cohort
U_H = unused removed base slots from other fingerprints whose
      cross-fingerprint destination is unresolved

cardinality_new_lb(H) = max(0, Q_H - S_H - U_H)

definitely_new_count(H) =
    proven_new_sites(H) + cardinality_new_lb(H)
```

`U_H` is intentionally pessimistic: for the lower-bound calculation, every
unresolved old slot is allowed to explain one otherwise-unmatched `H`
occurrence even when that ancestry would be unlikely. This can under-report
while measurement is incomplete, but it cannot manufacture a BLOCK finding.

The same unresolved slot may appear as possible capacity in the lower-bound
calculation of more than one current fingerprint. That does **not** assert that
it has multiple descendants; it only means the individual group lower bounds
are conservative. If exact allocation matters to a complete verdict, the
measurement remains INCOMPLETE rather than solving a heuristic global matching
problem.

When `U_H = 0`, ordinary same-fingerprint cardinality gives the familiar exact
replacement/new-copy arithmetic. When `U_H > 0`, the lower bound can still
preserve a certain FAIL if current multiplicity exceeds every plausible legacy
slot.

### 6. Judge each current canonical group with a conservative lower bound

Let:

```text
certain_bundles(H) =
    |certified source cohorts represented in H|
    + definitely_new_count(H)

certain_regressions(H) =
    max(0, certain_bundles(H) - 1)
```

Unknown lineage can increase the final bundle count, but it cannot erase
already-proven independent origins.

Therefore:

- preserve `certain_regressions(H)` as valid FAIL **regression units** even if
  other lineage in the group remains unresolved;
- render those units as site findings only where site attribution is proven;
  when only cardinality is proven, a group-level finding may carry the proven
  regression-unit count rather than fabricating several historical site claims;
- independently mark required duplication measurement INCOMPLETE when a valid
  ancestry assignment could change the total count, attribution, or absence of
  additional regressions;
- never convert incomplete evidence to PASS;
- never discard a proven FAIL merely because additional regressions are
  uncertain.

This follows #354 directly: a partial measurement may contain valid failing
evidence; judgment and measurement completeness are separate axes.

If `certain_regressions(H) == 0` and unresolved lineage could create a
regression, the group has no code-quality FAIL yet and required measurement is
INCOMPLETE.

A singleton current group cannot itself contain duplicate debt, though an
unresolved extraction hole may still make the overall measurement incomplete.

### 7. Deterministic finding assignment

The semantic unit is the clone-group regression, not a claim that one
historical site was certainly "the copier".

Root preference is explicit:

1. if any inherited certified source cohort exists, choose an inherited cohort
   as the free root;
2. prefer an exact-fingerprint inherited cohort;
3. then prefer the inherited cohort with the most attributed current
   occurrences;
4. use stable cohort key and current `(path, start, end)` only as tie-breaks;
5. only when no inherited source exists may a new singleton/count supply the
   free root.

For a proven-new site, anchor the corresponding finding there.

When cardinality proves one or more regression units but not which physical
sites are new, report a **group regression** carrying the proven unit count and
use one deterministic current representative only as a presentation location.
Do not emit multiple invented site identities merely to mirror the numeric unit
count. The wording must not say that representative certainly copied another
site.

When the regression is convergence of old cohorts, wording should say that
previously distinct implementations now converge rather than falsely accusing
one side of copying the other.

## Truth table

| Case | Base -> current | Bundle accounting | Result |
| --- | --- | --- | --- |
| First implementation | none -> one new `H` | 0 source + 1 new = 1 | PASS |
| Second/new copy | one `H` -> survivor + one new `H` | 1 source + 1 new = 2 | FAIL 1 |
| Third copy over legacy pair | base cohort `H` size 2 -> same cohort descendants + one new | 1 source + 1 new = 2 | FAIL 1 |
| Two identical new functions | none -> two new `H` | 0 source + 2 new = 2 | FAIL 1 |
| Synchronized edit of duplicate pair/group | one base cohort `H1` -> its descendants all become `H2` | 1 source + 0 new = 1 | PASS / held |
| Synchronized edit + third new copy | old cohort `H1` -> descendants at `H2`, plus new `H2` | 1 source + 1 new = 2 | FAIL 1 |
| Diverged member cannot leave coupon behind | `A,B:H1 -> A:H1, B:H2, C:H1(new)` with `B` lineage proven | `B` already consumes its slot at `H2`; `C` is new beside the `H1` origin | FAIL 1 |
| Removed slot and sole changed successor | `A,B:H1 -> A:H2, C:H2`, with `A` proven and `B` removed but no lineage for `C` | sole-successor shape is not ancestry proof | INCOMPLETE unless `C` lineage/newness is independently resolved |
| One legacy member edited to uniqueness | one old cohort splits between `H1` and `H2` | each current group has one source cohort | PASS |
| Two unrelated unique implementations converge | `H1` + `H2` -> both `H3` | 2 source cohorts = 2 | FAIL 1 |
| Delete + equivalent add | removed `H` occurrence replaced by new-site `H` | same-fingerprint replacement keeps one source cohort | PASS |
| Delete one of pair + equivalent add | legacy cohort `H` remains size 2 | one source cohort | PASS |
| Delete unique + add two equivalent | one base `H` -> two current `H` | one replacement source + one new = 2 | FAIL 1 |
| File rename/move, same fingerprint | `H` -> `H` elsewhere | structural identity or same-fingerprint replacement | PASS |
| Function rename, same fingerprint | `H` -> `H` with renamed declaration | same-fingerprint replacement | PASS |
| Same-file implementation edit | `H1` -> `H2` at a uniquely identified same-file site | certified structural lineage | PASS/held when appropriate |
| Cross-file move plus implementation edit | `H1` -> `H2` in another path | #425 does not prove cross-file structural lineage | INCOMPLETE when ancestry affects duplication judgment |
| Group split | one cohort `H1` -> descendant groups `H2`, `H3`, ... | every new group contains the same single source cohort | PASS; may reduce debt |
| Split + removed-slot allocation matters | one cohort has proven descendants in `H2` and `H3`, plus a removed slot and an unmatched occurrence on a branch | slot has multiple plausible successor branches | INCOMPLETE if allocation changes regression count |
| Two old duplicate groups merge | cohort `H1` + cohort `H2` -> one `H3` | 2 source cohorts = 2 | FAIL 1 |
| `m` old groups merge | `m` distinct source cohorts -> one group | `m` bundles | FAIL `m - 1` |
| Partial survival of legacy group | any subset of one old cohort survives together | one source cohort | PASS |
| Partial survival + genuinely new member | one old cohort + one new member | 1 source + 1 new = 2 | FAIL 1 |
| Ambiguous site identity, unchanged fingerprint | same-fingerprint cardinality may prove a count only if competing ancestry is ruled out | do not spend a replacement slot through ambiguity | FAIL lower bound and/or INCOMPLETE as evidence permits |
| Ambiguous site identity, changed fingerprint | ancestry could change bundle count | unknown lineage | INCOMPLETE, not normal PASS/FAIL |
| Replacement-laundering attack | `A,C:H1; B:H2 -> C:H1; B:H1`, with `A` removed and `B` ancestry unresolved | removed `A` slot cannot be assigned to `B` | INCOMPLETE, or FAIL 1 if `B:H2 -> H1` is certified |
| Proven FAIL plus unknown extra lineage | one certified old cohort + one definitely-new origin + one unknown origin in current `H` | at least 2 certain bundles | FAIL at least 1 **and** measurement INCOMPLETE |
| Detector/normalization version mismatch | base/current basis incompatible | no legal cohorts to compare | rederive base under current basis or INCOMPLETE; never manufacture new/resolved |
| Unsupported/incomplete extraction | required occurrences may be missing | measurement hole | INCOMPLETE; never clean |

## Split and merge semantics

The bundle equation gives useful algebraic guarantees.

### Split

Suppose one base cohort `C` of size `k` splits across `p` current
fingerprints.

Every resulting current group that contains descendants of `C` sees exactly
one inherited bundle from `C`.

Therefore a split by itself creates zero regressions.

The old cohort's `k - 1` held duplicate debt does not need to be "spent".
If the split makes implementations unique, debt simply disappears.

### Merge

Suppose one current fingerprint contains descendants from `m` distinct base
cohorts and no genuinely new occurrence.

Then:

```text
regressions = m - 1
```

This is true regardless of how large those old cohorts were.

Therefore:

- two old unique implementations converging creates one regression;
- two old duplicate groups merging also creates one regression;
- a huge legacy duplicate cohort cannot donate allowance to an unrelated
  cohort.

This is the core anti-laundering property.

## Why old duplicate allowance is not a transferable coupon

A naive "total legacy excess" budget is unsafe.

For example, if a base tree has one duplicate pair elsewhere, resolving that
pair could free one global allowance and hide a brand-new unrelated pair in the
same change.

Lineage-bundle multiplicity forbids that because allowance is never stored as a
global integer.

Held debt is attached to one base canonical cohort. It can follow only:

- certified cross-fingerprint descendants of that cohort; or
- same-fingerprint one-for-one replacements of removed occurrences when no
  unresolved competing ancestry could consume those slots.

It cannot cross from cohort `H1` to cohort `H2` merely because the total
number of duplicates in the repository stayed flat, because `H2` is the
cohort's only observed successor, or because current multiplicity happens to
equal old multiplicity.

## Ambiguity and incomplete measurement

The ratchet must not guess occurrence ancestry.

### Identity ambiguity

Examples include duplicate structural keys, anonymous/computed sites or a
family/language with no admitted identity rule.

Ambiguity is handled in three layers:

1. certified same-file structural lineage is consumed first;
2. same-fingerprint cardinality may still prove a lower-bound new count, but a
   replacement slot is spent only when competing cross-fingerprint ancestry has
   been ruled out;
3. unresolved cross-fingerprint lineage is never inferred.

If unresolved lineage could add regressions, required duplication measurement
is INCOMPLETE. Any regressions already proved by independent origin bundles
remain valid FAIL evidence at the same time.

This follows #475's product rule:

```text
unmeasured != clean
unknown != failure
```

and #425's rule that structural identity is optional, versioned and
conservative.

### Unsupported syntax / parse holes / resource caps

If any qualifying occurrence in required scope may be missing, the detector
must not publish the observed clone set as complete.

The result is INCOMPLETE with the concrete hole.

## Detector and normalization versions

A canonical cohort is meaningful only inside one compatible measurement basis.

The comparison basis must cover at least:

```text
duplication detector semantics version
normalization version
language adapter version where semantic output changes
parser/extractor identity where semantic output changes
scope/exclusion semantics
minimum-size rule
```

Execution-only data such as timestamps does not belong in the semantic basis.

If base/current basis differs:

1. do not compare fingerprints or structural lineage as ordinary debt;
2. do not emit normal new/resolved duplication findings;
3. if possible, rebuild/rederive the base duplication index using the current
   detector semantics;
4. if that cannot be completed, mark required duplication measurement
   INCOMPLETE/non-comparable.

A detector upgrade therefore cannot manufacture a regression merely because
canonical fingerprints changed.

## Move and replacement policy

The product semantics deliberately distinguish same-semantic replacement from
cross-semantic ancestry.

### Same canonical implementation

Removal of one `H` occurrence plus addition of one `H` occurrence may be
treated as a replacement slot retaining source cohort `C_h` **only when
competing cross-fingerprint ancestry for the current `H` occurrence has been
ruled out**.

Replacement accounting runs only after proven divergent site successors have
taken their own base slots, so a diverged member cannot leave a second allowance
behind. If competing ancestry remains unresolved, the slot is not spent and the
affected group remains incomplete.

This covers:

- file move;
- function rename when the declaration name is excluded from the canonical
  body fingerprint;
- delete + equivalent add;
- relocation among duplicated legacy copies.

Multiple replacements are bounded one-for-one by removed occurrences. An
additional current occurrence beyond replacement supply is new.

### Changed canonical implementation

When `H1 -> H2` inside the same physical file, lineage may carry only through
the admitted unique version-compatible structural identity.

A cross-file move plus implementation change is not certified by #425's
file-scoped identity and remains lineage-unknown when ancestry affects the
duplication judgment.

A removed `H1` slot never follows `H2` merely because `H2` is the only
observed successor group.

This is intentionally more conservative than retrospective clone genealogy:
for a blocker, uncertain ancestry becomes incomplete evidence rather than
portable legacy allowance.

## Persisted metadata and O(delta) warm work

The selected model does **not** require a repository-wide occurrence genealogy
database.

The compact persisted base duplication index can remain close to #48's shape.

Per base fingerprint/cohort:

```text
measurement-basis / detector version
structural-identity version when used for cross-fingerprint lineage
language
canonical fingerprint
base occurrence count
deterministic representative location
verification evidence needed to rule out hash collision before BLOCK
```

For changed files, the existing before/current parsed structural facts provide:

```text
fingerprint
optional structural identity
path/range
```

No structural identity for unchanged occurrences needs to be persisted merely
for this ratchet.

### Warm algorithmic shape

Let:

- `D_f` be changed files;
- `D_o` be qualifying before/current occurrences in changed files;
- `I` be base fingerprint index entries.

Expected warm work is:

```text
O(D_f + D_o log I)
```

with sorted-array lookup, or expected `O(D_f + D_o)` with an in-memory index.

Memory added during one run is:

```text
O(D_o + touched_fingerprints)
```

There is:

- no whole-tree source scan;
- no unchanged source read;
- no pairwise expansion of clone relationships;
- no `O(k^2)` work for a cohort with `k` copies.

High multiplicity affects counts, not relation enumeration.

This is compatible with #478/#48's desired O(delta)-shaped warm path. #482
still has to measure the actual constants.

## Candidate models considered

### 1. Canonical multiplicity per current fingerprint

**Reject as the complete ratchet. Retain as a same-fingerprint fast path.**

Strengths:

- simple;
- compact;
- excellent O(delta) shape;
- naturally handles first/second/third copies and exact replacements.

Failure:

- synchronized edits of one old clone group and convergence of independent old
  implementations are observationally identical after the fingerprint changes.

### 2. Global duplicated-occurrence/excess budget

**Reject.**

It can preserve total duplicate debt while moving that allowance to unrelated
new code. This is exactly the transferable-coupon failure #479 must prevent.

### 3. Pairwise duplicate relations / edges

For a group of size `k`, there are `k(k-1)/2` duplicate edges.

**Reject as the ratchet unit.**

It correctly notices new relationships, but:

- a new third copy over a legacy pair creates two new edges even though the
  product wants one new-excess finding;
- high multiplicity creates quadratic state/work if represented directly;
- edge selection complicates deterministic reporting without improving the
  product judgement.

The bundle model captures the useful "new relationship between independent
lineages" idea without pairwise expansion.

### 4. Clone-group lineage with pooled inherited capacity

**Reject.**

If a merged current group simply sums the old capacities of all parent groups,
two independent old groups can merge without a regression. That launders
cross-group convergence.

### 5. Lineage-bundle multiplicity

**Select.**

It is the smallest model found that simultaneously gives:

- held synchronized changes;
- one finding for a third copy over legacy debt;
- one finding for two new identical functions;
- detection of unrelated convergence;
- explicit split/merge behavior;
- exact same-fingerprint move/replacement semantics with ambiguity guards;
- no global or unique-successor allowance transfer;
- O(delta)-shaped work.

## Invariants

The selected model should be implemented/tests specified around these
invariants.

### I1. Slot conservation

Every base occurrence slot is consumed at most once.

### I2. Same-cohort consistency

If every current occurrence in a clone group descends from one base cohort and
there are no new occurrences, the group creates no regression.

### I3. New-copy cardinality

Adding `q` genuinely new copies to descendants of one source cohort creates
exactly `q` regressions.

### I4. Fresh-group cardinality

Creating `q >= 1` identical new implementations from no source cohort creates
exactly `q - 1` regressions.

### I5. Merge cardinality

A current group made only from `m >= 1` independent source cohorts creates
exactly `m - 1` regressions.

### I6. Split monotonicity

Splitting one source cohort across any number of current fingerprints creates
zero regressions by itself.

### I7. Replacement conservation

One removed occurrence may conserve at most one current occurrence. A same-
fingerprint replacement is legal only when competing cross-fingerprint ancestry
has been ruled out; no changed-successor heuristic may spend the slot.

### I8. Origin separation

Two different base cohorts never become one held allowance merely because their
current fingerprints match.

### I9. No global compensation

Resolving or splitting duplicate debt elsewhere cannot pay for a new origin in
this clone group.

### I10. No guessed lineage

No ambiguous/unsupported cross-fingerprint identity is converted to lineage
merely to make the verdict pass or fail.

### I11. Positive evidence survives incompleteness

Proven independent origin bundles produce their lower-bound FAILs even when
other lineage in the same measurement is unresolved.

### I12. Count is not site identity

Cardinality may prove a definitely-new count without proving which current
physical occurrence is new. Reporting never upgrades count evidence into a
historical-site claim.

### I13. Same-file identity boundary

Cross-fingerprint structural lineage uses only the scope actually proven by the
admitted identity contract. #425 does not certify a cross-file move plus body
edit.

### I14. Basis compatibility before lineage

Measurement-basis compatibility is checked before cohort or site matching.

## Implications for #48

The following #48 ideas survive:

- canonical function/method fingerprints;
- language-separated matching;
- compact persisted base fingerprint counts;
- changed-file before/current extraction;
- safe same-fingerprint remove/add replacement accounting after competing ancestry is resolved;
- one-for-one occurrence-slot conservation;
- deterministic assignment of only new excess findings;
- no repository duplicated-line percentage;
- O(delta)-shaped warm work.

The per-fingerprint multiplicity equation is no longer the complete ratchet.
It becomes the same-fingerprint special case inside a lineage-aware judgement.

The eventual implementation ticket must add:

- optional versioned structural occurrence identity for cross-fingerprint
  lineage;
- source-cohort bundle accounting with replacement safety against competing ancestry;
- explicit UNKNOWN/INCOMPLETE handling when lineage evidence needed for the
  judgement is ambiguous;
- basis/version compatibility for the duplication index.

## Implications for #478

#478 should use this selected ratchet contract while #480 calibrates what the
canonical fingerprint means.

The next research questions are therefore narrower:

1. can Rust and TypeScript canonicalization produce blocker-level precision?
2. which occurrence identities are available from the shared structural path
   at essentially zero extra parse cost?
3. how often would cross-fingerprint lineage be ambiguous in the calibration
   corpus?
4. does the resulting candidate resist cheap appeasement (#481)?
5. can the bundle/index accounting meet the measured Stop budgets (#482)?

#479 does not select normalization, minimum token count, placement or final
BLOCK/REVIEW strength.

## Prior-art check

Clone-evolution literature uses clone genealogies specifically to distinguish
consistent changes, inconsistent changes, additions, removals and group splits
over versions. That supports treating history/lineage as a first-class concept
rather than trying to infer all semantics from one snapshot.

The classic genealogy work traces new clone groups to ancestor groups and
models consistent/inconsistent evolution:

- Kim et al., *An Empirical Study of Code Clone Genealogies*:
  https://dada.cs.washington.edu/research/tr/2005/04/UW-CSE-05-04-01.pdf
- later work continues to use clone-group evolution/consistent-change
  terminology:
  https://doi.org/10.1016/j.jss.2017.08.045

Klin deliberately uses a stricter rule than general clone-genealogy tools:
heuristic textual/location similarity is not sufficient to carry blocking
ratchet debt. Cross-fingerprint ancestry needs admitted structural identity;
otherwise the result stays unknown/incomplete.

## Acceptance-criteria mapping

- Truth table covers every required case: **yes**, above.
- Synchronized edits remain held: **yes**, I2.
- Unrelated convergence is detected: **yes**, I5.
- Third copy over legacy debt is detected exactly once: **yes**, I3.
- Move/replacement semantics are explicit: **yes**.
- Group split/merge semantics are explicit: **yes**, I5/I6.
- Ambiguous identity is never guessed: **yes**, I10/I13.
- Detector-version mismatch cannot manufacture normal findings: **yes**, I14.
- Old allowance cannot be spent on unrelated code: **yes**, cohort-local debt.
- Complexity and persisted metadata are estimated: **yes**.
- #478 must be updated with this selected model: **required companion update**.


## Adversarial pass, 2026-10-05

A focused adversarial pass after the initial #479 discussion changed one part
of the model.

### A1. Unique-successor slot transfer is not conservative

Rejected rule:

```text
base:  A=H1  B=H1
after: A=H2(proven)  C=H2(unproven)
```

The tempting rule is to let the removed `B` slot follow `H2` because `H2`
is the only proven successor fingerprint. That is not safe. `C` may be:

- a relocation/continued edit of old `B`; or
- a genuinely new copy of edited `A`.

Those histories have different ratchet meaning and current evidence does not
distinguish them.

The failure amplifies with multiplicity:

```text
base:  100 occurrences of H1
after: 1 proven H2 successor + 99 unproven new H2 occurrences
```

A unique-successor allowance could mark all 99 as held merely because 99 old
slots disappeared. That is a transferable coupon inside one cohort and violates
the acceptance criterion that legacy allowance cannot launder genuinely new
duplication.

Decision: **remove the unique-successor fallback**. Changed-fingerprint lineage
requires certified occurrence/site lineage.

### A2. Greedy exact replacement can also hide ambiguity

Even exact-fingerprint replacement must run after accounting for unresolved
claims on the same base slots.

```text
base:  A=H1  B=H1
after: A=H2(ambiguous lineage)  C=H1  D=H1
```

If `A=H2` consumed one old slot, only one slot remains for `C,D`, so one
`H1` occurrence is new. If it did not, both `C,D` could be replacements.

Therefore an implementation must not greedily spend both old slots on `C,D`
and then treat `A=H2` as an unrelated singleton. When unresolved
cross-fingerprint ancestry competes for slots and allocation changes the
duplicate verdict, the affected measurement is INCOMPLETE.

### A3. Known regressions and incomplete measurement are orthogonal

Ambiguity can make the **total** regression count unknown without erasing a
regression already proven from distinct origins.

For a current group `H`, define the guaranteed floor:

```text
known_origins(H) = distinct certified source cohorts
                 + definitely-new singleton origins

known_regressions(H) = max(0, |known_origins(H)| - 1)
```

If `known_regressions(H) > 0`, those regressions are valid even when additional
lineage-unknown occurrences mean the complete count is unknown. In that case
klin may surface the proven finding(s) **and** report
`measurement=INCOMPLETE`; #475 already treats judgement and completeness as
orthogonal axes and gives INCOMPLETE exit precedence.

V1 does not need a max-flow/optimal ambiguity solver. It only needs to avoid
suppressing already-proven merges/new origins while being honest that more may
exist.

### A4. Result

The core lineage-unit equation survives, but with a stricter lineage boundary:

- cross-fingerprint debt moves only through certified site lineage;
- same-fingerprint replacement remains an explicit product special case;
- replacement slots cannot be greedily spent across unresolved competing
  ancestry;
- a sole successor fingerprint is insufficient evidence;
- proven regressions may coexist with incomplete total measurement.
