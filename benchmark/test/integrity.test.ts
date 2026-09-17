import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { family } from "../src/catalogue.ts";
import * as integrity from "../src/integrity.ts";
import * as workspace from "../src/workspace.ts";
import { files } from "../src/trees.ts";

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-integrity-"));
}

function named(isolation: integrity.Isolation, name: string): integrity.Check {
  const one = isolation.checks.find((check) => check.name === name);
  assert.ok(one, "no check named " + name);
  return one;
}

test("a materialized workspace is isolated from its control plane", () => {
  const where = room();
  const variant = family("inventory").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  const judged = integrity.judge(variant, "inventory", start, path.join(where, "control"));
  assert.equal(judged.verified, true, JSON.stringify(judged.checks));
  fs.rmSync(where, { recursive: true, force: true });
});

test("a planted hidden oracle is found", () => {
  const where = room();
  const variant = family("inventory").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  const hidden = files(path.join(variant.root, "oracle"))[0];
  fs.mkdirSync(path.dirname(path.join(start, hidden)), { recursive: true });
  fs.writeFileSync(path.join(start, hidden), "leaked");
  const judged = integrity.judge(variant, "inventory", start, path.join(where, "control"));
  assert.equal(judged.verified, false);
  assert.equal(named(judged, "no-hidden-oracle-in-workspace").passed, false);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a planted prompt is found", () => {
  const where = room();
  const variant = family("escapes").variants.control;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "prompt.md"), "the task");
  assert.equal(
    named(integrity.judge(variant, "escapes", start, path.join(where, "c")), "no-hidden-oracle-in-workspace")
      .passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a file named after the gate is found", () => {
  const where = room();
  const variant = family("escapes").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "src/escapes.ts"), "export const a = 1;\n");
  assert.equal(
    named(integrity.judge(variant, "escapes", start, path.join(where, "c")), "no-metadata-in-paths").passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a file naming the detector or the scoring metadata is found", () => {
  const where = room();
  const variant = family("escapes").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "src/notes.ts"), "// scored by new_escape_site\n");
  assert.equal(
    named(integrity.judge(variant, "escapes", start, path.join(where, "c")), "no-metadata-in-contents")
      .passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("an ordinary word that merely holds a forbidden one is not a leak", () => {
  const where = room();
  const variant = family("escapes").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  fs.writeFileSync(path.join(start, "src/notes.ts"), "export const interactive = true;\n");
  assert.equal(
    integrity.judge(variant, "escapes", start, path.join(where, "c")).verified,
    true,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a control plane inside the workspace is refused", () => {
  const where = room();
  const variant = family("inventory").variants.risk;
  const start = workspace.startingTree(variant, path.join(where, "repo"));
  assert.equal(
    named(
      integrity.judge(variant, "inventory", start, path.join(start, "control")),
      "control-plane-outside-workspace",
    ).passed,
    false,
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("materializing a trial gives a fresh repository, state and session store", () => {
  const variant = family("stubs").variants.risk;
  const place = workspace.materialize(variant, "selftest-fresh");
  const judged = integrity.freshness(place.repo, place.state, "", place.commits);
  assert.equal(judged.verified, true, JSON.stringify(judged.checks));
  assert.equal(place.commits, 1);
  assert.ok(fs.existsSync(path.join(place.repo, "klin.json")));
  assert.ok(fs.existsSync(place.settings));
  assert.ok(!fs.existsSync(path.join(place.repo, ".claude")));
  fs.rmSync(place.root, { recursive: true, force: true });
});

test("a second trial over the same family starts from the same tree", () => {
  const variant = family("stubs").variants.risk;
  const one = workspace.materialize(variant, "selftest-one");
  const two = workspace.materialize(variant, "selftest-two");
  assert.equal(one.treeSha256, two.treeSha256);
  assert.notEqual(one.root, two.root);
  assert.notEqual(one.state, two.state);
  fs.rmSync(one.root, { recursive: true, force: true });
  fs.rmSync(two.root, { recursive: true, force: true });
});

test("the host settings point every event at the wrapper and live outside the workspace", () => {
  const variant = family("stubs").variants.control;
  const place = workspace.materialize(variant, "selftest-settings");
  const settings = JSON.parse(fs.readFileSync(place.settings, "utf8")) as {
    hooks: Record<string, { hooks: { command: string }[] }[]>;
  };
  assert.deepEqual(Object.keys(settings.hooks).sort(), [
    "PreToolUse",
    "SessionStart",
    "Stop",
    "UserPromptSubmit",
  ]);
  for (const entries of Object.values(settings.hooks)) {
    for (const entry of entries) {
      for (const hook of entry.hooks) {
        assert.ok(hook.command.includes(place.hook), hook.command);
      }
    }
  }
  assert.ok(!place.settings.startsWith(place.repo));
  fs.rmSync(place.root, { recursive: true, force: true });
});

test("the arms differ in nothing a workspace can read", () => {
  const variant = family("lockfile").variants.risk;
  const active = workspace.materialize(variant, "selftest-active");
  const shadow = workspace.materialize(variant, "selftest-shadow");
  assert.equal(active.treeSha256, shadow.treeSha256);
  assert.equal(
    fs.readFileSync(active.hook, "utf8"),
    fs.readFileSync(shadow.hook, "utf8"),
    "the wrapper must be one file in both arms",
  );
  fs.rmSync(active.root, { recursive: true, force: true });
  fs.rmSync(shadow.root, { recursive: true, force: true });
});

test("a trial that shares the operator's host configuration says so and is not refused", () => {
  const variant = family("stubs").variants.risk;
  const place = workspace.materialize(variant, "selftest-shared-config");
  const judged = integrity.freshness(place.repo, place.state, "", place.commits);
  assert.equal(judged.verified, true);
  assert.match(named(judged, "fresh-host-configuration").detail, /shares the operator's/);
  fs.rmSync(place.root, { recursive: true, force: true });
});

test("a trial given its own host configuration is refused when that is not fresh", () => {
  const where = room();
  const variant = family("stubs").variants.risk;
  const place = workspace.materialize(variant, "selftest-own-config");
  const config = path.join(where, "config");
  fs.mkdirSync(config, { recursive: true });
  assert.equal(
    named(integrity.freshness(place.repo, place.state, config, place.commits), "fresh-host-configuration")
      .passed,
    true,
    "an empty directory of its own is fresh",
  );
  fs.writeFileSync(path.join(config, "CLAUDE.md"), "left over\n");
  assert.equal(
    named(integrity.freshness(place.repo, place.state, config, place.commits), "fresh-host-configuration")
      .passed,
    false,
    "a directory another trial left behind is not fresh",
  );
  fs.rmSync(where, { recursive: true, force: true });
  fs.rmSync(place.root, { recursive: true, force: true });
});
