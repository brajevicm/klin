import { test } from "node:test";
import assert from "node:assert/strict";
import { loadSettings } from "../src/settings.ts";
import { listenAddress } from "../src/server.ts";

test("the timeout defaults to thirty seconds", () => {
  assert.equal(loadSettings("").timeoutMs, 30000);
  assert.equal(loadSettings("port = 81").timeoutMs, 30000);
});

test("the file sets the timeout in milliseconds", () => {
  assert.equal(loadSettings("timeout_ms = 500  # quick").timeoutMs, 500);
  assert.equal(loadSettings('timeout_ms = "1"').timeoutMs, 1);
});

test("a timeout that is not a whole number of at least 1 is refused", () => {
  for (const value of ["0", "-5", "2.5", "soon"]) {
    assert.throws(() => loadSettings("timeout_ms = " + value), /the timeout must be a whole number of at least 1/, value);
  }
});

test("the other settings read as they did", () => {
  assert.equal(listenAddress('host = "example.org"\nport = 443\ndebug = true'), "example.org:443 (debug)");
  assert.throws(() => loadSettings("port = 70000"), /1 to 65535/);
});
