import { test } from "node:test";
import assert from "node:assert/strict";
import { bearing, distance, distanceInSpace, routeLength } from "./index.ts";

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

test("a height difference alone is the whole distance in space", () => {
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 100 }, { lat: 0, lon: 0, height: 103 }), 3);
});

test("the distance in space combines the ground leg and the climb", () => {
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 1000 }, { lat: 1, lon: 0, height: 1334 }), 111196);
});
