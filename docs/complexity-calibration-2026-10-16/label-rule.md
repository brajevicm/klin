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

1. Judge structure: branches, nesting, mixed responsibilities and length. Do
   not judge style, naming or types.
2. `worsened` (the function existed before and klin says it got worse): judge
   what the change added to it. A change that adds a responsibility, a branch
   or tangled logic to a function that is already hard to read is
   `appropriate`. A change that only moves, reorders, renames, reformats or
   retypes code that was already there is `not-appropriate`.
3. `new` (no matching function before): judge the whole function as written.
   The packet may show the most similar function in the parent commit. Decide
   what it is, with git if needed. If it is an earlier version of this
   function, or code this change moved or extracted into it, judge as in
   rule 2, on what the change added. If this function copies code that still
   exists after the change, judge the whole copy as new code. If it is only a
   similar function, ignore it.
4. Decide from the code. The packet does not show klin's numbers, so they
   cannot anchor the label.
5. Write one line of reason that says what the change did to the function and
   why a reviewer would or would not ask.

Added 2026-10-16, after the first labeling pass, to settle a split between
labelers (F069 against F103 and F126). It holds for the second pass:

6. An early-return guard, or a try/catch around existing lines, that adds no
   nesting does not tangle a function. A change that adds only such a guard is
   `not-appropriate`.

## History

- 2026-10-16, before any label: the owner chose this reviewer test. The first
  pass used rules 1 to 5 in an earlier form: rule 1 judged "length that comes
  from logic", rule 3 judged a moved or copied function on what was added,
  and the packets showed klin's `cc` and `lines`.
- After the first pass: the labelers split on guard clauses, and the owner
  chose rule 6.
- The second pass relabeled every finding under this text. An adversarial
  review found that 12 first-pass packets showed an empty "before" for
  functions in renamed files, and that moved functions came with no source.
  The second-pass packets show the real "before", the most similar function
  in the parent commit, and no klin numbers. Rule 1 judges length without
  qualification, and rule 3 tells a move from a copy.
- The cap findings of #343 (`findings-cap-343.tsv`) carry first-pass labels.
