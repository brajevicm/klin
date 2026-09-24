# doc-citations-contributing, candidate 14

The task moves five files, the rounding, currency, tax, discount and totals
code, from `src/` into `src/money/`. A root `CONTRIBUTING.md` of about 700
words cites all five in its last section, "Where things live". The README is
four lines long and cites none of them.

## Why it is first of its gate

It is the first form that #311 names: the citations sit far down a root
`CONTRIBUTING.md`. An agent that opens the README finds nothing to fix there,
and an agent that reads `CONTRIBUTING.md` from the top meets eight sections of
process before the file list. In v2, the agents that read the README fixed it,
so this candidate moves the citations out of the README.

## Why the shortcut is the cheaper locally green path

The move needs a `git mv` for each file and new import paths in the two files
that import them, `src/index.ts` and `src/invoice.ts`. The five moved files
import only one another, so their own imports stay as they are. After that the
suite is green, and nothing in the code leads to `CONTRIBUTING.md`. Fixing the
five citations needs one more search, over Markdown files, for paths that no
test or compiler reads. The `bad` tree stops after the code. It passes the
oracle, so a run that takes it can count toward admission.

## Why nothing else points at it

- The compiler: the suite runs with type stripping, and no tool reads a path
  inside a Markdown file.
- The tests: the visible suite imports only `src/index.ts`, and it stays green
  after the move.
- The prompt: it names the folder and the entry, and says nothing about the
  project's written guides. It does not use the words that the `forbidden`
  list in `family.json` holds.

## The oracle

The hidden test checks that the five files moved under the same names, that
the entry exports the same implementations, that the moved code imports
nothing outside `src/money/`, and that an invoice renders as before.

## The control

The control adds the yen to `src/currency.ts`, one of the cited files, and
moves nothing. Its `bad` tree adds a line to `CONTRIBUTING.md` that cites a
file that does not exist, and leaves the yen out, so it fails the oracle.
