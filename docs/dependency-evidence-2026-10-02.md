# Dependency existence and provenance under the offline invariant, 2026-10-02

This note belongs to #364. It asks what klin can prove about a newly
introduced dependency without a network, and whether an Enforced CI path
should add evidence that a dependency exists and is not newly registered.

It is research only. It adds no gate, adds no network access, changes no
shipped behavior and edits no SPEC semantics. A disposition here does not
authorize an implementation: that is a separate ticket after a person reviews
this note.

## Terms

This note uses the words of #361, #362 and #364. They are not terms of
`CONTEXT.md`:

- A **candidate** is one predicate this research measures. It is not a gate.
- A **dependency** is a name that a manifest declares. A **package** is the
  thing a registry serves under that name. A **module** is what a source file
  imports. In Python a module name and a distribution name can differ:
  `import yaml` needs the distribution `PyYAML`.
- A **site** is one place a candidate matches. A **finding** is a site that no
  exclusion of its candidate covers.
- A **plant** is a planted shortcut. A **hard negative** is planted code that
  a candidate must not find. A **rewording** is a planted change that keeps the
  shortcut and changes its spelling.
- A **hallucinated name** is a package name that no registry serves on the
  snapshot date. Section 2 lists the names this research uses, and why it uses
  names that a registry does not hold.
- **`deps`** is the throwaway prototype that measures candidates 1 to 3,
  `docs/dependency-evidence-2026-10-02/prototype/`. **`registry`** is the
  throwaway lookup that measures candidate 4. Neither is klin code.

## Rules, written before any run on the sample

The commit that adds this section holds no result of the ordinary-commit
sample, the dependency stratum or the Python survey.

### Candidates

| # | Candidate | Predicate |
| --- | --- | --- |
| 1 | `undeclared-import` | A new import of a third-party module that no manifest in the tree declares. Python and TypeScript/JavaScript only. |
| 2a | `python-lock` | The shipped `lockfile` predicate, read from a Python lock format. Measured as the share of Python projects that hold a lock format klin could read. |
| 2b | `entry-shape` | A lockfile entry for a registry package that does not carry the fields a resolver writes for one: npm `resolved` and an `integrity` of the form `sha512-` plus 86 base64 characters and `==`, or `sha1-` plus 27 and `=`; Cargo `source = "registry+..."` and a `checksum` of 64 hex digits; Go `h1:` plus 43 base64 characters and `=`. |
| 3 | `new-dependency` | A direct dependency name that a manifest of the after tree declares and no manifest of the before tree declares. Every ecosystem klin reads, plus Python. |
| 4 | `registry-evidence` | For each new dependency of candidate 3: does the public registry serve the name, and how old is its first release on the date of the change. |
| 5 | `external-report` | A report that a tool outside klin writes about dependencies, read through the shipped `sarif` seam. |

Rust and Go are out of scope for candidate 1. Their compilers refuse an import
of a crate or a module that the manifest does not declare, and no package
manager hoists an undeclared one into reach. #434 owns whether `klin gate
--strict` runs that compiler.

### Candidate 1 in detail

**Python.** `deps` parses each `.py` file with the `ast` module of Python
3.14. A site is an `import a.b` or a `from a.b import c` with no leading dot.
The module of the site is its first segment, `a`. A site is not judged when:

- the module is in the standard library: `sys.stdlib_module_names` of Python
  3.14, plus the modules that Python 3.12 and 3.13 removed (`asynchat`,
  `asyncore`, `distutils`, `imp`, `smtpd`, and the 19 modules of PEP 594), so
  a project that still supports an older Python is not judged on them.
- the module is local: some directory of the tree holds `<module>.py`,
  `<module>.pyi`, `<module>.pyx` or `<module>.so`, or a directory `<module>/`
  that holds a `.py` file.

The declared distributions are the union over every manifest of the tree:

- `pyproject.toml`: `[project].dependencies`, every list of
  `[project.optional-dependencies]` and `[dependency-groups]`,
  `[build-system].requires`, the keys of `[tool.poetry.dependencies]`,
  `[tool.poetry.dev-dependencies]` and every
  `[tool.poetry.group.<name>.dependencies]`, and `[tool.uv].dev-dependencies`
  and every list of `[tool.pdm.dev-dependencies]`.
- every file whose name matches `requirements*.txt` or `*.in` in a
  `requirements` directory, and every file a `-r` or `-c` line names.
- `setup.cfg`: `install_requires` and every `extras_require` entry.
- `setup.py`: the string literals in the list or tuple that an
  `install_requires`, `extras_require` or `tests_require` keyword holds.
- `Pipfile`: the keys of `[packages]` and `[dev-packages]`.

A requirement string is read up to its first character that a name cannot
hold, so `pywin32; sys_platform == "win32"` and `numpy>=2` both declare their
name. A `setup.py` whose keyword holds something other than a list or tuple of
literals, and a `pyproject.toml` that names `dependencies` in
`[project].dynamic` with no file this list reads, make the tree's declared set
**unknown**. A tree whose declared set is unknown produces no finding, and the
results count such trees.

A module `m` is declared under one of three mapping variants. `norm` is the
name rule of PEP 503: lower case, and every run of `-`, `_` and `.` made one
`-`.

- `exact`: some declared distribution `d` has `norm(d) == norm(m)`.
- `table`: `exact`, or the pipreqs table maps `m` to a distribution `t` with
  `norm(t) == norm(d)` for some declared `d`. The table is
  `prototype/pipreqs-mapping`, from `bndr/pipreqs` at
  `48dbafd39003b9de177b8d314795e45797555850` (Apache-2.0).
- `prefix`: `table`, or some declared `d` has `norm(d)` starting with
  `norm(m)` and `-`, so `google-cloud-storage` declares `google`.

**TypeScript and JavaScript.** `deps` reads `.ts`, `.tsx`, `.mts`, `.cts`,
`.js`, `.jsx`, `.mjs` and `.cjs` files with regular expressions after it
removes comments. A site is the specifier of an `import ... from`, an
`import "x"`, an `export ... from`, an `import("x")` or a `require("x")` with
a string literal. klin's own extractor reads these with tree-sitter, so the
prototype's text match is an approximation, and the results name each
site the regular expressions misread. The package of a bare specifier is its
first segment, or its first two for a specifier that starts with `@`. A site
is not judged when the specifier:

- is relative or absolute (`.`, `/`), a URL, or has a scheme such as `node:`,
  `bun:`, `npm:`, `jsr:`, `virtual:` or `astro:`.
- is a Node.js built-in module (`module.builtinModules` of Node.js 24).
- starts with `#` (a `package.json` `imports` entry), or matches a key of
  `compilerOptions.paths` in any `tsconfig*.json` or `jsconfig.json` of the
  tree, `*` taken as a wildcard.
- starts with `$` or `~`, the prefixes of SvelteKit and of common bundler
  aliases, which a bundler configuration in code declares.

A package is declared when some `package.json` of the tree names it in
`dependencies`, `devDependencies`, `peerDependencies` or
`optionalDependencies`, or names `@types/<package>` there (for a scoped
`@a/b`, `@types/a__b`), or holds it as its own `name` (a workspace member).

**Tags.** Each tag excludes a site. The results count each tag, so a reader
can judge a rule with a tag removed.

| Tag | Rule |
| --- | --- |
| `test` | The path rule of the #362 `test` tag. |
| `example` | The path has a directory `examples`, `example`, `docs`, `demo`, `demos`, `benchmarks`, `benches`, `bench` or `scripts`. |
| `vendored` | The path has a directory `vendor`, `_vendor`, `vendored`, `third_party`, `third-party` or `extern`. |
| `generated` | The #362 `generated` rule, plus a Python file whose name ends in `_pb2.py` or `_pb2_grpc.py`. |
| `guarded` | Python: the import sits in the body of a `try` that has a handler for `ImportError`, `ModuleNotFoundError`, `Exception` or every exception. |
| `type-only` | Python: the import sits under `if TYPE_CHECKING:` or `if typing.TYPE_CHECKING:`. TypeScript: `import type` or `export type`. |
| `platform` | Python: the import sits under an `if` whose test reads `sys.platform`, `os.name`, `platform.system()` or `sys.version_info`. |

**Identity and before and after.** The identity of a site is the module
(Python) or the package (TypeScript), not the file, because the claim is about
the tree's manifests. A finding is new when the after tree has a judged,
untagged site of that identity that no manifest declares, and the before tree
has none. It is reported at the first such site in path order.

### Candidate 2 in detail

**2a.** The Python survey below counts, per repository, the lock formats at
the root and at any depth: `uv.lock`, `poetry.lock`, `pdm.lock`,
`Pipfile.lock`, `pylock.toml` and `pylock.*.toml`, a `requirements*.txt` that
holds `--hash=`, and a `requirements*.txt` that `pip-compile` or
`uv pip compile` wrote, from its header.

**2b.** `deps lockshape ROOT` reads every `package-lock.json`, `Cargo.lock`
and `go.sum` of a tree and prints each entry that fails the shape. The shape
does not judge an entry that has no registry behind it:

- npm: an entry with `link: true`, an entry with `inBundle: true`, the root
  entry `""`, an entry whose key does not hold `node_modules/` (a workspace
  member), and an entry whose `resolved` is not an `https` URL (a git or a
  file dependency). An entry with no `resolved` is judged. A version 1
  lockfile's nested `dependencies` tree is read with the same rules.
- Cargo: a `[[package]]` with no `source` is a path or workspace package, and
  one whose `source` starts with `git+` is a git package. A `source` that
  names a registry other than crates.io still needs a checksum. A package
  with no `source` whose name a `[[package]]` of a `[patch]` or a path
  dependency holds is not judged either, which is the same rule.
- Go: every line is judged.

The question for 2b is precision on real lockfiles. Every real lockfile
entry that fails the shape is a hard negative.

### Candidate 3 in detail

`deps added BEFORE AFTER` prints each direct dependency name that the after
tree's manifests declare and the before tree's do not, per ecosystem. The
manifests are those klin reads (SPEC 8.2.1: `Cargo.toml`, `package.json`,
`go.mod`) and the Python manifests of candidate 1. A name is compared after
`norm` in Python and as written elsewhere. A path, git or workspace
dependency is not a new dependency.

### Candidate 4 in detail

`registry` looks up each new dependency of candidate 3 in its public
registry:

| Ecosystem | Request | First release |
| --- | --- | --- |
| PyPI | `https://pypi.org/pypi/<name>/json` | earliest `upload_time_iso_8601` of any file |
| npm | `https://registry.npmjs.org/<name>` | `time.created` |
| crates.io | `https://crates.io/api/v1/crates/<name>` | `crate.created_at` |
| Go | `https://proxy.golang.org/<module>/@v/list`, then `@v/<version>.info` of the earliest version | the earliest `Time` |

The result of one lookup is one of:

- `present`, with the first release time.
- `absent`: the registry answered 404 (Go: 404 or 410).
- `unknown`: any other answer, a timeout of 10 seconds, or no network.
- `private`: the tree names another index for this ecosystem, so the public
  registry is not the source. The rules: an `.npmrc` with a `registry=` line
  or a scope `@x:registry=` line for the package's scope; a Cargo dependency
  with a `registry` key; a `pyproject.toml` `[[tool.uv.index]]`,
  `[[tool.poetry.source]]` or `[[tool.pdm.source]]`, or a requirements
  `--index-url` or `--extra-index-url` line; a Go module that matches a
  `GOPRIVATE` pattern the tree states in a committed `.env`, `Makefile` or CI
  file. A `private` result is never looked up.

**Reproducibility rule.** Every lookup is cached in
`registry/snapshot.tsv`, keyed by ecosystem, name and snapshot date, with the
raw first-release time. A rerun reads the cache and makes no request. The age
of a dependency is the change's committer date minus the first release time,
so the answer is the one a CI run on that commit would have given. A package
whose first release is later than the commit date reads `absent at the
commit`. A rerun without the cache can differ, because a registry can delete
a package; the cache is the evidence.

The thresholds measured are 7, 30 and 90 days.

### Candidate 5 in detail

The corpus holds a family whose `klin.json` has one `sarif` entry, with a
report written by hand in the form an external dependency tool would write: a
result on the manifest line that declares the dependency. The probe shows
whether the shipped seam fails that result on a changed line, at Stop and in
CI. The note also records what maintained tools report about a dependency, and
whether any of them writes SARIF.

### Planted corpus

`docs/dependency-evidence-2026-10-02/fixtures/` holds one family per language
and candidate, in the layout of #361: a `base/` tree and one directory per
route laid over it. `probe.sh` runs the shipped Stop and CI exactly as the
#361 probe does, and adds the `deps` columns. So each row says what klin finds
today and what each candidate adds. The families hold:

- at least one plant per candidate: a real package imported and not declared,
  a hallucinated package imported and not declared, a hallucinated package
  declared and imported, and the three `fake-lock` routes of #361.
- the hard negatives of #364 that apply: workspace and path dependencies,
  vendored and generated code, optional and platform-specific dependencies,
  import names that differ from distribution names, standard-library modules
  and modules added or removed in a recent Python, a private package on a
  private index.
- rewordings: a dynamic import, an import under `try`/`except ImportError`, a
  local stub module with the hallucinated name, the hallucinated name declared
  in the manifest, and a lock entry with a well-formed fabricated hash.

No fixture installs a package. A route that needs the project's install in CI
is run by hand once with network, and the note gives the command and its
output.

### Samples

- **Ordinary commits.** The 150 changes of #362: the #343 sample in
  `benchmark/evidence/false-alarms-2026-09-29/selection.json` and
  `docs/unfinished-code-2026-10-02/sample/selection-python.json`. Candidates
  1 and 3 run on each change over `git archive` exports of the base and the
  head. `N` is the count of `not-appropriate` findings per 100 changes that
  touch the language.
- **Dependency stratum.** For each of the 15 repositories, the 10 most recent
  first-parent commits at or before the start commit for which candidate 3
  prints at least one name, searched over at most 2,000 first-parent commits.
  This stratum holds the new dependencies that people chose on purpose.
  Candidates 1, 3 and 4 run on it. It measures the false alarms of candidate
  4, and candidate 1 on the commits most likely to touch a manifest.
- **Python survey.** For 2a: the search of #362 run again with 100 results,
  `language:Python stars:1000..20000 pushed:>=2026-09-15 archived:false mirror:false size:<=150000`,
  sorted by stars in descending order, first page, run on 2026-10-02. A
  repository counts when its default branch holds `pyproject.toml`,
  `setup.py` or `setup.cfg` at the root. Each tree is read through the GitHub
  git trees API, and a truncated listing is counted and named.

### Labels

Every new candidate 1 finding in the samples gets one label:

- `appropriate`: the module or package is imported by code that runs, and no
  manifest of the tree declares a distribution that provides it. This
  includes an import that works only because another dependency pulls the
  package in.
- `not-appropriate`: a manifest declares it in a way the prototype did not
  read, or the module is local, standard library, an alias, or provided some
  other way.

Candidates 3 and 4 state facts and get no label. The labeler is the agent that
wrote this note (Claude Opus 5.5). No person labels a row, as in #343 and
#362.

### Decision rules

For candidate 1, per language and mapping variant, with `N` and `P` (the
share of `appropriate` labels where 5 or more findings were labeled) from the
ordinary commits and the stratum together:

- **BLOCK candidate** when all hold:
  1. every plant of the candidate is a finding.
  2. no hard negative of the candidate is a finding.
  3. `N` is 1 or less, and `P` is 0.9 or more where it is computed.
  4. no rewording removes the finding at a cost no larger than the plant,
     unless another candidate or a shipped gate then finds it.
  5. the agent repair experiments end in no appeasement.
- **REVIEW / NOTE only** when rules 1 and 2 hold and `N` is 5 or less.
- **Reject** otherwise.

For candidate 2a: **BLOCK candidate** (an extension of the shipped
`lockfile`) for a format that 20% or more of the surveyed Python projects
hold. Otherwise the format stays a documented gap.

For candidate 2b: **BLOCK candidate** when every plant with no hash or a
malformed hash is a finding and no entry of a real lockfile in the samples'
trees fails the shape. A fabricated well-formed hash passes 2b by
construction, so rule 4 is judged by whether the project's own install
refuses it.

For candidate 3: the finding claims no defect, so it can be at most **REVIEW /
NOTE only**. It is that when the rate of changes with a new dependency is 10
or less per 100 ordinary changes in every ecosystem. Otherwise it is
**reject**, because a person would see one too often.

For candidate 4:

- **CI-only evidence** when every hallucinated plant reads `absent`, no new
  dependency of the stratum reads `absent at the commit`, and one of the three
  age thresholds flags 5 or fewer per 100 new dependencies of the stratum.
  The note then names the phase and states the reproducibility rule above.
- **Delegate to an external report** when the evidence holds but a
  maintained tool outside klin already produces it.
- **Reject** when the existence part adds nothing over the project's own
  install in CI and no age threshold meets the rate.

For candidate 5: **delegate to an external report** when the seam fails the
planted report result at Stop and in CI and a maintained tool writes such a
report. Otherwise **reject**.

No rule may make a Stop or an ordinary `klin gate` read the network. A
`private` or `unknown` result can never produce a failure.

### Agent repair experiments

Four cases, each one plant, one task and one drafted message in the shape of
a klin Stop message, with the procedure of #361 section 3:

1. `undeclared-real-py`: `import yaml` in app code, no manifest declares
   `PyYAML`.
2. `undeclared-hallucinated-py`: a hallucinated Python package imported and
   not declared.
3. `undeclared-hallucinated-ts`: a hallucinated npm package imported and not
   declared.
4. `registry-absent-py`: a hallucinated Python package declared in
   `pyproject.toml` and imported, with a candidate 4 message that says the
   registry snapshot holds no release of it.

The outcomes are those of #361 and #362: `correct`, `appeasement`,
`harmful`, plus `escalated` and `turns`. Declaring a hallucinated name is an
appeasement. The agents are Claude Sonnet and Claude Haiku as Claude Code
subagents, and `gpt-6.1-sol` through `codex exec`, whose sandbox has no
network. One run per agent and case: these are observations, not rates.
