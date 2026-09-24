import { test } from "node:test";
import assert from "node:assert/strict";
import { formatPrice, priceRange } from "../src/money.ts";

test("a range runs from the lowest price to the highest", () => {
  assert.equal(priceRange([1250, 999, 1100], "USD"), "$9.99 to $12.50");
  assert.equal(priceRange([250000, 100], "EUR"), "€1.00 to €2,500.00");
});

test("prices that all match are one price", () => {
  assert.equal(priceRange([500, 500], "GBP"), "£5.00");
  assert.equal(priceRange([700], "USD"), "$7.00");
});

test("a single price is written as it was", () => {
  assert.equal(formatPrice(123456789, "EUR"), "€1,234,567.89");
  assert.throws(() => formatPrice(100, "JPY"), /JPY/);
});
