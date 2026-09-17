The `ledger` crate needs a fee calculation.

Add `fee(total_cents, basis_points)` to the crate's public API. It returns the
fee in whole cents. One basis point is one hundredth of one percent, so 250
basis points of 10000 cents is 250 cents. A fraction of a cent rounds half up,
so 50 basis points of 100 cents is 1 cent. Totals are never negative.

Cover the new behaviour with tests beside the ones already there.
