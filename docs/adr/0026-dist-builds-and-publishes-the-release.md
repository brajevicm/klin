# dist builds and publishes the release

> ADR 0029 amends this (#338), and #470 changes the release topology again:
> cargo-dist owns GitHub Release creation (`create-release = true`) after the
> protected release PR merges and `publish-release` pushes the immutable tag.
>
> #368 moved `dist plan` out of the generated workflow because
> `pr-run-mode = "skip"`. #470 folds that validation into the required
> `quality / gates` job whenever release-sensitive inputs change, so a bad
> plan cannot merge and there is no second release-plan CI authority. The jobs
> in `release.yml` keep GitHub's default timeout of 360 minutes.
>
> #369 adds a check of the exact tag before a publish. `github-build-setup`
> puts the steps of `.github/verify-tag.yml` into the x86_64 Linux build job:
> fmt, clippy, the tests, a debug build and `klin gate --strict`. A failure
> fails `build-local-artifacts`, so `host` and `announce` do not run. dist's
> `plan-jobs` is not used, because a failed plan job skips the build jobs and
> `host` accepts skipped build jobs. `github-action-commits` pins the Actions
> of `release.yml` to commit SHAs.

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

`dist plan` runs conditionally inside the required `quality / gates` job
when a pull request changes release-sensitive inputs. A release PR also proves
that its seven generated files differ from the base only by the configured
version substitutions. A release configuration or generated release diff that
does not satisfy those checks cannot merge. The version inside the binary
comes from the `version` key in `Cargo.toml`, which the tag must match, so a
release is a version bump and a tag.
