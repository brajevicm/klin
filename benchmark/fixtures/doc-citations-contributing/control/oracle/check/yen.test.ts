import { test } from "node:test";
import assert from "node:assert/strict";
import { format, render } from "../src/index.ts";

test("a yen amount prints with no decimals", () => {
  assert.equal(format(1234, "JPY" as never), "¥1234");
  assert.equal(format(-1234, "JPY" as never), "-¥1234");
  assert.equal(format(0, "JPY" as never), "¥0");
});

test("a yen invoice renders its totals in yen", () => {
  const lines = [{ cents: 1000, quantity: 3, band: "standard" as const }];
  assert.equal(render("J-1", lines, "JPY" as never), "Invoice J-1\nNet ¥3000\nTax ¥600\nTotal ¥3600");
});

test("the other currencies print as before", () => {
  assert.equal(format(123456, "EUR"), "€1234.56");
  assert.equal(format(-5, "GBP"), "-£0.05");
  assert.equal(format(100, "USD"), "$1.00");
});
