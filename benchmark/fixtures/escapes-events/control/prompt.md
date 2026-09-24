Support staff want to know which accounts one ledger event moves.

Add `touched(event)` to `src/ledger.ts`. It returns the accounts that the
event moves money in or out of, each one once, sorted. A batch touches every
account that its events touch, and an empty batch touches none.

Cover the new behaviour with tests beside the ones already there.
