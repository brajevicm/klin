# Naming a gate claims it runs, and `--strict` decides every gate

> ADR 0016 retires the second rule below. A derivable gate runs whether or
> not the config names it, so it cannot be unaccounted. The first rule, that
> naming an excluded gate is exit 2, stands.

A gate is excluded by setting its section to `false`. ADR 0005 chose that over
deleting the section so `init --add` leaves the decision alone. Two rules follow,
and both close the same hole: a run that prints green while measuring less than
its reader thinks.

## `--gate NAME` on an excluded gate is exit 2

CI names every gate on the command line. That is the claim ADR 0008 and 0009
rest on, and the workflow is under code owners. The exclusion lives in the
config, which is under code owners too, but the two files travel separately. A
pull request that sets `"escapes": false` and leaves the workflow alone would
turn `--gate escapes` into a no-op, and CI would print a green line for a gate
that measured nothing.

So naming a gate is a claim that it runs. On an excluded gate the run is a tool
error, exit 2, and the message says the gate is excluded. A person decides in
one place or the other: lift the exclusion, or drop the gate from the command
line.

## Under `--strict`, an unaccounted gate is exit 2

The alternative was what the runner already did. A gate the config does not
mention does not run, silently. That is right for a local run and wrong for CI,
because it lets a gate ship dead: a detent release that adds a gate reaches
every project that upgrades and changes nothing about any of them.

Under `--strict` every gate detent offers takes a decision. The message lists
each unaccounted gate and its two remedies, configure it or set its section to
`false`. `init --add` writes the section in one command, so one remedy is a
command and the other is a line.

This breaks every strict run after an upgrade that adds a gate. That break is
the point. A gate nobody notices protects nobody, and nobody reads a warning in
CI output.

## Consequences

Under ADR 0009 `--strict` has three failures: an unaccounted gate, a config
error, and an accepted entry that matches nothing. The first is this record.

`gate --list` names the excluded gates and the available gates the config does
not mention, so the two lists behind a strict failure are readable before it.

A local run is unchanged. Both rules cost nothing until CI runs `--strict`, or
until someone names a gate a person has switched off.
