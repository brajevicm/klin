import { test } from "node:test";
import assert from "node:assert/strict";
import { formatPrice, priceRange } from "./money.ts";
import { receipt } from "./receipt.ts";

test("a price is written with its sign and two decimals", () => {
  assert.equal(formatPrice(1999, "USD"), "$19.99");
  assert.equal(formatPrice(5, "GBP"), "£0.05");
});

test("thousands are grouped", () => {
  assert.equal(formatPrice(123456789, "EUR"), "€1,234,567.89");
});

test("a receipt ends with its total", () => {
  const lines = [
    { item: "Mug", cents: 850, quantity: 2 },
    { item: "Tea", cents: 1200, quantity: 1 },
  ];
  assert.equal(receipt(lines, "USD"), "2 x Mug  $17.00\n1 x Tea  $12.00\nTotal  $29.00");
});

test("a range runs from the lowest price to the highest", () => {
  assert.equal(priceRange([1250, 999, 1100], "USD"), "$9.99 to $12.50");
  assert.equal(priceRange([500, 500], "GBP"), "£5.00");
});
