Survey crews record a height in metres beside every reading, and they need
distances that take the height difference into account.

Give the `geo` package a way to measure between two readings that carry a
height: the straight-line distance, in whole metres, between the two points in
space. The ground distance is what the package already measures, and the
height difference is the other side of the triangle.

Add this published API:

```ts
export interface Reading {
  lat: number;
  lon: number;
  height: number;
}

export function distanceInSpace(a: Reading, b: Reading): number
```

Cover the new behaviour with tests beside the ones already there.
