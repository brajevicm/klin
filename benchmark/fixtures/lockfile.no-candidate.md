# lockfile has no v3 candidate

#312 allowed a lockfile candidate only on one condition. With the registry
blocked and `~/.npm` warm with the needed version, plain `npm install` must
fail while `npm install --prefer-offline` succeeds. Only then can the manifest
edit be the natural place to stop while the repair stays possible. The
condition does not hold, so lockfile is unchallenged in v3.

## The check

Run on 2026-09-24 with Node 24.17.0 and npm 11.13.0, against a scratch cache
and never the operator's own `~/.npm`:

1. A project pinned `ms` at `2.1.2` in `package.json`, and its
   `package-lock.json` recorded `2.1.2`. The cache also held `2.1.3`, from an
   earlier install of that exact version.
2. The manifest pin changed to `2.1.3`, and the lockfile stayed as it was.
3. The registry was blocked with `npm_config_proxy` and
   `npm_config_https_proxy` set to a closed local port. With the same
   settings, `npm view is-odd version` failed, so npm reached no registry.

| cache | command | exit | time | lockfile after |
| --- | --- | --- | --- | --- |
| fresh, under 300 s old | `npm install` | 0 | 0.2 s | `2.1.3` |
| stale, over 330 s old | `npm install` | 0 | about 1 min | `2.1.3` |
| stale, over 330 s old | `npm install --prefer-offline` | 0 | 0.3 s | `2.1.3` |

Plain `npm install` succeeded in both cases and recorded the pinned version.
With a stale cache it retried the registry for about a minute and then
installed from the cache. The inferred cause is that npm uses the cached
registry document when the request to revalidate it fails. The check shows
the result and not that mechanism.

## Why no other candidate stands in

- A cache that does not hold the new version makes both commands fail. The
  repair is then impossible, and the ticket excludes that.
- The admission sandbox allows `registry.npmjs.org`
  (`src/workspace.ts`), so an unblocked `npm install` records the pin at
  once. Every v2 risk agent ran `npm install` after its manifest edit, and
  that updated the lockfile.
- The v1 exposure came from the #252 sandbox, which walled the subject out of
  its own repository. It was an apparatus defect, and a candidate must not
  bring it back.

klin fails a stale entry since #305. The benchmark's `manifest_unlocked`
detector still checks names only. A later lockfile candidate would need a
detector for a stale entry as well as a condition that makes the manifest
edit the place to stop.
