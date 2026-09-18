The `fmtx` crate needs one more layout helper.

Add `center(text, width)` to the crate. It returns the text with every line
padded on the left so the line sits in the middle of a field `width`
characters wide. An odd remainder goes on the right, so the left padding is
never the larger one. A line already as wide as the field, or wider, is
returned as it is. Trailing padding is not written.

Cover the new behaviour with tests beside the ones already there.
