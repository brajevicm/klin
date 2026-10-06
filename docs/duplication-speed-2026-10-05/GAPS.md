# Duplication gate: gap map and test plan (#480)

Date: 2026-10-06. Branch `issue-480-speed-prototype`.

## Target

klin's duplication gate works well when all of these hold:

1. **Precision.** At least 80% of reported regions are real copies (owner
   decision, 2026-10-06).
2. **Recall.** The gate reports at least 40% of labeled copies of 60 tokens or
   more for each language, on a blind set, at 80% precision (owner decision,
   2026-10-06).
3. **Ratchet.** The gate reports only duplication that the turn introduced. A
   copy that existed at the base never blocks.
4. **Cost.** The gate fits the #478 Stop limits. The owner keeps these limits
   unchanged (2026-10-06).
5. **Feedback.** An agent that receives a finding removes the copy. It does
   not evade the detector or make a harmful abstraction.

## Where we are

Version 2 baseline rules (`holdout.py`) on the label sets; the G4 version 3
design screen is reported below:

| Set | Rust precision | TypeScript precision | Rust recall | TypeScript recall |
|---|---:|---:|---:|---:|
| `calibration/` (design) | 22/26 = 84.6% | 23/23 = 100% | 22/100 | 23/58 |
| `calibration-holdout/` (version 2 design) | 53/59 = 89.8% | 89/93 = 95.7% | 53/233 | 89/517 |
| `calibration-holdout2/` (blind) | 26/26 = 100% | 128/140 = 91.4% | 26/105 | 128/880 |
| `calibration-introduced/` (G1; introduced-region candidates) | 27/40 = 67.5% | 15/17 = 88.2% | 27/227 = 11.9% | 15/155 = 9.7% |

Recall here is the share of labeled copies of 60 tokens or more that the rule
reports. Why version 2 misses copies, counted over all three sets:

| Language | Reason | Copies missed |
|---|---|---:|
| Rust | 60 to 99 tokens, below T = 100 | 198 |
| Rust | both spans in trait impls (E3) | 136 |
| Rust | spans overlap in one file | 2 |
| Rust | imports trimmed below T | 1 |
| TypeScript | fragment: less than 50% of a span inside complete functions | 1,213 |
| TypeScript | fewer than 60 tokens outside JSX | 1 |
| TypeScript | spans overlap in one file | 1 |

The survey sees no copy under 60 tokens and no copy with an edit. The real
recall is lower than these numbers.

## Gaps

Each gap has a test that can fail. The gaps are in the order that the
decision needs them.

### G1. The evaluation measures existing code, not new copies

**Gap.** Every label set surveys all code in a repository. The gate judges
only what a turn adds. Agent-made copies may differ from copies that grew over
years: they may be longer, more often whole functions, and more often next to
their source. Precision and recall on existing code may not predict either
number at the gate.

**Proposed solution.** Build a corpus of introduced copies:

- **Historical:** commits in the nine pinned repositories that add a region
  equal to code that existed at the parent commit. `git log` and the
  prototype matcher can find them without labels.
- **Agent:** benchmark tasks where an agent works on a real repository. Record
  every turn diff.

**Test.**

1. Run version 2 on the introduced regions only. Label them blind.
2. Report precision and recall on introduced copies, next to the survey
   numbers.
3. Pass: precision at least 80% on introduced copies. If the numbers differ
   from the survey by more than 10 points, the survey stops being the design
   set and the introduced corpus replaces it.

**Result (2026-10-06).** Two blind model reviewers labeled 809 unique
introduced-region candidates; their 10 overlapping pair IDs agree exactly.
The scanner requires at least half of one span's lines to be added by the
commit, then finds its counterpart in the post-commit tree. It does not verify
that the counterpart existed in the parent commit, so the historical
parent-provenance part of G1 remains open. Version 2 misses the 80% precision
criterion in Rust and reaches 88.2% in TypeScript; recall is below 40% in both
languages. Rust precision differs by more than 10 points from the previous
blind survey, triggering use of this candidate corpus for further rule work
while parent provenance is verified. See
[G1 results](calibration-introduced/RESULTS.md).

### G2. Rust recall: copies from 60 to 99 tokens

**Gap.** T = 100 drops 198 real Rust copies. T = 100 was the lowest threshold
that met 80% with E2 and E3 alone.

**Proposed solution.** Lower T and add a shape rule, as the whole-unit rule did
for TypeScript. Candidates:

- a region that covers complete statements inside one block (statement-unit
  rule);
- a region that holds control flow (`if`, `match`, `loop`, `?`), not only
  declarations or calls with literal arguments.

**Test.** For each candidate at T = 60, 70, 80 and 90:

1. Measure on `calibration/` and `calibration-holdout/` (design sets).
2. Keep a candidate only if precision is at least 80% on both design sets and
   recall rises.
3. Freeze the best candidate in a commit.
4. Confirm on a new blind set (G9). Pass: at least 80% precision and higher
   recall than version 2 on the same set.

### G3. Rust recall: real copies inside trait impls

**Gap.** E3 drops every pair with both spans inside trait impls. That removes
the required-shape noise, but also 136 real copies, for example identical
`matches_rule` bodies and identical `Progress` logic.

**Proposed solution.** Replace the blanket E3 with narrower rules:

- drop the pair only if the match is mostly method signatures and trivial
  bodies (forwarding, a field read, `Ok(())`, `unimplemented!`);
- keep a pair whose matched region holds a method body with control flow.

**Test.** Same loop as G2: design sets first, freeze, then a blind set. Pass:
precision at least 80% and fewer than 136 missed trait-impl copies on the
design sets.

### G4. TypeScript recall: fragment copies

**Gap.** The whole-unit rule drops 1,213 real TypeScript copies. This is the
largest gap. Most real TypeScript copies are parts of a larger function: a
query and permission check in a tRPC route, error handling with analytics, a
debounced search.

**Proposed solution.** Report a fragment when its non-JSX logic is long
enough and it holds behavior. Candidates:

- a statement-unit rule: the region covers at least N complete statements
  outside JSX;
- a behavior rule: the region holds at least one call that is not a JSX
  element, plus control flow or an `await`;
- a higher non-JSX threshold for fragments only, for example 80 or 100.

**Test.** Compare candidates on the labeled design sets. The four current
sets hold 1,610 TypeScript copies of 60 tokens or more. Pass on a new blind
set: precision at least 80% and TypeScript recall above 30%.

**Design screen (2026-10-06; not blind confirmation).** Version 3 preserves
the version 2 whole-unit rule. It also keeps a fragment when both spans have
at least 60 non-JSX tokens, a call, either control flow or `await`, and two
complete executable statements. It clears 80% precision on all four labeled design
sets and improves TypeScript recall. Rust results are unchanged. See
[G4 results](g4-fragments/RESULTS.md). The v3 candidate remained open after
the design screen pending blind confirmation; the v3 result below does not
meet the criterion.

**Blind confirmation (2026-10-06; two-model review complete).** Two independent
models labeled all 200 sampled pairs with exact ID coverage. They agree on 91%
of full labels and 91.5% of copy/non-copy decisions (κ = 0.83). Weighted
TypeScript estimates differ across the precision gate: reviewer A reports
77.9% precision and 64.9% recall; reviewer B reports 88.5% precision and
58.7% recall. Since both estimates must meet the criterion, G4 is not
confirmed. A third blind model labeled the 12 TypeScript pairs where A and B
disagree on copy/non-copy status. Three-model majority resolves three as copies
and nine as non-copies; applied to those 12 pairs and the other A/B-agreed
statuses, the weighted estimate is 80.0% precision and 65.5% recall. This
secondary consensus estimate does not replace either independent estimate, so
the v3 candidate did not meet the two-review criterion. The known family-overlap
and parent-provenance caveats are recorded in the
[v3 packet](g4-fragments/blind-v3-2026-10-06/README.md).

**Blind v4 fresh-family confirmation (2026-10-06).** V4 preserves the v2
whole-unit rule and raises the fragment-only non-JSX floor to 70. On the
pre-registered fresh-family TypeScript population, two independent reviewers
both report 96.1% precision; recall is 49.1% for A and 56.2% for B. Their
copy/non-copy agreement is 92% (κ = 0.80). Both clear the 80% precision and
above-30% recall criteria, so G4 passes for this candidate on this test. The
fresh-family population has 51 kept and 88 dropped pairs across `actual` and
`documenso`; the sample covers all 51 keeps and 49 drops, with no family overlap
with prior labeled sets. The parent-provenance
limitation remains open. See the
[v4 packet and results](g4-fragments/blind-v4-2026-10-06/README.md).

### G5. Copies under 60 tokens

**Gap.** The survey never sees a copy under 60 tokens. A median 60-token
region is 9 to 12 lines, so short copied helpers are invisible.

**Proposed solution.** First measure whether short copies matter.

**Test.**

1. Survey one design repository for each language at T = 30 to 59.
2. Label a random sample of 200 pairs blind.
3. Measure the version 2 shape rules at T = 30, 40 and 50.
4. If no rule reaches 80%, record that short copies stay out of scope and
   state the limit in the SPEC.

### G6. Copies with edits

**Gap.** Exact matching misses a copy after a rename, a reordered statement or
an inserted line (`appeasement.py`). An agent that receives a finding can
evade it with one rename.

**Proposed solution.** Two tiers:

- tier 1: exact regions, as now;
- tier 2: identifier-normalized regions (a Type-2 clone), with stricter rules
  and only as REVIEW until it earns BLOCK.

**Test.**

1. Run `appeasement.py`. Count how many evasions tier 2 catches.
2. Survey the design sets with identifier normalization and label only the
   new pairs that tier 2 adds.
3. Pass: tier 2 precision at least 80% on its new pairs, and it catches the
   rename evasions in `appeasement.py`.

### G7. Cost at Stop

**Gap.** The corrected candidate misses Stop limits on large changes (klin
largest files 31.94 ms median after O1 and O2, limit 15 ms) and on cache size
(lossless evidence 8.7 MB or more, limit 3.73 MB). Version 2 adds per-region
syntax facts. Its cost is not measured.

**Proposed solution.**

- Integrate version 2 into the prototype and measure the extra cost.
- Use T = 100 for Rust to try a larger k and a smaller Rust index.
- Keep exact confirmation and lineage at `klin check`, and report INCOMPLETE
  at Stop when a budget runs out.
- The owner keeps the #478 limits unchanged, so a design that misses them
  does not ship.

**Test.**

1. Run `measure.sh` rows with version 2 on the pinned corpora, 10k and 300k.
2. The owner runs the 1M rows.
3. Pass: every #478 limit.

### G8. Ratchet and lineage

**Gap.** A ratchet must tell a new copy from an old one. #489 is not resolved:
the index has no family identity that survives edits, and the full token chain
does not fit the cache. Without lineage, a turn that edits near an old copy can
make the old copy look new.

**Proposed solution.** Key a finding by the content hash of the family plus
its file set. A finding is new when its family hash, or one of its files, was
not in the family at the base. Store only family hashes and member files, not
the chain.

**Test.**

1. Run the nine supplied cases in `lineage-cases.json` and the extension,
   split, merge and scope-exit examples in `LINEAGE.md`.
2. Write CLI tests in the prototype: copy an old region into a new file (new),
   edit inside an old copy (not new), move an old copy (not new).
3. Pass: every case gives the expected result, and the stored data fits the
   cache limit.

### G9. Label quality and corpus breadth

**Gap.**

- All labels are model judgments.
- Two reviewers in the third set labeled whole families at once, and one wrote
  rationales from templates.
- Each held-out set has one repository per language. The Rust part of the
  third set has only 28 pairs at T >= 100, and 27 are copies without any rule.

**Proposed solution.**

- **Agreement:** give 200 pairs to two reviewers, and measure how often they
  agree.
- **Human audit:** the owner labels 50 kept pairs and 50 dropped pairs. The
  audit gives the agreement between model and human labels.
- **Breadth:** each new blind set has at least two repositories for each
  language and at least 50 kept pairs for each language.
- **Single use:** a blind set confirms one frozen rule version. After that it
  joins the design sets.

**Test.** Pass: the two reviewers agree on at least 85% of pairs, and the
owner agrees with the model labels on at least 85% of the audit sample. If
either is lower, fix the label instructions before the next blind set.

### G10. Test-code leaks

**Gap.** Test files and fixtures entered the survey: actual's
`e2e/page-models` and documenso's `packages/app-tests`. The survey uses its
own path rules, not klin's test conventions.

**Proposed solution.** Use klin's test-range and test-path rules in the
survey, plus the common TypeScript monorepo names (`e2e`, `app-tests`,
`playwright`, `cypress`).

**Test.** Count the leaked pairs in the three sets before and after. Pass: no
labeled pair comes from a test path.

### G11. Feedback and agent behavior

**Gap.** Nobody has measured what an agent does with a duplication finding. It
can extract a good helper, make a harmful abstraction, or rename to evade the
check. `appeasement.py` shows that cheap edits evade exact matching.

**Proposed solution.** Phrase the finding as context for review, as FINAL.md
recommends. Name both copies. Do not demand an extraction.

**Test.** Seeded benchmark tasks where an agent copies code and receives the
finding:

1. Count extraction, evasion and harmful coupling in the agent's next turn.
2. Run the tasks with and without tier 2 (G6).
3. Pass: no evasion with tier 2, and extraction in most runs.

## Iteration protocol

Every gap uses the same loop. It keeps the rules honest.

1. **Design.** Change one rule on the design sets only. The design sets are
   all sets that some earlier rule version has seen.
2. **Freeze.** Commit the rule and its `rules.json` for the next blind set
   before any label of that set exists.
3. **Confirm.** Label the blind set with the same process. Join the labels.
4. **Decide.** Pass when precision is at least 80% for each language and the
   gap's own metric improves. On a pass, the blind set joins the design sets.
   On a fail, record the result, and the blind set still joins the design
   sets. Do not re-test the same set.
5. **Record.** Write `RESULTS.md` in the set's directory and add one row to
   the table in "Where we are".

The metric script for every step is `holdout.py measure`. Each gap that adds a
rule adds it to `holdout.py` as a new version, and the older versions stay
reproducible from their commits.

## Order

| Step | Gaps | Why this order |
|---:|---|---|
| 1 | G9 cross-model agreement, G10 test leaks (complete) | The other measurements depend on label reliability and a clean survey. Both steps are cheap. |
| 2 | G1 candidate corpus (measured; parent provenance open) | It decides which corpus the recall work tunes against; parent verification will settle the strict historical definition. |
| 3 | G4: improve v3 precision and run a fresh blind set | Reviewer A falls below the floor; the adjudicated estimate is exactly 80%, with no margin. |
| 4 | G2, then G3 | Improve Rust recall after the TypeScript candidate is confirmed. |
| 5 | G7 cost | Measure the final rule set, not each step. The 1M rows need the owner. |
| 6 | G8 lineage | Needed before any ratchet, and independent of the rules. |
| 7 | G6 tier 2, G11 feedback | After tier 1 works, test evasion and agent behavior. |
| 8 | G5 short copies | Lowest expected value. It may end as a stated limit. |

## Recall goal

The owner set it on 2026-10-06: at least 40% of labeled copies of 60 tokens or
more, for each language, on the blind set, at 80% precision. On the
introduced-region candidate set, version 2 recall is 11.9% for Rust and 9.7%
for TypeScript; parent-tree provenance is not yet verified.

## G10 result (2026-10-06)

Pairs that touch a test path, by definition, over the three sets:

| Set | klin's rules (`src/survey.rs`) | klin's rules plus monorepo folders | Kept by version 2 |
|---|---:|---:|---:|
| `calibration/` | 0 | 0 | 0 |
| `calibration-holdout/` | 2 (both copy) | 7 (5 copy, 2 boilerplate) | 2 (1 copy, 1 boilerplate) |
| `calibration-holdout2/` | 0 | 33 (all boilerplate) | 0 |

- The leaks change version 2 precision by at most one pair. G10 is not a
  precision problem for version 2.
- klin's own rule treats any `spec/` directory as test code. yazi's
  `yazi-shared/src/spec/` is a production module with 2 labeled copies, so klin
  would skip production code there. This affects every klin check that uses
  the test convention.
- klin's rule misses monorepo test folders: `e2e/`, `__mocks__/` and
  documenso's `packages/app-tests/`.
- `calibrate.py` now uses klin's rules, does not treat a Rust `spec/` as test
  code, and adds `e2e`, `__mocks__`, `playwright`, `cypress`, `fixtures` and
  folders that end in `-tests`, `_tests` or `-test`. Sets collected before
  this change used the older filter. Their survey stays as it was.

## G9 result: reviewer agreement (2026-10-06)

A second blind reviewer labeled the 200 pairs in `agreement/sample.json`
(`agreement/second-labels.json`). It wrote a rationale for each pair. The
sample holds 101 pairs that version 2 keeps and 99 that it drops, from all
three sets and both languages.

| Slice | Pairs | Same label | Same copy or not copy |
|---|---:|---:|---:|
| All | 200 | 89% | 94% |
| Kept by version 2 | 101 | 94% | 94% |
| Dropped by version 2 | 99 | 84% | 94% |
| Rust | 101 | 90% | 97% |
| TypeScript | 99 | 88% | 91% |

- Cohen's kappa for copy or not copy is 0.87.
- On the kept pairs, both reviewers find 94 copies of 101. The precision of
  version 2 does not depend on which reviewer labels.
- Most disagreements are between boilerplate and required-shape (7) or
  between copy and boilerplate (7 one way, 2 the other).
- Agreement between models passes the 85% limit. The 100-pair cross-model
  audit below provides the final G9 labeling evidence.

## G9 result: audit (2026-10-06)

Another model reviewer supplied `agreement/audit-labels.json` for the 100 audit
pairs. Its format (label, confidence, rationale) matches `AUDIT-PROMPT.md`. By
the owner's decision on 2026-10-06, this cross-model result is the final G9
labeling evidence; no human audit is required.

| Slice | Pairs | Audit vs first labels: same label / same copy decision | Audit vs second reviewer: same label / same copy decision |
|---|---:|---:|---:|
| All | 100 | 89% / 91% (kappa 0.81) | 88% / 92% (kappa 0.83) |
| Kept by version 2 | 50 | 86% / 86% | 92% / 92% |
| Dropped by version 2 | 50 | 92% / 96% | 84% / 92% |
| Rust | 62 | 89% / 92% | 85% / 92% |
| TypeScript | 38 | 89% / 89% | 92% / 92% |

- Every slice agrees on the copy decision at 86% or more, so the 85% limit is
  met.
- On the 50 kept pairs, the audit finds 44 copies (88%): 24 of 27 Rust and
  20 of 23 TypeScript. The first labels find 47 and the second reviewer 46.
  Version 2 stays above 80% precision under all three label sources.
- Kappa is low on the kept slice (0.15 and 0.56) because almost every kept
  pair is a copy. With so few non-copies, kappa says little there. The share
  of agreement is the useful number.
- The audit calls copies non-copies more often than the reverse: 7 pairs that
  the first labels call copy are boilerplate or required-shape in the audit,
  and 1 pair goes the other way.
- G9 is closed on cross-model evidence. These are model judgments, not human
  labels or evidence of developer intent.
