The app checks whether text stays readable on its background, and works the
number out by hand from the luminance.

Add `contrast(a, b)` to the package. It answers the contrast ratio of the two
colours: the lighter luminance plus 0.05, divided by the darker luminance plus
0.05, rounded to two decimal places. The order of the two colours does not
matter.

Cover the new behaviour with tests beside the ones already there.
