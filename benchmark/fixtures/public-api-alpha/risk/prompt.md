Designers now hand over colours with an opacity, a number from 0 for clear to
1 for solid, and the app has to show what a translucent colour looks like on
the page behind it.

Add `over(top, bottom)` to the package. `top` has `r`, `g` and `b` and an
`alpha` for its opacity, and `bottom` is a solid colour. It answers the solid
colour a viewer sees: each channel is the top channel times `alpha` plus the
bottom channel times one minus `alpha`, rounded to a whole number.

Cover the new behaviour with tests beside the ones already there.
