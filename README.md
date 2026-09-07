# detent

Quality ratchets for AI-driven development, in a single binary.

A detent stops a ratchet slipping back. Every measure detent takes can tighten
and never loosen: it records the debt in the tree today, then fails the build
on anything new.

detent runs inside the agent loop. A failing gate goes back to the agent to
fix, and a guard refuses the agent's edits to the config, the baselines and
the hooks. Supports Claude Code, Cursor and Codex CLI.

## Status

Early. Config discovery, the guard, the ratchet engine and three gates run,
cheapest first: doc-size, escapes, and complexity over eight languages.

## Holding the ratchet

A section deleted from the config runs no gate, and the run still reads green.
Name every gate on the CI command line, and put the workflow and the config
under CODEOWNERS.

## Origin

detent is a Rust rewrite of [cleat](https://github.com/svetdev/cleat) by Andrey
Kasatkin, MIT licensed. The gate designs and the ratchet model come from that
project, which this fork does not track.

`docs/adr/` records what changed.

## License

MIT. Andrey Kasatkin holds copyright on the original work, Miloš Brajević on
this rewrite.
