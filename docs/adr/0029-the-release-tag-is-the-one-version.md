# The release tag is the one version

> ADR 0040 amends this: `klin.json` has no `version` key, so nothing in a
> repository overrides the tag the Action is pinned at. The Action's own
> `version` input still does.
>
> The amendment below (#338) makes the Claude Code and Codex marketplace
> entries name the plugin at the release tag, so a plugin user gets the plugin
> files of the release the wrapper runs. The `ref` of each entry is one more
> place the version is written. #470 changes the release topology: a GitHub
> release PR writes the next version to `main`, and only the merged release
> commit is tagged and handed to cargo-dist.

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
manifest at run time. `prepare-release` runs `cargo-release --no-tag`, so cargo-release rewrites all
of them in one commit without creating the release tag. The workflow pushes
that commit to a deterministic release branch and opens a pull request. After
the PR merges, `publish-release` creates the tag on the exact merged commit. A
CLI test fails when a manifest or a marketplace entry disagrees with
`Cargo.toml`, so a bad release PR is caught before a tag exists.

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
because none holds a version of its own. The plugin, the installer and `klin update` reach a new tag only after its
release PR has passed the normal checks, merged to `main`, and cargo-dist has
published that merged commit. The
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

### A release reaches its users from a merged release PR

A release has one protected-code path:

1. A maintainer dispatches `prepare-release` in GitHub Actions with
   `patch`, `minor`, `major` or an exact `X.Y.Z`. The workflow runs
   `cargo-release --no-tag`, pushes the generated commit to
   `release/vX.Y.Z` and creates or refreshes a pull request to `main`.
2. That pull request runs the ordinary `quality / gates` check and every
   normal rule on `main`. No release workflow can push directly to `main`.
3. When the release PR actually merges, `publish-release` verifies the
   version-bearing files, creates `vX.Y.Z` on the exact merged commit and
   pushes that tag. The push uses `RELEASE_TOKEN` because a tag pushed by the
   workflow's `GITHUB_TOKEN` would not start the tag-triggered Release
   workflow.
4. The tag-triggered, cargo-dist-generated `release.yml` builds the exact
   tag, creates the GitHub Release with the generated install/download notes,
   uploads the artifacts and makes it available through GitHub Latest.
   `publish-release` does not own GitHub Release state.

The release branch is generated state. Re-running `prepare-release` for an
open version refreshes the same branch and PR from current `main`.
A release PR closed without merging is not silently recreated. Re-running the
publisher is safe when the tag already points to the merged commit, and it
refuses to move an existing tag.

The release PR is the human release boundary. A normal release requires no
local checkout, tag command or second promotion step.

### Host compatibility is independent release evidence

The host canary and manual host checks remain evidence for the current stable
Claude Code, Codex and Cursor surfaces, but manual smoke is not a mandatory
step for every release. Run targeted manual verification when a release
contains functional host-integration changes or when the compatibility
evidence is stale, red or inconclusive. Version-only manifest/ref rewrites in a
generated release PR do not by themselves make the release host-sensitive.

This separates two questions: the release PR and deterministic tests prove the
klin change is fit to merge; the host ledger records whether current external
hosts still honor klin's integration.

### Recovery

If preparation fails before the PR exists, re-run it. If cargo-dist fails
after the tag exists, do not move the tag: fix or retry the failed Release
workflow against the same commit. An existing tag at any other commit is a
hard error.

### Consequences

- `main` contains every release before a release tag is created.
- Branch protection stays authoritative; release automation needs no bypass of
  `main`.
- The release workflow remains generated and owned by cargo-dist; klin uses
  cargo-dist's normal GitHub Release creation path.
- The exact tag build still runs fmt, clippy, nextest, build and
  `klin gate --strict`. Because the tagged commit is already on `main`, the
  release PR's required `quality / gates` run is the authoritative diff gate.
- Homebrew, npm or other future dist publishers can use the same tag-triggered
  model without reintroducing a separate promotion phase.
