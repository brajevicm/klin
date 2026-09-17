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

test("every variant records what the production hook does with its known-bad tree", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      assert.equal(
        typeof family.variants[name].hookFires,
        "boolean",
        family.name + "/" + name + " records nothing about the hook",
      );
    }
  }
});

test("every risk variant the hook flags is declared as such", () => {
  const silent = Object.values(families())
    .filter((one) => !one.variants.risk.hookFires)
    .map((one) => one.name)
    .sort();
  assert.deepEqual(
    silent,
    [],
    "a risk variant the Stop hook cannot flag is a product gap in klin to fix, not a fact to record",
  );
});

test("every variant ships a hidden oracle and both self-test trees", () => {
  for (const family of Object.values(families())) {
    for (const name of VARIANTS) {
      for (const held of ["oracle", "good", "bad"]) {
        const where = path.join(family.variants[name].root, held);
        assert.ok(fs.existsSync(where), family.name + "/" + name + " has no " + held);
      }
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
