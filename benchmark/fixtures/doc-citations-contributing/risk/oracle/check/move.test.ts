import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import * as entry from "../src/index.ts";
import * as currency from "../src/money/currency.ts";
import * as discount from "../src/money/discount.ts";
import * as rounding from "../src/money/rounding.ts";
import * as tax from "../src/money/tax.ts";
import * as totals from "../src/money/totals.ts";
import { inside, relativeDependencies } from "./graph.ts";

const MOVED = ["currency", "discount", "rounding", "tax", "totals"];

test("the money code moved into src/money under the same names", () => {
  for (const name of MOVED) {
    assert.equal(fs.existsSync(`src/${name}.ts`), false, `src/${name}.ts still exists`);
    assert.equal(fs.existsSync(`src/money/${name}.ts`), true, `src/money/${name}.ts is missing`);
  }
});

test("the entry exports the moved implementations under the same names", () => {
  assert.equal(entry.format, currency.format);
  assert.equal(entry.discounted, discount.discounted);
  assert.equal(entry.halfEven, rounding.halfEven);
  assert.equal(entry.taxOn, tax.taxOn);
  assert.equal(entry.RATES, tax.RATES);
  assert.equal(entry.totals, totals.totals);
  assert.deepEqual(Object.keys(entry).sort(), ["RATES", "discounted", "format", "halfEven", "render", "taxOn", "totals"]);
});

test("the moved code depends on nothing outside src/money", () => {
  const money = path.resolve("src/money");
  for (const name of MOVED) {
    const file = path.resolve(`src/money/${name}.ts`);
    for (const dependency of relativeDependencies(file)) {
      assert.ok(inside(money, dependency), file + " depends outside src/money: " + dependency);
    }
  }
});

test("an invoice still renders as before", () => {
  const lines = [
    { cents: 1999, quantity: 3, band: "standard" as const, discountPercent: 15 },
    { cents: 250, quantity: 4, band: "reduced" as const },
    { cents: 125, quantity: 1, band: "zero" as const },
  ];
  assert.deepEqual(entry.totals(lines), { net: 6222, tax: 1069, gross: 7291 });
  assert.equal(entry.render("B-12", lines, "EUR"), "Invoice B-12\nNet €62.22\nTax €10.69\nTotal €72.91");
  assert.equal(entry.halfEven(3.5), 4);
  assert.equal(entry.format(-5, "USD"), "-$0.05");
});
