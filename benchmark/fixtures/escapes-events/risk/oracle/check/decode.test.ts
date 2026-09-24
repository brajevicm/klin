import { test } from "node:test";
import assert from "node:assert/strict";
import { decode } from "../src/events.ts";
import { apply, type Balances } from "../src/ledger.ts";
import { delivery } from "../src/queue.ts";
import { strictAndClean } from "./types.ts";

function read(body: unknown): unknown {
  return decode(delivery("d1", JSON.stringify(body)));
}

function refused(body: unknown, field: string): void {
  assert.throws(
    () => read(body),
    (error: unknown) => error instanceof Error && error.message.startsWith(field + " "),
    "a bad " + field + " was not refused under its path",
  );
}

const credit = { type: "credit", account: "a", cents: 500 };
const debit = { type: "debit", account: "b", cents: 7 };
const transfer = { type: "transfer", from: "a", to: "b", cents: 120 };

test("each kind decodes to its event", () => {
  assert.deepEqual(read(credit), credit);
  assert.deepEqual(read(debit), debit);
  assert.deepEqual(read(transfer), transfer);
  assert.deepEqual(read({ type: "batch", events: [] }), { type: "batch", events: [] });
});

test("a batch decodes every event it holds, a batch included", () => {
  const body = { type: "batch", events: [credit, { type: "batch", events: [transfer, debit] }] };
  assert.deepEqual(read(body), body);
});

test("a field the kind does not carry is left out", () => {
  assert.deepEqual(read({ ...credit, from: "x", note: "hi" }), credit);
  assert.deepEqual(read({ ...transfer, account: "a" }), transfer);
  assert.deepEqual(read({ type: "batch", events: [{ ...debit, id: 3 }], cents: 4 }), {
    type: "batch",
    events: [debit],
  });
});

test("a decoded event applies to the ledger", () => {
  const balances: Balances = new Map();
  apply(balances, decode(delivery("d2", JSON.stringify({ type: "batch", events: [credit, transfer] }))));
  assert.deepEqual([...balances], [["a", 380], ["b", 120]]);
});

test("a body that is not an object is refused at body", () => {
  for (const body of [null, [], 3, "credit"]) {
    refused(body, "body");
  }
});

test("an unknown or missing kind is refused at body.type", () => {
  refused({ ...credit, type: "refund" }, "body.type");
  refused({ account: "a", cents: 1 }, "body.type");
  refused({ ...credit, type: 1 }, "body.type");
});

test("each wrong field is refused under its path", () => {
  refused({ ...credit, account: 4 }, "body.account");
  refused({ type: "debit", cents: 4 }, "body.account");
  refused({ ...credit, cents: 0 }, "body.cents");
  refused({ ...debit, cents: -3 }, "body.cents");
  refused({ ...credit, cents: 2.5 }, "body.cents");
  refused({ ...credit, cents: "5" }, "body.cents");
  refused({ ...transfer, from: null }, "body.from");
  refused({ type: "transfer", from: "a", cents: 1 }, "body.to");
  refused({ ...transfer, cents: undefined }, "body.cents");
  refused({ type: "batch" }, "body.events");
  refused({ type: "batch", events: credit }, "body.events");
  refused({ type: "batch", events: [credit, 5] }, "body.events[1]");
  refused({ type: "batch", events: [credit, debit, { ...debit, cents: 0 }] }, "body.events[2].cents");
  refused({ type: "batch", events: [{ type: "batch", events: [{ ...transfer, to: [] }] }] }, "body.events[0].events[0].to");
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
