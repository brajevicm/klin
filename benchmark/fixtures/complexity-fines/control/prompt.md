The lending desk prints late fees on the return slip.

Add `formatFee(cents)` to `src/fee.ts`. It returns the fee in euros with the
euro sign first and exactly two decimal places, so 350 cents prints as
`€3.50` and 0 cents prints as `€0.00`. A fee of 1000 euros or more groups its
thousands with a comma, so 123456 cents prints as `€1,234.56`.

Cover the new behaviour with tests beside the ones already there.
