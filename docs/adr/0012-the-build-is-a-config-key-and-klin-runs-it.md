# The build is a config key, and klin runs it

> The decision on #434 amends this for the current product: the Stop hook is
> the only klin path that runs the build. `klin gate` outside the hook,
> `--strict` and CI included, runs no build entry, so the project's own CI
> must run the build and refuse a tree that does not build. This does not
> constrain a future agent-readiness path from using local build feedback.
>
> ADR 0048 amends this: a build block needs a tree that changed since the
> last one, and a shell exit of 127 is an unmeasured build whose gates run.
>
> ADR 0040 amends this: absent, the hook derives the build from the standard
> manifests and prints it, `init` writes no build, and `false` builds nothing.

The Stop hook used to be a shell wrapper. It ran the build, and it called
`klin gate --hook --changed` only when the build succeeded. Two programs held
one decision between them, and the wrapper had to parse the hook's JSON in
shell to take part in it.

An optional top level `build` key now names the build, and klin runs it before
it judges the tree. The key is one string for a single project:

```json
{ "build": "cargo build --all-targets" }
```

or a list of entries with a root, for a monorepo:

```json
{
  "build": [
    { "root": "api", "run": "cargo build --all-targets" },
    { "root": "web", "run": "tsc --noEmit" }
  ]
}
```

A string is the same as one entry with no root. An entry with no root covers
the whole tree.

`klin gate --hook` runs the entries, in the order the config lists them, and
blocks on the first failure with that command's output. No gate runs, because a
gate measuring a tree that does not compile measures nothing worth reading.
Under `--changed` klin runs only the entries whose root holds a changed file. A
changed file under no root runs every entry, because klin cannot know what that
file affects. Without `--changed`, and in CI, every entry runs.

## The key belongs to the hook, not to a gate

A config with no `build` key builds nothing, and that is not an error. ADR 0005
says a key a gate needs and does not find is an error naming the key, never a
default. This key is not one a gate needs. `init` writes one entry per manifest
it finds — a `Cargo.toml`, a `package.json` beside a `tsconfig.json`, a
`go.mod` — so a fresh tree gets the build step without asking for it.

The config is guarded, so an agent cannot change the command to skip the build.

## What this supersedes

ADR 0004's section "Why the logic spans a wrapper and a binary" no longer
holds. Nothing spans two programs. Its section on where the stamp lives still
holds, and so does its policy: a build failure and a gate failure each block
one stop per turn, and a build failure blocks every stop until the tree
compiles, bounded by Claude Code's cap of eight blocks per turn.

klin writes `.klin-build-blocked` itself now, in place of the wrapper, and for
the same reason. The two stops of one turn are two processes, so the fact that
the first stop's block was spent on the build has to reach the second stop
through a file. ADR 0004's argument for the path stands unchanged: not under
`target/`, which an agent empties as a matter of routine.

## Consequences

The build runs through `sh -c`, so the command is a shell line and not an
argument list. That is what a person writes in a config, and it is what the
wrapper already gave them.

klin computes the changed set twice under `--hook --changed`: once to pick the
build entries and once to scope the gates. It skips the first when every entry
has no root, which is the single project case and this repository's own. The
second computation is a `git diff` over an already warm repository.
