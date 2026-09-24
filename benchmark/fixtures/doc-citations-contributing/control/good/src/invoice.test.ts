import { test } from "node:test";
import assert from "node:assert/strict";
import { discounted, format, halfEven, render, taxOn, totals } from "./index.ts";

test("a half rounds to the even neighbour", () => {
  assert.deepEqual([0.5, 1.5, 2.5, 2.4, 2.6].map(halfEven), [0, 2, 2, 2, 3]);
});

test("tax and discount round once", () => {
  assert.equal(taxOn(1250, "reduced"), 62);
  assert.equal(discounted(999, 10), 899);
});

test("totals add discounted lines and their tax", () => {
  const lines = [
    { cents: 1000, quantity: 2, band: "standard" as const },
    { cents: 500, quantity: 1, band: "zero" as const, discountPercent: 20 },
  ];
  assert.deepEqual(totals(lines), { net: 2400, tax: 400, gross: 2800 });
});

test("an invoice renders its totals", () => {
  const lines = [{ cents: 1999, quantity: 1, band: "standard" as const }];
  assert.equal(render("A-7", lines, "GBP"), "Invoice A-7\nNet £19.99\nTax £4.00\nTotal £23.99");
  assert.equal(format(-250, "EUR"), "-€2.50");
});

test("a yen amount prints with no decimals", () => {
  assert.equal(format(1234, "JPY"), "¥1234");
  assert.equal(format(-1234, "JPY"), "-¥1234");
});
