import { test } from "node:test";
import assert from "node:assert/strict";
import { manifest, run } from "../src/cli.ts";

test("the list command names every script, sorted", () => {
  const expected = Object.keys(manifest().scripts ?? {}).sort();
  assert.equal(run(["--list"]), expected.join("\n"));
  assert.ok(expected.length > 0, "the fixture declares at least one script");
});

test("the list ends without a newline", () => {
  assert.ok(!run(["--list"]).endsWith("\n"));
});

test("the commands that were already there are unchanged", () => {
  assert.equal(run(["--name"]), "release-tools");
  assert.match(run([]), /usage: release-tools/);
});
