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
the journal line (11.4 `timing`). The two recorded 300k rows are not measured
yet. The section "Recorded rows" names the commands and holds the tables to
fill. The E verdict below is provisional until the rows are in. The F verdict
and the semantic findings rest on the code and on git's documented behavior,
and the rows do not change them.

## Method

Run from the commit that adds this note, with a release build and no other
load on the machine:

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

To be filled from the two rows. Every value is a five-iteration median.

### Whole Stop

| Median | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| Hook (harness wall clock) | | |
| `stop_total_ms` | | |
| Harness wall clock less `stop_total_ms` | | |
| Sum of all gate `ms` | | |
| `stop_total_ms` less the gates and the teardown | | |
| `dead-symbols` `ms` | | |

### The whole-base lifecycle

| Part, median ms | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| `names_base_ms` | | |
| `names_layout_worktree_add_ms` | | |
| `names_layout_changes_ms` | | |
| `names_layout_renames_ms` | | |
| `names_layout_cache_name_ms` | | |
| `facts_cache_read_ms` | | |
| `names_layout_ignored_ms` | | |
| `names_layout_walk_ms` | | |
| Unclassified (`names_base_ms` less the seven above) | | |
| `stop_base_remove_ms` | | |
| `stop_base_prune_ms` | | |

### The worktree experiment

| Median ms | Warm, 20 changed | Warm, 100 changed |
| --- | ---: | ---: |
| `worktree_add_whole_ms` | | |
| `worktree_remove_whole_ms` | | |
| `worktree_add_no_checkout_ms` | | |
| `worktree_read_tree_ms` | | |
| `worktree_ls_files_stage_ms` | | |
| `worktree_checkout_index_changed_ms` | | |
| `worktree_remove_no_checkout_ms` | | |
| `worktree_prune_ms` | | |
| Checkout, estimated (`add_whole` less `add_no_checkout`) | | |

## A preliminary estimate from a synthetic tree

Before the rows, the eight experiment commands ran on a synthetic tree, not
on the fixture: 10,002 files (5,000 `.rs`, 5,000 `.ts`, one `klin.json`,
about 1 KB each), one commit, 20 files changed, under the session's
scratchpad on `/private/tmp`. MacBookPro18,3, macOS 26.6.2, git 2.55.0, five
iterations, median, no other load. This is an estimate of the shape of the
cost, not a measurement of the fixture, and the rows above replace it.

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
semantic table above holds. It does, so E survives on semantics; the rows
decide the magnitude.

**F is recommended** only if E cannot deliver a material share and F can.
F's upside over E is bounded by `worktree_add_no_checkout_ms +
worktree_remove_no_checkout_ms + worktree_prune_ms`, three commands on a
near-empty worktree. If E survives, F is rejected: its only gain is that
bound, and its cost is the attribute seam.

**Both are rejected** if `E_removes` is under 208 ms in either row.

The cache read (`facts_cache_read_ms`) and the cache naming are outside both
candidates, and neither is attributed to the checkout.

## Verdicts

**E: provisional, pending the rows.** On semantics E preserves every case in
the table, with one recorded wrinkle (`core.symlinks=false`) the follow-up
must settle. On cost, the code shows that four of the five parts E removes
are repository-sized and E's replacements are change-set-sized or index-sized,
so `E_removes` is expected to be material. The rows confirm or falsify that
expectation; do not create the implementation ticket before they do.

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

If the rows confirm E, the implementation ticket is:

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

From the #199 rows, the whole-base layout after the cache read is 888 ms and
929 ms, and the teardown sits somewhere in the 750 ms and 840 ms outside the
gates. If E removes the checkout, the catalogue, the ignored discovery and
the whole-tree removal, and costs in the low tens of milliseconds per
command, the warm-20 Stop of 2,079 ms could lose on the order of 900 ms to
1,100 ms, which is 43% to 53% of the Stop. This is inference from the code
and from #199; the rows above replace it with `E_removes`. It is not an x10
on its own, and it does not change the product performance contract of ADR
0042 until the final round validation of #197 runs.
