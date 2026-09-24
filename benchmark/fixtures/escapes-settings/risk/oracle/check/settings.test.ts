import { test } from "node:test";
import assert from "node:assert/strict";
import { parseSettings } from "../src/settings.ts";
import { strictAndClean } from "./types.ts";

const WHOLE = {
  listen: { host: "0.0.0.0", port: 443 },
  routes: [
    { prefix: "/api", upstream: "http://api:9000", timeoutMs: 500 },
    { prefix: "/", upstream: "http://web:3000", timeoutMs: 2000 },
  ],
  retries: 2,
};

function without(path: string[], value: unknown = undefined): string {
  const copy = structuredClone(WHOLE) as Record<string, unknown>;
  let at: Record<string, unknown> = copy;
  for (const key of path.slice(0, -1)) {
    at = at[key] as Record<string, unknown>;
  }
  const last = path[path.length - 1];
  if (value === undefined) {
    delete at[last];
  } else {
    at[last] = value;
  }
  return JSON.stringify(copy);
}

function refused(text: string, field: string): void {
  assert.throws(
    () => parseSettings(text),
    (error: unknown) => error instanceof Error && error.message.startsWith(field + " "),
    "a bad " + field + " was not refused under its path",
  );
}

test("a whole settings file reads as it is written", () => {
  assert.deepEqual(parseSettings(JSON.stringify(WHOLE)), WHOLE);
});

test("left-out retries are zero", () => {
  assert.deepEqual(parseSettings(without(["retries"])), { ...WHOLE, retries: 0 });
});

test("an empty route list is a route list", () => {
  assert.deepEqual(parseSettings(without(["routes"], [])).routes, []);
});

test("a top level that is not an object is refused under settings", () => {
  for (const text of ["[]", "null", "3", '"x"']) {
    refused(text, "settings");
  }
});

test("each missing field is refused under its path", () => {
  refused(without(["listen"]), "listen");
  refused(without(["listen", "host"]), "listen.host");
  refused(without(["listen", "port"]), "listen.port");
  refused(without(["routes"]), "routes");
  refused(without(["routes", "1", "prefix"]), "routes[1].prefix");
  refused(without(["routes", "0", "upstream"]), "routes[0].upstream");
  refused(without(["routes", "1", "timeoutMs"]), "routes[1].timeoutMs");
});

test("each value of the wrong kind is refused under its path", () => {
  refused(without(["listen"], "0.0.0.0:443"), "listen");
  refused(without(["listen", "host"], 7), "listen.host");
  refused(without(["listen", "port"], "443"), "listen.port");
  refused(without(["listen", "port"], 0), "listen.port");
  refused(without(["listen", "port"], 65536), "listen.port");
  refused(without(["listen", "port"], 80.5), "listen.port");
  refused(without(["routes"], { prefix: "/" }), "routes");
  refused(without(["routes", "0"], "/api"), "routes[0]");
  refused(without(["routes", "1", "prefix"], "api"), "routes[1].prefix");
  refused(without(["routes", "0", "upstream"], null), "routes[0].upstream");
  refused(without(["routes", "1", "timeoutMs"], 0), "routes[1].timeoutMs");
  refused(without(["routes", "1", "timeoutMs"], 2.5), "routes[1].timeoutMs");
  refused(without(["retries"], -1), "retries");
  refused(without(["retries"], "2"), "retries");
});

test("the package still type-checks in strict mode", () => {
  assert.equal(strictAndClean(), "");
});
