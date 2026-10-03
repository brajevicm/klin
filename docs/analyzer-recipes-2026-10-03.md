# Analyzer recipes for slop families klin does not measure, 2026-10-03

This note belongs to #363, under #358. It asks whether klin should recommend
analyzer recipes for the slop families that mature analyzers already express,
and if so which families, in which phase, under which trust model and in what
form.

It is research only. It ships no recipe, preset, `init` behavior or default,
and it changes no SPEC semantics. A disposition here does not authorize an
implementation: that is a separate ticket after a person reviews this note.

The ticket names `76097d41` as its baseline. This work starts on `d9360450`
(`main` on 2026-10-03). Since the baseline, the `sarif` seam of SPEC 8.3 and
`src/sarif.rs` did not change in any way that this note measures. #452
replaced the public command names after the baseline: this note says Stop,
Finalize and CI for the three phases, and `klin gate` for the binary it runs,
because that is the binary that ships today.

## Terms

These words belong to this note. They are not terms of `CONTEXT.md`:

- A **family** is one slop family of the ticket: `injection`, `secrets`,
  `swallowed` (swallowed errors), `dead` (dead code and unused imports),
  `debug` (debug output) and `condition` (unnecessary conditions).
- An **entry** is one analyzer rule that states part of a family, such as
  Ruff `BLE001` for `swallowed`. `recipes/families.tsv` maps every entry to
  its family.
- A **recipe** is one tool, its pinned version, the entries it runs, its
  configuration and its invocation. The recipes of this note live in
  `docs/analyzer-recipes-2026-10-03/recipes/`.
- A **result** is one SARIF result. A **failure** is a result that klin's
  `sarif` gate fails: a result on a line the window changed (SPEC 8.3).
- A **plant** is a planted slop change. A **hard negative** is planted code
  that a family must not fail. A **rewording** is a planted change that keeps
  the slop and changes its spelling. A **suppression** is a planted change
  that keeps the slop and adds the tool's own opt-out comment.
- **Named recipe**, **user-owned SARIF** and **native detector** are the
  three integration models of the ticket.

## Rules, written before any run on the sample

The commit that adds this section holds no result of the ordinary-commit
sample. The fixture probe ran before this commit, to find mistakes in the
fixtures and the recipes. Its results are calibration, not admission
evidence, until the results section reports them.

### Languages and tools

TypeScript and Python. Rust is out of scope: its analyzer for these families
is Clippy, which needs a build, and #362 already delegated Clippy's
`let_underscore_must_use` to an analyzer (section 7 of that note).

| Tool | Version | Families | How the probe runs it |
| --- | --- | --- | --- |
| ESLint, with typescript-eslint 8.71.0 and eslint-plugin-sonarjs 4.2.2, TypeScript 6.0.3 | 10.12.0 | TypeScript: injection, swallowed, dead, debug, condition | `--no-config-lookup` with the generated config `recipes/eslint.config.mjs`, SARIF through `@microsoft/eslint-formatter-sarif` 3.1.0 |
| Ruff | 0.16.10 | Python: injection, secrets, swallowed, dead, debug | `--isolated --no-cache`, the entries of `families.tsv` as `--select` |
| Semgrep | 1.179.0 | Both: injection (SQL and shell text built from a value), secrets | the local rules `recipes/semgrep.yml`, `--metrics=off --disable-version-check` |
| Gitleaks | 8.30.1 | Both: secrets | its built-in rule set, `gitleaks dir` |

Every recipe runs over the whole working tree, through the shipped `sarif`
`run` seam, with the command `sh recipes/run.sh TOOL TOOLS RECIPES`. The
configuration is the recipe's own and not the project's. Section 11 measures
what changes under the project's own configuration.

Python has no `condition` entry. No Ruff rule states it, and a type checker
(mypy, Pyright) needs the project's installed dependencies. The note reports
`condition` for Python as not measured.

### Planted corpus

`fixtures/recipes-ts` and `fixtures/recipes-py` hold one small service each,
`base/`, and one directory per route laid over it. `generate.py` writes both
from its tables, and `fixtures/*/routes.tsv` names each route's family and
class. Each family holds plants, hard negatives, rewordings, suppressions and
one legitimate change.

The base trees hold results that the recipes report: a `print` in a CLI and a
`console.log` in a script, and two `rows[0]` guards. A change that does not
touch them must not fail on them.

`probe.sh` lays each route twice, once with `klin.json` as `{}` and once with
the recipe entries as a `sarif` section. For each it runs the first Stop and
a CI run of `klin gate --strict --json` on a clone, exactly as the #361 and
#362 probes do. A route's row names the exit codes and every finding of the
CI run, as `gate:rule`.

### Ordinary commits

The TypeScript sample is the #343 sample, frozen in
`benchmark/evidence/false-alarms-2026-09-29/selection.json`: five
repositories, ten changes each. The Python sample is the #362 sample, frozen in
`docs/unfinished-code-2026-10-02/sample/selection-python.json`: five
repositories, ten changes each. The Rust repositories of #343 are out of
scope.

For each TypeScript repository the project's dependencies are installed once
at the start commit, with the lockfile and with install scripts disabled, so
the type-aware entries see the project's types. An install that fails is
reported, and that repository's type-aware entries are reported as measured
without dependencies.

Each change runs the three recipes of its language over the head tree, the
way CI runs `klin gate` for a pull request: `GITHUB_BASE_REF` names the change's
base commit, and `klin gate --strict --json --gate NAME` judges only the recipe
gates. Each failure is one row.

ESLint, Ruff and Semgrep run over the files the change added, modified or
renamed, and Gitleaks over the whole tree. Every entry of `families.tsv`
judges one file at a time, so a result on a changed line is the same under
both scopes. The replay checks that claim on two changes per language with a
whole-tree run, and reports any difference.

### Labels

Every failure gets one label:

- `appropriate`: a reviewer of that change would ask for a change at that
  line for the reason the rule gives.
- `not-appropriate`: the line is intended, or the rule's reason does not hold
  there, such as a `print` in a command-line tool, a necessary guard that the
  type system cannot see, or a test credential.

Where one family has more than 40 failures in one language, a sample of 40 is
labeled: every k-th failure in the order the replay prints, with k the
smallest integer that gives 40 or fewer. The count of every failure is still
reported. Each failure also gets two tags, which do not enter the label: `test`
(the file is a test, by the path rule of #362) and `project-off` (the
project's own analyzer configuration turns that rule off or ignores that
file).

The labeler is the agent that wrote this note (Claude Opus 5.5). No person
labels each row, as in #343 and #362, and the results say so.

### Decision rules

Per family, per language, with `N` the count of `not-appropriate` failures per
100 changes and `P` the share of `appropriate` labels where 5 or more failures
are labeled:

- **Admit as failing** when all of these hold:
  1. the family's entries fail at least two thirds of its plants in CI.
  2. no hard negative fails on an entry of the family.
  3. `N` is 1 or less, and `P` is 0.9 or more where it is computed.
  4. the agent repair experiment ends in no appeasement.
- **Admit as review** when rule 1 holds, at most one hard negative fails, and
  `N` is 5 or less. Its results are reported and never fail a gate.
- **Reject** otherwise.

A rewording does not decide admission by itself, because klin owns no
analyzer's rule semantics. Every rewording that passes is reported, it enters
the public coverage statement, and an agent that takes one in the repair
experiment fails rule 4.

### Phase rules

Per admitted family and tool, from the cost measurements of section 6:

- **Stop** when the tool's median time over the 20 changed files of a Stop is
  1 second or less on the largest repository of its language, and the tool
  reads no installed project dependency. Otherwise it is not a Stop entry.
- **Finalize** when its median whole-tree time is 60 seconds or less on every
  repository of its language.
- **CI only** otherwise.

### Form rules

- **Adopt named recipes** for a tool when at least one of its families is
  admitted, and all of these hold for it:
  1. it gives the same results under a controlled invocation (a pinned
     executable, fixed arguments, an environment of `PATH` and `HOME` only,
     the network denied) as under the shell seam.
  2. it writes nothing into the audited tree.
  3. its report, or a documented guarantee, says which files it analyzed.
  4. its rule set can be pinned without the network.
- **Document recipes only** when a family is admitted and no tool meets the
  four conditions.
- **Do not recommend analyzers** when no family is admitted.

### Agent repair experiments

Every family that rule 1 to 3 admit as failing or as review gets an agent
repair experiment in each language it is admitted for. A planted route gets
a task text, and the agent gets the Stop message that klin printed for that
route, with the procedure of #361 section 3. The next Stop runs klin with the
same recipes. The outcomes are those of #361: `correct`, `appeasement` (the
failure goes and the slop stays, including a rewording or a suppression),
`harmful`, plus `escalated` and `turns`. Each case runs twice, once with the
message klin prints today and once with one remedy line per family added. The
agents are Claude Sonnet and Claude Haiku as Claude Code subagents. One run per
agent and case: these are observations, not rates.

### Measurements of the recipe contract

These run on the fixtures and the sample, and they decide no admission:

- **Cost**: each tool's time over the whole tree and over 20 changed files,
  median of 5 runs, on the largest repository of each language.
- **Absence and failure**: what the Stop and CI print when the tool is
  missing, exits non-zero with no report, writes a truncated report, runs
  past the limit, or reports a file it could not parse.
- **Observed scope**: what each report says about the files the tool read, for
  a file it could not parse, a file outside its selection and an ignored file.
- **Controlled invocation**: each tool under `env -i`, under a macOS sandbox
  profile that denies the network, and with a listing of the tree and of
  `HOME` before and after the run.
- **Version drift**: the results of each recipe at the start commit of every
  sample repository, under the pinned version and under one older version.
- **Suppressions**: which suppression forms `escapes` counts today, and which
  flags turn each tool's inline suppressions off.
- **Project configuration**: for each sample repository, whether its own
  analyzer configuration selects, turns off or ignores each entry.

## What was measured

- **Binary:** `klin` built with `cargo build --release` from `d9360450`.
- **Tools:** the versions of the rules table, installed in a scratch directory
  outside the repository. Section 12 lists their licenses and sizes.
- **Machine:** macOS on aarch64, Node 24.17.0, Python 3.13 in the tool
  environments. Every time below comes from this one machine.
- **Labels:** drafted by the agent that wrote this note. No person reviewed a
  row. The same caveat holds for #343, #353, #355, #362 and #364.

### Changes after the rules were registered

Each change below came after commit `b03ce673`, which registered the rules.

1. **A bug in two Semgrep rules of this note.** In `sql-template-ts` and
   `sql-format-py`, a `metavariable-regex` stood beside `pattern-either`, so
   it did not constrain the method name. The rules matched every method call
   with a template or an f-string, such as `process.stdout.write(...)`. The
   first replay counted 31 TypeScript and 2 Python failures from that bug.
   The fix moves the constraint into `patterns`. The probe ran again in full,
   and the replay ran again for Semgrep only (`PICK=semgrep`). Three probe
   rows changed: `recipes-ts/debug-reword-stdout`, `recipes-py/inj-plant-exec`
   and `recipes-py/debug-reword-stdout` lost a Semgrep failure. The replay
   lost all 33 injection failures. `sample/failures.tsv` holds the replay
   rows of ESLint, Ruff and Gitleaks and the Semgrep rows of the second run.
2. **`probe.sh stage` and `probe.sh finish`**, for the repair experiments, in
   the shape #361 and #362 used.
3. **`contract.sh`, `cost.sh` and `drift.sh`**, for the contract
   measurements that the rules name.

### Rules for the reruns after the PR review

The owner's adversarial review of PR #454
([comment](https://github.com/brajevicm/klin/pull/454#issuecomment-5971137435))
found five issues in the admission and placement result. This section was
committed before any of the reruns below.

1. **A Semgrep holdout.** The fixed Semgrep rules were corrected after they
   saw the ordinary-commit sample, so that sample cannot show their noise.
   The holdout is new changes from the same ten repositories: from the oldest
   `base` of each repository's ten sample changes, the ten earlier commits on
   the first-parent walk. Each holdout change is one commit against its first
   parent. `sample/holdout.sh` writes them to `sample/selection-holdout.json`.
   The replay runs the three Semgrep rules of `recipes/semgrep.yml` on them,
   unchanged since the fix, with `PICK=semgrep`. Every failure gets a label by
   the rules of the section "Labels". The admission of the Semgrep entries
   uses the holdout N, and the original sample N for Semgrep is exploratory.
   Until the holdout runs, the pairs that rest on Semgrep entries are
   exploratory.
2. **Cost runs that fail.** `cost.sh` records each run's exit status and
   whether it wrote a SARIF report with a `runs` array. A run without such a
   report is not a measurement, and the median uses only the runs with one.
   `cost.tsv` keeps the raw rows. A tool with fewer than three good runs of
   five has no median.
3. **Stop placement.** The phase rule of this note (1 second for 20 files)
   is looser than the Stop guidance of #358: "25–75 ms needs clear product
   value", a new external process on Stop is "exceptional", and the
   incremental time is measured on the controlled 1M / 20-changed workload.
   #358 governs. No admitted pair showed value on real changes, so no recipe
   is a Stop entry. A recipe that passes the other phase rules goes to
   Finalize at most. This note does not run the 1M workload.
4. **Semgrep is not a named recipe.** The rules of `recipes/semgrep.yml` are
   klin's text, so a named Semgrep recipe would make klin own detector
   semantics, against #363's boundary for a named recipe. The Semgrep entries
   stay a documented, user-owned example. If a person wants them in klin,
   they are a klin-owned detector with its own admission.
5. **The exact ESLint recipe.** `cost.sh` times a recipe of only `no-eval`,
   `@typescript-eslint/no-implied-eval` and `no-new-func`
   (`recipes/eslint-injection.config.mjs`), over the whole tree and over 20
   files, on every TypeScript repository of the sample. The phase rules
   apply to those times.

### How to reproduce

```bash
TOOLS=<tools> bash docs/analyzer-recipes-2026-10-03/probe.sh
```

```bash
TOOLS=<tools> bash docs/analyzer-recipes-2026-10-03/sample/replay.sh <clones> <out>
```

`<tools>` holds `js/` (an npm project with the ESLint packages and
`klin-recipe.config.mjs`, a copy of `recipes/eslint.config.mjs`), `py/.venv`
(Ruff and Semgrep) and `gitleaks`. `<clones>` holds one clone per sample
repository, named `owner__name`. The TypeScript clones need their
dependencies at the start commit: `npm ci --ignore-scripts`, or
`pnpm install --frozen-lockfile --ignore-scripts`. Tolaria needs pnpm 10,
because pnpm 11 refuses its lockfile.
`sample/label.py` writes `sample/labels.tsv` from `sample/failures.tsv`.

## Headline results

- **No family is admitted as failing.** Five family and language pairs pass
  the review rules: TypeScript `secrets`, and Python `injection`, `secrets`,
  `swallowed` and `dead`. Six pairs are rejected: TypeScript `injection`,
  `swallowed`, `dead`, `debug` and `condition`, and Python `debug`. Python
  `condition` is not measured.
- **The fixed Semgrep rules failed on a holdout.** After the PR review, the
  Semgrep rules that were fixed on the sample ran on 100 new changes. They
  failed three TypeScript lines, none appropriate: N is 6, so TypeScript
  `injection` is rejected (section 3).
- **The review pairs found nothing appropriate on real changes.** Their
  failures were 0 to 2 per language, every one `not-appropriate`. So the
  sample shows their noise is low. It does not show that they catch real
  slop. Only the planted corpus shows that.
- **The rejected pairs are noise on real code.** TypeScript `dead` has about
  110 `not-appropriate` failures per 100 changes, `condition` 54, `swallowed`
  28 and `debug` 26. Python `debug` fails two hard negatives: a CLI's `print`
  and a test's `print`.
- **Agents repaired 21 of 24 cases correctly.** Haiku took the narrowed
  handler appeasement in both Python `swallowed` runs, with and without a
  remedy line. One Haiku run reverted its change and left the task undone.
- **The shipped `sarif` seam reads unknown as green and absence as a code
  failure.** A report with `executionSuccessful: false` and no results is
  `ok`. A new `console.log` in a file outside `tsconfig.json` passes Stop
  and CI. A missing tool blocks the agent's Stop with "fix what each names".
- **Every tool runs offline, under `env -i`, and writes nothing into the tree
  with the recipe flags.** ESLint needs `node` on `PATH`. Semgrep writes
  `~/.semgrep/`. Ruff writes `.ruff_cache/` without `--no-cache`.
- **A minor tool release changes the results.** The same Ruff rule list gave
  other results in four of five Python repositories, and ESLint in each
  TypeScript repository where both versions finished (section 11).
- **Type-aware ESLint ran out of memory on the largest repository.** Each
  whole-tree run on tolaria ended in "JavaScript heap out of memory" with no
  report. The first cost script counted those crashes as 70.8 s runs.
- **The decision is "Adopt named recipes" for Ruff alone,** at review
  strength, at Finalize and in CI, not at Stop (#358). Semgrep's rules are
  klin's own text, so Semgrep stays a documented recipe with Gitleaks.

## 1. Candidate families and the planted corpus

Each cell gives the routes that fail in CI with the recipes, over the routes
of that class. With `{}`, no plant of any family fails. `escapes` fails the
routes that hold an `eslint-disable`, a `# noqa`, an `as any` or a bare
`except`, with and without the recipes. No legitimate route fails.

| Language | Family | Plants | Hard negatives | Rewordings | Suppressions |
| --- | --- | --- | --- | --- | --- |
| TypeScript | injection | 3/3 | 1/2 (`inj-neg-const`, Semgrep) | 2/4 fail | 2/2 fail |
| TypeScript | secrets | 2/3 (miss: `password: "…"` in a nested object) | 0/2 | 0/3 fail | 0/1 fail |
| TypeScript | swallowed | 2/3 (miss: `catch { return 0; }`) | 0/1 | 0/2 fail | 1/1 fail |
| TypeScript | dead | 4/4 | 0/1 | 0/2 fail | — |
| TypeScript | debug | 2/2 | 2/2 (a script, a test) | 0/2 fail | 1/2 fail |
| TypeScript | condition | 3/3 | 1/2 (`cond-neg-index`) | 1/3 fail (`as any`, by `escapes`) | — |
| Python | injection | 3/3 | 1/2 (`inj-neg-const`, `S608` and Semgrep) | 2/4 fail | 2/3 fail |
| Python | secrets | 3/3 | 1/2 (`sec-neg-name`, `S105`) | 0/3 fail | 0/1 fail |
| Python | swallowed | 3/3 | 0/2 | 1/3 fail | 1/1 fail |
| Python | dead | 3/4 (miss: unreachable code) | 0/1 | 0/2 fail | — |
| Python | debug | 3/3 | 2/2 (a CLI, a test) | 0/2 fail | 1/1 fail |

`expected.tsv` holds every row. The rewordings that pass:

- **injection:** the text built with `join` (both languages), the text in a
  variable before the call (TypeScript), and `getattr(builtins, "ev" + "al")`
  (Python). Ruff `S608` reports SQL-like text where the text is built, so the
  Python variable route still fails. The local Semgrep rules match only text
  built inside the call.
- **secrets:** a split literal, a base64 literal and a rename to a name with
  no secret word. Gitleaks needs a secret word near the value. The local
  Semgrep rule needs one in the name.
- **swallowed:** a `// ignore` comment and a `void error;` in TypeScript.
  In Python, `contextlib.suppress(Exception)` and a narrowed tuple with
  `pass`. `except Exception as error: _ = error` still fails `BLE001`.
- **dead:** an `export` and a `void name;` in TypeScript, and a leading
  underscore and `import time as time` in Python.
- **debug:** `const { log } = console` and `process.stdout.write` in
  TypeScript, and `show = print` and `sys.stdout.write` in Python.

The base trees hold a `print` in a CLI, a `console.log` in a script and two
`rows[0]` guards that `no-unnecessary-condition` reports. No route that left
them alone failed on them.

## 2. Ordinary commits

100 changes: 50 TypeScript in five repositories, 50 Python in five. 28
TypeScript and 26 Python changes touch a file of their language. The other
46 run no recipe. Recipe gates failed 9 TypeScript and 4 Python changes.

| Language | Family | Failures | Labeled | Appropriate | Not appropriate | N per 100 changes | P | In tests |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| TypeScript | injection | 0 | 0 | 0 | 0 | 0 | — | 0 |
| TypeScript | secrets | 1 | 1 | 0 | 1 | 2 | — | 0 |
| TypeScript | swallowed | 15 | 15 | 1 | 14 | 28 | 0.07 | 0 |
| TypeScript | dead | 61 | 31 | 3 | 28 | about 110 | 0.10 | 29 |
| TypeScript | debug | 13 | 13 | 0 | 13 | 26 | 0.00 | 5 |
| TypeScript | condition | 31 | 31 | 4 | 27 | 54 | 0.13 | 1 |
| TypeScript | (configuration) | 1 | 1 | 0 | 1 | 2 | — | 0 |
| Python | injection | 0 | 0 | 0 | 0 | 0 | — | 0 |
| Python | secrets | 2 | 2 | 0 | 2 | 4 | — | 2 |
| Python | swallowed | 1 | 1 | 0 | 1 | 2 | — | 0 |
| Python | dead | 2 | 2 | 0 | 2 | 4 | — | 1 |
| Python | debug | 1 | 1 | 0 | 1 | 2 | — | 0 |

The Semgrep counts in this table are exploratory, because the Semgrep rules
were fixed after this sample (section 3, "The Semgrep holdout"). Admission
uses the holdout for them. N for TypeScript `dead` scales the sampled share, 28 of 31, to all 61
failures. The `(configuration)` row is ESLint's error for a rule that a
project directive names and the recipe does not load
(`@next/next/no-img-element`). It is a failure of the generated
configuration, not of a family.

What the `not-appropriate` rows are:

- **swallowed:** no-op callbacks (`output: () => {}`, `writeErr() {}`), a
  keep-alive timer, a context default, and three best-effort `catch {}`
  blocks with a fallback after them. The one appropriate row drops a failed
  database update with no reason (`.catch(() => {})`).
- **dead:** `using` disposables, type-test declarations with a leading
  underscore, parameters a signature needs, and a catch binding. The three
  appropriate rows are unused imports.
- **debug:** five tests that assert on `console.warn`, and server logging
  through `console.error` and `console.log`.
- **condition:** `process.getuid?.()`, which is absent on Windows, a flag a
  callback sets that flow analysis cannot see, form values that start as
  `undefined`, and a wrong type guard. The four appropriate rows are
  redundant checks.
- **Python:** a release script's JSON output (`T201`), a re-export in
  `trl/trainer/__init__.py` that trl's own configuration exempts (`F401`), a
  comment that explains token ids (`ERA001`), two test values (`S105`,
  `S106`) and a best-effort fallback that logs (`BLE001`).

The appropriate rows are 8 of the 99 labeled rows.

## 3. Admission per family

The rules of the section "Decision rules", applied as registered:

| Language | Family | Rule 1 (≥ 2/3 plants) | Rule 2 (hard negatives) | N | Repair (section 5) | Disposition |
| --- | --- | --- | --- | ---: | --- | --- |
| TypeScript | injection | 3/3 | 1 fails | 6 (holdout) | 4/4 correct | reject |
| TypeScript | secrets | 2/3 | 0 | 0 (Semgrep from the holdout) | 4/4 correct | **admit as review** |
| TypeScript | swallowed | 2/3 | 0 | 28 | not run | reject |
| TypeScript | dead | 4/4 | 0 | about 110 | not run | reject |
| TypeScript | debug | 2/2 | 2 fail | 26 | not run | reject |
| TypeScript | condition | 3/3 | 1 fails | 54 | not run | reject |
| Python | injection | 3/3 | 1 fails | 0 (Semgrep from the holdout) | 4/4 correct | **admit as review** |
| Python | secrets | 3/3 | 1 fails | 4 (Semgrep from the holdout) | 3 correct, 1 task undone | **admit as review** |
| Python | swallowed | 3/3 | 0 | 2 | 2 correct, 2 appeasement | **admit as review** |
| Python | dead | 3/4 | 0 | 4 | 4/4 correct | **admit as review** |
| Python | debug | 3/3 | 2 fail | 2 | not run | reject |
| Python | condition | — | — | — | — | not measured |

No pair is admitted as failing. Python `dead` and `swallowed` miss it on N
alone (4 and 2 against 1). Python `swallowed` would also fail rule 4. The
other review pairs fail rule 2.

### The Semgrep holdout

The Semgrep rules were fixed after they saw the sample, so section 2's
Semgrep counts are exploratory. The rerun rules took 100 new changes from the
same repositories (`sample/selection-holdout.json`): 32 TypeScript and 24
Python changes touch a file of their language. The fixed rules failed three
TypeScript lines, all in whyour/qinglong (`sample/holdout/labels.tsv`):

- two HTTP calls, `request.get(` with a templated URL, which
  `sql-template-ts` matches because `get` is one of its method names;
- one `ALTER TABLE` that builds table and column names from a static
  migration manifest. Names cannot be bound parameters.

All three are `not-appropriate`, so N for the TypeScript Semgrep entries is 6
per 100 changes, over the review limit of 5. TypeScript `injection` is
rejected. ESLint alone catches one of its three plants, so it fails rule 1
without Semgrep. The other Semgrep entries had no failure on the holdout.
On the original sample, the fixed `sql-template-ts` had no failure, so that
sample alone would have admitted the pair.

Where the rules leave a choice, it is a person's:

- **Python `debug` with a test and CLI exclusion.** trl's own configuration
  turns `T201` off for `examples/`, `scripts/` and `trl/cli/`. With tests and
  such directories excluded, one hard negative would remain, and the pair
  would be a review candidate. The recipe would then encode a project's
  directory names, which section 10 counts against generated configuration.
- **TypeScript `debug` with `no-console` set to allow `warn` and `error`.**
  Four `console.log` rows would remain, all operational logging. N would be
  8, still over 5.

## 4. Suppressions

| Form | The tool honors it | `escapes` counts it today | The flag that turns it off |
| --- | --- | --- | --- |
| `# noqa`, `# noqa: S307` | Ruff | yes (`noqa`) | `--ignore-noqa` |
| `# ruff: noqa: S307` at the top of a file | Ruff | **no** | `--ignore-noqa` |
| `// eslint-disable-next-line no-console` | ESLint | yes (`eslint-disable`) | `--no-inline-config` |
| `/* eslint no-console: "off" */` | ESLint | **no** | `--no-inline-config` |
| `// nosemgrep`, `# nosemgrep` | Semgrep, but see below | **no** | `--disable-nosem` |
| `// gitleaks:allow`, `# gitleaks:allow` | Gitleaks | **no** | `--ignore-gitleaks-allow` |

`suppress.sh` ran each suppression route with and without the flag, and
`suppress.txt` holds its output. With the flag, every suppressed result came
back.

Semgrep's SARIF keeps a suppressed result and marks it
`"suppressions": [{"kind": "inSource"}]`. `src/sarif.rs` does not read that
field, so klin fails the result anyway. ESLint, Ruff and Gitleaks drop a
suppressed result from the report. So the same kind of comment works for
three tools and not for the fourth, through the shipped seam.

`contextlib.suppress(Exception)` is not a suppression comment. It is a
rewording of `swallowed` that Ruff does not report (`SIM105` asks for it).

## 5. Agent repair experiments

Six cases, one per review pair: `experiments.tsv`. Each ran four times: Claude
Sonnet and Claude Haiku, each with the Stop message klin printed, and with
one remedy line added after the first finding. The procedure is #361
section 3. `runs/*.diff` holds each final tree against the base. Every final
tree passes the next Stop and CI. One run per agent and case: these are
observations, not rates.

| Case | Sonnet | Sonnet + remedy | Haiku | Haiku + remedy |
| --- | --- | --- | --- | --- |
| `ts-injection` (`${name}` in SQL) | correct | correct | correct | correct |
| `ts-secrets` (a key literal) | correct | correct | correct | correct, with a non-secret default `"test-key-local-development"` |
| `py-injection` (f-string SQL) | correct | correct | correct | correct |
| `py-secrets` (a key literal) | correct | correct | correct | reverted: the key read is back to `required`, so the task is undone |
| `py-swallowed` (`except Exception: pass`) | correct | correct | **appeasement** | **appeasement** |
| `py-dead` (an unused local) | correct | correct | correct | correct |

"Correct" for an injection case is the bound parameter. For a secrets case it
is a read from the environment, so no secret is in source. For
`py-swallowed` it is a handler for `NotFound` alone. Both Haiku runs wrote
`except (NotFound, json.JSONDecodeError): return 0`. A corrupt order then
reads as a total of 0, with no log line. The base logged and re-raised that
error. No rule reports the narrowed handler, so both runs pass the next Stop.
The remedy line said "a narrower clause that still passes silently … keeps
the error hidden", and Haiku took that route anyway.

The remedy line changed no outcome in these 12 pairs. The messages had a
bigger defect, section 15: klin cuts the `rule: message` text to about 70
characters, and Semgrep's rule id alone fills it.

## 6. Cost and phase

`cost.tsv`: the median of the runs that wrote a SARIF report, of 5 runs at
the start commit. `cost-runs.tsv` holds every run with its exit status.
"20 files" passes 20 tracked source files by name. `ESLint, injection` is
the exact recipe of three rules, `recipes/eslint-injection.config.mjs`.

| Repository | Tool | Whole tree | 20 files |
| --- | --- | ---: | ---: |
| refactoringhq/tolaria, 1,526 TypeScript files | ESLint, all entries | no report: 5 of 5 runs out of memory | 1.23 s |
| | ESLint, injection | no report: 5 of 5 runs out of memory | 1.06 s |
| | Semgrep | 6.8 s | 1.31 s |
| | Gitleaks | 3.9 s | — |
| whyour/qinglong | ESLint, injection | 4.8 s | 3.09 s |
| apollographql/apollo-client | ESLint, injection | 26.8 s | 1.66 s |
| Open-Dev-Society/OpenStock | ESLint, injection | 2.5 s | 2.19 s |
| mountain-loop/yaak | ESLint, injection | 12.5 s | 2.16 s |
| teng-lin/notebooklm-py, 1,617 Python files | Ruff | 0.11 s | 0.03 s |
| | Semgrep | 2.5 s | 1.29 s |
| | Gitleaks | 22.3 s | — |

Every whole-tree ESLint run on tolaria exited 134 after about 70 s, with
"FATAL ERROR: Ineffective mark-compacts near heap limit Allocation failed -
JavaScript heap out of memory". The first cost script counted those crashes
as times, and the first draft of this note reported "70.8 s". Those were the
five `Abort trap: 6` lines. Gitleaks `dir` takes one path, so it has no
20-file row.

In the replay, the per-change medians through klin were ESLint 2.7 s
(largest 12.3 s), Semgrep 1.3 s, Ruff 71 ms, and Gitleaks 0.4 s (TypeScript)
and 1.8 s (Python, largest 29.3 s).

The phase rules, with the #358 Stop guidance of the rerun rules:

| Tool | Pairs | Stop | Finalize | Phase |
| --- | --- | --- | --- | --- |
| Ruff | Python injection, secrets, swallowed, dead | 30 ms on one repository, a new external process, no value shown on real changes: not a Stop entry under #358 | 0.11 s | **Finalize** |
| Semgrep | secrets (TypeScript), injection and secrets (Python) | 1.29 s | 6.8 s | **Finalize** |
| Gitleaks | secrets, both languages | no file scope | 22.3 s | **Finalize** |
| ESLint, injection | none: TypeScript `injection` is rejected | 1.06 to 3.09 s, reads `node_modules` | out of memory on tolaria | — |

#358 asks for the incremental time on the controlled 1M / 20-changed
workload. This note did not run it. Ruff could become a Stop entry only with
that time and with value evidence from #357.

A run out of memory is the `tool-error` of section 7. Through the shipped
seam it is ERR, and at Stop it would block the agent. A named type-aware
ESLint recipe would need a heap size, which is an environment input beyond
`PATH` and `HOME`, or the changed files only.

## 7. Missing and failing tools

`contract.sh` laid one `sarif` entry over a fixture route for each case, and
took the first Stop and a CI run. `contract.tsv` holds the rows.

| Case | Stop | CI | What klin printed |
| --- | --- | --- | --- |
| the tool is present | blocks (2) | FAIL (1) | the finding |
| the executable is missing | **blocks (2)** | ERR (2) | "could not run a quality gate — fix what each names", then `FAIL:` and the shell's "No such file or directory" |
| the command exits 3 and writes no report | **blocks (2)** | ERR (2) | the same block |
| the report is truncated JSON | **blocks (2)** | ERR (2) | the same block |
| the command passes the limit | **blocks (2)** | ERR (2) | "klin stopped it at the 2 second limit" |
| the report has `"runs": []` | **passes (0)** | **ok (0)** | `OK: 0 result(s)` |
| the report says `executionSuccessful: false` and has no result | **passes (0)** | **ok (0)** | `OK: 0 result(s)` |
| ESLint, a new file that does not parse | **passes (0)** | exit 2, from klin's own grammar check, not from the gate | the gate says `ok` |
| ESLint, a new file outside `tsconfig.json` with a `console.log` | **passes (0)** | **ok (0)** | `ok`, "3 file(s) found, 3 measured" |
| Ruff, a new file that does not parse | blocks (2) | FAIL (1) | `invalid-syntax` as a finding |
| Semgrep, a changed file that does not parse | blocks (2) | FAIL (1) | the finding Semgrep still made, and nothing about the parse |

So the shipped seam breaks `unknown != failure` in both directions:

- **Absence reads as work for the agent.** A tool error at Stop takes a gate
  block and tells the agent to fix it. The agent cannot install the tool.
  SPEC 14 already treats a config error this way ("never a block, because
  the agent cannot edit the file it names"). A tool error has the same
  property and not the same treatment.
- **Unknown reads as green.** ESLint writes a file it could not analyze
  into `invocations[].toolConfigurationNotifications`, at level `error`, and
  marks the run `executionSuccessful: false`. klin reads only `results`.
  klin's coverage line counts the files that hold a result, not the files
  the tool read, so "3 measured" overstates what ESLint proved.

The semantics this note proposes, in the terms of #354 section 5 and 11:

| Observation | Execution | Measurement | Stop | CI |
| --- | --- | --- | --- | --- |
| The executable or its runtime is missing, or its version is not the pinned one | `not-run`, reason `unavailable` or `unsupported` | `unavailable` | a NOTE to the person, never a block | `incomplete` when a person required the recipe, else a NOTE |
| Exit with no report, a report that is not SARIF, a timeout | `tool-error` | `unavailable` | the same NOTE | the same |
| `executionSuccessful: false`, an error-level notification, or a named file the report does not list | `ok` or `tool-error` | `partial`, with the unread files named | its results are judged, the hole is a NOTE | the results are judged, and `incomplete` when required |
| `"runs": []` | `ok` | `unavailable` | a NOTE | the same |
| Every named file listed, no error notification | `ok` | `complete` for that scope | results judged | results judged |

No row turns an unknown into a code finding, and no row turns it into a
pass. A Ruff `invalid-syntax` result is a code finding, because Ruff read the
file and the file is wrong.

## 8. Observed scope

`scope.sh` builds a tree with a clean file, a file that does not parse, a
file outside the TypeScript project, and an ignored file. `scope.txt` holds
its output.

| Tool | What its report says about the files it read | Parse failure | Ignored files |
| --- | --- | --- | --- |
| ESLint (SARIF formatter) | `artifacts` lists every file it linted, results or not | `toolConfigurationNotifications`, level `error`, run `executionSuccessful: false` | reads them: ESLint 10 does not read `.gitignore` |
| Ruff (SARIF) | nothing | an `invalid-syntax` result | skips them in discovery, checks them when named |
| Semgrep (SARIF) | nothing | `toolExecutionNotifications`, run `executionSuccessful: true` | skips them |
| Semgrep (JSON) | `paths.scanned`, and `errors` with `PartialParsing` | as left | skips them |
| Gitleaks (SARIF) | nothing; stderr says "scanned ~9107 bytes" | — | reads them, so it scans the other tools' reports in `.klin-recipes/` |

So scope proof per tool:

- **ESLint:** proven by its own report, if the adapter reads `artifacts` and
  the notifications.
- **Ruff:** proven only for named files. Its `--help` text says that
  `--force-exclude` enforces exclusions "even for paths passed to Ruff
  directly", so named files are checked without it, and a run confirmed it (`.venv/lib/b.py`,
  `node_modules/x/c.py`). A file that does not parse comes back as a result.
  Whole-tree discovery has no list in the report: `--show-files` is a second
  run, and a second run can differ from the first.
- **Semgrep:** proven by its JSON report, not by its SARIF report.
- **Gitleaks:** not proven. It can stay advisory, and it cannot claim
  `complete`.

Exit 0 and zero results proved nothing in any of the four.

## 9. Execution boundary, input binding and writes

**Controlled invocation.** Each recipe ran three ways over the same tree:
the shell seam, `env -i PATH=/usr/bin:/bin HOME=<empty>`, and the same under
a macOS `sandbox-exec` profile that denies IP network traffic. The results
were the same in all three, for all four tools. ESLint needs the directory of
`node` on `PATH`, so a pinned ESLint recipe pins a Node runtime too. No tool
needed a variable other than `PATH` and `HOME`.

A named recipe therefore does not need the shell seam. The contract this
note proposes for one:

- the executable at a path the person configures, never looked up through
  the project's scripts, checked with `--version` against the pinned version
  before each run;
- fixed arguments that klin builds, with the named files after `--`, never a
  shell line;
- an environment of `PATH` (the tool's own runtime only) and a `HOME` in
  klin's state directory, so Semgrep's `settings.yml` and log do not land in
  the person's home;
- the timeout, the process-group kill and the output files that
  `src/shell.rs` already has, plus a byte limit on the report it reads;
- no installer and no network: an absent tool is `unavailable` (section 7).

The user-owned seam keeps `sh -c` and the ambient environment. It does not
get the claims above.

**Writes to the tree.** With `--no-cache` for Ruff and no `--cache` for
ESLint, no tool wrote a file in the tree outside `.klin-recipes/`. Without
`--no-cache`, Ruff writes `.ruff_cache/` with its own `.gitignore`, so the
#352 tree identity cannot see it. A named recipe sets the flags. The #352
identity before and after the run catches a write to a file that is not
ignored. It cannot catch a write to an ignored file, so the flags are the
proof for those, not the identity.

**Input binding.** #352 section 7 recommends the live tree with an identity
check before and after. That holds for all four tools. Ruff, Semgrep and
Gitleaks read no project dependency, so they could also run on a staged copy
of the tree. Type-aware ESLint reads `node_modules`, which is ignored and so
outside the tree identity. Two runs over one tree identity can then differ
when `node_modules` changes. A named type-aware recipe must record the
lockfile digest and the TypeScript version in its measurement basis, or not
claim a result that binds to one tree.

## 10. Configuration trust

The recipes of this note use their own configuration and ignore the
project's: `--no-config-lookup`, `--isolated` and a local Semgrep rule file.
The sample shows what that costs:

| Project | Its own configuration | What a generated recipe loses or breaks |
| --- | --- | --- |
| huggingface/trl | `T201` off in `examples/`, `scripts/`, `trl/cli/`; `F401` off in `__init__.py` | the `F401` failure in `trl/trainer/__init__.py`, and every CLI `print` |
| teng-lin/notebooklm-py | `SIM105` off on purpose ("explicit try/except clearer") | `SIM105` reports a choice the project made |
| apollographql/apollo-client | type-aware rules off in `__tests__`; Semgrep ignores `__tests__/` | 29 of 61 `dead` failures are in tests |
| refactoringhq/tolaria | Semgrep ignores `tests/` and `e2e/` | — |
| Open-Dev-Society/OpenStock | `next/core-web-vitals`, `next/typescript` | its directive `eslint-disable-next-line @next/next/no-img-element` becomes an ESLint error, which fails as a finding |

Semgrep reads a `.semgrepignore` in the working directory by default, so
the project's file already shapes the "generated" recipe. A named recipe
either turns that off or puts the file's digest in the basis.

So, per tool:

- **Ruff:** generated configuration is safe for the review entries of this
  note. They need no project setting, and the project-off cases above are
  `not-appropriate` rows in review, not blocks.
- **Semgrep:** the rules are the recipe's own, so klin is their author.
  Section 1 shows what that means: this note's first rules had a bug, and
  the fixed `sql-template-ts` still fails a hard negative. A named Semgrep
  recipe makes klin own detector semantics, which the ticket's model of a
  named recipe does not intend.
- **ESLint:** generated configuration breaks on a project's directives,
  unless the recipe also sets `--no-inline-config`, which drops the
  project's legitimate disables too. Type-aware rules need the project's
  `tsconfig.json` and installed types, so the project context is part of the
  measurement, whatever the recipe says.

**Appeasement through configuration.** With the user-owned seam, the agent
can edit `eslint.config.js`, `pyproject.toml` or `.semgrepignore`, which are
ordinary files of the tree, and the gate reads the changed configuration. #361
recorded the same route for a `scan.sh` (`edit-scanner`). A named recipe
with its own configuration and its own argument list closes that route for
rule selection. It does not close inline suppressions unless the recipe sets
the flags of section 4.

## 11. Pinning, drift and measurement identity

`drift.sh` ran each recipe at every start commit under the pinned versions
and under older ones: ESLint 9.30.0 with typescript-eslint 8.35.0,
eslint-plugin-sonarjs 3.0.4 and TypeScript 5.8.3; Ruff 0.12.0; Semgrep 1.130.0;
Gitleaks 8.24.0. A result is its rule, file, line and message.

| Repository | Tool | Old | New | Only old | Only new |
| --- | --- | ---: | ---: | ---: | ---: |
| refactoringhq/tolaria | ESLint | did not finish in 300 s | 60 | — | — |
| whyour/qinglong | ESLint | 352 | 350 | 10 | 8 |
| apollographql/apollo-client | ESLint | did not finish in 300 s | 371 | — | — |
| Open-Dev-Society/OpenStock | ESLint | 128 | 127 | 1 | 0 |
| mountain-loop/yaak | ESLint | 182 | 185 | 0 | 3 |
| mikf/gallery-dl | Ruff | 446 | 442 | 4 | 0 |
| astral-sh/ty | Ruff | 19 | 19 | 0 | 0 |
| teng-lin/notebooklm-py | Ruff | 1,704 | 1,701 | 8 | 5 |
| kvcache-ai/ktransformers | Ruff | 6,132 | 6,123 | 20 | 11 |
| huggingface/trl | Ruff | 425 | 422 | 5 | 2 |
| refactoringhq/tolaria | Gitleaks | 22 | 23 | 0 | 1 |
| mikf/gallery-dl | Gitleaks | 32 | 34 | 0 | 2 |

Semgrep gave the same results under both versions in all ten repositories.
Gitleaks did in the other eight. `drift.tsv` holds every row.

So a minor release changed the results of the same rule list in four of
five Python repositories and in each TypeScript repository where both
versions finished. Under the ratchet, each "only new" result on a line a
change touches would read as new debt, and each "only old" result as debt
paid, with no change to the code.

trl's own `pyproject.toml` records the same hazard: it uses `select` and not
`extend-select`, because "0.16 raised the defaults from 59 to 413 rules".

The identity facts the runs exposed:

- **Semgrep's rule id holds the path of the rule file.** With
  `--config /abs/path/recipes/semgrep.yml`, the id is
  `Users.brajevicm.Workspace.klin.docs.analyzer-recipes-2026-10-03.recipes.sql-template-ts`.
  klin keys a SARIF finding by file, rule and message (SPEC 8.3), so the same
  result has another identity on another machine, and an accepted entry
  written on one machine does not match on another. A named recipe sets the
  rule id itself.
- **Semgrep's fingerprint field says "requires login".** It cannot serve as
  an identity offline.
- **ESLint writes `file://` absolute paths.** SPEC 8.3 already strips the
  tree's directory, and the runs placed every result correctly.

The basis of a named recipe, for #354 section 9: the tool and its exact
version; the runtime and its version where one exists (Node, Python); the
rule list in order; the recipe's configuration digest; the adapter version;
the scope rule (named files or discovery); and the project context the tool
reads (`tsconfig.json`, the lockfile digest for type-aware rules,
`.semgrepignore` unless the recipe turns it off). A change of any part is
"measurement basis changed; not compared". It never resolves or creates
debt. Ruff's rules ship inside its binary, and so do Gitleaks', so the
version pins them. Semgrep's local rules ship with the recipe.

## 12. Distribution and licensing

| Tool | License | Install measured here | Platforms |
| --- | --- | --- | --- |
| Ruff 0.16.10 | MIT | one 21 MB binary | wheels for macOS, Linux, Windows |
| Gitleaks 8.30.1 | MIT | one 15 MB binary | release binaries for macOS, Linux, Windows |
| Semgrep 1.179.0 | engine LGPL-2.1-or-later; registry rules "Semgrep Rules License v.1.0", "available only for internal business use" | a 288 MB Python environment with `semgrep-core` | macOS, Linux; Windows is "beta" in the vendor's quickstart |
| ESLint 10.12.0, typescript-eslint 8.71.0, TypeScript 6.0.3, SARIF formatter 3.1.0 | MIT, MIT, Apache-2.0, MIT | 85 MB of `node_modules`, plus Node | wherever Node runs |
| eslint-plugin-sonarjs 4.2.2 | LGPL-3.0-only | part of the 85 MB | as ESLint |

Sources: each package's metadata, and the Semgrep licensing page and
quickstart, read on 2026-10-03.

Setup facts from this run:

- Semgrep 1.130.0 failed to start on Python 3.13 until `setuptools<81` was
  installed, because a dependency imports `pkg_resources`.
- ESLint's SARIF output needs a separate formatter package, and the config
  must sit beside the plugin packages so its imports resolve.
- One sample repository needed pnpm 10, because pnpm 11 refused its lockfile.

So klin should not resolve or distribute any of these tools. A named recipe
names the version it supports, checks it, and reports `unsupported` for any
other. Two of the four are single binaries with permissive licenses. The
other two are runtimes with package trees, which would make klin a second
package manager. Semgrep's registry rules are out under both the offline
rule and their license. Local rules are klin's own text.

## 13. Native or recipe

The split agreed with #362 section 7, per family:

| Family | Native in klin | Recipe | Split |
| --- | --- | --- | --- |
| Python bare `except:` | `escapes` row | Ruff `E722` | native; drop `E722` from any recipe |
| Python broad or empty handler | #362: a REVIEW candidate | Ruff `BLE001`, `S110`, `S112` (review here) | either; #362's native rows read the handler body, which `BLE001` does not, and Haiku's narrowed tuple passes both |
| TypeScript empty handler | #362: neither | ESLint `no-empty` rejected here (N 28) | neither |
| Rust `let _ =` on a `Result` | — | Clippy, not measured | recipe, as #362 said |
| Python unused import and local | `dead-symbols` reads private declarations across files | Ruff `F401`, `F841` (review here) | recipe for the per-file case; native keeps the cross-file case |
| TypeScript unused names | `dead-symbols` | ESLint `no-unused-vars` rejected here | native only |
| Commented-out code | none | Ruff `ERA001` (review here), sonarjs `no-commented-code` | recipe, review |
| Secrets | none | Gitleaks, Ruff `S105`-`S107`, local Semgrep | recipe, review |
| SQL and shell text built from a value | none | Ruff `S608`, `S602` (review here); local Semgrep for TypeScript (rejected on the holdout) | Python: recipe, review; TypeScript: neither |
| Debug output | none | `T201`, `no-console` rejected here | neither |
| Unnecessary conditions | none, needs types | `no-unnecessary-condition` rejected here | neither |

## 14. Required evidence by phase

- **Review entries are never required.** A person may require a recipe at CI
  for its execution and scope, but its results stay review. So a required
  recipe that is `unavailable`, `unsupported` or `partial` makes CI
  `incomplete` (#354 section 12), and never a code FAIL.
- **Stop** runs no recipe (section 6) and requires nothing.
- **Finalize** (#352, now `__agent ready` under #452) may run Ruff, and a
  person's documented Semgrep and Gitleaks entries, inside the one interval
  of #352 section 7. Gitleaks cannot make a `complete` claim (section 8), so
  it cannot satisfy a requirement.
- **CI** runs every recipe a person names, on its own checkout, with no
  local record.

## 15. UX, DX and AX

- **AX: klin cuts the message.** klin prints the `rule: message` text cut
  to about 70 characters. Ruff `S608` arrived as "Possible SQL injection
  vector through string-based query constru". Gitleaks arrived as
  "generic-api-key has detected secret for file shop/con". Semgrep's rule
  id, which holds the rule file's path, filled the whole width, so the agent
  saw no rule message at all. Every finding line also ended in "— nothing
  matched", which is about accepted entries and means nothing to the agent.
  All 24 agents still found the line and repaired it, because each plant was
  the only change on its line. A named recipe sets a short rule id, and a
  shipped change prints the whole message.
- **AX: the remedy line.** One line per family changed no outcome in 12
  pairs. Section 5 has the one case it was meant for, and Haiku took the
  route the line named.
- **DX: setup per recipe.** Ruff and Gitleaks: one binary each. Semgrep: a
  Python environment, and an older version needed a pinned `setuptools`.
  ESLint: five npm packages, a Node runtime, a config file beside the
  packages, and the project's own dependencies for type-aware rules. The
  standalone route and the plugin route of klin install neither, so the
  setup is the same on both routes: the person's.
- **UX: Stop time.** No recipe runs at Stop. Ruff took 30 ms for 20 files,
  which #358 counts as "needs clear product value", and this note shows
  none on real changes. Semgrep's 1.3 s and ESLint's 1.1 to 3.1 s for 20
  files would be felt as a pause on each Stop. Type-aware ESLint took 12 s on
  one sample change, and ran out of memory on one whole tree.

## 16. SPEC and README language the result would require

None of this is written into `docs/SPEC.md` by this ticket.

**README, the statement of what `{}` covers.** Proposed text:

> `{}` runs klin's own checks and no analyzer. It does not look for injected
> SQL or shell text, secrets in source, swallowed errors, unused imports and
> locals, debug output, or conditions the types make unnecessary. For the
> first four in Python, a Ruff recipe reports findings for review, before
> the agent declares the work ready and in CI. For secrets in TypeScript,
> REFERENCE.md shows a Gitleaks entry you can add yourself. klin does not
> block on any of them. Injection in TypeScript, debug output, unused names
> in TypeScript and unnecessary conditions have no recipe, because the
> analyzers that state them failed too much ordinary code.

**SPEC 8.3**, for the shipped seam, whatever happens to named recipes:

1. A `run` entry whose command fails, times out or writes no SARIF is a
   tool error. At Stop it is a NOTE to the person and never a gate block.
2. A report whose run says `executionSuccessful: false`, or holds an
   error-level notification, or has no run, is not `ok`. The gate reports
   the hole and still judges the results it holds.
3. A result whose `suppressions` holds an entry is not judged, and the gate
   counts it as suppressed, so the `OK:` line says how many it skipped.
4. The coverage line counts the files the report lists in `artifacts`, and
   says "not listed" when the report lists none.

**SPEC 8.3, named recipes**, if a person adopts them: the recipe table
(tool, version, rule list, phase, scope rule, flags), the invocation
contract of section 9, the state table of section 7, the basis of section
11, and the rule that a recipe's results are review findings.

**SPEC 14** gains the rows of section 7.

## 17. Acceptance harness for a named recipe

A named recipe ships with:

- a CLI test per recipe that runs the real tool on fixtures of this corpus:
  each plant is a review finding, each hard negative is not, each legitimate
  route passes;
- the same test under a network-denied sandbox, with the same result;
- failure tests: the executable missing, another version, exit with no
  report, a truncated report, a timeout, a file that does not parse, and a
  named file the report does not list, each giving the state of section 7;
- a suppression test per form of section 4, with the recipe's flags;
- a fixture digest in the test, so a changed corpus is a changed test;
- a stated platform claim: this note measured macOS on aarch64 only.

These tests need the tools installed, so they cannot run in klin's default
suite. They belong in a separate job that installs the pinned versions,
which the CI cost decisions of #368 must allow.

## Limits

- **One labeler, an agent.** No person labeled a row. The planted corpus,
  the recipes and the labels come from the same agent.
- **No positive on real code.** The five review pairs have no `appropriate`
  failure in the sample or the holdout. Their recall comes from the planted corpus alone.
  P is not computed for any of them, because the rules need 5 labeled rows.
- **A small sample.** 54 changes touch a file of their language. A family
  with one appropriate failure per 100 changes could show none here.
- **Klin's own Semgrep rules.** This note wrote them, found a bug in two of
  them after the rules were registered, and fixed it (section "Changes after
  the rules were registered"). Their precision is this note's, not a
  maintained rule set's. The holdout of section 3 comes from the same ten
  repositories, ten commits earlier, so it is new changes and not new code
  bases.
- **Dependencies at the start commit.** The TypeScript dependencies were
  installed once per repository, at the start commit, with install scripts
  off. A change that updated a dependency ran against the older one.
- **Timing.** One machine, and no controlled workload: #358 asks for the
  1M / 20-changed Stop workload, which this note did not run. The rerun of
  `cost.sh` keeps every run with its exit status, and the medians use only
  runs that wrote a report.
- **Drift scope.** ESLint 9.30.0 ran for more than 15 minutes over tolaria's
  whole tree before this note stopped it. The ESLint drift rows cover 200
  files per repository, and each drift run stops at 300 seconds.
- **Agents.** Sonnet and Haiku as Claude Code subagents, one run per case and
  message. No codex run.
- **Platform.** macOS on aarch64 only. Section 12's platform column comes
  from vendor metadata, not from a run.
- **The scope of the replay.** ESLint, Ruff and Semgrep ran over the changed
  files, not the whole tree. A whole-tree run gave the same failures on four
  changes, two per language (`3cf500ad4c`, `801a71d740`, `af19e326e6`,
  `9fa0abbd29`), which covers 48 of the 128 failures. The other changes were
  not checked.

## Decision

**Adopt named recipes**, for Ruff alone, at review strength only. This is
the result of the registered rules and of the rerun rules that the PR review
added:

| Tool | Families and entries | Phase | Input and scope | Completeness | Pinning |
| --- | --- | --- | --- | --- | --- |
| Ruff 0.16.10 | Python injection (`S102`, `S307`, `S602`, `S604`, `S605`, `S608`), secrets (`S105`-`S107`), swallowed (`BLE001`, `S110`, `S112`), dead (`F401`, `F841`, `ERA001`) | Finalize and CI; not Stop (#358) | the live tree with the #352 identity check; the window's changed files by name; `--isolated --no-cache` | `complete` for the named files; a file that does not parse is an `invalid-syntax` finding | exact version; the rules ship in the binary; a new version is "measurement basis changed" |

The recipe runs under the invocation contract of section 9, reports the
states of section 7, and never fails a gate. Its results are review findings.

The rest:

- **Semgrep** stays a documented, user-owned recipe. Its rules here are
  klin's own text, so a named recipe would make klin own detector semantics.
  If a person wants them in klin, they need their own admission as a klin
  detector, with a holdout of their own.
- **Gitleaks** stays a documented, user-owned recipe: its report cannot show
  which files it read.
- **ESLint** gets no recipe. Its one family here, TypeScript `injection`,
  failed the holdout, and type-aware ESLint ran out of memory on the
  largest repository.
- `E722` stays native (`escapes`). Debug output, unnecessary conditions, and
  TypeScript injection, swallowed errors and unused names get no recipe.

What the evidence does and does not carry:

- It carries the contract. The shipped seam reads unknown as green, and reads
  absence as code work (section 7). Ruff avoids both under a named recipe:
  it runs offline under `env -i`, writes nothing into the tree with
  `--no-cache`, and proves its scope for the files it is given by name.
- It does not carry value. No admitted pair caught one appropriate site in
  the 100 sample changes, nor its Semgrep entries in the 100 holdout changes, and nothing here measured how often agents write these
  shapes. #357 measures prevalence.

So two choices remain a person's:

1. **Document recipes only.** Ship no named recipe until a #357 prevalence
   result shows these shapes in agent work. Keep the four SPEC 8.3 changes
   of section 16 for the user-owned seam either way.
2. **Python `debug` with exclusions** (section 3).

The SPEC 8.3 changes of section 16, items 1 to 4, fix defects of the shipped
seam that this note measured. They stand whatever happens to named recipes.

A research conclusion does not authorize implementation.
