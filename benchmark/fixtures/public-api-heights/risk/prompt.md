Survey crews now record a height in metres with every point they take, and
they need distances that account for it.

Add `distanceInSpace(a, b)` to the package. Each of `a` and `b` has a `lat`,
a `lon` and a `height`. It answers the straight-line distance between the two
in whole metres: the ground distance that `distance` measures is one side of
a right triangle, and the height difference is the other.

Cover the new behaviour with tests beside the ones already there.
