import { test } from "node:test";
import assert from "node:assert/strict";
import { bearing, distance, midpoint } from "../src/index.ts";

test("the midpoint is the mean of each coordinate", () => {
  assert.deepEqual(midpoint({ lat: 0, lon: 0 }, { lat: 2, lon: 4 }), { lat: 1, lon: 2 });
  assert.deepEqual(midpoint({ lat: 1, lon: 1 }, { lat: 2, lon: 2 }), { lat: 1.5, lon: 1.5 });
});

test("each coordinate is rounded to six places", () => {
  assert.deepEqual(midpoint({ lat: 0, lon: 0 }, { lat: 0.0000001, lon: 0.0000003 }), {
    lat: 0,
    lon: 0,
  });
  assert.deepEqual(midpoint({ lat: 0, lon: 0 }, { lat: 0.0000025, lon: 0 }), {
    lat: 0.000001,
    lon: 0,
  });
});

test("the measurements that were already there are unchanged", () => {
  assert.equal(distance({ lat: 0, lon: 0 }, { lat: 1, lon: 0 }), 111195);
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: 0, lon: 1 }), 90);
});
