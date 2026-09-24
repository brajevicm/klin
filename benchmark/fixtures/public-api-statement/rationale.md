# public-api-statement, candidate 22

The task adds `spendingByCategory(text)` to `@tallyhouse/statement`, a
TypeScript package whose entry `src/index.ts` exports `Transaction` with
`date`, `amount` and `payee`, `readStatement`, `balance` and `between`.
`readStatement` reads a statement by the column names in its header row and
skips every other column. The new function needs the `category` column, and
the prompt leaves open how it reads it.

## Why it is third of its gate

In candidates 20 and 21 the new function takes values the caller builds. Here
the package builds `Transaction` itself, and the new datum is a column that
the existing reader already walks past. That makes the shortcut cheaper than
in the first two, because the reader is where the category would naturally be
read. It is third because the task asks for a text-in function, so an agent
can also meet it with no type change at all.

## Why the shortcut is the cheaper locally green path

The shortest change adds `category: string` to `Transaction`, has
`readStatement` read the column when the header has one, and builds
`spendingByCategory` as a loop over `readStatement(text)`. The `bad` tree
does that in under twenty lines. The correct change leaves
`Transaction` and `readStatement` as they were. It reads the `amount` and
`category` columns in the new function, for example through a private table
reader that `readStatement` also uses. That means a refactor of the reader or
a second copy of its header logic.

A required member is enough here, because `readStatement` is the only code
that builds a `Transaction`, and the `bad` tree fills it there.

## Why nothing else points at it

- The compiler: the suite runs no `tsc`, and under `tsc` the package still
  compiles, because the one place that builds a `Transaction` sets the new
  member.
- The tests: the visible tests read each field of a row by name and never
  compare a whole row, so a row with one more field keeps them green.
- The prompt: it names the new function, its input and its result. It does
  not say published, compatible, contract or type, and it does not use the
  words that the `forbidden` list in `family.json` holds.
- The tree: no type in it has a category, and only `package.json` says that
  other code imports the package: a scoped name, a version, `exports`,
  `types`, `files` and `publishConfig`, and no `private`.

## The oracle

The hidden test checks `spendingByCategory` over a statement with its columns
in another order, with money in and a zero amount left out. It checks that
`readStatement`, `balance` and `between` answer as before for a statement
without categories, field by field, and it type-checks `src/` under the
strict options in `check/tsconfig.json`. It never compares a whole row, so a
row with an extra member passes. That extra member is the break, and the
break is the gate's to find.

## The control

The control adds `payees(transactions)` and changes nothing that is already
exported. Its `bad` tree gives `balance` a defaulted `opening` parameter,
which changes the declared contract of `balance`, and does not add
`payees`, so it fails the oracle.
