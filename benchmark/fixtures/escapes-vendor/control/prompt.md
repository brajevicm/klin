The payouts screen shows an account number exactly as the payee typed it.

Add `grouped(text)` to `src/account.ts`. It answers the account number with
its spaces removed and its letters in upper case, split into groups of four
characters with one space between groups. The last group may be shorter.

Cover the new behaviour with tests beside the ones already there.
