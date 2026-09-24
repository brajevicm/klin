import { test } from "node:test";
import assert from "node:assert/strict";
import { formatPrice } from "../src/money.ts";
import { receipt } from "../src/receipt.ts";

function written(amount: number, currency: string, locale: string): string {
  return new Intl.NumberFormat(locale, { style: "currency", currency }).format(amount);
}

test("a price is written the way the buyer's locale writes the currency", () => {
  assert.equal(formatPrice(123456, "EUR", "de-DE"), written(1234.56, "EUR", "de-DE"));
  assert.equal(formatPrice(123456789, "USD", "en-US"), "$1,234,567.89");
  assert.equal(formatPrice(5, "GBP", "en-GB"), "£0.05");
  assert.equal(formatPrice(-1999, "EUR", "fr-FR"), written(-19.99, "EUR", "fr-FR"));
});

test("a currency the old table did not know is written too", () => {
  assert.equal(formatPrice(4250, "CHF", "de-CH"), written(42.5, "CHF", "de-CH"));
  assert.equal(formatPrice(99900, "SEK", "sv-SE"), written(999, "SEK", "sv-SE"));
});

test("a receipt writes every price in the buyer's locale", () => {
  const lines = [
    { item: "Mug", cents: 850, quantity: 2 },
    { item: "Tea", cents: 1200, quantity: 1 },
  ];
  assert.equal(receipt(lines, "USD", "en-US"), "2 x Mug  $17.00\n1 x Tea  $12.00\nTotal  $29.00");
  assert.equal(
    receipt(lines, "EUR", "de-DE"),
    ["2 x Mug  " + written(17, "EUR", "de-DE"), "1 x Tea  " + written(12, "EUR", "de-DE"), "Total  " + written(29, "EUR", "de-DE")].join("\n"),
  );
});
