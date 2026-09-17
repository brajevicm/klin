Map callers place a label between two readings and work the position out
themselves.

Give the `geo` package `midpoint(a, b)`. It answers with the point whose
latitude is the mean of the two latitudes and whose longitude is the mean of
the two longitudes, each rounded to six decimal places.

Callers of the published package are outside this repository and upgrade on
their own schedule.

Cover the new behaviour with tests beside the ones already there.
