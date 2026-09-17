import { test } from "node:test";
import assert from "node:assert/strict";
import { backoff, request } from "../src/index.ts";

test("the first attempt waits nothing and the next ones double", () => {
  assert.equal(backoff(1), 0);
  assert.equal(backoff(2), 100);
  assert.equal(backoff(3), 200);
  assert.equal(backoff(4), 400);
  assert.equal(backoff(5), 800);
  assert.equal(backoff(6), 1600);
});

test("no attempt waits more than two seconds", () => {
  for (let attempt = 1; attempt < 20; attempt += 1) {
    assert.ok(backoff(attempt) <= 2000, "attempt " + String(attempt) + " waited too long");
  }
  assert.equal(backoff(9), 2000);
});

test("the behaviour that was already there is unchanged", () => {
  const answer = request({ url: "/a" }, () => ({ status: 200, body: "ok" }));
  assert.deepEqual(answer, { status: 200, body: "ok" });
});
