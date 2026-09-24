import { test } from "node:test";
import assert from "node:assert/strict";
import { decode } from "./events.ts";
import { delivery } from "./queue.ts";

function read(body: unknown): unknown {
  return decode(delivery("d1", JSON.stringify(body)));
}

test("a batch of events decodes to the same events", () => {
  const body = {
    type: "batch",
    events: [
      { type: "credit", account: "a", cents: 5 },
      { type: "transfer", from: "a", to: "b", cents: 2 },
    ],
  };
  assert.deepEqual(read(body), body);
});

test("a field the kind does not carry is left out", () => {
  assert.deepEqual(read({ type: "debit", account: "a", cents: 5, to: "b" }), { type: "debit", account: "a", cents: 5 });
});

test("a wrong field is refused under its path", () => {
  assert.throws(() => read([]), /^Error: body /);
  assert.throws(() => read({ type: "refund" }), /^Error: body\.type /);
  assert.throws(() => read({ type: "batch", events: [{ type: "credit", account: "a", cents: 0 }] }), /^Error: body\.events\[0\]\.cents /);
});
