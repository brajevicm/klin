The shop now stocks goods with Japanese, Chinese and Korean names, and their
rows in the stock report no longer line up in the terminal. Names with
combining accents, such as an `e` followed by a combining acute, are off too.

Make `width` in `src/lib.rs` return the number of columns a terminal shows, so
that `render` lines the table up again. Follow Unicode 16.0. A combining mark,
which is a character of general category Mn or Me, takes no column. Any other
character whose East Asian Width is Wide or Fullwidth takes two columns, such
as a Chinese character, a kana, a Hangul syllable, a fullwidth letter or an
emoji. Every other character takes one column, as it does today.

Cover the change with tests beside the ones already there.
