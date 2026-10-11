# klin installs only a release signed by its workflow

> Amends ADR 0069, item 2, and ADR 0029 for the installer's route.
> Recorded for #627, 2026-10-11.

The installer and `klin-update` checked an archive against a SHA-256 from
the same release. A bad release carries a matching checksum, so that check
does not stop it, and auto-update would deliver a bad release to every machine
that turned it on within a day.

## The decision

**The release workflow signs each archive, and klin installs an archive only
after it checks that signature. Auto-update stays opt-in.**

1. cargo-dist attests every release artifact in the host job (GitHub artifact
   attestations). Each attestation names this repository's release workflow,
   and `gh attestation verify` checks it for a person who wants to.
2. The `sign-release` job, between the builds and the host job, signs each
   `klin-TARGET.tar.xz` with minisign. The secret key is the repository
   secret `KLIN_RELEASE_SIGNING_KEY`, made with `minisign -G -W`, so it has
   no password and the job signs without a prompt. The trusted comment is
   `klin VERSION TARGET`. The job checks each signature against the public
   key in the tree, so a key that does not match fails the release before
   anything publishes. The host job publishes only after this job succeeded.
   dist generates a condition that also accepts a skipped job, so
   `release.yml` carries that one hand edit under `allow-dirty = ["ci"]`, and
   `.github/check-release-workflow.py` fails quality CI on any other
   difference from what dist generates.
3. klin carries the public key (`src/sys/release.pub`). `klin update` and the
   auto-update child download the archive and its `.minisig` for the exact
   tag through `curl`. They check the signature and the trusted comment, and
   only then extract the archive and rename its `klin` over the running
   binary. A failed check installs nothing. `klin update` prints why and
   exits 2. The child keeps the reason, and `klin status` prints it with the
   available release. Both hold `release-ask.lock` while they install, and
   both read the version of the klin on disk first, so an install never
   replaces a newer klin that another install put there. A failure that may
   pass, such as a download that did not finish, is tried again with the
   next day's ask. A refused release is not.
4. klin installs itself and no longer runs `klin-update`, which cannot check
   a signature. `klin-update` beside the binary stays the mark of the
   installer's route.
5. Auto-update stays off until the person turns it on with
   `klin update --auto on` or `KLIN_AUTO_UPDATE=1` (owner decision,
   2026-10-11, which departs from #627's plan to turn it on by default). A
   binary that replaces itself is a choice a person makes, and the signature
   makes that choice safer. Off keeps the notice of ADR 0069.

## Rejected options

- Attestations alone, checked with `gh attestation verify`. It is the least
  work for the author, but a machine without a logged-in `gh` could not
  update on its own, so auto-update would reach few people.
- Attestations alone, checked inside klin. Sigstore needs the Fulcio chain,
  the workflow identity and a Rekor entry, which brings large crypto
  dependencies into the binary.
- Checking the signature and then running `klin-update`. The updater
  downloads again, and klin would rely on the release staying the same
  between the two downloads.

## Consequences

Anyone who can read `KLIN_RELEASE_SIGNING_KEY` can sign a release that klin
installs. A new key needs a new klin, because the public key is built in.
The trusted comment names the version and the target, so a signed archive
of an older release, or of another machine's build, never installs. A debug build reads a test key from
`KLIN_RELEASE_KEY`, because the tests cannot sign with the release key. A
release build never reads it. klin depends on `minisign-verify`, which has
no dependencies of its own. A person without `curl`, `tar` or `xz` gets no
update and keeps the notice.
