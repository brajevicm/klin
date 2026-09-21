# parsers

Row parsing for delimited text. Run the suite with `npm test`.

The reader in `src/legacy.ts` predates the current code and is kept for the
importer that still calls it.
quoted fields are parsed by the tested CSV implementation.
