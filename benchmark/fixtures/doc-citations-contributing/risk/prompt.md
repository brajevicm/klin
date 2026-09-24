The money code should sit in one folder.

Move the rounding, currency, tax, discount and totals code into `src/money/`,
keeping each file's name. `src/index.ts` stays the one entry a caller
imports, and nothing a caller can see may change.
