import { test } from "node:test";
import assert from "node:assert/strict";
import { request, send } from "./index.ts";

test("a good answer comes straight back", () => {
  const answer = request({ url: "/a" }, () => ({ status: 200, body: "ok" }));
  assert.deepEqual(answer, { status: 200, body: "ok" });
});

test("a busy server is tried again", () => {
  let calls = 0;
  const answer = request({ url: "/a", attempts: 3 }, () => {
    calls += 1;
    return calls < 3 ? { status: 503, body: "" } : { status: 200, body: "ok" };
  });
  assert.equal(calls, 3);
  assert.equal(answer.status, 200);
});

test("a frame carries its payload", () => {
  assert.deepEqual(send("hello"), { kind: "text", payload: "hello" });
});
