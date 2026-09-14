# `init` writes every section it can infer, and the runner defaults nothing

> Superseded by ADR 0016. The config is optional, a missing value is derived
> from the derivation commit and printed, and `init` pins what the run would
> derive. The `false` exclusion below stands. ADR 0040 amends 0016 in turn:
> `init` writes `{}`, and `init --pin` writes guardrails, never derived topology.

Two rules pull against each other. A key a gate needs and does not find is an
error naming the key, never a default, and
`a_missing_key_is_an_error_naming_the_key_not_a_default` pins it. At the same
time a user should get every gate that applies to their tree without opting
into each one by hand. At full parity that is seventeen sections, and nobody
writes seventeen sections.

The reading that makes both true is that "by default" is a property of `init`,
not of the runner. `init` surveys the tree and writes every section it can work
out, plus the baselines for them, so day one is green with the full set. The
runner is unchanged: a gate runs when its section exists, and a missing key is
still an error.

The alternative was to run all seventeen whatever the config holds. It fails on
its own terms. Nothing can guess which directories are sources, so sixteen
gates would report a missing key and the run would exit 2 on a fresh tree.
Worse, some gates cannot apply at all. `manifests` judges a generated Xcode
project, and a Rust library has none, so every such user would have to write
exclusions to get a green run.

`init` writes only what it can infer, so `sarif` and `manifests` are normally
absent from a generated config. They stay opt-in in practice. `gate --list`
names the gates that are available and not configured, which is where someone
already asking what exists will look.

## Consequences

`init` creates accepted debt without a person reading each entry, which is the
one place the tool does that. Two things follow. The guard refuses `init --add`
the way it already refuses the flag that rewrites a baseline. And `AGENTS.md`'s
rule against rewriting a baseline binds an agent working on this repository,
not the tool's behaviour on a user's tree.

Re-running `init` keeps an existing config untouched. `--add` fills in sections
that are missing and writes only their baselines. There is no `--force`:
rewriting every baseline is the guarded flag under another name, and the guard
refuses that command by name.

Because `--add` would otherwise resurrect a section someone deleted on purpose,
a gate is excluded by setting its section to `false` rather than by deleting
it. The key is present, so `--add` leaves it alone.
