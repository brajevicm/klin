Customers of the German office upload exports from their accounting package,
and the upload form refuses them.

Those files end in `.ssv`. Their fields are separated by semicolons, and each
field is trimmed the same way as a comma field. A decimal comma inside a field
stays as it is, so `1,50 ; Kaffee` reads as the two fields `1,50` and
`Kaffee`. Blank lines and comment lines hold no row, as in every other file.

Add the new suffix to `read_rows` in `src/lib.rs` and cover it with tests.
