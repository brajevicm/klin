# The plugin ships a wrapper that fetches the binary

> Supersedes ADR 0002. ADR 0030 amends the last paragraph: Codex CLI installs
> this same plugin.

ADR 0002 kept the binary out of the Claude Code plugin for two reasons. A
repository pinned a klin version beside its baselines, and a second binary
inside the plugin could disagree with that pin. And a plugin's `bin/` is
unavailable when the plugin is distributed through organization settings, so
the plugin could never be the only install route.

The first reason went with ADR 0009. There are no baselines, both trees are
measured by one binary, and a version difference between a local run and CI is
a note rather than a failure. The second reason is real and is handled below.

## The decision

The plugin ships `hooks.json` with the three hooks, one skill, two slash
commands, and a `bin/klin` wrapper. The wrapper is a shell script. On first
run it downloads the release the plugin version pins into
`~/.cache/klin/bin/<version>/klin`, verifies the checksum, and executes it.
Every later run executes the cached binary with no network call. When the
download fails, the wrapper prints one line saying so and exits 0, so a turn
is never blocked by a missing network.

Where `bin/` is unavailable, the hooks find `klin` on PATH from the install
script, a package manager, or `cargo install`, and the skill names the command
that installs it. When neither is present the Stop hook says so once and lets
the turn end.

With the optional config of ADR 0016, installing the plugin is the complete
install for Claude Code. No `init` runs. The first stop is gated.

## Consequences

This is the one place klin touches the network, and it is install, not
measurement. The determinism rule for checks is untouched.

The plugin pins its own klin version and upgrades when the plugin does. A
repository that also pins `version` in `klin.json` sees a note when the two
differ.

Cursor and Codex CLI have no plugin that carries a binary. For them the binary
comes from an install route and `klin init --hooks` writes the host's hook
file into the repository, where CODEOWNERS covers it.
