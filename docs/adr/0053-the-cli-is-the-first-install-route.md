# The CLI is the first install route

> Amends ADR 0023 and ADR 0046, which made the native plugin the preferred
> local product experience and the standalone route the alternative.
>
> ADR 0055 amends the consequence that `klin install` writes no hooks a
> plugin already supplies. It now writes them, and one copy takes each event.
>
> ADR 0056 makes the two commands one, run from the repository, and drops the
> need for `--host` in it. `--host` still narrows the hosts.

A native plugin gives a person klin's hooks and no `klin` command. No host
puts a plugin's `bin/` on the person's own PATH: Claude Code adds it to the
Bash tool alone, and Codex and Cursor add it nowhere. None of the three runs
anything when a plugin is installed. So a plugin user cannot run `klin init`,
`klin gate --list`, `klin init --pin` or `klin turn reset`, and the last of
these is a command only a person may run. The plugin route's opt-in was
`echo '{}' > klin.json`, a file format where the CLI has a command.

The standalone route gives everything in two commands on every first-class
host: the installer puts `klin` on PATH, and `klin install --host NAME` opts
the repository in and writes the hooks and the skill for that host. Where the
klin plugin already serves the host, `klin install` leaves the hooks and the
skill to it (19.3). The
committed hook files also reach a teammate who clones the repository, which a
plugin a person installed does not.

## The decision

Documents lead with the standalone route: the installer, then `klin install`.
The native plugins stay supported and self-sufficient, as the alternative for
a person who wants the host's own plugin. Claude Code and Codex install and
update theirs in one step. Cursor's verified route is a local copy a person
makes from a release tag and makes again to update. A
plugin still fetches its own pinned runtime and needs no binary for its hooks.

The plugin does not install the CLI. Once per machine the wrapper names the
command that installs it, and the person runs it. An install hint is never a
`followup_message`, because Cursor submits one as the next prompt and would
hand the installer to the agent.

## Rejected options

- A plugin hook that installs a shared `~/.local/bin/klin`. It installs
  software outside the plugin without asking, it may edit shell startup
  files, Codex hides hooks behind trust for this reason, and Cursor's
  marketplace says it ships no binaries. klin would maintain a package
  manager inside three hosts.
- A host activation command on each host (#299, #300). It serves activation
  alone, it adds a vendor contract per host to canary, and Codex's
  `UserPromptSubmit` prompt drops a skill the person picked from a menu, so
  its signal is unreliable. `klin install` and `klin init`, which a person
  runs, already activate a repository. Claude Code's `/klin:init` (#298) is
  built and verified on its own branch and is deferred, not rejected: it
  ships only if plugin-only users turn out to stop at activation.
- Dropping the plugins. A plugin in the host's own directory is how a person
  finds klin and trusts that it is native to the host.

## Consequences

The Homebrew and npm channels would make the first command shorter and would
edit no shell startup file. Spec 19.1 forbids an install command for a channel
that has not shipped, and both would download from release assets that are
not public yet, so they wait for #315.

A person may take both routes. `klin install` then writes no hooks the plugin
already supplies (19.3), so the CLI arrives without a second copy of the
hooks.

A plugin user's CLI and plugin may run different versions. Spec 5.2 already
reads a version difference as a NOTE.
