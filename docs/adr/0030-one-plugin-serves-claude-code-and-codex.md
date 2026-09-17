# One plugin serves Claude Code and Codex CLI

> Amends ADR 0023, whose last paragraph said Codex CLI has no plugin that
> carries a binary.

ADR 0023 made the Claude Code plugin the whole install for that host: the
hooks, a skill, two commands, and a wrapper that fetches the pinned release.
It left Codex CLI on the other route, where a person installs the binary and
`klin install --host codex` writes a hook file into the repository. That
route is three steps and a commit. ADR 0046 renamed that command, which was
`klin init --hooks --host codex` when this decision was taken.

Codex's plugin system turned out to read the same shapes. It looks for a
manifest at `.codex-plugin/plugin.json` and, failing that, at
`.claude-plugin/plugin.json`. It exports `CLAUDE_PLUGIN_ROOT` to plugin hooks
for compatibility with existing plugins. Its `hooks.json` has Claude Code's
shape, and the tool names it sends on stdin, `Bash` and `apply_patch`, are
the names klin's guard already reads. Only the marketplace differs: Codex
reads `.agents/plugins/marketplace.json`, and its plugin `source` is an
object rather than a path string.

## The decision

The plugin directory, then `plugins/claude-code`, is the plugin for both
hosts. Two marketplace files point at it, one per host. The manifest names
the hooks file, and the pre-tool matcher names `apply_patch` beside Claude
Code's edit tools. Nothing else in the plugin changes, and the version pin
stays in one manifest.

ADR 0045 extends this directory to Cursor with a third marketplace file and
a Cursor-format hooks file. The wrapper and the pin stay one copy.

For Codex the install is two commands and one review of the plugin's hooks:

```
codex plugin marketplace add brajevicm/klin
codex plugin add klin@klin
```

`klin install --host codex` stays, for a team that wants the hooks
committed and covered by CODEOWNERS, and for the Codex IDE extension, which
loads no plugins. It writes nothing when the plugin is enabled. Each host's
adapter knows where its host lists enabled plugins, because Claude Code keeps
a JSON key in its settings file and Codex keeps TOML tables in `config.toml`.
The check is one trait method with one body per host, and no module outside
the adapter names either host.

## Two protocol facts the first install taught

Codex substitutes the literal `${CLAUDE_PLUGIN_ROOT}` into a plugin's hook
line. The hook lines had used the shell default form
`${CLAUDE_PLUGIN_ROOT:-}`, which Codex left alone and the shell expanded to
nothing, so the wrapper was never found. Whether Codex also exports the
variable is not settled by what was read of its source, so the hook lines do
not rely on it. Claude Code exports the variable and reads the bare form the
same way, so the bare form is the one both hosts share.

Codex rejects plain text on a Stop hook that exits 0. Claude Code writes that
text to its debug log and shows nothing. Both hosts show a JSON
`systemMessage` on every event, so that is the shape every notice the plugin
prints on exit 0 now takes: the install hint, and the wrapper's two lines
about a release it could not fetch.

## Consequences

Codex users get the install ADR 0023 gave Claude Code users. One pin, one
wrapper, one hooks file. A release moves both hosts at once.

The plugin directory kept its name here, on the reasoning that renaming it
would move the one manifest the release tooling rewrites for no gain a person
would notice. ADR 0045 renamed it to `plugins/klin`, because a third host made
the install instructions name Claude Code at a person who is not using it.

Codex asks the person to trust the plugin's hooks once. That is Codex's
policy for every plugin, and klin does not try to bypass it.

The Codex plugin check reads `config.toml` with a line scan, the way the
lockfile check reads `Cargo.toml`, so klin gains no TOML dependency. A table
header that klin's scan misreads costs one duplicate hook install, which the
person sees as klin running twice, and nothing worse.
