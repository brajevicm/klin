The `fmtx` crate lays text out and cannot break a paragraph to a width.

Add `wrap(text, width)` to the crate. It returns the lines the text breaks
into, and it must hold all three of these:

1. a line never runs past `width` characters;
2. a break happens at a space, and that space is not kept in either line;
3. a single word longer than `width` is broken across lines with a `-` at the
   end of each part but the last, and the `-` counts toward the width.

Cover the new behaviour with tests beside the ones already there.
