# Unfinished and error-masking production code, 2026-10-02

This note belongs to #362. It asks which shapes of unfinished or
error-masking production code klin can find from syntax alone, as new or
worsened sites, with enough precision, rewording resistance and repair
legitimacy to become future findings.

It is research only. It adds no gate, changes no shipped behavior and edits no
SPEC semantics. A disposition here does not authorize an implementation: that
is a separate ticket after a person reviews this note.

## Terms

This note uses the words of #361 and #362. They are not terms of `CONTEXT.md`:

- A **candidate** is one predicate this research measures. It is not a gate.
- A **site** is one place a candidate matches. Its identity is the file, the
  candidate and the trimmed text of the site's line, the way `stubs` and
  `escapes` key a site (SPEC 8.2.1).
- A **finding** is a site that no exclusion of its candidate covers.
- A **plant** is a planted shortcut. A **hard negative** is planted code that
  a candidate must not find. A **rewording** is a planted change that keeps the
  shortcut and changes its spelling.
- **`shapes`** is the throwaway prototype that measures every candidate,
  `docs/unfinished-code-2026-10-02/prototype/`. It is not klin code. It uses
  the tree-sitter grammars klin pins (`Cargo.toml`), so it parses each file the
  way klin does.

## Rules, written before any run on the sample

The commit that adds this section holds no result of the ordinary-commit
sample. The prototype ran before this commit only on four local repositories
outside the sample, to find parser mistakes.

### Languages

Rust (`.rs`), TypeScript (`.ts`, `.mts`, `.cts`) and TSX (`.tsx`), and Python
(`.py`, `.pyi`). JavaScript, Go and the other languages of klin are out of
scope. Like klin, the prototype skips the directories of
`DEFAULT_SKIP_DIRS` in `src/files.rs`, `fixtures` among them, and `.git`.

### Candidates

The ticket names candidates 1 to 8. Candidate 9 comes from the #361 hand-off
comment on #362, which lists throw-only bodies (`unreachable!()`,
`raise RuntimeError(...)`, `throw new UnsupportedFormat("csv")`) as rewordings
that pass `stubs`.

A **constant** below is one of these expressions, and the shape name of the
site says which:

- `empty`: `[]`, `{}`, `()`, `""`, `0`, `None`, `null`, `undefined`;
  `list()`, `dict()`, `set()`, `tuple()`, `str()`, `frozenset()`;
  `new Map()`, `new Set()`, `new Array()`, `new Object()`; `vec![]`;
  `Vec::new()`, `String::new()` and the other standard collections' `new()`;
  `X::default()` for any `X`. A TypeScript `as`, `satisfies` or parenthesis
  around one of these is read through.
- `wrapped`: `Ok(K)`, `Some(K)`, `Promise.resolve()` or `Promise.resolve(K)`,
  where `K` is a constant.

`true`, `false` and other numbers or strings are not constants, because a
flag or a name function returns them by design.

| # | Candidate | Predicate |
| --- | --- | --- |
| 1 | `constant-return` | A function whose body, comments and a Python docstring left out, is one `return K` or, in Rust, one tail `K`, where `K` is a constant. A TypeScript arrow function `const f = () => K` at the top of a module or as a class field is shape `arrow-*`. |
| 2 | `ellipsis-body` | A Python function whose body, a docstring left out, is the single expression `...`. |
| 3 | `placeholder-wording` | A comment or a string literal that holds a phrase of the lexicon below, as whole words, case-insensitively. One site per line. |
| 4 | `empty-handler` | A handler whose body holds no statement, or only `pass`, `...`, `continue`, `break` or a bare `return`: a TypeScript `catch` clause or `.catch(callback)`, a Python `except` clause, a Rust `Err(..) =>` match arm. Also a Python `with suppress(...)`, a Rust `let _ = <call>;` and a Rust `<expr>.ok();` statement. |
| 5 | `default-handler` | A handler whose body is only `return K` (Rust: only `K`), where `K` is a constant, and a TypeScript `.catch(() => K)`. Also a Rust `.unwrap_or_default()`, `.unwrap_or(K)` and `.unwrap_or_else(\|..\| K)`. |
| 6 | `log-handler` | A handler whose body is only log calls (shape `log`), or log calls and then one statement that candidate 4 or 5 accepts (shape `log-default`). |
| 7 | `broad-handler` | A broad handler that candidates 4 to 6 do not take and that does not propagate: no `raise`, no `throw`, no `Promise.reject`, no Rust `?`, `Err` or `panic!`. Python: `except:`, `except Exception`, `except BaseException`, or a tuple that holds one. TypeScript: every `catch`, because it has no type, unless its body holds `instanceof`. Rust: `Err(_)`, `Err(name)` or `Err(..)`. |
| 8 | `mock-name` | A declared name (function, class, struct, enum, const, static, a variable or field) whose words hold `mock`, `fake`, `dummy` or `placeholder` (and their plurals and `-ed` forms), or a string literal whose words do. Words are split at `_`, at non-alphanumerics and at a lower-to-upper case change. |
| 9 | `throw-body` | A function whose body, comments and a docstring left out, is one `raise`, one `throw`, one Rust `panic!`, `unreachable!`, `todo!`, `unimplemented!` or `bail!`, or one Rust `Err(..)` as its tail or `return`. |

A log call is `print(...)`, `traceback.print_exc()`, `warnings.warn(...)`, a
call `X.log/warn/warning/error/info/debug/trace/exception/critical(...)`
where `X` is `console` or holds `log` in any case, and a Rust `println!`,
`eprintln!`, `print!`, `eprint!`, `dbg!`, `log!`, `trace!`, `debug!`,
`info!`, `warn!` or `error!`, a path prefix such as `tracing::` allowed.

The lexicon of candidate 3, in the order the prototype reads it. The first
phrase that matches a line names the site's shape:

```text
in a real implementation, in a real app, in a real application,
in a real system, in a real scenario, in a real world, in production, you,
in production you, real implementation, actual implementation,
replace with actual, replace with real, replace this with, should be replaced,
needs to be implemented, to be implemented, not yet implemented,
not implemented yet, not supported yet, not yet supported, coming soon,
later release, future release, future version, simplified version,
simplified implementation, simplified logic, temporary implementation,
temporary fix, hardcoded for now, hard-coded for now, mock data,
mock implementation, mock response, dummy data, dummy implementation,
fake data, for demonstration, for demo purposes, for simplicity,
would normally, for now, placeholder, stub (and stubs)
```

The lexicon holds none of the `stubs` comment markers (`TODO`, `FIXME`, `XXX`,
`HACK`), so it does not count a site that `stubs` counts.

### Exclusions

Each exclusion is a tag on a site. A site with any tag is not a finding. The
results report each tag's count, so a reader can judge a rule with a tag
removed.

| Tag | Applies to | Rule |
| --- | --- | --- |
| `test` | all | The path has a segment `test`, `tests`, `__tests__`, `spec`, `specs`, `testing`, `e2e`, `__mocks__`, `mocks`, `testdata`, `test_utils` or `testutils`; or the file is `test_*.py`, `*_test.py`, `*_test.rs`, `conftest.py`, `tests.rs`, `*.test.*`, `*.spec.*` or `*.stories.*`; or the site is inside a Rust `#[cfg(test)]` item. |
| `example` | all | The path has a directory `examples`, `example`, `benches`, `bench`, `docs`, `demo` or `demos`. |
| `generated` | all | The first 2 KiB of the file hold `@generated`, `do not edit`, `auto-generated`, `autogenerated` or `code generated`, case-insensitively. |
| `declared` | 1, 2, 9 | A `.pyi` or `.d.ts` file; a decorator `abstractmethod`, `abstractproperty` or `overload`; a class whose bases name `Protocol`, `ABC` or `ABCMeta`. |
| `trait-default` | 1, 2, 9 | A default method in a Rust trait. |
| `trait-impl` | 1, 2, 9 | A method in a Rust `impl Trait for Type`. |
| `override` | 1, 2, 9 | A method in a Python class with a base other than `object`, or in a TypeScript class with `extends` or `implements`. |
| `base-default` | 1, 2, 9 | A method with a body in a TypeScript abstract class. |
| `arrow` | 1 | Shape `arrow-*`. |
| `documented` | 1, 2, 9 | A comment ends on the line above the declaration, or the Python function has a docstring. |
| `commented` | 1, 2, 4 to 7, 9 | The body or the handler holds a comment. |
| `unreferenced` | 1, 9 | The function's name occurs once or less as a word in the tree's source files that the `test` rule does not cover. This approximates "production code calls it". |
| `specific` | 4 to 7 | The handler names an error type that is not broad: a Python type other than `Exception` and `BaseException`, a `suppress(...)` whose arguments name an `Error`, a Rust `Err` pattern other than `_`, a name or `..`. |
| `typed-return` | 5 | The function that holds the handler declares a return type that names `Option`, `Optional`, `None`, `null`, `undefined` or `Result`, so the constant may be the documented error value. |
| `stubs` | 9 | The body is a site `stubs` already finds: `todo!(`, `unimplemented!(`, `raise NotImplementedError`, a `throw new ...Error(` that says "not implemented". |
| `escapes` | 4 to 7 | A bare Python `except:`, which `escapes` already finds. |

### Identity and before and after

A site's identity is the candidate, the file and the trimmed line text. A site
is new when the after tree holds more sites of that identity than the before
tree, as a `count` rise in `stubs` (SPEC 8.2.1). Exclusion tags do not enter
the identity, so a site the base holds is held whatever its tags.

`shapes new BEFORE AFTER FILE...` prints the after sites whose identity count
rises. It reads the whole of each tree for the `unreferenced` word counts and
judges only the files named.

### Planted corpus

`docs/unfinished-code-2026-10-02/fixtures/` holds one family per language and
candidate group, in the layout of #361: a `base/` tree and one directory per
route laid over it. Each family holds:

- at least one plant for each candidate the family covers;
- the hard negatives of #362 that apply to the language;
- at least the six rewording attacks of #362, where they apply;
- a legitimate repair.

`probe.sh` lays each route, runs the shipped Stop and CI exactly as the #361
probe does, and adds the `shapes new` findings of the route. So each row says
what klin finds today and what each candidate adds.

### Ordinary commits

The sample is the #343 sample, frozen in
`benchmark/evidence/false-alarms-2026-09-29/selection.json`: five Rust and
five TypeScript repositories, ten changes each. #343 has no Python, so this
note adds five Python repositories by the #343 rules, with these
differences:

- The query is
  `language:Python stars:1000..20000 pushed:>=2026-09-15 archived:false mirror:false size:<=150000`,
  run on 2026-10-02, sorted by stars in descending order, 30 results, first
  page.
- The root file is `pyproject.toml`, `setup.py` or `setup.cfg`.
- The start commit is the first commit on the first-parent walk of the default
  branch whose committer date is earlier than 2026-09-29T00:00:00Z, the #343
  cutoff.

The response and the selection are kept beside the corpus. For each change,
the files are those `git diff --name-only --diff-filter=AMR base head` names
with an extension above, and `shapes new` runs over `git archive` exports of
the base and the head.

Each repository's start commit is also scanned whole. The count of existing
sites is context: how often mature code holds each shape. It does not enter a
decision.

### Labels

Every new finding gets one label:

- `appropriate`: the site is unfinished work, or an error that the code drops
  with no stated reason, and a reviewer of the change would ask for a change.
- `not-appropriate`: the site is a constant, a no-op or a best-effort handler
  that the surrounding code shows to be intended, such as an interface
  contract, a documented fallback, a feature that is off, or a correct empty
  answer.

Where a candidate has more than 40 new findings in one language, a sample of
40 is labeled: every k-th finding in the order `shapes` prints, with k the
smallest integer that gives 40 or fewer. The count of every finding is still
reported. Excluded new sites are counted by tag and not labeled.

The labeler is the agent that wrote this note (Claude Opus 5.5). No person
labels each row, as in #343, and the results say so.

### Decision rules

For each candidate, per language, with `N` the count of `not-appropriate` new
findings per 100 changes and `P` the share of `appropriate` labels, computed
only where 5 or more findings were labeled:

- **BLOCK candidate** when all of these hold:
  1. every plant of the candidate is a finding;
  2. no hard negative of the candidate is a finding;
  3. `N` is 1 or less in every language, and `P` is 0.9 or more where it is
     computed;
  4. no rewording attack of #362 removes the finding at a cost no larger than
     the plant, unless another candidate or a shipped gate then finds it;
  5. the agent repair experiment ends in no appeasement.

  #362 says that a candidate that is cheaply reworded must not graduate
  directly to a blocking predicate. So a candidate that fails only rule 4 is
  at most a FINALIZE/REVIEW candidate.
- **FINALIZE/REVIEW candidate** when rules 1 and 2 hold and `N` is 5 or less
  in every language.
- **Delegate to an analyzer recipe (#363)** when a maintained analyzer rule
  states the same predicate, and the native version would need a fact that
  syntax does not hold, such as a type that tells `Result` from `Option`.
- **Reject** otherwise: when `N` is over 5 in a language, or when the claim
  needs a fact that syntax does not hold and no analyzer rule states it.

A candidate can be admitted for one language and not another. Where the
labels or the rules leave a choice, the note says so and names it.

### Stop-path cost

`shapes time ROOT` parses every file of a tree and walks it once, and reports
the parse time and the walk time apart. The walk is the incremental cost,
because klin already parses each changed file at Stop for `complexity` and
`stubs`. The note reports the walk time of 20 changed files on the largest
repository of each language, and says which facts klin's current function
walk and structural index already hold. The prototype is not tuned, so its
walk time is an upper bound.

### Agent repair experiments

Every BLOCK or FINALIZE/REVIEW candidate gets an agent repair experiment. A
planted case gets the task text, and the agent gets the drafted remedy text in
the shape of a klin Stop message, with the procedure of #361 section 3. The
next stop runs `shapes` and klin. The outcomes are those of #361: `correct`,
`appeasement` (the finding goes and the shortcut stays, including a spelling
change), `harmful`, plus `escalated` and `turns`. The agents are Claude Sonnet
and Claude Haiku as Claude Code subagents, and `gpt-6.1-sol` through
`codex exec` where it runs. One run per agent and case: these are
observations, not rates.
