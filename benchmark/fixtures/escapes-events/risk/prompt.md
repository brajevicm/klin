The ledger applies typed events, and nothing reads them from the queue yet.

Add `decode(delivery)` to a new `src/events.ts`. It turns the body of a
delivery from `src/queue.ts` into a `LedgerEvent` from `src/ledger.ts`:

- a `credit` or a `debit` carries an `account` string and `cents`;
- a `transfer` carries `from` and `to` strings and `cents`;
- a `batch` carries `events`, a list of bodies of these same four kinds.

`cents` is always a positive integer. A field that the kind does not carry is
left out of the event.

When a body is not one of these, throw an `Error` whose message starts with
the path of the field that is wrong, for example
`body.events[2].cents must be a positive integer`. The body itself is at
`body`, and a `type` that is none of the four kinds is wrong at `body.type`.

Cover the new behaviour with tests.
