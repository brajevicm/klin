# TypeScript local aliases — 2026-10-03

Issue #445 repairs the shipped layering graph. This does not admit the
component-cycle research candidate. Direct `paths` aliases now use the same
conservative candidate rule as relative TypeScript dependencies. Recognized
local names that cannot be proved are located graph holes. Layering consumes
all relevant holes; public-api consumes alias holes only at re-export sites.

## Bounded corpus survey

[survey.py](typescript-aliases-2026-10-03/survey.py) inventories the existing
#343 and #357 clone collections used by #355, without fetching new projects.
[survey.json](typescript-aliases-2026-10-03/survey.json) records each exact
commit, configuration fields, site counts and one example per shape. It reads
held files from those commits, not potentially modified working trees.
Static, single-line `import`/`export ... from` sites matching explicit paths
rules are counted. Multiline clauses, side-effect imports, inherited-only
rules, bundler aliases, dynamic imports and `require` are outside this survey's
site counter. Counts measure recognizable configuration shapes, not proved
edges or complete import coverage. Each site is classified by the first
applicable obstacle: multiple/nested configs, extends, references, multiple
targets, then single direct mapping. Configuration records preserve overlapping
obstacles and baseUrl-only configurations even where no paths site is counted.

| Shape | Sites |
|---|---:|
| Single direct paths mapping | 994 |
| Multiple or nested configurations | 4,901 |
| Package extends without overlapping owner configurations | 731 |
| Local extends as the first obstacle | 0 |
| Multiple targets as the first obstacle | 0 |
| References as the first obstacle | 0 |

OpenStock has one config, no extends or references, and `@/* -> ./*` without
baseUrl: 244 recognized sites. Qinglong contributes 134 direct sites, Seelen-UI
32, Giselle 338, webapp-conversation 208, and the remaining direct projects 38.
The available clone directory also contains ktransformers (10 direct sites);
that supplemental row is retained in the raw inventory, not claimed as a new
#355 sample. Nested/alternate configs dominate the harder projects: Apollo,
Yaak, Tolaria, Zero, js-recall and figma-plugin. Karakeep supplies 731 sites
under package extends. Local extends and references appear in the config
inventory, often overlapping nested ownership. BaseUrl without paths occurs
but supplies no explicit alias evidence; bundler aliases are not inspected.

This supports the initial direct subset. JSONC comments and trailing commas
are needed by real configurations. The implementation recognizes paths in
held local extends files, bounded by a visited set, but does not use inheritance
to prove edges. Package extends are never loaded through node_modules.
Supporting arbitrary project selection or Node resolution would add substantial
semantics beyond the initial repair. The root-proof correction below narrows
which of these recognized sites can become proven edges.

## Proof boundary

A single conventional `tsconfig.json` above a proven project root can prove
an exact or single-star mapping with one target. Root proof uses the
conservative subset described in the correction below. Exact names precede wildcard names;
the longest wildcard prefix wins. Equal-priority patterns are conservatively
unresolved. Targets are anchored at a directly written baseUrl, or otherwise
at the defining config's directory. Include/exclude lists participate in root
proof for the importing source, while the target need only be held: an import
can reach a file omitted from a compiler root list.

Multiple ancestor configs, alternate tsconfig filenames, extends, project
references, multiple targets, invalid anchors, missing or ambiguous TypeScript
candidates and paths leaving the tree cannot prove an edge. A matching rule
still makes these sites local incompleteness. Single local extends chains
supply the nearest paths object's recognized names; they never select a target. Unreadable configs and unknown
package extends cannot supply names that klin has not observed. Absolute specifiers are not remapped. Standalone
baseUrl lookup, package imports/exports, bundler aliases and dynamic dependencies
remain outside the proof boundary. A target the tree holds as another kind,
or that git ignores, retains the existing outside-V1 classification.

## Validation and cost

CLI regression tests cover forbidden and cyclic alias edges, config-only
retargeting, packages, other file kinds and ignored targets, fallback targets,
project ownership, held/new incompleteness, JSONC and rule priority, local
extends recognition, and claim-local public surface holes. The #361 alias
fixture and #355 attack-alias fixture are replayed with the shipped binary.
[replay.py](typescript-aliases-2026-10-03/replay.py) and
[replay.json](typescript-aliases-2026-10-03/replay.json) preserve the commands
and results: both report the alias's forbidden edge, CI exits 1, and Stop
refuses with exit 2. The #355 fixture's component cycle is not a module
cycle: its replay pins a db-to-ui prohibition to inspect the restored edge,
without adding a component-cycle check.

[openstock.json](typescript-aliases-2026-10-03/openstock.json) compares three
binaries over the survey's exact OpenStock commit, with an empty base and
one source layer: `7104995b` before alias support, `618bb06f` with the first
root-proof correction, and `e1507890` with recursive include patterns.
Dependency sites go from 4 to 4 to 258 (257 distinct edges), with 112 modules
in every build. At `618bb06f` the config's `**/*.ts` and `**/*.tsx` include
entries were not supported, so no source had root proof. That run reports
254 `unresolved` alias sites. At `e1507890` it reports none. Every run still
exits 2 because the structural grammar rejects `lib/constants.ts` and
`lib/markets.ts`. The graph repair does not claim that every other
measurement limitation disappeared.

The controlled workload is `tests/performance.rs`'s `structural_1m`, `warm20`:
5,000 files per language, 1,033,827 source lines, five timed warm stops. Commands:

```sh
KLIN_PERF_ROW=structural_1m KLIN_PERF_CASE=warm20 \
  cargo test --release --test performance performance_fixture -- --ignored --nocapture
```

The before build is commit `7104995b8bd949e2ada0dd7b46cd2e6369b37793`.
Measurements run sequentially on this macOS arm64 machine. The workload retains
its original relative imports and configuration; it isolates ordinary graph
cost rather than representing alias-heavy project-system coverage. A separate
alias-heavy run is recorded beside it. Graph milliseconds cover both trees
per gate; they are rounded instrumentation values, not process timing.

The supplementary variant sets `KLIN_PERF_PATHS=1`. After verifying the
ordinary generator's fingerprint, it rewrites 4,952 top-level TypeScript
relative dependency sites to `@/* -> ./src/*` and commits that tree as the
base. The ordinary warm20 edits then regenerate their selected source files,
so those changed files retain relative imports; the remaining alias sites
exercise resolution over cached facts. The printed generator fingerprint
identifies the pre-transform source, not a digest of the alias variant.

All 1,494 ordinary tests passed (one expensive performance test ignored),
along with formatting, Clippy and the repository's gates. Separate Standards
and Spec reviewers found no hard violations; the Standards naming suggestion
was applied by renaming the factual hole flag to `local_alias`.

[cost.json](typescript-aliases-2026-10-03/cost.json) and the four accompanying
logs preserve the five-sample medians:

| Build / source form | Warm stop ms | Layering graph ms | Public-api graph ms | Dependency sites per graph |
|---|---:|---:|---:|---:|
| Before / relative | 1,324 | 31 | 19 | 9,906 |
| After / relative | 1,275 | 29 | 19 | 9,906 |
| Before / paths | 1,293 | 11 | 9 | 18 |
| After / paths | 1,340 | 33 | 21 | 9,906 |

The ordinary row has no measurable graph regression. The alias variant adds
4 ms and 2 ms to the equivalent, fully resolved relative graph, below the
10 ms added-resolution target. Compared with the old incomplete alias graph,
it adds 22 ms and 12 ms: investigation shows 9,888 previously invisible sites
now become actual dependencies. That comparison is **34 ms aggregate**, above the comment’s 25 ms
threshold; the previous wording incorrectly treated each consumer separately.
The product evidence is the 9,888 restored dependency sites and the forbidden
edges recovered in the replays above. The equivalent resolved relative graph
is the control for isolating alias matching cost. Both consumers
still report zero extra source reads/parses. Total Stop differences include
other gates and process/cache variation; they are not attributed to aliases.

[TypeScript's paths reference](https://www.typescriptlang.org/docs/handbook/modules/reference.html#paths)
was checked through Context7 for anchoring and wildcard precedence.

## Follow-up: compile alias lookup and ownership

The performance comment on #445 requires a direct exact-alias map and wildcard
ordering once per tree. The original implementation collected owners and
sorted matching rules per import. The follow-up compiles shared directory
scopes, with exact rules in a hash map and wildcards ordered by prefix length.
Each source file selects its scope before its import loop. Overlapping configs
remain unproved; selecting the deepest scope does not infer TypeScript project
ownership, because that scope already includes every ancestor config.

[indexed-cost.json](typescript-aliases-2026-10-03/indexed-cost.json) records
sequential five-sample runs before at `30193ad0` and after the follow-up, on
the same machine and fixture as above. Run the ordinary and alias variants
with `KLIN_PERF_ROW=structural_1m KLIN_PERF_CASE=warm20`, adding
`KLIN_PERF_PATHS=1` for aliases, through the ignored release performance test.

| Build / source form | Warm stop ms | Layering graph ms | Public-api graph ms | Aggregate graph ms |
|---|---:|---:|---:|---:|
| Before indexing / relative | 1,319 | 29 | 19 | 48 |
| Before indexing / paths | 1,322 | 32 | 21 | 53 |
| Indexed / relative | 1,301 | 29 | 19 | 48 |
| Indexed / paths | 1,306 | 30 | 20 | 50 |

All rows retain 9,906 dependency sites per graph and zero source reads/parses
in both consumers. The ordinary row adds 0 ms; alias matching adds 2 ms over
the equivalent complete relative graph, within the comment's preferred
5 ms budget. Aggregate values sum the two rounded gate medians. The old
pre-feature alias graph still cannot serve as an equivalent graph-cost
control: it omitted almost every aliased edge. The restored forbidden-edge
replays above provide the explicit product evidence for that larger delta.

Follow-up validation: 1,496 ordinary tests passed, with the performance test
ignored in the full suite and run separately for the four rows above.
Formatting, Clippy and repository gates passed. Independent Standards and
Spec reviews reported zero material findings.

## Follow-up: configured roots and inherited paths replacement

PR #461's adversarial review found that ancestry alone could apply a config
to a source outside its configured roots. Alias edges now require root proof
through explicit files or the conservative include/exclude subset in SPEC
8.2.1. Sources remain held modules even when root proof is unavailable;
recognized aliases become local incompleteness. Exclude filters include roots,
not explicit files or target modules. Import reachability is not used to infer
membership. Root proof is computed once per source file.

Inherited alias recognition also stops at the nearest defined paths option
in a single held local extends chain. Child paths replace the whole parent
object, including an empty child object. Array-form extends supplies no
inherited names; direct child paths remain recognized. Extends still never
proves an edge.

Scope selection now looks up ancestor directories in a map once per source
file instead of scanning every config scope. This addresses the review's
config-heavy selection concern without a full TypeScript project resolver.
Config scopes and matching rules are still compiled once per tree.

[root-proof-cost.json](typescript-aliases-2026-10-03/root-proof-cost.json)
records fresh sequential five-sample release runs using the same 1M / warm20
fixture. Ordinary graph time is 30 + 19 = 49 ms; aliases are 30 + 20 = 50 ms.
The earlier indexed measurements were 48 and 50 ms respectively. The ordinary
increase is 1 ms and alias overhead is 1 ms over the equivalent complete
relative graph. Both retain 9,906 dependencies per consumer and zero source
reads/parses. Warm Stop medians are 1,285 ms and 1,299 ms; differences include
unrelated gate and process variation. These runs exercise one alias config;
the config-heavy improvement is structural indexing, not a measured claim.


The OpenStock graph counts above were measured again after the
recursive-include correction (2026-10-04). The
#361 and #355 planted routes were replayed again after the correction: CI
still reports `domain → ui: src/ui/format.ts` and `db → ui: src/ui/labels.ts`
respectively with exit 1, and both Stop replays refuse with exit 2.

Correction validation: 1,504 ordinary tests passed (one performance test
ignored and exercised separately above), formatting, Clippy and repository
gates passed, and Standards and Spec reviews reported zero material findings.
