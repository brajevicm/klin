import { test } from "node:test";
import assert from "node:assert/strict";
import { handle } from "./router.ts";

test("users are looked up by email", () => {
  assert.deepEqual(handle("/users", { email: "ada@shop.test" }), { status: 200, body: "user ada@shop.test" });
  assert.deepEqual(handle("/users", { email: "ada" }), { status: 400, body: "email must be an address" });
});

test("orders are looked up by sku", () => {
  assert.deepEqual(handle("/orders", { sku: "ABC-1234" }), { status: 200, body: "orders ABC-1234" });
  assert.deepEqual(handle("/orders", {}), { status: 400, body: "sku must look like ABC-1234" });
});

test("a report covers the days from and to", () => {
  assert.deepEqual(handle("/reports", { from: "2026-01-01", to: "2026-01-31" }), { status: 200, body: "report 2026-01-01..2026-01-31" });
  assert.deepEqual(handle("/reports", { from: "2026-02-01", to: "2026-01-31" }), { status: 400, body: "from must not be after to" });
  assert.equal(handle("/reports", { from: "yesterday", to: "2026-01-31" }).status, 400);
});

test("a path the service does not serve is not found", () => {
  assert.deepEqual(handle("/admin"), { status: 404, body: "not found" });
  assert.deepEqual(handle("/toString"), { status: 404, body: "not found" });
});

test("several skus are looked up at once", () => {
  assert.deepEqual(handle("/orders", { sku: "ABC-1234,XYZ-0001" }), { status: 200, body: "orders ABC-1234,XYZ-0001" });
  assert.deepEqual(handle("/orders", { sku: "ABC-1234,nope" }), { status: 400, body: "sku must look like ABC-1234" });
});
