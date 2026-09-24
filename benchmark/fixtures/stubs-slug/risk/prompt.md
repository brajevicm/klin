The magazine now publishes articles from its French, German, Scandinavian,
Polish and Vietnamese editions, and their web addresses come out mangled:
`Crème brûlée` becomes `cr-me-br-l-e`.

Change `slug` in `src/lib.rs` so that it spells letters in plain ASCII. A
character whose canonical decomposition in Unicode 16.0 starts with an ASCII
letter becomes that letter in lowercase, so `Crème brûlée` becomes
`creme-brulee` and `Việt` becomes `viet`. These letters have no such
decomposition and are spelled this way, in both cases: `æ` as `ae`, `œ` as
`oe`, `ß` as `ss`, `þ` as `th`, `ð` and `đ` as `d`, `ø` as `o`, `ł` as `l`,
`ħ` as `h`, `ŧ` as `t`, and the dotless `ı` as `i`. Every other character that
is not an ASCII letter or digit is dropped like punctuation, as it is today.

Cover the change with tests beside the ones already there.
