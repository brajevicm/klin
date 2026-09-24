The stock report is hard to read because the counts line up on the left.

Change `render` in `src/lib.rs` so that a column whose cells are all whole
numbers, made only of the digits 0 to 9, lines up on the right. Every other
column lines up on the left, as it does today. A row that ends before a
column has no cell in it, and that missing cell does not change how the
column lines up.

Cover the change with tests beside the ones already there.
