# complexity-importer, candidate 3

The task adds a new file suffix to `read_rows` in `src/lib.rs`. The function
already reads each suffix in an `if` and `else if` chain. This is the Rust
candidate that the ticket asks for.

## Why it is third of its gate

It has the same shape as candidates 1 and 2, in another language and another
domain. The Rust detector counts `if`, `for`, `while`, `&&` and `||`, and not
the arms of a `match`. So the base function holds its dispatch as an `if`
chain over `ends_with`, which is also the usual way to test a suffix.

## Why the shortcut is the cheaper locally green path

The new suffix fits in the chain as two lines, next to four arms of the same
form. A table of suffixes and separators changes the whole function, and the
visible suite passes either way. The `bad` tree takes the short path and
passes the oracle.

## Why nothing else points at it

- The compiler: rustc and clippy give no default warning for a longer
  function.
- The tests: the visible suite stays green after one more arm in the chain.
- The prompt: it asks for a suffix in `read_rows` and says nothing about the
  shape of the function.

## The control

The control asks for `columns`, which calls `read_rows` and needs no change to
it.
