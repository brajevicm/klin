# The Claude Code plugin ships no binary

> Superseded by ADR 0023. The plugin ships a wrapper that fetches the pinned
> binary, and falls back to PATH where `bin/` is unavailable.

A Claude Code plugin may include a `bin/` directory whose contents are added to
the Bash tool's PATH while the plugin is enabled. detent does not use it. The
plugin ships hooks, one skill and the slash commands, and expects `detent` to
already be on PATH from the install script or from npm.

Two reasons. A repository pins a detent version beside its baselines, and a
second binary inside the plugin could disagree with that pin, producing a
baseline failure that names neither version. And `bin/` is unavailable to
plugins distributed through claude.ai organization settings, so the plugin could
never be the only install path regardless.

## Consequences

Installing the plugin is not enough to run the gates. The binary is a separate
step. This is deliberate: the same install serves Cursor, Codex and CI, none of
which have a plugin at all.
