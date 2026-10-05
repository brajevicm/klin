# Duplication ratchet semantics and occurrence lineage, 2026-10-05

This note belongs to #479 under #478. It defines the duplication ratchet before
normalization or implementation is frozen.

The baseline is `5f6f333526d5e41b92191a87eb9ad5852fd76a47`.

The research intentionally used issue bodies and repository documents as inputs.
Discussion comments were not used.

## Decision

Use **lineage-bundle multiplicity** (the same semantic unit the issue discussion calls a **lineage unit / origin component**).

A current canonical clone group is not judged only by its current fingerprint
multiplicity. Instead, every current occurrence is classified as one of:

- a certified descendant of one base canonical cohort;
- definitely new;
- lineage-unknown.

Within one current canonical group, all descendants of the **same** base cohort
form one lineage bundle. Every definitely-new occurrence is its own bundle.
The first bundle is allowed. Every additional bundle is one new duplication
regression.

For one current canonical fingerprint `H`, when lineage is complete:

```text
sources(H) = distinct base cohorts with >=1 certified descendant in H
new(H)     = definitely-new current occurrences in H

regressions(H) = max(0, |sources(H)| + new(H) - 1)
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

A current occurrence may be attributed to a base cohort only through a rule
that conserves one base occurrence slot. The ratchet does not need to claim an
exact historical site when cohort-level evidence is sufficient.

There are three lineage mechanisms.

1. **Version-compatible structural site identity**

   A before and current occurrence may pair across fingerprint changes when
   both carry the same supported family-specific structural identity, that
   identity is unambiguous on both sides, and the path relationship is known
   (same path or an explicit rename/move mapping).

   This is the strongest bridge for synchronized edits such as `H1 -> H2`.
   The rule does not admit raw line proximity or a guessed ordinal as identity.

2. **Same-fingerprint replacement**

   After proven site lineage is consumed, an unmatched removed base occurrence
   of cohort `H` may be consumed by one unmatched current occurrence whose
   fingerprint is also `H`.

   The exact historical site does not need to be guessed. The only fact needed
   by this ratchet is that one occurrence of the same canonical implementation
   disappeared and one appeared. This preserves #48's move/replacement
   behavior.

3. **Unique changed-fingerprint continuation**

   After the first two mechanisms, a still-unused removed slot from base cohort
   `C_h` may be attributed to an unmatched occurrence in a changed-fingerprint
   successor group only when all of these hold:

   - the cohort has at least one **proven** current descendant;
   - it has no current continuation at the original fingerprint `h`;
   - every proven current descendant of the cohort belongs to the same one
     successor fingerprint `H'`;
   - the slot is consumed by an occurrence in that same `H'` group.

   This is cohort-level continuation, not a guessed site-to-site match. It
   handles, for example:

   ```text
   base:  A=H1  B=H1
   after: A=H2  C=H2
   ```

   when `A -> A` is proven and `B` was removed: the unused `B` slot may
   follow the sole proven successor group `H2`, so the legacy pair remains
   held.

   If the cohort has multiple successor groups, or no proven successor at all,
   no changed-fingerprint slot is assigned by this rule.

A base occurrence slot can supply at most one current occurrence. No other
cross-fingerprint attribution is inferred.

### Definitely new

A current occurrence is definitely new when, after certified structural
pairing and same-fingerprint replacement accounting, no compatible base
occurrence can supply it and its creation is positively established by the
available identity/cardinality evidence.

For same-fingerprint multiplicity growth, cardinality itself can prove how many
current occurrences are new even if several equivalent sites are otherwise
indistinguishable.

### Lineage-unknown

A current occurrence is lineage-unknown when deciding whether it descended
from an old cohort would require guessing an ambiguous, unsupported or
incompatible identity.

Unknown lineage is not silently treated as either new or held.

## The equation

Let a current clone group for fingerprint `H` contain:

- `S_H`: the set of distinct base cohorts represented by certified
  descendants in the group;
- `N_H`: the multiset of definitely-new occurrences in the group.

When there are no lineage-unknown occurrences that can affect the judgement:

```text
bundle_count(H) = |S_H| + |N_H|

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

Within each mapped before/current path pair:

1. consider only identities whose version is known and equal;
2. an identity participates only when it is uniquely identified on both sides;
3. pair equal identity keys one-to-one;
4. record the current occurrence's source cohort as the base occurrence's
   canonical fingerprint cohort.

This pairing runs before same-fingerprint replacement so a newly converged
implementation cannot steal an old same-fingerprint occurrence from its real
site.

### 3. Account for implicit unchanged survivors

For each base cohort `C_h`, determine how many base occurrences were outside
the changed paths.

If at least one remains, current group `h` contains source cohort `C_h`
without reading that file.

Changed-path structural pairs that remain at fingerprint `h` also establish
the same source cohort.

Only presence of the source cohort is needed for the regression equation; its
full unchanged occurrence list is not.

### 4. Apply same-fingerprint replacement accounting

For each fingerprint `h`:

- count unmatched removed base slots from cohort `C_h`;
- count unmatched current occurrences with fingerprint `h`;
- consume up to the smaller count as replacement descendants of `C_h`.

No site-to-site guess is required because every consumed base occurrence belongs
to the same cohort and the current canonical implementation is unchanged.

### 5. Carry unused slots only to a unique changed successor

For each base cohort with still-unused removed slots after step 4:

1. collect its proven current descendants from structural lineage;
2. if any current descendant remains at the original fingerprint, do not carry
   a removed slot across fingerprints;
3. if the proven changed descendants occupy exactly one non-empty successor
   fingerprint group `H'`, consume unused removed slots one-for-one against
   unmatched current occurrences in `H'`;
4. if proven descendants split across multiple successor groups, do not choose a
   branch.

The third step conserves legacy multiplicity inside one continuing cohort
without turning an unused slot into a repository-wide coupon.

### 6. Classify remaining current occurrences

For each unmatched current occurrence:

- if no still-valid base-slot attribution can supply it, classify it as
  definitely new;
- if an unresolved cross-fingerprint allocation could supply it, classify it
  as lineage-unknown.

Do not invent a best-effort cross-fingerprint match. In particular, zero proven
successors is not a "unique successor": a fully changed-fingerprint replacement
with no safe lineage remains UNKNOWN/INCOMPLETE when the distinction matters.

### 7. Judge each current canonical group

If a current group has no judgement-relevant lineage-unknown occurrence:

```text
R(H) = max(0, |distinct source cohorts| + |definitely new occurrences| - 1)
```

Emit exactly `R(H)` regression units.

If a group contains lineage-unknown occurrences and a different valid ancestry
assignment could change whether or how many regressions exist, that group's
measurement is UNKNOWN and required duplication measurement is INCOMPLETE
under #475/#354.

A singleton current group is not duplicate debt, so unknown lineage inside a
singleton does not by itself create a duplication finding.

### 8. Deterministic finding assignment

The semantic unit is the group regression, not a claim that one historical site
was certainly "the copier".

For reporting, choose the root origin deterministically with this preference:

1. an exact-fingerprint carried base cohort;
2. another carried base cohort rather than a brand-new singleton;
3. the carried cohort with the most attributed current occurrences;
4. stable cohort key and then current `(path, start, end)` as tie-breaks.

Emit one finding for each remaining origin unit.

This preference makes a genuinely new singleton the finding site whenever an
inherited origin exists, while keeping merge findings stable under high
multiplicity.

When the regression is a convergence of old cohorts, wording should say that
previously distinct implementations now converge rather than falsely accusing
one site of having copied the other.

For a genuinely new occurrence over one inherited cohort, the new site is the
natural finding anchor.

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
| Removed slot follows sole changed successor | `A,B:H1 -> A:H2, C:H2`, with `A` proven and `B` removed | one proven successor group; `C` consumes the unused `B` slot | PASS |
| One legacy member edited to uniqueness | one old cohort splits between `H1` and `H2` | each current group has one source cohort | PASS |
| Two unrelated unique implementations converge | `H1` + `H2` -> both `H3` | 2 source cohorts = 2 | FAIL 1 |
| Delete + equivalent add | removed `H` occurrence replaced by new-site `H` | same-fingerprint replacement keeps one source cohort | PASS |
| Delete one of pair + equivalent add | legacy cohort `H` remains size 2 | one source cohort | PASS |
| Delete unique + add two equivalent | one base `H` -> two current `H` | one replacement source + one new = 2 | FAIL 1 |
| File rename/move, same fingerprint | `H` -> `H` elsewhere | structural identity or same-fingerprint replacement | PASS |
| Function rename, same fingerprint | `H` -> `H` with renamed declaration | same-fingerprint replacement | PASS |
| Rename/move plus implementation edit | `H1` -> `H2` | PASS with certified site lineage, or via an unused slot only when `H2` is the sole proven successor group; otherwise UNKNOWN/INCOMPLETE if judgement depends on it | explicit |
| Group split | one cohort `H1` -> descendant groups `H2`, `H3`, ... | every new group contains the same single source cohort | PASS; may reduce debt |
| Split + removed-slot allocation matters | one cohort has proven descendants in `H2` and `H3`, plus a removed slot and an unmatched occurrence on a branch | slot has multiple plausible successor branches | INCOMPLETE if allocation changes regression count |
| Two old duplicate groups merge | cohort `H1` + cohort `H2` -> one `H3` | 2 source cohorts = 2 | FAIL 1 |
| `m` old groups merge | `m` distinct source cohorts -> one group | `m` bundles | FAIL `m - 1` |
| Partial survival of legacy group | any subset of one old cohort survives together | one source cohort | PASS |
| Partial survival + genuinely new member | one old cohort + one new member | 1 source + 1 new = 2 | FAIL 1 |
| Ambiguous site identity, unchanged fingerprint | same-fingerprint cardinality/replacement may still decide | no cross-fingerprint guess needed | decide if cardinality suffices |
| Ambiguous site identity, changed fingerprint | ancestry could change bundle count | unknown lineage | INCOMPLETE, not normal PASS/FAIL |
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

- certified descendants of that cohort; or
- same-fingerprint one-for-one replacements of removed occurrences.

It cannot cross from cohort `H1` to cohort `H2` merely because the total
number of duplicates in the repository stayed flat.

## Ambiguity and incomplete measurement

The ratchet must not guess occurrence ancestry.

### Identity ambiguity

Examples include duplicate structural keys, anonymous/computed sites or a
family/language with no admitted identity rule.

Ambiguity is handled in two layers:

1. same-fingerprint replacement/cardinality remains usable because it does not
   require choosing one historical site;
2. cross-fingerprint lineage is not inferred.

If unresolved cross-fingerprint lineage could change the bundle count of a
current duplicate group, that claim is UNKNOWN and required duplication
measurement is INCOMPLETE.

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

Removal of one `H` occurrence plus addition of one `H` occurrence is a
replacement slot and retains source cohort `C_h`.

This exact-fingerprint replacement is consumed only after proven divergent site
successors have taken their own base slots, so a diverged member cannot leave a
second allowance behind.

This covers:

- file move;
- function rename when the declaration name is excluded from the canonical
  body fingerprint;
- delete + equivalent add;
- relocation among duplicated legacy copies.

Multiple replacements are bounded one-for-one by removed occurrences. An
additional current occurrence beyond replacement supply is new.

### Changed canonical implementation

When `H1 -> H2`, occurrence-level lineage carries through certified structural
identity. Unused slots may additionally follow a changed fingerprint only via
the unique-successor cohort rule above.

A fully changed-fingerprint replacement with no proven successor is not assumed
held. A split cohort does not donate its removed slots to an arbitrary branch.

This preserves synchronized legacy debt without turning every fingerprint
change into a transferable allowance.

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
- exact same-fingerprint move/replacement semantics;
- no global allowance transfer;
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

One removed occurrence may conserve at most one current occurrence. It may do
so through an exact-fingerprint replacement or the unique changed-successor
rule; it cannot be spent on multiple branches.

### I8. Origin separation

Two different base cohorts never become one held allowance merely because their
current fingerprints match.

### I9. No global compensation

Resolving or splitting duplicate debt elsewhere cannot pay for a new origin in
this clone group.

### I10. No guessed lineage

No ambiguous/unsupported cross-fingerprint identity is converted to lineage
merely to make the verdict pass or fail.

### I11. Basis compatibility before lineage

Measurement-basis compatibility is checked before cohort or site matching.

## Implications for #48

The following #48 ideas survive:

- canonical function/method fingerprints;
- language-separated matching;
- compact persisted base fingerprint counts;
- changed-file before/current extraction;
- exact same-fingerprint remove/add replacement accounting;
- one-for-one occurrence-slot conservation;
- deterministic assignment of only new excess findings;
- no repository duplicated-line percentage;
- O(delta)-shaped warm work.

The per-fingerprint multiplicity equation is no longer the complete ratchet.
It becomes the same-fingerprint special case inside a lineage-aware judgement.

The eventual implementation ticket must add:

- optional versioned structural occurrence identity for cross-fingerprint
  lineage;
- source-cohort bundle accounting plus the bounded unique-successor slot rule;
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
- Synchronized edits remain held: **yes**, I1.
- Unrelated convergence is detected: **yes**, I4.
- Third copy over legacy debt is detected exactly once: **yes**, I2.
- Move/replacement semantics are explicit: **yes**.
- Group split/merge semantics are explicit: **yes**, I4/I5.
- Ambiguous identity is never guessed: **yes**, I7.
- Detector-version mismatch cannot manufacture normal findings: **yes**, I8.
- Old allowance cannot be spent on unrelated code: **yes**, cohort-local debt.
- Complexity and persisted metadata are estimated: **yes**.
- #478 must be updated with this selected model: **required companion update**.
