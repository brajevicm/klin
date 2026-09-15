# Config is policy, the tree is fact, and the runner composes them

Before this, every check loaded `klin.json` for itself: the runner read it
once to plan, and each gate's `Config::load` read it again, validated it
again, and surveyed the tree again for the sections the file left out, because
the survey lived behind a cell inside `Config`. Each gate then walked its own
roots, and asked git once per root what it ignores, so a section naming a
hundred source areas paid a hundred walks and a hundred processes. The survey
asked the `escapes` check which files were source. A top-level `gates` list
let a person run one check twice under two names, and the runner carried a
per-gate section override through every check to make that work. Whether a
check ran without its section was read off whether the survey derived
anything for it, which made `conventions` and `sarif` look alike when they
are not.

Issue #159 is the expand step of the compact `klin.json` work in #179 and
#180: the foundation those migrations stand on, with the checks' user-visible
behaviour unchanged.

## The decision

Three things, each with one owner.

**`config::Config` is the policy a person wrote.** It holds the file, the
values and the validation of section 14, and nothing klin computed. It no
longer derives, says or holds a survey.

**`project::Tree` is one tree's facts.** The working tree and the base laid
out beside it are two trees. A tree reads its file list once: one walk from
its root, one `git ls-files` for what git ignores, every path relative and
sorted. `files::found` selects a section's roots out of that list in memory,
so root count adds a filter and not a walk. A root the list did not reach —
outside the tree, or under a directory every walk skips — is walked on its
own, as every root once was. Which files are source is a table of extensions
in `project`, and no check's opinion.

**`project::Project` is one run.** It owns the one `Config`, the working
`Tree`, the changed set against the base and what the survey derives, each
computed on the first call that needs it and never again. The runner builds
one and every check borrows it through `check::Context`. A hook's build phase
and its gates read the same one. A command a person runs by hand builds one
of its own.

### What the survey derives is derived per section

`survey::Derived` reads the two trees' facts once and holds one cell per
derivable section and one per expensive number: the complexity sample of the
derivation commit, the document ceilings, the reachability families. A run
that names one gate derives that gate's section and no other's, and prints
the `derived:` lines of that section alone. A run of every gate derives and
prints what it did before. Planning a run asks only whether the survey
supplies a section, which the facts answer without a number.

Planning a run asks only whether the survey supplies a section, which the
facts answer for every section but `reachability`, whose families only the
derivation commit proves and which are cached under it.

In the hook, the build phase and the gates read the same `Project`. A build
whose `build` section the survey derives reads the working tree's file list
before the build command runs, and the gates then select from that list. What
a build writes belongs under `.gitignore` (8.3), which the list never holds,
and a file's contents are read when a gate reads it, so a build that rewrites
tracked files is judged as it left them.

### Activation is a row of the catalogue

`check::Activation` says what a section's absence means: Automatic derives,
Policy runs nothing, Integration runs nothing. `derives` stays on the row and
says which keys the survey supplies, which is a different question.

### The `gates` list is gone

A check that runs as several gates holds them in its own section, as the
named entries of `sarif` and the named conventions already do. With no list
there is no per-gate section override, so `Context.with` and
`Config::load_with` are gone with it, and `sarif` finds its entry by the name
the runner gave the gate.

### The scope contract is one module

`scope::Selector` is the `in` and `except` path #42 proved: repository
relative, a string or a non-empty list, itself and everything below it, `./`
and a trailing slash normalized, absolute, escaping and glob-shaped paths
refused, and `.` the root. Conventions reads it from there. What each check
does with a path in `before` against `after` stays the check's own.

## What this is not

No container, no trait with one implementation, no plugin, no event bus and
no concurrency. `OnceCell` holds the lazy facts because the run is one
thread. The survey's section derivations stay in `survey` as the
compatibility layer #179 and #180 remove; no registry replaces it. Nothing
here caches across runs, and no tree is parsed because a `Tree` exists.

## Consequences

- One configuration lifetime per run. A check reads `at.project` and never
  loads.
- The 2,000-file fixture split over 2, 100 and 500 source areas runs in the
  same time, where before each area was a walk and a git process. Measured
  on 2026-09-13, macos/aarch64, whole-tree strict, median of five:

  | source areas | before | after |
  |---|---|---|
  | 2 | 1,627 ms | 1,218 ms |
  | 100 | 9,031 ms | 1,278 ms |
  | 500 | 39,668 ms | 1,377 ms |

- The #157 rows on the same machine and day, before and after: 2k warm hook
  1,608 to 1,113 ms; 2k cold survey 2,340 to 1,791 ms; 2k strict 1,648 to
  1,158 ms; 10k warm hook 5,067 to 4,300 ms; 10k cold survey 14,537 to
  13,769 ms; 10k strict 6,366 to 5,391 ms. In the cold rows the complexity
  gate's own `ms` grew, because the sample of the derivation commit is now
  taken when that gate reads its section and not before any gate is timed;
  the run's total fell.
- `git` process consolidation beyond this is #161's.

## Follow-up: compact source policy (#179)

The compatibility layer no longer derives configuration-shaped source
sections. `complexity`, `escapes`, `stubs`, `dead_symbols` and `reachability`
discover their supported files from `Project` facts and own the meaning and
provenance of their policy. Their optional objects contain only human
decisions: `in` / `except`, the two complexity ceilings,
`skip_rust_tests`, or `ignore`, as applicable. Reachability families and
complexity ceilings remain lazy, check-owned derivations. The runner plans
these Automatic checks from the presence of source facts without computing
either one, and no central source-policy registry replaces the removed
survey sections.

## Follow-up: one extraction per tree (#176)

`dead-symbols` and `reachability` each read, parsed and extracted every
structural file of both trees, so one run extracted each file four times.
Under `--changed`, each of the two checks also laid the whole base out again.
On the 1M-line fixture of spec 13 those extractions were most of both gates'
time.

A `Tree` now holds `structural::Extracted`: the outcome each file came to,
extracted when a check first asks for that file, and held until the tree
drops at the end of the run. `structural::measure` takes the tree and not a
root, so the extraction a measurement reads is always the extraction of the
tree it names. `Project::whole_base` lays the base out whole once, for the
checks that resolve names against the whole base while the runner lays out a
scoped one, and `base::whole` chooses between that checkout and the runner's
own tree.

The per-file outcome is shared, and nothing above it. Each check still selects
its own files under its own scope and builds its own `SourceIndex` from them,
so a file one check excepts never resolves a name for that check because
another check read the file. The facts sit behind an `Rc`, which the tree and
every index built over them hold. The store is a `RefCell` map and not a
`OnceCell`, because it fills one file at a time as checks ask for files, where
a tree's file list fills once. Nothing outlives the run, and no provider,
registry or service came with this: the tree already owned what a run knows
about one set of files.

A gate's row records what it extracted and what it shared under `facts`, with
the time of its own extractions, so a benchmark row separates extraction from
indexing and judging (spec 11.2, 13).

### The parse is not shared

`complexity`, `stubs`, `escapes` and a `conventions` code rule each walk a
Tree-sitter tree under a policy of their own, and no extracted fact replaces
that walk. Sharing their parse means keeping trees alive from one gate to the
next. On the 1M-line fixture, the peak resident memory of a strict run of
`complexity` alone was 63 MB when it dropped each tree and 1,842 MB when it
kept every tree of both trees, and the whole strict run peaked at 1,113 MB
before this change. So a parse is dropped once its facts are extracted, and
those checks keep their own parse.

## Follow-up: one base extraction for both trees (#190)

After #176 and #183, `dead-symbols` still extracted every structural file of
the working tree and every structural file of the base, although a warm
Stop changes a few files. `structural::Unchanged` holds the whole base tree,
its file list and the changed run's `Change` set. A working-tree file that
set does not name, and that the base lists under the same name, holds the
base's bytes at the same path, by git's word. The working tree's measurement
takes that file's outcome from the base tree's `Extracted` and not from its
own. Any other file is extracted from the working tree. `dead-symbols`
measures both trees this way, so an unchanged file is extracted once for the
comparison.

Only a changed run that is not strict does this, which includes the hook. It
reads the change set from `Context.changes`, the seam #188 made for it, and
never from `Context.only`. A strict run, a whole run and the check by hand
extract both trees as before, because git's word is only as good as its view
of the index: an `assume-unchanged` or `skip-worktree` file, or bytes a clean
filter hides, reads as unchanged. A changed run already takes git's word for
its scope, so it takes it for the facts too. Strict and whole verification
keep reading every byte. The base list, and not a stat of the base copy,
decides whether the base holds the file, so on a case-insensitive disk a file
renamed by case alone, which git reports under its old name, is read from
the working tree.

The base keeps each renamed file at its current path, so an
extension-changing rename reads the old bytes under the new path's grammar,
as before. The file set of each tree is still that tree's listing under that
tree's scope, each measurement still builds its own `SourceIndex`, and the
facts stay behind an `Rc` and die with the run.

## Follow-up: reachability over one base extraction (#191)

The same `structural::Unchanged` view now serves `reachability`. In a changed
run that is not strict, it takes the base extraction for every working-tree
file the change set leaves out and the base lists under the same name. A run
of both structural gates therefore extracts each unchanged file once for the
comparison. Strict runs, whole runs and the check by hand keep extracting
both trees, and each check still builds its own index from the files it
selected.

## Follow-up: the base's outcomes kept between runs (#192)

After #191 a warm Stop still extracted every structural file of the base once
per Stop, although the turn's base is one immutable commit for the whole
turn. `structural::Cache` keeps the outcomes of that commit's base tree in the
state directory, one file per commit, and the base tree's `Extracted` takes an
outcome from it the first time a check asks for the file. The cache is the
one thing here that outlives a run, and it holds only what `Extracted` already
held: per-file outcomes without parse trees. No `SourceIndex`, family state
or other derived view is kept, because each is cheap to build again from the
facts.

The key is the full commit id, and the file carries an explicit epoch, the
binary version, a checksum of the extraction sources built into the binary,
the commit, and a checksum of the configuration's root, `git config --list`
and the attribute files git reads outside the tree. Those settings decide
the bytes `git worktree add` writes for the base, so a filter or line-ending
change names another cache. Only the checksum is stored, never a
configuration value. A development build whose extraction
code differs therefore reads another build's cache as nothing, without
anyone raising the epoch. The body has its own checksum and must decode to
its last byte. Any doubt reads as no cache, and the run extracts the base as
before, so a torn write or a copied file costs time and never a verdict.

The cache is read and written only where `Unchanged` is used: a changed run
that is not strict. A path the change set names, as `path` or as `was`, is
never taken from the cache and never written to it, because the base tree
may hold renamed bytes there. The file is replaced whole through
`write::atomic_write`, and only when the run extracted an outcome that the
cache did not hold. It lives under `cache/`, so `klin cache clean` removes it
with the survey. The format is a small length-prefixed binary written by
hand, because the crate carries no serialization framework beyond
`serde_json`, and a `serde_json::Value` would allocate a map for every
declaration and reference beside the facts built from it. The whole file is
read at once. A lazy per-file
or per-symbol format waits until a measurement shows that reading the whole
file is the cost. Each turn's stamp is a new commit, so each write keeps the
four newest cache files and removes the rest. A storage budget across bases
is #193's.
