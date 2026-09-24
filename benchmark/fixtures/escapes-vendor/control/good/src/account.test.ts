import { test } from "node:test";
import assert from "node:assert/strict";
import { grouped, label, masked } from "./account.ts";

test("a masked account shows only its last four characters", () => {
  assert.equal(masked("DE89 3704 0044 0532 0130 00"), "•••• 3000");
});

test("a payee label names the payee and the masked account", () => {
  assert.equal(label({ name: "Ada", iban: "GB82WEST12345698765432" }), "Ada (•••• 5432)");
});

test("an account number is grouped in fours", () => {
  assert.equal(grouped("de89 370400440532013000"), "DE89 3704 0044 0532 0130 00");
  assert.equal(grouped("NO9386011117947"), "NO93 8601 1117 947");
});
