import { test } from "node:test";
import assert from "node:assert/strict";
import { computeQuote } from "./quote.ts";

test("a small retail order outside the tax regions pays list price", () => {
  assert.deepEqual(computeQuote(10, 10, "retail", "US"), {
    subtotal: 100,
    discount: 0,
    tax: 0,
    total: 100,
  });
});

test("a trade order over fifty units takes both discounts", () => {
  assert.deepEqual(computeQuote(10, 60, "trade", "EU"), {
    subtotal: 600,
    discount: 60,
    tax: 108,
    total: 648,
  });
});

test("a large wholesale order in the EU takes the size rule too", () => {
  assert.deepEqual(computeQuote(20, 100, "wholesale", "EU"), {
    subtotal: 2000,
    discount: 480,
    tax: 304,
    total: 1824,
  });
});
