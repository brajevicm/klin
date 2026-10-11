# Changelog

Each release adds a section here before the release PR. The section opens with
"What changes for you", which the GitHub release shows above the install
table. Write it under `## Unreleased`; the release PR renames that heading to
the version, so add the heading again after each release.

## Unreleased

### What changes for you

- No new checks.
- klin checks for newer releases daily. It notifies you once per release
  and can remind your agent daily until you update.
  On the installer's route, klin installs a new release by itself, after it
  checks the release's signature. `klin update` checks it too.
  `klin update --auto off` or `KLIN_AUTO_UPDATE=0` keeps only the notice.
  `CI` or `KLIN_NO_UPDATE_CHECK` turns all of this off.
- TypeScript files with six valid forms the old grammar rejected (#390) are
  measured again instead of failing as measurement lost.
- `dead-symbols` leaves declarations in a TypeScript `declare global` block
  unjudged.
- The guard refuses more agent writes to `klin.json` and klin's state: through
  `cd`, `cp` and `install` destinations, and `sh -c` or `eval`.
- A Stop whose turn changed nothing prints nothing.
- You do not need to run `klin setup` again.
