import { test } from "node:test";
import assert from "node:assert/strict";
import { handle } from "../src/router.ts";

test("a list of valid skus answers with the list as given", () => {
  assert.deepEqual(handle("/orders", { sku: "ABC-1234,XYZ-0001" }), { status: 200, body: "orders ABC-1234,XYZ-0001" });
  assert.deepEqual(handle("/orders", { sku: "ABC-1234,ABC-1234,DEF-9999" }), { status: 200, body: "orders ABC-1234,ABC-1234,DEF-9999" });
});

test("one bad item refuses the whole list with the single-sku reason", () => {
  const reason = { status: 400, body: "sku must look like ABC-1234" };
  assert.deepEqual(handle("/orders", { sku: "ABC-1234,xyz-0001" }), reason);
  assert.deepEqual(handle("/orders", { sku: "ABC-1234," }), reason);
  assert.deepEqual(handle("/orders", { sku: ",ABC-1234" }), reason);
  assert.deepEqual(handle("/orders", { sku: "" }), reason);
  assert.deepEqual(handle("/orders", {}), reason);
});

test("a single sku and the other endpoints answer as before", () => {
  assert.deepEqual(handle("/orders", { sku: "ABC-1234" }), { status: 200, body: "orders ABC-1234" });
  assert.deepEqual(handle("/users", { email: "ada@shop.test" }), { status: 200, body: "user ada@shop.test" });
  assert.deepEqual(handle("/reports", { from: "2026-01-01", to: "2026-01-02" }), { status: 200, body: "report 2026-01-01..2026-01-02" });
});
