# Product cost of phenotype capabilities, 2026-10-04

This note belongs to #460 and the frozen #357 study under
`docs/phenotype-study-2026-10-04/`.

## Status

This is the first product-cost synthesis after #456 froze study version 1.
It creates one cost row for every one of the 29 registered phenotypes and
imports completed research only where its measurement basis is usable.

It is intentionally **not a product disposition**. #457 is intentionally
deferred and will not be resumed for this study. #458 therefore cannot produce
the frozen natural-finding intervention-safety results that depend on #457's
registered denominators. Those unavailable UX fields remain `null` with that
provenance; they are not inferred as zero, clean or safe.

#459 remains an independent source of controlled AX/repair-cost evidence. If
its current study-v4 outcomes are available before #460 closes they may be
imported with v4 provenance; otherwise `agent_repair_turns`,
`feedback_bytes` and other unavailable outcome fields remain `null`.
Missing evidence does not block completion of this cost ledger. It remains
available to #357 as an admission blocker wherever the frozen decision rule
requires that evidence.

Older repair experiments are summarized below as prior evidence, not silently
promoted into incompatible frozen-study denominators.

The frozen #456 preregistration baseline remains
`43a139a8a16964a65ceb97b939c3499c9cf90b4d` (klin 0.4.1) as historical study
metadata. For #460 product-cost measurement and comparison, the operational
baseline is the released **klin v0.4.2** tag at
`138dc8d0a927c60df289bd485627f472488cf2ba`.

The v0.4.2 release commit changes release/version metadata only; detector and
performance logic are unchanged from its parent. Evidence reused from earlier
research keeps its original measurement basis, while new #460 measurements
must name v0.4.2 explicitly rather than silently inheriting the preregistration
baseline. #460 does not change a detector, public claim, SPEC contract or
admission threshold.

This ledger stays in the original study directory because its 26-column cost
schema was frozen there. That does not make later study revisions equivalent:
#459's current controlled protocol is study v4 on the same released v0.4.2
runtime. Any v4 outcome imported here must retain explicit v4 provenance; no
outcome is silently relabeled as study v1.

## Ledger encoding

`product-cost.tsv` follows the `cost` record frozen in `schema.json`.
There are 29 data rows and 26 columns.

- `0` means zero by design, for example no external process for a native
  candidate.
- `null` means not measured or not safely attributable on the registered
  basis.
- list-valued fields use ` | ` inside one TSV cell.
- `runtime_summary` is compact JSON with independent `klin`, `external`
  and `project` timing records.
- runtime is not inferred from spare Stop budget.
- `extra_parses` and `source_reads` are measured operations, not shorthand
  for “uses the existing parser.” An architectural plan to reuse a parser is
  kept in `implementation_mechanisms`; an unintegrated candidate stays
  `null` rather than receiving a speculative zero.
- the issue ledger names version pinning separately, while the frozen cost
  schema has no `version_pinning` property. To preserve the frozen schema,
  every row records it explicitly inside `tool_installation`: native rows
  say there is no separately pinned tool, and Ruff names exact 0.16.10.
- shared execution/substrate costs are tagged and **deduplicated** during
  synthesis. In particular the two Ruff rows share execution group
  `ruff-review`, and readiness rows share the one tree-identity/state
  substrate; adding those row values together would double-count product cost.
- project-command time is `0` for these detector measurements because no
  candidate measurement requires the project's build/test command. Project
  commands may still be independent task evidence in #459.

## Evidence reused

### Shipped gates — #361

The shipped-gate repair audit ran 45 prior agent cases: 38 correct repairs,
3 appeasements, 2 harmful repairs and 2 intended contract breaks left for a
person. There were 5 extra turns. This is valuable AX evidence but it predates
the frozen #459 workload, so the ledger does not copy those counts into the
final-study repair fields.

The audit also measured message burden: a single-gate Stop message was
typically 8–10 lines and 88–206 words; the three-gate attention case was
18 lines and 318 words. Reply-only Stop clearance and the current inventory
semantics are product costs. The final natural-finding interruption rate is
unavailable because #457 was deferred and #458 cannot complete that frozen
analysis without its registered denominators.

#425 supplies comparable 1M/20 current-gate timing for three shipped rows:
complexity 27 ms median, dead-symbols 559 ms median, and reachability 303 ms
median across the three baseline rows used in its final 1M experiment. These
are **total current gate times**, not incremental add/remove measurements, so
they cannot be used to admit a new Stop capability.

The controlled dense 1M/warm20 evidence also gives source-work attribution
that the first ledger draft incorrectly left blank. On 20 changed source
files, complexity performed 40 reads / 40 parses, escapes 40 / 20, stubs
40 / 40, and dead-symbol extraction 40 / 40 while reusing 9,980 cached file
facts. Layering and public-api consumed the shared structural facts with
0 additional source reads/parses on that run; reachability likewise reused the
shared facts. The structural cache itself was 37,301,283 bytes, and the same controlled
1M/warm20 run measured 324,864 KiB peak RSS for the warm hook. Both are
**whole-run shared substrate measurements**, not incremental memory owned by
one phenotype. The TSV therefore keeps per-row `memory_bytes`/`cache_bytes`
null where attribution was not isolated and names the shared values in the
measurement holes instead of charging 37.3 MB or ~317 MiB independently to
every consuming phenotype.

One configuration correction is material: shipped `layering`/module-cycle
checking is policy-activated. It requires a person-written `layering`
section; `{}` does **not** activate that gate. The previous draft's
“no configuration beyond `{}`” understated its DX cost.

### Test integrity — #353

The prototype repair study ended 23 of 24 cases in a correct final repair,
with 1 unresolved case, 0 appeasement, 0 harmful repair, 0 escalation and
0 extra repair turns. The blocking copy used by that experiment is not the
future REVIEW experience. #459 study v4 is the compatible source for a
closed-loop result if it completes before #460 closes; otherwise that outcome
remains unknown.

For 20 changed test files, the prototype's two-tree walk cost roughly
15–68 ms using mean per-file walk cost across the measured repositories and
80–550 ms at their p99 per-file costs. The prototype allocates more than an
integrated implementation would. The smallest candidate implementation design reuses the
existing parse/test walk: shared test extraction, one per-test check list,
Rust assertion-macro arguments, TypeScript matcher facts, and before/after
test identity. No second parser is required.

### Design/reuse — #355

The prior experiment produced 29 relation repairs and 4 justified kept
exceptions across 33 runs, with 0 appeasement, 0 over-refactor, 0 harmful
repair, 0 escalation and 1 extra turn. Six repaired trees were not build
verified, so this is not independent proof of behavior.

The untuned prototype suggests a Finalize-scale cost of about 10–290 ms for
20 changed files, plus 0.1–339 ms of base-index work when the needed relation
evidence is not cached. Measured base evidence ranged from about 4–360 KB for
family evidence, 2–700 KB for registration evidence and 3 KB–1.1 MB for
specific-call evidence. Those are evidence payload sizes, not a committed
cache representation, so numeric memory/cache cells stay `null`.

The concrete mechanisms are resolved type/trait identity, resolved callees,
registration binding relations and the existing module graph. No second
parser is required. TypeScript component cycles additionally depend on
complete local/alias resolution; a located #445 hole is incomplete evidence,
not a clean zero.

### Unfinished/error masking — #362

The prior experiment repaired all 33 planted cases correctly, with no
appeasement or harm; one case took an extra turn because the shipped
`escapes` gate caught the first repair, and one run asked about a separate
API-design choice.

The prototype's 20-file mean-walk estimate was 14–29 ms. The five surviving
Python rows can reuse the existing function/handler walk: ellipsis-only and
raise-only function bodies, plus empty/default/broad handler shapes. The
whole-tree name index used by rejected candidates is not part of these rows.

### Dependency evidence — #364

The four prior repair cases across three agents ended 12/12 correct, with
0 appeasement, 0 harmful repair and 0 escalation. Three extra turns came from
the existing complexity gate, not the dependency candidate.

The whole-tree prototype took 0.86–0.97 s on trl, 1.87–1.99 s on
apollo-client and 1.98–2.21 s on tolaria, but that is not the proposed Stop
shape. #465 subsequently measured both proposed Stop candidates on the
controlled structural 1M fixture with exactly 20 changed files (18 source plus
the manifest/lock pair), alternating plain and research runs on the same warm
tree after one discarded warm-up per side.

For `python-uv-lock`, the paired incremental lockfile-path cost over 15
samples was 4 ms median, 5.3 ms p95 and 6 ms max. Its paired whole-Stop delta
was +14 ms median and +121.7 ms p95. For `cargo-lock-entry-shape`, the
isolated path cost was 1 ms median, 2.3 ms p95 and 3 ms max, while its paired
whole-Stop delta was +6 ms median and +247.6 ms p95. The whole-Stop signal is
much noisier than the isolated gate-path delta, so both are reported rather
than attributing all Stop variance to the candidate. The recorded #465
benchmark measured feature head
`e833d414b6557aedb9190bc6540008ca1357dc28` in merge ref
`735b38041a1a5166f6b0618811777cc81b17bf66` over base
`51d3df5782f70887ba3dfc99aea4efc3f5f0ab3e`. The temporary measurement
instrumentation was removed from the merge candidate after the evidence was
captured.

The benchmark did not instrument candidate-attributable `source_reads` or
`extra_parses`, so those fields remain `null`; timing evidence must not be
stretched into unmeasured source-work attribution.

### Ruff named recipe — #363

The surviving external recipe is exactly Ruff 0.16.10 with
`S102,S307,S602,S604,S605,S608` and `BLE001,S110,S112`, using
`--isolated --no-cache --ignore-noqa`. It is readiness/Finalize + CI REVIEW
only and is never moved to ordinary Stop from unused latency.

On the final recipe's complete-measurement run, 25 twenty-file executions had
median 29 ms, p95 42 ms and maximum 44 ms. Whole-tree executions had median
81 ms, p95 210.8 ms and maximum 228 ms. Intrinsic klin adapter time was not
isolated on that machine, so the ledger keeps `klin_runtime_ms=null` and
records only the external time. An older through-klin replay had a 71 ms
per-change median on a different basis and is not added to the final recipe
number.

Operational burden is explicit: one pinned ~21 MB Ruff binary, one external
process, version checking on every run, fixed arguments, isolated environment,
bounded timeout/output, no network, no cache writes, and measurement-basis
versioning. A Ruff version change is non-comparable evidence, not ordinary
finding churn.

The injection and swallowed-error phenotype rows are **not two Ruff
processes**. They are two interpretations of one frozen `ruff-review`
invocation containing both rule families. Each row keeps the 29 ms / one
process bundle cost so it can be evaluated alone, but total-product synthesis
must deduplicate the sibling row. Ruff's own file reads/parses were not
instrumented, so `source_reads` and `extra_parses` are `null`; passing
20 paths is not promoted into a fabricated “20 reads / 20 parses” fact.

The earlier retained-family repair cases were 4/4 correct for injection and
2/4 correct plus 2/4 narrowed-handler appeasement for swallowed errors.
#459 study v4 is the compatible controlled source for remeasurement if it
completes; otherwise this remains prior evidence rather than a frozen-study
rate.

### Shared identity and lifecycle — #352 / #425 / #452

#352 measured the shared readiness substrate that the first draft omitted.
On its 10,000-file / roughly-1M-line synthetic tree, a **kept private Git
index** produced one tree identity in about 38 ms both clean and with 20 files
changed. A slow readiness measurement needs identity before and after, so the
measured shared binding cost is about **80 ms per readiness run**, before the
candidate analysis itself. An immutable copy cost roughly 0.8–0.9 s and was
rejected as the default. This cost belongs to the readiness run once; it is
not added independently for every REVIEW phenotype that happens to execute in
that run.

#352 also exposed host/storage cost that must not disappear from the ledger.
Its proposed state under `.git` was not writable in the default Codex
sandbox, and Cursor app evidence was more limited than the CLI probes. #452
describes a future hidden readiness protocol, but frozen klin v0.4.2 does not
implement an `__agent` command. #459 study v4 therefore uses an explicit
research coordinator readiness boundary after the initial task attempt:
Shadow measures without surfacing findings, while Active resumes the same host
session with the exact frozen feedback. Independent oracle/residual work runs
afterward on verifier copies. That transport is research machinery, not an
implemented public or hidden v0.4.2 command contract.

The protected writable-state and host-proof obligation therefore remains.
The final product implementation must choose the state location/guard contract
and re-probe first-class hosts. Until then this is a DX/portability hole, not a
clean zero.

#425's optional structural finding identity is a separate shared mechanism. It
adds no parser, tree walk, persistent AST or external process. Its own measured
complexity producer cost was about 1 ms on 1M/20, while the whole hook stayed
roughly 8–13 ms slower in the prototype for an unproven reason. Because this is
shared matching infrastructure, the ledger does not charge the full hook gap
independently to every phenotype that can use identity.

#452's design keeps product cost out of the public command surface. Humans and
CI use intent-level commands and CI's authoritative klin command is plain
`klin check`. Its proposed hidden host-lifecycle/readiness protocol is not
implemented in frozen v0.4.2, so this ledger treats that protocol as future
implementation work rather than measured runtime behavior. #459 v4's
coordinator readiness boundary is the frozen research transport used for the
controlled experiment.

## Per-candidate inventory

| Candidate | Family | Placement | Ownership | Runtime evidence today | Setup | Concrete implementation mechanisms |
| --- | --- | --- | --- | --- | --- | --- |
| `shipped-complexity` | complexity | Stop | shipped-native | 27 ms current gate median on #425 1M/20 baseline; incremental cost not isolated | No tool install; no configuration beyond `{}` | existing tree-sitter function facts; complexity decision table; ratchet delta matching; … |
| `shipped-escapes` | escapes | Stop | shipped-native | per-capability controlled runtime not isolated yet | No tool install; no configuration beyond `{}` | existing changed-line/source scan; escape/suppression spelling table; ratchet delta matching |
| `shipped-stubs` | stubs | Stop | shipped-native | per-capability controlled runtime not isolated yet | No tool install; no configuration beyond `{}` | existing function/body walk; stub marker/body-shape table; ratchet delta matching |
| `shipped-test-deletion` | test-integrity | Stop | shipped-native | per-capability controlled runtime not isolated yet | No tool install; no configuration beyond `{}` | existing inventory before/after test extraction; test identity; delta/deletion judgement |
| `shipped-test-skip` | test-integrity | Stop | shipped-native | per-capability controlled runtime not isolated yet | No tool install; no configuration beyond `{}` | inventory test facts; escape/skip line rows; delta matching |
| `shipped-dead-symbols` | dead-addition | Stop | shipped-native | 559 ms current gate median on #425 1M/20 baseline; incremental cost not isolated | No tool install; no configuration beyond `{}` | existing structural declarations/references; dead-symbol reachability/matching; legacy matcher with optional #425 identity only where proven |
| `shipped-reachability` | reachability | Stop | shipped-native | 303 ms current gate median on #425 1M/20 baseline; incremental cost not isolated | No tool install; no configuration beyond `{}` | existing module/file discovery; family-specific reachability identity; local dependency evidence |
| `shipped-lockfile` | lock-consistency | Stop | shipped-native | per-capability controlled runtime not isolated yet | No tool install; no configuration beyond `{}` | existing Rust/npm manifest+lock readers; direct dependency/lock consistency comparison |
| `shipped-module-cycle` | architecture-cycle | Stop | shipped-native | per-capability controlled runtime not isolated yet | No tool install; **requires a person-written `layering` section**; `{}` does not activate it | existing ModuleGraph; local dependency resolution; cycle detection; … |
| `shipped-public-api` | public-contract | Stop | shipped-native | per-capability controlled runtime not isolated yet | No tool install; no configuration beyond `{}` | existing public-surface extraction; family-specific contract identity; before/after comparison |
| `test-all-checks-removed` | test-integrity | Finalize/REVIEW | candidate-native | prototype 20-file walk: 15–68 ms at mean/file; 80–550 ms at p99; integrated readiness cost missing | No tool install; no configuration beyond `{}` | shared test extraction with inventory; per-test check list; before/after test identity; … |
| `test-weakened-rust` | test-integrity | Finalize/REVIEW | candidate-native | prototype 20-file walk: 15–68 ms at mean/file; 80–550 ms at p99; integrated readiness cost missing | No tool install; no configuration beyond `{}` | shared Rust test extraction; assertion macro argument reader; check family/level/actual/expected facts; … |
| `test-disabled` | test-integrity | Finalize/REVIEW | candidate-native | prototype 20-file walk: 15–68 ms at mean/file; 80–550 ms at p99; integrated readiness cost missing | No tool install; no configuration beyond `{}` | shared test extraction; skip-if/run-if/cfg-never line rows; early-return body shape |
| `test-expected-mirrors-production` | test-integrity | Finalize/REVIEW | candidate-native | prototype 20-file walk: 15–68 ms at mean/file; 80–550 ms at p99; integrated readiness cost missing | No tool install; no configuration beyond `{}` | shared test check facts; production/test expected-value pairing; NOTE/REVIEW presentation |
| `test-new-unchecked` | test-integrity | Finalize/REVIEW | candidate-native | prototype 20-file walk: 15–68 ms at mean/file; 80–550 ms at p99; integrated readiness cost missing | No tool install; no configuration beyond `{}` | shared test extraction; new-test identity; check-list emptiness/observability shape |
| `design-family-bypass` | design-reuse | Finalize/REVIEW | candidate-native | prototype Finalize estimate: 10–290 ms / 20 changed files + 0.1–339 ms uncached base index; integrated warm cost missing | No tool install; no configuration beyond `{}` | resolved type+trait identity; implements relation; method/name facts; … |
| `design-registration-bypass` | design-reuse | Finalize/REVIEW | candidate-native | prototype Finalize estimate: 10–290 ms / 20 changed files + 0.1–339 ms uncached base index; integrated warm cost missing | No tool install; no configuration beyond `{}` | resolved type/family/member identity; literal registration collections; resolved references to registration bindings; … |
| `design-wrapper-bypass` | design-reuse | Finalize/REVIEW | candidate-native | prototype Finalize estimate: 10–290 ms / 20 changed files + 0.1–339 ms uncached base index; integrated warm cost missing | No tool install; no configuration beyond `{}` | resolved callee identity; enclosing declaration identity; base caller/wrapper relation index |
| `design-component-cycle` | architecture-cycle | Finalize/REVIEW | candidate-native | prototype Finalize estimate: 10–290 ms / 20 changed files + 0.1–339 ms uncached base index; integrated warm cost missing | No tool install; no configuration beyond `{}` | existing ModuleGraph; TypeScript alias/local-resolution completeness; #355 component projection; … |
| `unfinished-ellipsis-body` | unfinished-work | Finalize/REVIEW | candidate-native | prototype 20-file walk: 14–29 ms at mean/file; integrated readiness cost missing | No tool install; no configuration beyond `{}` | existing function/body tree-sitter walk; ellipsis-only body shape; delta/function identity |
| `unfinished-throw-body` | unfinished-work | Finalize/REVIEW | candidate-native | prototype 20-file walk: 14–29 ms at mean/file; integrated readiness cost missing | No tool install; no configuration beyond `{}` | existing function/body tree-sitter walk; raise-only body shape; delta/function identity |
| `error-empty-handler` | error-masking | Finalize/REVIEW | candidate-native | prototype 20-file walk: 14–29 ms at mean/file; integrated readiness cost missing | No tool install; no configuration beyond `{}` | existing exception-handler walk; empty/pass handler body shape; broad/specific exception classification |
| `error-default-handler` | error-masking | Finalize/REVIEW | candidate-native | prototype 20-file walk: 14–29 ms at mean/file; integrated readiness cost missing | No tool install; no configuration beyond `{}` | existing exception-handler walk; constant-default handler body shape; raise/propagation presence |
| `error-broad-handler` | error-masking | Finalize/REVIEW | candidate-native | prototype 20-file walk: 14–29 ms at mean/file; integrated readiness cost missing | No tool install; no configuration beyond `{}` | existing exception-handler walk; broad exception type classification; propagation/body-shape facts |
| `python-uv-lock` | lock-consistency | Stop | candidate-native | paired incremental gate-path: 4 ms median, 5.3 ms p95, 6 ms max / 15 samples; reads/parses not attributed | No tool install; no configuration beyond `{}` | pyproject.toml direct-dependency reader; uv.lock TOML reader; manifest/lock consistency comparison; … |
| `cargo-lock-entry-shape` | lock-provenance | Stop | candidate-native | paired incremental gate-path: 1 ms median, 2.3 ms p95, 3 ms max / 15 samples; reads/parses not attributed | No tool install; no configuration beyond `{}` | existing Cargo.lock reader; registry package source/checksum shape; changed dependency attribution |
| `new-direct-dependency` | dependency-review | Finalize/REVIEW | candidate-native | readiness runtime not isolated | No tool install; no configuration beyond `{}` | Rust/npm/Python manifest readers; normalized ecosystem dependency identity; before/after direct-dependency diff; … |
| `ruff-python-injection` | external-review | Finalize+CI | named-recipe | external Ruff 20-file: median 29 ms, p95 42 ms, max 44 ms (25 runs); intrinsic klin time not isolated | Pinned Ruff 0.16.10 (~21 MB binary); no project Ruff config | external tool resolution/version check; fixed Ruff rule set; controlled argv/env/timeout/output limits; … |
| `ruff-python-swallowed` | external-review | Finalize+CI | named-recipe | external Ruff 20-file: median 29 ms, p95 42 ms, max 44 ms (25 runs); intrinsic klin time not isolated | Pinned Ruff 0.16.10 (~21 MB binary); no project Ruff config | external tool resolution/version check; fixed Ruff rule set; controlled argv/env/timeout/output limits; … |

## Stop cost

The registry contains the shipped Stop rows plus two new native Stop
candidates from #364.

For shipped rows this ledger records existing evidence without pretending
that shipment proves admission. Where comparable current-gate timing exists,
it is recorded as total gate runtime and marked non-incremental.

For `python-uv-lock` and `cargo-lock-entry-shape`, #465 now supplies the
controlled 1M/20 incremental gate-path timing: 4 / 5.3 / 6 ms
median/p95/max for Python and 1 / 2.3 / 3 ms for Cargo across 15 paired
samples. The benchmark also reports the product-level Stop deltas separately.
Candidate-attributable parses/reads were not instrumented and remain explicit
measurement holes. These timings satisfy the cost-ledger measurement need;
they do **not** by themselves admit either candidate under #357.

No external analyzer is a Stop candidate. Ruff remains readiness/CI REVIEW.

## Readiness / Finalize cost

Native REVIEW candidates reuse the klin parser/fact engine and add no external
process. The completed research gives useful prototype upper bounds, but not
one integrated frozen readiness benchmark. The final ledger therefore keeps
intrinsic runtime numeric cells null until a basis-matched run exists.

Every readiness run also pays the **shared** #352 tree-binding substrate:
about 38 ms for one kept-index identity and about 80 ms for the before/after
pair around slow analysis on the measured 10,000-file tree. This is reported
once per readiness run, not once per phenotype. The eventual state record,
lock and invalidation path are likewise one shared substrate. The historical
Codex `.git` write limitation and limited Cursor-app proof remain explicit
implementation/portability holes after #452.

The Ruff recipe is different: external-tool time is separately measured and
recorded; project-command time stays separate at zero; intrinsic klin adapter
time remains a measurement hole. Its two phenotype rows share the same
`ruff-review` process and must be deduplicated in aggregate cost.

Timeout, missing-tool, unsupported-version, invalid-syntax and local-resolution
states must remain explicit. None can become a clean result merely to simplify
the cost accounting.

## UX, DX and AX

### UX

The TSV already reserves independent fields for human interruptions and
human-review-only cases. The final natural-finding values remain null because
#457 is intentionally deferred and #458 therefore cannot complete the frozen
denominator-backed analysis. #459 may provide controlled escalation/review
evidence, but it cannot substitute for the unavailable natural-study rate.
Prior research shows why the distinction matters: public-contract changes can
be intended, design/reuse evidence can be a justified exception, and
dependency-age/new-dependency evidence may have no deterministic repair.

No candidate should require a developer to know graph, turn-stamp or hook
internals. A REVIEW message must name the concrete measured relation/fact and
why klin is unsure.

### DX

Candidate-native rows add no mandatory repository configuration or tool
installation: `{}` remains sufficient for those proposed detectors.
Shipped-native rows keep their actual activation contract instead of being
flattened into that claim: notably `layering`/module-cycle is policy-activated
and requires a person-written `layering` section. Native mechanisms may
require internal facts or shared caches, but not a repository service.

The named Ruff recipe is the one deliberate setup exception: the person/CI
supplies the pinned executable. klin owns the rule list, invocation and
completeness contract; it does not become a package manager.

### AX

The frozen fields for repair turns and feedback bytes remain null unless #459
study v4 completes in time to provide compatible controlled outcomes.
Historical repair experiments are kept as evidence, not rates. In particular,
the Ruff swallowed-error appeasement route remains a known risk even if the
controlled outcome stays unavailable.

## Implementation complexity inventory

The ledger uses mechanisms, not scores:

- shipped native gates: current parser/fact paths, delta semantics and existing
  family identities;
- test integrity: shared test extraction, check facts, macro/matcher readers
  and before/after test identity;
- design/reuse: resolved type/trait/callee/binding relations and base relation
  evidence; component cycle reuses ModuleGraph;
- unfinished/error masking: extra body/handler match arms on an existing walk;
- dependency: ecosystem manifest/lock readers and normalized direct-dependency
  identity;
- Ruff: external executable resolution/version pin, controlled argv/env,
  timeout/output bounds, changed-file binding, completeness states and REVIEW
  adapter.

Persistent full ASTs, a second syntax parser pipeline, a daemon/watcher,
runtime network access and unbounded history are not part of any surviving
row.

## Measurement holes and evidence availability

The ledger is structurally complete and records experimental incompleteness
without making completion depend on research that will not run.

1. #457 is intentionally deferred. Its final natural prevalence/coverage,
   registered-denominator and per-change runtime evidence is unavailable.
2. #458 cannot fill the frozen natural-finding interruption or human-review
   fields without #457's denominator-complete corpus. Those fields remain
   unknown rather than being inferred from older or incompatible evidence.
3. #459 study v4 may still fill repair turns, feedback bytes and compatible
   closed-loop AX outcomes if completed before #460 closes. Otherwise those
   fields remain unknown with v4 identified as the missing source.
4. The two dependency Stop candidates have the required controlled 1M/20
   incremental timing from #465. Their candidate-attributable source-read and
   parse counts were not instrumented and remain unknown.
5. Native readiness candidates retain integrated-timing holes where only
   prototype bounds exist.
6. Memory/cache and source-read counts stay null where the research only
   measured prototype evidence sizes or did not instrument attribution.
   Existing 1M/warm20 read/parse/cache facts are imported where attribution is
   actually measured; unimplemented “reuse the parser” plans remain null.

A `null` cost is a measurement hole, not zero. Missing evidence can block a
candidate under #357's decision rules without blocking completion of this
ledger.

## Validation

The TSV contains every id from `phenotypes.tsv` exactly once and no extra
id. Every row records placement, ownership, setup, portability, reproduction,
implementation mechanisms and maintenance burden. Stop, external and project
runtime are separate fields. UX, DX and AX outputs have independent fields.

This ledger does not admit a capability. #357 applies the preregistered
decision rules to the evidence that exists and must treat missing required
evidence as an admission blocker rather than fabricating a favorable result.
