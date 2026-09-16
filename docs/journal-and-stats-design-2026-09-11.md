# The journal, `klin stats` and the benchmark, 2026-09-11

The design behind #152 to #157 and the rewritten #115. The tickets say what
each slice does and how it is accepted. This file says how the slices fit,
what a new gate costs, where the words live, and what was decided and why.
`docs/SPEC.md` is the authority, and sections 9.6, 11.4 and 11.5 land with the
tickets. Where this file and the spec differ, the spec wins.

The words are in `CONTEXT.md`: Turn, Session, Intervention, Journal, and
Regression. #172 replaced Shortcut with Regression and moved `shortcut` to the
avoid list. See "What #172 changed" at the end of this file.

## What it is for

Three things, one data source.

1. A record klin keeps of what every stop saw, so the maintainer can ask
   which gate blocks most, which one is red again on the second stop, how
   often a person resets, and how long the hook costs on a real tree.
2. A report that tells the person what klin caught, what the agent fixed on
   its own, and what is still open, in their words and not klin's.
3. A benchmark that reads the same record, so the numbers a user sees locally
   and the numbers the README claims come from one vocabulary.

## The shape

```text
gate.rs stopped()   guard.rs run()   turn.rs reset()   turn.rs on a prompt
        │                │                │                 │
  journal::stop    journal::guard   journal::reset   journal::prompt
        │                │                │                 │
        └────────────────┴──── append, private ─────────────┘
                                     │
                            <state dir>/journal.jsonl
                                     │
                          journal::read(root) -> lines + skipped count
                                     │
                     stats::episodes(&lines) -> Vec<Episode>     pure, no I/O
                        │                        │
              stats::text(scope, &episodes)   --json           <- the #115 harness reads this
              stats::turn_line(&episodes)                       <- gate.rs puts it in Stop::Tell
```

Two new modules carry the work. Everything else is a small edit where a
decision is already made today.

### The journal is one deep module

Its interface is four verbs, one per line kind, and one reader. It hides the
file location under the state directory, the JSON lines, the `schema`
integer, the best-effort append, the truncated last line, and the upgrade of
older schemas. No caller knows the file exists.

The write rule is the one `state.rs` already states: nothing klin writes for
itself may change a block or a pass. A failed append prints nothing to the
agent. The hook never prunes. `cache clean` leaves the file alone. The reader
tolerates a half-written last line and skips a line whose `schema` it does
not know, and returns how many it skipped so the report can say so.

The schema is an enum, `Schema`, and the reader matches on the current
variant with no wildcard arm. A bump adds a variant, and the reader does not
compile until it has an arm for that variant. See departure 1.

### The record is the object `gate --json` already prints

`Records` is the accumulator that flows out of every gate today. The journal
adds to it and builds nothing beside it. Since #158 it lives in `check` and a
runner hands each check a `&mut` to it rather than a shared cell, which
changes who holds it and not what it holds. `judge` puts `ms` and `held` on each
gate row as it goes.

What only the hook knows lives in one second accumulator, `journal::Stop`.
`stopped` creates it from the host event, carries it through the stop, and
writes it as one line at the end. See departure 2.

The stop line, as #153 states it:

```text
schema, version, time, kind: "stop", host, session, prompt
the 11.2 object: window, derived, gates[], findings[], notes[], exit
gates[].ms, gates[].held
hook:    { blocked, delivery, gate_spent, build_blocks, blocked_before }
verdict: "green" | "red" | "none", with why when none
timing:  { total_ms, build_ms, lock_ms, klin_ms }
asked:   the site ids this stop asked about
flags:   ["turn-restored", "branch-fallback", "count-unwritable"], or empty
told:    ["note", "turn", "weekly"], or empty
config_hash
```

`exit` on the stop line is the code the stop returned: 2 for a stop that
blocks, 0 for one that passes. See departure 3. `told` names the parts of
the `systemMessage` the stop printed, and the reader finds the last weekly
line from it. See departure 4.

`delivery` is `block` or `none` today. `follow-up` and `report` are reserved
for a host whose stop cannot block, which #67 brings. `config_hash` is
recorded and not read in v1, so a later reader can tell a fix from a config
change without a schema bump.

The other three kinds, from #154: `prompt` with the counter, the first line
of the prompt cut at 80 characters and the radius facts, `guard` with the
decision and a hyphenated reason, and `reset` with the counter. An allow
writes nothing. The guard's reasons are `config-write`, `state-write`,
`config-mention`, `state-mention`, `init` and `turn-reset`. A
`SessionStart` event moves the mark and raises the prompt counter, but
appends no line. See departure 5.

### The reader has two layers, and the boundary is a pure function

`episodes` turns lines into `Episode` values with no I/O, no clock and no
formatting. `text` turns episodes into the report. The #115 harness never
links Rust. It runs `klin stats --json` and reads episodes, which keeps the
one seam AGENTS.md asks for.

An intervention is a gate failure on a stop that spent the prompt's gate
block. That is the definition ADR 0004 and 0022 make observable, and it is
the ADR #153 writes: not a finding followed by its `id`, because the `id`
changes on a path rename, `doc-size` findings carry none, and ADR 0031 makes
a deleted test green on the next stop by rule.

The outcome is a relation between two lines, and the writer never stores it:

```rust
enum Outcome { FixedNext, FixedLater, Reset, Open, AskedOnce }
```

Every use of it is an exhaustive match: the JSON name, the report sentence,
the better-or-worse rule and the turn-end line. A sixth class cannot ship
with a missing sentence.

`AskedOnce` is read from the data, not from a gate name. A stop that lists a
site in `asked` and a next stop where that gate is green is an ask-once
episode. Today only `inventory` behaves so. A later gate that adopts the rule
gets the class with no edit to the reader.

### The words live in two places, by audience

The agent's words stay in `gate.rs` and in each check's `REMEDY`. They use the
glossary. The person's words live in `stats.rs`, in `text`, `turn_line` and
`weekly_line`. They use no glossary word. "Stop" becomes "klin ran 184
times". "Worktree" becomes "repository" when the repository has one.
"CI will refuse it" becomes "before you push".

A sentence for the person is changed by editing one function and one
expected string in the CLI tests. The turn-end line and the report headline
are built by the same code, so the voice is one.

## What a new gate costs

One row in `CHECKS`, as today. The journal records gate rows by name.
`episodes` groups by name. `text` prints the name only beside an item.
Nothing in `journal.rs` or `stats.rs` names a check. A CLI test in #155
feeds a journal line for a gate klin does not have and asserts it prints.
That test is the contract.

If per-check verbs ever come back, they are one more `&'static str` on the
`Check` row. Still one edit.

## The report

The shape is Safari's privacy report: one number, one sentence with klin as
the actor, the list one step away.

```text
klin, this week in this repository

klin caught 9 shortcuts. The agent fixed 8 of them on its own and asked you once.
One is still there.

Still there
  unwrap() in src/io.rs:12, left on Thursday
    Handle the error, or accept it in klin.json, before you push.

Fixed after klin asked
  Today
    todo!() in src/pay.rs:41, while you asked for the refund flow
    `as any` in web/cart.ts:88, while you asked for the cart total
  Tuesday
    empty test body in tests/cart.rs:20
  and 5 more. klin stats --all

You were asked once
  Wednesday
    test refund_twice deleted from tests/pay.rs. The agent said why.

Last week: 12 shortcuts, 2 left open. This week is better.
klin ran 184 times and took 4 seconds in total.
```

The rules:

- The first sentence has klin as its subject, the second has the agent.
- Open items come first, each with the check's remedy under it. They are the
  one group with an action.
- Five items per group, `--all` lifts the cap. `--json` has everything.
- Days, not clock times: Today, Yesterday, the weekday inside seven days,
  then the date. The local offset is read from the system once per report,
  with UTC as the fallback, because `std` has no time zone and klin takes no
  date crate.
- Words for zero and one at the start of a sentence, numerals elsewhere.
- The comparison is with this worktree's own previous window and nothing
  else. The word is better, worse or the same, judged on open items first.
- A Measurement footer prints only when a file was unreadable or lost, or a
  line was skipped. Green with half the tree unparsed is the one lie the
  report must not tell.
- The empty window says what klin did. The empty journal says klin started
  watching today.
- No score, no color, no glyphs, no praise, no estimate of time saved. Each
  would be the first thing a skeptical engineer distrusts.

The turn end prints one `systemMessage` line, only on a turn that had an
intervention: the count fixed on a green stop, the one still there on a red
pass-through. At most once every seven days that line also carries last
week's headline and names the command. Never a line at session start.

## Test seams

One seam, the binary's command line through the harness, for everything.

- #153 and #154 run a hook stop and read the journal from the state
  directory, which the harness already does for the turn file. Spec 11.4
  makes the record a contract, so the file is a public surface. Once #155
  lands, later tests assert through `klin stats --json` and no test reads
  the file.
- #155 and #156 copy a fixed journal into the state directory and run
  `klin stats`. One fixture per state: empty, first run, a week with every
  outcome, a week with an unknown gate, two weeks for the comparison.
- #157 is one ignored test that builds the fixture trees and prints medians.
  No benchmark crate.
- #115 is shell outside the crate. It wraps the hook lines so the shadow arm
  swallows exit code and output, and reads `klin stats --json`.

## Rust choices

- `serde_json::Value` maps, like the rest of the crate. No serde derive, no
  new dependency.
- One `timed` helper around the check run, the build and the lock wait, with
  `u64::try_from` on the milliseconds.
- The append is the one place a `Result` is dropped on purpose, and its doc
  comment says why.
- `BufReader::lines` for the read. A malformed line is skipped and counted.
- `Scope` is an enum, `Turn | Session | Since(Duration)`, and the title is an
  exhaustive match on it.
- The guard's `Decision` keeps its shape. The reason tag is set where each
  decision is built.

Out, on purpose: `tracing`, SQLite, a trait for observers, a plugin list of
report sections, a template engine for the words, and any Rust API the
benchmark links against. Each would add a name and hide nothing.

## Decisions, in short

From the grilling of 2026-09-11. The reason is beside each.

- Journal first, stats second, benchmark third. The journal is the data
  source for both readers, and it starts collecting on day one.
- The journal is specified. Two readers depend on its shape.
- One rich stop line, not a stream of small events. One stop is one
  decision, and the 11.2 object already exists.
- Guard allows are not recorded. The guard runs on every tool call under
  50 ms, and an allow tells a reader nothing.
- The journal survives `cache clean`. It is history, not a cache.
- "Session" is a glossary term of its own. The host's grouping of turns is
  a different concept from Turn, not a synonym.
- Intervention at gate granularity. One stop with three failing gates is
  one blocked stop and three interventions, and both counts are reported.
- Five outcomes, and a reset is never labeled a false positive. Only a
  person with the diff can say that.
- Gate time covers the gate's own measure and judge. Shared work is in
  `klin_ms` and in no gate.
- Outcomes are computed by the reader. A class is a relation between two
  lines, and a reader can change the rule without rewriting history.
- Bare `klin stats` is the last seven days. Rolling windows, `std::time`
  only.
- `--turn` reads from the current stamp. A reset is a person's decision to
  start the judgment over, and the report agrees with the hook.
- SPEC 13 fixture first among the benchmarks. It is a MUST with no agent
  budget.
- Shadow mode lives in the harness. The journal already records what klin
  did, so klin needs no flag.
- #115 is Claude Code only, one model. A duplicate Codex install advances
  the prompt counter twice until #149 lands.
- Stories before tallies. A line that names `todo!()` in the reader's own
  file shows them, and a count of "3 stubs" asks them to trust klin. #172
  reverses this for the default report and keeps it for `--all`.
- "Shortcut" is the person's word for a new finding the agent introduced in
  a turn. Finding stays the ratchet's unit. #172 replaced the word with
  Regression and changed the unit it counts.
- The prompt excerpt is recorded. It is local, like the file paths and code
  text the journal already holds, and it turns a list into the person's own
  story. A config key turns it off.
- The two earlier research documents were deleted. Their prerequisites had
  closed and their finding-id model was retracted.

## Open until implementation

- The exact `systemMessage` wording on Codex, which shows it as a warning,
  so the line must read as good news in one sentence.
- Whether `date +%z` or another system call gives the local offset on every
  platform klin ships for.
- A later round of #115 may replay historical klin tasks, starting the
  agent at the parent of a fixing commit. Not a ticket until the synthetic
  round has run.
- A debug trace under an environment variable, richer than the journal.
  Not a ticket until a profiling session needs one.

## Where the implementation departed

Spec 9.6, 11.4 and 11.5 record the result. The reason is beside each.

1. **The schema is an enum.** The design said one `match` on the schema
   number, exhaustive up to the current one. A `match` on a `u64` needs a
   wildcard arm, so a bump compiled and the reader then skipped every line
   the binary wrote. An enum with no wildcard arm makes the bump a compile
   error, which is what 11.4 asks for (#163).
2. **The hook's facts travel in `journal::Stop`.** The design said no struct
   carries facts across phases. The facts of one stop are known at
   different points: the lock wait before the window, the flags inside the
   window, the build stamp and the code in `hook`, the verdict and its `why`
   in `written`, and the `told` parts in `tell`. `Records` reaches none of
   those functions. One accumulator, written once, keeps one line per stop.
   The turn end also reads `journal::line` of that accumulator before the
   append, to count the stop the turn ends on.
3. **`exit` is the stop's code.** The 11.2 object is built before 16.3
   decides whether the stop blocks, so the first implementation recorded the
   gates' code beside `hook.blocked`. A blocked stop showed `exit: 1`. The
   stop now sets `exit` once its code is known (#163). The object a
   `--hook --json` run prints on stdout still carries the gates' code.
4. **`told` is a field.** The design did not name one. The weekly headline
   prints at most once every seven days, and the reader needs a record of
   the last one. The journal is that record, so the stop line names each
   part it printed.
5. **The guard names a mention apart from a write, and SessionStart writes
   nothing.** ADR 0033 splits a proven write from a command that only names
   a guarded path, so the state directory has `state-mention` beside
   `config-mention`. A `SessionStart` event opens a window and ends no turn,
   so a line for it would be a `prompt` line with no prompt.

## Ticket map

| Ticket | Delivers | Blocked by |
|---|---|---|
| #152 | `gate --json` prints `derived` and `exit` | none |
| #153 | the journal records every hook stop, spec 9.6 and 11.4, the ADR | #152 |
| #154 | session, prompt, guard and reset lines | #153 |
| #155 | `klin stats`, `--since`, `--all`, `--json`, spec 11.5, Shortcut | #153 |
| #156 | `--turn`, `--session`, "You were asked", last week, the turn end | #154, #155 |
| #157 | the spec 13 performance fixture | none |
| #115 | shadow against active, six tasks, ten runs, three labels | #155, #133 |

## What #172 changed, 2026-09-16

The journal is unchanged. Only its interpretation moved, which is what
"outcomes are computed by the reader" was for.

The report above is an activity dashboard. It answers "what happened" when the
person's question is "what needs me". #172 makes the default an attention and
value summary, two or three lines long, and moves the stories, the audit trail,
the timing and the previous-window comparison behind `--all` and `--json`.

Five decisions of this file are reversed or replaced.

- **Stories before tallies** now holds for `--all` and not for the default.
- **"Nothing in the reader names a check"** is replaced. The human label for a
  gate is presentation metadata on the catalogue row, a singular and a plural,
  and a new row does not compile without it. The reader still holds no table of
  gate names, which is what the rule was protecting, and a gate the binary has
  no row for still prints under its recorded name.
- **Intervention at gate granularity** stays the hook's unit and stops being
  the person's. `klin stats` counts one Regression per finding site per window,
  with its latest outcome.
- **Finding ids key nothing** is replaced. A current id is the primary stats
  identity. A record with no id falls back to gate, file, line and text, and
  the rename limitation is stated rather than worked around.
- **"regression" on the avoid list** is reversed. `shortcut` is on it instead.

Three rules were added, each reading what the record already held.

- `config_hash` was recorded and not read. A site that goes from a measurement
  taken under a different hash is now reported as resolved after the config
  changed, and never as a code fix.
- A site absent from a stop that measured its gate went, whether or not another
  site kept that gate red, so two sites under one gate resolve apart.
- Measurement confidence is one decision for the whole report, over unparsed
  and lost files, not-measured files, unresolved evidence, gate `ERR` rows and
  skipped journal lines. It outranks the value story, so a positive claim never
  sits above a window klin did not measure whole.

No copy klin prints for a person says who authored a fix. The journal proves a
site was present and later absent from a measurement, and nothing more.

`stats::turn_end` still reads the bounded tail of #184 and never the whole
journal. The long-history fixture in `tests/stats.rs` now carries blocked stops
with their own regressions, so the richer aggregation is what the bound holds
out of the stop path.

Spec 9.5 and 11.5, `CONTEXT.md` and ADR 0034 carry the result.
