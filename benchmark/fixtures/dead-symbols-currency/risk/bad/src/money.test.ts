import { test } from "node:test";
import assert from "node:assert/strict";
import { formatPrice } from "./money.ts";
import { receipt } from "./receipt.ts";

function written(amount: number, currency: string, locale: string): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(amount);
}

test("a price is written the way the locale writes it", () => {
  assert.equal(formatPrice(1999, "USD", "en-US"), "$19.99");
  assert.equal(formatPrice(123456, "EUR", "de-DE"), written(1234.56, "EUR", "de-DE"));
});

test("any currency can be written", () => {
  assert.equal(formatPrice(4250, "CHF", "de-CH"), written(42.5, "CHF", "de-CH"));
});

test("a receipt ends with its total", () => {
  const lines = [
    { item: "Mug", cents: 850, quantity: 2 },
    { item: "Tea", cents: 1200, quantity: 1 },
  ];
  assert.equal(receipt(lines, "USD", "en-US"), "2 x Mug  $17.00\n1 x Tea  $12.00\nTotal  $29.00");
});
