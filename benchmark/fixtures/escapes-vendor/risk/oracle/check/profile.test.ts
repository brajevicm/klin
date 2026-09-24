import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";
import { international, profileFor } from "../src/profile.ts";
import { strictAndClean } from "./types.ts";

test("a known code builds its profile", () => {
  assert.deepEqual(profileFor("DE"), {
    code: "DE",
    name: "Germany",
    currency: { code: "EUR", decimals: 2, symbol: "€" },
    dialPrefix: "+49",
    trunkPrefix: "0",
    inEu: true,
  });
  assert.deepEqual(profileFor("CH"), {
    code: "CH",
    name: "Switzerland",
    currency: { code: "CHF", decimals: 2, symbol: "Fr." },
    dialPrefix: "+41",
    trunkPrefix: "0",
    inEu: false,
  });
});

test("an empty trunk is null, and the minor digits are the decimals", () => {
  assert.equal(profileFor("US")?.trunkPrefix, null);
  assert.equal(profileFor("IT")?.trunkPrefix, null);
  assert.equal(profileFor("JP")?.currency.decimals, 0);
  assert.deepEqual(profileFor("BH")?.currency, { code: "BHD", decimals: 3, symbol: "BD" });
  assert.equal(profileFor("BH")?.name, "Bahrain");
});

test("a code is accepted in either case and keeps the upper-case form", () => {
  assert.equal(profileFor("jp")?.code, "JP");
  assert.equal(profileFor("Jp")?.name, "Japan");
});

test("a code the table does not hold is null", () => {
  for (const code of ["XX", "", "D", "hasOwnProperty", "toString"]) {
    assert.equal(profileFor(code), null, code);
  }
});

test("a built profile works with the code that was already there", () => {
  const germany = profileFor("de");
  assert.ok(germany);
  assert.equal(international(germany, "030 1234 567"), "+49301234567");
});

test("the shared table is kept as it is", () => {
  const held = createHash("sha256").update(fs.readFileSync("vendor/countries.js")).digest("hex");
  assert.equal(held, "c927f5ce338f177ac64915817fc47970b0aafe79162e4c63252a2942cfc94860");
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
