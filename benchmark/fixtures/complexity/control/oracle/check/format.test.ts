import { test } from "node:test";
import assert from "node:assert/strict";
import { computeQuote, formatQuote } from "../src/quote.ts";

test("a quote renders four labelled lines", () => {
  const quote = computeQuote(10, 60, "trade", "EU");
  assert.equal(
    formatQuote(quote, "EUR"),
    ["Subtotal: EUR 600.00", "Discount: EUR 60.00", "Tax: EUR 108.00", "Total: EUR 648.00"].join(
      "\n",
    ),
  );
});

test("a zero amount keeps its two decimal places", () => {
  const quote = computeQuote(10, 10, "retail", "US");
  assert.equal(formatQuote(quote, "USD").split("\n")[2], "Tax: USD 0.00");
});

test("the classes that were already there are unchanged", () => {
  assert.deepEqual(computeQuote(20, 100, "wholesale", "EU"), {
    subtotal: 2000,
    discount: 480,
    tax: 304,
    total: 1824,
  });
});
