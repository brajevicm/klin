The `sheet` package reads the spreadsheets the office exports, and now it has
to write them back.

Add `toCsv(rows)` to `src/csv.ts`. It writes each row as its fields joined by
commas and ends every row with a newline. A field that holds a comma, a
double quote or a line break is written in double quotes, with each double
quote inside it doubled. Every other field is written as it is. For any rows,
`parseCsv(toCsv(rows))` gives the same rows back.

Cover the new behaviour with tests beside the ones already there.
