# The config is optional, and a missing value is derived from the derivation commit

> Supersedes ADR 0005 and the `--strict` rule of ADR 0010. ADR 0028 amends
> this for the `--hook` path: there the file is the opt-in marker. ADR 0040
> amends the `init` rule: `init` writes `{}`, `--pin` writes guardrails, and
> `--add` and `--force` are gone.
>
> The amendment below (#385) restates the source root rule as the survey
> applies it, and lets a test root sit inside a package's source root.

ADR 0005 held two rules apart. A key a gate needs and does not find is an
error naming the key, never a default. And `init` writes every section it can
infer, so a person gets every gate without writing seventeen sections. The
reading that made both true was that "by default" belonged to `init`.

That reading leaves a step. A tree with no `klin.json` is exit 2 until someone
runs `init`, and every number `init` writes is a snapshot of the day it ran.
The complexity ceilings are not even that: they come from a constant in the
binary. The goal is a tool a person manages nothing for, and a required setup
step with hand-written numbers is management.

## The decision

`klin.json` is optional. A check whose section can be surveyed from the tree
runs whether or not the file names it, over a section klin derives. A value the
file pins is used as written. A value it does not pin is derived, printed with
the word `derived` and the rule that produced it, and cached.

Derivation reads one commit, called the derivation commit. Under the turn
window of ADR 0017 that is the parent of the turn stamp, which is HEAD when
the stamp was taken. Under the branch and push windows it is the base itself.
The survey is a pure function of that commit and the binary version, so its
result is cached by commit id and the cache hits on every stop between two
commits.

The rules are per check and documented in each check. Source roots are the
shallowest directories that hold nothing but source of a known language,
outside the default skip set. A complexity ceiling is the 95th percentile of
the measured distribution at the derivation commit, rounded up, with a floor,
and the floor alone below 50 functions. A document ceiling is the word count at
the derivation commit rounded up to the next 50. Radius follows ADR 0014.

A derived ceiling is the day-one default and no more. It is not monotone. A
percentile rises when simple functions leave the tree, so a deletion that
worsens no surviving function can raise the next derived ceiling above the
last. klin keeps no history that could prevent this, and every run prints the
ceiling it used. A ceiling that cannot loosen is a pinned one. Tightening is a
dated schedule a person pins once, and `init` may offer to write one under a
flag, never unasked.

Amended 2026-09-09. The first text of this paragraph claimed the derived
ceiling was monotone. The 2026-09-09 review of `docs/SPEC.md` gave the
deletion counterexample, and SPEC.md 5.4 carries it.

`init` pins. It writes what the run would derive into the file so a person can
read, edit and review it. `--add` fills in missing sections. `--force` re-pins
every derivable section from today's tree. The guard denies `init` in every
form from an agent, so `--force` is a person's flag.

## What ADR 0005 feared, and why it no longer applies

ADR 0005 refused defaults because a gate that guesses its roots wrong reports
green while measuring nothing. Two rules close that. Every derived value is
printed on the run that uses it, so a wrong root is visible in the output that
says it is derived. And a survey that finds no source root is exit 2 under
`--strict` and a note in the hook, so a CI job in the wrong directory cannot
apply every gate to nothing and print green.

## Consequences

ADR 0010's `--strict` failure for an unaccounted gate has nothing left to
catch. A derivable gate cannot be unaccounted. A gate is still excluded by
setting its section to `false`, and naming an excluded gate on the command
line is still exit 2.

Every derivable check compares two trees. A check that judges one tree against
a number exists only for a number a person pinned. Without this a tree with a
stale citation would be red on day one with no config, and the promise that
day one is green would be false. `doc-citations` gains the base comparison.

The first stop after a commit pays a whole-tree survey. The performance
budget records that cost separately from the warm case.

A key klin does not know is still an error. A `baseline` key is still an
error saying the key is gone.

Amended 2026-09-09, second amendment. The decision above says the survey is
a pure function of the derivation commit. That holds for every derived
number, and for the path sets as they stand at that commit, and those are
what the cache holds. The path sets a run uses are wider. Roots, languages,
documents and manifests are the union of the cached survey and an uncached
walk over the `after` tree, so a directory added in a turn is measured on the
turn that adds it. A derived number is computed over the derivation commit's
own paths and never over a path found only in `after`, and a site under a
path the survey did not hold is `new`. So the union can only widen a gate. The
fourth review of `docs/SPEC.md` found the purity sentence and the union rule
in conflict, and 4.3, 5.4 and 7.1 carry the resolution.

## Amendment: a test root may sit inside a package's source root (#385)

The decision above calls source roots the shallowest directories that hold
nothing but source. The survey instead starts at the directory of each
source file and merges upward while the directory above holds nothing but
source, so a source file directly in a directory that holds anything else
starts its root in that directory. A crate's `build.rs` beside its
`Cargo.toml` makes the crate directory a source root, and the directories
beneath it are no roots of their own. SPEC.md 5.4 states this rule.

A test root used to be a source root that a test directory segment names or
whose every source file carries a test affix. The crate directory then held
`tests/`, so a crate with a `build.rs` had no test root, and `escapes` judged
`unwrap` and `expect` in its integration tests. The wgpu replay of #343 found
this in `naga/tests/naga`. The survey now looks for a test root among every
directory the merge ends at. A test root may sit inside another such
directory only when that directory directly holds a `Cargo.toml`, `go.mod`,
`package.json` or `tsconfig.json`, and a test root another test root holds
is not listed. So `tests/` beside a crate's `build.rs`, or beside a
package's `jest.config.js`, is a test root, while `src/spec/` beside
`src/schema.sql` stays in the source root `src`, as before.

Source roots do not change. `reachability` still derives its families from
the files under the source roots, less the source roots that are test roots,
so a test root inside a package's source root changes no family.
