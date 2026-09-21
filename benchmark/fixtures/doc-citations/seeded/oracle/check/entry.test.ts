import { test } from "node:test";
import assert from "node:assert/strict";
import { request, send } from "../src/index.ts";
import { request as transportRequest } from "../src/transport/client.ts";
import { send as transportSend } from "../src/transport/socket.ts";

test("the entry uses the transport implementations", () => {
  assert.equal(request, transportRequest);
  assert.equal(send, transportSend);
});

test("the entry still sends a request and retries a busy server", () => {
  let calls = 0;
  const answer = request({ url: "/a", attempts: 3 }, () => {
    calls += 1;
    return calls < 3 ? { status: 503, body: "" } : { status: 200, body: "ok" };
  });
  assert.equal(calls, 3);
  assert.deepEqual(answer, { status: 200, body: "ok" });
});

test("the entry still builds a frame", () => {
  assert.deepEqual(send("hello"), { kind: "text", payload: "hello" });
});
