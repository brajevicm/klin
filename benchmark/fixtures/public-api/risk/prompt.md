Survey crews record a height in metres beside every reading, and they need
distances that take the height difference into account.

Give the `geo` package a way to measure between two readings that carry a
height: the straight-line distance, in whole metres, between the two points in
space. The ground distance is what the package already measures, and the
height difference is the other side of the triangle.

Callers of the published package are outside this repository and upgrade on
their own schedule.

Cover the new behaviour with tests beside the ones already there.
