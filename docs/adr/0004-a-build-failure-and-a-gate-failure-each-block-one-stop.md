# A build failure and a gate failure each block one stop

> ADR 0012 supersedes the section "Why the logic spans a wrapper and a binary".
> The build is a config key and klin runs it, so there is no wrapper. The
> policy below, and the argument for where the stamp lives, still hold.

detent's Stop hook is a shell wrapper. It builds the tree, and it runs
`detent gate --hook --changed` only when the build succeeded. Claude Code sends
the hook a `stop_hook_active` flag, true once the hook has already blocked the
turn. The wrapper and the binary both read it, and each took a true flag to mean
that the turn's block was spent: report, and let the turn end.

So a turn that started with a compile error got a free pass on the gates. The
build failure blocked the first stop. At the second stop the tree compiled, a
gate failed, and detent printed the failure and returned 0. The agent never saw
it, and the turn ended ungated.

## The policy

A build failure and a gate failure each block one stop per turn.

The wrapper no longer reads `stop_hook_active`. A build failure blocks every
stop until the tree compiles. Claude Code overrides a Stop hook after it blocks
eight times in a row without progress, so a tree that never compiles cannot trap
the agent, and that backstop bounds the loop better than a flag the wrapper has
to match by hand. Matching it by hand is also where a bug lived: the wrapper
stripped spaces and newlines from the payload first, so a tab-indented or CRLF
event defeated the match.

When the wrapper blocks for the build it writes `.detent-build-blocked` beside
`quality.json`. `detent gate --hook` deletes that file on every run, and when
the file was there, treats the turn's gate block as unspent. It blocks even
though `stop_hook_active` is true. The delete happens on every run, a passing
one included, so a stop that passes ends the stamp's life and a later failure in
the same turn gets no extra block.

The stamp does not live in `target/`. An agent that meets a build error it
cannot read runs `cargo clean`, and that would delete the stamp and hand the
turn the free pass this record exists to close. `detent guard` does not protect
the stamp either, so keeping it out of the directory an agent empties as a
matter of routine is what makes it survive the turn.

## Why the logic spans a wrapper and a binary

Only the wrapper knows the build failed. It has to run the build, because a gate
measuring a tree that does not compile measures nothing worth reading, and cargo
must finish before detent starts. Only the binary knows a gate failed. The stamp
on disk is how the earlier stop tells the later one what already happened, and a
file is the only channel the two share.

The stamp is also what keeps the wrapper small. Deciding this inside the wrapper
means parsing the hook's JSON in shell, which is where the whitespace bug came
from.

## Consequences

The stamp carries no session key. Two sessions gating one repository can hand
each other one extra block. The eight-block cap bounds that too, and the price
of preventing it is shell JSON parsing again.

A stamp an abandoned turn left behind changes nothing. It matters only when
`stop_hook_active` is true, and at a fresh turn's first stop the flag is false.
detent blocks on that stop regardless, and deletes the stamp as it goes.

A gate failure can block twice in one turn, so a turn can reach three blocks:
the gate fails and blocks, the fix breaks the build, the build failure blocks,
and the repaired build meets the same failing gate. `unspent` records that a
build failed, not that the gate has yet to block, and telling those apart needs
a second piece of state on disk. The eight-block cap bounds the turn, and the
cost is that the second gate block repeats the first-time wording rather than
the wording for a stop that follows a round of fixes.

The path is written once, in `src/gate.rs`, under the directory that holds the
configuration. That stays the right directory when `klin gate --hook` runs from
a subdirectory.
