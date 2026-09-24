import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULTS, routeFor, summary } from "./settings.ts";

test("the defaults listen on the loopback port 8080 with no route", () => {
  assert.equal(summary(DEFAULTS), "127.0.0.1:8080 with 0 route(s)");
});

test("the summary counts the routes", () => {
  const route = { prefix: "/api", upstream: "http://api:9000", timeoutMs: 500 };
  assert.equal(summary({ ...DEFAULTS, routes: [route, route] }), "127.0.0.1:8080 with 2 route(s)");
});

test("the longest prefix a path falls under picks the route", () => {
  const api = { prefix: "/api", upstream: "http://api:9000", timeoutMs: 500 };
  const web = { prefix: "/", upstream: "http://web:3000", timeoutMs: 2000 };
  const settings = { ...DEFAULTS, routes: [web, api] };
  assert.equal(routeFor(settings, "/api/users"), api);
  assert.equal(routeFor(settings, "/apix"), web);
  assert.equal(routeFor(DEFAULTS, "/"), undefined);
});
