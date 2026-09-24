import { test } from "node:test";
import assert from "node:assert/strict";
import { loadSettings } from "../src/settings.ts";
import { listenAddress } from "../src/server.ts";

test("a JSON file holds settings over the defaults", () => {
  assert.deepEqual({ ...loadSettings('{ "host": "example.org", "port": 443, "debug": true }') }, { host: "example.org", port: 443, debug: true });
  assert.deepEqual({ ...loadSettings('{ "port": 9000 }') }, { host: "localhost", port: 9000, debug: false });
  assert.deepEqual({ ...loadSettings("{}") }, { host: "localhost", port: 8080, debug: false });
});

test("a port that is not a whole number from 1 to 65535 is refused", () => {
  for (const port of ["0", "65536", "80.5", '"443"']) {
    assert.throws(() => loadSettings('{ "port": ' + port + " }"), /the port must be a whole number from 1 to 65535/, port);
  }
  assert.equal(loadSettings('{ "port": 65535 }').port, 65535);
});

test("the address comes from the JSON settings", () => {
  assert.equal(listenAddress('{ "host": "0.0.0.0", "port": 80 }'), "0.0.0.0:80");
  assert.equal(listenAddress('{ "debug": true }'), "localhost:8080 (debug)");
});
