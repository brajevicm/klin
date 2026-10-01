# Cyclic dependency inventory, 2026-10-01

This note belongs to #371, the WP0 of the cycle-first architecture foundation
(#370). It records the held cyclic edges on klin's own tree and assigns each
edge to one WP1 ticket. ADR 0065 records the rules that the WP1 tickets
follow.

## The measured commit

- Commit: `bffa48cd622537d42e6a636fb33c3a9c14d5e04d`, the merge of #424. #421
  is closed, so `klin layering` counts an edge written through a bare child
  path.
- Binary: `cargo build --release` at that commit.
- `klin layering` on the repository, with the `klin.json` of that commit:

  ```text
  OK: 723 dependency site(s) judged, 0 forbidden, 282 cyclic, all held at the base
  ```

**The held cyclic count at `bffa48cd` is 282.** Each WP1 PR measures its
count before and after with `klin layering`. The numbers below come from the
diagnostic method, and `klin layering` stays authoritative (ADR 0065).

## Method

The in-repo output names 20 held edges and then stops. The diagnostic method
uses the same binary on a copy of the tree:

1. Make a scratch repository with one empty commit as the base.
2. Copy `src/`, `Cargo.toml` and `Cargo.lock` into the scratch repository.
   Write a `klin.json` that holds only the `layering` section of the
   measured commit, and stage everything.
3. Run `klin layering`. The base holds nothing, so the run prints all 282
   cyclic edges as new, each with its line, its site count and one shortest
   cycle.
4. Replace the layer map with one layer per file and `can_use: []`. The run
   then prints every judged edge as forbidden. This is the full module graph:
   64 modules and 366 edges.
5. Find the strongly connected components (SCCs) of that graph with Tarjan's
   algorithm. The edges inside the SCCs are exactly the 282 edges of step 3.
6. Model each WP1 ticket as a set of edge changes, in the order of #370.
   Apply them one at a time, and find the SCCs again after each one. An edge
   that leaves the SCCs at a ticket belongs to that ticket.

The scripts were one-off, and the repository does not keep them. This note
holds their results: the SCCs, the hubs and every cyclic edge. The 84 edges
outside the SCCs close no cycle, so the note does not list them.

## Limits

- Spec 8.2.1 reads no edge from a path outside an import that starts with a
  name a `use` binds. The `check` module imports `syntax` and then writes
  `syntax::structural::ExtractionCost` and
  `syntax::structural::footprint::Footprint`. klin counts the edge to
  `syntax` and no edge to `syntax::structural` or its `footprint` child.
  So this inventory does not contain those two edges. The plan below makes
  the contract depend on `syntax::structural` anyway, which is downward.
- Step 6 is a model. The projected counts are true only if each ticket
  makes the change that the model gives it, and if each new module adds no
  edge back into an SCC.

## The graph

There are two SCCs.

- **The large SCC holds 46 modules and 274 cyclic edges:** base, build,
  cache, ceiling, changed, check, complexity, config, conventions,
  conventions::report, coverage, dead_symbols, doc_citations, doc_size,
  escapes, files, hunks, inventory, journal, layering, lockfile, markers,
  modules, modules::rust, modules::typescript, project, public_api, radius,
  ratchet, reachability, reference, sarif, scope, stubs, surface,
  surface::rust, surface::typescript, survey, syntax, syntax::convention,
  syntax::pattern, syntax::structural, syntax::structural::cache,
  syntax::structural::rust, syntax::structural::typescript, and turn.
- **The host SCC holds 5 modules and 8 cyclic edges:** host and its children
  claude, codex, cursor and generic. The parent names each child in its
  adapter table, and each child imports the `Adapter` trait and helpers from
  the parent.

The 13 modules outside the SCCs are gate, git, guard, handoff, hooks, init,
main, shell, state, stats, syntax::structural::footprint, update and write.

### Hubs

These modules hold the most cyclic edges at `bffa48cd`.

| Module | Cyclic edges in | Cyclic edges out | Why it is a hub |
|---|---|---|---|
| `check` | 22 | 21 | It holds the execution contract and `CATALOGUE`, which names every check. |
| `config` | 32 | 9 | It owns `Error`, and its validation calls into checks and `CATALOGUE`. |
| `project` | 20 | 7 | It holds `Language`, `Tree` and `Project`, which reach `base`, `files` and `survey`. |
| `reference` | 20 | 3 | It holds the low key metadata and renders the catalogue. |
| `ratchet` | 19 | 2 | It holds `Values` and `body_hash`, and it imports `check`. |
| `syntax::structural` | 15 | 10 | It holds the facts and also the composition that needs `Tree`, `coverage` and `files`. |
| `syntax` | 16 | 4 | It reports unparsed files through `check`. |
| `base` | 15 | 5 | It takes `Context` and `Sink`, and `Prior` holds a `Tree`. |
| `scope` | 15 | 3 | It reaches `config`, `ratchet` and `reference` for primitives only. |
| `coverage` | 13 | 3 | It writes its report through `check`. |

### Leaf candidates

- **Items to move into new leaves.** `config::Error`, `ratchet::Values` and
  `ratchet::body_hash` depend on `std` and `serde_json` only (#372). The key
  metadata of `reference` depends on `std` only: `Key`, `Shape`,
  `SectionShape`, `Languages`, `ROOTS`, `LANGUAGES`, `EXCLUDE`, `SKIP_DIRS`
  and `extensions_by_name` (#373).
- **A module that becomes a true leaf at #374.** `ceiling` imports only
  `error` once its read takes a file path in place of `Config`.
- **Modules that become true leaves after #372.** `cache`, `changed` and
  `hunks` are cyclic only through `config::Error`.
- **True leaves at `bffa48cd`.** `git`, `shell`, `update` and `write`
  import no other module. `state` imports only `git`, and `handoff` imports
  only `state` and `write`.

## Assignment

The sequence is #372, #373, #374, #375, #376, #377, #379, #378, and then
#427 for the parent and child cycles. Every one of the 282 edges has one
owner. 93 edges are *active*: the owner changes the edge's site. 189
edges are *passive*: the site does not change, and the owner removes the
path back. A passive edge needs no planned direction, unless a ticket other
than its owner moves its site.

| Ticket | Active | Passive | Projected held count after it |
|---|---|---|---|
| `bffa48cd` | | | 282 |
| #372 | 15 | 17 | 250 |
| #373 | 21 | 8 | 221 |
| #374 | 7 | 36 | 178 |
| #375 | 3 | 88 | 87 |
| #376 | 29 | 16 | 42 |
| #377 | 1 | 3 | 38 |
| #379 | 3 | 5 | 30 |
| #378 | 2 | 4 | 24 |
| #427 | 12 | 12 | 0 |

### Active families

Each active edge belongs to one family. A count in parentheses counts
active edges unless it says passive. 14 passive edges have a site that a
ticket other than their owner moves, and they take that ticket's family:
F7b 1, F7c 7, F7d 1 and F9 5. The appendix names the family of each edge.

| Family | Current source | Current target | Why cyclic | Owning ticket | Planned direction |
|---|---|---|---|---|---|
| F1 `config::Error` importers (11) | base, cache, changed, complexity, escapes, hunks, inventory, lockfile, public_api, stubs, syntax | `config` | `config` reaches `check`, which reaches every module. | #372 | The caller imports the error type from a new leaf. `config` imports it from the leaf too. |
| F2 `Values` and `body_hash` importers (4) | files, scope, syntax, syntax::convention | `ratchet` | `ratchet` imports `check` and `config`. | #372 | The caller imports `Values` or `body_hash` from a new leaf. |
| F3 key metadata importers (20) | check, complexity, config, conventions, dead_symbols, doc_citations, doc_size, escapes, files, inventory, layering, lockfile, markers, public_api, reachability, sarif, scope, stubs, syntax, syntax::pattern | `reference` | `reference` imports `check` and `config` to render them. | #373 | The caller imports the metadata from a new low module. `reference` imports it too and keeps the rendering. |
| F4 build keys in config (1) | config | `build` | `build` imports `config`, `check`, `project` and `survey`. | #373 | `build::RUN` and `build::ROOT` move to the low metadata module, and `config` imports them there. `build → config` stays, downward. |
| F5 config semantic calls (7) | config | `check`, `conventions`, `ceiling`, `doc_size`, `doc_citations`, `public_api`, `ratchet` | Each target imports `config`. | #374 | `config` stops naming `check`, `conventions`, `doc_size`, `doc_citations`, `public_api` and `ratchet`. The runner passes the catalogue's section descriptors to `config` as data, so neither `config` nor `project` names the catalogue. `ceiling` drops its `config` import, and `config → ceiling` stays, downward: the refusal of a schedule with no step due stays in `Config::load`, so every load site keeps it. Rules that `klin.json` alone decides, such as the retired section keys and the accepted entries of a `Conventions` section, move into `config`. |
| F6 check contracts in low modules (3) | base, syntax, coverage | `check` | `check` imports `base` and every check. | #375 | The low module takes narrow facts and returns structured data. The calling check, or an existing high module, writes through `Sink` and `Records`. |
| F7a catalogue to checks (13) | check | complexity, conventions, dead_symbols, doc_citations, doc_size, escapes, inventory, layering, lockfile, public_api, reachability, sarif, stubs | Each check imports `check`. | #376 | The new catalogue module imports the concrete checks one-way. No check imports the catalogue. |
| F7b contract importers (16) | complexity, conventions, conventions::report, dead_symbols, doc_citations, doc_size, escapes, inventory, layering, lockfile, markers, public_api, ratchet, reachability, sarif, stubs | `check` | `check` holds `CATALOGUE`, which imports each of them. | #376 | The caller imports `Context`, `Sink`, `Records` and the outcome names from the new contract module directly. The contract imports no check and no catalogue. |
| F7c contract and catalogue reads (passive, 7) | check | `base`, `changed`, `config`, `project`, `modules`, `surface`, `syntax` | Each target reaches `check` until #372, #374 or #375. | #372, #374 or #375 owns each one (see the appendix), and #376 moves the sites | The contract imports `Prior`, `Change`, `Config`, `Project` and the cost types that `Records` holds. The catalogue imports `syntax` for the language tables. Each edge points down. |
| F7d reference to catalogue (passive, 1) | reference | `check` | `check` imports `reference` for metadata until #373. | #373 owns it, and #376 moves the site | `reference` imports the catalogue to render it. No low module imports `reference`. |
| F8 files to Tree (1) | files | `project` | `project` imports `files`. | #377 | `files` takes a root, a file list or a path set. `project → files` stays, one-way. |
| F9 structural composition (3) | project, syntax::structural | `syntax::structural`, `project`, `coverage` | Low structural code names `Tree`, and `Measurement` holds `coverage::Files`. | #379 | `measure`, `measure_all`, `Unchanged` and the measurement views move to a new module above `project` and `base`. Low structural code names no `Tree`, `coverage` or `files`. `Tree → syntax::structural` stays for `Extracted`. `Tree` and `Project` stop sharing a module. `base` and `survey` then import `Tree`, and `Tree` imports `files` and `scope`. `base → Tree` and `Project → base` are both one-way. This direction also covers 5 passive edges whose sites #379 moves. |
| F10 survey and window (2) | radius, survey | `turn` | `turn` imports `radius` and `base`, and `base` reaches `survey`. | #378 | `survey` takes the derivation commit as data. `turn` and `radius` import the mark, tree and derivation-commit lookups from a new module below both. `turn → radius` stays, one-way. |
| F11 child to parent (12) | host's 4 children, modules::rust, modules::typescript, surface::rust, surface::typescript, syntax::structural's cache, rust and typescript children, conventions::report | the parent | The parent names the child in a dispatch table or a call. | #427 | The items a child reads from its parent move to a sibling module. The parent and its children import the sibling. The parent keeps its dispatch, so parent → child stays, one-way. |

### Passive families

| Owner | Edges | The path back that the owner removes |
|---|---|---|
| #372 | 17 | Edges into `cache`, `changed` and `hunks`, which reach the SCC only through `config::Error`. |
| #373 | 8 | Edges out of `build` and `reference`. Only `config → build` and the metadata importers reach them. |
| #374 | 36 | Edges into `config`, `ceiling`, `scope` and `journal`. `config` no longer reaches a check or `ratchet`. |
| #375 | 88 | Edges from checks and `check` into low modules, and the children of `syntax`, `modules` and `surface`. No low module reaches `check` after #375. |
| #376 | 16 | Edges from checks into `ratchet` and `markers`. Both point to the contract, which is downward. |
| #377 | 3 | `project`, `survey` and `syntax::structural` into `files`. |
| #379 | 5 | `base → syntax::structural`, `coverage → project`, `coverage → syntax::structural`, `project → base` and `project → survey`. |
| #378 | 4 | `base → project` and `survey → project`, which #379 points at `Tree`, and `turn → base` and `turn → radius`. #378 removes `Tree → survey` and `survey → turn`. |
| #427 | 12 | Each parent → child dispatch edge. |

### The project and survey pair

`project → survey` (appendix row 188) is passive under #379.
`survey → project` (row 250) is passive under #378, and #379 points its site
at `Tree`. After both tickets, `Project` imports `survey`, and `survey` imports `Tree` and nothing in `Project`.

## Order constraints the graph proves

- **#379 before #378.** #379 points `base` and `survey` at `Tree`, and #378
  removes the edge from `Tree` to `survey`. The `base → Tree` edge stops
  being cyclic only at #378. #370 already orders #378 last.

  Result: #379 also removed `Tree → survey`. A `tree` module that still
  called `survey` closes a new cycle with `survey → Tree`, and rule 5 of
  ADR 0065 forbids that. `base` reads the run through a trait that
  `Project` implements, so `base → project` went too. After #379, `klin
  layering` holds 26 edges, and #378 owns only `radius ↔ turn`.
- **The descriptors for #374 come from the runner.** If `project` passed
  the catalogue's descriptors to `config`, the edge `project → catalogue`
  would close a new cycle through the checks.
- **#427 comes after #376.** The items that conventions::report
  reads from `conventions` include `at_the_base`, which takes `Context` and
  `Sink`. A sibling module that holds them imports the check contract, so
  rule 4 of ADR 0065 forbids it before #376. The host, modules, surface and
  syntax::structural pairs have no such constraint.

## The family no planned ticket owned

The 24 edges of F11 and its passive partner stay cyclic after #378. No
ticket that #370 planned owned them. So #371 added #427 to #370 before
implementation began:

| Pair | Edges |
|---|---|
| host and its 4 children | 8 |
| `modules` and its rust and typescript children | 4 |
| `surface` and its rust and typescript children | 4 |
| `syntax::structural` and its cache, rust and typescript children | 6 |
| `conventions` and conventions::report | 2 |

## Appendix: every cyclic edge

Paths are relative to `src/`. "Line" and "Sites" come from the scratch run.
"Shortest cycle" is the cycle that `klin layering` printed at `bffa48cd`.
The owner is the ticket after which the edge is not cyclic. "Site also
changes in" names another ticket that moves the site of the edge. A passive
edge that another ticket moves takes the direction of that ticket's family.

| # | Source | Target | Line | Sites | Shortest cycle at `bffa48cd` | Owner | Direction | Site also changes in |
|---|---|---|---|---|---|---|---|---|
| 1 | base.rs | changed.rs | 9 | 1 | base.rs → changed.rs → config.rs → check.rs → base.rs | #372 | passive |  |
| 2 | base.rs | check.rs | 10 | 1 | base.rs → check.rs → base.rs | #375 | F6 |  |
| 3 | base.rs | config.rs | 11 | 1 | base.rs → config.rs → check.rs → base.rs | #372 | F1 |  |
| 4 | base.rs | project.rs | 13 | 1 | base.rs → project.rs → base.rs | #378 | passive, F9 | #379 |
| 5 | base.rs | syntax/structural/mod.rs | 15 | 1 | base.rs → syntax/structural/mod.rs → project.rs → base.rs | #379 | passive |  |
| 6 | build.rs | changed.rs | 5 | 1 | build.rs → changed.rs → config.rs → build.rs | #372 | passive |  |
| 7 | build.rs | check.rs | 6 | 1 | build.rs → check.rs → config.rs → build.rs | #373 | passive, F7b | #376 |
| 8 | build.rs | config.rs | 7 | 1 | build.rs → config.rs → build.rs | #373 | passive |  |
| 9 | build.rs | project.rs | 8 | 1 | build.rs → project.rs → config.rs → build.rs | #373 | passive |  |
| 10 | build.rs | scope.rs | 9 | 1 | build.rs → scope.rs → config.rs → build.rs | #373 | passive |  |
| 11 | build.rs | survey.rs | 11 | 1 | build.rs → survey.rs → cache.rs → config.rs → build.rs | #373 | passive |  |
| 12 | cache.rs | config.rs | 6 | 1 | cache.rs → config.rs → doc_size.rs → cache.rs | #372 | F1 |  |
| 13 | ceiling.rs | config.rs | 6 | 1 | ceiling.rs → config.rs → ceiling.rs | #374 | passive |  |
| 14 | changed.rs | config.rs | 3 | 1 | changed.rs → config.rs → build.rs → changed.rs | #372 | F1 |  |
| 15 | check.rs | base.rs | 14 | 1 | check.rs → base.rs → check.rs | #375 | passive, F7c | #376 |
| 16 | check.rs | changed.rs | 15 | 1 | check.rs → changed.rs → config.rs → check.rs | #372 | passive, F7c | #376 |
| 17 | check.rs | config.rs | 16 | 1 | check.rs → config.rs → check.rs | #374 | passive, F7c | #376 |
| 18 | check.rs | project.rs | 17 | 1 | check.rs → project.rs → base.rs → check.rs | #375 | passive, F7c | #376 |
| 19 | check.rs | reference.rs | 18 | 2 | check.rs → reference.rs → check.rs | #373 | F3 | #376 |
| 20 | check.rs | complexity.rs | 19 | 1 | check.rs → complexity.rs → check.rs | #376 | F7a |  |
| 21 | check.rs | conventions.rs | 19 | 1 | check.rs → conventions.rs → check.rs | #376 | F7a |  |
| 22 | check.rs | dead_symbols.rs | 19 | 1 | check.rs → dead_symbols.rs → check.rs | #376 | F7a |  |
| 23 | check.rs | doc_citations.rs | 19 | 1 | check.rs → doc_citations.rs → check.rs | #376 | F7a |  |
| 24 | check.rs | doc_size.rs | 19 | 1 | check.rs → doc_size.rs → check.rs | #376 | F7a |  |
| 25 | check.rs | escapes.rs | 19 | 1 | check.rs → escapes.rs → check.rs | #376 | F7a |  |
| 26 | check.rs | inventory.rs | 19 | 1 | check.rs → inventory.rs → check.rs | #376 | F7a |  |
| 27 | check.rs | layering.rs | 19 | 1 | check.rs → layering.rs → check.rs | #376 | F7a |  |
| 28 | check.rs | lockfile.rs | 19 | 1 | check.rs → lockfile.rs → check.rs | #376 | F7a |  |
| 29 | check.rs | modules/mod.rs | 19 | 1 | check.rs → modules/mod.rs → syntax/structural/mod.rs → config.rs → check.rs | #375 | passive, F7c | #376 |
| 30 | check.rs | public_api.rs | 19 | 1 | check.rs → public_api.rs → check.rs | #376 | F7a |  |
| 31 | check.rs | reachability.rs | 19 | 1 | check.rs → reachability.rs → check.rs | #376 | F7a |  |
| 32 | check.rs | sarif.rs | 19 | 1 | check.rs → sarif.rs → check.rs | #376 | F7a |  |
| 33 | check.rs | stubs.rs | 19 | 1 | check.rs → stubs.rs → check.rs | #376 | F7a |  |
| 34 | check.rs | surface/mod.rs | 19 | 1 | check.rs → surface/mod.rs → syntax/structural/mod.rs → config.rs → check.rs | #375 | passive, F7c | #376 |
| 35 | check.rs | syntax/mod.rs | 19 | 1 | check.rs → syntax/mod.rs → check.rs | #375 | passive, F7c | #376 |
| 36 | complexity.rs | base.rs | 8 | 1 | complexity.rs → base.rs → check.rs → complexity.rs | #375 | passive |  |
| 37 | complexity.rs | ceiling.rs | 9 | 1 | complexity.rs → ceiling.rs → config.rs → check.rs → complexity.rs | #374 | passive |  |
| 38 | complexity.rs | changed.rs | 10 | 2 | complexity.rs → changed.rs → config.rs → check.rs → complexity.rs | #372 | passive |  |
| 39 | complexity.rs | check.rs | 11 | 1 | complexity.rs → check.rs → complexity.rs | #376 | F7b |  |
| 40 | complexity.rs | config.rs | 12 | 1 | complexity.rs → config.rs → check.rs → complexity.rs | #372 | F1 |  |
| 41 | complexity.rs | coverage.rs | 13 | 1 | complexity.rs → coverage.rs → check.rs → complexity.rs | #375 | passive |  |
| 42 | complexity.rs | files.rs | 14 | 1 | complexity.rs → files.rs → config.rs → check.rs → complexity.rs | #375 | passive |  |
| 43 | complexity.rs | project.rs | 15 | 1 | complexity.rs → project.rs → base.rs → check.rs → complexity.rs | #375 | passive |  |
| 44 | complexity.rs | ratchet.rs | 16 | 1 | complexity.rs → ratchet.rs → check.rs → complexity.rs | #376 | passive |  |
| 45 | complexity.rs | reference.rs | 17 | 4 | complexity.rs → reference.rs → check.rs → complexity.rs | #373 | F3 |  |
| 46 | complexity.rs | scope.rs | 18 | 1 | complexity.rs → scope.rs → config.rs → check.rs → complexity.rs | #374 | passive |  |
| 47 | complexity.rs | syntax/mod.rs | 19 | 1 | complexity.rs → syntax/mod.rs → check.rs → complexity.rs | #375 | passive |  |
| 48 | complexity.rs | cache.rs | 20 | 1 | complexity.rs → cache.rs → config.rs → check.rs → complexity.rs | #372 | passive |  |
| 49 | complexity.rs | survey.rs | 20 | 1 | complexity.rs → survey.rs → cache.rs → config.rs → check.rs → complexity.rs | #375 | passive |  |
| 50 | config.rs | reference.rs | 6 | 6 | config.rs → reference.rs → config.rs | #373 | F3 |  |
| 51 | config.rs | conventions.rs | 204 | 2 | config.rs → conventions.rs → config.rs | #374 | F5 |  |
| 52 | config.rs | ceiling.rs | 205 | 2 | config.rs → ceiling.rs → config.rs | #374 | F5 |  |
| 53 | config.rs | check.rs | 219 | 12 | config.rs → check.rs → config.rs | #374 | F5 |  |
| 54 | config.rs | doc_size.rs | 292 | 2 | config.rs → doc_size.rs → config.rs | #374 | F5 |  |
| 55 | config.rs | build.rs | 446 | 2 | config.rs → build.rs → config.rs | #373 | F4 |  |
| 56 | config.rs | doc_citations.rs | 872 | 1 | config.rs → doc_citations.rs → config.rs | #374 | F5 |  |
| 57 | config.rs | public_api.rs | 875 | 1 | config.rs → public_api.rs → config.rs | #374 | F5 |  |
| 58 | config.rs | ratchet.rs | 959 | 1 | config.rs → ratchet.rs → config.rs | #374 | F5 |  |
| 59 | conventions.rs | check.rs | 20 | 1 | conventions.rs → check.rs → conventions.rs | #376 | F7b |  |
| 60 | conventions.rs | config.rs | 21 | 2 | conventions.rs → config.rs → conventions.rs | #374 | passive |  |
| 61 | conventions.rs | coverage.rs | 22 | 1 | conventions.rs → coverage.rs → check.rs → conventions.rs | #375 | passive |  |
| 62 | conventions.rs | project.rs | 23 | 1 | conventions.rs → project.rs → config.rs → conventions.rs | #375 | passive |  |
| 63 | conventions.rs | ratchet.rs | 24 | 1 | conventions.rs → ratchet.rs → check.rs → conventions.rs | #376 | passive |  |
| 64 | conventions.rs | reference.rs | 25 | 6 | conventions.rs → reference.rs → check.rs → conventions.rs | #373 | F3 |  |
| 65 | conventions.rs | scope.rs | 26 | 1 | conventions.rs → scope.rs → config.rs → conventions.rs | #374 | passive |  |
| 66 | conventions.rs | syntax/pattern.rs | 27 | 1 | conventions.rs → syntax/pattern.rs → reference.rs → check.rs → conventions.rs | #375 | passive |  |
| 67 | conventions.rs | syntax/mod.rs | 28 | 1 | conventions.rs → syntax/mod.rs → check.rs → conventions.rs | #375 | passive |  |
| 68 | conventions.rs | base.rs | 29 | 1 | conventions.rs → base.rs → check.rs → conventions.rs | #375 | passive |  |
| 69 | conventions.rs | files.rs | 29 | 1 | conventions.rs → files.rs → config.rs → conventions.rs | #375 | passive |  |
| 70 | conventions.rs | conventions/report.rs | 285 | 1 | conventions.rs → conventions/report.rs → conventions.rs | #427 | passive |  |
| 71 | conventions/report.rs | check.rs | 10 | 1 | conventions/report.rs → check.rs → conventions.rs → conventions/report.rs | #376 | F7b |  |
| 72 | conventions/report.rs | config.rs | 11 | 1 | conventions/report.rs → config.rs → conventions.rs → conventions/report.rs | #374 | passive |  |
| 73 | conventions/report.rs | project.rs | 12 | 1 | conventions/report.rs → project.rs → config.rs → conventions.rs → conventions/report.rs | #375 | passive |  |
| 74 | conventions/report.rs | ratchet.rs | 13 | 1 | conventions/report.rs → ratchet.rs → check.rs → conventions.rs → conventions/report.rs | #376 | passive |  |
| 75 | conventions/report.rs | scope.rs | 14 | 1 | conventions/report.rs → scope.rs → config.rs → conventions.rs → conventions/report.rs | #374 | passive |  |
| 76 | conventions/report.rs | syntax/mod.rs | 15 | 1 | conventions/report.rs → syntax/mod.rs → check.rs → conventions.rs → conventions/report.rs | #375 | passive |  |
| 77 | conventions/report.rs | syntax/pattern.rs | 15 | 1 | conventions/report.rs → syntax/pattern.rs → reference.rs → check.rs → conventions.rs → conventions/report.rs | #375 | passive |  |
| 78 | conventions/report.rs | conventions.rs | 17 | 1 | conventions/report.rs → conventions.rs → conventions/report.rs | #427 | F11 |  |
| 79 | coverage.rs | check.rs | 6 | 1 | coverage.rs → check.rs → complexity.rs → coverage.rs | #375 | F6 |  |
| 80 | coverage.rs | project.rs | 7 | 1 | coverage.rs → project.rs → syntax/structural/mod.rs → coverage.rs | #379 | passive |  |
| 81 | coverage.rs | syntax/structural/mod.rs | 8 | 1 | coverage.rs → syntax/structural/mod.rs → coverage.rs | #379 | passive |  |
| 82 | dead_symbols.rs | base.rs | 14 | 1 | dead_symbols.rs → base.rs → check.rs → dead_symbols.rs | #375 | passive |  |
| 83 | dead_symbols.rs | check.rs | 15 | 1 | dead_symbols.rs → check.rs → dead_symbols.rs | #376 | F7b |  |
| 84 | dead_symbols.rs | config.rs | 16 | 1 | dead_symbols.rs → config.rs → check.rs → dead_symbols.rs | #374 | passive |  |
| 85 | dead_symbols.rs | coverage.rs | 17 | 1 | dead_symbols.rs → coverage.rs → check.rs → dead_symbols.rs | #375 | passive |  |
| 86 | dead_symbols.rs | files.rs | 18 | 1 | dead_symbols.rs → files.rs → config.rs → check.rs → dead_symbols.rs | #375 | passive |  |
| 87 | dead_symbols.rs | project.rs | 19 | 1 | dead_symbols.rs → project.rs → base.rs → check.rs → dead_symbols.rs | #375 | passive |  |
| 88 | dead_symbols.rs | ratchet.rs | 20 | 1 | dead_symbols.rs → ratchet.rs → check.rs → dead_symbols.rs | #376 | passive |  |
| 89 | dead_symbols.rs | reference.rs | 21 | 2 | dead_symbols.rs → reference.rs → check.rs → dead_symbols.rs | #373 | F3 |  |
| 90 | dead_symbols.rs | scope.rs | 22 | 1 | dead_symbols.rs → scope.rs → config.rs → check.rs → dead_symbols.rs | #374 | passive |  |
| 91 | dead_symbols.rs | syntax/mod.rs | 23 | 1 | dead_symbols.rs → syntax/mod.rs → check.rs → dead_symbols.rs | #375 | passive |  |
| 92 | dead_symbols.rs | syntax/structural/mod.rs | 23 | 1 | dead_symbols.rs → syntax/structural/mod.rs → config.rs → check.rs → dead_symbols.rs | #375 | passive |  |
| 93 | doc_citations.rs | base.rs | 13 | 1 | doc_citations.rs → base.rs → check.rs → doc_citations.rs | #375 | passive |  |
| 94 | doc_citations.rs | changed.rs | 14 | 1 | doc_citations.rs → changed.rs → config.rs → doc_citations.rs | #372 | passive |  |
| 95 | doc_citations.rs | check.rs | 15 | 1 | doc_citations.rs → check.rs → doc_citations.rs | #376 | F7b |  |
| 96 | doc_citations.rs | config.rs | 16 | 1 | doc_citations.rs → config.rs → doc_citations.rs | #374 | passive |  |
| 97 | doc_citations.rs | coverage.rs | 17 | 1 | doc_citations.rs → coverage.rs → check.rs → doc_citations.rs | #375 | passive |  |
| 98 | doc_citations.rs | files.rs | 18 | 1 | doc_citations.rs → files.rs → config.rs → doc_citations.rs | #375 | passive |  |
| 99 | doc_citations.rs | project.rs | 20 | 1 | doc_citations.rs → project.rs → config.rs → doc_citations.rs | #375 | passive |  |
| 100 | doc_citations.rs | ratchet.rs | 21 | 1 | doc_citations.rs → ratchet.rs → check.rs → doc_citations.rs | #376 | passive |  |
| 101 | doc_citations.rs | reference.rs | 22 | 1 | doc_citations.rs → reference.rs → doc_citations.rs | #373 | F3 |  |
| 102 | doc_size.rs | base.rs | 13 | 1 | doc_size.rs → base.rs → check.rs → doc_size.rs | #375 | passive |  |
| 103 | doc_size.rs | cache.rs | 14 | 1 | doc_size.rs → cache.rs → config.rs → doc_size.rs | #372 | passive |  |
| 104 | doc_size.rs | ceiling.rs | 15 | 1 | doc_size.rs → ceiling.rs → config.rs → doc_size.rs | #374 | passive |  |
| 105 | doc_size.rs | changed.rs | 16 | 1 | doc_size.rs → changed.rs → config.rs → doc_size.rs | #372 | passive |  |
| 106 | doc_size.rs | check.rs | 17 | 1 | doc_size.rs → check.rs → doc_size.rs | #376 | F7b |  |
| 107 | doc_size.rs | config.rs | 18 | 1 | doc_size.rs → config.rs → doc_size.rs | #374 | passive |  |
| 108 | doc_size.rs | coverage.rs | 19 | 1 | doc_size.rs → coverage.rs → check.rs → doc_size.rs | #375 | passive |  |
| 109 | doc_size.rs | project.rs | 20 | 1 | doc_size.rs → project.rs → config.rs → doc_size.rs | #375 | passive |  |
| 110 | doc_size.rs | reference.rs | 21 | 2 | doc_size.rs → reference.rs → check.rs → doc_size.rs | #373 | F3 |  |
| 111 | escapes.rs | check.rs | 3 | 1 | escapes.rs → check.rs → escapes.rs | #376 | F7b |  |
| 112 | escapes.rs | config.rs | 4 | 1 | escapes.rs → config.rs → check.rs → escapes.rs | #372 | F1 |  |
| 113 | escapes.rs | markers.rs | 5 | 1 | escapes.rs → markers.rs → check.rs → escapes.rs | #376 | passive |  |
| 114 | escapes.rs | ratchet.rs | 6 | 1 | escapes.rs → ratchet.rs → check.rs → escapes.rs | #376 | passive |  |
| 115 | escapes.rs | reference.rs | 7 | 1 | escapes.rs → reference.rs → check.rs → escapes.rs | #373 | F3 |  |
| 116 | escapes.rs | scope.rs | 8 | 1 | escapes.rs → scope.rs → config.rs → check.rs → escapes.rs | #374 | passive |  |
| 117 | files.rs | config.rs | 6 | 1 | files.rs → config.rs → conventions.rs → files.rs | #374 | passive |  |
| 118 | files.rs | project.rs | 8 | 1 | files.rs → project.rs → files.rs | #377 | F8 |  |
| 119 | files.rs | ratchet.rs | 9 | 1 | files.rs → ratchet.rs → check.rs → complexity.rs → files.rs | #372 | F2 |  |
| 120 | files.rs | reference.rs | 10 | 1 | files.rs → reference.rs → doc_citations.rs → files.rs | #373 | F3 |  |
| 121 | files.rs | scope.rs | 11 | 1 | files.rs → scope.rs → config.rs → conventions.rs → files.rs | #374 | passive |  |
| 122 | host/claude.rs | host/mod.rs | 5 | 2 | host/claude.rs → host/mod.rs → host/claude.rs | #427 | F11 |  |
| 123 | host/codex.rs | host/mod.rs | 5 | 2 | host/codex.rs → host/mod.rs → host/codex.rs | #427 | F11 |  |
| 124 | host/cursor.rs | host/mod.rs | 5 | 3 | host/cursor.rs → host/mod.rs → host/cursor.rs | #427 | F11 |  |
| 125 | host/generic.rs | host/mod.rs | 5 | 1 | host/generic.rs → host/mod.rs → host/generic.rs | #427 | F11 |  |
| 126 | host/mod.rs | host/codex.rs | 176 | 1 | host/mod.rs → host/codex.rs → host/mod.rs | #427 | passive |  |
| 127 | host/mod.rs | host/cursor.rs | 177 | 1 | host/mod.rs → host/cursor.rs → host/mod.rs | #427 | passive |  |
| 128 | host/mod.rs | host/claude.rs | 178 | 2 | host/mod.rs → host/claude.rs → host/mod.rs | #427 | passive |  |
| 129 | host/mod.rs | host/generic.rs | 280 | 1 | host/mod.rs → host/generic.rs → host/mod.rs | #427 | passive |  |
| 130 | hunks.rs | config.rs | 4 | 1 | hunks.rs → config.rs → check.rs → sarif.rs → hunks.rs | #372 | F1 |  |
| 131 | inventory.rs | base.rs | 15 | 1 | inventory.rs → base.rs → check.rs → inventory.rs | #375 | passive |  |
| 132 | inventory.rs | check.rs | 16 | 1 | inventory.rs → check.rs → inventory.rs | #376 | F7b |  |
| 133 | inventory.rs | config.rs | 17 | 1 | inventory.rs → config.rs → check.rs → inventory.rs | #372 | F1 |  |
| 134 | inventory.rs | coverage.rs | 18 | 1 | inventory.rs → coverage.rs → check.rs → inventory.rs | #375 | passive |  |
| 135 | inventory.rs | project.rs | 20 | 1 | inventory.rs → project.rs → base.rs → check.rs → inventory.rs | #375 | passive |  |
| 136 | inventory.rs | ratchet.rs | 21 | 1 | inventory.rs → ratchet.rs → check.rs → inventory.rs | #376 | passive |  |
| 137 | inventory.rs | reference.rs | 22 | 1 | inventory.rs → reference.rs → check.rs → inventory.rs | #373 | F3 |  |
| 138 | inventory.rs | scope.rs | 23 | 1 | inventory.rs → scope.rs → config.rs → check.rs → inventory.rs | #374 | passive |  |
| 139 | inventory.rs | survey.rs | 24 | 1 | inventory.rs → survey.rs → cache.rs → config.rs → check.rs → inventory.rs | #375 | passive |  |
| 140 | inventory.rs | syntax/convention.rs | 25 | 1 | inventory.rs → syntax/convention.rs → ratchet.rs → check.rs → inventory.rs | #375 | passive |  |
| 141 | inventory.rs | syntax/mod.rs | 26 | 1 | inventory.rs → syntax/mod.rs → check.rs → inventory.rs | #375 | passive |  |
| 142 | inventory.rs | turn.rs | 27 | 1 | inventory.rs → turn.rs → base.rs → check.rs → inventory.rs | #375 | passive |  |
| 143 | journal.rs | config.rs | 7 | 1 | journal.rs → config.rs → build.rs → survey.rs → turn.rs → journal.rs | #374 | passive |  |
| 144 | layering.rs | check.rs | 20 | 1 | layering.rs → check.rs → layering.rs | #376 | F7b |  |
| 145 | layering.rs | config.rs | 21 | 1 | layering.rs → config.rs → check.rs → layering.rs | #374 | passive |  |
| 146 | layering.rs | modules/mod.rs | 22 | 1 | layering.rs → modules/mod.rs → syntax/structural/mod.rs → config.rs → check.rs → layering.rs | #375 | passive |  |
| 147 | layering.rs | project.rs | 25 | 1 | layering.rs → project.rs → base.rs → check.rs → layering.rs | #375 | passive |  |
| 148 | layering.rs | ratchet.rs | 26 | 1 | layering.rs → ratchet.rs → check.rs → layering.rs | #376 | passive |  |
| 149 | layering.rs | reference.rs | 27 | 3 | layering.rs → reference.rs → check.rs → layering.rs | #373 | F3 |  |
| 150 | layering.rs | scope.rs | 28 | 1 | layering.rs → scope.rs → config.rs → check.rs → layering.rs | #374 | passive |  |
| 151 | layering.rs | syntax/mod.rs | 29 | 1 | layering.rs → syntax/mod.rs → check.rs → layering.rs | #375 | passive |  |
| 152 | layering.rs | syntax/structural/mod.rs | 29 | 1 | layering.rs → syntax/structural/mod.rs → config.rs → check.rs → layering.rs | #375 | passive |  |
| 153 | layering.rs | base.rs | 30 | 1 | layering.rs → base.rs → check.rs → layering.rs | #375 | passive |  |
| 154 | layering.rs | coverage.rs | 30 | 1 | layering.rs → coverage.rs → check.rs → layering.rs | #375 | passive |  |
| 155 | lockfile.rs | check.rs | 12 | 1 | lockfile.rs → check.rs → lockfile.rs | #376 | F7b |  |
| 156 | lockfile.rs | config.rs | 13 | 1 | lockfile.rs → config.rs → check.rs → lockfile.rs | #372 | F1 |  |
| 157 | lockfile.rs | coverage.rs | 14 | 1 | lockfile.rs → coverage.rs → check.rs → lockfile.rs | #375 | passive |  |
| 158 | lockfile.rs | project.rs | 15 | 1 | lockfile.rs → project.rs → base.rs → check.rs → lockfile.rs | #375 | passive |  |
| 159 | lockfile.rs | ratchet.rs | 16 | 1 | lockfile.rs → ratchet.rs → check.rs → lockfile.rs | #376 | passive |  |
| 160 | lockfile.rs | reference.rs | 17 | 1 | lockfile.rs → reference.rs → check.rs → lockfile.rs | #373 | F3 |  |
| 161 | lockfile.rs | scope.rs | 18 | 1 | lockfile.rs → scope.rs → config.rs → check.rs → lockfile.rs | #374 | passive |  |
| 162 | lockfile.rs | base.rs | 19 | 1 | lockfile.rs → base.rs → check.rs → lockfile.rs | #375 | passive |  |
| 163 | lockfile.rs | changed.rs | 19 | 1 | lockfile.rs → changed.rs → config.rs → check.rs → lockfile.rs | #372 | passive |  |
| 164 | markers.rs | base.rs | 8 | 1 | markers.rs → base.rs → check.rs → escapes.rs → markers.rs | #375 | passive |  |
| 165 | markers.rs | changed.rs | 9 | 1 | markers.rs → changed.rs → config.rs → check.rs → escapes.rs → markers.rs | #372 | passive |  |
| 166 | markers.rs | check.rs | 10 | 1 | markers.rs → check.rs → escapes.rs → markers.rs | #376 | F7b |  |
| 167 | markers.rs | config.rs | 11 | 1 | markers.rs → config.rs → check.rs → escapes.rs → markers.rs | #374 | passive |  |
| 168 | markers.rs | coverage.rs | 12 | 1 | markers.rs → coverage.rs → check.rs → escapes.rs → markers.rs | #375 | passive |  |
| 169 | markers.rs | files.rs | 13 | 1 | markers.rs → files.rs → config.rs → check.rs → escapes.rs → markers.rs | #375 | passive |  |
| 170 | markers.rs | project.rs | 14 | 1 | markers.rs → project.rs → base.rs → check.rs → escapes.rs → markers.rs | #375 | passive |  |
| 171 | markers.rs | ratchet.rs | 15 | 1 | markers.rs → ratchet.rs → check.rs → escapes.rs → markers.rs | #376 | passive |  |
| 172 | markers.rs | reference.rs | 16 | 2 | markers.rs → reference.rs → check.rs → escapes.rs → markers.rs | #373 | F3 |  |
| 173 | markers.rs | scope.rs | 17 | 1 | markers.rs → scope.rs → config.rs → check.rs → escapes.rs → markers.rs | #374 | passive |  |
| 174 | markers.rs | syntax/mod.rs | 18 | 1 | markers.rs → syntax/mod.rs → check.rs → escapes.rs → markers.rs | #375 | passive |  |
| 175 | modules/mod.rs | syntax/structural/mod.rs | 18 | 1 | modules/mod.rs → syntax/structural/mod.rs → config.rs → check.rs → modules/mod.rs | #375 | passive |  |
| 176 | modules/mod.rs | modules/rust.rs | 28 | 1 | modules/mod.rs → modules/rust.rs → modules/mod.rs | #427 | passive |  |
| 177 | modules/mod.rs | modules/typescript.rs | 29 | 1 | modules/mod.rs → modules/typescript.rs → modules/mod.rs | #427 | passive |  |
| 178 | modules/rust.rs | modules/mod.rs | 20 | 2 | modules/rust.rs → modules/mod.rs → modules/rust.rs | #427 | F11 |  |
| 179 | modules/rust.rs | survey.rs | 21 | 1 | modules/rust.rs → survey.rs → cache.rs → config.rs → check.rs → modules/mod.rs → modules/rust.rs | #375 | passive |  |
| 180 | modules/rust.rs | syntax/structural/mod.rs | 22 | 1 | modules/rust.rs → syntax/structural/mod.rs → config.rs → check.rs → modules/mod.rs → modules/rust.rs | #375 | passive |  |
| 181 | modules/typescript.rs | modules/mod.rs | 10 | 1 | modules/typescript.rs → modules/mod.rs → modules/typescript.rs | #427 | F11 |  |
| 182 | project.rs | base.rs | 15 | 1 | project.rs → base.rs → project.rs | #379 | passive |  |
| 183 | project.rs | changed.rs | 16 | 1 | project.rs → changed.rs → config.rs → build.rs → project.rs | #372 | passive |  |
| 184 | project.rs | config.rs | 17 | 1 | project.rs → config.rs → build.rs → project.rs | #374 | passive |  |
| 185 | project.rs | syntax/structural/mod.rs | 18 | 1 | project.rs → syntax/structural/mod.rs → project.rs | #379 | F9 |  |
| 186 | project.rs | files.rs | 19 | 1 | project.rs → files.rs → project.rs | #377 | passive, F9 | #379 |
| 187 | project.rs | scope.rs | 19 | 1 | project.rs → scope.rs → config.rs → build.rs → project.rs | #374 | passive, F9 | #379 |
| 188 | project.rs | survey.rs | 19 | 1 | project.rs → survey.rs → project.rs | #379 | passive |  |
| 189 | public_api.rs | base.rs | 16 | 1 | public_api.rs → base.rs → check.rs → public_api.rs | #375 | passive |  |
| 190 | public_api.rs | check.rs | 17 | 1 | public_api.rs → check.rs → public_api.rs | #376 | F7b |  |
| 191 | public_api.rs | config.rs | 18 | 1 | public_api.rs → config.rs → public_api.rs | #372 | F1 |  |
| 192 | public_api.rs | coverage.rs | 19 | 1 | public_api.rs → coverage.rs → check.rs → public_api.rs | #375 | passive |  |
| 193 | public_api.rs | modules/mod.rs | 20 | 1 | public_api.rs → modules/mod.rs → syntax/structural/mod.rs → config.rs → public_api.rs | #375 | passive |  |
| 194 | public_api.rs | project.rs | 21 | 1 | public_api.rs → project.rs → config.rs → public_api.rs | #375 | passive |  |
| 195 | public_api.rs | ratchet.rs | 22 | 1 | public_api.rs → ratchet.rs → check.rs → public_api.rs | #376 | passive |  |
| 196 | public_api.rs | reference.rs | 23 | 1 | public_api.rs → reference.rs → check.rs → public_api.rs | #373 | F3 |  |
| 197 | public_api.rs | surface/mod.rs | 24 | 1 | public_api.rs → surface/mod.rs → syntax/structural/mod.rs → config.rs → public_api.rs | #375 | passive |  |
| 198 | public_api.rs | syntax/mod.rs | 25 | 1 | public_api.rs → syntax/mod.rs → check.rs → public_api.rs | #375 | passive |  |
| 199 | public_api.rs | syntax/structural/mod.rs | 25 | 1 | public_api.rs → syntax/structural/mod.rs → config.rs → public_api.rs | #375 | passive |  |
| 200 | radius.rs | cache.rs | 6 | 1 | radius.rs → cache.rs → config.rs → build.rs → survey.rs → turn.rs → radius.rs | #372 | passive |  |
| 201 | radius.rs | config.rs | 7 | 1 | radius.rs → config.rs → build.rs → survey.rs → turn.rs → radius.rs | #374 | passive |  |
| 202 | radius.rs | turn.rs | 9 | 1 | radius.rs → turn.rs → radius.rs | #378 | F10 |  |
| 203 | ratchet.rs | check.rs | 7 | 1 | ratchet.rs → check.rs → complexity.rs → ratchet.rs | #376 | F7b |  |
| 204 | ratchet.rs | config.rs | 8 | 1 | ratchet.rs → config.rs → ratchet.rs | #374 | passive |  |
| 205 | reachability.rs | base.rs | 15 | 1 | reachability.rs → base.rs → check.rs → reachability.rs | #375 | passive |  |
| 206 | reachability.rs | check.rs | 16 | 1 | reachability.rs → check.rs → reachability.rs | #376 | F7b |  |
| 207 | reachability.rs | config.rs | 17 | 1 | reachability.rs → config.rs → check.rs → reachability.rs | #374 | passive |  |
| 208 | reachability.rs | coverage.rs | 18 | 1 | reachability.rs → coverage.rs → check.rs → reachability.rs | #375 | passive |  |
| 209 | reachability.rs | files.rs | 19 | 1 | reachability.rs → files.rs → config.rs → check.rs → reachability.rs | #375 | passive |  |
| 210 | reachability.rs | project.rs | 20 | 1 | reachability.rs → project.rs → base.rs → check.rs → reachability.rs | #375 | passive |  |
| 211 | reachability.rs | ratchet.rs | 21 | 1 | reachability.rs → ratchet.rs → check.rs → reachability.rs | #376 | passive |  |
| 212 | reachability.rs | reference.rs | 22 | 3 | reachability.rs → reference.rs → check.rs → reachability.rs | #373 | F3 |  |
| 213 | reachability.rs | scope.rs | 23 | 1 | reachability.rs → scope.rs → config.rs → check.rs → reachability.rs | #374 | passive |  |
| 214 | reachability.rs | survey.rs | 24 | 1 | reachability.rs → survey.rs → cache.rs → config.rs → check.rs → reachability.rs | #375 | passive |  |
| 215 | reachability.rs | syntax/structural/mod.rs | 25 | 1 | reachability.rs → syntax/structural/mod.rs → config.rs → check.rs → reachability.rs | #375 | passive |  |
| 216 | reachability.rs | syntax/mod.rs | 28 | 1 | reachability.rs → syntax/mod.rs → check.rs → reachability.rs | #375 | passive |  |
| 217 | reachability.rs | cache.rs | 29 | 1 | reachability.rs → cache.rs → config.rs → check.rs → reachability.rs | #372 | passive |  |
| 218 | reachability.rs | changed.rs | 29 | 1 | reachability.rs → changed.rs → config.rs → check.rs → reachability.rs | #372 | passive |  |
| 219 | reference.rs | config.rs | 13 | 1 | reference.rs → config.rs → reference.rs | #373 | passive |  |
| 220 | reference.rs | check.rs | 14 | 1 | reference.rs → check.rs → reference.rs | #373 | passive, F7d | #376 |
| 221 | reference.rs | doc_citations.rs | 14 | 1 | reference.rs → doc_citations.rs → reference.rs | #373 | passive |  |
| 222 | sarif.rs | base.rs | 15 | 1 | sarif.rs → base.rs → check.rs → sarif.rs | #375 | passive |  |
| 223 | sarif.rs | check.rs | 16 | 1 | sarif.rs → check.rs → sarif.rs | #376 | F7b |  |
| 224 | sarif.rs | config.rs | 17 | 2 | sarif.rs → config.rs → check.rs → sarif.rs | #374 | passive |  |
| 225 | sarif.rs | coverage.rs | 18 | 1 | sarif.rs → coverage.rs → check.rs → sarif.rs | #375 | passive |  |
| 226 | sarif.rs | hunks.rs | 19 | 1 | sarif.rs → hunks.rs → config.rs → check.rs → sarif.rs | #372 | passive |  |
| 227 | sarif.rs | project.rs | 20 | 1 | sarif.rs → project.rs → base.rs → check.rs → sarif.rs | #375 | passive |  |
| 228 | sarif.rs | ratchet.rs | 21 | 1 | sarif.rs → ratchet.rs → check.rs → sarif.rs | #376 | passive |  |
| 229 | sarif.rs | reference.rs | 22 | 4 | sarif.rs → reference.rs → check.rs → sarif.rs | #373 | F3 |  |
| 230 | scope.rs | config.rs | 11 | 1 | scope.rs → config.rs → build.rs → scope.rs | #374 | passive |  |
| 231 | scope.rs | ratchet.rs | 12 | 1 | scope.rs → ratchet.rs → check.rs → complexity.rs → scope.rs | #372 | F2 |  |
| 232 | scope.rs | reference.rs | 13 | 3 | scope.rs → reference.rs → check.rs → complexity.rs → scope.rs | #373 | F3 |  |
| 233 | stubs.rs | check.rs | 3 | 1 | stubs.rs → check.rs → stubs.rs | #376 | F7b |  |
| 234 | stubs.rs | config.rs | 4 | 1 | stubs.rs → config.rs → check.rs → stubs.rs | #372 | F1 |  |
| 235 | stubs.rs | markers.rs | 5 | 1 | stubs.rs → markers.rs → check.rs → stubs.rs | #376 | passive |  |
| 236 | stubs.rs | ratchet.rs | 6 | 1 | stubs.rs → ratchet.rs → check.rs → stubs.rs | #376 | passive |  |
| 237 | stubs.rs | reference.rs | 7 | 1 | stubs.rs → reference.rs → check.rs → stubs.rs | #373 | F3 |  |
| 238 | stubs.rs | scope.rs | 8 | 1 | stubs.rs → scope.rs → config.rs → check.rs → stubs.rs | #374 | passive |  |
| 239 | surface/mod.rs | modules/mod.rs | 13 | 1 | surface/mod.rs → modules/mod.rs → syntax/structural/mod.rs → config.rs → check.rs → surface/mod.rs | #375 | passive |  |
| 240 | surface/mod.rs | syntax/structural/mod.rs | 14 | 1 | surface/mod.rs → syntax/structural/mod.rs → config.rs → check.rs → surface/mod.rs | #375 | passive |  |
| 241 | surface/mod.rs | surface/rust.rs | 24 | 1 | surface/mod.rs → surface/rust.rs → surface/mod.rs | #427 | passive |  |
| 242 | surface/mod.rs | surface/typescript.rs | 25 | 2 | surface/mod.rs → surface/typescript.rs → surface/mod.rs | #427 | passive |  |
| 243 | surface/rust.rs | surface/mod.rs | 11 | 2 | surface/rust.rs → surface/mod.rs → surface/rust.rs | #427 | F11 |  |
| 244 | surface/rust.rs | modules/mod.rs | 12 | 1 | surface/rust.rs → modules/mod.rs → syntax/structural/mod.rs → config.rs → check.rs → surface/mod.rs → surface/rust.rs | #375 | passive |  |
| 245 | surface/rust.rs | syntax/structural/mod.rs | 13 | 1 | surface/rust.rs → syntax/structural/mod.rs → config.rs → check.rs → surface/mod.rs → surface/rust.rs | #375 | passive |  |
| 246 | surface/typescript.rs | surface/mod.rs | 13 | 1 | surface/typescript.rs → surface/mod.rs → surface/typescript.rs | #427 | F11 |  |
| 247 | surface/typescript.rs | modules/mod.rs | 14 | 1 | surface/typescript.rs → modules/mod.rs → syntax/structural/mod.rs → config.rs → check.rs → surface/mod.rs → surface/typescript.rs | #375 | passive |  |
| 248 | surface/typescript.rs | survey.rs | 15 | 1 | surface/typescript.rs → survey.rs → cache.rs → config.rs → check.rs → surface/mod.rs → surface/typescript.rs | #375 | passive |  |
| 249 | surface/typescript.rs | syntax/structural/mod.rs | 16 | 1 | surface/typescript.rs → syntax/structural/mod.rs → config.rs → check.rs → surface/mod.rs → surface/typescript.rs | #375 | passive |  |
| 250 | survey.rs | project.rs | 12 | 1 | survey.rs → project.rs → survey.rs | #378 | passive, F9 | #379 |
| 251 | survey.rs | scope.rs | 13 | 1 | survey.rs → scope.rs → config.rs → build.rs → survey.rs | #374 | passive |  |
| 252 | survey.rs | cache.rs | 14 | 1 | survey.rs → cache.rs → config.rs → build.rs → survey.rs | #372 | passive |  |
| 253 | survey.rs | files.rs | 14 | 1 | survey.rs → files.rs → project.rs → survey.rs | #377 | passive |  |
| 254 | survey.rs | turn.rs | 14 | 1 | survey.rs → turn.rs → base.rs → project.rs → survey.rs | #378 | F10 |  |
| 255 | syntax/convention.rs | ratchet.rs | 8 | 1 | syntax/convention.rs → ratchet.rs → check.rs → inventory.rs → syntax/convention.rs | #372 | F2 |  |
| 256 | syntax/convention.rs | syntax/mod.rs | 9 | 1 | syntax/convention.rs → syntax/mod.rs → check.rs → inventory.rs → syntax/convention.rs | #375 | passive |  |
| 257 | syntax/mod.rs | check.rs | 12 | 1 | syntax/mod.rs → check.rs → syntax/mod.rs | #375 | F6 |  |
| 258 | syntax/mod.rs | config.rs | 13 | 1 | syntax/mod.rs → config.rs → check.rs → syntax/mod.rs | #372 | F1 |  |
| 259 | syntax/mod.rs | ratchet.rs | 14 | 1 | syntax/mod.rs → ratchet.rs → check.rs → syntax/mod.rs | #372 | F2 |  |
| 260 | syntax/mod.rs | reference.rs | 15 | 1 | syntax/mod.rs → reference.rs → check.rs → syntax/mod.rs | #373 | F3 |  |
| 261 | syntax/pattern.rs | reference.rs | 19 | 1 | syntax/pattern.rs → reference.rs → check.rs → conventions.rs → syntax/pattern.rs | #373 | F3 |  |
| 262 | syntax/pattern.rs | syntax/mod.rs | 20 | 2 | syntax/pattern.rs → syntax/mod.rs → check.rs → conventions.rs → syntax/pattern.rs | #375 | passive |  |
| 263 | syntax/structural/cache.rs | syntax/structural/mod.rs | 12 | 1 | syntax/structural/cache.rs → syntax/structural/mod.rs → syntax/structural/cache.rs | #427 | F11 |  |
| 264 | syntax/structural/cache.rs | syntax/mod.rs | 16 | 1 | syntax/structural/cache.rs → syntax/mod.rs → check.rs → base.rs → syntax/structural/mod.rs → syntax/structural/cache.rs | #375 | passive |  |
| 265 | syntax/structural/mod.rs | changed.rs | 17 | 1 | syntax/structural/mod.rs → changed.rs → config.rs → public_api.rs → syntax/structural/mod.rs | #372 | passive |  |
| 266 | syntax/structural/mod.rs | config.rs | 18 | 1 | syntax/structural/mod.rs → config.rs → public_api.rs → syntax/structural/mod.rs | #374 | passive |  |
| 267 | syntax/structural/mod.rs | coverage.rs | 19 | 1 | syntax/structural/mod.rs → coverage.rs → syntax/structural/mod.rs | #379 | F9 |  |
| 268 | syntax/structural/mod.rs | files.rs | 20 | 1 | syntax/structural/mod.rs → files.rs → project.rs → syntax/structural/mod.rs | #377 | passive, F9 | #379 |
| 269 | syntax/structural/mod.rs | project.rs | 21 | 1 | syntax/structural/mod.rs → project.rs → syntax/structural/mod.rs | #379 | F9 |  |
| 270 | syntax/structural/mod.rs | syntax/mod.rs | 22 | 4 | syntax/structural/mod.rs → syntax/mod.rs → check.rs → base.rs → syntax/structural/mod.rs | #375 | passive |  |
| 271 | syntax/structural/mod.rs | syntax/convention.rs | 23 | 1 | syntax/structural/mod.rs → syntax/convention.rs → ratchet.rs → check.rs → base.rs → syntax/structural/mod.rs | #375 | passive |  |
| 272 | syntax/structural/mod.rs | syntax/structural/cache.rs | 31 | 1 | syntax/structural/mod.rs → syntax/structural/cache.rs → syntax/structural/mod.rs | #427 | passive |  |
| 273 | syntax/structural/mod.rs | syntax/structural/rust.rs | 709 | 1 | syntax/structural/mod.rs → syntax/structural/rust.rs → syntax/structural/mod.rs | #427 | passive |  |
| 274 | syntax/structural/mod.rs | syntax/structural/typescript.rs | 710 | 1 | syntax/structural/mod.rs → syntax/structural/typescript.rs → syntax/structural/mod.rs | #427 | passive |  |
| 275 | syntax/structural/rust.rs | syntax/mod.rs | 7 | 1 | syntax/structural/rust.rs → syntax/mod.rs → check.rs → base.rs → syntax/structural/mod.rs → syntax/structural/rust.rs | #375 | passive |  |
| 276 | syntax/structural/rust.rs | syntax/structural/mod.rs | 9 | 1 | syntax/structural/rust.rs → syntax/structural/mod.rs → syntax/structural/rust.rs | #427 | F11 |  |
| 277 | syntax/structural/typescript.rs | syntax/structural/mod.rs | 12 | 1 | syntax/structural/typescript.rs → syntax/structural/mod.rs → syntax/structural/typescript.rs | #427 | F11 |  |
| 278 | syntax/structural/typescript.rs | syntax/mod.rs | 15 | 1 | syntax/structural/typescript.rs → syntax/mod.rs → check.rs → base.rs → syntax/structural/mod.rs → syntax/structural/typescript.rs | #375 | passive |  |
| 279 | turn.rs | base.rs | 8 | 1 | turn.rs → base.rs → check.rs → inventory.rs → turn.rs | #378 | passive |  |
| 280 | turn.rs | config.rs | 9 | 1 | turn.rs → config.rs → build.rs → survey.rs → turn.rs | #374 | passive |  |
| 281 | turn.rs | journal.rs | 13 | 1 | turn.rs → journal.rs → config.rs → build.rs → survey.rs → turn.rs | #374 | passive |  |
| 282 | turn.rs | radius.rs | 14 | 1 | turn.rs → radius.rs → turn.rs | #378 | passive |  |
