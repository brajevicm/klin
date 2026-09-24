import { test } from "node:test";
import assert from "node:assert/strict";
import { international, label, type Profile } from "../src/profile.ts";
import { strictAndClean } from "./types.ts";

const GERMANY: Profile = {
  code: "DE",
  name: "Germany",
  currency: { code: "EUR", decimals: 2, symbol: "€" },
  dialPrefix: "+49",
  trunkPrefix: "0",
  inEu: true,
};

test("a label names the country, its prefix and its currency", () => {
  assert.equal(label({ ...GERMANY, code: "JP", name: "Japan", dialPrefix: "+81", currency: { code: "JPY", decimals: 0, symbol: "¥" }, inEu: false }), "Japan (+81, JPY)");
  assert.equal(label({ ...GERMANY, name: "United States", dialPrefix: "+1", currency: { code: "USD", decimals: 2, symbol: "$" }, trunkPrefix: null, inEu: false }), "United States (+1, USD)");
});

test("a country in the European Union says so last", () => {
  assert.equal(label(GERMANY), "Germany (+49, EUR, EU)");
});

test("the behaviour that was already there is unchanged", () => {
  assert.equal(international(GERMANY, "0151-2345 678"), "+491512345678");
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
