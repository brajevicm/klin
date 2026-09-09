# The config is optional, and a missing value is derived from the derivation commit

> Supersedes ADR 0005 and the `--strict` rule of ADR 0010.

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
