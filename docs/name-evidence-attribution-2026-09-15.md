# Name-evidence cost and the A/B decision (#199)

#197 asks two questions about the warm Stop:

- **A.** Can consumer-shaped queries over one shared owner of declaration and
  reference semantics remove material warm time?
- **B.** Can persistent base name evidence plus a changed-name delta remove
  material warm time?

This note records the attribution method, the correctness analysis, the
invariants and the decision rule. It records no implementation of A or B.

## Status

The attribution counters are in the gate rows (11.2 `names`). The two
recorded 300k rows are not measured yet. The section "Recorded rows" names
the commands. The verdicts below are provisional. They use the #193 and #196
rows as background evidence, and the new rows confirm or falsify them.

## Method

Run from the commit that adds this note, with a release build and no other
load on the machine:

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture
```

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm100 cargo test --release --test performance -- --ignored perf --nocapture
```

Each row primes the base, changes 20 files (or 100 files), and times five
warm Stops. It prints the median of each counter. Do not run the 1M row, the
cold row or the strict row for this decision. A Stop hook of an agent session
that runs on the klin repository during a row is load, so record it.

Record `git rev-parse HEAD`, `sysctl -n hw.model`, `sw_vers -productVersion`
and `rustc --version` beside the rows.

### Where each part of the gate time goes

The harness prints each `names` value as `<gate>_names_<field>` and each tree
value as `<gate>_names_<tree>_<field>`.

| Ticket part | `dead-symbols` counter | `reachability` counter |
| --- | --- | --- |
| Base layout and structural cache read, shared | `names_base_ms` | `names_base_ms` (0 when `dead-symbols` paid it) |
| Cache read and decode, shared | `facts_cache_read_ms` (inside `names_base_ms`) | `facts_cache_read_ms` |
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

### Size counters

Each tree also prints `files`, `declarations`, `references` and
`distinct_names` of its index. These counts are deterministic. A cost is
repository-sized when it stays flat from the 20-file row to the 100-file row
and follows these counts. A cost is delta-sized when it grows with the changed
files.

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

## Background evidence

The #193 and #196 rows are from the MacBook Pro 18,3 (Apple M1 Pro, macOS
26.6.2), release builds, median of five.

| 300k row | Hook | `dead-symbols` | `reachability` | `dead-symbols` extraction | Cache read |
| --- | ---: | ---: | ---: | ---: | ---: |
| Warm, 20 changed | 2,317 ms (#196) | 1,209 ms (#196) | 95 ms (#193) | 11 ms (#192) | 27 ms (#192) |
| Warm, 100 changed | 2,192 ms (#196) | 1,146 ms (#196) | 98 ms (#193) | not recorded | not recorded |

Inference from these rows:

- `reachability` measures both trees, builds two indexes over the same 20,000
  shared facts, and queries every family member in both trees, in about
  95 ms. So one gate's two indexes, two measurements and repository-wide
  queries cost less than 100 ms at 300k.
- `dead-symbols` builds indexes of the same size. About 1,170 ms of its time
  is outside extraction and the cache read, and that time does not grow from
  20 to 100 changed files. So it is repository-sized.
- The most probable owner of that time is the whole-base `git worktree add`,
  which `names_base_ms` now separates. That work is neither A nor B.

## Decision rule

"Material" means at least 10% of the warm-20 hook median. The x10 target
needs more than 90% of the Stop removed, so a smaller part cannot move it.

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

Both bounds are inference, not measurement. Record them from the medians.

## Provisional verdicts

**A: probably falsified as a warm win.** The whole `reachability` gate is
about 4% of the warm-20 Stop. `A_max` cannot be larger than the smaller gate's
index and measurement time, so `A_max` is below 100 ms (inference). If A ever
ships for other reasons, it must be one shared owner of declaration and
reference semantics, and each gate must keep its own evidence selection.
Separate dead-symbol and reachability indexes are not recommended.

**B: probably falsified.** An overlay around the current
`Unchanged` `Rc<FileFacts>` that still rebuilds whole-repository indexes
removes nothing. A persistent base index removes at most `B_max`, and the
background rows put `B_max` near 200 ms (about 9% of the Stop, inference).
B survives only if the recorded rows show `B_max` of at least 10% and flat.

**Outside A and B.** If `names_base_ms` holds most of the `dead-symbols`
remainder, the largest repository-sized warm cost is the base checkout. A
follow-up ticket for that cost needs the recorded rows first.

## Correctness analysis

A surviving implementation of A or B must give the findings, notes, coverage
and exit codes that two independent extractions give (8.4). These cases decide
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

## Invariants for a surviving implementation

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
