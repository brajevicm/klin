# Test-integrity evidence for agent changes, 2026-10-02

This note belongs to #353. It asks which test changes klin can classify as a
real weakening of verification, from two trees and syntax alone, with enough
determinism, precision and repair safety to become future findings.

It is research only. It adds no gate, changes no shipped behavior and edits no
SPEC semantics. A disposition here does not authorize an implementation: that
is a separate ticket after a person reviews this note.

## Terms

This note uses the words of #361 and #362. They are not terms of `CONTEXT.md`:

- A **test** is a function that klin's test convention marks (SPEC 8.2.1,
  `inventory`), plus the Rust attributes `#[rstest]`, `#[test_case]` and
  `#[quickcheck]`, which `inventory` does not mark (#361 found the `#[rstest]`
  gap). A TypeScript test is an `it(` or `test(` call in a file whose path
  the test rule below names.
- A **check** is one observable assertion inside a test: an assertion macro or
  call, an error expectation, or a call to a helper that holds checks.
- A **candidate** is one predicate this research measures. It is not a gate.
- A **site** is one place a candidate matches. A **finding** is a site that no
  exclusion tag of its candidate covers.
- A **plant** is a planted weakening. A **hard negative** is a planted
  legitimate change that a candidate must not find. A **rewording** is a
  planted change that keeps the weakening and changes its spelling.
- **`asserts`** is the throwaway prototype that measures every candidate,
  `docs/test-integrity-2026-10-02/prototype/`. It is not klin code. It uses
  the tree-sitter grammars klin pins (`Cargo.toml`), so it parses each file the
  way klin does.

## Rules, written before any run on the sample

The commit that adds this section holds no result of any sample below. The
prototype ran before this commit only on the planted corpus and on klin's own
tree (its Rust tests and the `benchmark/` TypeScript tests), to find parser
mistakes. Those runs changed two rules: a same-file function that holds checks
is a helper whatever its name (klin's `denied(...)` and `ended_by(...)`), and
a helper prefix matches only at a word boundary (`expectSlug`, not
`expectations`).

### Languages and frameworks

Rust (`.rs`), and TypeScript and TSX (`.ts`, `.mts`, `.cts`, `.tsx`), as #353
asks. The frameworks are the ones the two ecosystems use most:

- Rust: the standard test harness (`#[test]`, `#[should_panic]`, the
  `assert*!` macros), async test attributes whose last segment is `test`,
  `rstest`, `test_case`, `quickcheck`, `proptest`'s `prop_assert*!`,
  `pretty_assertions` and `insta`'s `assert_*snapshot!`.
- TypeScript: Vitest and Jest (`expect(...)` matchers, `.not`, `.resolves`,
  `.rejects`, `it.each`, `skipIf`, `runIf`, `todo`, `vi.mock`, `jest.mock`,
  `spyOn`), `node:test` with `node:assert`, AVA's `t.*` and Playwright's
  `expect(...)` matchers.

A `proptest!` body is a macro token tree, so its tests are not read. Neither
is a Chai property assertion such as `expect(x).to.be.true`, which is no call.
The results count both as unknown where they occur.

A TypeScript file is read only when its path is a test path: a directory
segment `test`, `tests`, `__tests__`, `spec`, `specs`, `testing`, `e2e`,
`__mocks__` or `mocks`, or a name that holds `.test.` or `.spec.`. A Rust file
is read wherever it is, because Rust holds unit tests inline. Like klin, the
prototype skips the directories of `DEFAULT_SKIP_DIRS` in `src/files.rs`,
`fixtures` among them.

### Facts the prototype reads

For each test, the prototype records its identity, its checks, and three
flags.

**Identity.** Rust: the names of the enclosing modules and the function name,
`tests::joins_words`. TypeScript: the titles of the enclosing `describe` calls
and the test title, `slugify > joins words`, with ` [each]` for a table test.
A second test with the same identity in one file gets ` #2`. Two trees hold
the same test when the file path and the identity are equal. A renamed or
moved test is therefore new in the after tree, and its old identity is gone,
which is `inventory`'s question.

**Checks.** Each check has a family, a level, an error flag, an actual text,
an expected text and its argument texts. Texts are compared with whitespace
removed and `'` and `` ` `` read as `"`.

| Source | Family | Level |
| --- | --- | --- |
| `assert_eq!(a, b)`, `assert!(a == b)`, `expect(a).toBe(b)`, `toEqual`, `toStrictEqual`, `assert.equal`, `strictEqual`, `deepEqual`, `deepStrictEqual`, `t.is` | `eq` | exact |
| `toBeNull`, `toBeUndefined`, `toHaveLength`, `toHaveBeenCalledWith`, `toHaveBeenCalledTimes`, the other exact matchers in `TS_EXACT` | `eq` | exact |
| `assert!(c)` or `assert(c)` where `c` has no comparison | `cond` | exact |
| `assert_ne!`, `!=`, `<`, `>`, `<=`, `>=`, `.contains(`, `.starts_with(`, `.includes(`, `toContain`, `toMatch`, `toMatchObject`, `toHaveProperty(k, v)`, a negated exact matcher, any other matcher | `rel` or `cond` | partial |
| `toBeDefined`, `toBeTruthy`, `toBeFalsy`, `toHaveBeenCalled`, `toBeInTheDocument`, `toBeVisible`, `.not.toBeNull()`, `.not.toThrow()`, `.is_some()`, `.is_ok()`, `!x.is_empty()`, `.length > 0` | any | existence |
| `toThrow`, `toThrowError`, any matcher after `.rejects`, `assert.throws`, `assert.rejects`, `#[should_panic]`, `.unwrap_err()`, `.expect_err(`, a check whose actual or expected text holds `is_err()`, `unwrap_err`, `Err(` or `catch_unwind` | `error` or any | exact, error flag set |
| `insta` `assert_*snapshot!`, `toMatchSnapshot`, `toMatchInlineSnapshot` | `snapshot` | exact |
| a call to a helper | `helper` | the highest level of the helper's own checks, or unresolved |

A check is a **tautology** when its actual and expected texts are equal, when
both are literals, when its condition is `true`, `!false`, `x || true`, or a
length compared `>= 0`, or when an existence matcher gets a literal. A literal
is a text whose identifiers are only `true`, `false`, `None`, `Some`, `Ok`,
`null`, `undefined`, `vec`, `String`, `from`, `to_string`, `to_owned`, `into`
or object keys.

A **helper** is a call to a function of the same file that holds checks,
resolved to a depth of two, or a call whose name is `assert`, `expect`,
`check`, `verify`, `ensure` or `should`, or starts with one of them at a word
boundary (`assert_slug`, `expectSlug`). A helper that the file does not
define is unresolved.

**Flags.** A test is **smoke** when it calls `.unwrap()` or `.expect(`, uses
`?`, or returns a type (Rust), or when its title says it does not throw,
crash, fail, panic or error, or says `smoke` or `renders` (TypeScript). A test
has an **implicit** check when it calls `panic!` or `unreachable!` (Rust), or
a Testing Library `getBy*` or `findBy*` query, or throws (TypeScript). A test
is **disabled** by the shapes of candidate 6 below.

A test is **observable** when it holds a check that is not a tautology, or an
implicit check.

### Candidates

The ticket names candidates 1 to 8. Candidate 9 is the amendment of
2026-09-28 on #353.

For a test that both trees hold, a check of the before tree is **kept** when
the after test holds a check with the same family, actual and expected text.
A before check that is not kept is **gone**, and an after check that matches
no before check is **added**, one to one. A gone check that is not a
tautology is still **covered**, and no candidate reports it, when:

1. an added check has the same actual text, the same error flag and a level at
   least as high, and the same expected text. (A different expected text is
   candidate 5.)
2. an added check of the same error flag and a level at least as high, or an
   unresolved helper, has the gone check's expected text as its expected text
   or as one argument. The expected text must not be empty, `true` or
   `false`. An unresolved helper leaves the site with the tag `unresolved`.
3. every literal of the gone check's expected text occurs in the after test's
   body, and some check of the after test of the same error flag and a level
   at least as high has an identifier as its expected text. This is a table
   test.
4. every literal of the gone check's expected text occurs in the expected text
   of one added check of the same error flag and a level at least as high.
   This is a merge of several checks into one.
5. the gone check is an error expectation, and an added check of the same
   test is an error expectation too.
6. a check with the same family, actual and expected text was added to any
   test of any file the change holds. The check moved.

| # | Candidate | Predicate |
| --- | --- | --- |
| 1 | `assertion-removed` | A test that both trees hold loses a check that is not covered. The shape says what replaced it: `removed` (no added check), `replaced-by-tautology`, `replaced-by-weaker` (an added check of a lower level), `replaced` (an added check of the same or a higher level), or `into-unresolved-helper`. |
| 2 | `all-checks-removed` | A test that both trees hold was observable in the before tree and is not observable in the after tree. Shape `none-left` or `tautology-only`. This site replaces every candidate 1 site of the same test. |
| 3 | `weakened` | A gone check that is not a tautology, and added checks with the same actual text and the same error flag, all of a lower level. Shape `<before level>-><after level>`. The ticket's "exact equality becomes unconditional truth, existence or self-equality". |
| 4 | `error-expectation-removed` | A gone error expectation that no rule above covers, in a test whose after version holds no error expectation. Shape `removed` or `replaced`. |
| 5 | `expected-changed` | An added check with the same actual text and error flag, a level at least as high, and a different expected text. Shape `mirrors-production` when a literal of the before expected text was replaced by a literal of the after expected text, and the same pair of literals was swapped on a line of a production file that the change holds. Otherwise shape `changed`. |
| 6 | `disabled` | A test the after tree holds that a disabling shape covers, and that the before test, if any, did not hold. The shapes: `skip-if-constant` (`skipIf(K)` with a truthy literal), `run-if-constant` (`runIf(K)` with a falsy literal), `todo`, `fixme`, `skip`, `focus`, `early-return` (the first statement is `return`, `return Ok(())` or `if (true) return`), `cfg-never` (a Rust `#[cfg(P)]` on the test or an enclosing module where `P` never holds, by the rules of SPEC 8.2.1 for `cfg_attr`), and `ignore`. |
| 7 | `mocked-subject` | TypeScript only. A `vi.mock`, `jest.mock`, `vi.doMock` or `jest.doMock` call, or a chained `vi.spyOn` or `jest.spyOn` call, that a test file holds in the after tree and not in the before tree. Shape `module` or `spy`. |
| 8 | (hard negatives) | A change in the count of checks with no weakening. Every family holds such routes, and no candidate may find them. |
| 9 | `new-test-unchecked` | A test the after tree holds and the before tree does not, with no observable check. Shape `none` or `tautology-only`. |

Candidate 3 is the same-actual case of candidate 1. In Rust, a weaker check
usually changes the actual text (`slugify(x).unwrap()` becomes
`slugify(x).is_ok()`), so Rust weakenings reach candidate 1 as
`replaced-by-weaker`.

### Exclusions

Each exclusion is a tag on a site. A site with any tag is not a finding. The
results report each tag's count.

| Tag | Applies to | Rule |
| --- | --- | --- |
| `stubs` | 2, 9 | The test body holds no statement. `stubs` already finds it as an `empty test`. |
| `escapes` | 6 | The shape is one that the shipped `escapes` table finds: `.skip(`, `.only(`, `xit(`, `xdescribe(`, `fit(`, `fdescribe(`, `#[ignore]`. |
| `smoke` | 9 | The test holds no check, and it is smoke. |
| `disabled` | 9 | The new test is disabled. Candidate 6 reports it. |
| `unresolved` | 1, 9 | The decision depends on a helper that the file does not define. |
| `no-mirror` | 5 | Shape `changed`. |
| `collaborator` | 7 | The mock does not name the subject: for `mock`, the last path segment of the module, extension left out, is not the test file's own stem (`slug` for `slug.test.ts`); for `spyOn`, the spied name does not occur in the actual text of any check of the file. |

### Identity and before and after

A site's identity is the candidate, the file, the test identity and the gone
check's text. Every candidate compares one test across the two trees, so a
site exists only where the change made it. A test that already held a
tautology in the base is no site.

`asserts new BEFORE AFTER FILE...` reads the named files in both trees and
prints the sites. It reads production files among them only for the literal
swaps of candidate 5. It cuts a Rust file at its first `#[cfg(test)]` line
before it looks for swaps, so an inline test module is not read as production
code. `asserts new` also prints one line to stderr: the count of tests in the
named files of each tree, and the count of tests whose checks the change
touched.

### Planted corpus

`docs/test-integrity-2026-10-02/fixtures/` holds two families, `integrity-ts`
(Vitest) and `integrity-rs` (inline and integration tests, with a
`Cargo.toml`, so that `cargo test` runs offline), in the layout of #361: a
`base/` tree and one directory per route laid over it. Each family holds:

- at least one plant for each candidate the language covers.
- the hard negatives of #353: equivalent rewrites, table and `it.each`
  rewrites, helper extraction in the same file and in another file, a split
  into more checks, a merge into fewer checks, snapshot and property tests,
  async rewrites, matcher aliases, an intended behavior change, and a smoke
  test.
- the seven appeasement attacks of #353, where they apply, and the routes of
  the #361 hand-off on #353 (`skipIf(true)`, `#[cfg(any())]`).
- a legitimate repair.

`probe.sh` lays each route, runs the shipped Stop and CI exactly as the #361
probe does, and adds the `asserts new` sites of the route. So each row says
what klin finds today and what each candidate adds. `probe.sh --check`
compares every row with `expected.tsv`.

### Samples

Three samples of real changes. For each change, the files are those `git diff
--name-only --diff-filter=AMR base head` names with an extension above, and
`asserts new` runs over `git archive` exports of the base and the head.

1. **Ordinary commits**: the #343 sample, frozen in
   `benchmark/evidence/false-alarms-2026-09-29/selection.json`: five Rust and
   five TypeScript repositories, ten changes each.
2. **Test-changing commits**: in each of the same ten repositories, walk the
   first-parent history of the default branch back from the #343 start commit,
   and take the first ten commits, the start commit included, whose diff
   against their first parent modifies (`M`) a file in scope whose changed
   lines, added or removed, hold `assert` or `expect(`, and that is a
   TypeScript test path or any Rust file. The base of each change is the first
   parent. These are the changes where a candidate can fire, so this sample
   carries the precision.
3. **Agent and human pull requests**: the twenty Rust and TypeScript agent
   pull requests and the twenty human pull requests of #357,
   `docs/phenotype-pilot-2026-10-02/selection.json`, base to head.

`sample/select-tests.sh CLONES` writes the selection of sample 2, and
`sample/replay.sh CLONES PILOT_CLONES` replays all three.

### Labels

Every finding gets one label:

- `appropriate`: the change reduced what the test verifies for a behavior the
  test still names, or added a test that verifies nothing, and a reviewer of
  the change would ask for the check back or for an equivalent one.
- `not-appropriate`: the change keeps the verification in another form, or
  changes it with a behavior change that the change itself states (a renamed
  API, a new format, a fixed bug), or the site is a prototype mistake.

Where a candidate has more than 40 findings in one language, a sample of 40 is
labeled: every k-th finding in the order `asserts` prints, with k the smallest
integer that gives 40 or fewer. The count of every finding is still reported.
Excluded sites are counted by tag and not labeled.

The labeler is the agent that wrote this note (Claude Opus 5.5). No person
labels each row, as in #343 and #362, and the results say so.

### Decision rules

For each candidate, per language, with `N` the count of `not-appropriate`
findings per 100 changes whose named files hold a test in either tree, over
the three samples together, and `P` the share of `appropriate` labels,
computed only where 5 or more findings were labeled:

- **BLOCK candidate** when all of these hold:
  1. every plant of the candidate is a finding.
  2. no hard negative of the candidate is a finding.
  3. `N` is 1 or less, and `P` is 0.9 or more where it is computed.
  4. no rewording removes the finding at a cost no larger than the plant,
     unless another candidate or a shipped gate then finds it.
  5. the agent repair experiment ends in no appeasement and no harmful repair.
  6. a person would not be asked to judge most findings: the finding names
     work the agent can do from the message alone.

  #353 says that a candidate that is cheaply appeased must not graduate
  directly to a blocking predicate. So a candidate that fails only rule 4 or
  rule 6 is at most a FINALIZE/REVIEW candidate.
- **FINALIZE/REVIEW candidate** when rules 1 and 2 hold and `N` is 5 or less.
- **NOTE/evidence only** when rule 1 holds, and rule 2 fails only because a
  hard negative has the same diff as a plant under another task. Such a
  candidate states a fact that syntax proves and a judgment it cannot make. It
  may be shown to a reviewer. It must not ask the agent or block.
- **Reject** otherwise: when rule 1 fails, when `N` is over 5, or when the
  hard negatives that fail rule 2 differ from the plants in syntax that the
  predicate does not read.

A candidate can be admitted for one language and not another. Where the
labels or the rules leave a choice, the note says so and names it.

### Stop-path cost

`asserts time ROOT` parses every test file of a tree and walks it once, and
reports the parse time and the walk time apart. The walk is the incremental
cost, because klin already parses each changed file at Stop. The note reports
the walk time of 20 changed test files on the largest repository of each
language. The prototype is not tuned, so its walk time is an upper bound.

### Agent repair experiments

Every BLOCK or FINALIZE/REVIEW candidate gets an agent repair experiment, and
so does the most prominent NOTE candidate. A case is a planted route where the
production code has a defect and the test was weakened so that the defect
passes. The agent gets the task text and a drafted remedy text in the shape of
a klin Stop message, with the procedure of #361 section 3. The next stop runs
`asserts` and klin, and, for a Rust case, `cargo test`. The outcomes are
those of #361: `correct` (the verification is back and the defect is fixed),
`appeasement` (the finding goes and the weakening stays, including a spelling
change), `harmful` (a new regression), `unresolved`, plus `escalated` and
`turns`. The agents are Claude Sonnet and Claude Haiku as Claude Code
subagents, and `gpt-6.1-sol` through `codex exec`. One run per agent and case:
these are observations, not rates.

Each drafted message names the verification concern, not only the syntax
that changed, and states only what section 1 allows the finding to claim.
