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
