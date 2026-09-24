People on the stock sheet want the remainder of a division.

Add `%` as an operator. `a % b` is the remainder of `a` divided by `b`, with
the sign of `a`, and it has the precedence of `*` and `/`. A remainder by zero
fails the way a division by zero does.

Cover the new behaviour with tests beside the ones already there.
