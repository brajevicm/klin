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
