# Changelog

Each release adds a section here before the release PR. The section opens with
"What changes for you", which the GitHub release shows above the install
table. Write it under `## Unreleased`; the release PR renames that heading to
the version, so add the heading again after each release.

## Unreleased

### What changes for you

- `complexity` no longer blocks on a `cc` ceiling that klin derives. A
  function over it is an `INFO` note at `klin check`, and the Stop does not
  measure it. A `cc` you pin in `klin.json` still blocks, including one that
  `klin setup --pin` or `klin policy --suggest` wrote. When `klin.json` pins
  no complexity value, the Stop reads no file for `complexity`.
- `complexity` moves to semantics version 2. `klin report` shows a
  derived-`cc` regression from before the upgrade as not compared, and does
  not count it as fixed.
- An asset import, such as `import Logo from "../logo.svg?react"`, the same
  path through a `paths` alias, or a `./styles.css` the tree does not hold, is
  no longer a hole. klin strips a `?query` suffix, and a path that ends in an
  asset extension, such as an SVG, PNG, CSS or JSON file, counts as external
  or unsupported. So does any path with a `?raw`, `?url` or `?inline` query,
  unless the query also asks for a worker.
  `./worker?worker` and a missing `./user.service` are still holes.

## 0.5.6

### What changes for you

- Your agent can edit `klin.json` again. The guard no longer refuses it, and
  the klin skill tells the agent to change it only when you ask. The agent
  shows you the diff, and asks first when it picks a value itself. A command
  that deletes or moves `klin.json` still asks you first, because without it
  klin is off. klin's own state under `.git/klin` stays guarded.
- You can ask your agent to tailor `klin.json` to your repository. The skill
  has it run `klin policy --suggest`, explain each value, and write only what
  you pick or approve. With no `klin.json`, the agent asks first, because the
  file turns klin on. `klin setup` now ends by naming this request.
- When a change edits `klin.json`, `klin check` names the findings the edit
  hides. It judges a second time under the base's `klin.json` and lists each
  finding that only the base's policy fails as a `config-loosened` review
  item. The exit code still follows the tree's `klin.json`, so you can accept
  debt in the same PR. A first `klin.json` and a deleted one both count as a
  change.
- `klin policy` refuses an `accepted` entry that leaves out a number its gate
  ratchets. It exits 2 with the error `klin check` gives. Before, only
  `klin check` failed.
- A skipped or focused JavaScript or TypeScript test, such as `it.skip(` or
  `test.only(`, still counts as a test. `escapes` reports the skip, and
  `inventory` no longer reports the test as gone.
- The Stop shows a test that went away as a question. The row reads
  `ASK   inventory`, and when the question is all the Stop holds, it tells the
  agent to answer each ASK. The JSON output does not change.
- klin writes nothing into the agent's context at session start or at a
  prompt, except the release notice. After a wide turn, the radius note goes
  to you at the next prompt on Claude Code and Codex. Cursor has no channel to
  you, so there the note goes to stderr.
- `klin report` no longer prints "The agent said why.", since klin never reads
  the reply. `klin status` names each open finding as `file:line` and shows
  times as ages. A FAIL's "beyond the N the base holds" counts only base sites
  that fail. The Stop's turn-end line counts only regressions flagged since
  the current prompt.
- Run `klin setup` once to get the new klin skill. The plugin carries it in
  its update.

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
