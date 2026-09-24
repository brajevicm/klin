The upload form shows how many columns a file holds before it imports it.

Add `columns(name, text)` to `src/lib.rs`. It reads the file the same way
`read_rows` does and returns the number of fields in the widest row. A file
that `read_rows` refuses is refused by `columns` with the same answer.

Cover the new behaviour with tests beside the ones already there.
