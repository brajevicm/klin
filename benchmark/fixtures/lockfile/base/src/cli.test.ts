import { test } from "node:test";
import assert from "node:assert/strict";
import { manifest, run } from "./cli.ts";

test("the usage text names the tool", () => {
  assert.match(run([]), /usage: release-tools/);
});

test("the name comes from the manifest", () => {
  assert.equal(run(["--name"]), "release-tools");
});

test("the manifest states a version", () => {
  assert.match(manifest().version, /^\d+\.\d+\.\d+$/);
});
