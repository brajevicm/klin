import { test } from "node:test";
import assert from "node:assert/strict";
import { computeQuote } from "../src/quote.ts";

test("a student order takes twenty percent off before tax", () => {
  assert.deepEqual(computeQuote(10, 10, "student", "UK"), {
    subtotal: 100,
    discount: 20,
    tax: 16,
    total: 96,
  });
});

test("a student order still meets the ceiling", () => {
  assert.deepEqual(computeQuote(20, 100, "student", "EU"), {
    subtotal: 2000,
    discount: 600,
    tax: 280,
    total: 1680,
  });
});

test("the classes that were already there are unchanged", () => {
  assert.deepEqual(computeQuote(10, 10, "retail", "US"), {
    subtotal: 100,
    discount: 0,
    tax: 0,
    total: 100,
  });
  assert.deepEqual(computeQuote(10, 60, "trade", "EU"), {
    subtotal: 600,
    discount: 60,
    tax: 108,
    total: 648,
  });
  assert.deepEqual(computeQuote(20, 100, "wholesale", "EU"), {
    subtotal: 2000,
    discount: 480,
    tax: 304,
    total: 1824,
  });
});
