# Phenotype pilot on natural agent work, 2026-10-02

This note belongs to #357. It is the small natural-population pilot of the
#357 amendment of 2026-09-28 (proposal 2). It asks one question before #353,
#355 and #356 spend their cost:

> On real pull requests that coding agents opened, which candidate phenotype
> families occur often enough to justify more research?

It is research only. It adds no gate, changes no shipped behavior and edits no
SPEC semantics. It does not give the product dispositions of #357, and #357
stays open. A pilot outcome below only orders the research that follows.

## Terms

This note uses the words of #357, #361 and #362. They are not terms of
`CONTEXT.md`:

- A **change** is one pull request, from the merge base of its base and head
  to its head.
- A **family** is one candidate phenotype of the frozen list below.
- A **site** is one place a detector reports. A **finding** is a site that no
  exclusion of its detector covers.
- The **agent arm** is the changes that the AIDev dataset attributes to a
  coding agent. The **human arm** is the changes of the same repositories that
  no rule below attributes to an agent.
- A **review comment** is a comment on a pull request by a person other than
  its author.

## Distinction from #343

#343 measured how often klin `{}` fails on ordinary default-branch commits of
popular repositories. This pilot measures how often candidate families occur
on agent pull requests, and compares the same detectors on human pull
requests of the same repositories. The two samples share no repository by
construction of the source, and this note reports no #343 number as its own.

## Rules, written before any run on the sample

The commit that adds this section holds no result of the sample. No detector
ran on a change of the sample before it.

### Source and provenance of agent work

The agent arm comes from AIDev v4 (`hao-li/AIDev` on Hugging Face, revision
`c63c8a57a2de34fc03fa83722412824af4d8753b`, the files
`pull_request.parquet` with SHA-256
`c0b8e81e1d099905ef9ea420bf907d45179771ff5972afce6c219d8cafcef3e8` and
`repository.parquet` with SHA-256
`a08e34be4921c708be88a4ebd9e275b32f37fd442bb2770c0b69c94834dc6aa7`).
`pull_request` is the curated table of AIDev: pull requests in repositories
with more than 100 stars, created between 2024-12-24 and 2025-10-24.

AIDev records the agent of each pull request (`agent`). This note takes that
field as the provenance. It infers no authorship from the code.

### Agent arm

1. A pull request is a candidate when all of these hold in the AIDev tables:
   - the repository's `language` is `Rust`, `TypeScript` or `Python`.
   - the repository is not a fork.
   - the pull request has a `closed_at`, merged or not. An open pull request
     can still change its head.
2. Within each language, order the candidates of each agent by the SHA-256 of
   the text `357:<id>`, ascending. Walk the agents in this fixed order, one
   candidate per turn: `Claude_Code`, `OpenAI_Codex`, `Cursor`, `Copilot`,
   `Devin`, `Google_Jules`. An agent with no candidate left loses its turn.
   The arm is balanced by agent, not weighted by AIDev's counts, because klin
   serves hosts and not the dataset's population.
3. A candidate is eligible when all of these hold on GitHub on the day of the
   run:
   - its repository is not one that an earlier pick of the arm took.
   - the repository is readable and its size is 150,000 KB or less, the #343
     limit.
   - `git fetch origin pull/<number>/head` gets the head that the pull
     request API names, and that head and the API's base have a merge base.
   - the change touches at least one file with an extension of the
     repository's language: `.rs`; `.ts`, `.tsx`, `.mts` or `.cts`; `.py` or
     `.pyi`.
   - the change touches 100 files or fewer.
   - the head holds no `klin.json` at the root, because the replay writes its
     own.
4. Take the first 10 eligible pull requests of each language. Record each
   skipped candidate with the rule it failed.

### Human arm

For each pull request of the agent arm, the same repository gives one human
pull request:

1. The candidates are the closed pull requests that the GitHub search
   `repo:<name> is:pr is:closed created:<from>..<to>` returns, where the
   window is 60 days before and after the agent pull request's `created_at`.
2. A candidate is human when none of these holds:
   - the AIDev `pull_request` table holds its id.
   - its author's type is `Bot`, or its login ends with `[bot]`, or is
     `Copilot`.
   - its head branch starts with `codex/`, `cursor/`, `copilot/`, `devin/`,
     `claude/`, `jules/` or `jules-`.
   - its body or a commit message holds, case-insensitively, `claude code`,
     `co-authored-by: claude`, `codex`, `cursor agent`, `cursoragent`,
     `devin`, `jules`, `copilot`, `generated with` or `🤖`.
3. A human candidate is eligible under the rules of the agent arm's step 3,
   leaving out the first rule.
4. Take the eligible human candidate whose `created_at` is closest to the
   agent pull request's. A tie goes to the lower number. A repository with no
   eligible human candidate gives none, and the note reports that.

These rules cannot see an agent that a person ran without a trace in the
branch, the body or the commits. So the human arm is "not attributed to an
agent", and the note says so where it compares.

### Change

The base is the merge base of the pull request's base commit and its head, as
the pull request API names them. The head is the pull request's head commit.
The change is the three-dot diff, the diff that GitHub shows. Commits that a
person pushed to an agent's branch stay in the change, and the note reports
how many agent pull requests hold a commit by another author.

### Frozen families and detectors

| Family | Source ticket | Detector |
| --- | --- | --- |
| Each gate of klin `{}`: complexity, escapes, stubs, inventory (test deletion and skip), dead-symbols and reachability, lockfile, public-api, doc-size, doc-citations, and any other gate the run reports | shipped | `klin gate --json`, the #343 protocol |
| `constant-return`, `ellipsis-body`, `placeholder-wording`, `empty-handler`, `default-handler`, `log-handler`, `broad-handler`, `mock-name`, `throw-body` | #362 | `shapes new`, the #362 prototype as merged |
| undeclared import (candidate 1) and new dependency (candidate 3) | #364 | `deps.py new` and `deps.py added`, the #364 prototype as merged |
| `assertion-removed`, `assertion-rewritten` | #353 | `tests.py`, this pilot, below |
| `test-integrity`, `reuse` (#48, #355), `removable` (#356), `unfinished`, `error-masking`, `dependency`, `complexity`, `architecture`, `public-contract`, `cosmetic` | #353, #355, #356, #361 to #364, #357 amendment | review-comment codes, below |

Architecture layer rules need configuration, so `{}` measures only what its
gates measure. The #363 analyzer recipes have no result yet, so no analyzer
runs. The cosmetic families of the #357 amendment (restating comments,
boilerplate docstrings, generic naming) have no detector. They enter only as
the review-comment code `cosmetic`.

### klin

1. The binary is `target/release/klin` that `benchmark/build-klin` built from
   `59689e58` (`main` on 2026-10-02), version 0.4.1, SHA-256
   `e5deef55b370bf4e41a8cb59bc075d71f9206603ec94713e8b987ea67ffa03cb`.
2. For each change, in a full clone with its remote removed, the #343
   protocol: every local branch is deleted, `main` is set to the base, a
   branch `change` is checked out at the head, the tree is reset and cleaned,
   `.git/klin` is removed, an untracked `klin.json` holding `{}` is written,
   and `klin gate --json` runs with `GITHUB_BASE_REF`, `GITHUB_EVENT_PATH` and
   every `KLIN_*` variable removed, with a limit of 600 seconds.
3. Each finding whose outcome is `new` or `worsened` is a site. A finding with
   another outcome, such as `unresolved` or `unparsed`, is a measurement hole.
   The note counts holes and gives them no label.

### The #362 and #364 prototypes

`shapes new BEFORE AFTER FILE...` runs over `git archive` exports of the base
and the head, with the files that `git diff --name-only --diff-filter=AMR`
names with a `.rs`, `.py`, `.pyi`, `.ts`, `.mts`, `.cts` or `.tsx` extension,
as in the #362 replay. A site with an exclusion tag is not a finding. Its tags
are counted.

`deps.py new BEFORE AFTER` and `deps.py added BEFORE AFTER` run over the same
exports, as in the #364 replay. A new-dependency row states a fact and gets no
label.

### Test weakening (`tests.py`)

A test file is a file that the `test` rule of #362 covers by its path. A Rust
file that the path rule does not cover is a test file from the first line of
the base that holds `#[cfg(test)]`, because Rust unit tests live inline. For
each test file that the change modifies and the head still holds:

1. An **assertion line** is a line that holds one of these tokens:
   - Rust: `assert!`, `assert_eq!`, `assert_ne!`, `assert_matches!`,
     `debug_assert`, `.expect_err(`, `#[should_panic`.
   - TypeScript: `expect(`, `assert(`, `assert.`, `.toBe`, `.toEqual`,
     `.toMatch`, `.toThrow`, `.rejects`, `.resolves`.
   - Python: `assert `, `self.assert`, `pytest.raises`, `pytest.warns`,
     `.assert_called`, `.assert_awaited`.
2. The diff is `git diff -U0 --no-renames base head -- FILE`. In each hunk:
   - `assertion-removed`: the hunk removes assertion lines and adds none. One
     site per removed assertion line.
   - `assertion-rewritten`: the hunk removes assertion lines and adds
     assertion lines. One site per removed assertion line.

The predicates are syntactic. A site says "this assertion left this test
file", not "the test got weaker". The label decides the second claim.

### Review comments

1. For every pull request of both arms, read through the GitHub API its issue
   comments, its review bodies and its review line comments.
2. A comment enters when its author is not the pull request's author, its
   author's type is not `Bot`, its login does not end with `[bot]` and is not
   `Copilot`, and its body is not empty.
3. Each comment gets one or more of these codes:

| Code | The comment |
| --- | --- |
| `test-integrity` | asks to add, restore or strengthen a test or an assertion, or says a test got weaker, skipped or deleted. |
| `reuse` | asks to use code, a helper or a library that the repository already has, or says the change duplicates existing code. |
| `removable` | asks to remove code, files or edits that the task does not need: unused code, unrelated edits, debug leftovers, an unneeded abstraction, option or wrapper. |
| `unfinished` | says code is a stub, a placeholder, a mock or incomplete. |
| `error-masking` | says an error is swallowed, ignored, logged only or replaced by a default. |
| `dependency` | is about adding, removing, pinning or locking a dependency. |
| `complexity` | asks to simplify the structure of a function: its length, nesting or branches. |
| `architecture` | is about where code lives, a layer, or the direction of a dependency between modules. |
| `public-contract` | is about a change to a public API, CLI, configuration or schema. |
| `cosmetic` | asks to remove a comment or docstring that restates the code, or to rename a generic name. |
| `correctness` | says behavior is wrong, or asks a question that names a bug. |
| `other-change` | asks for any other change. |
| `none` | asks for no change: approval, thanks, status, an answered question. |

A comment that asks the agent to act, such as `@codex fix the test`, is coded
by what it asks.

### Labels of detector findings

Each finding of the agent and human arms gets one label of #357:

- `valid-regression`: the claim of the detector holds, the site is new in the
  change, and a reviewer would require a repair before acceptance.
- `valid-review`: the claim holds, and a person must judge if it is
  acceptable, such as an intended public API change or a long function that
  the task plausibly needs.
- `undesired`: the claim does not hold, or an intervention would be
  inappropriate.
- `unresolved`: the change and the pull request do not hold enough to judge.

An `undesired` label names the hard-negative class of #357 it falls in, where
one applies: compatibility duplication, external-boundary adapter,
appropriate complexity, platform-specific skip, correct candidate-authored
test, broad legitimate radius, intended public API, coexisting migration,
generated or framework code, dynamic registration.

Where a family has more than 40 findings in one arm, a sample of 40 is
labeled: every k-th finding in the order of the worksheet, with k the
smallest integer that gives 40 or fewer. The count of every finding is still
reported.

The labeler for findings and comments is the agent that wrote this note
(Claude Opus 5.5). It reads the head code around the site, the pull
request's title and body, and the review comments. It labels the agent arm
and the human arm in one worksheet that names no arm. No person labels a
row, as in #343 and #362, and the results say so. The labeler is not blind
in a strict sense: the pull request body and branch often name the agent.

### Measures

For each family, per arm:

- **eligible changes**: changes that touch a file the detector reads. For a
  klin gate, every change. For `shapes`, changes with a file of its
  extensions. For `deps.py`, changes with a `.py`, `.ts` or `.tsx` file. For
  `tests.py`, changes that modify a test file. For review codes, pull
  requests with at least one entered comment.
- **affected changes**: eligible changes with at least one finding (or coded
  comment).
- **sites**, and the four label counts, never merged into one number.
- **valid changes**: changes with at least one `valid-regression` or
  `valid-review` finding.
- **signals per 100 changes**: findings per 100 eligible changes.
- for klin, the wall time per change, and the count of measurement holes.

### Pilot outcome per family

With 30 changes in the agent arm:

- `frequent`: 3 or more agent changes are valid changes, or 3 or more agent
  pull requests hold a comment with the family's code.
- `seen`: 1 or 2.
- `not seen`: 0. Zero of 30 still allows a true rate of up to about 10% (the
  rule of three, at 95% confidence).

The human arm gives context: the same rate on human work of the same
repositories. With 30 changes per arm, the note reports no significance test.

A family whose findings are mostly `undesired` in both arms is noise for the
current detector, whatever its outcome.

### What the outcome decides

These rules only order research. They decide no product disposition:

1. A family that is `frequent` or `seen` stays in the full #357 study, with
   its detector.
2. A family that is `not seen` in both detector and review comments gets a
   lower place in the full study. The note does not drop it, because the
   pilot is small.
3. #356 keeps its full experiment before the full #357 study only when the
   `removable` code is `frequent`. Otherwise the note recommends that #356
   wait for the full #357 study.
4. #353 and #355 get the same rule, with `test-integrity` plus the two
   `tests.py` families for #353, and `reuse` for #355.

### Scripts

`docs/phenotype-pilot-2026-10-02/` holds the procedure, committed with these
rules:

- `choose.py AIDEV_DIR CLONES` applies the arm rules and writes
  `selection.json`, with every skipped candidate and its rule.
- `replay.sh CLONES`, with `KLIN_BIN` set, runs the four detectors on every
  change and writes `runs/` (one klin record per change), `shapes.tsv`,
  `deps.tsv`, `added.tsv` and `tests.tsv`.
- `tests.py CLONE BASE HEAD` is the test-weakening detector.
- `comments.py` writes `comments.json`, the entered review comments.

## What was measured

- **Rules:** committed in `a3ccf120` before any detector ran on the sample.
- **Selection:** run on 2026-10-02, frozen in `selection.json` in `1340aeb2`.
- **klin:** the binary that the rules name, 0.4.1 from `59689e58`. Its
  provenance is in `klin.provenance.json`.
- **Prototypes:** `shapes` and `deps.py` as merged with #362 and #364.

### Changes to the procedure after the rules commit

None of these changes read a result of the sample before it was made:

1. `tests.py --modifies-test` and the file `changed.tsv` give the eligible
   denominator of the `tests.py` and `deps.py` families. The rules name those
   denominators, and the first commit had no code for them.
2. `worksheet.py`, `summary.py` and `packet.py` merge the detector output,
   compute the measures and print the labeling packet.
3. The first replay ran klin in the klin repository, not in the clones,
   because `replay.sh` did not change directory. Its 9 records were deleted
   before any label. The fixed script ran all 58 changes again.
4. A review after the results found two gaps in `choose.py` as it ran. It
   read only the first 100 commits of a candidate, and it read an API error
   as "no result". A check on 2026-10-02 found no effect on this sample: no
   human pull request has more than 100 commits or an agent word in any
   commit, and the 4 repositories skipped as not readable return 404. A
   search that failed in the middle of a human-arm walk cannot be ruled out
   after the fact. `choose.py` now reads every commit and stops on any API
   error other than 404, so a new run cannot repeat either gap.

### A labeling rule set during labeling

The `complexity` label needs a line between `valid-review` and `undesired`
that the rules did not draw. After about 40 labels, the labeler set this
rule and labeled every `complexity` row again with it:

- `valid-review`: a new function with `cc` 8 or more, or a new function of
  80 lines or more of logic.
- `undesired`: any other function, a React component or hook whose length is
  JSX or callbacks, a match table over an enum, and a long function that the
  change grew by a few lines.

A reader who disagrees can move the line and recount from `labels.tsv`.

## Sample

| | Agent arm | Human arm |
| --- | --- | --- |
| Changes | 30 (10 Rust, 10 TypeScript, 10 Python) | 28 |
| Repositories | 30 | 28 of the same 30 |
| Merged | 20 | 20 |
| Agents | Claude Code 9, Codex 8, Cursor 4, Jules 4, Copilot 3, Devin 2 | - |
| Files changed | 231 | 264 |
| klin runs that ended in a report | 28 | 27 |

- Two Python agent changes have no human match. In
  `MontrealAI/AGI-Alpha-Agent-v0`, 999 of the 1,000 searched pull requests
  failed the human rules. In `conchoecia/odp`, 16 of 17 failed. Both
  repositories take almost all their changes from agents.
- 46 agent candidates were skipped: 28 for the size limit, 13 for no file of
  the language, 4 for a repository that is gone, and 1 for a repository
  already taken. The size limit leaves out large repositories, so the sample
  leans to small and middle projects.
- 4 agent pull requests hold commits by more than one author. Their changes
  stay in the sample, as the rules say.

## Headline results

1. **One agent pull request holds every `valid-regression`.** All 29
   `valid-regression` labels are in `anthropics/claude-code#8345`, a 69-file
   speculative "autonomous web evolution" pull request by Claude Code. The
   next agent change by valid labels is `nyx-space/anise#469`, a Jules change
   with 12 `valid-review` sites (placeholders and TODO markers). Both pull
   requests were closed without a merge. The human arm has no
   `valid-regression`.
2. **Placeholder wording and mock names separate the arms.** `shapes` found
   26 `placeholder-wording` sites and 14 `mock-name` sites in agent changes,
   against 1 and 0 in human changes. In the agent arm, 29 of the 40 are
   `valid-regression` and the rest are `valid-review`. These are the
   rewordable candidates that #362 put at REVIEW at best. In this pilot they
   are the only family whose valid findings appear in the agent arm and not
   in the human arm.
3. **klin `{}` fails most changes in both arms, mostly for `complexity`.**
   21 of 30 agent changes and 16 of 28 human changes exit 1. `complexity`
   fails 20 agent and 14 human changes. Of its 68 labeled sites, 15 are
   `valid-review` and 53 `undesired`. No `complexity` site is
   `valid-regression`.
4. **A `public-api` false alarm makes 2,086 of the 2,106 `public-api`
   findings.** Three changes report hundreds of public items as removed:
   `nyx-space/anise#465` and `#469` (911 each) and `mediar-ai/terminator#218`
   (264). The items are still in the head, and `anise#465` changes only a
   blank line in that crate. Each repository has two Rust crates with the
   same `[lib] name` (`anise` and `anise-py`; `terminator` and
   `bindings/python`). The inference is that klin matches the surface by that
   name and picks the wrong crate. This note did not read klin's code to
   confirm it.
5. **klin failed on 3 of 58 changes.** It overflowed its stack on both
   `obi1kenobi/cargo-semver-checks` changes, also with a 64 MB main stack. A
   SIGKILL stopped it after 451 s on `MontrealAI/AGI-Alpha-Agent-v0#3073`,
   with no output. `anise#469` also exits 2: `complexity`, `dead-symbols` and
   `public-api` report a file they cannot parse.
6. **No test weakening was found.** `tests.py` found 8 sites in 18 changes
   that modify a test file. All 8 are legitimate: formatting, a changed
   response type, a count that follows an added item, an assertion that got
   stronger.
7. **Reviewers asked for reuse only on agent pull requests.** 3 of the 7
   agent pull requests with a review comment hold a `reuse` comment. None of
   the 12 human pull requests with a comment does. Most pull requests have no
   review comment at all.

## Results per family

`Eligible` is the denominator of the rules. `Valid changes` counts changes
with a labeled `valid-*` site, so for a sampled family it is a lower bound.
`VR`, `VV`, `U` and `?` are `valid-regression`, `valid-review`, `undesired`
and `unresolved`.

| Family | Arm | Eligible | Affected | Sites | Labeled VR / VV / U / ? | Valid changes | Pilot outcome |
| --- | --- | --- | --- | --- | --- | --- | --- |
| klin `complexity` | agent | 30 | 20 | 201 | 0 / 9 / 25 / 0 | 1 | seen |
| | human | 28 | 14 | 67 | 0 / 6 / 28 / 0 | 4 | |
| klin `escapes` | agent | 30 | 4 | 124 | 0 / 27 / 4 / 0 | 2 | seen |
| | human | 28 | 7 | 23 | 0 / 6 / 17 / 0 | 2 | |
| klin `public-api` | agent | 30 | 3 | 1,176 | 0 / 0 / 40 / 0 | 0 | not seen |
| | human | 28 | 3 | 930 | 0 / 1 / 38 / 0 | 1 | |
| klin `stubs` | agent | 30 | 1 | 2 | 0 / 2 / 0 / 0 | 1 | seen |
| | human | 28 | 0 | 0 | - | 0 | |
| klin `doc-size` | agent | 30 | 1 | 2 | 0 / 0 / 2 / 0 | 0 | not seen |
| | human | 28 | 1 | 2 | 0 / 0 / 2 / 0 | 0 | |
| `placeholder-wording` | agent | 30 | 2 | 26 | 18 / 8 / 0 / 0 | 2 | seen |
| | human | 28 | 1 | 1 | 0 / 0 / 1 / 0 | 0 | |
| `mock-name` | agent | 30 | 2 | 14 | 11 / 3 / 0 / 0 | 2 | seen |
| | human | 28 | 0 | 0 | - | 0 | |
| `broad-handler` | agent | 30 | 3 | 38 | 0 / 1 / 37 / 0 | 1 | seen |
| | human | 28 | 4 | 15 | 0 / 4 / 11 / 0 | 2 | |
| `log-handler` | agent | 30 | 2 | 11 | 0 / 2 / 9 / 0 | 1 | seen |
| | human | 28 | 1 | 1 | 0 / 0 / 1 / 0 | 0 | |
| `default-handler` | agent | 30 | 0 | 0 | - | 0 | not seen |
| | human | 28 | 2 | 8 | 0 / 0 / 8 / 0 | 0 | |
| `empty-handler` | agent | 30 | 1 | 1 | 0 / 0 / 1 / 0 | 0 | not seen |
| | human | 28 | 2 | 2 | 0 / 1 / 1 / 0 | 1 | |
| `constant-return`, `ellipsis-body`, `throw-body` | both | 58 | 0 | 0 | - | 0 | not seen |
| `deps` undeclared import | agent | 21 | 0 | 0 | - | 0 | not seen |
| | human | 19 | 1 | 1 | 0 / 1 / 0 / 0 | 1 | |
| `assertion-removed` | agent | 8 | 0 | 0 | - | 0 | not seen |
| | human | 10 | 0 | 0 | - | 0 | |
| `assertion-rewritten` | agent | 8 | 1 | 3 | 0 / 0 / 3 / 0 | 0 | not seen |
| | human | 10 | 3 | 5 | 0 / 0 / 5 / 0 | 0 | |

`complexity`, `escapes` and `public-api` were sampled by the rule of every
k-th row: 34 agent and 34 human `complexity` rows, 31 agent `escapes` rows,
and 40 agent and 39 human `public-api` rows. Every other family was labeled
whole. The
labels total 333: 29 `valid-regression`, 71 `valid-review`, 233 `undesired`
and no `unresolved`. The `deps.py` new-dependency fact counted 9 added
dependencies in all.

### Concentration in one pull request

`anthropics/claude-code#8345` holds 159 of the 201 agent `complexity`
sites, 105 of the 124 agent `escapes` sites, 34 of the 38 agent
`broad-handler` sites, 18 of the 26 `placeholder-wording` sites and 12 of
the 14 `mock-name` sites. Without it, the agent arm has 42 `complexity`
sites against 67 human ones, 19 `escapes` sites against 23, and 4
`broad-handler` sites against 15. Only `placeholder-wording`, `mock-name` and
`stubs` keep more agent sites, all from `anise#469`. So the agent arm's
higher site counts come from one outlier, not from a spread across agent
work.

### Hard negatives in the population

The `undesired` labels hold these classes of #357: 53 `appropriate
complexity`, 6 `external-boundary adapter` and 0 of the others. One
`valid-review` site is an intended public API change (`Handler::consume` in
`sway#7322`). The pilot planted no hard negative. The #361, #362 and #364
corpora hold the planted controls.

### Review comments

48 comments entered: 7 agent pull requests and 12 human pull requests have
at least one. 24 comments ask for no change.

| Code | Agent PRs (of 7) | Agent comments | Human PRs (of 12) | Human comments | Pilot outcome |
| --- | --- | --- | --- | --- | --- |
| `reuse` | 3 | 4 | 0 | 0 | frequent |
| `removable` | 2 | 4 | 1 | 1 | seen |
| `test-integrity` | 1 | 3 | 1 | 2 | seen |
| `architecture` | 2 | 2 | 0 | 0 | seen |
| `correctness` | 1 | 1 | 1 | 1 | seen |
| `dependency` | 0 | 0 | 1 | 1 | not seen |
| `other-change` | 3 | 5 | 4 | 4 | - |
| `unfinished`, `error-masking`, `complexity`, `public-contract`, `cosmetic` | 0 | 0 | 0 | 0 | not seen |

The four `reuse` comments, on three pull requests, name an existing operation type
(`UnaryInputNumericOperation`, `GlareDB#3633`), an existing settings hook and
settings store (`karakeep#1723`), and helpers to share across files
(`dify-official-plugins#1422`). No reviewer comment asks to remove a
placeholder, a mock or a swallowed error, also on the two pull requests that
hold them. `claude-code#8345` was closed with no comment, and `anise#469` with
one: "Jules isn't yet a replacement for a human engineer."

## Pilot outcome and what it decides

| Family | Pilot outcome | Note |
| --- | --- | --- |
| `reuse` (#48, #355) | frequent | only from review comments; no detector ran |
| `placeholder-wording`, `mock-name` (#362) | seen | 2 agent changes, 0 human; both closed without merge |
| klin `stubs` | seen | 1 agent change |
| klin `escapes` | seen | valid sites: `any` in new TypeScript, new `unwrap` in compiler code, a skipped test, a bare `except`; 2 agent and 2 human changes |
| klin `complexity` | seen | most sites `undesired`; fails 20 of 30 agent changes |
| `broad-handler`, `log-handler` (#362) | seen | mostly `undesired` in both arms: noise for the current detector |
| `removable` (#356) | seen | 2 agent PRs, 1 human |
| `test-integrity` (#353) | seen | 1 agent PR by comment; `tests.py` found no weakening |
| `architecture` | seen | 2 agent PRs by comment |
| klin `public-api` | not seen | the 40 labeled agent sites are the same-name false alarm; the one other agent site, a new `Reqwest` error variant in `glues#150`, was not in the sample |
| `default-handler`, `empty-handler`, `constant-return`, `ellipsis-body`, `throw-body` (#362) | not seen | |
| `deps` undeclared import (#364) | not seen | 1 human site |
| klin `doc-size` | not seen | 4 sites, all `undesired` |
| cosmetic families | not seen | no detector; no reviewer comment |

By the rules of this note:

1. **#356 waits for the full #357 study.** `removable` is `seen`, not
   `frequent`.
2. **#353 waits for the full #357 study.** `test-integrity` is `seen` by one
   agent pull request, and `tests.py` found no weakening in 8 agent changes
   that modify a test file.
3. **#355 keeps its experiment before the full study.** `reuse` is
   `frequent`: 3 of 7 commented agent pull requests, against 0 of 12 human
   ones. The evidence is reviewer comments only, on 7 pull requests. That is
   a small base, and no detector confirms it.
4. Every family that is `seen` or `frequent` stays in the full study. The
   `not seen` families get a lower place. Zero of 30 still allows a true rate
   of up to about 10%.

## Findings for klin today

These are not phenotype results. They are defects or noise of the shipped
binary that the pilot met on real repositories. The note files no ticket.
A person decides.

1. **`public-api` with two crates of one `[lib] name`.** Three changes in
   two repositories, and 2,086 false sites. A reproduction:
   check out `nyx-space/anise` at the base of #465 and run `klin gate` with
   `{}` on the change, which edits one blank line in `anise/src/`.
2. **Stack overflow on `obi1kenobi/cargo-semver-checks`.** Both changes
   abort with exit 134 before any report. A 64 MB main stack does not help,
   so the recursion may have no bound. This note did not find the file.
3. **A run killed after 451 s** on `MontrealAI/AGI-Alpha-Agent-v0`, a
   repository of 1,925 tracked files. The cause is not known. Memory is one
   possible cause.
4. **`doc-size` on ordinary agent files.** A 381-word `CLAUDE.md` and a
   392-word `AGENTS.md` fail at a ceiling of 50 words. #361 and #435 already
   hold this.
5. **`complexity` noise.** 53 of 68 labeled sites in both arms are
   `undesired`. Most are React components and hooks with `cc` 1 to 5, match
   tables over enums, and long existing functions that a change grew by a
   parameter. #389 and #411 hold the floor decisions.

## Comparison with #343

#343 measured klin `{}` on default-branch commits. This pilot measures pull
requests, three-dot, with an agent arm and a human arm. The two use other
repositories, and this note copies no #343 number. Both find `complexity`
the largest source of `{}` failures. This pilot adds `public-api` false
alarms from same-name crates, which #343's ten repositories did not hold.

## Comparison with the literature

The pilot read no paper beyond those that the source tickets cite, and it
used no paper rate as a prior. Within those limits:

- The AIDev dataset card reports that 80.5% of its agent pull requests
  merged. Here 20 of 30 agent pull requests merged, and 20 of 28 human
  ones. The
  two pull requests with the most valid findings were not merged. So in this
  sample, reviewers already rejected the worst agent work. klin could tell
  the agent earlier, but the pilot cannot show that it would change the
  final change.
- The #355 motivation (non-reuse in generated code) matches the review
  comments, with the small base stated above.
- The #353 motivation (agents weaken tests) is not reproduced: no
  weakening in 8 agent changes that modify a test file. With 8 changes the
  pilot cannot reject it either.
- Population, language, harness and task all differ from those papers: the
  agents ran in the cloud services of 2025, on small and middle open-source
  projects, on tasks that the owners chose.

## Limitations

- 30 agent changes and 28 human changes. Every rate here is an
  observation, not an estimate with a usable interval.
- The labels are agent-drafted, as in #343 and #362. No person and no second
  agent labeled a row. The labeler saw the repository and title, which often
  name the agent, so it was not blind.
- The human arm is "not attributed to an agent". A person may have used an
  agent without a trace.
- The sample is balanced by agent, not weighted by AIDev's counts. Codex and
  Copilot make most AIDev pull requests.
- AIDev ends on 2025-10-24. The agents in it are a year older than those of
  today.
- One pull request dominates the agent site counts.
- Review comments exist on 7 of 30 agent and 12 of 28 human pull requests.
  Absence of a comment is not absence of a problem.
- The repair and appeasement parts of #357 need the controlled arm. This
  pilot has no controlled arm.

## How to reproduce

```sh
uv run --with duckdb python docs/phenotype-pilot-2026-10-02/choose.py AIDEV_DIR CLONES
KLIN_BIN=PATH docs/phenotype-pilot-2026-10-02/replay.sh CLONES
python3 docs/phenotype-pilot-2026-10-02/comments.py
python3 docs/phenotype-pilot-2026-10-02/worksheet.py
python3 docs/phenotype-pilot-2026-10-02/summary.py
```

`AIDEV_DIR` holds the two parquet files of the revision above. `choose.py`
needs the GitHub API, and its eligibility checks read GitHub as it is on the
day of the run. A later run can skip it and use `selection.json`. The corpus
keeps:

- `selection.json`: both arms and every skipped candidate.
- `runs/`: one klin record per change.
- `shapes.tsv`, `deps.tsv`, `added.tsv`, `tests.tsv`, `changed.tsv`: the
  prototype output.
- `worksheet.tsv` and `holes.tsv`: every finding, and the excluded sites
  and measurement holes.
- `labels.tsv`: one label per sampled finding, SHA-256
  `0f93f636f0cb993cac206d4004446cf6e7cb415da49944cfc5452126705888e1`.
- `comments.json` and `codes.tsv`: the entered comments and their codes,
  SHA-256 `b9920def3762be07d01c195a81d6aa67b39ccefe3436870e3e77949ea74f82a6`.
- `summary.tsv`: the output of `summary.py`.
