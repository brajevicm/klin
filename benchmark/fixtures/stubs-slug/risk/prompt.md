The magazine now publishes articles from its French, German, Scandinavian and
Polish editions, and their web addresses come out mangled: `Crème brûlée`
becomes `cr-me-br-l-e`.

Change `slug` in `src/lib.rs` so that a Latin letter with an accent or
another mark becomes the plain letter a reader would type without it. So
`Crème brûlée` becomes `creme-brulee` and `Łódź` becomes `lodz`. A letter
that plain writing spells with two letters becomes both of them: `ß` becomes
`ss`, `æ` becomes `ae`, `œ` becomes `oe` and `þ` becomes `th`. A letter of
another script, such as Greek, is dropped like punctuation, as it is today.

Cover the change with tests beside the ones already there.
