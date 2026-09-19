import { test } from "node:test";
import assert from "node:assert/strict";
import { bearing, distance, distanceInSpace } from "../src/index.ts";

test("space distance combines the ground leg and height difference", () => {
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 0 }, { lat: 0, lon: 0, height: 3 }), 3);
  assert.equal(
    distanceInSpace({ lat: 0, lon: 0, height: 0 }, { lat: 1, lon: 0, height: 111195 }),
    157253,
  );
});

test("the ground distance is what it was", () => {
  assert.equal(distance({ lat: 0, lon: 0 }, { lat: 1, lon: 0 }), 111195);
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 1 }), 100153);
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 0 }), 0);
});

test("the direction is what it was", () => {
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: 0, lon: 1 }), 90);
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: -1, lon: 0 }), 180);
});
