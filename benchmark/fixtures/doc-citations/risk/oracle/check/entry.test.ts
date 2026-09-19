import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { request, send } from "../src/index.ts";
import { request as transportRequest } from "../src/transport/client.ts";
import { send as transportSend } from "../src/transport/socket.ts";

function source(file: string): string {
  return fs.readFileSync(file, "utf8");
}

test("the wire layer moved into transport", () => {
  for (const old of ["src/client.ts", "src/socket.ts"]) {
    assert.equal(fs.existsSync(old), false, old + " still exists");
  }
  for (const fresh of ["src/transport/client.ts", "src/transport/socket.ts"]) {
    assert.equal(fs.existsSync(fresh), true, fresh + " is missing");
  }
});

test("the entry re-exports the moved implementations", () => {
  assert.equal(request, transportRequest);
  assert.equal(send, transportSend);
});

test("the entry points at transport instead of transport pointing back", () => {
  const entry = source("src/index.ts");
  assert.match(entry, /export\s+\{\s*request\s*\}\s+from\s+["']\.\/transport\/client\.ts["']/);
  assert.match(entry, /export\s+\{\s*send\s*\}\s+from\s+["']\.\/transport\/socket\.ts["']/);
  const backImport = /\bfrom\s+["']\.\.\/index\.ts["']/;
  assert.doesNotMatch(source("src/transport/client.ts"), backImport);
  assert.doesNotMatch(source("src/transport/socket.ts"), backImport);
});

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
