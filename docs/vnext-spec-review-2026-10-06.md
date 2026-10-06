# Adversarial review of the vNext core SPEC (#493)

Date: 2026-10-06. Subject: the first draft of `docs/SPEC.md` on branch
`issue-493-vnext-core-spec`. This note records the review that #493 requires
before the core implementation roadmap opens, and what the draft changed in
response.

## Method

Three independent reviewers read the draft. None of them edited a file.

| Lens | Inputs the reviewer read | Findings |
| --- | --- | --- |
| Traceability | #493, #475 with its #484 amendment, #484 note, #492, #452, #358, the #425 and #354 notes | 9 major, 19 minor, 0 blocker |
| Consistency and implementability | the draft against itself and against every 0.x section it carries forward | 2 blocker, 12 major, 12 minor |
| UX, AX, DX, performance, trust | the draft, the appeasement audit, the perf baseline, #492 | 11 major, 10 minor |

The traceability reviewer found no readiness remnant and no research result
written as a product commitment.

## Owner decisions taken during the review

The review raised four questions that only the owner could settle. The owner
answered them on 2026-10-06. `docs/SPEC.md` section 0.4 records each.

1. **Exit precedence.** ERROR 2 > FAIL 1 > INCOMPLETE 3 > 0. The draft had
   #475's INCOMPLETE over FAIL. Two reviewers showed that it hid proven
   failures, and that an agent could turn exit 1 into exit 3 by making a
   file unreadable.
2. **A hole the change opened keeps the Stop stamp red**, with no block. The
   draft made it green, so the next prompt inherited the hole, and an agent
   could hide a FAIL that way. #493 says such states remain non-green.
3. **`klin setup --rebase-window`.** A narrow reversal of #475's "no public
   reset": a person-only command moves the stamp to the branch base that
   `klin check` uses. It cannot silence what CI would fail. It answers the
   destructive de facto reset (deleting the state directory) and a merge of
   the default branch inside a turn.
4. **An unmatched accepted entry is a review item** at `klin check` and a
   note at the Stop. The draft made it a FAIL, which turned CI red after a
   semantics change and needed a special person-remedy class in the engine.

## Findings and dispositions

The table groups findings that make the same point. "C", "T" and "U" name
the consistency, traceability and UX reviewers, with their finding numbers.

| Finding | Disposition | Where |
| --- | --- | --- |
| C1: about 16 inherited 0.x "exit 2" cases would stop the whole run | Applied. Error kinds with run or capability scope. A capability error leaves the other rows usable. | 7.3 |
| C2, C3: exit 2 from the hidden ingress denies every tool call and blocks every Stop (old binary, removed spelling, usage error, panic) | Applied. The ingress exits 2 only to block or deny. Hook-line spelling frozen. Project-scope files keep legacy lines until legacy dispatch goes. | 10.1, 10.10, 17.3 |
| C4, U7: deleted-test ask undefined with no block left, MAY, "every later Stop" | Applied. MUST ask when a block remains, otherwise the stamp stays red and the question comes later. Evidence gap recorded. | 9.2, 6.6 |
| C5, U5: inheritance of tree-level holes, docs-only repositories | Applied. Per-reason table: can be inherited, and its site. `no-source-root` became `nothing-measured`. | 7.2 |
| C6, U5: no remote gives exit 3, shallow clone gives a vacuous pass | Applied. No remote passes with a note. A present candidate that does not resolve is exit 2 naming `fetch-depth`. | 6.5 |
| C7, T8: "required" circular, no UNKNOWN or advisory class, no `ambiguous` reason | Applied. Required defined, non-holes listed, advisory unknown is a note, `ambiguous` added. | 7.2, 4.4 |
| C8, T2: person-remedy item judged differently at Stop and check | Resolved by owner decision 4. The engine returns one item. The Stop renders it as a note. | 6.1, 7.6 |
| C9: JSON fields missing | Applied. Field tables, enum tolerance, run-level holes, capability `execution`, perf fields under `diagnostics`. | 11.7 |
| C10, T7: journal line shape, legacy lines, basis for report | Applied. Journal schema 2 is the check document plus hook fields. Schema 1 lines read as 0.x and are "not compared". | 13.1, 8.3 |
| C11: 0.x 9.5 shapes neither kept nor dropped | Applied. Restated with vNext command names. | 10.6 |
| C12, C22, T19: 0.3 map errors and gaps | Applied. Table completed, cross-references fixed, 8.4 and 8.5 backlog prose made non-normative. | 0.3 |
| C13: Action args with `--gate`, old pinned versions | Applied. Migration error for 0.x flags on `klin check`. Action refuses versions without `check`. | 11.3, 12.3 |
| C14: "tool error at Stop" is vacuous | Applied. Removed from the Stop rules and tests. | 10.4, 19.2 |
| C15: rebuild-cannot-finish has no CLI trigger | Applied in part. Only the rebuild path is pinned. The hole stays a contract. | 19.2 |
| C16: 50 ms as a CLI test, dispatch not observable | Applied. 50 ms moved to performance evidence. Dispatch shown with an invalid `klin.json`. | 19.2, 19.3 |
| C17: missing test items | Applied. | 19.2 |
| C18: reference test and `--reference` exit | Applied. Exit 0 anywhere. | 11.6 |
| C19, U20: guard deny list, fabricated events, `klin update` | Applied. Denies `setup`, `update`, `__agent`, and the 0.x spellings without an end date. | 10.8 |
| C20, U13, T21: opt-in versus "reads no configuration" | Applied. Filesystem walk, no parse, no git, inside 50 ms. | 5.1, 10.2 |
| C21: status states undefined | Applied. | 11.4 |
| C23, T18: missing ADR rows | Applied. | 0.5 |
| C24: selector collisions, legacy `radius` versus a person's | Applied. | 5.3, 17.1 |
| C25: row words and summary line | Applied. | 11.3 |
| C26: missing `--config PATH` | Applied. Exit 2. | 5.1 |
| T3: SARIF completeness | Applied. Complete for the changed-line claim, coverage claim `unverified`, empty report complete. | 9.4 |
| T4: #425 rules missing | Applied. Pairing, `persists`, move targets, duplicate count. | 8.4 |
| T5, U10: stale entries after a semantics bump | Resolved by owner decision 4. | 7.6, 8.3 |
| T6: no per-capability execution axis | Applied. | 7.3, 11.7 |
| T9 to T17, T20: attribution, unrecorded choices, dedup, network wording, harness fail-closed, duplication non-goal | Applied as section 0.4 rows or one-line fixes. | 0.4, 2.2, 10.6, 16.3 |
| T31: fixed release counts in 17.3 | Applied. Invariants stay, counts go to the roadmap. | 17.3 |
| T32: status enum freezes for #344 | Applied. The states are defined and listed for #344. | 11.4, 17.5 |
| U1, U2 | Resolved by owner decisions 2 and 1. | 6.6, 7.4 |
| U3, U4 | Resolved by owner decision 3. | 6.6, 18.7 |
| U6: Enforced overclaims | Applied. Required check, non-zero is failure, history depth, pinned versions. Review items in job summary and annotations. | 16.2, 12.3 |
| U8: Cursor notices invisible | Applied in part. `klin status` and every `klin report` scope show undelivered notices. Not applied: carrying the notice into the next blocking `followup_message`, because #492 forbids spending an agent turn on a person-only notice. | 10.7, 11.4, 13.2 |
| U9: `klin check` text reaches agents too | Applied. The renderer rules bind every renderer, and hole text never suggests push, install or deletion. | 10.6 |
| U11: "compatible basis" undefined | Applied. Same capability and semantics version. Policy changes are reported only. | 8.3 |
| U12: basis records could add Stop work | Applied. Changed scope only, no extra git or walk. | 8.1, 14.3 |
| U14: performance envelope edges | Applied in part. Release limit and envelope read together, 100-file row rule, contributor 300k relative measure. No new ceiling for the 100-file row. | 14.2 |
| U15: `klin check` side effects | Applied. Base worktree cleanup, no lock wait, atomic cache writes. | 6.3 |
| U16: silent `{}` | Applied. `config:` line and a note when the base held `klin.json`. | 5.1 |
| U17: SARIF command not found | Applied. Detail `command-not-found`. | 7.2, 9.4 |
| U18: status cannot say why the window is red | Applied in part. `status` shows the stamp verdict and the open regressions. Not applied: printing accepted-entry text to paste, which the 0.x rule against naming a way to accept debt in klin's own output still covers. | 11.4 |
| U19: `--reference` behind `policy`, `setup --user`, "klin cannot see CI" | Applied in part. Wording fixed for `setup` and section 15. `--reference` stays under `policy`: its flags are stable and its content is documentation. | 11.1, 11.6, 15 |
| U21: agent-facing text says "run `klin setup`" | Applied. "Ask a person to run `klin setup`". | 10.6 |

## What remained open after the first pass

- The deleted-test ask rests on small evidence (3 #361 runs, #484 case 8).
  The SPEC states that, and section 18.2 requires evidence for any other ask.
- The window rebase of this round was later removed (see the second pass).
- Project-scope hook files stay on legacy lines until legacy dispatch is
  removed. The roadmap sets that release and its minimum version.
- No second review pass ran over the revised draft. The owner's acceptance
  is the next gate.

## Second pass and the frictionless revision

A second reviewer read the revised draft on 2026-10-06. It found 2 blockers,
7 major and 9 minor findings, confirmed 15 first-round fixes, found 5 only
partly fixed, and found no readiness remnant and no new hot-path work.

The two blockers were `nothing-measured` firing on any changed scope with no
measurable file, and configuration errors that left the stamp green against
#492. Several major findings showed that the first-round owner decisions 2
and 3 (an opened hole keeps the stamp red, `klin setup --rebase-window`)
created states that nobody could clear without person work.

The owner then set the governing rule: **klin should be almost fully
frictionless.** `docs/SPEC.md` section 2.3 states it. It supersedes
first-round decisions 2 and 3:

- Only a FAIL that the agent can fix blocks or keeps the local window red.
  Holes, coverage notes and configuration errors are told once, recorded,
  and judged in CI.
- klin's own limits (`unreadable`, `unresolved`, `ambiguous`,
  `resource-limit`, `left-scope`) are coverage notes. They never make
  `klin check` incomplete, and the summary counts the files not measured.
- A file that the base parsed and that the change made unparseable is a
  `parse-lost` FAIL. This closes the route of hiding a finding by breaking
  syntax, without a red state that nobody can clear.
- Exit 3 remains only for holes someone can act on: `tool-error`,
  `comparison-unproven`, whole-tree `nothing-measured`, and `unsupported`.
- `--rebase-window` is gone. The Stop re-anchors by itself when the
  merge-base with the default branch moves into the turn, and holds the sites
  that the default branch holds.

Second-pass findings and dispositions ("S" numbers):

| Finding | Disposition | Where |
| --- | --- | --- |
| S1: `nothing-measured` on changed scopes | Applied. Whole-tree runs only. An empty changed scope is complete. | 7.2 |
| S2: configuration errors leave the stamp green | Resolved by the frictionless rule: they stay non-green in CI, the journal and the report, and never hold the local window. Recorded in section 0.4 against #492's wording. | 0.4, 2.3, 6.6 |
| S3: an opened hole red forever | Resolved: klin's limits are coverage notes, and a lost parse is an agent-fixable FAIL. `status` shows why a window is red. | 7.2, 11.4 |
| S4, S5, S17: rebase refs, report counts, edge cases | Resolved: the rebase command is removed. The re-anchor writes a `reanchor` line, and the report counts re-anchored regressions apart from fixes. | 6.6, 13.2, 13.3 |
| S6: force-push exits 2 | Applied. A rewritten push `before` in a full checkout falls back to the merge-base with a note. | 6.5 |
| S7: journal schema 2 drops fields | Applied. The stop line keeps `verdict`, `asked`, `config_hash`, `timing` and `session`, and holds the result under `result` with `command: "stop"` and a null `exit`. | 13.1 |
| S8: capability error at Stop contradicts 10.10 | Applied. 10.10 covers run-scope and ingress failures only. A FAIL beside a capability error spends its block. | 7.3, 10.4, 10.10 |
| S9: scope narrowing blocked under Enforced | Applied. `left-scope` is a coverage note. | 7.2 |
| S10: ask and FAIL sharing a block | Applied. One block carries both. | 9.2 |
| S11: JSON gaps | Applied. Holes are run- or gate-sited, rows carry `state`, findings carry `kind`, `needs-policy` selectors are `unsupported`. | 7.2, 11.7 |
| S12: Cursor notices pile up | Applied. A notice expires with its window. | 10.7, 13.2 |
| S13: opt-in walk with no stopping point | Applied. Stops at the first `.git` entry. | 5.1 |
| S14: worktree cleanup race | Applied. One worktree per run, removed by its owner. | 6.3 |
| S15: invisible removed-spelling notice | Applied. Told through the host's person channel. | 10.10 |
| S16: unknown harness version against 10.10 | Applied. Named as fail-closed. | 10.10 |
| S18: smaller inconsistencies and test gaps | Applied. | 13.1, 15, 17.2, 19.2 |

Open after the frictionless revision:

- `parse-lost` has one false positive: valid syntax that the grammar does not
  read yet. Same-tree pass-through bounds it at the Stop, and CI needs an
  `except` entry until a grammar update ships.
- The re-anchor adds one `git merge-base` to each Stop and to each new stamp,
  and a second measurement of the changed files on the Stop where the
  merge-base moved. Its cost is not yet measured on the 1M row.
- No review pass has read the frictionless revision.

## Third pass

A third reviewer read the frictionless revision on 2026-10-06. It found 3
blockers, 10 major and 11 minor findings. It found no readiness remnant.

The blockers all came from new machinery: the automatic re-anchor did not
persist past one Stop, it could forgive the agent's own commits on a default
branch with no remote, and a line over the source-line ceiling reopened the
route that `parse-lost` closed.

The owner chose two directions:

1. **Merge fallback instead of a re-anchor.** A merge of the default branch
   into the turn takes the existing 0.x branch fallback against the new
   merge-base. It is skipped when the stamp's parent is an ancestor of the new
   merge-base, which is the case when the turn's own commits moved it.
2. **An opened gap is a review item.** A gap the change opened (a new file no
   grammar reads, a new unresolved or ambiguous form, a scope cut by a file
   other than `klin.json`) is an `unmeasured` review item at `klin check`
   with a pull-request annotation, and a note at the Stop.

Third-pass findings and dispositions ("P" numbers):

| Finding | Disposition | Where |
| --- | --- | --- |
| P1, P2, P4, P5, P7, P18, P23: re-anchor persistence, forgiving turn commits, rebase order, deletions from main, cost, bookkeeping, stale default branch | Resolved by direction 1. The fallback judges the branch window, so deletions and code the default branch holds are not new. The descent test skips turn-made moves. Old stamps and rewritten merge-bases record and take no fallback. Status names the default-branch ref. | 6.6, 11.4, 15 |
| P3: resource ceiling reopens the hiding route | Applied. `parse-lost` widened to `measurement-lost`, which covers the line ceiling and invalid manifests. | 7.2 |
| P6: stacked pull requests | Recorded as the one known red window the agent cannot close, and as the trigger for 18.7. | 6.6, 18.7 |
| P8: `parse-lost` under-specified | Applied. Strict parse, tolerant readers excluded, manifests, renames, a gate-independent `id` with null `check` and `line`, other rows count the file. | 7.2, 11.7 |
| P9: the false positive loops per prompt | Applied. One accepted entry of gate `measurement-lost` holds the file for every capability, named to the person. | 7.2 |
| P10, P11: opened gaps hide FAILs, "complete" overclaims | Resolved by direction 2, plus a `not_measured` count in every `check` document and the #358 row in section 0.4. | 0.4, 7.2, 11.7, 12.3 |
| P12: configuration errors and the stamp | Applied. A new `unmeasured` verdict keeps the stamp without red or block. | 6.6, 15 |
| P13: "hole" leftovers | Applied. A cache that cannot rebuild is a capability `git` error. A detector's own work bound is a `work-limit` hole, which keeps #493's "resource exhaustion is INCOMPLETE". A file over a byte ceiling falls under 7.2. | 7.2, 15, 18.3 to 18.5 |
| P14: vacuous holes at the Stop | Applied. Only `work-limit` can occur at the Stop, and it never blocks. | 7.2 |
| P15, P16, P17, P19: wording, unasked deletion, forced red, "told once" | Applied. | 2.3, 6.6, 10.6 |
| P20: count fields | Applied. `not_read`, `gaps`, `limits` and run-level `not_measured`. | 11.7 |
| P21: `nothing-measured` ambiguity | Applied. Only when no capability applies to the whole tree. | 7.2 |
| P22: state directory deleted | Applied. A surviving turn ref restores the stamp. | 6.6 |
| P24: test gaps | Applied. | 19.2 |

Open after the third pass:

- Stacked pull requests keep a red window at the Stop until a Stop ends green
  or the change reaches the default branch.
- The merge fallback and the default-branch lookup are not yet measured on
  the 1M row.
- No review pass has read this revision.

## Fourth pass

A fourth reviewer read the third revision on 2026-10-06. It found 1 blocker,
9 major and 10 minor findings, and no readiness remnant.

The blocker was the third-pass merge rule again: the descent test skipped
the branch fallback on fast-forward pulls and on pulling the default branch
back after a merge, because commit ancestry cannot tell those apart from the
agent's own push. When the fallback did fire, it widened the window to the
whole branch.

The owner accepted a design that needs no inference from ancestry:

- When the merge-base with the default branch moves, every Stop until the
  stamp moves also judges against the current merge-base, for the files that
  the default branch changed since the recorded merge-base. A site that
  either base holds is held. The window stays turn-sized.
- No second base applies when the default ref is HEAD's own branch, which is
  the no-remote case. An agent's push to the default branch is a known local
  limit that CI's push window covers.
- The merge-base is cached under (HEAD commit, default-branch commit).

Fourth-pass findings and dispositions ("Q" numbers):

| Finding | Disposition | Where |
| --- | --- | --- |
| Q1, Q2, Q3, Q15: descent test, whole-branch widening, `asked` on rebase, false fixes | Resolved by the two-base rule. A rebase onto the default branch uses it too, and the branch fallback stays only for missing state and unrelated history, with `asked`, `told` and `intervened` kept. `rebased` covers sites the agent did not change. | 6.6, 13.2, 15 |
| Q4: `unmeasured` stamp grows without bound | Applied. A run-scope configuration error writes `green` and records that the Stop judged nothing. `unmeasured` stays for capability errors the change caused and for a klin failure, and `status` shows since when. | 6.6, 11.4, 15 |
| Q5: grammar-lag loop | Applied. The finding carries the first error node's line and column. The accepted entry stays matched while the file still fails to parse. 18.7 names grammar lag. | 7.2, 18.7 |
| Q6: form changes | Applied. Encoding, NUL bytes, symbolic links and `.gitattributes` binary or generated marks are `measurement-lost`. | 7.2 |
| Q7: `left-scope` through an agent-editable file | Applied. klin measures the file once under the base's scope, and a new or worsened finding stays a FAIL. | 7.2 |
| Q8: rename detection | Applied. Pinned to `-M50%`, user configuration ignored, "or" fixed to "and". | 7.2 |
| Q9: `unreadable` against `not_read`, annotation volume | Applied. `unreadable` covers languages klin reads only. One review item per file and reason. The Action annotates failures first and counts the rest. | 7.2, 12.3 |
| Q10: git process invariant | Applied. Reworded to name the cached merge-base, the ref reads and the `--is-ancestor` check. | 10.2, 14.4 |
| Q11, Q14, Q20: leftovers, 16.1 amendment, journal and table omissions | Applied. | 0.3, 0.5, 9.1, 13.1, Appendix A |
| Q12, Q13: naming the accepted entry, entry shape | Applied. The Stop says that a person can hold the file and points to `klin policy`. 0.3 records the 4.8 amendment, and the entry matches by file alone. | 0.3, 7.2 |
| Q16: crash leaves an earlier green | Applied. A Stop writes `unmeasured` first and replaces it. | 6.6, 15 |
| Q17: selection of `measurement-lost` | Applied. It fires when a selected capability reads the file, and renders as its own row. | 7.2 |
| Q18: unclassified 0.x holes | Applied. One table maps each named case to a class. | 7.2 |
| Q19: tests outside the CLI seam | Applied. `work-limit` and the 0.x-stamp case are named as `#[cfg(test)]` pins or use a 0.x binary. | 19.2 |

Open after the fourth pass:

- Stacked pull requests, and grammar lag before a person holds the file, are
  the two known red windows the agent cannot close (18.7).
- An agent's push to the default branch is held at the Stop by the second
  base. CI's push window judges it.
- The second base and the merge-base cache are not measured on the 1M row.
- No review pass has read this revision.

## Fifth pass

A fifth reviewer read the fourth revision on 2026-10-06. It found 1 blocker,
10 major and 8 minor findings, and no readiness remnant. Merge handling was
the blocker for the third pass in a row: the two-base rule had no defined
way to combine two bases, and it blocked on the default branch's code in
several cases while it hid some agent reverts.

The owner chose **degrade to report**. When a tracked remote ref (the
default branch or HEAD's upstream) moves into the turn, when HEAD leaves the
stamped history, or when the stamp and ref are missing, that Stop is
advisory: it measures, blocks nothing, and tells its findings once. The next
prompt takes a fresh stamp. CI judges precisely. A repository with no
remote-tracking ref has no CI to judge what an advisory Stop skips, so it
never takes one: a local branch move changes nothing, and a branch switch or
missing stamp keeps the 0.x branch fallback. This replaces the second base,
and the branch fallback wherever a remote exists.

Fifth-pass findings and dispositions ("F" numbers):

| Finding | Disposition | Where |
| --- | --- | --- |
| F1 to F7, F13, F15, F16: two-base combination, survey paths, lost files from main, hidden reverts, cross-file blocks, local ref forgiveness, branch switches, file set, report markers, missing red windows | Resolved by the advisory window. No second base exists to combine. A repository without remote-tracking refs is never advisory through a local ref. Open regressions at an advisory Stop are `set-aside`. | 6.6, 13.1, 13.2, 15, 18.7 |
| F8: an agent switches a capability off by moving files | Applied. A `scope-lost` FAIL. | 7.3, 15 |
| F9: a nested `klin.json` triggers the configuration `green` | Applied. The hooks read only the worktree root's `klin.json`. A run-scope configuration error writes `unjudged`, which `status` and `report` show as "nothing judged". | 5.1, 6.6 |
| F10: the `measurement-lost` entry cannot hold | Applied. No value is ratcheted, the entry stays matched while the file is lost for any reason, and the names are reserved. | 7.2 |
| F11: `work-limit` with no action | Applied. The text names `except` or `false`, and a bounded capability must not reach its bound on the 1M row. | 7.2 |
| F12: form-rule determinism | Applied. Both sides use git's stored representation, invalid UTF-8 alone is not lost, the attributes are `binary` and `-text`, the symbolic link test uses the index mode, a `not-text` gap reason exists, and rename detection has no limit. | 7.2 |
| F14: cost wording | Applied. Ref reads on each event, `git merge-base` only on a changed pair, and the 1M cost stated as a requirement to verify. | 10.2, 14.4 |
| F17: annotation limits | Applied. `error` for failures, `warning` for review items, at most the host's limit of each, the rest in the summary. | 12.3 |
| F18: the `measurement-lost` row | Applied. Kind `built-in`. | 7.2, 11.7 |
| F19: an opened gap told only to the person | Applied. The agent sees the note too, without a block. | 7.2 |

Open after the fifth pass:

- `measurement-lost` from grammar lag before a person holds the file, and a
  cross-file finding caused by an earlier branch commit, are the known red
  windows the agent cannot close (18.7).
- An agent can make one Stop advisory by moving a remote-tracking ref or the
  history. CI judges the branch.
- The tracked-ref check is not measured on the 1M row.
- No review pass has read this revision.

## Sixth pass

A sixth reviewer read the fifth revision on 2026-10-06. It found 1 blocker,
11 major and 13 minor findings. No `parse-lost` text remained.

The blocker: the agent's own `git push` moved the upstream ref and made the
Stop advisory, so every "commit and push" turn blocked nothing. The owner
asked for the fixes and one more review.

Sixth-pass findings and dispositions ("X" numbers):

| Finding | Disposition | Where |
| --- | --- | --- |
| X1: own push makes the Stop advisory | Applied. Rule 1 fires only when a tracked ref's reflog since the stamp holds an entry that is not a push. A newly tracked upstream is recorded, not advisory. | 6.6 |
| X2: amend and squash trigger advisory | Applied. Rule 2 is a branch change, rule 3 a recorded merge-base that left HEAD history. Same-branch rewrites keep the turn window. | 6.6 |
| X3: "a remote means CI" | Applied. "Where CI runs `klin check`", in 6.6 and 16.1. | 6.6, 16.1 |
| X4: advisory lasts until the next prompt | Applied. The advisory Stop takes the fresh stamp itself. | 6.6 |
| X5: advisory against build blocks, verdict precedence | Applied. Build failures still block at an advisory Stop. The verdict table is ordered, first row wins. | 6.6 |
| X6: "no remote-tracking ref" ambiguous | Applied. The test is "no `refs/remotes/*` ref exists". | 6.6 |
| X7: hooks and `klin check` read different files | Applied. Every command reads only the worktree root's `klin.json`, and names a nested one as ignored. | 5.1 |
| X8, X9, X10: `scope-lost` undefined, loops on requested moves, partial moves hide findings | Applied. A renamed policy path is followed and becomes a `moved-pin` review item. A deleted one is a note and a review item. A file moved out of a scope keeps its base scope membership, so its new findings still fail. The `scope-lost` kind is gone. | 7.3, 15 |
| X11: `-text` is not "not text" | Applied. The attributes are `binary` and `-diff`. | 7.2 |
| X12: clean filters | Applied. `working-tree-encoding` and end-of-line handling are decoded natively for every reason. A filtered path is a coverage note. | 7.2 |
| X13: index mode for symlinks | Applied. `lstat`. | 7.2 |
| X14, X17: leftovers and an unimplementable journal marker | Applied. | 0.3, 0.5, 13.1, 17.2 |
| X15: unasked deletion across an advisory Stop | Applied. It is told, and 2.3 names the exception. | 2.3, 6.6 |
| X16: `unjudged` forgives a red window | Applied. A red stamp stays red, and a moved `unjudged` stamp sets regressions aside. | 6.6, 13.2 |
| X18: a stamp with no merge-bases after a pull | Applied. Its first Stop is advisory when a tracked merge-base is not an ancestor of the stamp's parent. | 6.6 |
| X19: forgiving fallback without a remote | Applied. The fallback judges from the stamp's parent while it is readable, and 16.1 states the rest. | 6.6, 16.1 |
| X20: unlimited inexact renames | Applied. Exact renames unlimited, inexact ones under a fixed limit of 1,000 candidates. | 7.2 |
| X21: annotation job total | Applied. | 12.3 |
| X22: the opt-in walk rule | Applied. The filesystem rule is exact, and `GIT_DIR` and `core.worktree` are named as not seen. | 5.1 |
| X23: unadmitted Stop work | Applied. | 14.4 |
| X24: readiness remnant in 0.x 9.3 | Applied. Not carried forward. | 6.4 |
| X25: test gaps | Applied. | 19.2 |

## Seventh pass and acceptance

A seventh reviewer read the sixth revision on 2026-10-06. It found 2
blockers, 6 major and 6 minor findings. No readiness remnant and no text of
the abandoned designs remained, apart from a few stale table rows.

Both blockers were in the advisory triggers again: a merge after a separate
fetch went undetected, and an amend of a pushed commit or `git switch -c`
made a Stop advisory. A blocker had come from this one area in five passes in
a row, because git history alone cannot reliably tell other people's commits
from the agent's own rewrites.

The owner chose to apply the seventh-pass fixes and accept the SPEC with the
remaining edge cases written down as known limits (section 18.7), with no
eighth review.

Seventh-pass findings and dispositions ("Z" numbers):

| Finding | Disposition | Where |
| --- | --- | --- |
| Z1: merges after a separate fetch, rebases, missing reflogs | Applied. Rule 1 reads HEAD's own reflog for `merge`, `pull`, `rebase` or `reset` entries with a moved default-branch merge-base, compared by position. Without a reflog, a moved merge-base alone is advisory. | 6.6, 18.7 |
| Z2: amend of a pushed commit, `switch -c`, paused rebase | Applied. Only the default-branch merge-base is tracked, never the upstream. Rule 2 needs the stamp's parent to leave HEAD history. An in-progress rebase counts as its `head-name` branch. | 6.6 |
| Z3: stale rows | Applied. | 10.2, 15, 19.2 |
| Z4: `filter=` hiding route | Applied. A filter the change added to a base-measured path is `measurement-lost`. A new filtered path is a `filtered` gap. An inherited one is a `filtered` coverage note. | 7.2 |
| Z5: decoding | Applied. Only the working-tree side is decoded, only in-tree `.gitattributes` are read, carriage returns are removed before line measurement, and an undecodable encoding is a coverage note or gap. | 7.2 |
| Z6: `klin.json` in a subdirectory | Applied. The hooks tell a notice to move it. The 17.2 row and the two `tests/base.rs` pins are named. | 5.1, 17.2, 17.4 |
| Z7: move with rewrite reads as deletion | Applied. The mixed case is defined, and move with rewrite is a stated known limit. | 7.3, 18.7 |
| Z8: unadmitted stamp capture | Applied. Admitted in 6.3 and 14.4, with the lock order and the prompt mark. | 6.3, 6.6, 14.4 |
| Z9: cost wording | Applied. `pre_tool` excluded, reflog reads listed. | 6.6, 10.2, 14.4 |
| Z10: verdict order in an advisory window | Applied. `unjudged` comes before `advisory`, and `aborted` is written only under the lock. | 6.6 |
| Z11: stale `moved-pin` turns into an error | Applied. A pin that selects nothing in either tree is a review item. | 7.3 |
| Z12: rename scoring across git versions | Applied as a known limit. | 7.2, 18.7 |
| Z13: `unmeasured` with two meanings | Applied. The verdict is now `aborted`. | 6.6, 11.7, 15 |
| Z14: the 6.3 Stop row | Applied. | 6.3 |

The known limits accepted with the SPEC are in section 18.7. Each roadmap
ticket that implements the area must measure them.
