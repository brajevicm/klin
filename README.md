# detent

Quality ratchets for AI-driven development, in a single binary.

A detent is the catch that holds a ratchet in position and stops it slipping
back. That is the idea here: every measure detent takes can tighten and never
loosen. It baselines the debt already in the tree once, then fails the build on
anything new.

detent runs inside the agent loop. A failing gate goes back to the agent as the
next thing to fix, and a guard refuses the agent's attempts to edit the
baselines instead of the code. It supports Claude Code, Cursor and Codex CLI.

## Status

Nothing works yet. This commit is the license, the glossary and two decision
records. Code follows.

## Origin

detent is a Rust rewrite of [cleat](https://github.com/svetdev/cleat) by Andrey
Kasatkin, MIT licensed. The gate designs, the ratchet model and the baseline
format all come from that project. This is a hard fork and does not track
upstream.

See `docs/adr/` for what changed and why.

## License

MIT. Andrey Kasatkin holds copyright on the original work, Miloš Brajević on
this rewrite.
