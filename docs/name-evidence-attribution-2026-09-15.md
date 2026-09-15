# Name-evidence cost and the A/B decision (#199)

#197 asks two questions about the warm Stop:

- **A.** Can consumer-shaped queries over one shared owner of declaration and
  reference semantics remove material warm time?
- **B.** Can persistent base name evidence plus a changed-name delta remove
  material warm time?

This note records the attribution method, the measured rows, the verdicts,
the correctness analysis and the invariants. It records no implementation of
A or B.

## Method

Run from `a57c30b060ea3c3da6faaaa58ec1390813cd8b62` with a release build:

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture
```

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm100 cargo test --release --test performance -- --ignored perf --nocapture
```

Each row primes the base, changes 20 files (or 100 files), and times five
warm Stops. It prints the median of each counter. These two rows are the only
rows for this decision. No 1M, cold or strict row ran.

Machine: MacBookPro18,3, Apple M1 Pro, macOS 26.6.2. Toolchain: rustc 1.98.1.
klin 0.1.1. Five iterations, median. The repository owner ran the rows from a
clean tree.

### Where each part of the gate time goes

The harness prints each `names` value as `<gate>_names_<field>` and each tree
value as `<gate>_names_<tree>_<field>`.

| Ticket part | `dead-symbols` counter | `reachability` counter |
| --- | --- | --- |
| Base layout, base file list and structural cache read, shared | `names_base_ms` | `names_base_ms` (0 when `dead-symbols` paid it) |
| Cache naming, read and decode, shared | `facts_cache_read_ms` (inside `names_base_ms`) | `facts_cache_read_ms` |
| Before Measurement/selection assembly | `names_before_measure_ms` | `names_before_measure_ms` |
| After Measurement/selection assembly | `names_after_measure_ms` | `names_after_measure_ms` |
| Before `SourceIndex::of` | `names_before_index_ms` | `names_before_index_ms` |
| After `SourceIndex::of` | `names_after_index_ms` | `names_after_index_ms` |
| State and name queries | `names_before_query_ms`, `names_after_query_ms` | same |
| `lost_reference` | `names_lost_ms` | none |
| Remaining ratchet, coverage and report work | `ms` less the parts above | `ms` less the parts above |

`measure_ms` holds the tree's share of `facts_ms`, which is extraction of the
changed files. The cache write (`facts_cache_write_ms`) is outside every
part, and a warm Stop over a fresh cache writes nothing.

The hook total less the sum of the gate `ms` values holds the build, the turn
window, the journal and the removal of the base worktree when the run ends.
No counter separates those parts.

## Recorded rows

### Whole Stop

| Median | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| Hook | 2,079 ms | 2,320 ms |
| Sum of all gate `ms` | 1,329 ms | 1,480 ms |
| Hook outside the gates | 750 ms | 840 ms |
| `dead-symbols` `ms` | 1,030 ms | 1,119 ms |
| `reachability` `ms` | 107 ms | 111 ms |

### Parts of the two gates

| Part, median ms | `dead-symbols`, 20 | `reachability`, 20 | `dead-symbols`, 100 | `reachability`, 100 |
| --- | ---: | ---: | ---: | ---: |
| `names_base_ms` | 946 | 0 | 986 | 0 |
| inside it: `facts_cache_read_ms` | 58 | 0 | 57 | 0 |
| `names_before_measure_ms` | 10 | 4 | 30 | 4 |
| `names_after_measure_ms` | 20 | 7 | 39 | 7 |
| `facts_ms` (extraction, inside measure) | 14 | 0 | 53 | 0 |
| `names_before_index_ms` | 20 | 19 | 20 | 20 |
| `names_after_index_ms` | 20 | 20 | 20 | 20 |
| `names_before_query_ms` | 0 | 12 | 1 | 12 |
| `names_after_query_ms` | 0 | 13 | 1 | 13 |
| `names_lost_ms` | 0 | none | 0 | none |
| Remainder | 14 | 32 | 22 | 35 |

The remainder is the gate `ms` less the parts. A part's median and the gate's
median come from different Stops, so the remainder is approximate.

### Index size

All eight indexes of the two rows hold the same evidence:

| Counter | Each tree, each gate, each row |
| --- | ---: |
| `files` | 10,000 |
| `declarations` | 90,057 |
| `references` | 320,114 |
| `distinct_names` | 65,590 |

The two gates select the same files in this fixture, so `A_max` below applies.

## What the rows show

- Name evidence is small. Measurement, indexes, queries and lost references
  take 145 ms over both gates at 20 changed files (7.0% of the Stop) and
  187 ms at 100 (8.1%).
- One `SourceIndex::of` over 10,000 files and 320,114 reference sites takes
  about 20 ms in both rows. A warm Stop builds four of them.
- `dead-symbols` state queries take at most 1 ms after #196.
  `reachability` queries every family member in both trees, in 25 ms in both
  rows.
- Repository-sized parts stay flat from 20 to 100 changed files: the index
  builds, the `reachability` queries, the cache read, and `dead-symbols`
  measurement less extraction (16 ms in both rows). `dead-symbols` measurement
  grows by 39 ms, and its extraction grows by 39 ms, so that growth is
  delta-sized.
- The largest repository-sized part of the gates is `names_base_ms`: 946 ms
  (45.5% of the Stop) and 986 ms (42.5%). Less the cache read, 888 ms and
  929 ms go to laying out the whole base (`git worktree add`, the change set,
  renamed files moved) and listing the base tree's files. This work is shared,
  and it is neither A nor B.
- From 20 to 100 changed files the Stop grows by 241 ms: 151 ms in the gates
  and 90 ms outside them. The rows do not show how the time outside the gates
  divides.

## What the code shows about size

This table comes from the code, not from a measurement.

| Work in one warm Stop | Times per Stop | Size |
| --- | --- | --- |
| `git worktree add` of the whole base, and renamed files moved to their current paths | 1, shared | Repository |
| Structural cache naming, read and decode | 1, shared | Repository |
| File selection and outcome lookup (`measure`) | 2 gates × 2 trees | Repository |
| `SourceIndex::of` | 2 gates × 2 trees | Repository |
| `dead-symbols` states (#196) | 2 trees | File scan is repository-sized, state construction is delta-sized |
| `lost_reference` | once per dead finding in the judged scope | Delta |
| `dead-symbols` coverage `lost` over both file lists | 1 | Repository |
| `reachability` states: `reached` and `proven` for every family member | 2 trees | Repository |
| `reachability` `covered`: family glob per measured file | 3 | Repository |
| Base worktree removal | 1, after the gates | Repository |

A warm Stop builds four name indexes over the same shared `Rc<FileFacts>`:
one per tree in each gate. The two base indexes are the same every Stop over
one base commit, one scope and one change set.

## Decision rule

"Material" means at least 10% of the warm-20 hook median, which is 208 ms.
The x10 target needs more than 90% of the Stop removed, so a smaller part
cannot move it.

**A** survives only if the duplicated semantic work is material:

```text
A_max = Σ over trees of min(dead.tree.index_ms, reach.tree.index_ms)
      + Σ over trees of min(dead.tree.measure_ms, reach.tree.measure_ms)
```

This bound applies only where both gates' `files` counts of a tree are equal.
With different selections, one index cannot serve both gates without a
per-query file filter, and that filter costs time.

**B** survives only if the repository-sized reconstruction is material in
both rows and flat between them:

```text
B_max = Σ over gates of (before.measure_ms + before.index_ms + after.index_ms)
      + reach.before.query_ms + reach.after.query_ms
```

`B_max` is an upper bound: it assumes that applying the delta costs nothing
and that every after-tree query becomes delta-sized.

Both bounds are inference from the medians, not measurements of an
implementation.

## Verdicts

| Bound | Warm, 20 changed | Share of that Stop | Warm, 100 changed | Share of that Stop |
| --- | ---: | ---: | ---: | ---: |
| `A_max` | 19 + 20 + 4 + 7 = 50 ms | 2.4% | 20 + 20 + 4 + 7 = 51 ms | 2.2% |
| `B_max` | 50 + 43 + 25 = 118 ms | 5.7% | 70 + 44 + 25 = 139 ms | 6.0% |

**A: falsified as a warm win.** Sharing one index and one measurement between
the two gates removes at most 50 ms of 2,079 ms and 51 ms of 2,320 ms
(inference). No follow-up ticket for A. If A ships later for another reason,
it must be one shared owner of declaration and reference semantics, and each
gate must keep its own evidence selection. Separate dead-symbol and
reachability indexes are not recommended.

**B: falsified.** An overlay around the current `Unchanged` `Rc<FileFacts>`
that still rebuilds whole-repository indexes removes nothing. A persistent
base index with a changed-name delta removes at most 118 ms and 139 ms
(inference), below the 208 ms threshold in both rows. `B_max` grows by 21 ms
from 20 to 100 changed files because `dead-symbols` extraction grows inside
`before.measure_ms`, which a persistent base cannot remove. No follow-up
ticket for B.

**Outside A and B.** The repository-sized warm cost that the gates measure is
the base layout inside `names_base_ms`: 888 ms at warm-20 and 929 ms at
warm-100. A follow-up for that cost is outside #199. The correctness analysis
below still applies to it, because the base layout decides which bytes the
base evidence reads.

## Correctness analysis

A future implementation of A or B must give the findings, notes, coverage and
exit codes that two independent extractions give (8.4). These cases decide
that:

- **Different selections.** `dead-symbols` selects every structural extension
  under its `in` and `except` scope. Its base tree uses the scope the base
  commit recorded (`Scope::at_base`), so the before and after selections can
  differ. `reachability` selects the extensions of its families under the
  whole root, with no scope, and its scope only decides family membership. The
  two gates therefore resolve names over different file sets in general.
- **Global evidence, scoped judgement (#196).** In a changed run that is not
  strict, `dead-symbols` builds state only for the judged files. Both indexes
  stay complete, so an unchanged file's reference keeps a changed declaration
  alive. A persistent or shared index must stay complete in the same way.
- **Unchanged declaration files through changed names.** A changed file that
  adds or removes a reference to `name` changes `reached` for every family
  member that declares `name`, including unchanged files. A changed file that
  adds or removes a declaration of `name` changes `proven` for every member
  that declares `name`, because `proven` needs exactly one declaration. The
  ratchet restricts findings to `Context.only`, but `sibling` reads `proven`
  of unchanged members, so the remedy text of a changed file's finding can
  change. Strict and whole runs report the unchanged files themselves.
- **Duplicate names in `dead-symbols`.** A declaration is dead when no
  reference site of its name is outside its own span. A new declaration
  changes no other declaration's state. A new or removed reference changes
  the state of every declaration of that name in the language partition.
- **Additions, deletions and renames.** An addition adds a file's
  contributions and a deletion removes them. The base lays each renamed file
  out at its current path, so the base evidence of a run depends on its change
  set, not on the commit alone. A rename changes the path in
  `reference.file != file.file`, the scope match and the family glob.
- **Unparsed or unsupported changes.** A changed file that no longer parses,
  or that no adapter reads, contributes no declarations and no references
  after the change. Its base contributions still leave, and its coverage
  outcome changes.
- **Language partitions.** Rust and TypeScript names never resolve across
  partitions, and TSX is TypeScript. A `.ts` to `.tsx` rename stays in one
  partition. A rename across languages moves the contributions to another
  partition.
- **First lost reference.** `lost_reference` names the first base reference in
  path order, then line order, that is outside the declaration's span and
  absent now. A merged index must keep each name's sites sorted by path and
  line, with one site per file and line.
- **Strict and whole runs.** These runs extract both trees and read no cache.
  They must stay complete and must not read a persisted delta.

## Invariants for a future implementation

1. Evidence stays complete over each gate's own selection. Judgement scope
   never narrows evidence.
2. One owner holds declaration and reference semantics. A consumer asks a
   query of that owner and keeps its own eligibility and selection.
3. Persisted base evidence is keyed like the structural cache (commit,
   checkout identity, binary and extraction version) and also by the
   selection: extensions and base scope for `dead-symbols`, family extensions
   for `reachability`.
4. Invalidation works on name contributions. A contribution is one file's
   declarations and reference sites of one name in one language partition, at
   one path. The removed contributions are the base versions of the changed,
   deleted and renamed paths. The added contributions are their working-tree
   versions. The affected names are the names of both sets. The affected
   derived evidence is:
   - `dead` of every declaration of an affected name, in every file
   - `reached` of every family member that declares an affected name
   - `proven` of every family member whose affected name changes its
     declaration count across one, or gains or loses its last outside
     reference
   - `sibling` of every finding in a family that holds such a member
5. A path change moves contributions and runs the scope, the family glob and
   the self-reference exclusion again for that path.
6. Sites stay in path and line order, deduplicated per file and line.
7. Strict, whole and by-hand runs keep today's full reconstruction.
