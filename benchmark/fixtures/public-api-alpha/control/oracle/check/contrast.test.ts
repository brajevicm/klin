import { test } from "node:test";
import assert from "node:assert/strict";
import { contrast, luminance, mix, toHex } from "../src/index.ts";
import { compiles } from "./types.ts";

test("the package still type-checks with its mixing code", () => {
  assert.equal(compiles(), "");
});

test("black on white is the widest contrast, in either order", () => {
  assert.equal(contrast({ r: 0, g: 0, b: 0 }, { r: 255, g: 255, b: 255 }), 21);
  assert.equal(contrast({ r: 255, g: 255, b: 255 }, { r: 0, g: 0, b: 0 }), 21);
  assert.equal(contrast({ r: 40, g: 40, b: 40 }, { r: 40, g: 40, b: 40 }), 1);
});

test("the ratio is rounded to two places", () => {
  assert.equal(contrast({ r: 0, g: 128, b: 255 }, { r: 255, g: 255, b: 255 }), 3.8);
  assert.equal(contrast({ r: 118, g: 118, b: 118 }, { r: 255, g: 255, b: 255 }), 4.54);
});

test("the colour arithmetic that was already there is unchanged", () => {
  assert.equal(toHex(mix({ r: 10, g: 20, b: 30 }, { r: 250, g: 120, b: 0 }, 0.25)), "#462d17");
  assert.equal(luminance({ r: 0, g: 128, b: 255 }), 0.2266);
});
