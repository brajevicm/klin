# One check catalogue and an explicit execution sink

Before this, a check was registered in two places that did not know about
each other. `gate.rs` held a private `CHECKS` table with the run function,
the base need, the scope support and the per-entry rule; `survey.rs` held
`DERIVABLE` with the same section names again and the keys the survey
supplies for each. `config.rs` asked `gate::sections()` which top-level keys
it may accept, so the configuration knew the runner. `reference.rs` asked
`gate::catalogue()` what to print.

What a check was told was one struct, `config::Flags`, that mixed the
configuration location, the base and tree state, the scope, the caller, the
output behaviour and a `RefCell<Records>` the check wrote through without the
runner naming it. Every check built one of these for itself.

The next five structural gates — #52, #51, #46, #42, #50 — each add a
consumer. Adding one had to be a row and a check, not an edit across
configuration validation, reference generation and the survey registry.

## The decision

`src/check.rs` owns what a check is, and nothing in it knows the runner.

### One catalogue

`check::CATALOGUE` is the one table. It is the source for the runnable
checks, the valid configuration section names, the command-name aliases such
as `doc-size` for `doc_size`, the reference sections and language tables, the
keys the survey derives, the base need, scope support and the per-entry gate
rule. `survey::DERIVABLE` is gone: a row states what it derives, `None` for a
section the survey never supplies, and `build` stays in `survey` because it
is the one derivable section no check reads.

`config`, `reference` and `survey` read the catalogue. None of them imports
`gate`.

The table is ordered, cheapest first, and a run executes its gates in that
order. No row carries a cost of its own, so where a check sits in the table is
the whole of the ordering decision. SPEC 4.6.

### Clap stays a second edit site, with a test between them

Issue #21's trade-off stands: derive-generated Clap help is worth the second
edit. A new check is a `Command` variant and a catalogue row. No macro owns
the `Command` enum to force one edit site, and no subcommand is generated
from the catalogue.

A CLI test reads the subcommands out of `klin --help` and the check names out
of the runner's own "names no check called" error, and fails when one holds a
name the other does not. Two checks, `inventory` and `lockfile`, run only
inside a gate, and the test names those two. Adding a third is then a
reviewed edit and not a silent gap.

The rest of `tests/catalogue.rs` reads the catalogue back the same way, off
the binary and never off the source: every section the reference prints is a
top-level key the configuration accepts, every language table belongs to a
section the reference names, every check name that is not its section is
corrected to that section, and `gate --list` accounts for every check as
running, excluded, or needing a section a person writes. That last one is
where the survey's derivability shows: `sarif` derives nothing, so the plan
asks a person for it.

One property here has no CLI test. That a new `Command` variant does not
compile until its dispatch is decided is a compiler fact, and a test that runs
the binary cannot see it. Each command's own behaviour is covered by its own
suite, which is what a wildcard success arm would have broken.

### Context is input, and the sink is output

`check::Context` is everything one check is told, borrowed for the length of
the call. `check::Sink` is where it writes: the report text, and
`Option<&mut Records>`. A check a person runs by hand is given a sink with no
records, so a direct `klin complexity` records nothing without a branch for
it. There is no `RefCell`, no `Rc`, no observer, no callback list and no
global recorder.

Two of the old `Flags` fields went away rather than moving. `context` and
`hook` were a pair of booleans that could disagree; they are now one
`Caller`, and whether a check says the run's own `window:` and `derived:`
lines is `Caller::Hand` and not quiet. The runner prints those once for the
whole run, which is the only reason the flag existed.

`Config::open(flags, start)` became `Config::load_with(explicit, start,
with)`, which takes configuration concerns only. `Config::say` no longer
decides whether to print — the caller that knows the run decides.

### `Records` stays the one accumulator

`docs/journal-and-stats-design-2026-09-11.md` made `Records` the one 11.2
object, and that decision is unchanged. It moved from `config` to `check`
with its field names and its meaning intact, and it is now passed rather than
reached through interior mutability. No `CheckReport` duplicates it, and
nothing else flows across the phases of a run.

### Dispatch is exhaustive

The `_ => Ok(0)` arm is gone, and so is the three-way split that hid it behind
`check`, `tool` and `runner`.

One exhaustive match over fifteen variants is one function over klin's own
complexity ceiling, and klin does not accept its own debt. So `Command` is
three groups — `Check`, `Runner` and `Tool` — each a `Subcommand` enum that
Clap's `#[command(flatten)]` merges back into one subcommand list. The command
line a person types and the help that lists it are unchanged, down to the
order. Each group is matched exhaustively on its own, no match has a binding
or wildcard arm, and a new variant does not compile until its dispatch is
decided.

`main` still answers the guard and the updater before the working directory is
read, because neither needs it, and both variants are answered in their group's
match as well.

## Consequences

`check::named_entries` and the `name` key moved out of `gate` with the
per-entry rule they belong to, so `sarif` no longer imports the runner to
read its own list.

The catalogue references the check modules and the check modules reference
`check`, which is a module cycle inside one crate and not a dependency edge
between crates. The catalogue aggregates constants and functions the check
modules declare; it does not restate them.

No plugin interface, dependency-injection container, tracing framework or
observer was added. A check is a function pointer in a table. When klin has a
check implementation it does not compile — which it does not — that decision
can be revisited; a trait with one implementation would state nothing today.

This is a structural change. No gate was added, no finding identity, ratchet
rule, journal schema, `gate --json` object or human report line changed, and
the #157 fixture was measured before and after on one machine.

## Final self-enforcement

Klin's own `klin.json` now pins these boundaries with
`conventions/no-old-flags`, `conventions/no-records-side-channel` and
`conventions/exhaustive-cli-dispatch`. They keep the retired execution bag,
direct `Records` mutation and wildcard-success dispatch from returning; the
layering section keeps the catalogue out of the runner. No accepted debt is
needed.
