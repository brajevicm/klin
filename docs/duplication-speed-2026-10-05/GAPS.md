# Duplication gate: gap map and test plan (#480)

Date: 2026-10-06. Branch `issue-480-speed-prototype`.

## Target

klin's duplication gate works well when all of these hold:

1. **Precision.** At least 80% of reported regions are real copies (owner
   decision, 2026-10-06).
2. **Recall.** The gate reports most real copies that an agent introduces in
   a turn. The owner has not set a recall limit yet. This plan proposes one.
3. **Ratchet.** The gate reports only duplication that the turn introduced. A
   copy that existed at the base never blocks.
4. **Cost.** The gate fits the #478 Stop limits, or the owner relaxes them
   with a stated reason.
5. **Feedback.** An agent that receives a finding removes the copy. It does
   not evade the detector or make a harmful abstraction.

## Where we are

Version 2 rules (`holdout.py`) on three label sets:

| Set | Rust precision | TypeScript precision | Rust recall | TypeScript recall |
|---|---:|---:|---:|---:|
| `calibration/` (design) | 22/26 = 84.6% | 23/23 = 100% | 22/100 | 23/58 |
| `calibration-holdout/` (version 2 design) | 53/59 = 89.8% | 89/93 = 95.7% | 53/233 | 89/517 |
| `calibration-holdout2/` (blind) | 26/26 = 100% | 128/140 = 91.4% | 26/105 | 128/880 |

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

**Test.** Same loop as G2. The design sets hold 463 fragment copies in
`calibration/` and `calibration-holdout/`, which is enough to compare the
candidates. Pass on the blind set: precision at least 80% and TypeScript
recall above 30%.

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
- If the limits still fail, give the owner the numbers for a limit per
  changed token, which the owner asked about on 2026-10-06.

**Test.**

1. Run `measure.sh` rows with version 2 on the pinned corpora, 10k and 300k.
2. The owner runs the 1M rows.
3. Pass: every #478 limit, or an owner decision that changes a limit.

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
| 1 | G9 agreement and human audit, G10 test leaks | The other measurements depend on labels and a clean survey. Both steps are cheap. |
| 2 | G1 introduced-copy corpus | It decides which corpus the recall work tunes against. |
| 3 | G4, then G2 and G3 | Largest recall gaps first. G4 alone is 1,213 missed copies. |
| 4 | G7 cost | Measure the final rule set, not each step. The 1M rows need the owner. |
| 5 | G8 lineage | Needed before any ratchet, and independent of the rules. |
| 6 | G6 tier 2, G11 feedback | After tier 1 works, test evasion and agent behavior. |
| 7 | G5 short copies | Lowest expected value. It may end as a stated limit. |

## Proposed recall goal

Set a recall goal so that the work has a target: at least 40% of labeled
copies of 60 tokens or more, for each language, on the blind set, at 80%
precision. Version 2 is at 15% to 25%. The owner decides the number.
