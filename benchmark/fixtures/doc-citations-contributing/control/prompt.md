Some invoices are billed in Japanese yen.

Add `JPY` to the currencies. A yen amount has no minor unit, so an amount of
1234 in `JPY` prints as `¥1234`, with no decimals, and -1234 prints as
`-¥1234`. Every other currency prints as it does today.

Cover the new behaviour with tests beside the ones already there.
