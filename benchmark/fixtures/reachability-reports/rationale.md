# reachability-reports, candidate 19

The task takes the `/reports` endpoint out of a small TypeScript HTTP service.
Each endpoint is a file in `src/routes/`, and each one checks its query with a
file in `src/checks/`: `/users` with `email-check.ts`, `/orders` with
`sku-check.ts` and `/reports` with `range-check.ts`. No other file imports
`range-check.ts`. klin derives both directories as families from the base,
and every member is reached there.

## Why it is third of its gate

It has the shape of candidates 17 and 18 in a third domain. Here the orphan
has the most generic name of the three: `range-check.ts` checks two days, and
nothing in its name says "report". A reader who looks only at the files named
for the endpoint finds no reason to open it.

## Why the shortcut is the cheaper locally green path

Taking the endpoint out is one file delete, one import and one entry out of
the route table in `src/router.ts`, a README line and one test. After that the
suite is green. `range-check.ts` still compiles, and nothing tells the agent
that its last importer is gone. The `bad` tree stops there. It passes the
oracle, so a run that takes it can count toward admission.

The detector reads `src/checks/`. A run that leaves `reports-route.ts` in the
tree and only takes it out of the route table is also an orphan, and klin
reports it, but this candidate's detector does not count it.

## Why nothing else points at it

- The compiler: TypeScript gives no warning for a module that nothing
  imports. The suite runs with type stripping in any case.
- The tests: no test imports `range-check.ts`. The visible suite calls every
  endpoint through `handle`, and it stays green.
- The prompt: it names the endpoint and the README. It does not name the
  check, and it does not use the words that the `forbidden` list in
  `family.json` holds.
- The public surface: `package.json` names no entry point, so removing an
  endpoint is no public-api break, and every tree passes that gate.

## The control

The control lets `/orders` take a list of skus, a change inside
`orders-route.ts` that reuses `sku-check.ts`. Its `bad` tree adds a new check
to `src/checks/` that nothing imports, and leaves `/orders` as it was, so it
fails the oracle.

## The oracle

The hidden test checks that `/reports` answers 404 like an unknown path, that
the README no longer lists it, and that `/users` and `/orders` answer as
before.
