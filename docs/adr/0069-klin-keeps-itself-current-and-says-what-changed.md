# klin keeps itself current and says what changed

> Amends the line of SPEC section 1 that said klin runs offline. Recorded for
> #625, 2026-10-11. ADR 0070 amends items 2 and 3: klin installs a signed
> release itself, and auto-update is on by default.

A person who installed klin with the installer never learned that a newer
release was out, and an update is routine work that SPEC section 2, goal 1
keeps from the person. The plugin routes have no such gap for Claude Code and
Codex, because the host updates the plugin.

## The decision

**klin asks GitHub for the newest release once a day in a detached child,
can install it, and tells the person and the agent what changed. No command
and no hook waits on the network.**

1. A session start, `klin status` and `klin setup` start the child when the
   last ask is a day old. The child reads the redirect of `releases/latest`
   through `curl`. klin carries no HTTP client, and the redirect spends no
   API rate limit. The ask counts from the moment the child starts, so an
   offline machine tries once a day.
2. With auto-update on, the child runs the `klin-update` beside the binary
   for a newer release, once for each release. `klin update --auto on|off`
   writes the setting to a file in klin's cache, because a host started
   outside a shell does not pass a shell profile's variables to its hooks.
   `KLIN_AUTO_UPDATE=1` or `0` overrides the file. One binary measures
   both trees at a Stop (ADR 0001), so a swap between two events of a turn
   changes no verdict.
3. Auto-update is opt-in until releases are signed and the updater checks
   the signature (#627). A checksum from the same release does not protect
   against a bad release, and auto-update would deliver one to every machine
   within a day.
4. A binary with `klin-update` beside it asks, updates and tells. A plugin's
   binary, which the wrapper marks with `KLIN_PLUGIN=1`, asks and tells, and
   names its host's update step. It never updates itself, because the host
   owns the plugin. Claude Code leaves auto-update off for a marketplace that
   is not its own, so the Claude Code step also names that switch. `CI` and
   `KLIN_NO_UPDATE_CHECK` turn everything off.
5. The person hears the news in a Stop's notice: a newer release with the one
   command that installs it, or the update that happened, with `klin setup`
   named only when a copy of klin's integration is in conflict: with a commit
   for a repository copy, and as `klin setup --user --host HOST` for a copy
   in the person's home. The agent hears it
   in the session-start text, because a Stop's notice reaches only the
   person. An update is news once. The person hears of an available release
   once, and `klin status` keeps showing it. The agent forgets between
   sessions, so it hears of an available release at most once a day, until
   the person updates (owner decision, 2026-10-11).
6. The guard denies `klin update` and the hidden `klin __release` to an
   agent, so an agent never chooses the binary that judges it.

## Rejected options

- Only `klin update --check` asks. No command contacts GitHub on its own, but
  a person who never runs it never hears of a release.
- An ask that waits, from `klin status` and `klin setup` only. It costs a
  person up to the timeout once a day and never reaches a person who only
  works through the agent.
- An ask from a Stop. A Stop runs inside the hook's time budget, and a
  session start already reaches every person who uses an agent.
- An HTTP client in the binary. It adds a TLS stack for one request a day.
- Telling the agent through the Stop's notice. A `systemMessage` reaches the
  person and not the model.

## Consequences

The ask sends no data of the repository. A person without `curl` gets no
notice. The Codex step is `codex plugin marketplace upgrade klin`. Codex
CLI 0.162.1 was run against a Git marketplace that pins its plugin to a
tag: after a release moved the pin, the upgrade alone replaced the
installed plugin with the new version and dropped the old one from the
cache, and `codex plugin add` afterwards changed nothing.
