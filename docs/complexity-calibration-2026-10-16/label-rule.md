# Label rule for complexity findings (#585), fixed before labeling

Each finding is one function that klin's `complexity` check reported in one
change (a commit against its first parent). Give it one label:

- `appropriate`: a careful reviewer of this change, reading the function as
  the change left it, would ask the author to simplify the function's
  structure (split it, flatten it, or extract logic from it), and the request
  is about what this change did to the function.
- `not-appropriate`: the reviewer would not ask, or would ask only about code
  this change did not touch.

How to apply the test:

1. Judge structure: branches, nesting, mixed responsibilities, and length that
   comes from logic. Do not judge style, naming or types.
2. `worsened` (the function existed before and klin says it got worse): judge
   what the change added to it. A change that adds a responsibility, a branch
   or tangled logic to a function that is already hard to read is
   `appropriate`. A change that only moves, reorders, renames, reformats or
   retypes code that was already there is `not-appropriate`.
3. `new` (no matching function before): judge the whole function as written.
   A function that the change moved or copied unchanged from another place is
   judged as in rule 2, with nothing added.
4. Decide from the code. klin's numbers and ceilings are context, not the
   answer.
5. Write one line of reason that says what the change did to the function and
   why a reviewer would or would not ask.

Added 2026-10-16, after the first labeling pass, to settle a split between
labelers (F069 against F103, F122, F126):

6. An early-return guard, or a try/catch around existing lines, that adds no
   nesting does not tangle a function. A change that adds only such a guard is
   `not-appropriate`.
