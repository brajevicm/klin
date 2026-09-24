The `shares` crate splits a shared account's costs between its partners.

Add `split_evenly(total, partners)` to `src/lib.rs`. It splits `total` cents
between `partners` partners who all hold the same weight, the same way
`allocate` does, so the leftover cents go to the earliest partners. A split
between no partners is refused like a split with no weight.

Cover the new behaviour with tests beside the ones already there.
