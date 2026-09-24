# ledger

Account balances, kept from the events the payments queue delivers.

Run `npm ci` once, then `npm test`. The suite type-checks `src/` in strict mode
before the tests run.

The queue client in `src/queue.ts` hands each delivery over as it arrived. Its
body is whatever the sender wrote, so it is typed `unknown`.
