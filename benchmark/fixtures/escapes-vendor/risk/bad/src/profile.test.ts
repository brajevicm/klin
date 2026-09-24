import { test } from "node:test";
import assert from "node:assert/strict";
import { international, profileFor, type Profile } from "./profile.ts";

const GERMANY: Profile = {
  code: "DE",
  name: "Germany",
  currency: { code: "EUR", decimals: 2, symbol: "€" },
  dialPrefix: "+49",
  trunkPrefix: "0",
  inEu: true,
};

test("a local number loses its trunk prefix and gains the dial prefix", () => {
  assert.equal(international(GERMANY, "030 1234 567"), "+49301234567");
});

test("a country with no trunk prefix keeps every digit", () => {
  const italy: Profile = { ...GERMANY, code: "IT", name: "Italy", dialPrefix: "+39", trunkPrefix: null };
  assert.equal(international(italy, "06 1234 5678"), "+390612345678");
});

test("a profile is built from the shared table", () => {
  assert.deepEqual(profileFor("de"), GERMANY);
  assert.equal(profileFor("US")?.trunkPrefix, null);
  assert.equal(profileFor("XX"), null);
});
