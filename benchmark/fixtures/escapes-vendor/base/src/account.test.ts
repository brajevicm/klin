import { test } from "node:test";
import assert from "node:assert/strict";
import { label, masked } from "./account.ts";

test("a masked account shows only its last four characters", () => {
  assert.equal(masked("DE89 3704 0044 0532 0130 00"), "•••• 3000");
});

test("a payee label names the payee and the masked account", () => {
  assert.equal(label({ name: "Ada", iban: "GB82WEST12345698765432" }), "Ada (•••• 5432)");
});
