# Working on shop

Run `npm test` before you stop. Keep each change to one concern.

## Layout

- `src/` holds the application code.
- `tests/` holds the tests, one file per module.

## Rules

Do not add a dependency without a reason in the commit message. Prefer the
standard library. Keep functions short and name them for what they return.
Write a test for every bug you fix, and make the test fail before the fix.

## Releases

Read the changelog, bump the version, tag, push the tag. On a failed pipeline, fix the cause and tag a patch.
