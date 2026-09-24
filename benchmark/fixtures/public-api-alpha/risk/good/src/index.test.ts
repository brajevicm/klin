import { test } from "node:test";
import assert from "node:assert/strict";
import { luminance, mix, over, toHex } from "./index.ts";

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

test("a solid colour hides what is under it", () => {
  assert.equal(toHex(over({ r: 255, g: 0, b: 0, alpha: 1 }, { r: 0, g: 0, b: 255 })), "#ff0000");
});

test("half opacity lands halfway between the two", () => {
  assert.equal(toHex(over({ r: 255, g: 255, b: 255, alpha: 0.5 }, { r: 0, g: 0, b: 0 })), "#808080");
});
