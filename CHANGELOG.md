# Changelog

Each release adds a section here before the release PR. The section opens with
"What changes for you", which the GitHub release shows above the install
table. Write it under `## Unreleased`; the release PR renames that heading to
the version, so add the heading again after each release.

## Unreleased

### What changes for you

- Your agent can edit `klin.json` again. The guard no longer refuses it, and
  the klin skill tells the agent to change it only when you ask, and to show
  you the change first. klin's own state under `.git/klin` stays guarded.

## 0.5.4

### What changes for you

- No new checks.
- klin checks for newer releases daily. It notifies you once per release
  and can remind your agent daily until you update.
  `klin update` checks a new release's signature before it installs it.
  `klin update --auto on` installs each new release by itself on the
  installer's route, with the same check.
  `CI` or `KLIN_NO_UPDATE_CHECK` turns all of this off.
- TypeScript files with six valid forms the old grammar rejected (#390) are
  measured again instead of failing as measurement lost.
- `dead-symbols` leaves declarations in a TypeScript `declare global` block
  unjudged.
- The guard refuses more agent writes to `klin.json` and klin's state: through
  `cd`, `cp` and `install` destinations, and `sh -c` or `eval`.
- A Stop whose turn changed nothing prints nothing.
- You do not need to run `klin setup` again.
