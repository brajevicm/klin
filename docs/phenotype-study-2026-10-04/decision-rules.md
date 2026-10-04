# Product decision rules

These rules are study version 1. They are necessary conditions for #357
dispositions; none is a scalar score and one strong axis cannot compensate for
a failed safety axis.

Rates use adjudicated labels when present and primary labels otherwise.
`unresolved` is never silently removed from the denominator: resolved-label
rates state both the resolved denominator and the unresolved share.

## 1. Common evidence requirements

A user-facing capability, whether blocker or REVIEW, needs all of:

1. **Natural support:** at least 3 affected agent changes in at least 3
   repositories, and at least 2% of eligible agent changes are affected by a
   useful finding (`valid-regression` for a blocker;
   `valid-regression|valid-review` for review evidence).
2. **No single-change concentration:** no single change may contribute more
   than 50% of the useful sites used to meet the bar.
3. **Reproducibility:** every counted result has a frozen measurement basis;
   `partial|unsupported|unavailable|tool-error|invalid-syntax|
   local-resolution-incomplete` is not a clean zero.
4. **Hard negatives:** every applicable preregistered hard-negative family has
   at least one evaluated case, or the candidate is deferred.
5. **Identity:** repeated-finding/attention claims use compatible #425 or
   source-prototype identity; ambiguous identities are not guessed.
6. **No AI-specific claim from rate alone:** agent and matched-human rates are
   reported separately. Equal/lower human rate is not required for product
   usefulness, but an “AI-specific” claim requires evidence beyond this study's
   descriptive comparison.

If a phenotype has fewer than 3 eligible agent changes, it cannot satisfy the
natural-support bar in this study.

## 2. Fast blocking admission

A new ordinary-Stop blocker may be recommended for **admit for implementation
research** only if every condition below holds.

### Prevalence and safety

- the common natural-support bar holds using `valid-regression` only;
- among resolved natural labels, at least 95% are `valid-regression`;
- natural `undesired` is <=2% of resolved labels;
- natural `unresolved` is <=10% of all labeled findings;
- zero preregistered hard-negative case is a compulsory false block;
- zero known incomplete measurement is counted as PASS.

A source ticket's earlier “BLOCK candidate” does not waive these conditions.

### Closed-loop repair

At least 8 valid Active finding deliveries are observed across at least 3
controlled tasks and both agent families. Of those deliveries:

- `correct-repair / valid-delivered >= 0.75`;
- `appeasement / valid-delivered <= 0.10`;
- `harmful-repair = 0`;
- every harmful behavior change, even if the detector becomes green, counts as
  harmful repair;
- paired task outcomes have no case where Active is incorrect/partial while its
  Shadow pair is correct solely because of the intervention;
- phenotype-specific pairwise benefit uses only pairs where that phenotype was
  the sole surfaced intervention. `Active-better` means the Active final change
  has higher independent task correctness than Shadow, or equal task
  correctness with fewer adjudicated `valid-regression` residual sites and no
  increase in `undesired` residual sites. `Shadow-better` is the mirror.
  Unresolved task correctness is incomparable;
- `Active-better - Shadow-better >= 2` among those attributable pairs.

If fewer than 8 valid deliveries occur, the candidate cannot become a new
blocker from this study; it may still qualify for REVIEW or benchmark-only.

### Attention

- unnecessary human escalations caused by the capability are <=1 per 100
  eligible controlled runs;
- a compatible finding identity is surfaced at most once per lifecycle pass
  unless new evidence changes the decision.

### Stop cost

On the controlled warm 1M/20-changed fixture, the candidate itself must have:

- median incremental intrinsic klin time <=15 ms;
- maximum of five controlled iterations <=25 ms;
- zero extra parses when the required syntax facts were already parsed;
- zero unchanged-source reads caused solely by the candidate;
- zero whole-tree walk or whole-base checkout on the warm path;
- total comparable warm Stop median regression <5%.

Cold/strict remain inside SPEC 13. An optimization miss may cost time but may not
turn unknown evidence green.

## 3. Finalize/REVIEW admission

A candidate may be recommended as **keep as Finalize/REVIEW evidence** when all
of these hold:

### Prevalence and intervention safety

- the common natural-support bar holds using
  `valid-regression + valid-review`;
- among resolved labels,
  `(valid-regression + valid-review) / resolved >= 0.80`;
- `undesired / resolved <= 0.15`;
- `unresolved / all labels <= 0.25`;
- no hard-negative family shows a systematic intervention route: two or more
  undesired findings of the same hard-negative mechanism rejects the current
  predicate until narrowed and re-preregistered.

### Closed loop

At least 6 valid Active deliveries occur across at least 3 controlled tasks and
both agent families.

For cases where the correct outcome is repair:

- correct repair >=60%;
- appeasement <=20%;
- harmful repair <=5%.

For contextual review cases, `justified-retention` is a successful outcome and
the agent must not be forced to eliminate the evidence. A human escalation is
acceptable only when the label says human judgment is required.

If fewer than 6 valid deliveries occur, a candidate may not become new
user-facing REVIEW evidence in this round; it is benchmark-only at most.

### Human attention

- `undesired` human interruptions are <=5 per 100 eligible natural changes;
- the same compatible finding identity is not repeated in one readiness pass;
- the default message is sufficient for the human to identify the site,
  concrete fact and available keep/repair decision without an internal klin
  concept.

### Readiness/CI cost

For a native readiness candidate:

- median intrinsic klin time <=1 s;
- p95 intrinsic klin time <=2 s;
- no new daemon, watcher or network dependency.

For a named external recipe:

- external-tool median <=5 s and p95 <=15 s on the controlled readiness
  workload;
- timeout is <=30 s and yields unavailable/partial, never green;
- no network at check time;
- exact tool/rule/config basis is recorded;
- klin time and external-tool time are reported separately.

Exceeding these limits means benchmark/defer unless #357 records a specific
human-value reason and preregistered evidence shows the slower placement is
still usable. It never justifies moving the work to Stop.

## 4. Configuration and implementation-cost bar

A candidate cannot be admitted in this round when its minimum implementation
requires any of these without separate evidence establishing that the mechanism
is indispensable:

- a daemon/background service;
- runtime network access for Stop or ordinary gate;
- a second parser pipeline for syntax klin already parses;
- persistent full ASTs;
- unbounded history/cache;
- arbitrary ambient-shell execution presented as a klin-owned reproducible
  detector;
- mandatory repository topology configuration solely to make an automatic
  phenotype measurable.

A named recipe may require one pinned external executable, but it must not
require project-owned analyzer configuration to claim the stronger named-recipe
basis unless that configuration is itself frozen and measured.

Configuration beyond `{}`, installation steps, CI changes, host portability,
new files/services and maintenance/versioning are reported independently in
#460. A capability with materially greater setup than a same-value alternative
is deferred in favor of the simpler one.

## 5. Benchmark/product-metric only

Use **benchmark/product metric only** when the phenotype itself is supported but
a user-facing intervention fails an admission axis. This includes any of:

- at least 2 useful natural or independently judged controlled occurrences, but
  the 3-change natural-support bar is not met;
- useful detection but insufficient controlled intervention opportunities;
- acceptable classification with excessive human attention;
- useful evidence whose current detector has too many unresolved measurement
  holes;
- a repair signal that is easy to appease;
- acceptable semantic evidence with unacceptable runtime/configuration/
  maintenance cost.

Benchmark-only means no Stop failure, no readiness interruption and no public
claim that klin protects the user from the phenotype.

## 6. Reject/defer

Use **reject/defer for low value or unsafe precision** when any of these holds:

- zero useful natural findings and no independent controlled evidence of value;
- `undesired > 25%` of resolved natural labels;
- a repeated harmful-intervention mechanism is observed;
- the predicate cannot distinguish a preregistered hard negative without facts
  the proposed product surface does not have;
- measurement incompleteness is routinely indistinguishable from zero;
- repair commonly optimizes the detector while preserving the underlying
  problem;
- the implementation violates the operational bar and no narrower measured
  form survives.

“Defer” is used when a concrete missing measurement could change the decision;
“reject” is used when the frozen predicate itself fails. The final report names
which one.

## 7. Disposition hierarchy

For each registry row, #357 chooses exactly one of its four requested outcomes:

1. **admit for implementation research** — all intended-placement bars pass;
2. **keep as Finalize/REVIEW evidence** — REVIEW bars pass, even if blocker
   bars do not;
3. **benchmark/product metric only** — phenotype/value evidence exists but a
   user-facing bar fails;
4. **reject/defer for low value or unsafe precision** — insufficient or unsafe.

Do not select a stronger disposition when a weaker placement is the first one
whose complete bar passes.

For a capability already shipped at `study_commit`, “admit” means **retain at
its current strength**. If a shipped capability fails the safety/repair bar, the
result is a follow-up to narrow, demote or remove it; existing shipment is not
evidence that it passed this study.

## 8. Special frozen cases

### #363 Ruff

Python injection and swallowed-error are REVIEW-only candidates. They cannot
graduate to Stop in this study regardless of spare latency. Swallowed-error
repair must count narrowed-handler appeasement as appeasement, not success.

### #355 design/reuse

A justified exception is a successful REVIEW outcome. Forcing a family,
registration or wrapper relation merely to erase the finding is not repair.

The TypeScript component-cycle row is complete only when relevant local
dependency edges are proved under #445 semantics. A located known-local hole is
partial.

### #353 test integrity

Restoring assertion syntax without restoring observable verification is
appeasement. Candidate-authored tests cannot independently prove correctness.

### #364 dependencies

A new dependency REVIEW row makes no claim that the package is bad or missing.
Registry existence/age is not silently imported into the decision.

### #356

Counterfactual/removable analysis is deferred before outcome inspection and has
no phenotype row in study version 1.
