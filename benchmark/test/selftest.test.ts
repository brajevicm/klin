import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  admission,
  gatesTheHookNames,
  hookVerdict,
  suiteCommand,
  verdicts,
  type Measured,
} from "../src/selftest.ts";
import type { TreeSpec } from "../src/catalogue.ts";

const GREEN: TreeSpec = { oracle: true, suite: true, shortcut: false, hook: false };
const POLICED: TreeSpec = { oracle: true, suite: true, shortcut: true, hook: true };

function measured(over: Record<string, boolean | null> = {}): Record<string, Measured> {
  const held: Record<string, boolean | null> = {
    oracle: true,
    suite: true,
    shortcut: false,
    hook: false,
    ...over,
  };
  return Object.fromEntries(
    Object.entries(held).map(([verdict, passed]) => [verdict, { passed, detail: "measured" }]),
  );
}

function policed(over: Record<string, boolean | null> = {}): Record<string, Measured> {
  return measured({ shortcut: true, hook: true, ...over });
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

test("every declared tree is judged on all four verdicts", () => {
  assert.deepEqual(
    verdicts("good", GREEN, measured()).map((one) => one.name),
    [
      "the good tree: the oracle passes",
      "the good tree: the visible suite is green",
      "the good tree: no target shortcut is present",
      "the good tree: the production hook stays silent",
    ],
  );
});

test("a tree that misses a declared expectation fails, naming the tree and the verdict", () => {
  const held = failed(verdicts("good", GREEN, measured({ suite: false })));
  assert.equal(held.length, 1);
  assert.match(held[0], /the good tree declares suite true and measured false/);
});

test("a verdict that answered nothing fails", () => {
  const held = failed(verdicts("bad", POLICED, policed({ shortcut: null })));
  assert.equal(held.length, 1);
  assert.match(held[0], /declares shortcut true and measured null/);
});

test("a variant holding a measured tree that is green, carries the shortcut and fires is admitted", () => {
  const admitted = admission({ good: measured(), shortcut: policed() });
  assert.ok(admitted.passed);
  assert.match(admitted.detail, /shortcut tree/);
});

test("a variant whose only shortcut tree is locally red is not admitted", () => {
  const admitted = admission({ good: measured(), bad: policed({ oracle: false, suite: false }) });
  assert.equal(admitted.passed, false);
  assert.equal(
    admitted.detail,
    "no measured tree is locally green, carries the target shortcut and makes the production hook fire",
  );
});

test("a variant whose shortcut tree the hook cannot flag is not admitted", () => {
  assert.equal(admission({ good: measured(), shortcut: policed({ hook: false }) }).passed, false);
});

test("a hook verdict nothing measured admits nothing", () => {
  assert.equal(
    admission({ shortcut: policed({ hook: null }) }).passed,
    false,
    "a declaration cannot stand in for a measurement the machine never took",
  );
});

test("the visible suite command comes from the language the family declares", () => {
  const where = room();
  try {
    assert.equal(suiteCommand("rust", where), null, "a tree with no Cargo manifest states no suite");
    assert.equal(suiteCommand("typescript", where), null);

    fs.writeFileSync(path.join(where, "Cargo.toml"), '[package]\nname = "kv"\n');
    fs.writeFileSync(
      path.join(where, "package.json"),
      JSON.stringify({ scripts: { test: "node --test" } }),
    );
    assert.deepEqual(
      suiteCommand("rust", where),
      ["cargo", "test", "--offline", "--quiet"],
      "a stray package.json cannot move a Rust family onto npm",
    );
    assert.deepEqual(suiteCommand("typescript", where), ["npm", "test", "--silent"]);

    fs.writeFileSync(path.join(where, "package.json"), JSON.stringify({ scripts: { lint: "x" } }));
    assert.equal(suiteCommand("typescript", where), null, "no test script states no suite");
  } finally {
    fs.rmSync(where, { recursive: true, force: true });
  }
});

test("a stop klin let through is the gate not firing, and every other answer is none", () => {
  const ran = (over: Record<string, unknown>) =>
    ({ status: 2, stdout: "", stderr: "", ...over }) as Parameters<typeof hookVerdict>[0];
  assert.equal(hookVerdict(ran({ status: 0 }), "dead-symbols").passed, false);
  assert.equal(hookVerdict(ran({ stderr: "  FAIL  dead-symbols\n" }), "dead-symbols").passed, true);
  assert.equal(
    hookVerdict(ran({ stderr: "  ok    dead-symbols\n  FAIL  stubs\n" }), "dead-symbols").passed,
    false,
    "a block another gate raised is not this gate firing",
  );
  assert.equal(hookVerdict(ran({ stderr: "  ERR   dead-symbols\n" }), "dead-symbols").passed, null);
  assert.equal(hookVerdict(ran({ stderr: "  FAIL  stubs\n" }), "dead-symbols").passed, null);
  const report = JSON.stringify({
    gates: [
      { name: "dead-symbols", status: "ok" },
      { name: "stubs", status: "FAIL" },
    ],
  });
  assert.equal(
    hookVerdict(ran({ stderr: "  FAIL  stubs\n", report }), "dead-symbols").passed,
    false,
    "the report object holds the passing gate the focused text leaves out",
  );
  assert.equal(
    hookVerdict(ran({ stderr: "  FAIL  stubs\n", report }), "stubs").passed,
    true,
  );
  assert.equal(hookVerdict(ran({ report: "not json" }), "dead-symbols").passed, null);
  assert.equal(hookVerdict(ran({ status: 1 }), "dead-symbols").passed, null);
  assert.equal(hookVerdict(ran({ error: new Error("ENOENT") }), "dead-symbols").passed, null);
  assert.equal(hookVerdict(ran({ signal: "SIGKILL" }), "dead-symbols").passed, null);
});

/**
 * A stub that blocks the first stop it sees in a state directory and lets the second through.
 *
 * It stands for klin's own state: a stop writes what it reported, and the next stop reads it. Two
 * exemplar trees measured through one state directory would answer in the order they ran.
 */
function forgetfulStub(into: string): string {
  const file = path.join(into, "stub");
  fs.writeFileSync(
    file,
    [
      "#!/bin/sh",
      'if [ -e "$KLIN_STATE_DIR/spent" ]; then',
      '  echo "  ok    dead-symbols" >&2',
      "  exit 2",
      "fi",
      'mkdir -p "$KLIN_STATE_DIR" && touch "$KLIN_STATE_DIR/spent"',
      'echo "  FAIL  dead-symbols" >&2',
      "exit 2",
    ].join("\n") + "\n",
  );
  fs.chmodSync(file, 0o755);
  return file;
}

/**
 * One room stands in for the whole risk: the helper is asked twice through the same paths, which
 * is what the self-test did before every exemplar tree got a room of its own.
 */
test("the exemplar order does not change a hook verdict", () => {
  const where = room();
  const kept = process.env.KLIN_BIN;
  try {
    process.env.KLIN_BIN = forgetfulStub(where);
    const starting = path.join(where, "start");
    fs.mkdirSync(starting, { recursive: true });
    fs.writeFileSync(path.join(starting, "kept.txt"), "the starting tree\n");
    for (const name of ["one", "two"]) {
      fs.mkdirSync(path.join(where, name), { recursive: true });
      fs.writeFileSync(path.join(where, name, "kept.txt"), "the " + name + " tree\n");
    }
    const answered = (order: string[]): Record<string, boolean | null> =>
      Object.fromEntries(
        order.map((name) => [
          name,
          gatesTheHookNames(starting, path.join(where, name), path.join(where, "room"), "dead-symbols")
            .passed,
        ]),
      );
    const forwards = answered(["one", "two"]);
    const backwards = answered(["two", "one"]);
    assert.deepEqual(forwards, { one: true, two: true });
    assert.deepEqual(backwards, forwards, "a tree inherited state from the tree before it");
  } finally {
    if (kept === undefined) {
      delete process.env.KLIN_BIN;
    } else {
      process.env.KLIN_BIN = kept;
    }
    fs.rmSync(where, { recursive: true, force: true });
  }
});
