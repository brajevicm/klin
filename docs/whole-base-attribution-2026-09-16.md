# The whole-base layout and the E/F decision (#202)

#199 falsified A and B and left one repository-sized warm cost standing: the
whole base laid out inside `names_base_ms`, 888 ms at warm-20 and 929 ms at
warm-100 after the cache read. #202 asks two questions about that cost:

- **E.** Can a lightweight linked worktree, one that registers the base
  commit without checking the whole tree out, remove a material share of it
  while keeping the base semantics?
- **F.** Failing E, can a commit-backed factual base with no linked worktree
  do so, without a second broad repository abstraction?

This note records the attribution method, the semantic-equivalence analysis,
the decision rule and the verdicts. It implements neither E nor F.

## Status

The attribution counters are in the gate rows (11.2 `names.layout`) and in
the journal line (11.4 `timing`). The two 300k rows are recorded below. E
survives on both rows and on semantics. F is rejected. One implementation
ticket follows, for E.

E landed in #203. "After #203" at the end of this note records the same two
rows over the implemented layout, beside the rows below.

## Method

Run from `2f740bc`, the commit that adds this note, with a release build.
The repository owner ran both rows on MacBookPro18,3 (Apple M1 Pro, macOS
26.6.2), rustc 1.98.1, git 2.55.0, klin 0.1.1, five iterations, median:

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm20 cargo test --release --test performance -- --ignored perf --nocapture
```

```bash
KLIN_PERF_ROW=structural_300k KLIN_PERF_CASE=warm100 cargo test --release --test performance -- --ignored perf --nocapture
```

Each row primes the base, changes 20 files (or 100 files), and times five
warm Stops. It prints the median of each counter. After its stops, a
targeted row also runs the worktree experiment described below. These two
rows are the only rows for this decision. Do not run the 1M row, the cold row
or the strict row. A Stop hook of an agent session that runs on the klin
repository during a row is load, so record it.

Record `git rev-parse HEAD`, `sysctl -n hw.model`, `sw_vers -productVersion`,
`rustc --version` and `git --version` beside the rows.

### Where each part of the whole-base lifecycle goes

The lifecycle of one warm changed Stop is:

```text
Project::whole_base()  (first name-resolving gate, inside its names_base_ms)
  -> tempdir, rev-parse --show-toplevel
  -> git worktree add --detach                  names_layout_worktree_add_ms
  -> project.changes(before)                    names_layout_changes_ms
  -> move_within() for each rename              names_layout_renames_ms
base::unchanged()      (same gate, same timer)
  -> Cache::at(): checkout identity, 3 git      names_layout_cache_name_ms
     processes and the attribute files
  -> cache.read(): read and decode              facts_cache_read_ms
  -> prior.tree().files()
       -> git ls-files --others --ignored       names_layout_ignored_ms
       -> read_dir walk, one stat per entry     names_layout_walk_ms
gates run
Project::teardown_base()  (after the gates, before the journal line)
  -> git worktree remove --force                stop_base_remove_ms
  -> git worktree prune                         stop_base_prune_ms
```

| Ticket part | Counter | Where |
| --- | --- | --- |
| Linked-worktree registration and checkout | `dead-symbols_names_layout_worktree_add_ms` | One git command holds both. The experiment separates them. |
| Base change lookup | `dead-symbols_names_layout_changes_ms` | In the hook the runner computed the change set first, so this is a borrow. |
| Rename normalization | `dead-symbols_names_layout_renames_ms` | The fixture renames nothing, so this is the cost of finding that out. |
| Base file catalogue | `dead-symbols_names_layout_walk_ms` | `read_dir` over the base worktree and one `is_dir` stat per entry. |
| Ignored-path discovery | `dead-symbols_names_layout_ignored_ms` | `git ls-files --others --ignored --exclude-standard --directory` in the base worktree. |
| Structural-cache naming | `dead-symbols_names_layout_cache_name_ms` | `rev-parse --git-common-dir`, `config --get core.attributesFile`, `config --list -z`, and the attribute files read. |
| Structural-cache read and decode | `dead-symbols_facts_cache_read_ms` | The cache file read and decoded. The naming is no longer inside it. |
| `worktree remove --force` | `stop_base_remove_ms` | Journal `timing`. Before this note the removal ran after the journal line, inside the harness wall clock and outside every timer. |
| `worktree prune` | `stop_base_prune_ms` | Journal `timing`. |
| Unclassified whole-base lifecycle | `names_base_ms` less the six layout parts and the cache read | The tempdir, `rev-parse --show-toplevel`, `create_dir_all`, the changed-path set in `keep`, and the `Unchanged` view. |

The row that lays the base out carries `names.layout`. The other
name-resolving row carries null there, as it carries 0 in `names_base_ms`.

The harness wall clock (`median_ms`) less `stop_total_ms` is the process
start, the configuration load and the exit. `stop_total_ms` now holds the
worktree removal, which it did not before this note, so `klin_ms` of a row
recorded before this commit and after it are not the same measure.

### The worktree experiment

A targeted row also times, on the fixture's repository and apart from every
Stop, the git commands the candidate E would run, five times each, median:

| Counter | Command | Role |
| --- | --- | --- |
| `worktree_add_whole_ms` | `git worktree add --detach --quiet DIR HEAD` | What klin runs today: registration and the whole checkout. |
| `worktree_remove_whole_ms` | `git worktree remove --force DIR` | What klin runs today at teardown. |
| `worktree_add_no_checkout_ms` | `git worktree add --detach --no-checkout --quiet DIR HEAD` | E: registration alone. |
| `worktree_read_tree_ms` | `git -C DIR read-tree HEAD` | E: the base commit's index, with no file written. |
| `worktree_ls_files_stage_ms` | `git -C DIR ls-files -z --stage` | E: the path catalogue with modes, from the index. |
| `worktree_checkout_index_changed_ms` | `git -C DIR checkout-index -f -- CHANGED...` | E: the changed files' base bytes, checkout-equivalent. |
| `worktree_remove_no_checkout_ms` | `git worktree remove --force DIR` | E: teardown of a near-empty worktree. |
| `worktree_prune_ms` | `git worktree prune` | Both. |

Registration is estimated as `worktree_add_no_checkout_ms`, and the checkout
as `worktree_add_whole_ms` less that. These are estimates from separate git
commands, not measurements inside klin, and the note labels them so.

## Recorded rows

Every value is a five-iteration median. A part's median and a total's median
come from different Stops, so a difference between them is approximate.

### Whole Stop

| Median | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| Hook (harness wall clock) | 2,279 ms | 2,538 ms |
| `stop_total_ms` | 2,245 ms | 2,495 ms |
| Harness wall clock less `stop_total_ms` | 34 ms | 43 ms |
| Sum of all gate `ms` | 1,430 ms | 1,609 ms |
| Teardown (`stop_base_remove_ms` + `stop_base_prune_ms`) | 613 ms | 639 ms |
| `stop_total_ms` less the gates and the teardown | 202 ms | 247 ms |
| `dead-symbols` `ms` | 1,130 ms | 1,243 ms |

The #199 rows put 750 ms and 840 ms "outside the gates" with no name. The
teardown is 613 ms and 639 ms of that. The rest, 202 ms and 247 ms, is the
lock, the turn window, the change set, the scoped base, the survey and the
journal, and it grows with the change set.

### The whole-base lifecycle

| Part, median ms | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| `names_base_ms` | 1,042 | 1,107 |
| `names_layout_worktree_add_ms` | 926 | 995 |
| `names_layout_changes_ms` | 0 | 0 |
| `names_layout_renames_ms` | 0 | 0 |
| `names_layout_cache_name_ms` | 18 | 19 |
| `facts_cache_read_ms` | 34 | 35 |
| `names_layout_ignored_ms` | 13 | 14 |
| `names_layout_walk_ms` | 38 | 37 |
| Unclassified (`names_base_ms` less the seven above) | 13 | 7 |
| `stop_base_remove_ms` | 607 | 632 |
| `stop_base_prune_ms` | 6 | 7 |

`git worktree add` is 89% and 90% of `names_base_ms`. The file catalogue
(`walk` plus `ignored`) is 51 ms in both rows. The cache naming and read
together are 52 ms and 54 ms, which is the 58 ms and 57 ms the #199 rows
called `facts_cache_read_ms` before the naming moved out of it. Nothing in
the lifecycle grows with the change set except `worktree add` itself, by
69 ms, which the experiment's whole checkout also shows (974 ms to 1,089 ms),
so that growth is run-to-run variance of the checkout, not the change set.

### The worktree experiment

| Median ms | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| `worktree_add_whole_ms` | 974 | 1,089 |
| `worktree_remove_whole_ms` | 607 | 629 |
| `worktree_add_no_checkout_ms` | 10 | 11 |
| `worktree_read_tree_ms` | 14 | 14 |
| `worktree_ls_files_stage_ms` | 9 | 9 |
| `worktree_checkout_index_changed_ms` | 10 | 21 |
| `worktree_remove_no_checkout_ms` | 9 | 15 |
| `worktree_prune_ms` | 6 | 11 |
| Checkout, estimated (`add_whole` less `add_no_checkout`) | 964 | 1,078 |

Registration is about 10 ms; the checkout is the rest. The experiment's whole
`worktree add` (974 ms, 1,089 ms) and removal (607 ms, 629 ms) agree with
klin's own timers (926 ms, 995 ms; 607 ms, 632 ms), so the timers and the
experiment measure the same work. `checkout-index` of the changed files
grows from 10 ms at 20 files to 21 ms at 100, the one change-set-sized part
of E.

### The decision rule applied

| | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| `today_E` (`worktree_add` + `walk` + `ignored` + `remove` + `prune`) | 926 + 38 + 13 + 607 + 6 = 1,590 ms | 995 + 37 + 14 + 632 + 7 = 1,685 ms |
| `E_cost` (six E commands) | 10 + 14 + 9 + 10 + 9 + 6 = 58 ms | 11 + 14 + 9 + 21 + 15 + 11 = 81 ms |
| `E_removes` | 1,532 ms | 1,604 ms |
| Share of the Stop | 67% | 63% |
| Threshold | 208 ms | 208 ms |

Of the 888 ms and 929 ms that #199 left standing, this run's equivalent is
1,008 ms and 1,072 ms (`names_base_ms` less `facts_cache_read_ms`). Of that,
`worktree add`, the walk and the ignored discovery are 977 ms and 1,046 ms,
and E removes them. The cache naming (18 ms), the unclassified remainder
(13 ms, 7 ms) and the cache read (34 ms, 35 ms) stay. The teardown, 613 ms
and 639 ms, was outside every #199 timer and E removes it too.

## A preliminary estimate from a synthetic tree

Before the rows, the eight experiment commands ran on a synthetic tree, not
on the fixture. The rows above supersede it; it stays as the record of what
was known when the experiment was designed. The tree: 10,002 files (5,000 `.rs`, 5,000 `.ts`, one `klin.json`,
about 1 KB each), one commit, 20 files changed, under the session's
scratchpad on `/private/tmp`. MacBookPro18,3, macOS 26.6.2, git 2.55.0, five
iterations, median, no other load. This is an estimate of the shape of the
cost, not a measurement of the fixture.

| Command | Median ms |
| --- | ---: |
| `worktree add --detach` (whole) | 1,136 |
| `ls-files --others --ignored` in the whole worktree | 13 |
| A Python `os.walk` of the whole worktree, for scale | 5 |
| `worktree remove --force` (whole) | 514 |
| `worktree add --detach --no-checkout` | 13 |
| `read-tree HEAD` | 13 |
| `ls-files -z --stage` | 10 |
| `checkout-index -f -- ` 20 changed paths | 10 |
| `worktree remove --force` (no checkout) | 10 |
| `worktree prune` | 7 |

On this tree the whole checkout and its removal are 1,650 ms, and the six E
commands are 63 ms. The synthetic files are smaller than the fixture's and
the volume differs, so the fixture's numbers will differ; the ratio is the
point. The ignored-path discovery and the walk are small here, so the
repository-sized cost is the checkout and the removal, which is what E and
the teardown timer target.

## What the code shows about the parts

This section comes from the code and from git's documented behavior, not
from a measurement.

- **`git worktree add --detach` writes every file of the commit.** The fixture
  holds about 10,000 source files, so the checkout is repository-sized. It is
  the only part of the lifecycle that writes source bytes to disk.
- **The walk stats every entry.** `files::visit` calls `path.is_dir()` on
  each entry, so the catalogue costs one `read_dir` per directory and one
  stat per file, repository-sized.
- **Ignored-path discovery in the base finds nothing, by construction.**
  `git ls-files --others --ignored` lists untracked paths that an ignore rule
  matches. A worktree git has just checked out holds tracked files only, and
  a tracked file is never `--others`, whatever `.gitignore` says. The base's
  ignored set is therefore empty on every run, and the process still reads
  the index and scans the tree to say so. This is the one part a follow-up
  can remove with no semantic question at all.
- **The cache naming runs three git processes.** `checkout()` asks for the
  common directory, `core.attributesFile` and the whole configuration, then
  reads the attribute files. It does not scale with the repository, but each
  process is a fork and an exec.
- **The teardown deletes every file the checkout wrote.** `worktree remove
  --force` removes the directory tree, repository-sized, and `prune` walks
  the `worktrees` directory, small. Before this note, both ran when `Project`
  dropped, after the journal line, so the #199 "hook outside the gates" of
  750 ms and 840 ms held them without a name.
- **Between 20 and 100 changed files, none of these parts changes size.**
  Only `checkout_index_changed` in the experiment and the renamed and changed
  sets scale with the change set.

## Candidate E, as the code allows it

E keeps a real linked worktree registered for the base commit and writes only
the files a warm changed run reads from the base:

```text
git worktree add --detach --no-checkout DIR BASE      registration
git -C DIR read-tree BASE                             the base's index, no files
git -C DIR ls-files -z --stage                        catalogue: path and mode
rename mapping over the catalogue                     was -> path, as move_within does
git -C DIR checkout-index -f -- PATHS                 klin.json, every Cargo.toml and
                                                      package.json, every changed or
                                                      deleted file's base path, and
                                                      every structural file the cache
                                                      does not hold
move_within() for each rename                         as today
Tree::listed(root, catalogue)                         the file list, no walk, no
                                                      ignored discovery
```

What each existing seam does under E:

- `Project::whole_base()` decides E or the full checkout from the run: a
  changed run that is not strict, with a state directory. Every other run
  takes today's path.
- `base::materialize()` gains the E layout beside `checked_out()`.
- `git::Repo` gains `read_tree`, `ls_files_stage` and `checkout_index`, three
  narrow commands in the one process seam of ADR 0041.
- `project::Tree` gains one constructor that takes a file list instead of
  walking. `files()` returns it. `covers()`, `linked()` and `extracted()` are
  unchanged.
- `base::unchanged()` computes, after `cache.read()`, the structural files the
  catalogue holds and the cache does not, less the changed paths, and
  materializes them with one `checkout-index` before any gate measures. On a
  warm hit this set is empty; on a cold cache it is the whole tree, which is
  the full checkout by another route, so a cold cache takes today's path
  instead (see "Fallback").
- `Extracted::outcome()`, `Topology`, `Scope::at_base`, `measure_all`,
  `dead-symbols`, `reachability`, `layering` and `public-api` are unchanged.
  They read a root directory and a file list, and E gives them both.

What E costs: registration, `read-tree`, `ls-files`, one `checkout-index` of
a change-set-sized path list, and the removal of a near-empty worktree. The
experiment measures each on the fixture.

Blast radius: `base.rs`, `project.rs`, `git.rs`, and the `Unchanged`
construction in `syntax/structural/mod.rs`. No gate changes. No new module.

## Candidate F, as the code allows it

F drops the linked worktree on the warm path and builds the same Prior root
from the main repository:

```text
GIT_INDEX_FILE=TMP git read-tree BASE                 a private index
git ls-files -z --stage (with that index)             catalogue
git checkout-index --prefix=DIR/ -f -- PATHS          the same selected bytes
```

or reads the bytes with `git cat-file --batch --filters`, which klin's `blobs`
seam already resembles.

F saves, over E, the registration and the removal of a near-empty worktree:
`worktree_add_no_checkout_ms` plus `worktree_remove_no_checkout_ms` plus
`worktree_prune_ms`, less the private-index bookkeeping F does instead. That
is the whole of F's measurable upside.

F's cost is one semantic seam E does not open, described under "Checkout
attributes" below.

## Semantic equivalence

The whole base a gate measures today is a directory git checked out, moved
renamed files within, and walked. Each case below says what the current tree
does, and whether E and F give the same findings, notes, coverage and exit
codes.

| Case | Today | E | F |
| --- | --- | --- | --- |
| Historical `klin.json`, base scope | `Scope::at_base` reads `prior.root()/klin.json` | Materialized by name from the catalogue. Same bytes. | Same. |
| Base `Cargo.toml`, Rust targets | `Topology::read` reads any manifest on demand from the base root | Every `Cargo.toml` in the catalogue is materialized. Bounded count. | Same. |
| Base `package.json`, TypeScript entries | Same, `package.json` | Every `package.json` materialized. | Same. |
| Manifest changes and renames | Changed manifests are in the change set; `move_within` moves a renamed one | In the change set, so materialized at the base path, then moved as today. | Same. |
| Same-extension rename | `move_within(was, path)`; the cache excludes `was` and `path`, so the base file is extracted from disk at `path` | The catalogue maps `was` to `path`; the base bytes are materialized at `was` and moved. | Same. |
| Extension-changing rename | As above; the moved file may leave the structural extensions | Same mapping; the file is then unselected, as today. | Same. |
| Case-only rename git does not report | No `Change`, no move; the base lists the old spelling, the working tree the new; `Unchanged::copy` misses and extracts the working file | The index lists the old spelling, the same as the checked-out name. Same outcome. | Same. |
| Addition | Not in the base | Not in the catalogue. | Same. |
| Deletion | `was` is set; the base file is selected from the base list and extracted from disk | In the change set, so materialized. | Same. |
| Base source unsupported after normalization | The moved file matches no adapter or extension | Same, after the same mapping. | Same. |
| Configuration root below the git top level | `Prior::new(dir.join(inside))`; `Repo::at(root)` runs git in that subdirectory | `create_dir_all(inside)` as today; `ls-files` run in the subdirectory lists that subtree, relative to it, as `survey::listed` relies on. | Same, with `--prefix`. |
| Default skip directories | The walk does not descend a directory named in the skip set | Filter every catalogue path whose segments hit `files::skipped`. One shared predicate. | Same. |
| Git-ignored files | Empty by construction in a fresh checkout (see above) | No discovery. Provably the same set. | Same. |
| Symlinks | The walk skips a symlink entry and never descends one | Mode `120000` entries are dropped. A file below a symlinked directory is not in the tree at all. **Wrinkle:** with `core.symlinks=false`, git checks a symlink out as a plain file holding the target path, and today's walk lists it. E must keep `120000` entries when `core.symlinks` is false to match, or accept that one difference. Record it in the follow-up. | Same wrinkle. |
| Submodules | An uninitialized submodule is an empty directory: no files | Mode `160000` entries are dropped. | Same. |
| Checkout attributes | `worktree add` reads `.gitattributes` from the base commit's index first (checkout direction), then `info/attributes` and the global file | `checkout-index` in a worktree with no `.gitattributes` on disk falls back to the index, which `read-tree` filled from the base commit. Same precedence. | **Differs.** `checkout-index` or `cat-file --filters` in the main repository reads the main working tree's `.gitattributes` first. When `.gitattributes` changed between the base and now, the base bytes are converted by today's rules. |
| Smudge filters | Applied at checkout, under the attributes above | Applied by `checkout-index`. The `structural_cache` test that rewrites `src/caller.rs` through a smudge filter holds. | Applied, under today's attributes. |
| Line-ending conversion | `core.autocrlf`, `eol` and `text` at checkout | Same conversion by `checkout-index`. | Same conversion, today's attributes. |
| Missing, corrupt or incompatible cache | `cache.read()` is `None`; every base file is extracted from disk | The materialization set is the whole tree, so E takes today's path (see "Fallback"). Same bytes. | Same. |
| Layering, module-graph base topology | `Topology::new(prior.root(), prior.tree().files(), facts, renamed)` | The catalogue is the file list; manifests are on disk. | Same. |
| Public-api base surfaces | Manifest entries plus facts | Same. | Same. |
| Dead-symbol and reachability before/after | Facts from the cache or from disk; `Scope::at_base` | Same facts; the same misses materialized. | Same. |
| Strict, whole and by-hand runs | Full checkout | Unchanged, today's path. | Unchanged. |

Two findings decide E against F:

1. **Every case E must handle is a path-list computation over the catalogue
   plus one `checkout-index`.** Nothing in E changes what a gate reads; it
   changes which files exist under the same root before the gate reads them.
2. **F cannot reproduce checkout-direction attribute precedence without a
   worktree whose on-disk attributes are the base's.** Registering an empty
   worktree is exactly that, and it is E. F could fall back to the full
   checkout whenever `.gitattributes` is in the change set, but that is a
   rule about bytes klin would otherwise get wrong, where E has no such rule.

The ticket's requirement that a commit-backed read preserve
checkout-equivalent bytes is therefore met by E through git itself, and by F
only by adding the worktree back.

## Fallback

Optimization state affects cost, never the verdict, as with the structural
cache. E takes its path only when all of these hold:

- the run is changed and not strict (`at.changes` is `Some`, `at.strict` is
  false), so a cache may serve the base;
- the state directory exists and `Cache::at` names a cache;
- `cache.read()` returns outcomes, so the materialization set is
  change-set-sized rather than the tree;
- every git command of the E layout succeeds.

Otherwise `checked_out()` runs as it does today, on the same tempdir, and
the run continues with no other difference. A failure inside E after
registration removes the worktree and takes the full path; it never leaves a
half-laid base for a gate.

```text
changed, not strict, cache named and read, E commands succeed
    -> E layout
otherwise
    -> today's worktree add --detach
```

Strict, whole and by-hand runs never enter E.

## Decision rule

"Material" means at least 10% of the warm-20 hook median of #199, 208 ms,
the same threshold as #199.

The cost E removes is what today's lifecycle spends on the whole checkout,
the catalogue and the teardown, less what E's own commands cost:

```text
today_E   = names_layout_worktree_add_ms + names_layout_walk_ms
          + names_layout_ignored_ms + stop_base_remove_ms + stop_base_prune_ms

E_cost    = worktree_add_no_checkout_ms + worktree_read_tree_ms
          + worktree_ls_files_stage_ms + worktree_checkout_index_changed_ms
          + worktree_remove_no_checkout_ms + worktree_prune_ms

E_removes = today_E − E_cost
```

`E_cost` comes from separate git processes on an idle repository, so it is a
lower bound on what klin would pay, and `E_removes` an upper bound. Both are
inference.

**E survives** if `E_removes` is at least 208 ms in both rows and the
semantic table above holds.

**F is recommended** only if E cannot deliver a material share and F can.
F's upside over E is bounded by `worktree_add_no_checkout_ms +
worktree_remove_no_checkout_ms + worktree_prune_ms`, three commands on a
near-empty worktree. If E survives, F is rejected: its only gain is that
bound, and its cost is the attribute seam.

**Both are rejected** if `E_removes` is under 208 ms in either row.

The cache read (`facts_cache_read_ms`) and the cache naming are outside both
candidates, and neither is attributed to the checkout.

## Verdicts

**E: survives.** `E_removes` is 1,532 ms at warm-20 and 1,604 ms at
warm-100, seven times the 208 ms threshold, 67% and 63% of the Stop. Every
part it removes is repository-sized and flat between the rows; its own cost
is 58 ms and 81 ms and grows only in the `checkout-index` of the changed
files. On semantics E preserves every case in the table, with one recorded
wrinkle (`core.symlinks=false`) the follow-up must settle. Blast radius:
`base.rs`, `project.rs`, `git.rs` and the `Unchanged` construction, no gate.
The implementation ticket for E is #203.

**F: rejected.** F is not smaller than E in any way that matters: its
measurable upside over E is three git commands on an empty worktree, and it
loses checkout-equivalent bytes whenever `.gitattributes` differs between the
base and the working tree. It also needs a private index and a `--prefix`
layout, which is E's worktree by another name. Do not create a ticket for F.

**Outside E and F.** The ignored-path discovery in the base is empty by
construction. Even if E fails the threshold, removing that one process from a
whole-base checkout is a semantics-free change worth its own line in any
follow-up.

## The smallest surviving direction

The implementation ticket, #203, is:

- `Repo::read_tree(dir, commit)`, `Repo::ls_files_stage(dir)` and
  `Repo::checkout_index(dir, paths)` in `src/git.rs`, and nothing else that
  launches git.
- `base::light(project, before, changes, cache)` beside `checked_out`, chosen
  by `Project::whole_base` under the fallback rule, returning the same
  `Prior`. `Prior` keeps its `Drop`, its `root()`, its `tree()` and its
  `teardown()`.
- `Tree::listed(root, files)` in `src/project.rs`, one constructor.
- The materialization set computed once in `base::unchanged()` after the
  cache read, and written with one `checkout-index`.
- The `structural_cache`, `structural_views` and `structural` CLI tests
  extended with a rename, a deletion, a manifest change, a `.gitattributes`
  change, a smudge filter and a symlink, each run warm, each compared with
  the same run over a removed cache.
- The two targeted rows again, with `names_layout_*` and `stop_base_*`
  showing the removed parts.

No virtual filesystem, no provider trait, no persistent index, no daemon.

## Upper-bound expected improvement (inference)

`E_removes` is the upper bound: 1,532 ms of the 2,279 ms warm-20 Stop and
1,604 ms of the 2,538 ms warm-100 Stop, so a Stop of about 750 ms and about
930 ms if E's commands cost inside klin what they cost in the experiment.
That is inference, for two reasons: `E_cost` comes from separate git
processes on an idle repository, so klin's own process launches and path
handling add to it, and the experiment's `checkout-index` wrote only the
changed files, where E also writes `klin.json`, the manifests and any file
the cache lacks. A Stop of about 750 ms is a 3x improvement, not an x10 on
its own, and it does not change the product performance contract of ADR 0042
until the final round validation of #197 runs.

## After #203

#203 implemented E. This section records the same two rows over the
implemented layout, so the before and after medians sit beside each other.

### Method

Run from `783e21e`, the commit that lays the whole base out from the base
commit's index, with a release build. The repository owner ran both rows,
macos/aarch64, rustc 1.98.1, klin 0.1.1, five iterations, median. The rows
are the same two commands as above. The fixture digest is unchanged
(`f62c3dae3eaff7ab`), and the structural cache is one file of 11,850,967
bytes at warm-20 and 11,850,966 at warm-100, as before.

`before` is the row recorded above, from `2f740bc`. `after` is this run.

### Whole Stop

| Median, ms | 20 before | 20 after | 100 before | 100 after |
| --- | ---: | ---: | ---: | ---: |
| Hook (harness wall clock) | 2,279 | 687 | 2,538 | 825 |
| `stop_total_ms` | 2,245 | 655 | 2,495 | 794 |
| Sum of all gate `ms` | 1,430 | 492 | 1,609 | 585 |
| Teardown (`stop_base_remove_ms` + `stop_base_prune_ms`) | 613 | 14 | 639 | 17 |
| `stop_total_ms` less the gates and the teardown | 202 | 149 | 247 | 192 |
| `dead-symbols` `ms` | 1,130 | 207 | 1,243 | 249 |

### The whole-base lifecycle

| Part, median ms | 20 before | 20 after | 100 before | 100 after |
| --- | ---: | ---: | ---: | ---: |
| `names_base_ms` | 1,042 | 117 | 1,107 | 119 |
| `names_layout_worktree_add_ms` | 926 | 58 | 995 | 63 |
| `names_layout_written` | null | 27 | null | 107 |
| `names_layout_changes_ms` | 0 | 0 | 0 | 0 |
| `names_layout_renames_ms` | 0 | 0 | 0 | 0 |
| `names_layout_cache_name_ms` | 18 | 17 | 19 | 16 |
| `facts_cache_read_ms` | 34 | 32 | 35 | 31 |
| `names_layout_ignored_ms` | 13 | 0 | 14 | 0 |
| `names_layout_walk_ms` | 38 | 0 | 37 | 0 |
| Unclassified (`names_base_ms` less the seven above) | 13 | 10 | 7 | 9 |
| `stop_base_remove_ms` | 607 | 9 | 632 | 12 |
| `stop_base_prune_ms` | 6 | 5 | 7 | 5 |

`names_layout_worktree_add_ms` now holds four git commands rather than one
checkout: `worktree add --no-checkout`, `read-tree`, `ls-files --stage` and
one `checkout-index`. `names_layout_written` is 27 where 20 files changed and
107 where 100 did, so the layout writes the change set plus a constant seven
paths, and not the repository. `walk` and `ignored` are zero on both rows,
because the file list comes from the index and no `ls-files --others
--ignored` runs in the base.

### The decision rule applied, measured

| | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| `today_E` before | 1,590 ms | 1,685 ms |
| The same five counters after | 58 + 0 + 0 + 9 + 5 = 72 ms | 63 + 0 + 0 + 12 + 5 = 80 ms |
| Measured removal | 1,518 ms | 1,605 ms |
| #202's predicted `E_removes` | 1,532 ms | 1,604 ms |

The prediction was an upper bound from separate git processes on an idle
repository. The implemented layout removed 1,518 ms of a predicted 1,532 at
warm-20, 99% of the bound, and 1,605 ms of a predicted 1,604 at warm-100.

The whole Stop fell by more than the layout did: 1,592 ms at warm-20 and
1,713 ms at warm-100, against the 1,518 ms and 1,605 ms above. Of that
difference, 53 ms and 55 ms sit in `stop_total_ms` less the gates and the
teardown, and the rest is spread over the other gates and the harness wall
clock. This note does not attribute either part. Both may be run-to-run
variance, and neither is claimed as an effect of the layout.

### The worktree experiment, this run

| Median ms | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| `worktree_add_whole_ms` | 948 | 927 |
| `worktree_remove_whole_ms` | 508 | 581 |
| `worktree_add_no_checkout_ms` | 10 | 9 |
| `worktree_read_tree_ms` | 12 | 12 |
| `worktree_ls_files_stage_ms` | 8 | 8 |
| `worktree_checkout_index_changed_ms` | 9 | 18 |
| `worktree_remove_no_checkout_ms` | 12 | 12 |
| `worktree_prune_ms` | 6 | 6 |
| The six commands of candidate E | 57 | 65 |

klin's own five counters cost 72 ms and 80 ms against the experiment's 57 ms
and 65 ms, 15 ms more on both rows. #202 predicted that gap: klin adds its own
process launches and path handling, and it writes 27 and 107 paths where the
experiment wrote 20 and 100.

### Verdict

A warm changed Stop on the 300k dense fixture is 687 ms where it was
2,279 ms, and 825 ms where it was 2,538 ms. That is 3.3 times and 3.1 times
faster, and it beats the 750 ms and 930 ms the note called an upper-bound
prediction, because parts of the Stop outside E also fell.

This is not an x10 on its own, and it does not change the product performance
contract of ADR 0042 until the final round validation of #197 runs. No 1M row
and no RSS measurement was taken for #203.
