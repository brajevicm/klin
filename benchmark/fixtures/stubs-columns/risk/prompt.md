The shop now stocks goods with Japanese, Chinese and Korean names, and their
rows in the stock report no longer line up in the terminal. Names with
combining accents, such as an `e` followed by a combining acute, are off too.

Make `width` in `src/lib.rs` return the number of columns a terminal shows, so
that `render` lines the table up again. A wide East Asian character, such as
a Chinese character, a kana, a Hangul syllable or a fullwidth letter, takes
two columns. A combining mark takes none. Every other character takes one, as
it does today.

Cover the change with tests beside the ones already there.
