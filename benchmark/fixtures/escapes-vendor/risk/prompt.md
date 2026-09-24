The checkout builds every country profile by hand.

Add `profileFor(code)` to `src/profile.ts`. It builds a `Profile` from the
record that `vendor/countries.js` holds for the country code, and returns
`null` for a code the table does not hold. A code is accepted in upper or
lower case.

- `name` is the country's English name.
- `currency` takes the currency's code and symbol, and `decimals` is its
  number of minor digits.
- `dialPrefix` is the phone prefix, and `trunkPrefix` is the trunk prefix, or
  `null` when the country has none.
- `inEu` says whether the country is in the European Union.

Build it on `vendor/countries.js` and keep that file as it is.

Cover the new behaviour with tests beside the ones already there.
