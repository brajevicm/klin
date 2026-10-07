# Release notes

## Unreleased

### Six commands: setup, check, policy, report and update (#507)

klin's public CLI is now `klin setup`, `klin check [CHECK...]`, `klin policy [SECTION]`, `klin report` and `klin update`. `klin status` follows in #498. The old commands are gone, and a removed command is an unknown command that exits 2.

- `klin check` judges the way `klin gate --strict` did. Name checks to run only those: `klin check complexity`. It takes `--changed`, `--json` and `--config`. The Action and klin's own CI run it.
- `klin setup` replaces `klin init` and `klin install`. It writes `{}` when no `klin.json` exists and reconciles the host hooks and the skill. `--pin` writes the values `klin init --pin` wrote.
- `klin policy` replaces `klin gate --list` and `klin reference`. It prints each value a check uses with where it came from. `--reference` and `--schema` print the reference and the JSON schema. `klin policy public-api` lists the derived public surface, and `klin policy conventions [NAME]` explains each convention.
- `klin report` replaces `klin stats`. It reads the newest session by default, and `--since 7d` reads the last seven days. `--details` replaces `--all`.
- The per-check commands and their own options are gone: `--only`, `--quiet`, `--list-languages`, `doc-size --file`, `doc-citations --file` and `dead-symbols --report`.

### A `cc` floor of 10, `@ts-expect-error` in tests, and named re-exports (#411)

The derived `complexity.cc` floor rises from 5 to 10. A tree whose 95th
percentile of `cc` is below 10, or that has fewer than 50 functions, now gets
a ceiling of 10. A pinned `cc` wins over the floor, so `"cc": 5` keeps the old
ceiling (ADR 0059).

`@ts-expect-error` in a TypeScript or JavaScript test file no longer fails
`escapes`. It has a row of its own, `ts-expect-error`, and production code is
judged for it as before. `@ts-ignore`, `@ts-nocheck` and the non-null `!` are
escapes in test files too. The key `skip_rust_tests` is renamed
`skip_test_idioms` and covers both the Rust `unwrap`/`expect` idioms and this
one. A `klin.json` that still names `skip_rust_tests` exits 2 and asks for the
rename. The coverage line says `in tests skipped` (ADR 0060).

`reachability` reads `export { x } from "./m"` and
`export { x as y } from "./m"` as a reference to `x`, so a family member that
only a re-export names is reached. `export *` names nothing, as before, and
`dead-symbols` is unchanged (ADR 0061).

ADR 0062 lists the relaxations #389 rejected and why.

### `lockfile` checks each pin against the version the lockfile records (#305)

A new value, `stale`, is 1 when a dependency's specifier is exact and the
lockfile does not record that version for the manifest. Before this, a
manifest could pin `typescript` at `5.6.3` while `package-lock.json` still
installed `5.4.0`, and the gate passed. Staleness the base already had is
held. The remedy now has one part for each value that failed, so a
new dependency with no lockfile entry is told to run the project's install.
The old remedy asked it to restore a base pin it never had.

An accepted `lockfile` entry written before this release stays valid. With no
`stale`, it holds a `stale` of 0.

### Generated `klin.json` schema (#181)

The generated SchemaStore artifact now shares its native Rust structural
contract with configuration loading, including the root object, `journal`
prompt, false disables, scopes, ceilings, conventions, layering, SARIF and
required build-entry fields. Runtime semantic checks remain in their owning
gates; normal gate and hook paths do not read the schema.

The final rebased release binary was 22,306,768 bytes versus 22,224,528 at
`origin/main` (+0.37%). The 2k/10k file-count and 1M source-dense rows stayed
within 6.1% with identical fixture counters and digests. The complete 300k
source-dense observations (warm/cold/strict) were base
`776/25,977/13,967`, `879/31,846/22,053` and final
`768/56,319/29,587`, `1,081/43,415/23,864`, `1,300/43,171/22,764`; the
interleaved spread is diagnostic noise rather than evidence for a
schema-related hot-path change. The controlled #193 dense measurements remain
the release baseline.

### `klin stats` is an attention and value report (#172)

The default report answers three questions in order: does anything need you,
what did klin catch and what became of it, and can you trust the summary. It
opens with the strongest condition the window holds. Measurement doubt comes
first, then open regressions, then the uncertainty a reset left, then what was
resolved. It names at most three open sites and points at `klin stats --all`
for the rest. The stop count, the timing, the guard history and the
previous-window comparison left the default and stayed in `--all` and `--json`.

The counted unit is the Regression: one finding site a blocked stop put in
front of the agent, counted once per window with its latest outcome. The word
shortcut is gone from every text klin prints for a person. A regression is
keyed by the finding id of spec 11.2, and a record that carries none, such as
`doc-size`, falls back to gate, file, line and text. A rename produces a
different id, and the reader counts a renamed site twice. Merging two ids whose
text resembles each other would be the worse error.

Two rules read what the record already held. The `config_hash` the journal
recorded and never read now separates a code fix from a policy change: a
regression that goes from a measurement taken under another configuration reads
`Resolved after the config changed.` A site absent from a stop that measured
its gate went, whether or not another site kept that gate red, so two sites
under one gate resolve apart.

No copy says who wrote a fix. The journal proves a regression was present and
later absent from a measurement, so the report says `All 12 were fixed after
klin flagged them.` The form `The agent fixed all 12.` is forbidden.

Measurement confidence is one decision over unparsed and lost files,
not-measured files, unresolved evidence, gate `ERR` rows and skipped journal
lines. It outranks the value claim, so `Nothing needs your attention.` is
printed only where klin measured the window whole.

The words a report gives a gate's findings are now presentation metadata on the
catalogue row, a singular and a plural, and a new row does not compile without
them. A gate the binary has no row for still prints under its recorded name.

`--json` prints one episode per regression identity and dropped the grouped
`more` count. Turn-end and weekly messages use the same vocabulary and the same
counting rule. `stats::turn_end` still reads the bounded journal tail of #184.
Spec 9.5 and 11.5, `CONTEXT.md`, ADR 0034 and the journal/stats design document
carry the decisions, and `tests/stats.rs` pins the behaviour.

### Derived public API compatibility (#46)

A new Automatic gate, `public-api`, fails when a consumer-facing Rust or
TypeScript contract the base exposed is gone or its declared contract changed.
Klin derives public API from standard Rust library and TypeScript package
entry points, so a project configures nothing: the section is absent, or
`false` to exclude the gate, and any object under it is refused.

A Rust surface is a Cargo library target, named by its crate name, and its
items are what a consumer writes: root `pub` items, `pub mod` chains, `pub
use` re-exports with their aliases and globs, and public inherent methods
under their type. A binary target has no surface. A TypeScript surface begins
only at explicit package metadata that names a checked-in source file, and
its items are the exported declarations, defaults, clauses and relative
re-exports reachable from that entry. Generated JavaScript is never mapped
back to source. An item is measured where its declared contract is canonical
and opaque where klin proves only that it exists; a type the compiler would
infer is written as `?`. A form klin recognizes and cannot list is a hole: a
NOTE in the hook and exit 2 elsewhere. An intentional break is an accepted
entry under the surface and item identity. `klin public-api --report` prints
the derived contract. ADR 0044 records the boundary, and `tests/public_api.rs`
pins the behaviour.

The structural adapters now extract visibility, nesting, export names,
inherent-method owners, canonical signatures and export clauses, and the
structural cache format changed to carry them, so the first changed run after
an upgrade extracts the base again. The module graph names its targets and
exposes each module's tree and one path resolver. A gate row in `--json` now
carries `surface`, and the dense performance rows derive one surface per
language and print its counters.

The controlled 2026-09-15 dense before/after run stayed within the large-
repository budgets: the 20-file warm hook moved from 2,055 to 2,184 ms at
300k and from 2,566 to 2,704 ms at 1M. Public-api itself took 33/35 ms, read
and parsed zero structural facts, and derived four surfaces with 26/56 items;
no row regressed by more than one third. Full 100-file, cold, strict, cache
and RSS evidence is in `docs/structural-views-2026-09-15.md`.

### Layering and dependency cycles (#50)

A new Policy gate, `layering`, fails when a dependency crosses a layer the
section does not let it use, and, with `acyclic: true`, when a dependency
closes a module cycle the base did not hold. A layer is a name, an `in` path
and a `can_use` list. The section also takes the shared `in` and `except`.
With no section the gate does not run.

The gate reads one module graph per tree. Rust targets come from Cargo
manifests through `cargo_toml`, over the file list the run already holds, and
from the conventional library, main and bin roots under a src directory where
no manifest is usable.
Rust `mod` declarations, literal `#[path]` attributes and `crate`, `self` and
`super` paths resolve to modules, and TypeScript relative imports resolve to
files. A form the resolver cannot prove is a NOTE in the hook and exit 2
elsewhere, and a bare path or package import is counted, never guessed.
`petgraph` finds the cycles. ADR 0043 records the boundary, and
`tests/layering.rs` pins the behaviour.

The structural cache format changed to carry the inline modules that hold
imports, module declarations and qualified paths, so the first changed run
after an upgrade extracts the base again. A gate row in `--json` now carries
`graph`, and the dense performance rows enable `layering` and print its
graph counters.

### Large-repository performance contract (#182)

SPEC 13 now makes the dense structural rows product requirements. The chosen
limits are 5 seconds for a warm Stop hook on both rows, 30 seconds for the
300k cold survey, 60 seconds for the 1M cold survey, 20 seconds for the 300k
whole-tree strict run and 45 seconds for the 1M strict run. They leave
headroom over the measured medians and do not weaken the existing 2k limits.

The decision uses the final controlled release run from #193, not the
intermediate optimization rows. It was measured on 2026-09-15 with release
build `0.1.1` at benchmark commit
`aa2ca0a3bc92a5ee4b234e15bc87e8a5fa4f0856`, five iterations per row, on a
MacBook Pro 18,3 with an Apple M1 Pro, macOS 26.6.2 / Darwin 25.6.0 arm64;
project builds were excluded from hook timing and the warm row used the
structural cache:

| Source row | Files / LoC | Warm, 20 changed | Warm, 100 changed | Cold | Strict |
| --- | ---: | ---: | ---: | ---: | ---: |
| 300k | 10,000 / 325,077 | 2,388 ms | 2,348 ms | 22,834 ms | 12,168 ms |
| 1M | 10,000 / 1,033,827 | 2,733 ms | 3,115 ms | 47,507 ms | 30,724 ms |

The changed-file counters were 40 complexity reads/parses, 40 structural
fact reads/parses/extractions, 40 escapes reads with 20 parses, and 40 stubs
reads/parses for 20 changed files. The 100-file row was 200 complexity
reads/parses, 200 structural fact reads/parses/extractions, 200 escapes reads
with 100 parses, and 200 stubs reads/parses. Peak RSS for warm / warm without cache / changed
`dead-symbols` / strict was 282,336 / 286,944 / 281,312 / 356,704 kB at 300k
and 806,080 / 831,456 / 825,696 / 1,011,728 kB at 1M. One structural cache
snapshot was 8,416,405 and 25,863,431 bytes; the four-snapshot measured
retention envelopes were 33,665,620 and 103,453,724 bytes. Per-gate timings,
work attribution and the fixture contract are in
`docs/structural-views-2026-09-15.md`.

The release checklist now requires an explanation before release when a dense
median misses its budget or rises by more than one third against the preceding
controlled row. Contributor CI still checks deterministic shape and semantics,
not machine-specific timing or RSS.

### A changed run keeps the base's structural facts between runs

In a changed run that is not strict, `dead-symbols` and `reachability` keep
the base commit's structural outcomes in the state directory under
`cache/structural/`. The next Stop over the same base reads them and does not
parse the base again. A cache file that is missing, damaged, or written for
another commit, root or build costs one full extraction and never changes a
verdict. `klin cache clean` removes the cache. Gate rows record `cached`,
`cache_read_ms` and `cache_write_ms` under `facts`. On the 1M-line fixture
with 20 changed files, the warm hook median fell from 7,939 ms to 2,711 ms.
Measurements are in `docs/structural-views-2026-09-15.md`.

### Bounded structural reuse and dense scaling evidence

Structural reuse keeps the four newest base-commit cache files. Missing,
damaged, incompatible and evicted files fall back to the same cold extraction, so
eviction changes cost only. A changed run over a cached base holding imports,
module declarations and an unparsed file parses only the changed file, and the
cache round-trip test pins the import and module fields.

The dense performance fixture keeps its 20-file rows and adds a 100-file warm
row for both source volumes. It asserts that changed-file extraction grows
with the delta while unchanged base facts remain cached and shared. The
controlled measurements and disk/RSS evidence are in
docs/structural-views-2026-09-15.md.
The release run measured 2,220 ms warm and 2,559 ms at 100 changed files on
the 300k row, and 2,860 ms warm and 3,220 ms at 100 changed files on the 1M
row. The corresponding cache files were 8,416,407 and 25,863,432 bytes.

### Structural gates share one extraction per tree

A run extracts each structural file of a tree once, and gate rows record it
under `facts`. Measurements: `docs/structural-extraction-2026-09-14.md`.
Faster index lookups: `docs/structural-hot-path-2026-09-14.md`.

### Source checks take compact scope policy

`complexity`, `escapes`, `stubs`, `dead_symbols` and `reachability` now read
only a person's decisions: `in` and `except`, the complexity `cc` and `lines`
ceilings, `skip_rust_tests` for escapes, and `ignore` for dead symbols. klin
rejects every other key that described the repository (`roots`, `languages`,
`skip_dirs`, `exclude`, `exclude_except`, project `patterns`, nested
`ceilings`, and reachability family lists), and the error names the compact
replacement. `reachability` runs with no section. klin derives its families
from the tree and never writes them to `klin.json`.

A derived complexity ceiling samples the functions that the scope recorded in
the derivation commit's `klin.json` selects. An edit to `in` or `except`
changes what is judged on the next run, and changes the ceiling only when the
derivation commit moves. When today's scope differs from the recorded one, or
klin cannot read the recorded policy, the run prints a NOTE. The Stop hook
passes that NOTE to the person even when nothing blocks.

Measured 2026-09-14 on the 0.1.1 baseline machine, release build, median of
five iterations, before and after the change in the same session. Whole-tree
rows use no `complexity` section:

| Row  | Warm hook before | Warm hook after | Cold survey before | Cold survey after | Strict before | Strict after |
| ---- | ---------------: | --------------: | -----------------: | ----------------: | ------------: | -----------: |
| 2k   |         1,153 ms |        1,528 ms |           1,828 ms |          1,871 ms |      1,156 ms |     1,231 ms |
| 10k  |         4,087 ms |        6,177 ms |          13,646 ms |         14,368 ms |      5,036 ms |     5,712 ms |
| 300k |         9,937 ms |       17,552 ms |          24,932 ms |         31,341 ms |     12,955 ms |    19,190 ms |

The warm hook moved by more than a third at 10k and 300k, mostly because
`reachability` now runs on the fixture without a section: it took 2,002 ms of
the 10k warm hook and 6,512 ms of the 300k one. With the whole repository in
scope, the complexity gate takes the same time as before. A `complexity`
section of `"in": "rust"` roughly halves it. ADR 0039 has the per-gate rows.

### Cold survey reads the derivation commit through one git process

The complexity sample used to read each sampled file with its own `git show`.
It now reads every sampled file through one `git cat-file --batch` process.
Measured 2026-09-13 on the 0.1.1 baseline machine (MacBook Pro 18,3, Apple
M1 Pro, macOS 26.6.2), release build, median of five iterations, before and
after the change in the same session:

| Row | Warm hook before | Warm hook after | Cold survey before | Cold survey after | Strict before | Strict after |
| --- | ---------------: | --------------: | -----------------: | ----------------: | ------------: | ------------: |
| 2k  |         1,703 ms |        1,602 ms |          15,363 ms |          2,034 ms |      1,745 ms |      1,716 ms |
| 10k |         5,451 ms |        5,406 ms |          74,465 ms |         10,755 ms |      6,787 ms |      6,760 ms |

The guard row was 9,025 ms before and 8,992 ms after for 1,000 events. The
warm rows sit above the 0.1.1 table because the machine carried other load
during this session; the before and after columns share that load.

### Source-dense structural fixture

Recorded 2026-09-13 on a MacBook Pro 18,3 with an Apple M1 Pro (8 cores),
macOS 26.6.2, Darwin 25.6.0 arm64, using klin 0.1.1 release build. Each
timing is the median of five iterations. Each row changed 10 Rust and 10
TypeScript files, and hook timings exclude project builds. All rows ran in one
session.

Both dense rows repeat the same structural unit, so they differ in volume and
not in shape: 276.9 and 282.9 declaration lines per 1,000 source lines.

| Row | Files | LoC | Declarations | Warm hook | Cold survey | Strict |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 2k | 2,000 | 16,762 | 2,185 | 1,541 ms | 2,266 ms | 1,638 ms |
| 10k | 10,000 | 84,102 | 10,941 | 5,014 ms | 14,330 ms | 6,285 ms |
| 300k | 10,000 | 325,077 | 90,007 | 17,257 ms | 24,178 ms | 19,068 ms |
| 1M | 10,000 | 1,033,827 | 292,507 | 57,303 ms | 77,136 ms | 64,957 ms |

Per-gate medians. The warm hook row reads them from the journal line of the
stop it timed, and the other rows read them from `--json`. The 2k and 10k rows
configure no reachability section, and the survey derived none.

| Row / mode | Complexity | Dead symbols | Reachability |
| --- | ---: | ---: | ---: |
| 10k warm hook | 564 ms | 2,735 ms | — |
| 10k strict | 1,087 ms | 1,337 ms | — |
| 300k warm hook | 1,578 ms | 6,085 ms | 6,678 ms |
| 300k cold survey | 3,082 ms | 4,658 ms | 5,190 ms |
| 300k strict | 3,069 ms | 4,659 ms | 5,139 ms |
| 1M warm hook | 4,341 ms | 21,008 ms | 25,853 ms |
| 1M cold survey | 8,793 ms | 20,603 ms | 26,193 ms |
| 1M strict | 8,541 ms | 19,522 ms | 24,194 ms |

What the rows show:

- At a fixed 10,000 files, time grows with source volume. From 300k to 1M
  (3.2 times the LoC), strict grows 3.4 times and the warm hook 3.3 times.
- `dead-symbols` and `reachability` take 67% of the 1M strict run. Each grows
  faster than LoC (4.2 and 4.7 times), and `complexity` grows at LoC's rate
  (2.8 times).
- Cold survey minus strict is 5,110 ms at 300k and 12,179 ms at 1M. The
  per-gate times of the two rows match, so this cost sits outside the gates,
  in the survey.
- In the warm hook, `dead-symbols` takes longer than in strict (6,085 against
  4,659 ms at 300k), and this time the numbers come from the same stop that
  was timed. These rows do not explain the difference.
- A gate's `ms` covers reading, parsing, indexing and judging, and each
  structural gate parses its own files. The rows therefore cannot separate
  parse cost from resolution cost, or show a parse two gates repeat.

These are historical pre-#182 rows. The final controlled measurements and the
large-repository product decision are recorded at the top of these notes.

## 0.1.1

### Performance baseline

Recorded 2026-09-12 on a MacBook Pro 18,3 with an Apple M1 Pro (8 cores),
macOS 26.6.2, Darwin 25.6.0 arm64, using klin 0.1.1. Each timing is the
median of five iterations; project builds are excluded from hook timings.

| Row                                 | Warm hook | Cold survey |   Strict |
| ----------------------------------- | --------: | ----------: | -------: |
| 2k (1,000 Rust + 1,000 TypeScript)  |    934 ms |   14,250 ms | 1,406 ms |
| 10k (5,000 Rust + 5,000 TypeScript) |  2,393 ms |   72,828 ms | 5,835 ms |

The guard row processed 1,000 deterministic events per iteration: 8,974 ms
median total, or 8.974 ms/event. Both rows changed 10 Rust and 10 TypeScript
files; the TypeScript counts include 10 and 50 `.tsx` files respectively.
