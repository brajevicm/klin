# Changelog

Each release adds a section here before the release PR. The section opens with
"What changes for you", which the GitHub release shows above the install
table. Write it under `## Unreleased`; the release PR renames that heading to
the version, so add the heading again after each release.

## Unreleased

### What changes for you

- Your agent can edit `klin.json` again. The guard no longer refuses it, and
  the klin skill tells the agent to change it only when you ask. The agent
  shows you the diff, and asks first when it picks a value itself. A command that deletes or moves `klin.json` still
  asks you first, because without it klin is off. klin's own state under
  `.git/klin` stays guarded.

## 0.5.5

### What changes for you

- A new `secrets` check fails a new literal shaped like a live vendor token:
  OpenAI, Anthropic, Stripe, GitHub, AWS, Slack, Google or GitLab. A finding
  shows only the vendor prefix and a hash of the token, so the key itself
  stays out of klin's output. The check also reads `.env` files that git does
  not ignore.
- New TypeScript and JavaScript escape rows: `as unknown as`, `any` as a
  generic argument, `biome-ignore`, `/* eslint rule: ... */` comments,
  `NOSONAR`, `@codescene(disable` and empty `catch` handlers. Test files may
  still use `ts-expect-error`, double casts and empty handlers.
- `stubs` catches placeholder comments such as "in a real implementation" or
  "replace with actual".
- Some findings now ride along and never block a Stop on their own. A Stop
  shows them only when something else blocks, and `klin check` lists them as
  review items without changing the exit code. `biome-ignore` is the first.
- `reachability` judges a Next.js app from its routes and entry files. It
  reports files that no entry reaches and exports that no reached file
  imports.
- `layering` accepts `"acyclic": true` without `layers`, so it can fail new
  import cycles alone. TypeScript imports now resolve through `baseUrl`,
  through a tsconfig's own `paths` under `extends`, and through competing
  tsconfigs that agree. A literal `import()` counts as an import.
- `lockfile` reads `bun.lock`.
- `public-api` says nothing about a package with no entry point, such as a
  Next.js app.
- `klin policy --suggest` prints a starter policy for your tree and writes
  nothing. `klin.json` accepts `$schema`.
- `klin setup` ends by telling you what debt klin holds as the floor and
  which checks judge nothing in this tree. `klin policy` shows what each
  check judges.
- The Stop text is shorter. It lists at most 20 new sites per gate and gives
  each escape kind its own remedy. `klin check` still lists every site.
- On a branch cut from another feature branch, `klin check` infers that
  branch as the base, so the parent's commits no longer count as new.
  `git config branch.<name>.klin-base REF` sets the base by hand.
- You do not need to run `klin setup` again.

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
