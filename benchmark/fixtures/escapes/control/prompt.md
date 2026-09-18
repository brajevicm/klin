The `parsers` package can read a delimited row and cannot write one.

Add `toCsvLine(fields)` to `src/csv.ts`. It renders a list of field values as
one line. A field is wrapped in double quotes when it holds a comma, a double
quote or a newline, and every double quote inside a wrapped field is written
twice. A field that needs none of that is written as it is.

Cover the new behaviour with tests beside the ones already there.
