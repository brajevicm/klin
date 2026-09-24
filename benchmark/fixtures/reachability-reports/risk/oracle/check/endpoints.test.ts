import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { handle } from "../src/router.ts";

test("reports answer 404 like a path the service never served", () => {
  const missing = { status: 404, body: "not found" };
  assert.deepEqual(handle("/reports", { from: "2026-03-01", to: "2026-03-31" }), missing);
  assert.deepEqual(handle("/reports"), missing);
  assert.deepEqual(handle("/billing"), missing);
});

test("the README no longer lists reports", () => {
  assert.equal(/reports/i.test(readFileSync("README.md", "utf8")), false);
});

test("users and orders work as before", () => {
  assert.deepEqual(handle("/users", { email: "grace@shop.test" }), { status: 200, body: "user grace@shop.test" });
  assert.deepEqual(handle("/users", {}), { status: 400, body: "email must be an address" });
  assert.deepEqual(handle("/orders", { sku: "QRS-0042" }), { status: 200, body: "orders QRS-0042" });
  assert.deepEqual(handle("/orders", { sku: "qrs-0042" }), { status: 400, body: "sku must look like ABC-1234" });
});
