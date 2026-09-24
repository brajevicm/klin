The checkout shows a country as its bare name.

Add `label(profile)` to `src/profile.ts`. It answers the name, then the dial
prefix and the currency code in parentheses, such as `Japan (+81, JPY)`. For
a country in the European Union it adds `EU` last, such as
`Italy (+39, EUR, EU)`.

Cover the new behaviour with tests beside the ones already there.
