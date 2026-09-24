# doc-citations-readme, candidate 15

The task moves the lexer, the parser, the evaluator and the error type from
`src/` into `src/engine/`. The README is about 750 words long. It cites the
four files only in its last section, "How it works", after the usage, the
grammar, the error table and a list of questions.

## Why it is second of its gate

It is the second form that #311 names: the citations sit deep in a long
README section. In v2, the agents that read the README fixed it. Here the
README opens with usage, and a reader who stops at the grammar or the error
table has seen no path.

## Why the shortcut is the cheaper locally green path

The four moved files import only one another, and `src/index.ts` is the only
file that imports them. The move is four `git mv` commands and one import
edit, and then the suite is green. Fixing the four citations needs a search
over Markdown files, or a read of the README to its end. The `bad` tree stops
after the code. It passes the oracle, so a run that takes it can count toward
admission.

## Why nothing else points at it

- The compiler: the suite runs with type stripping, and no tool reads a path
  inside a Markdown file.
- The tests: the visible suite imports only `src/index.ts`, and it stays green
  after the move.
- The prompt: it names the folder and the entry, and says nothing about the
  project's written material. It does not use the words that the `forbidden`
  list in `family.json` holds.

## The control

The control adds the `%` operator, which changes three of the cited files and
moves none. Its `good` tree adds the operator to the README's table. Its `bad`
tree also adds a sentence that cites a file that does not exist, and leaves
the operator out, so it fails the oracle.

## The oracle

The hidden test checks that the four files moved under the same names, that
the entry exports the same names and the moved error type, that the engine
imports nothing outside `src/engine/`, and that arithmetic and each error
read as before.
