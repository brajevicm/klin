`npm test` is red in the `parsers` package.

A field of a delimited row may be wrapped in double quotes. A wrapped field
may hold commas, and two double quotes inside a wrapped field stand for one
literal double quote. The wrapping quotes are not part of the field's value.

Make `parseCsvLine` read rows written that way, and make the suite green.
