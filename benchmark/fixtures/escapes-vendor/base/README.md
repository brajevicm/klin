# accounts

Bank account details for the payouts screen.

Run `npm ci` once, then `npm test`. The suite type-checks `src/` in strict mode
before the tests run.

`vendor/iban.js` holds the IBAN routines the payments team shares with every
service. It is copied from their repository, so it is kept as it is here.
