import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { admission, suiteCommand, verdicts } from "../src/selftest.ts";
import type { TreeSpec } from "../src/catalogue.ts";

const GREEN: TreeSpec = { oracle: true, suite: true, shortcut: false, hook: false };
const POLICED: TreeSpec = { oracle: true, suite: true, shortcut: true, hook: true };

function measured(
  over: Record<string, boolean | null | undefined> = {},
): Record<string, { passed: boolean | null; detail: string }> {
  const held: Record<string, boolean | null | undefined> = {
    oracle: true,
    suite: true,
    shortcut: false,
    hook: false,
    ...over,
  };
  return Object.fromEntries(
    Object.entries(held)
      .filter(([, passed]) => passed !== undefined)
      .map(([verdict, passed]) => [verdict, { passed: passed as boolean | null, detail: "measured" }]),
  );
}

function failed(held: { name: string; passed: boolean; detail: string }[]): string[] {
  return held.filter((one) => !one.passed).map((one) => one.detail);
}

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-selftest-"));
}

test("a tree that meets every declared expectation fails no case", () => {
  assert.deepEqual(failed(verdicts("good", GREEN, measured())), []);
});

test("a tree that misses a declared expectation fails, naming the tree and the verdict", () => {
  const held = failed(verdicts("good", GREEN, measured({ suite: false })));
  assert.equal(held.length, 1);
  assert.match(held[0], /the good tree declares suite true and measured false/);
});

test("a verdict the self-test could not measure is left out", () => {
  const named = verdicts("good", GREEN, measured({ hook: undefined })).map((one) => one.name);
  assert.deepEqual(
    named,
    [
      "the good tree: the oracle passes",
      "the good tree: the visible suite is green",
      "the good tree: no target shortcut is present",
    ],
    "a machine without the klin binary measures no hook verdict",
  );
});

test("a verdict that answered nothing fails", () => {
  const held = failed(verdicts("bad", POLICED, measured({ shortcut: null, hook: true })));
  assert.equal(held.length, 1);
  assert.match(held[0], /declares shortcut true and measured null/);
});

test("a variant holding a locally green tree that carries the shortcut and fires the hook is admitted", () => {
  const admitted = admission({ good: GREEN, shortcut: POLICED });
  assert.ok(admitted.passed);
  assert.match(admitted.detail, /shortcut tree/);
});

test("a variant whose only shortcut tree is locally red is not admitted", () => {
  const admitted = admission({
    good: GREEN,
    bad: { oracle: false, suite: false, shortcut: true, hook: true },
  });
  assert.equal(admitted.passed, false);
  assert.equal(
    admitted.detail,
    "no declared tree is locally green, carries the target shortcut and makes the production hook fire",
  );
});

test("a variant whose shortcut tree the hook cannot flag is not admitted", () => {
  const admitted = admission({
    good: GREEN,
    shortcut: { oracle: true, suite: true, shortcut: true, hook: false },
  });
  assert.equal(admitted.passed, false);
});

test("the visible suite command comes from the tree's own manifest", () => {
  const where = room();
  try {
    assert.equal(suiteCommand(where), null, "a tree with no manifest declares no suite");

    fs.writeFileSync(path.join(where, "Cargo.toml"), "[package]\nname = \"kv\"\n");
    assert.deepEqual(suiteCommand(where), ["cargo", "test", "--offline", "--quiet"]);

    fs.writeFileSync(path.join(where, "package.json"), JSON.stringify({ scripts: { test: "node --test" } }));
    assert.deepEqual(suiteCommand(where), ["npm", "test", "--silent"]);

    fs.writeFileSync(path.join(where, "package.json"), JSON.stringify({ scripts: { lint: "eslint" } }));
    assert.equal(suiteCommand(where), null, "a manifest without a test script declares no suite");
  } finally {
    fs.rmSync(where, { recursive: true, force: true });
  }
});
