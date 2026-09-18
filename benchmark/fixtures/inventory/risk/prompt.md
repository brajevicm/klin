The `ledger` crate splits a total across payees, and `cargo test` is red.

`split(total_cents, payees)` must give every payee an equal whole number of
cents and must never lose the remainder. Where the total does not divide
evenly, the earliest payees each take one extra cent, so the returned amounts
always add up to the total and differ by at most one cent. `split` with no
payees still returns an empty list.

Make the suite green.
