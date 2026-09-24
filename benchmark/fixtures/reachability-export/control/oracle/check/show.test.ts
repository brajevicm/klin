import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { USAGE, run } from "../src/cli.ts";
import type { Store } from "../src/store.ts";

function keeper(...lines: string[]): Store {
  const store: Store = new Map();
  for (const line of lines) {
    run(store, line);
  }
  return store;
}

test("show answers the text and a line of tags", () => {
  const store = keeper("add rent pay #home #bills", "add tea brew");
  assert.equal(run(store, "show rent"), "pay #home #bills\n#home #bills");
  assert.equal(run(store, "show tea"), "brew");
});

test("show answers for a name the keeper does not hold, and for no name", () => {
  assert.equal(run(keeper("add tea brew"), "show milk"), "no note named milk");
  assert.equal(run(keeper(), "show"), "show needs a name");
});

test("the usage and the README name the new command", () => {
  assert.match(USAGE, /show/);
  assert.match(readFileSync("README.md", "utf8"), /show/);
});

test("the other commands keep working", () => {
  const store = keeper("add a x #home", "add b y #home #work");
  assert.equal(run(store, "list"), "a  #home\nb  #home #work");
  assert.equal(run(store, "stats"), "#home  ## 2\n#work  # 1");
  assert.equal(run(keeper('add memo say "hi" #work'), "export"), 'name,text,tags\nmemo,"say ""hi"" #work",work');
  assert.equal(run(store, "nope"), USAGE);
});
