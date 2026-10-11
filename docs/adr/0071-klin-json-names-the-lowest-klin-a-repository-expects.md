# klin.json names the lowest klin a repository expects

> Amends ADR 0040 on the retired `version` key. Recorded for #628,
> 2026-10-11.

Teammates run different klin versions on one repository, and CI runs the
version its workflow pins. A newer klin may add a check an older one does not
run, so two people can get different verdicts on the same tree. ADR 0040
retired the `version` key that `init` wrote, because a binary version froze
no semantics.

## The decision

**An optional `"klin": ">=X.Y.Z"` names the lowest klin the repository
expects. It is a floor, not a pin, and it changes no verdict.**

- An older klin says so in `klin status` and once in a Stop's notice, with
  the step that installs a newer one. With auto-update on, it asks for a
  release at once instead of at the day's ask (SPEC 11.9).
- The GitHub Action fails the job when the klin it installed is older
  (SPEC 12.3). CI is where two versions most often meet.
- klin reads the key before it judges the rest of the file, so an older klin
  that does not know a newer key still names the version to install.
- Only `>=` is accepted. A range or an exact pin would freeze the binary,
  which ADR 0040 rejected.

`version` stays retired. Its error now names `klin` as the key to use.

## Consequences

- A team raises the floor in a reviewed commit to `klin.json`, like any
  other policy.
- A klin built before this key refuses a configuration that names it as an
  unknown key. That error also tells the person to update.
