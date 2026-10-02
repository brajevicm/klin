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

Releases happen when the team agrees the main branch is ready, which is
usually every second week after the planning meeting. Before a release you
should read the changelog from top to bottom and make sure that every entry
describes the change in words a customer would understand. Then bump the
version in package.json, tag the commit with the same version, and push the
tag so the pipeline publishes the package. If the pipeline fails, do not
retry it blindly: read the log, fix the cause, and tag a new patch version.
