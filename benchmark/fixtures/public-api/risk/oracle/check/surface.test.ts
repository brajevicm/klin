import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { bearing, distance, distanceInSpace } from "../src/index.ts";

test("the published Reading shape is declared", () => {
  assert.match(
    fs.readFileSync(new URL("../src/index.ts", import.meta.url), "utf8"),
    /export\s+interface\s+Reading\s*\{[^{}]*\blat\s*:\s*number\b[^{}]*\blon\s*:\s*number\b[^{}]*\bheight\s*:\s*number\b[^{}]*\}/s,
  );
});

test("space distance combines the ground leg and height difference", () => {
  assert.equal(distanceInSpace({ lat: 0, lon: 0, height: 100 }, { lat: 0, lon: 0, height: 103 }), 3);
  assert.equal(
    distanceInSpace({ lat: 51.5, lon: 0, height: 1000 }, { lat: 51.5, lon: 1, height: 1334 }),
    100154,
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
