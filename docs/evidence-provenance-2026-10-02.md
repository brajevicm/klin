# Evidence provenance without an oracle score (#354)

Research dated 2026-10-02 for [#354](https://github.com/brajevicm/klin/issues/354),
including its 2026-09-28 UX/DX/AX amendment and
[behavioral cross-check comment](https://github.com/brajevicm/klin/issues/354#issuecomment-5877000069).
The fixed research baseline is `76097d415656e76d84fecea3d697892101c6d0b1`.
This note proposes semantics only: no production model, gate, configuration key,
command or SPEC change is authorized by its recommendation.

**Evidence labels:** baseline facts below come from `git show` of that commit;
**measured** means the frozen experiment in section 4, baseline CLI probe or
coding-agent trials in section 7 ran here;
**scenario** means an adversarial design exercise, not a real-tool/agent trial;
**reported** means another issue or note supplied the observation. The frozen
examples are counterexamples to overclaiming, not a prevalence estimate or a
cross-language detector corpus. No probabilities are assigned.

## 1. Evidence-source taxonomy

These are capabilities, not ranks. Each item supports only the claim it actually
checks. A source may have several capabilities and several provenance properties.

| Source | Narrow supported claim | What remains outside it |
| --- | --- | --- |
| Pre-existing regression tests | Recorded cases still satisfy their assertions | Incorrect old expectations; untested inputs; changed harness |
| Hidden/acceptance tests | Independently supplied cases satisfy expectations | Hidden does not imply authoritative or comprehensive |
| Declared contract/schema | Selected instances satisfy the declared constraints | Undeclared behavior; contract correctness; unvalidated instances |
| Candidate-authored tests | Candidate assertions held on the recorded execution | Independence; whether assertions express intended behavior |
| Build/typecheck | Selected build/type obligations succeeded | Runtime behavior, accepted casts, unbuilt targets |
| Changed-line coverage | Reported executable changed lines were hit | Assertion effectiveness; unreported source/lines |
| Mutation | Specified generated mutants had specified outcomes | Unselected operators, equivalent mutants, omitted behavior |
| SARIF/analyzer | Selected detector reported stated findings | Unobserved scope; analyzer false positives/negatives |
| Runtime/property/benchmark | Stated observations hold under recorded inputs/environment | Other inputs/platforms; benchmark portability |
| Counterfactual | Specified intervention preserves stated observations | Removal safety outside those observations; necessity |

Coverage.py's [FAQ](https://coverage.readthedocs.io/en/latest/faq.html) explains
that definition lines can execute on import even when no tests have run. Stryker's
[mutant states and metrics](https://stryker-mutator.io/docs/mutation-testing-elements/mutant-states-and-metrics/)
defines several mutant outcomes and denominators: retain those outcomes, not just
a percentage. These sources motivate narrow interpretation; they do not prescribe
klin policy.

## 2. Provenance model

Reject the proposed six-value enum. PREEXISTING and CANDIDATE_AUTHORED concern
origin/time; DECLARED_CONTRACT concerns authority; DERIVED_MEASUREMENT concerns
method; EXTERNAL_ANALYZER concerns ownership. They overlap. A candidate-authored
schema can be both a contract and a derived analyzer input.

Minimum per evidence item: producer ownership, capability, origin reference,
relation to the Window (`unchanged-before`, `added`, `modified`, or `unknown`),
and authority/dependence established by the execution boundary. Origin references
are repository path + revision for a test/contract, or external artifact digest +
trusted supplying party when available. `unknown` is an honest value, never
PREEXISTING by default. A hidden label alone proves no independence.

Independence means the candidate could not author/change the decisive assertion,
fixture and execution configuration in the audited Window. Unchanged before-tests
are independent of this change's authorship but may share assumptions or execute
candidate-controlled helpers. Record those dependencies; do not certify an entire
suite independent solely because its top-level file is unchanged. Changing
assertions, fixture, mock, helper, selection or runner makes affected evidence
modified/dependent. An origin from a protected CI checkout is stronger provenance
than an unverified local claim, within the existing Feedback/Enforced distinction
([baseline SPEC 15](https://github.com/brajevicm/klin/blob/76097d415656e76d84fecea3d697892101c6d0b1/docs/SPEC.md#15-trust-model-and-conformance-levels)).

Candidate authorship does not invalidate evidence. It limits the conclusion:
“these assertions passed,” rather than “independent acceptance confirmed the
intended behavior.” Provenance changes interpretation, never adds score points.

## 3. Claim/result model

Adopt claim-local evidence sets. The two concrete consumers are native measurement
holes/findings already reported by `Records`, and external coverage/SARIF evidence
with phase/completeness needs in #70/#54/#363. Reuse that accumulator; no provider
registry or second evidence engine.

A result contains a producer-owned id, claim, site/scope, phase, evidence references,
judgment, measurement state and execution state. Its evidence holds origin,
measured tree reference, basis, intended/observed scope, observations, holes and
limitations. Before/after and comparison compatibility exist only for comparisons.
Repair guidance and verification step are required when actionable, rather than
empty mandatory fields on every note. Anti-appeasement copy belongs to the remedy:
“exercise the missing behavior; do not exclude its source.”

Keep three dimensions:

| Dimension | Values | Meaning |
| --- | --- | --- |
| Judgment | `pass`, `fail`, `review`, `null` | Policy satisfied, violated, contextual judgment, or no judgment justified |
| Measurement | `complete`, `partial`, `unavailable`, `unsupported` | Coverage of the explicitly named measurement obligation |
| Execution | `ok`, `tool-error`, `not-run` | Producer completed normally, failed, or was not invoked |

Execution follows the producer's declared protocol: an expected assertion-failure
or findings-present nonzero exit can still be execution `ok`, with judgment
`fail`. Abnormal termination, failure to start, or invalid required output is
`tool-error`; a raw exit-code convention alone must not relabel code failures.

`null` avoids inventing a REVIEW-worthy code observation for a missing executable.
“UNKNOWN” is the short rendering of measurement/execution holes, not another
code-quality judgment. A partial measurement may contain a valid failing finding;
its positive evidence survives, but it cannot establish absence elsewhere.

For each narrow claim, evidence can support, contradict, or leave it inconclusive.
The claim's declared authority resolves disagreement: a recognized contract failure
contradicts the contract claim even when candidate tests pass. Equally authoritative
conflicting/unstable observations leave the broad behavioral claim inconclusive
and preserve both records. Majority voting is rejected. A contradictory,
reproducible authoritative witness is not canceled by unrelated passing evidence.
A local detector's deterministic result is not a universal correctness verdict.

## 4. Scenarios and adversarial cases

The following frozen experiment was run with Python 3 on 2026-10-02. It uses no
production klin code and no real coverage/mutation tool. Copy the fenced program
to `/tmp/evidence354-probe.py` and run `python3 /tmp/evidence354-probe.py`.
It defines the behavior, cases and assertions completely; no external fixture is
required. The coverage/mutation values below are deliberately synthetic.

```python
from itertools import product

def intended(order): return order * 2

def special(order): return order * 2 if order in (1, 2) else 0

def wrong(order): return order

def observed(fn):
    return {'candidate': all(fn(n) >= 0 for n in (1, 2)),
            'regression': fn(3) == 6,
            'contract': all(fn(n) == n * 2 for n in range(1, 5))}
for name, fn in [('intended', intended), ('wrong', wrong), ('special', special)]:
    print(name, observed(fn))
assert observed(intended) == {'candidate': True, 'regression': True, 'contract': True}
assert observed(wrong) == {'candidate': True, 'regression': False, 'contract': False}
assert observed(special) == {'candidate': True, 'regression': False, 'contract': False}
for runs in product((True, False), repeat=3):
    state = 'supported' if all(runs) else ('contradicted' if not any(runs) else 'inconclusive')
    assert (state == 'inconclusive') == (len(set(runs)) == 2)
print('repeat sequences: 8, unstable: 6')
assert (100 / 100) == 1  # all generated mutants killed, synthetic
assert special(3) != intended(3)  # an omitted dimension is still wrong
prefix = ['window', 'derived scope', 'derived cc', 'derived lines', 'resolution', 'coverage']
repairs = ['FAIL contract: preserve doubling for unseen ids', 'FAIL regression: restore order 3']
reviews = [f'REVIEW observation {i}' for i in range(8)]
before = prefix + repairs + reviews
after = repairs + ['REVIEW 8 observations; inspect, do not edit to clear']
assert before.index(repairs[0]) == 6 and after.index(repairs[0]) == 0
print('render lines:', len(before), '->', len(after), '; lines before first repair:', 6, '->', 0)
```

Measured output:

```text
intended {'candidate': True, 'regression': True, 'contract': True}
wrong {'candidate': True, 'regression': False, 'contract': False}
special {'candidate': True, 'regression': False, 'contract': False}
repeat sequences: 8, unstable: 6
render lines: 16 -> 3 ; lines before first repair: 6 -> 0
```

The regression assertion stands for a frozen before-test: bad change fails,
repair to `intended` passes. The candidate assertion passes both behaviors.
The special-casing example reproduces the *logical failure mode* in the owner's
comment; it does not rerun that TypeScript/Python probe. Assertions added by this
researcher demonstrate separability, not independent acceptance of this proposal.

| Case | Evidence/result | Safe action/conclusion | Basis |
| --- | --- | --- | --- |
| Bad change, unchanged regression fails, repair passes | Independent case contradicts then supports the narrow claim | Repair behavior, retain expectation | Measured frozen function |
| New weak candidate test passes correct and wrong code | Dependent support for nonnegative result only | Do not infer doubling correctness | Measured |
| Contract disagrees with candidate test | Preserve both; contract claim fails | Repair to declared contract, inspect contract if disputed | Measured |
| Hard-code test order ids | Candidate cases pass; independent order 3 fails | Add/retain non-candidate examples; no assertion-count remedy | Measured |
| 100% changed coverage, weak assertions | Execution claim supported, correctness inconclusive | Improve behavioral assertions, not coverage number | Scenario |
| Low coverage, one hidden acceptance witness | Witness supports its specific required behavior | Neither universal pass nor automatic behavioral failure | Scenario |
| 100/100 selected mutants killed, order 3 wrong | Operator-local mutation claim passes | Keep operator/scope limitations; behavior remains wrong | Synthetic measured counterexample |
| Repeated outcomes disagree | 6 of 8 three-run Boolean sequences unstable | Preserve attempts, inconclusive broad conclusion | Measured finite enumeration |
| Coverage omits changed `src/new.rs` | Intended source not observed; partial | Regenerate report/restore inclusion; never zero uncovered lines | Scenario |
| Runtime check unavailable on Windows | Unavailable, not-run; judgment null | Use supported execution environment or disclose missing obligation | Scenario |
| Removal passes tests but violates explicit doubling contract | Tests preserve their narrow outcomes, contract fails | Restore contracted behavior; removability is not slop | Scenario using same contract |
| Suite changed inside Window | Modified assertions/harness provenance | Do not pool as unchanged independent evidence | Scenario |
| Empty analyzer result, exit 0, no scope proof | Execution ok, measurement at most partial | Request coverage evidence; do not report complete clean scan | Scenario |
| Parser rejects one file; native bound exhausts | Partial with reason/path or remaining obligation | Preserve observed findings; repair measurement gap | Scenario |
| Analyzer versions/options differ | Non-comparable | Remeasure both under one basis; no finding churn | Scenario |
| Two analyzers read different trees | Invalid for single-tree Finalize | Discard/re-run affected evidence under #352 | Scenario |
| Same observation emitted eight times | One correlated observation, not eight independent concerns | Deduplicate for presentation, preserve multiplicity where meaningful | Scenario |

[#353](https://github.com/brajevicm/klin/issues/353) remains open at inspection;
no completed #353 research result is assumed. Its
[2026-10-02 handover](https://github.com/brajevicm/klin/issues/353#issuecomment-5948141865)
reports test-deletion reply clearance, renamed/commented tests, missed assertion
weakening and three agents restoring tests. Those are **reported**, not trials
performed here. They reinforce that a green gate cannot attest test integrity.
No proposed blocker in this note is admitted from those anecdotes alone.

## 5. Skipped, unavailable and flaky evidence

`skipped` is a run decision with reason (`phase`, `not-selected`, `dependency`, or
`policy`), represented as execution `not-run`; it is never a passing measurement.
Unavailable evidence names the absent tool/artifact/platform. Unsupported evidence
names the producer's semantic limitation. A crash is `tool-error`, potentially
partial if validated observations were retained. A stale or unbound report is
unavailable for this tree even if its bytes parse.

Flaky evidence retains each attempted input, outcome, environment and repeat
policy. Disagreement marks the affected claim inconclusive; never select the last
green run or silently omit failing attempts. A stable failure may still justify
a narrow failure; a previously flaky suite's later single success cannot prove
stability. An unrun test is unknown, not known flaky. Timeouts do not automatically
prove code slowness: only a separately declared benchmark obligation with a valid
measurement environment can judge that behavior. No generic retries on Stop.

## 6. Aggregation alternatives

| Alternative | Adversarial outcome | Disposition |
| --- | --- | --- |
| Weighted scalar | Coverage/mutation/build points can offset broken contract | Reject universal score |
| Fixed coverage + mutation minima | Weak assertions/special cases pass; strong focused witness can fail | Reject universal oracle; retain explicitly chosen capability policy |
| Weak/medium/strong oracle tier | Conflates dependent test volume with independent case | Reject global tier |
| Claim-local sets | Preserves contract disagreement and missing scope | Adopt |
| Capability-specific policy | “reported changed executable lines meet chosen minimum” is truthful | Adopt only for its bounded claim |
| Co-occurring minor observations produce one REVIEW | Can direct attention; correlated detectors and harmless verbose code can inflate it | Conditional contextual review, never block |

Challenge co-occurrence with the owner's eight-signal example: restating comments,
boilerplate docs, console noise, defensive checks, log/default handling, weak tests,
complexity and scope are not eight independent behavioral witnesses. Several can
share one generated wrapper as their cause; two detectors can report the same
console statement. Conversely a diagnostic-heavy integration legitimately holds
many of them. Therefore no “N signals = slop” or numerical independence claim.

A future explicitly admitted, named review predicate may group distinct supported
observations on the same behavioral scope and ask one concrete question, such as
“does this fallback preserve the declared error contract?” It must list contributing
claims, deduplicate overlapping observations, state why their combination is
relevant, and survive harmless/co-correlated negatives. Until calibrated against
real changes and agent repairs, co-occurrence stays a Finalize inspection summary,
not an automatic judgment upgrade. REVIEW allows handing work back with the
observation recorded; the agent must not keep editing to erase it.

## 7. Human/JSON output and attention experiment

One semantic result is rendered differently; surfaces cannot change its claim or
turn unavailable into pass. Stop shows repairable blockers first and minimal next
steps; configured expensive evidence appears only as a concise “not run here”
summary when relevant. Finalize/CI show execution obstacles to useful measurement,
then deterministic blockers, then other actionable findings, reviews and useful
notes. A build failure belongs first because it prevents downstream measurement;
a missing optional advisory tool need not outrank a proven behavioral regression.
Tie-break deterministically by existing catalogue order, producer-owned id and
path/line. Priority is presentation policy, not evidence strength.

Bound the agent queue to three actionable results per response, with exact omitted
counts and an inspection pointer; do not truncate stored results or CI enforcement.
Group duplicate remedies/causes only when doing so preserves all sites. A later
repair exposes the next queue; no finding is forgiven because it fell off screen.
Developer text retains before/after, basis differences, coverage/holes; CI annotations
retain claim/site/remedy/id and incompleteness; JSON retains the full evidence.
Normal success states exactly what ran and how many configured obligations did not.

Illustrative future copy, not current CLI:

```text
FAIL src/orders.rs:12 — unseen order ids return the wrong total.
Preserve the doubling contract for every supported id; verify the frozen order-3 case.
Verification incomplete: runtime evidence unavailable on this platform; run it on Linux.
REVIEW 1 fallback concern — inspect the error contract; do not edit merely to clear this note.
```

The owner's comment reports about six diagnostic lines before the first current
Stop failure. [Baseline SPEC 11.1](https://github.com/brajevicm/klin/blob/76097d415656e76d84fecea3d697892101c6d0b1/docs/SPEC.md#111-text)
requires window/derived lines ahead of rows. The section-4 rendering experiment
freezes six diagnostics, two blockers and eight reviews: 16 lines become 3,
first repair moves from index 6 to 0, eight reviews remain accounted for. This
is a **measured rendering prototype**, not measured baseline terminal output,
agent repair success, or token/latency benchmark.

An additional **measured baseline CLI probe** built the exact baseline (binary
version 0.3.0) in a throwaway checkout. With `{}` and a committed three-line
`src/lib.rs` containing `pub fn value() -> u32 { 1 }` (formatted over three lines),
a turn changed its body to `Some(1).unwrap()`. Reproduction: initialize Git on
`main`, commit the clean source and `{}` marker, switch to `work` and make an
empty commit; run the baseline binary's `radius` with stdin
`{"hook_event_name":"UserPromptSubmit"}`, then write the changed source and run
`klin gate --hook --changed` with stdin
`{"hook_event_name":"Stop","stop_hook_active":false}`. It returned exit 2,
with this capture:

```text
klin: a quality gate failed — fix what each names, then stop again (gate block 1 of 2 in this turn):
  window: turn — base 233c349, the turn stamp, taken just now
  changed: 1 file(s) against the base — the scoped gates judge those; CI judges everything
  FAIL  escapes
        FAIL: 1 new escape site(s) where the code opts out of a check, beyond the 0 the base holds:
          src/lib.rs:2  unwrap  Some(1).unwrap()  — nothing matched
        Fix what the escape hides: handle the error instead of unwrapping it, address the lint instead of allowing it. Accepting a new escape is a policy decision for a person, in the config, in a reviewed commit.
klin: 6 gate(s), 1 failed.
```

This fixture has two context lines before the first FAIL row, plus one hook
preamble, rather than the owner's roughly six diagnostics. Progressive disclosure
would place claim/site/remedy first (zero leading diagnostic lines), retain the
same failed escape and six-gate accounting, and move Window detail to inspection.
The measured count is baseline rendering only; the proposed ordering is a design
comparison. Diagnostic counts depend on configured/derived gates, not a universal
six-line baseline.

The frozen example retains two repairs
for measurement; in production, identical contract/regression remedies would group.
It directs repair to unseen-input behavior and explicitly discourages REVIEW
appeasement. A small actual coding-agent trial below checks the immediate repair
response; broader repair-rate claims still require realistic paired trials.

**Measured coding-agent repair trial.** One delegated agent received the frozen
special-cased `total` implementation, original weak candidate assertion, frozen
contract and this exact feedback: “FAIL total — unseen order ids return the wrong
total. Preserve doubling for every supported id; verify frozen order-3 case.
UNKNOWN runtime-linux — evidence unavailable on this platform; use supported
environment, do not invent code repair. REVIEW fallback concern — inspect the
error contract; do not edit merely to clear contextual observation.” The trial
instruction required retaining both assertions, a minimal repair and a runnable
check. These inputs plus the before/after diff reproduce its decision context;
agent sampling itself is not deterministic. It ran the original program
(exit 1: candidate assertion passed, contract failed), then made one repair in
one agent turn:

```diff
-def total(order): return order * 2 if order in (1,2) else 0
+def total(order): return order * 2
```

It retained `assert all(total(n) >= 0 for n in (1,2))` and
`assert all(total(n) == n * 2 for n in range(1,5))`. The same program then exited
0; a separate check confirmed `total(3) == 6`. It explicitly left Linux runtime
evidence unresolved and made no code edit for UNKNOWN. It inspected the supplied
contract for REVIEW and made no additional edit to clear the observation.
Observed outcome: one correct behavioral repair, zero metric-only edits, zero
known regressions in the four frozen cases, zero invented UNKNOWN repairs, and
one contextual observation retained. This is an instructed toy trial, with the
contract supplied; it does not measure spontaneous readiness, human intervention
rates, unseen regressions or the quality of a future detector.

**Measured comparator trial.** A second isolated coding agent received the same
special-case implementation and weak candidate assertion, the explicit product
requirement “every supported positive integer id receives doubled total,” and
synthetic aggregate feedback: “PASS — candidate tests pass; changed coverage
100%; mutation score 100%; oracle strong. REVIEW fallback concern. Runtime
evidence skipped.” It inspected the requirement despite that PASS, observed
`total(3) == 0`, and made this one-turn repair:

```diff
-def total(order): return order * 2 if order in (1,2) else 0
+def total(order): return order * 2 if order > 0 else 0
-assert all(total(n)>=0 for n in (1,2))
+assert all(total(n) == n * 2 for n in (1, 2, 3, 100))
```

The original weak test passed; the repaired check exited 0. The test edit
strengthens the behavioral assertion rather than optimizing a numeric metric;
no actual coverage/mutation tool ran. Both agents repaired the positive-id bug
in one turn, with zero observed metric-only repairs. Therefore this paired toy
probe **does not show the proposed representation improves repair rates over
aggregate feedback**. It shows claim-local guidance can yield the intended repair
without UNKNOWN/REVIEW churn, and that an agent with an explicit requirement can
resist misleading aggregate PASS. Different feedback details/visible assertions,
one sample per arm and the artificial task prevent causal or population inference.
Keep the no-score recommendation on the semantic counterexamples, not on an
unsupported claim of agent-performance improvement.

Illustrative JSON fragment (field spelling is proposed, no shipped schema):

```json
{
  "phase": "finalize",
  "verification": "incomplete",
  "results": [{
    "id": ["native", "klin", "contract", "order-total-v1"],
    "claim": "supported order ids receive doubled totals",
    "scope": "src/orders.rs",
    "judgment": "fail",
    "measurement": "complete",
    "execution": "ok",
    "evidence": ["contract-case-3"]
  }],
  "evidence": [{
    "id": "contract-case-3",
    "capability": "contract-test",
    "tree": "T",
    "origin": {"relation": "unchanged-before", "reference": "base:tests/order3"},
    "observation": {"expected": 6, "actual": 0},
    "basis": {"producer": "frozen-contract", "version": "1", "scope": "case-3"},
    "intended": "case-3",
    "observed": "case-3",
    "limitations": ["does not cover all ids"]
  }],
  "requirements": [{"capability": "runtime-linux", "satisfied": false,
                    "reason": "unavailable"}]
}
```

For future explanation/reproduction retain Window/base and exact measured tree,
producer/basis, original observations or stable artifact references/digests,
before/after when applicable, completeness diagnostics, provenance/dependencies,
policy judgment rule/version, and verification command/inputs. Duration/timestamp
may aid debugging but cannot establish semantic compatibility. Hidden artifacts
need trusted references and redacted disclosure, not leakage into agent output.
No `klin explain` command is implemented or required by this note.

## 8. Migration from Records

At the baseline, [src/check.rs](https://github.com/brajevicm/klin/blob/76097d415656e76d84fecea3d697892101c6d0b1/src/check.rs)
owns `Records` (findings, notes, gates, coverage, derived values, held/accepted and
work diagnostics) and the optional explicit `Sink`. There is no baseline
`src/record.rs` or `src/records.rs`.
[ADR 0036](adr/0036-one-check-catalogue-and-an-explicit-execution-sink.md)
keeps one accumulator and no generic plugin interface. Current source splits the
contract into `src/check/contract.rs`; that later layout is not baseline evidence.

A later authorized slice should add concrete measurement/execution fields to gate
rows and evidence/provenance references only for the two needing consumers, preserve
current finding/note meanings, and version any public structured contract before
renderers consume it. Legacy records missing metadata become `unknown`/unbound;
never retrofit complete or independent provenance. Native holes already recorded
as unparsed/not-measured/lost/unresolved/unbuilt map to explicit measurement
reasons; derived ceiling provenance stays distinct from behavioral provenance.

[Baseline coverage.rs](https://github.com/brajevicm/klin/blob/76097d415656e76d84fecea3d697892101c6d0b1/src/coverage.rs)
counts discovered/measured/not-measured/excluded/unreadable units and tracks lost
measurement. Extend this evidence, not replace it with a percentage. Ratcheted
finding metrics remain gate-owned. Existing JSON/text and journal readers need
version/unknown handling tests at the CLI seam; no second report accumulator,
generic registration system, mandatory evidence DB or whole-tree parse.

## 9. Measurement basis and compatibility

Minimum semantic basis, only where relevant: producer/detector identity and
semantic version, rule-set identity/version, klin adapter version, parser/extractor
semantic identity, normalized measurement-relevant options, and selection/scope
semantics plus the selected obligation set. Unknown relevant components prevent
comparison. Pin exact producer versions by default; permit cross-version equality
only via an explicitly documented semantic compatibility rule, never guess from
matching output. Canonicalize maps/sets deterministically and preserve ordered
options when order affects semantics. Runtime/benchmark platform/environment and
test-harness/assertion revision belong here when they alter the claim; contextual
execution metadata is not always irrelevant.

Exclude machine path, timestamp, pid, scratch location and duration from method
identity. Root-relative placement must preserve meaningful path/scope options;
normalization cannot erase a rule config, target, feature flag or environment input.
Source contents normally define the compared subject, not a different method.
An unchanged selector with an expanded selected obligation set requires either
remeasurement over one shared scope or comparison restricted to explicitly named
common obligations, with additions/holes disclosed. Do not label unseen scope clean
or resolved. No per-source content hashes on ordinary Stop to construct the basis;
tree binding remains #352's lifecycle responsibility.

Compatibility has `comparable`, `not-comparable`, `unknown` with reasons. Only
comparable evidence enters ordinary new/resolved/worsened comparison. A changed
adapter/rule/parser/scope either remeasures both trees under one basis (preferred,
matching ADR 0001), or produces “measurement basis changed; not compared.” It
cannot resolve debt, claim regressions or silently carry old accepted evidence.
Partial common-scope comparison must disclose which obligations it compares and
exclude unobserved portions from deltas. #425 owns optional site matching identity;
this note does not invent a structural key or duplication fingerprint.

Ownership is collision-proof in a structured tuple, minted by klin: native
`["native", "klin", family, family-owned-id]`; external
`["external", configured-entry-id, producer-id, producer-owned-id]`.
No raw concatenation without escaping. External reports cannot choose the owner
field; even a ruleId named “klin/complexity” stays external. Duplicate configured
entry identities are rejected by the integration contract rather than merged.
This is a namespace rule, not a provider registry. Site identity remains optional
and family-owned, with multiplicity preserved according to #425.

## 10. Intended versus observed completeness

Each measurement names the intended obligation (paths, targets, test cases, rule
set or experiment inputs), records what the producer proves observed, and records
diagnostics/unsupported context separately. `complete` means all of *that*
obligation was measured under its declared semantics; never behavior complete.
Empty intended scope can be complete only when its emptiness is established by the
selector, not inferred from zero results. `partial` includes unknown observed
coverage, not just known missing files. All scope unsupported is `unsupported`;
no usable evidence is `unavailable`; mixed usable/unsupported is `partial` with
unsupported holes.

[OASIS SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/sarif-v2.1.0-os.html)
separates results from invocations, including execution success and tool execution
notifications (3.20). Those can inform execution/diagnostics; they do not alone
prove selected per-file analysis. An adapter may use a documented producer
manifest/coverage guarantee instead of a per-file list, recording the guarantee's
origin. An arbitrary user-owned report without such evidence is advisory/partial
for completeness, including exit 0 with no findings.

[Baseline SARIF read/wrote/judge](https://github.com/brajevicm/klin/blob/76097d415656e76d84fecea3d697892101c6d0b1/src/sarif.rs)
requires a report, accepts its command's nonzero exit if a report exists, checks
freshness without `run`, places findings and judges changed lines. That is not a
producer-completeness proof. Retain current changed-line judgments; stronger
Finalize completeness requires separately justified producer evidence.

## 11. Resource exhaustion

A work/time/output/memory bound that truncates analysis records the bound type,
configured value, observed work where known and unmeasured remainder. Validated
findings before exhaustion remain usable positive witnesses; absence claims cannot
cover the remainder. With any validated observations: measurement partial. Without
usable observations: unavailable. Native deliberate budget stop has execution ok
with `exhausted` reason; external timeout/kill has tool-error unless its protocol
explicitly treats a bounded partial result as normal. A malformed truncated report
provides no guessed partial findings. Output truncation of diagnostics alone does
not imply truncated analysis; discarded result data does.

Never substitute `0 findings`, `0 uncovered`, or `100%` for exhausted work. Re-run
under a permitted larger bound or narrower declared scope; narrowing cannot satisfy
the original required obligation without an explicit policy change. A deterministic
work cap makes repeatable exhaustion; a wall-time cap can vary with scheduling and
must expose that limitation. Bounded native and external work share these result
semantics, not necessarily an execution mechanism. Stop keeps its bound and cannot
silently escalate into an unbounded scan.

## 12. Phase-aware required evidence

A phase can declare a capability/scope obligation authoritative for that phase.
A requirement is satisfied only by matching complete, usable evidence bound to the
same tree and compatible required basis. A complete failing test satisfies *the
measurement requirement* while producing a code failure. Missing/partial/unsupported/
unavailable/not-run required evidence leaves verification incomplete, with no
fabricated code violation. Requirements themselves and reasons for satisfaction
belong in structured output so aggregate success is auditable.

Phase outcome is: `findings` when valid deterministic policy failures exist;
`incomplete` when required verification is missing, with failures still preserved
if both occur; `finalized` only when obligations are satisfied and no blockers
remain, with reviews disclosed. Store incomplete independently of judgment so
precedence in a short summary loses no facts. Optional unavailable evidence is
disclosed and cannot support its claim; it does not automatically veto every
finalization. REVIEW alone does not require repair or prevent handback.

The landed [#352 research note](finalization-lifecycle-2026-10-02.md), sections
3–7, proposes `finalized/findings/incomplete`, one-tree binding and no deep Stop
work. This note supplies evidence semantics beneath those outcomes, not a second
lifecycle. Its section 5 phase spelling and #70's Normal/Postflight selection
must be reconciled in an implementation decision; do not ship competing phase
systems. Finalize/CI-only requirements cannot trigger commands, report stats,
source reads or hashes during ordinary Stop. A skipped requirement in that phase
is “not applicable here / required at Finalize,” not a false Stop failure.
No new `klin.json` key or exit-code contract is designed here.

## 13. Implications for related work

- [#70](https://github.com/brajevicm/klin/issues/70): phase-filter before requirements
  and any external work; runner synthesizes skipped rows. Shared concrete report
  preparation owns run/freshness/external timing, not evidence judgment or Git.
- [#53](https://github.com/brajevicm/klin/issues/53): streaming selected executable-line
  facts remain separate from native measurement coverage. Unreported lines are
  unknown, not uncovered; parser owns no command/Window/freshness.
- [#54](https://github.com/brajevicm/klin/issues/54): project-chosen coverage policy
  supports only reported changed executable-line coverage, with noise-floor
  non-judgment and source-absent holes explicit. No base coverage percentage
  ratchet. Strict holes are verification errors, not behavioral defects.
- [#363](https://github.com/brajevicm/klin/issues/363): user-owned arbitrary SARIF
  and any named klin-owned recipe retain distinct reproducibility claims. A recipe
  must establish version/config/adapter scope proof, bounded execution and tree
  binding; this note admits no tool/rule family. No report completeness from exit.
- [#425](https://github.com/brajevicm/klin/issues/425): keeps matching identity
  optional, versioned and family-specific. Basis compatibility is checked before
  any site matching; an id does not establish compatible measurement.
- [#353](https://github.com/brajevicm/klin/issues/353): test-integrity facts become
  provenance/limitations when justified. No assertion count graduates to strength
  or a blocker without its own hard negatives and repair experiments.
- Mutation: record generated/selected operators, scope, killed/survived/uncovered/
  error/timeout outcomes and denominator policy. Aggregate percentage is a local
  observation, never correctness credit.
- Counterfactual: record intervention, original/counterfactual tree references,
  invariant/contract and observation delta. “Removal did not affect these tests”
  supports only those tests; do not call it unnecessary code or slop. Contract
  contradiction dominates a removability story for that contract.

## 14. Proposed SPEC contract and decision

For a later reviewed implementation, extend SPEC 8.3/8.6, 11.1/11.2/11.4 and 14;
add phase semantics with #70/#352 rather than changing today's SPEC in this ticket.
The proposed contract is:

1. Every result MUST retain distinct policy judgment, measurement completeness and
   producer execution. UNKNOWN MUST NOT prescribe a code repair for tool absence.
2. Claims MUST name their scope and supporting/contradicting observations. Passing
   tests MUST NOT imply complete behavior. Candidate-authored evidence MUST NOT be
   represented as independent; unknown provenance MUST remain unknown.
3. Complete measurement MUST identify intended obligations and proof of observed
   coverage. Successful empty output alone MUST NOT establish completeness.
4. Skipped, missing, unsupported, unstable and exhausted evidence MUST be explicit.
   Unmeasured MUST NOT be clean; unknown MUST NOT be unused; unobserved MUST NOT
   be unnecessary; deterministic MUST NOT be universally correct.
5. Comparisons MUST check semantic measurement basis before matching findings.
   Different/unknown bases MUST NOT produce ordinary regression/resolution churn.
   Contents/tree identity MUST remain distinct from measurement method identity.
6. External identifiers MUST remain in a klin-assigned external namespace and MUST
   NOT impersonate native findings. Site multiplicity and family-owned matching
   MUST remain intact.
7. Required evidence MUST affect phase verification authority separately from code
   quality. Missing required measurement MUST prevent authoritative success without
   inventing a code finding. Expensive requirements MUST NOT run during Stop merely
   because another phase requires them.
8. All evidence satisfying one Finalize verdict MUST refer to that verdict's one
   tree under #352. Unbound/drifting evidence MUST NOT satisfy that obligation.
9. One semantic result MUST feed human, agent, CI and JSON renderers. Agent output
   MUST present a bounded repair queue, disclose omissions, and retain full results
   for inspection. REVIEW MUST NOT instruct repeated edits to erase context.
10. Reproduction MUST retain Window/tree, basis, exact observations/references,
    provenance, completeness, judgment rule and verification step. Legacy records
    without these facts MUST NOT be upgraded by inference.

Open evidence gaps: real-tool scope guarantees and recipe conformance belong to
#363; changed-coverage admission to #53/#54/#70; test-integrity calibration to #353;
broader agent repair and co-occurrence calibration require realistic paired trials. The measured
frozen counterexamples justify refusing substitution between evidence kinds and
support the minimum representation. They do not authorize a gate or demonstrate
improved repair rates beyond these paired toy trials.

**Adopt claim-local evidence sets** — the exact minimum is one `Records` result
with claim/scope/phase, producer-owned id, judgment + measurement + execution,
references to observations carrying origin/dependence, tree binding, relevant
measurement basis, intended/observed obligations and holes; before/after and remedy
only where used. No universal score/tier, no provider registry, and no automatic
co-occurrence blocker. Implementation requires a separate human-reviewed decision.
