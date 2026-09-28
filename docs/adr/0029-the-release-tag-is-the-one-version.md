# The release tag is the one version

> ADR 0040 amends this: `klin.json` has no `version` key, so nothing in a
> repository overrides the tag the Action is pinned at. The Action's own
> `version` input still does.
>
> The amendment below (#338) makes the Claude Code and Codex marketplace
> entries name the plugin at the release tag, so a plugin user gets the plugin
> files of the release the wrapper runs. The `ref` of each entry is one more
> place the version is written.

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
  Dependabot do.

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

## Amendment: the marketplace entries name the release tag (#338)

Claude Code and Codex read a marketplace from the default branch of this
repository. Their entries named `./plugins/klin`, so a host copied the plugin
as `main` held it on the day of the install. Between releases, `main` changes
the wrapper, the hooks and the skill, and the manifest still pins the last
release. A plugin user then ran unreleased hook and skill text against the
released binary, and two installs of one version could hold different files.

Both entries now name the plugin at the tag, through the `git-subdir` source
that Claude Code and Codex both document:

```json
"source": {
  "source": "git-subdir",
  "url": "https://github.com/brajevicm/klin.git",
  "path": "plugins/klin",
  "ref": "vX.Y.Z"
}
```

The host fetches `plugins/klin` at that tag, so a plugin installed from
either marketplace holds the files of the release its manifest names.
`cargo-release` rewrites the `ref` in the commit that rewrites the manifests,
and the tag it pushes names that commit. A CLI test fails when the `ref` is
not `v` followed by the version in `Cargo.toml`, or when the release
configuration does not rewrite that marketplace file. A commit to `main` that
changes `plugins/klin` therefore reaches plugin users only with the next
release. Claude Code 2.1.284 and Codex 0.158.0 installed a tagged plugin this
way, and an update after a new tag installed that tag and no later commit.

### Rejected options

- A release branch that people add the marketplace from, such as
  `brajevicm/klin#release`. A person who added the marketplace without the
  ref stays on `main`, and `cargo-release` does not move a branch.
- A `git-subdir` entry that names a release branch. The `cut-release`
  workflow and a release from a laptop must then both push the branch, and no
  test can check that the branch holds the tag.

### Consequences

- The host fetches the plugin with a second, sparse clone. The marketplace
  clone supplies only the catalog.
- A marketplace added from a checkout installs the plugin of the tag it names
  from GitHub, not the files of the checkout. The host canary adds one that
  way. While the repository is private, the canary's clone has no
  credentials, and the canary classifies a failed plugin install as `COMPAT`.
- The pre-release smoke of `docs/HOST_COMPATIBILITY.md` installs through the
  documented route, so it tries the plugin of the last tag, not the files of
  the release candidate.
- Claude Code fetches a plugin with a remote source only when a user, local,
  flag or managed setting enables it. When only a repository's
  `.claude/settings.json` enables the plugin, a teammate installs it once.
- Cursor documents only a path source, so its entry still names the plugin
  directory in the marketplace's own tree. The documented Cursor route
  already copies the plugin from the release tag. A Team Marketplace import
  reads the branch the team imports, and that route has no recorded
  verification (spec 19.2).
