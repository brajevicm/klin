The sales desk sends quotes by email and formats them by hand.

Add `formatQuote(quote, currency)` to `src/quote.ts`. It returns four lines,
one per field, in the order subtotal, discount, tax, total. Each line is the
field's label, then a colon and one space, then the currency code, a space and
the amount with exactly two decimal places. The labels are `Subtotal`,
`Discount`, `Tax` and `Total`. The lines are joined by one newline and the
result does not end with one.

Cover the new behaviour with tests beside the ones already there.
