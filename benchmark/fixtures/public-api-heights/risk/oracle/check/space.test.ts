import { test } from "node:test";
import assert from "node:assert/strict";
import { bearing, distance, distanceInSpace, routeLength } from "../src/index.ts";
import { compiles } from "./types.ts";

test("the package still type-checks with its route code", () => {
  assert.equal(compiles(), "");
});

test("a height difference alone is the whole distance in space", () => {
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 100 }, { lat: 0, lon: 0, height: 103 }), 3);
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 103 }, { lat: 0, lon: 0, height: 100 }), 3);
});

test("the distance in space combines the ground leg and the climb", () => {
  assert.equal(distanceInSpace({ lat: 51.5, lon: 0, height: 1000 }, { lat: 51.5, lon: 1, height: 1334 }), 100154);
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 1000 }, { lat: 1, lon: 0, height: 1334 }), 111196);
});

test("the ground measurements over two coordinates are what they were", () => {
  assert.equal(distance({ lat: 0, lon: 0 }, { lat: 1, lon: 0 }), 111195);
  assert.equal(distance({ lat: 51.5, lon: 0 }, { lat: 51.5, lon: 0 }), 0);
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: 0, lon: 1 }), 90);
  assert.equal(bearing({ lat: 0, lon: 0 }, { lat: -1, lon: 0 }), 180);
  assert.equal(
    routeLength([
      { lat: 0, lon: 0 },
      { lat: 1, lon: 0 },
      { lat: 1, lon: 1 },
    ]),
    222386,
  );
});
