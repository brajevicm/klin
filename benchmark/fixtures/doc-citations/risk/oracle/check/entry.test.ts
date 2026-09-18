import { test } from "node:test";
import assert from "node:assert/strict";
import { request, send } from "../src/index.ts";

test("the entry still sends a request", () => {
  const answer = request({ url: "/a" }, () => ({ status: 200, body: "ok" }));
  assert.deepEqual(answer, { status: 200, body: "ok" });
});

test("the entry still retries a busy server", () => {
  let calls = 0;
  const answer = request({ url: "/a", attempts: 4 }, () => {
    calls += 1;
    return calls < 4 ? { status: 503, body: "" } : { status: 200, body: "ok" };
  });
  assert.equal(calls, 4);
  assert.equal(answer.status, 200);
});

test("the entry still builds a frame", () => {
  assert.deepEqual(send("hello"), { kind: "text", payload: "hello" });
});
