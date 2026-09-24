import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULTS, parseSettings, summary } from "./settings.ts";

test("the defaults listen on the loopback port 8080 with no route", () => {
  assert.equal(summary(DEFAULTS), "127.0.0.1:8080 with 0 route(s)");
});

test("the summary counts the routes", () => {
  const route = { prefix: "/api", upstream: "http://api:9000", timeoutMs: 500 };
  assert.equal(summary({ ...DEFAULTS, routes: [route, route] }), "127.0.0.1:8080 with 2 route(s)");
});

const FILE = {
  listen: { host: "0.0.0.0", port: 443 },
  routes: [{ prefix: "/api", upstream: "http://api:9000", timeoutMs: 500 }],
  retries: 1,
};

test("a settings file reads into settings", () => {
  assert.deepEqual(parseSettings(JSON.stringify(FILE)), FILE);
});

test("left-out retries are zero", () => {
  const { retries, ...rest } = FILE;
  assert.equal(parseSettings(JSON.stringify(rest)).retries, 0);
});

test("a bad field is refused under its path", () => {
  assert.throws(() => parseSettings("[]"), /^Error: settings /);
  assert.throws(() => parseSettings(JSON.stringify({ ...FILE, listen: { host: "h", port: 0 } })), /^Error: listen\.port /);
  const route = { prefix: "/api", upstream: 3, timeoutMs: 500 };
  assert.throws(() => parseSettings(JSON.stringify({ ...FILE, routes: [route] })), /^Error: routes\[0\]\.upstream /);
});
