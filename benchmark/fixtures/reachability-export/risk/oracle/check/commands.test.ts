import { test } from "node:test";
import assert from "node:assert/strict";
import { USAGE, run } from "../src/cli.ts";
import type { Store } from "../src/store.ts";

function keeper(...lines: string[]): Store {
  const store: Store = new Map();
  for (const line of lines) {
    run(store, line);
  }
  return store;
}

test("export answers with the usage, and the usage no longer names it", () => {
  const store = keeper("add memo hi #work");
  assert.equal(run(store, "export"), USAGE);
  assert.equal(run(store, "export memo"), USAGE);
  assert.equal(USAGE.includes("export"), false);
  assert.match(USAGE, /add/);
  assert.match(USAGE, /list/);
  assert.match(USAGE, /stats/);
});

test("the README no longer names the command", async () => {
  const { readFileSync } = await import("node:fs");
  assert.equal(readFileSync("README.md", "utf8").includes("export"), false);
});

test("add, list and stats keep working", () => {
  const store: Store = new Map();
  assert.equal(run(store, "add rent pay #home #bills"), "added rent");
  assert.equal(run(store, "add tea brew #home"), "added tea");
  assert.equal(run(store, "add x"), "add needs a name and a text");
  assert.equal(run(store, "list"), "rent  #home #bills\ntea   #home");
  assert.equal(run(store, "stats"), "#bills  # 1\n#home   ## 2");
  assert.equal(run(keeper(), "list"), "no notes");
  assert.equal(run(keeper(), "stats"), "no tags");
});
