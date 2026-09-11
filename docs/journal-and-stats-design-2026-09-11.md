# The journal, `klin stats` and the benchmark, 2026-09-11

The design behind #152 to #157 and the rewritten #115. The tickets say what
each slice does and how it is accepted. This file says how the slices fit,
what a new gate costs, where the words live, and what was decided and why.
`docs/SPEC.md` is the authority, and sections 9.6, 11.4 and 11.5 land with the
tickets. Where this file and the spec differ, the spec wins.

The words are in `CONTEXT.md`: Turn, Session, Intervention, Journal, and
Shortcut once #155 lands.

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
gate.rs stopped()   guard.rs run()   turn.rs reset()   radius on a prompt
        │                │                │                 │
        └────────────────┴──── journal::append(root, line) ─┘
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

The upgrade is one `match` on the schema number, exhaustive up to the current
one. A bump without an upgrade arm does not compile.

### The record is the object `gate --json` already prints

`Records` is the accumulator that flows out of every gate today. The journal
adds to it and builds nothing beside it. `judge` puts `ms` and `held` on each
gate row as it goes. `stopped` adds what only the hook knows. There is no
new struct that carries facts across phases, so there is no
prepare-process-finalize object to keep in step.

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
config_hash
```

`delivery` is `block` or `none` today. `follow-up` and `report` are reserved
for a host whose stop cannot block, which #67 brings. `config_hash` is
recorded and not read in v1, so a later reader can tell a fix from a config
change without a schema bump.

The other three kinds, from #154: `prompt` with the counter, the first line
of the prompt cut at 80 characters and the radius facts, `guard` with the
decision and a hyphenated reason, and `reset` with the counter. An allow
writes nothing.

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
  file shows them; a count of "3 stubs" asks them to trust klin.
- "Shortcut" is the person's word for a new finding the agent introduced in
  a turn. Finding stays the ratchet's unit.
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
