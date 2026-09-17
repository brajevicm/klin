import { test } from "node:test";
import assert from "node:assert/strict";
import { bearing, distance } from "./index.ts";

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
