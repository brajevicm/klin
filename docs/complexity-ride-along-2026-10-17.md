# Complexity ride-along validation, 2026-10-17

No candidate rule passed, so once the follow-up ticket ships, `complexity`
stops a person for nothing and its findings go to the report-only tier of
#696. Until then the shipped binary still blocks on `cc`.

This note belongs to #697. It tests two candidate rules for the ride-along
row of `complexity`, and the `cc` row that blocks today, on a frozen sample of
agent pull requests in Rust and TypeScript. It is research only. It changes no
shipped behavior and no SPEC semantics. The change of behavior is a separate
ticket.

The method, the pins and the run are in
`complexity-ride-along-2026-10-17/protocol.md`. The method and the decision
rule were committed before any run on the sample, and nothing below was tuned
on it.

## The candidates

`complexity-ride-along-2026-10-17/rules.py` defines them. "Over the ceiling"
means over the `cc` ceiling that the finding carries.

| Candidate | Keeps a finding when |
|---|---|
| first | it is `new` and `cc` is over the ceiling, or it is `worsened` and `lines` grew by 20 or more |
| second | `cc` is over the ceiling, and it is `new` or `cc` rose |
| `cc` | `cc` is over the ceiling (the row that blocks today) |

## Sample and run

- 380 agent pull requests from AIDev v4: 180 Rust and 200 TypeScript, one
  per repository. Only 180 Rust repositories had an eligible pull request,
  so the Rust sample holds all of them. 48 Rust and 75 TypeScript pull
  requests have a single commit.
- klin measured all 380 in full, so no pull request was replaced. It found
  526 findings. 281 of them have at least one candidate.
- A pull request stands in for a Stop turn. It holds several turns, so its N
  is likely higher than N per turn. The verdict below holds for pull
  requests. The single-commit stratum is the closest thing to one turn.

## Labels

- Agents labeled the 281 findings in two passes under the unchanged
  `label-rule.md` of #585, rule 6 included. A change is a pull request from
  its merge base to its head. The passes agree on 94%, with kappa 0.88.
- An adversarial review checked the packets and labels. It found packet
  defects, listed in the protocol, but no label that a defect made wrong.
- **The spot-check was done by model judges, not by a person.** The owner
  decided this on 2026-10-18. Five judges gave blind verdicts on the 30
  findings drawn with the seed `697-spot-v1`: two Opus 5.5 runs, two Codex
  `gpt-6.1-sol` runs and one Sonnet run. Each judge read only the worksheet,
  the packets, `label-rule.md` and the clones. The owner gave each Opus and
  Codex run weight 2 and Sonnet weight 1, and the weighted majority of 9 is
  the verdict. `complexity-ride-along-2026-10-17/spot-check-agents697.tsv`
  holds every vote.
- The verdict differs from the agent label on 2 of the 30: P083 (weight 7)
  and P143 (weight 9), both from `appropriate` to `not-appropriate`. That is
  within the limit of 3, so no relabel round was due. Both packets are in
  the protocol's list of defects: their "before" leaves out the source that
  the function copies. Both agent labels rest on that copy. The judges'
  reasons judge the function's own structure and do not count the copy. So
  the packet defect, or a second reading of the label rule on duplication,
  may explain both changes.
- The two Opus runs agreed on 24 of 30 rows, the two Codex runs on 29.

## Result

N is the count of `not-appropriate` findings per 100 pull requests. Each N
comes with a bootstrap 95% interval over the pull requests. The exact Poisson
interval is for information only, because findings cluster: one pull request
holds 13 of the `not-appropriate` findings of every candidate.

| Candidate | Stratum | Changes | Kept | `appropriate` | N | Bootstrap 95% | Poisson 95% |
|---|---|---|---|---|---|---|---|
| first | pooled | 380 | 147 | 78 | 18.2 | 11.1–27.4 | 14.1–23.0 |
| first | Rust | 180 | 62 | 34 | 15.6 | 8.3–23.9 | 10.3–22.5 |
| first | TypeScript | 200 | 85 | 44 | 20.5 | 9.5–36.5 | 14.7–27.8 |
| first | single-commit | 123 | 19 | 11 | 6.5 | 2.4–11.4 | 2.8–12.8 |
| second | pooled | 380 | 192 | 85 | 28.2 | 19.2–39.2 | 23.1–34.0 |
| second | Rust | 180 | 98 | 44 | 30.0 | 18.3–43.9 | 22.5–39.1 |
| second | TypeScript | 200 | 94 | 41 | 26.5 | 14.5–44.0 | 19.9–34.7 |
| second | single-commit | 123 | 36 | 19 | 13.8 | 5.7–24.4 | 8.1–22.1 |
| `cc` | pooled | 380 | 261 | 92 | 44.5 | 33.7–57.1 | 38.0–51.7 |
| `cc` | Rust | 180 | 135 | 46 | 49.4 | 33.3–67.8 | 39.7–60.8 |
| `cc` | TypeScript | 200 | 126 | 46 | 40.0 | 26.0–57.5 | 31.7–49.8 |
| `cc` | single-commit | 123 | 51 | 20 | 25.2 | 14.6–38.2 | 17.1–35.8 |

- No candidate has a pooled N of 10 or less. Even the lower end of the first
  rule's interval is above 10.
- The `cc` row measures 44.5 out of sample, against 19.6 on the Next.js
  replay that the interim exception rests on. The unit differs: there a
  change was one commit, here it is a pull request.
- The first rule on single-commit pull requests (6.5) is the only figure
  under 10. The decision rule uses the pooled N, so it decides nothing. It
  suggests that a later design measured per turn may find a rule that
  passes.
- A `not-appropriate` ride-along finding asks an agent that is already
  stopped for a refactor nobody wanted. The best candidate keeps 69 such
  findings, which is N 18.2. They fall in 41 of the 380 pull requests, so
  about one pull request in nine gets at least one.

## Where the noise comes from

| Candidate | `new`, `not-appropriate` | `worsened`, `not-appropriate` | `new`, `appropriate` | `worsened`, `appropriate` |
|---|---|---|---|---|
| first | 58 | 11 | 40 | 38 |
| second | 58 | 49 | 40 | 45 |
| `cc` | 58 | 111 | 40 | 52 |

- Most noise of the `cc` row is `worsened` findings: a small edit to a
  function that was already over the ceiling, such as an added field or a
  `?`. The first rule removes most of it.
- The `new` findings are the same in all three candidates, and 58 of 98 are
  `not-appropriate`. They alone give N 15.3, so no rule that keeps every
  `new` finding over the ceiling can pass on this sample.
- By the labelers' own reasons, at least 30 of those 58 are functions that
  existed at the base: moved, ported, renamed, or an earlier version under
  rule 3. Twelve come from one pull request that ports `TinyMDE.js` to
  TypeScript. Two (P057, P170) come from the path-case mismatch in the
  protocol. klin's function identity does not follow these, so it calls
  them `new`.
- Without those 30, N would be 10.3 for the first rule, 20.3 for the second
  and 36.6 for the `cc` row. No candidate would pass even then, but the
  first rule would miss by less than one point. Function identity across
  moves and renames is the largest single source of noise on this sample.

## Decision

- No candidate earns a tier. `cc` has no row, and its findings fall to the
  report-only tier of #696. `complexity` stops blocking.
- The interim exception to ADR 0073, recorded in
  `complexity-calibration-2026-10-16.md`, ends with this note.
- The first rule would have sent some lines-only findings to the ride-along
  tier. Because it did not pass, they stay in the report-only tier of #696.
- The shipped binary still blocks on `cc` until the follow-up ticket changes
  it. That ticket is #696, or a new ticket if the owner prefers.
- This decision rests on the model-judge spot-check. Acceptance criterion 2
  of #697 names a person, so the owner's decision of 2026-10-18 stands in
  for it.

## Cases for CLI tests

All five judges and the agent label agree on these. The ticket that changes
`complexity` may use them as CLI test cases.

| Packet | Outcome | Function | Verdict | Why |
|---|---|---|---|---|
| P131 | `new` | a ~270-line migration function with a per-language if/else chain | `appropriate` | New code that is tangled when written |
| P197 | `worsened` | a long file-list function that gains a three-level nested closure | `appropriate` | A new responsibility, nested, in a long function |
| P100 | `worsened` | a command function over the `cc` ceiling where four calls gain `?` | `not-appropriate` | The change adds no branch, nesting or responsibility |
| P268 | `worsened` | a scanner loop that gains one early-return guard | `not-appropriate` | Rule 6: a guard adds no nesting |
| P121 | `worsened` | a long `main()` that gains one field in an object literal | `not-appropriate` | The function was long before, and the change adds no logic |

## Limits

- No person checked the labels. The model judges were stricter than the
  agent labels, so a person's check is more likely to raise N than to lower
  it, but that is an inference.
- The packet defects in the protocol were not repaired. The review found
  that none made an agent label wrong, but both labels that the spot-check
  changed sit on defective packets.
- The verdict holds for pull requests, not for single turns.

## Reproduce

Build the pinned binary as `protocol.md` says. The `person` column of
`labels-agents697.tsv` is the `verdict` column of `spot-check-agents697.tsv`.

```sh
python3 scripts/replay/replay.py run agents697 complexity --klin target/ride-along/klin-2adfde08
python3 docs/complexity-ride-along-2026-10-17/rules.py agents697 --klin target/ride-along/klin-2adfde08
python3 docs/complexity-ride-along-2026-10-17/labels.py summary agents697 --klin target/ride-along/klin-2adfde08
```
