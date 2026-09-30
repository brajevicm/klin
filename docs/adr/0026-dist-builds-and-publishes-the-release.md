# dist builds and publishes the release

> ADR 0029 amends this (#338): with `create-release = false`, `dist` fills
> and publishes a draft prerelease that `cut-release` makes, and the release
> becomes Latest at its promotion.
>
> #368 amends the paragraph on `dist plan`. The generated workflow skips pull
> requests (`pr-run-mode = "skip"`), and `release-plan.yml` runs `dist plan`
> on the pull requests that touch a release input. The jobs in `release.yml`
> keep the GitHub default timeout of 360 minutes. dist 0.32 has no setting for
> a job timeout, and a hand edit would end the drift check of `dist plan`.

`dist` (formerly `cargo-dist`) owns the release pipeline. A pushed tag runs
the workflow `dist` generates, which builds the four targets, writes the
checksums, generates the install script, and publishes the GitHub release.
klin writes no build or upload steps and no install script of its own.

The cost is that `dist` names the artifacts, not klin. Section 19.1 once
asked for four bare binaries called `klin-<os>-<arch>`, one `SHA256SUMS`, and
an `install.sh` in the repository taking `--version` and `--dir`. `dist`
publishes `klin-<target-triple>.tar.xz` archives, a `.sha256` beside each,
one `sha256.sum`, and a `klin-installer.sh` that pins its own version and
reads `KLIN_INSTALL_DIR`. The `dist` shapes win, because a route that
consumes them stays on the tool's supported path, and the tool also generates
the Homebrew tap and the npm package that routes 2 and 3 of 19.1 need.

`dist plan` runs on every pull request through the generated workflow, so a
release configuration that no longer resolves fails before a tag exists. The
version inside the binary comes from the `version` key in `Cargo.toml`, which
the tag must match, so a release is a version bump and a tag.
