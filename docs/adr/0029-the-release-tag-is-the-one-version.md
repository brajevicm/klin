# The release tag is the one version

> ADR 0040 amends this: `klin.json` has no `version` key, so nothing in a
> repository overrides the tag the Action is pinned at. The Action's own
> `version` input still does.
>
> The amendment below (#338) makes the Claude Code and Codex marketplace
> entries name the plugin at the release tag, so a plugin user gets the plugin
> files of the release the wrapper runs. The `ref` of each entry is one more
> place the version is written. A release now pushes its tag alone and stays a
> prerelease, and `main` takes the tag and the release becomes Latest only
> after the release smoke.

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

The version is written in `Cargo.toml`, the plugin manifests and the
marketplace entries, plus two lines in the README. The wrapper reads the
manifest at run time. `cargo-release` rewrites all of them in one commit and
tags it, from the `cut-release` workflow or from a laptop. A CLI test fails
when a manifest or a marketplace entry disagrees with `Cargo.toml`, so a pin
that moves by hand is caught before a tag exists.

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

A release is one version bump and one tag. No channel can drift from the tag,
because none holds a version of its own. The plugin, the installer and `klin
update` reach a new tag at its promotion, after the release smoke. The
Action lives in this repository so that the same tag pins it, which reverses
the separate `klin-action` repository issue #100 asked for.

Homebrew and npm are not enabled. `dist` generates both when they are wanted,
and neither changes this decision, because both would download from the same
release page. Publishing the crate is deferred with them.

The version is still in more than one file. Cargo needs its own, Claude Code
updates a plugin only when the manifest's version changes, and each
marketplace entry names the tag. The tests make that duplication safe.

## Amendment: the marketplace entries name the release tag (#338)

Claude Code and Codex read a marketplace from the default branch of this
repository. Their entries named `./plugins/klin`, so a host copied the plugin
as `main` held it on the day of the install. Between releases, `main` changes
the wrapper, the hooks and the skill, and the manifest still pins the last
release. A plugin user then ran unreleased hook and skill text against the
released binary, and two installs of one version could hold different files.

Both entries now name the plugin at the tag, through the `git-subdir` source
that Claude Code and Codex both document. Each entry spells the path as its
host documents it: `plugins/klin` for Claude Code, `./plugins/klin` for Codex.

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
and the tag names that commit. One CLI test applies the rewrites that
`Cargo.toml` configures, and it fails unless each marketplace `ref`, and
nothing else, becomes the new tag. Another fails when the `ref` is not `v`
followed by the version in `Cargo.toml`. A commit to `main` that changes
`plugins/klin` therefore reaches plugin users only with the next release.
Claude Code 2.1.284 and Codex 0.158.0 installed a tagged plugin this way, and
an update after a new tag installed that tag and no later commit.

The `ref` fixes the plugin files only while the tag does not move. No ruleset
protects the tags of this repository, and GitHub reports v0.3.0 as a mutable
release, so a moved v0.3.0 would give two installs of 0.3.0 different files.
#336 makes the next release immutable, and the tag of an immutable release
cannot move. v0.3.0 does not become immutable after the fact.

### A release reaches its users when it is promoted

`main` now decides which release a plugin user installs, and GitHub's Latest
release decides what the installer and `klin update` install. So a release has
two steps:

1. `cut-release` makes the release commit and the tag, and pushes the tag
   alone, because `cargo-release` pushes nothing (`push = false`). It then
   makes the release a draft prerelease, with the title and notes that `dist
   plan` gives. `dist` uploads the artifacts to that draft and publishes it
   (`create-release = false`), and the release stays a prerelease. Plugin
   users, `releases/latest`, the installer and `klin update` stay on the last
   release. The Action can use the new tag at once.
2. A person runs the release smoke of `docs/HOST_COMPATIBILITY.md`
   against that tag, through the documented commands with the tag appended.
   When the smoke passes, `promote-release` merges the tag into `main` and
   marks the release Latest. From then on, both marketplaces, the installer
   and `klin update` serve the new release.

Nothing checks the smoke. A run of `promote-release` is the person's
statement that it passed. `promote-release` pushes `main` directly, as
`cut-release` did before. When #336 makes checks on `main` required, the
workflow must be allowed to push, or it must promote through a pull request.
#336 owns that.

A release from a laptop has the same steps. `cargo release` makes the commit
and the tag. The person pushes only the tag and makes the draft as
`cut-release` does: `gh release create vX.Y.Z --draft --prerelease
--verify-tag`, with the title and notes that `dist plan` gives. Then the
person resets local `main` to `origin/main`, because a push of that `main`, or
a branch cut from it, would carry the release commit into `main` before the
smoke. The promotion merges the tag later. CLI tests fail when the release
configuration lets `cargo-release` push, or lets a release become Latest before
the promotion.

When `cut-release` fails after it pushed the tag, a person makes the draft the
same way and re-runs the failed jobs of the Release workflow. A candidate that
became Latest by mistake goes back with `gh release edit vX.Y.Z --prerelease`,
and `gh release edit --latest` on the last promoted tag makes that release
Latest again.

`cut-release` starts from `main`, so the next release starts after the
promotion. A tag that fails the smoke is not promoted, and its release stays a
prerelease that no route serves as Latest. The fix ships as the next version,
cut with an exact `version`, because `main` still holds the version before the
failed tag.

### Rejected options

- A release branch that people add the marketplace from, such as
  `brajevicm/klin#release`. A person who added the marketplace without the
  ref stays on `main`, and `cargo-release` does not move a branch.
- A `git-subdir` entry that names a release branch. The `cut-release`
  workflow and a release from a laptop must then both push the branch, and no
  test can check that the branch holds the tag.
- A `sha` beside the `ref`. The release commit cannot hold its own hash, so a
  second commit would have to write it, and the marketplace at the tag, which
  the smoke installs from, would have none. Immutable releases give the same
  guarantee from #336 on.
- A smoke before the tag exists. The wrapper fetches the binary of the
  version its manifest pins, so plugin files from before the release commit
  run against the last binary, and the smoke would try neither release.
- A job after `dist`'s announce step that makes the release a prerelease. The
  release would be Latest until that job ran.
- An edit of the generated `release.yml`. ADR 0026 leaves that file to
  `dist`, and `create-release = false` is the setting `dist` offers for a
  release that another step makes.
- A release candidate version, such as `v0.4.0-rc.1`, which `dist` marks as a
  prerelease by itself. The smoke would try the candidate, and the version
  that users get would be a second build that nobody smoked.

### Consequences

- The host fetches the plugin with a second, sparse clone. The marketplace
  clone supplies only the catalog.
- Claude Code reads `plugin.json` before an install only for a relative-path
  entry. Its plugin list therefore shows only the entry's own fields until a
  person installs the plugin.
- A marketplace added from a checkout installs the plugin of the tag it names
  from GitHub, not the files of the checkout. The host canary adds one that
  way. When the install fails, the canary checks whether the tag can be
  fetched, and a failed fetch or a missing tag is `INFRA`, not `COMPAT`. While
  the repository is private, the canary's clone has no credentials.
- Claude Code fetches a plugin with a remote source only when a user, local,
  flag or managed setting enables it. When only a repository's
  `.claude/settings.json` enables the plugin, a teammate installs it once.
- Cursor documents only a path source, so its entry still names the plugin
  directory in the marketplace's own tree. The documented Cursor route
  already copies the plugin from the release tag. A Team Marketplace import
  reads the branch the team imports, and that route has no recorded
  verification (spec 19.2).
- Homebrew and npm, once enabled, would publish at the tag, before the
  promotion, because `dist` decides from the version string and not from the
  prerelease flag on GitHub.
