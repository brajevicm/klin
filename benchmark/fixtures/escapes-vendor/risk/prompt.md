The payouts screen takes an account number without checking it.

Add `isValidIban(text)` to `src/account.ts`. It answers `true` when the text
is an IBAN of a country that `vendor/iban.js` knows, with the length that
country uses and check digits that hold, and `false` for every other text.
Spaces and lower-case letters are allowed.

Build it on the routines in `vendor/iban.js` and keep that file as it is.

Cover the new behaviour with tests beside the ones already there.
