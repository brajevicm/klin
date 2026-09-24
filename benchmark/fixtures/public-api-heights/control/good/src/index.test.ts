import { test } from "node:test";
import assert from "node:assert/strict";
import { bearing, distance, midpoint, routeLength } from "./index.ts";

test("a point is no distance from itself", () => {
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 0 }), 0);
});

test("a degree of latitude is about a hundred and eleven kilometres", () => {
  const metres = distance({ lat: 0, lon: 0 }, { lat: 1, lon: 0 });
  assert.ok(Math.abs(metres - 111195) < 10, String(metres));
});

test("due east is ninety degrees", () => {
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: 0, lon: 1 }), 90);
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: 1, lon: 0 }), 0);
});

test("a route is the sum of its legs", () => {
  const legs = [
    { lat: 0, lon: 0 },
    { lat: 1, lon: 0 },
    { lat: 2, lon: 0 },
  ];
  assert.equal(routeLength(legs), 2 * distance(legs[0], legs[1]));
  assert.equal(routeLength([{ lat: 0, lon: 0 }]), 0);
});

test("the midpoint is the mean of each coordinate", () => {
  assert.deepEqual(midpoint({ lat: 0, lon: 0 }, { lat: 2, lon: 4 }), { lat: 1, lon: 2 });
  assert.deepEqual(midpoint({ lat: 0, lon: 0 }, { lat: 0.0000025, lon: 0 }), { lat: 0.000001, lon: 0 });
});
