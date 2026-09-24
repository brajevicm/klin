import { test } from "node:test";
import assert from "node:assert/strict";
import { contrast, luminance, mix, toHex } from "./index.ts";

test("a colour is written as six hex digits", () => {
  assert.equal(toHex({ r: 255, g: 0, b: 16 }), "#ff0010");
});

test("halfway from black to white is mid grey", () => {
  assert.equal(toHex(mix({ r: 0, g: 0, b: 0 }, { r: 255, g: 255, b: 255 }, 0.5)), "#808080");
});

test("black and white bound the luminance", () => {
  assert.equal(luminance({ r: 0, g: 0, b: 0 }), 0);
  assert.equal(luminance({ r: 255, g: 255, b: 255 }), 1);
});

test("black on white is the widest contrast", () => {
  assert.equal(contrast({ r: 0, g: 0, b: 0 }, { r: 255, g: 255, b: 255 }), 21);
  assert.equal(contrast({ r: 255, g: 255, b: 255 }, { r: 0, g: 0, b: 0 }), 21);
});
