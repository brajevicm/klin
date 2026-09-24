# Contributing to invoicing

Thank you for helping. This guide says how a change gets from your machine to
a release. Read it once before your first change, and come back to the part
you need later.

## Before you start

Open an issue before a change that alters what an invoice prints. A printed
invoice is a legal record in most of the countries we bill, so the finance
team reviews every change to its wording, its layout or its numbers. A change
that only moves code, or adds a test, needs no issue.

Keep one change to one purpose. A pull request that fixes a rounding bug and
also renames a function is two pull requests.

## Setting up

The package has no runtime dependencies. You need Node.js 22 or later, because
the suite runs the TypeScript sources directly with Node's type stripping.

1. Clone the repository.
2. Run `npm test`. Every test must pass before you change anything.

There is no build step. Nothing is compiled or bundled, and the package entry
is the TypeScript source itself.

## Branches and commits

Work on a branch named after the issue, such as `412-reduced-rate-rounding`.
Rebase on `main` before you ask for a review, and do not merge `main` into
your branch.

Write each commit message in the imperative mood, with a subject line of at
most 60 characters. Say in the body why the change is needed. The diff
already says what changed.

## Money rules

These rules hold everywhere in the package. A change that breaks one of them
is refused in review, whatever else it does.

- An amount is a whole number of cents. Nothing stores a fraction of a cent,
  and no function returns one.
- Rounding happens once per line, when a rate or a percentage is applied.
  Totals add rounded amounts and never round again.
- A half cent rounds to the even neighbour. Finance chose this because it
  does not drift upwards over thousands of lines.
- A currency is printed with its symbol before the number and two decimals.
  A negative amount prints its sign before the symbol.

## Tests

Every change comes with a test that fails without it. Put a test next to the
code it covers, in a file whose name ends in .test.ts.

A test that pins a printed invoice compares the whole text, not a fragment
of it. The finance team reads those tests as the specification, so write the
expected text out in full.

Do not use random numbers or the current date in a test. A test must give the
same answer on every machine and on every day.

## Code style

Name a function after what it returns. Prefer a plain function over a class.
Export a type with `export type`, so that type stripping can remove it.

Keep a function short enough to read without scrolling. When one grows past
that, split it at a step that has its own name.

## Review

Ask for a review from one person on the billing team. A change to rounding,
tax or discounts also needs a review from finance. Say in the pull request
which of the money rules above the change touches.

A reviewer answers within two working days. When they ask for a change, push
a new commit rather than amending, so they can see what moved.

## Releasing

A release is a tag on `main`. The billing team tags a release every other
Tuesday, and the invoicing service picks it up the next morning. Do not tag a
release yourself.

Write one line for the changelog in your pull request description. Say what
a reader of an invoice will see differently, or say that nothing changes for
them.

## Where things live

The package entry is `src/index.ts`, and it is the only file a caller
imports. Every other file is internal.

- The printed form of an invoice is built in `src/invoice.ts`.
- Line totals, their tax and the invoice total are added up in
  `src/totals.ts`.
- Tax bands and their rates are in `src/tax.ts`. A new band starts there, and
  it needs a review from finance.
- Percentage discounts are applied in `src/discount.ts`.
- Half-even rounding is `src/rounding.ts`. Every rounded amount goes through
  it.
- Currency symbols and the printed form of an amount are in
  `src/currency.ts`.
- The number of decimals each currency prints is in
  `src/currency/decimals.ts`.
