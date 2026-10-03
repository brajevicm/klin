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
