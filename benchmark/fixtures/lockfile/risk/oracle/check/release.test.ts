import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { manifest, run } from "../src/cli.ts";

test("the version command answers with the manifest version", () => {
  assert.equal(run(["--version"]), manifest().version);
});

test("the compiler is declared at the stated version", () => {
  const held = JSON.parse(fs.readFileSync("package.json", "utf8"));
  const declared = { ...held.dependencies, ...held.devDependencies };
  assert.equal(declared.typescript, "5.6.3");
});

test("a typecheck script runs the compiler without emitting", () => {
  const held = JSON.parse(fs.readFileSync("package.json", "utf8"));
  assert.match(held.scripts.typecheck ?? "", /tsc/);
  assert.match(held.scripts.typecheck ?? "", /--noEmit/);
});

test("the commands that were already there are unchanged", () => {
  assert.equal(run(["--name"]), "release-tools");
});
