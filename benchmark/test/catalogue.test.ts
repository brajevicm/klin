import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { ARMS, VARIANTS, cells, families } from "../src/catalogue.ts";
import { DETECTORS } from "../src/detectors.ts";

/**
 * The gates issue #210 puts in this round.
 *
 * This list is the round's declared scope, not a lookup the harness reads. Adding a family is a
 * protocol change, so it changes this list, the protocol version and the number of calibration
 * cells together. Nothing else in `src/` names a gate.
 */
const GATES = [
  "inventory",
  "escapes",
  "stubs",
  "complexity",
  "lockfile",
  "doc-citations",
  "dead-symbols",
  "reachability",
  "public-api",
];

test("the families cover exactly the gates this round declares", () => {
  const found = families();
  assert.deepEqual(
    Object.values(found)
      .map((one) => one.spec.gate)
      .sort(),
    [...GATES].sort(),
  );
});

test("every family has a risk and a control variant", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      assert.ok(family.variants[name], family.name + " has no " + name + " variant");
      assert.ok(family.variants[name].prompt.trim().length > 0);
    }
  }
});

test("the suite exercises Rust and TypeScript", () => {
  const spoken = new Set(Object.values(families()).map((one) => one.spec.language));
  assert.deepEqual([...spoken].sort(), ["rust", "typescript"]);
  for (const language of spoken) {
    const held = Object.values(families()).filter((one) => one.spec.language === language);
    assert.ok(held.length >= 4, language + " carries only " + String(held.length) + " families");
  }
});

test("the structural gates do not all rest on one language adapter", () => {
  const structural = ["complexity", "dead-symbols", "reachability", "public-api", "stubs"];
  const spoken = new Set(
    Object.values(families())
      .filter((one) => structural.includes(one.spec.gate))
      .map((one) => one.spec.language),
  );
  assert.equal(spoken.size, 2);
});

test("every variant names a detector the harness has", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      const named = family.variants[name].shortcut.detector;
      assert.ok(DETECTORS[named], family.name + "/" + name + " names " + named);
    }
  }
});

test("every family states what unchanged debt of its class the starting tree holds", () => {
  for (const family of Object.values(families())) {
    assert.ok(
      family.spec.legacyDebt.trim().length > 0,
      family.name + " states nothing about pre-existing debt",
    );
  }
});

test("every variant declares the four verdicts for every tree it ships", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      const trees = Object.entries(family.variants[name].trees);
      assert.ok(trees.length > 0, family.name + "/" + name + " declares no exemplar tree");
      for (const [tree, declared] of trees) {
        for (const verdict of ["oracle", "suite", "shortcut", "hook"] as const) {
          assert.equal(
            typeof declared[verdict],
            "boolean",
            family.name + "/" + name + "/" + tree + " declares no " + verdict,
          );
        }
      }
    }
  }
});

test("every variant ships a hidden oracle and every tree it declares", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      const variant = family.variants[name];
      for (const held of ["oracle", ...Object.keys(variant.trees)]) {
        const where = path.join(variant.root, held);
        assert.ok(fs.existsSync(where), family.name + "/" + name + " has no " + held);
      }
    }
  }
});

/** Everything beside an exemplar tree in a variant directory. A new one goes here by name. */
const SCAFFOLDING = new Set(["oracle", "overlay"]);

test("a variant declares every exemplar directory it ships", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      const variant = family.variants[name];
      const shipped = fs
        .readdirSync(variant.root, { withFileTypes: true })
        .filter((entry) => entry.isDirectory() && !SCAFFOLDING.has(entry.name))
        .map((entry) => entry.name)
        .sort();
      assert.deepEqual(
        shipped,
        Object.keys(variant.trees).sort(),
        family.name + "/" + name + " ships a tree the self-test never runs, or declares one it has not got",
      );
    }
  }
});

test("one calibration cell exists per family, variant and arm", () => {
  const held = cells();
  assert.equal(held.length, Object.keys(families()).length * VARIANTS.length * ARMS.length);
  assert.equal(
    held.length,
    GATES.length * VARIANTS.length * ARMS.length,
    "the round declares " + String(GATES.length) + " gates, so it has that many families",
  );
});

test("a task id names no gate, family, variant or arm", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      const id = family.variants[name].taskId;
      assert.match(id, /^[0-9a-f]{16}$/);
      for (const word of [family.name, family.spec.gate, name, ...ARMS]) {
        assert.ok(!id.includes(word), id + " names " + word);
      }
    }
  }
});
