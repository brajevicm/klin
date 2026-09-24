import { test } from "node:test";
import assert from "node:assert/strict";
import { luminance, mix, over, toHex } from "../src/index.ts";
import { compiles } from "./types.ts";

test("the package still type-checks with its mixing code", () => {
  assert.equal(compiles(), "");
});

test("a solid colour hides what is under it, and a clear one shows it", () => {
  assert.deepEqual(over({ r: 255, g: 0, b: 0, alpha: 1 }, { r: 0, g: 0, b: 255 }), { r: 255, g: 0, b: 0 });
  assert.deepEqual(over({ r: 0, g: 128, b: 255, alpha: 0 }, { r: 9, g: 9, b: 9 }), { r: 9, g: 9, b: 9 });
});

test("each channel weighs the top by its opacity and the bottom by the rest", () => {
  assert.equal(toHex(over({ r: 255, g: 255, b: 255, alpha: 0.5 }, { r: 0, g: 0, b: 0 })), "#808080");
  assert.equal(toHex(over({ r: 200, g: 100, b: 50, alpha: 0.25 }, { r: 10, g: 20, b: 30 })), "#3a2823");
});

test("colours given as three channels work as they did", () => {
  assert.equal(toHex({ r: 255, g: 0, b: 16 }), "#ff0010");
  assert.equal(toHex(mix({ r: 10, g: 20, b: 30 }, { r: 250, g: 120, b: 0 }, 0.25)), "#462d17");
  assert.equal(luminance({ r: 0, g: 128, b: 255 }), 0.2266);
});
