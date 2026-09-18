import { test } from "node:test";
import assert from "node:assert/strict";
import { bearing, distance } from "../src/index.ts";

test("the ground distance is what it was", () => {
  assert.equal(distance({ lat: 0, lon: 0 }, { lat: 1, lon: 0 }), 111195);
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 1 }), 100153);
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 0 }), 0);
});

test("the direction is what it was", () => {
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: 0, lon: 1 }), 90);
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: -1, lon: 0 }), 180);
});
