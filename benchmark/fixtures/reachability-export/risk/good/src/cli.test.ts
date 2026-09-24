import { test } from "node:test";
import assert from "node:assert/strict";
import { USAGE, run } from "./cli.ts";
import type { Store } from "./store.ts";

function keeper(...lines: string[]): Store {
  const store: Store = new Map();
  for (const line of lines) {
    run(store, line);
  }
  return store;
}

test("add stores a note and its tags", () => {
  const store: Store = new Map();
  assert.equal(run(store, "add milk buy #home #shop"), "added milk");
  assert.deepEqual(store.get("milk"), { name: "milk", text: "buy #home #shop", tags: ["home", "shop"] });
  assert.equal(run(store, "add milk"), "add needs a name and a text");
});

test("list shows each note with its tags in columns", () => {
  assert.equal(run(keeper(), "list"), "no notes");
  assert.equal(run(keeper("add milk buy #home", "add rent pay #home #bills"), "list"), "milk  #home\nrent  #home #bills");
});

test("stats counts the notes of each tag", () => {
  assert.equal(run(keeper(), "stats"), "no tags");
  assert.equal(run(keeper("add a x #home", "add b y #home #work"), "stats"), "#home  ## 2\n#work  # 1");
});

test("an unknown command answers with the usage", () => {
  assert.equal(run(keeper(), "delete milk"), USAGE);
  assert.equal(run(keeper(), ""), USAGE);
  assert.equal(run(keeper("add a x"), "export"), USAGE);
});
