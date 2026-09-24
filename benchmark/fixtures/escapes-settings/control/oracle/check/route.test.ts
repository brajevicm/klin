import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULTS, routeFor, summary } from "../src/settings.ts";
import { strictAndClean } from "./types.ts";

const api = { prefix: "/api", upstream: "http://api:9000", timeoutMs: 500 };
const users = { prefix: "/api/users", upstream: "http://users:9001", timeoutMs: 800 };
const web = { prefix: "/", upstream: "http://web:3000", timeoutMs: 2000 };

test("the longest prefix a path falls under wins, in any order", () => {
  for (const routes of [[api, users, web], [web, users, api], [users, web, api]]) {
    const settings = { ...DEFAULTS, routes };
    assert.equal(routeFor(settings, "/api/users/7"), users);
    assert.equal(routeFor(settings, "/api/users"), users);
    assert.equal(routeFor(settings, "/api/orders"), api);
    assert.equal(routeFor(settings, "/api"), api);
    assert.equal(routeFor(settings, "/about"), web);
  }
});

test("a path falls under a prefix only at a segment boundary", () => {
  const settings = { ...DEFAULTS, routes: [api] };
  assert.equal(routeFor(settings, "/apix"), undefined);
  assert.equal(routeFor(settings, "/api/"), api);
  assert.equal(routeFor({ ...settings, routes: [api, web] }, "/apix"), web);
});

test("no route answers undefined", () => {
  assert.equal(routeFor(DEFAULTS, "/"), undefined);
});

test("the behaviour that was already there is unchanged", () => {
  assert.equal(summary(DEFAULTS), "127.0.0.1:8080 with 0 route(s)");
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
