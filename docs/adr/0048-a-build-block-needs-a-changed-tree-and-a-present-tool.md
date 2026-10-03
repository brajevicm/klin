# A build block needs a changed tree and a present tool

> The decision on #434 amends this for the current product: the Stop hook is
> the only klin path that runs the build. `klin gate` outside the hook,
> `--strict` and CI included, runs no build entry, so the project's own CI
> must run the build and refuse a tree that does not build. This does not
> constrain a future agent-readiness path from using local build feedback.
>
> Amends ADR 0012, ADR 0022 and ADR 0040. The build-first policy stands. What
> changes is which stops a build failure blocks, what an exit of 127 means,
> and what a failing derived build says about itself.

A benchmark trial on 2026-09-18 showed the policy at its worst. The task asked
an agent to add `typescript` as a development dependency and a `tsconfig.json`.
The agent declared the dependency, wrote a lockfile that recorded nothing, and
never ran an install. The new `tsconfig.json` beside `package.json` made the
hook derive `tsc --noEmit`. The shell answered `tsc: command not found`. Under
ADR 0012 no gate runs while the build fails, so the `lockfile` gate, whose one
finding named the cause, never ran. Under ADR 0022 the failure blocked eight
stops in one turn. All eight carried the same line, the agent changed nothing
between them, and the count ran to the bound. The `derived:` line ADR 0040
promised was printed only after a passing build, so the agent never learned
that its own `tsconfig.json` had created the build.

Three assumptions failed at once. A build failure is a compile error the agent
can read and fix. Each block follows an edit. A derived command explains
itself. None of the three holds when the tool is absent.

## The decision

**A build block needs a changed tree.** The build stamp records the working
tree the last block was taken over, hashed the way the turn stamp hashes it.
A stop whose tree is the same one spends no block: the hook reports the
failure, says the tree did not change, and lets the turn end with the RED
verdict already written. A block over a tree the agent did not touch teaches
it nothing, and CI refuses the tree either way. Each block that is spent names
its number in the turn, so the agent reads the budget it is spending.

**An exit of 127 is an unmeasured build, not a failed one.** The shell returns
127 when it cannot find the command. The tool is absent and the code is
unjudged, so klin has nothing to say about the build and no reason to hold
the gates. The hook records one NOTE that names the command, quotes the shell,
and states the action left: install the project's dependencies, or a person
sets `build` to `false`. The gates run over the tree. The note counts as told,
so a stop nothing blocks still carries it. klin reads the exit code and
nothing else. It does not parse the shell's message and it does not guess
which word of the command was the tool. An absent tool skips its own entry
and no other: the entries after it still run, and a compile error among them
still blocks. The note is for a build in which every entry that ran passed.

**A failing derived build names its origin.** The `derived:` line names each
command and the manifest it came from, `tsc --noEmit from package.json beside
tsconfig.json`, and the failure text opens with it. ADR 0040 said the hook
prints the derived build. It now does so on every run that derives one.

## What this is not

The build-first policy of ADR 0012 stands for a build that ran and failed. A
compile error still blocks before any gate runs, because a gate measuring a
tree that does not compile measures nothing worth reading. The bound of ADR
0022 stands at eight. This record narrows what a block is spent on and what a
failure is. It does not run gates over a tree whose compiler spoke.

## Consequences

- The eight-message loop of the trial becomes one stop that runs the gates
  and tells the `lockfile` finding, or, for a compile error the agent does
  not touch, two stops: one block and one report.
- An agent can skip the local build by deleting the tool. That was already
  true for a tool under `node_modules`. CI is authoritative under ADR 0009,
  and a tree whose declared dependency is not installed is the `lockfile`
  gate's finding.
- A build block costs one hash of the working tree, only at a stop whose
  build failed. The hash goes through an index of the build stamp's own,
  `build-index`, because the turn stamp reads the absence of its own index
  as a first session and a stop must not take that reading away.
- The build stamp gains one field, `tree`. A record without it reads as a
  changed tree and blocks, so a stamp an older binary wrote costs one block.
- `docs/SPEC.md` 5.4, 8.2, 9.3 and 16.3 carry the rule.
