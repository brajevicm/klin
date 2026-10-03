# Counterfactual simplification feasibility (#356)

Research dated 2026-10-03 for [#356](https://github.com/brajevicm/klin/issues/356),
including its UX/DX/AX amendment. Research baseline:
`76097d415656e76d84fecea3d697892101c6d0b1`. The implementation checkout began at
`af429485efdb22ade7e231ccebef90a2a3d8c8da`.

**Result:** compact trajectory grouping found one coordinated reversal that this
final-patch search missed, but required more oracle executions overall. Seven
constructed tasks produced harmful oracle-green reductions. This supports an
optional research tool, not automatic simplification or a production gate.

Measured below means the checked-in disposable experiment ran here. Literature
results are reported, not replicated. These are constructed calibration tasks,
not a sample of completed agent work, a prevalence estimate, or evidence that
real-task cost is acceptable. No shipped check, configuration, hook or SPEC
behavior changes.

## Sources and prerequisite contracts

[TRIM, arXiv v1](https://arxiv.org/html/2607.18161v1) reduces feedback-request
sequences, files and edits. It reports approximately 1.9× lower validation cost
than DD-Hunk on CrashFixer kernel repairs. That baseline is hunk reduction, not
our file/hunk/function hierarchy. Its source-bearing edit replay is richer than
turn ids and fingerprints; its repair results do not establish usefulness for
features, external APIs or requested architecture.

[Delta Debugging, Zeller and Hildebrandt](https://www.cs.purdue.edu/homes/xyzhang/spring07/Papers/delta-debugging.pdf)
distinguishes unresolved outcomes and one-minimality from a global minimum.
[C-Reduce, author preprint](https://users.cs.utah.edu/~regehr/papers/pldi12-preprint.pdf)
shows the value of domain-specific transformations for compiler-test reduction.
Neither paper establishes that smaller product implementations preserve intent.

Consume [#352's lifecycle recommendation](finalization-lifecycle-2026-10-02.md):
only explicit Finalize starts expensive work, one verdict names one stable tree,
and local state is Feedback rather than CI authority. Consume
[#354's claim-local evidence model](evidence-provenance-2026-10-02.md): record
provenance, execution/completeness and the exact observation obligation. Missing
or unstable evidence never authorizes removal. A successful intervention supports
only “removable under recorded oracle O.” It establishes no slop/design judgment.

## Corpus selection and frozen evidence

Selection rule, encoded in [corpus.py](counterfactual-2026-10-03/corpus.py): cover
all six requested task shapes and every adversarial case with the smallest
executable fixtures that expose their differences. Two independent fixture
project families are used: Python **ledger** and JavaScript **router**. These
are constructed repository-shaped trees, not two sampled upstream repositories;
no genuine agent trajectories were available for this corpus. The experiment
therefore cannot establish cross-repository or cross-model generality.

Each task freezes source segments, required oracle assertions, independent
holdout assertions, requested intent and exclusions **before search**. The oracle
and holdout are written into fresh copies, outside the removable source set.
Candidate generation cannot change them. All A/B/C configurations for a task use
one identical `oracle_id`, including runtime, repeat policy and bounds. Asserted
comparability is checked by [summarize.py](counterfactual-2026-10-03/summarize.py).
Source changes are the comparison subject, not a change of oracle.

All assertions and fixtures were authored by this researcher. “Prior” below means
a constructed pre-existing regression scenario, not provenance from authentic
project history. “Candidate” means a simulated candidate-authored assertion.
Holdouts are independent of the search algorithm and never consulted during
selection; they share researcher authorship with the corpus. This is weaker than
independent external acceptance. No coverage or mutation evidence was gathered
for any task. No build/typechecker evidence was required by these tiny oracles;
interpreter/module loading failures reject candidates. Completeness means only
the listed assertions ran, never that requested behavior is fully observed.

| Task / shape / project | Required oracle | Held-out contract or intent | Evidence gaps |
| --- | --- | --- | --- |
| bug / repair / ledger | Prior: negative and positive amounts | Unseen negative and zero amounts | Small input sample |
| weak-feature / feature / ledger | Candidate: result is a string | Mixed case normalizes to lowercase | Candidate assertion admits identity |
| wrong-expectation / feature / ledger | Candidate: doubling task returns triple | Explicit doubling assertion | Candidate test encodes its own wrong behavior |
| performance / refactor / ledger | Prior: lookup result on iterable/indexed input | Indexed-only fixture forbids scanning | Functional oracle misses non-functional requirement; not a latency benchmark |
| architecture / architecture / ledger | Prior: default provider result | Caller supplies a different provider | Future extensibility unobserved |
| ordering / feature / ledger | Contract: doubled nonnegative total | Unseen total | Interchangeable implementations; orders choose different minima |
| flaky / repair / ledger | Prior amount case plus injected alternating failure | Unseen negative amount | Three observations disagree; no reduction authorized |
| api / API extension / router | Prior: existing route | External consumer imports added API | External consumer absent from required oracle |
| integration / dependency/integration / router | Prior: modern route | Dynamic import of intentionally duplicated legacy shim | No real package resolution/install; vendor/generated file excluded |
| dynamic / feature / router | Prior: static route | Dynamic plugin registration and invocation | Framework-like reachability absent from oracle |
| paired-history / refactor + feature / router | Prior route and explicit normalization example | Unseen routing/normalization input | Scripted abandoned rename group; no real agent history |
| boundary / refactor / router | Prior: identity behavior | Named adaptation boundary requested for maintainers | Both behavioral oracles miss maintainability intent |

The wrong-expectation original already contradicts its holdout. No removal is
accepted there; do not count that original defect as harm caused by reduction.
Integration's unchanged vendor/generated contribution stays in every candidate,
and is charged in total edit cost rather than silently disappearing.

## Arms, search and repeat policy

A removes final-patch groups in file → diff-hunk → function-like segment order.
Hunks come from the final diff with three lines of context. Segments are explicitly
bounded top-level declarations supplied by the fixture, not a generic semantic
extractor. The `hunk` configuration omits the fine search level; `semantic` includes
it. Revisit the ordered groups to a fixed point after accepted removals.

B prepends groups of surviving segments introduced in the same scripted turn,
then runs the identical A levels. Turn ids, before/after tree fingerprints and
segment ids are the entire retained search history. No prompts, transcripts or
source text live in that history. Corpus segments supply the independent replay
source; fingerprints alone cannot reconstruct reverted or superseded source.
Partially superseded real-agent trajectories remain unmeasured.

C separately tries one frozen identity-collapse transform on boundary. Other
C rows record an initial oracle/holdout run with zero applicable candidates;
these are **not** successful transform trials. No admitted klin transform is
created. D, agent-proposed search, was not run. The repair trials below are not D.

This is ordered greedy group deletion, **not ddmin** or a replication of TRIM.
A broader final-patch subset/complement search could discover the paired rename
without trajectory. B's advantage here demonstrates a grouping opportunity, not
unique information or superiority over all final-patch strategies.

Run both orders within each coarse-to-fine level, both atomizations, and two
independent repetitions: 12 tasks × 3 arms × 2 orders × 2 atomizations × 2 repeats
= **288 configurations**. Each ordinary observation invokes the frozen command
once. The known flaky fixture requires three executions, retains all outcomes,
and stops search if they disagree. No retry-until-green behavior or pass caching.

A/B audit every surviving eligible segment against the final candidate. Only
conclusive rejection of every single removal permits `one_minimal=true`. This is
one-minimal under the supplied segment vocabulary and oracle; it permits paired
removals and says nothing about global optimality or design. C makes no such claim.

## Raw measurements and results

[results.json](counterfactual-2026-10-03/raw/results.json) retains every group
attempt, audit, tree fingerprint, command exit/output/reason/duration, candidate
retention set, before/after cost, absolute reduction, relative share, completeness,
feedback bytes and surfaced count. [summary.tsv](counterfactual-2026-10-03/raw/summary.tsv)
contains all 36 per-task/arm summaries, including calls, separated times, harmful
outcomes, one-minimality, repeatability and history storage.

Edit cost is added plus deleted lines against the fixed source base, computed
from `ndiff`; a replacement costs two. It is not a quality score. Each table
value below is constant over eight configurations unless stated otherwise.

| Task | Initial cost | A removed | B removed | C removed | Removal/intent result |
| --- | ---: | ---: | ---: | ---: | --- |
| bug | 4 | 2 (50%) | 2 (50%) | — | Holdout preserved; unused diagnostic helper removed |
| weak-feature | 2 | 2 (100%) | 2 (100%) | — | Harmful: required normalization lost |
| wrong-expectation | 2 | 0 | 0 | — | Original contradicts contract; wrong oracle prevents repair |
| performance | 2 | 2 (100%) | 2 (100%) | — | Harmful: scanning restored |
| architecture | 4 | 4 (100%) | 4 (100%) | — | Harmful: provider extension lost |
| ordering | 4 | 2 (50%) | 2 (50%) | — | Holdout preserved; different retained helper by order |
| flaky | 4 | 0 | 0 | — | Inconclusive initial evidence, no search/minimality claim |
| api | 1 | 1 (100%) | 1 (100%) | — | Harmful: external API removed |
| integration | 2 | 1 (50%) | 1 (50%) | — | Harmful: compatibility shim removed; generated edit retained |
| dynamic | 1 | 1 (100%) | 1 (100%) | — | Harmful: dynamic plugin removed |
| paired-history | 6 | 0 | 4 (67%) | — | Holdout preserved; coordinated rename reverted |
| boundary | 3 | 3 (100%) | 3 (100%) | 3 (100%) | Behavioral holdout passes; requested boundary lost |

Across configurations A executes **376** required-oracle commands, B **420**,
and C **120** (most C rows only establish original observations). Search-group
attempts are 216/276/8; final singleton audits add 48/32/0. Initial commands and
audits are included in oracle counts; two original/final independent holdout
observations per configuration are recorded separately. B spends 44 more oracle
commands than A, about 12%, with one additional task reduced. There is no universal
cost/benefit threshold justified by these fixtures.

Total external command times, including holdouts, are A **20.275 s**, B **21.938 s**,
C **10.795 s**. Prototype analysis/materialization/cleanup time is separately
A **0.907 s**, B **0.946 s**, C **0.504 s**. Native klin analysis time is explicitly
**0**: no counterfactual implementation exists in the baseline, and the experiment
does not invoke klin. These timings measure disposable Python orchestration and
process startup on macOS, not klin Finalize performance or real project builds.
[environment.json](counterfactual-2026-10-03/raw/environment.json) records Python
3.14.8, Node v24.17.0, platform, corpus digest and execution bounds. Commands run
sequentially within a configuration; unrelated validation work ran on the host,
so small timing differences are not causal evidence of algorithm speed.

All same-order/same-atomization repetitions reach identical final fingerprints.
Ordering chooses either `a` or `b` on ordering: two distinct one-minimal trees of
identical cost. Finer atomization changes execution counts but no final tree in
this corpus. A's paired-history final tree is one-minimal even though B removes
four lines together. One-minimality therefore does not establish necessity.
The 24 flaky configurations retain mixed initial outcomes and make no claim.

## Harmful removal and human intent review

Seven task identities produce harmful removals under A and B (56 of 96
configurations each); boundary's eight applicable C configurations also lose
intent. These are repeated constructed counterexamples, **not harm rates**.
Six task identities fail executable holdouts; boundary requires intent review.
The search did not see those assertions or the human answer during selection.

The repository owner reviewed five described interventions on 2026-10-03:
provider extension removal, external API removal, and named identity-boundary
removal. The first answer was **“Reject all three reductions.”** A follow-up
review of the explicitly required compatibility shim and indexed-lookup removals
answered **“Reject both reductions.”**
[human-review.json](counterfactual-2026-10-03/raw/human-review.json) preserves the
question, answer and scope. This is actual human review of described requested
intent, not a blinded review of source quality or all corpus candidates. Other
tasks have researcher contract/rubric evaluation and no human review. Machine-run
rows retain `human_review=not-run`; this later review is a separate evidence item.

Thus neither an oracle-green reduction nor a smaller patch is “better” by
definition. The boundary counterexample survives stronger functional testing;
the explicit intent still rejects it. Design intent needs its own authority.

## Agent experience and repair cost

Each configuration surfaces at most **one** successful candidate, independently
of explored count. Feedback is 260–298 UTF-8 bytes for surfaced cases, zero where
none is surfaced. Unexplored/unaccepted candidates are never dumped into agent
context. Full developer records remain available separately.

Three fresh research sub-agents each received one frozen candidate packet and a
short intent instruction, without explored candidates. They share this session's
model family; exact model versions and a second family were not controlled.
The first trial allowed one response; A/C then received a second repeated REVIEW
over their restored source (see below). These are research agent-response turns,
not measured host/klin Turns, and do not establish repair-loop behavior in a long
real task. No token counts, latency or host automatic continuation cost is claimed.

| Arm / task | Packet bytes | Additional response turns | Observed action | Repair evaluation |
| --- | ---: | ---: | --- | --- |
| A / boundary | 641 | 2 including repeat | Restore named adapter | Intent-preserving repair |
| B / paired-history | 845 | 1 | Accept coordinated reversal | Requested routing/normalization preserved |
| C / boundary | 641 | 2 including repeat | Restore named adapter | Intent-preserving repair |

The agents explained the distinction between preserved behavior and requested
intent. None made an unrelated refactor or optimized the reduction metric.
The repeated-experiment results below evaluate the repair-loop risk. Assignment
text adds about 1 KB per trial, recorded separately in the agent reports. Context
figures exclude system instructions and later tool output. A/C were explicitly
told the original boundary, so their success does not show that agents infer lost
intent unaided, and the unequal trial tasks do not support ranking arm repair
quality. Agent-proposed search outperforming deterministic search is unmeasured.

[ax/](counterfactual-2026-10-03/ax/) contains the exact surfaced packets and reports.
Agents ran their own local intent/behavior tests, not the frozen oracle. A
separate [verification](counterfactual-2026-10-03/raw/ax-verification.json) re-ran
the frozen oracle and holdout on all three returned/accepted candidates: all pass.
For every other task/arm, raw `agent_turns=null` and `repair_quality=not-run` expose
unmeasured repair cost; do not interpret them as zero-cost repairs.

A supplemental [repeat experiment](counterfactual-2026-10-03/raw/loop-results.json)
ran A and C on the **exact** source returned by the repairing agents, including
an added blank line. [loop.py](counterfactual-2026-10-03/loop.py) checks the replay
source equals those files byte-for-byte. Both again found the identical simplified
candidate fingerprint. These two runs are separate from the 288 primary runs;
their command/analysis times and costs remain in the supplemental records.

Each agent received one repeated REVIEW packet, **969 bytes**, while its restored
working code remained intact. Both chose **keep**, changed zero code files, and
explained that a repeated unapplied proposal requires no repair. Total measured
candidate-packet context was **1,610 bytes each** for A/C, versus 845 for B's
single-response trial, excluding assignment text/tool output. A/C spent one extra
response each despite making no second repair. The reducer would repeatedly
propose the harmful removal; the bounded REVIEW semantics prevented a code repair
loop across these two observations. Blocking or auto-applying that proposal was
not tested and would risk alternation between removal and restoration. Long
sessions, ignored intent instructions and repeated model proposals remain unknown.

## Storage, execution and trust cost

Serialized history is **176–353 bytes per task**, one or two events, stored once
in [trajectory.json](counterfactual-2026-10-03/raw/trajectory.json), **2,979 bytes**
including task keys. Growth is linear in events plus group ids; path identities
can reveal project structure, and stable fingerprints allow correlation. The
source-bearing corpus and selected AX packets are charged separately. Raw
measurement results occupy **877,668 bytes**, including bounded command outputs;
that diagnostic archive is not required retained trajectory. No full prompts,
agent transcripts or every-candidate source snapshots are archived. Reproduce
accepted trees from corpus segments plus `retained`, or the frozen C transform.

[Security observations](counterfactual-2026-10-03/raw/security.json) measure:
2 s command timeouts, 16 KiB output bounds, inherited-pipe and closed-pipe
background descendants, rejected assertions, tree drift, cleanup and preservation
of a dirty caller file. Timeout and output exhaustion are inconclusive; source
writes invalidate the observation. Normal parent exit also kills its remaining
process group. Both descendant cases leave no delayed marker after 5.1 s.
Every candidate copy is removed after success, rejection or bounded termination;
no Git reset/clean or write to the caller's source tree occurs.

This is same-group termination, not containment of descendants that create a new
session or processes surviving an uncatchable controller kill. Controller signal
cleanup and hostile commands are not verified. Commands inherit the local
execution environment; they could access network, credentials or arbitrary paths.
The known fixtures use no network or package installs. A disposable copy/worktree
is sufficient for accidental source isolation, **not** a sandbox for untrusted
project commands. Future execution must retain SPEC 9.3's process cleanup, 300 s
per-command/600 s deadline contract and §15's Feedback/Enforced distinction; these
2 s research bounds neither weaken #341 nor propose replacement production limits.

## Reproduction and SPEC implications

From the repository root, using Python 3 and Node already installed:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 docs/counterfactual-2026-10-03/probe-test.py
PYTHONDONTWRITEBYTECODE=1 python3 docs/counterfactual-2026-10-03/run.py --output /tmp/klin356
python3 docs/counterfactual-2026-10-03/summarize.py /tmp/klin356
PYTHONDONTWRITEBYTECODE=1 python3 docs/counterfactual-2026-10-03/run.py --security --output /tmp/klin356
PYTHONDONTWRITEBYTECODE=1 python3 docs/counterfactual-2026-10-03/verify-ax.py
PYTHONDONTWRITEBYTECODE=1 python3 docs/counterfactual-2026-10-03/loop.py
```

The last two commands validate stored agent responses and re-experiment on their
restored source, not repeat model invocations. They refresh the corresponding
checked-in verification observations. Fresh runs cannot
reproduce host timing exactly; oracle ids, conclusions and retained trees should
match under the recorded runtime/fixture basis. Different runtimes require a new
basis rather than silently pooling results. The baseline governs lifecycle/trust
interpretation; fixture source is self-contained and no baseline binary is needed.

No SPEC amendment is admitted. A separately authorized future experiment would
need SPEC contracts for: an explicit opt-in Finalize budget; candidate/original
tree identity and frozen evidence provenance; inconclusive resource/flaky/drift
states; REVIEW without compulsory agent repair; bounded surfaced candidates;
inspectable command/attempt/time counts; and independent CI remeasurement. It must
never count local trajectory as CI authority or alter normal Stop work.

If pursued, the minimum **research** capability is an isolated final-patch reducer
with one frozen evidence set, explicit budget, one REVIEW candidate and a replay
recipe. Add compact projected turn groups only when replayable history already
exists; do not collect source snapshots or transcripts to support this one fixture
win. Normal users should see only “REVIEW: candidate preserves listed evidence;
inspect intent,” while developers can inspect candidate digest/patch, oracle
command and version, attempts and split times. Acceptance never auto-applies code.

Missing adoption evidence: authentic multi-repository agent tasks, retained real
supersession history, broader final-patch searches, independently authored external
acceptance, longer repair-loop trials, multiple model families and real build/test
cost. The constructed dependency case does not measure package installation or
integration infrastructure. Those gaps prevent either a positive product decision
or a justified claim that trajectory is useless. No global removable-share/BPDR
threshold is used.

**Keep counterfactual analysis experimental/optional**.
