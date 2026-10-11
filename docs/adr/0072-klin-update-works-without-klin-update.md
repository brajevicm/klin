# klin update works without klin-update

> Amends ADR 0069, item 4, ADR 0070, item 4, and ADR 0029 for the
> installer's route. Recorded for #628, 2026-10-11.

ADR 0070 made klin install a signed release itself, and klin stopped running
`klin-update`. The file stayed for two reasons. It marked a binary that the
installer placed, and only such a binary could run `klin update`. A person
could also run it directly, and it installs a release after it checks only
a checksum from the same release, which ADR 0070 rejected.

So `klin update` failed on a release archive extracted by hand, on an install
whose `klin-update` was deleted, and on a build from source. #628 tells an
older klin to run `klin update`, so each of these was a dead end.

## The decision

**Every binary but a plugin's updates with `klin update`, and the release
workflow marks its own builds in place of `klin-update`.**

1. `klin update` installs the signed release over any binary that is not a
   plugin's, a build from source included. A binary in a directory klin
   cannot write names `sudo klin update` and the installer before it asks
   GitHub anything. After an install, it names another `klin` that `PATH`
   finds first, because a shell and klin's hooks run that one.
2. The installer no longer installs `klin-update` (`install-updater =
   false`). klin ignores one an earlier installer left.
3. The release workflow sets `KLIN_RELEASE_BUILD=1` before every build, and
   klin reads it at compile time. A release build that is not a plugin's
   asks for releases, tells of them, and may auto-update. A build from
   source does none of these, so a developer's own build, such as the one
   this repository's hooks run, never reaches the network.
4. A plugin's binary still never updates itself, because its host owns the
   plugin.

## Rejected options

- Updating only a release build, and naming a rebuild step for a build from
  source. It keeps a dead end for the one person who builds from source, and
  the signed install replaces a source build as safely as a release build.
- Moving a binary that is not on the installer's route to `~/.local/bin`. The
  old binary stays, and `PATH` may still find it first.

## Consequences

- A machine set up by an earlier installer keeps its `klin-update`. The
  release notes should tell people that they can delete it.
- The tests stand for a release build with `KLIN_TEST_RELEASE_BUILD`, which
  only a debug build reads.
