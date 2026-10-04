# Blind labeling rules

These rules belong to study version 4. They are frozen before #457 produces
natural-study outcomes.

The labeler judges the code/change evidence, not whether a named detector is
“good.” Packets therefore omit detector identity and provenance where the case
remains judgeable without them.

## Primary label

Every measured finding receives exactly one:

### `valid-regression`

Use only when the packet is sufficient to establish all of these:

1. the reported condition is present in the after change;
2. it is new or worsened relative to the base;
3. it conflicts with the task/repository contract strongly enough that a
   reasonable reviewer should require repair before accepting the change;
4. repairing the underlying problem can preserve the requested behavior.

A stylistic preference, an inherited problem, or a true fact whose acceptability
depends on a product/design choice is not a regression.

### `valid-review`

Use when the evidence is concrete and accurate, and seeing it at review/readiness
is useful, but the packet does not justify compulsory repair.

Typical cases:

- an intentional-looking public-contract or dependency decision that the task
  did not already authorize;
- a design/reuse relation with more than one coherent design choice;
- an error-handling or test-integrity fact whose correctness depends on intent;
- a candidate where keeping the current code with a documented reason is a
  legitimate outcome.

If the task already explicitly authorizes the exact decision and no further
human choice remains, do **not** use `valid-review` merely because the detector's
fact is technically true. Use `undesired` for an unnecessary intervention.

### `undesired`

Use when a product intervention at the registered placement should not have
occurred. This includes:

- false or mislocated evidence;
- inherited/unworsened debt emitted as new work;
- generated/framework/test/excluded code that the frozen predicate should not
  judge;
- a hard negative or explicitly authorized task behavior;
- a metric change whose requested “repair” would make the task worse;
- a duplicate/repeated signal that adds no new decision under compatible finding
  identity;
- a claim that treats a partial/unsupported measurement as clean or failing.

A finding may state a syntactically true fact and still be `undesired` when
surfacing it would predictably pressure the agent/person away from explicitly
requested behavior.

### `unresolved`

Use when the packet cannot support one of the other labels without guessing.
Examples:

- missing task/design intent that materially changes the judgment;
- partial parse or local dependency resolution;
- unavailable external evidence;
- ambiguous identity or contradictory evidence;
- insufficient surrounding code to tell a required adapter/default/error
  boundary from a defect.

Do not force unresolved cases into a favorable or unfavorable label.

## Separate fields

The primary label never encodes these dimensions. Record them independently.

### Task correctness

One of:

- `correct`: available independent evidence supports the requested behavior;
- `incorrect`: independent evidence shows a task requirement is not met;
- `partial`: some requirements pass and some fail;
- `unresolved`: available evidence cannot decide.

Candidate-authored tests in the current run are dependent evidence and cannot by
themselves produce `correct`.

### Delta status

One of:

- `new`;
- `worsened`;
- `held`;
- `unresolved`.

A `held` site cannot be a `valid-regression` for this study.

### Behavior preservation

For an observed repair or proposed mandatory repair:

- `preserved`: requested behavior remains satisfied;
- `changed-intended`: behavior changed, but the frozen task asked for it;
- `changed-harmful`: the intervention lost or contradicted requested behavior;
- `not-applicable`;
- `unresolved`.

### Human judgment

- `required`: more than one coherent product/design choice remains;
- `not-required`: the evidence and task contract determine the repair/keep
  outcome;
- `unresolved`.

A `valid-review` normally has `required`, but the fields remain separate so
exceptions are visible.

## Hard-negative rules

Apply the same four labels. Do not create a special “negative” label.

- deliberate compatibility duplication: a reuse/duplication signal is
  `undesired` unless it reveals a separate unrequested defect;
- one-off external-boundary adapter: forwarding/reuse evidence is
  `undesired` when the boundary is the responsibility;
- appropriate high-complexity algorithm: a complexity intervention is
  `undesired` when decomposition would obscure the algorithm without reducing
  responsibility;
- intentional platform-specific test skip: `undesired` when the platform
  condition is part of the test contract;
- candidate-authored but correct test: do not discount it merely because an
  agent wrote it;
- legitimate broad cross-package task: radius/architecture evidence needs the
  actual contract; use `undesired` if the broad change is explicitly required;
- intentional public API addition: `undesired` if the task explicitly
  authorizes that exact contract change and no decision remains, otherwise
  `valid-review`;
- architecture migration with old/new patterns coexisting: `valid-review`
  only when the migration leaves a real design choice; otherwise
  `undesired`;
- generated/framework code: `undesired` when the frozen exclusion says it is
  out of scope;
- dynamic registration/reachability: `unresolved` when the static evidence
  cannot prove the relation;
- broad exception/subprocess/eval boundary: `valid-review` when the broad
  boundary is real but intent matters, `undesired` when the task/repository
  already establishes it as the intended boundary.

## Repeated identities

Under a compatible #425/prototype identity:

1. the first surfaced instance is labeled normally;
2. a repeated instance in the same lifecycle pass is separately recorded as
   `repeated=true`;
3. if it adds no new site/evidence/decision, its intervention label is
   `undesired` for attention accounting even when the underlying original
   finding was valid;
4. a measurement-basis change or ambiguous identity never permits deduplication
   by guess.

## Blindness

Primary packets hide phenotype/detector, source ticket/disposition, population
(agent/human), agent family and Active/Shadow status wherever possible.

The labeler may request one hidden field only by marking the case
`needs-unblind:<field>`. The unblinded answer and reason are recorded. The
primary label after unblinding remains traceable and the case is excluded from
claims of full blindness.

## Label passes

### Primary

Label every packet once under the rules above. The entire primary worksheet is
frozen before any aggregate phenotype results are shown to the labeler.

### Secondary

Re-review, without deleting the primary value:

- every primary `unresolved`;
- every packet that required unblinding;
- every hard-negative packet;
- every packet of a phenotype that later has >10% primary `undesired`;
- a deterministic sample of exactly `ceil(0.20 * N)` of the remaining `N`
  packets (zero when `N = 0`), selected by ascending
  `(SHA-256("357-secondary-v1:" + packet_id), packet_id)`.

The coordinator merges the selected packets into one list ordered by SHA-256 of
`357-secondary-v1:<packet_id>`. The secondary reviewer does not see why a
packet was selected.

The secondary reviewer must not see aggregate per-phenotype rates before
finishing the selected packets.

### Adjudication

A disagreement keeps both labels. Adjudication records:

```text
packet_id
primary_label
secondary_label
final_label
reason
adjudicator
```

No row is silently overwritten. Product decisions use `final_label` where an
adjudication exists and the primary label otherwise.
