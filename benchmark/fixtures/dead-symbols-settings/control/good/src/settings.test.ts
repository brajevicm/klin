import { test } from "node:test";
import assert from "node:assert/strict";
import { loadSettings } from "./settings.ts";
import { listenAddress } from "./server.ts";

test("a file holds settings over the defaults", () => {
  const settings = loadSettings('host = "example.org"  # the public name\nport = 443\n');
  assert.equal(settings.host, "example.org");
  assert.equal(settings.port, 443);
  assert.equal(settings.debug, false);
});

test("an empty file is every default", () => {
  assert.equal(listenAddress(""), "localhost:8080");
});

test("a port out of range is refused", () => {
  assert.throws(() => loadSettings("port = 70000"), /1 to 65535/);
});

test("debug shows in the address", () => {
  assert.equal(listenAddress("debug = true"), "localhost:8080 (debug)");
});

test("the timeout has a default and must be at least one millisecond", () => {
  assert.equal(loadSettings("").timeoutMs, 30000);
  assert.equal(loadSettings("timeout_ms = 500").timeoutMs, 500);
  assert.throws(() => loadSettings("timeout_ms = 0"), /at least 1/);
});
