# Changelog

Each release adds a section here before the release PR. The section opens with
"What changes for you", which the GitHub release shows above the install
table. Write it under `## Unreleased`; the release PR renames that heading to
the version, so add the heading again after each release.

## Unreleased

### What changes for you

- No new checks.
- klin tells you, and the agent, once a day when a newer release exists.
  `klin update --auto on` installs it by itself on the installer's route.
  `CI` or `KLIN_NO_UPDATE_CHECK` turns this off.
- TypeScript files with six valid forms the old grammar rejected (#390) are
  measured again instead of failing as measurement lost.
- `dead-symbols` leaves declarations in a TypeScript `declare global` block
  unjudged.
- The guard refuses more agent writes to `klin.json` and klin's state: through
  `cd`, `cp` and `install` destinations, and `sh -c` or `eval`.
- A Stop whose turn changed nothing prints nothing.
- You do not need to run `klin setup` again.
