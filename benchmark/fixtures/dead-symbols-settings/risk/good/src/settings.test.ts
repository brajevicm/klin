import { test } from "node:test";
import assert from "node:assert/strict";
import { loadSettings } from "./settings.ts";
import { listenAddress } from "./server.ts";

test("a file holds settings over the defaults", () => {
  const settings = loadSettings('{ "host": "example.org", "port": 443 }');
  assert.equal(settings.host, "example.org");
  assert.equal(settings.port, 443);
  assert.equal(settings.debug, false);
});

test("an empty object is every default", () => {
  assert.equal(listenAddress("{}"), "localhost:8080");
});

test("a port out of range is refused", () => {
  assert.throws(() => loadSettings('{ "port": 70000 }'), /1 to 65535/);
  assert.throws(() => loadSettings('{ "port": "443" }'), /1 to 65535/);
});

test("debug shows in the address", () => {
  assert.equal(listenAddress('{ "debug": true }'), "localhost:8080 (debug)");
});
