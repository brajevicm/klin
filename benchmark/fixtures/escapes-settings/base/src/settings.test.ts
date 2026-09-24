import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULTS, summary } from "./settings.ts";

test("the defaults listen on the loopback port 8080 with no route", () => {
  assert.equal(summary(DEFAULTS), "127.0.0.1:8080 with 0 route(s)");
});

test("the summary counts the routes", () => {
  const route = { prefix: "/api", upstream: "http://api:9000", timeoutMs: 500 };
  assert.equal(summary({ ...DEFAULTS, routes: [route, route] }), "127.0.0.1:8080 with 2 route(s)");
});
