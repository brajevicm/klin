# The release tag is the one version

klin reaches a person through several routes: the Claude Code plugin and its
wrapper, the install script, a GitHub Action, and a release page someone
downloads from by hand. Cursor and Codex will add hook files that call the
same binary. Each route could carry its own version and its own update
story, and a maintainer would then publish to each of them.

## The decision

The git tag `vX.Y.Z` is the version of everything. It names the binary, the
plugin, the Action, the installer and the updater that `dist` publishes under
it (ADR 0026). Every route is a pointer to that one release page, and no
route holds a build of its own.

The version is written in two files, `Cargo.toml` and the plugin manifest,
plus the `uses:` line in the README. The wrapper reads the manifest at run
time. `cargo-release` rewrites all of them in one commit and pushes the tag,
from the `cut-release` workflow or from a laptop. A CLI test fails when the
two disagree, so a pin that moves by hand is caught before a tag exists.

Each route updates with the tool the person already uses:

- The plugin, through `/plugin marketplace update` or Claude Code's
  auto-update. The new plugin pins the new version and the wrapper fetches it
  on the next run, then drops the versions before it from the cache.
- The installer and a hand download, through `klin update`, which runs the
  `klin-update` the installer placed beside the binary.
- CI, by moving the tag in `uses: brajevicm/klin@vX.Y.Z`, which Renovate and
  Dependabot do. The `version` key in `klin.json` overrides it.

The plugin's hooks run the wrapper by its path under `CLAUDE_PLUGIN_ROOT`
before any `klin` on PATH, because Claude Code appends plugin `bin/`
directories last. Without that, a binary from an earlier installer run would
outlive every plugin update.

## Consequences

A release is one version bump and one tag. No channel is published by hand,
and no channel can drift from another, because none holds a version of its
own. The Action lives in this repository so that the same tag pins it, which
reverses the separate `klin-action` repository issue #100 asked for.

Homebrew and npm are not enabled. `dist` generates both when they are wanted,
and neither changes this decision, because both would download from the same
release page. Publishing the crate is deferred with them.

The version is still in two files. Cargo needs its own, and Claude Code
updates a plugin only when the manifest's version changes. The test makes
that duplication safe.
