import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { family } from "../src/catalogue.ts";
import * as integrity from "../src/integrity.ts";
import * as workspace from "../src/workspace.ts";
import { files } from "../src/trees.ts";
import * as paths from "../src/paths.ts";
import type { Check, Isolation } from "../src/record.ts";

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-integrity-"));
}

const KLIN = "/opt/klin/klin";

/** One trial, with a plane of its own. `clear` removes both halves. */
function laid(familyName: string, variantName: "risk" | "control", id: string, deliver = true) {
  const plane = path.join(room(), "plane");
  const place = workspace.materialize(
    family(familyName).variants[variantName],
    id,
    plane,
    KLIN,
    deliver,
  );
  const clear = (): void => {
    fs.rmSync(place.root, { recursive: true, force: true });
    fs.rmSync(plane, { recursive: true, force: true });
  };
  return { place, plane, clear };
}

function named(isolation: Isolation, name: string): Check {
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
  const { place, clear } = laid("stubs", "risk", "selftest-fresh");
  const judged = integrity.freshness(place.repo, place.state, "", place.commits);
  assert.equal(judged.verified, true, JSON.stringify(judged.checks));
  assert.equal(place.commits, 1);
  assert.ok(fs.existsSync(path.join(place.repo, "klin.json")));
  assert.ok(fs.existsSync(place.settings));
  assert.ok(!fs.existsSync(path.join(place.repo, ".claude")));
  clear();
});

test("the repository is the only thing in its own parent directory", () => {
  const { place, clear } = laid("stubs", "risk", "selftest-alone");
  assert.deepEqual(
    fs.readdirSync(place.root),
    ["repo"],
    "an ls of the subject's parent must reach nothing the harness owns",
  );
  for (const held of [place.hook, place.settings, place.state, place.hooks]) {
    assert.ok(
      !held.startsWith(place.root),
      held + " sits beside the subject, one ls away from it",
    );
  }
  clear();
});

test("a second trial over the same family starts from the same tree", () => {
  const one = laid("stubs", "risk", "selftest-one");
  const two = laid("stubs", "risk", "selftest-two");
  assert.equal(one.place.treeSha256, two.place.treeSha256);
  assert.notEqual(one.place.root, two.place.root);
  assert.notEqual(one.place.state, two.place.state);
  one.clear();
  two.clear();
});

test("the host settings point every event at the wrapper and live outside the workspace", () => {
  const { place, clear } = laid("stubs", "control", "selftest-settings");
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
  clear();
});

test("the arms differ in nothing a workspace can read", () => {
  const active = laid("lockfile", "risk", "selftest-active", true);
  const shadow = laid("lockfile", "risk", "selftest-shadow", false);
  assert.equal(active.place.treeSha256, shadow.place.treeSha256);
  assert.deepEqual(
    fs.readFileSync(active.place.hook),
    fs.readFileSync(shadow.place.hook),
    "the wrapper must be one file in both arms, byte for byte",
  );
  assert.notEqual(
    fs.readFileSync(active.place.settings, "utf8"),
    fs.readFileSync(shadow.place.settings, "utf8"),
    "the arm is in the settings, which sit in the plane and not beside the subject",
  );
  assert.equal(
    workspace.wiringSha256(active.place.settings, active.place.plane, active.place.root),
    workspace.wiringSha256(shadow.place.settings, shadow.place.plane, shadow.place.root),
    "the digest names the plane, the workspace and the arm, so two arms of one cell must match",
  );
  for (const one of [active, shadow]) {
    assert.deepEqual(fs.readdirSync(one.place.root), ["repo"]);
  }
  active.clear();
  shadow.clear();
});

test("the wiring digest attests the settings file's own bytes", () => {
  const { place, clear } = laid("lockfile", "risk", "selftest-wiring");
  const digestNow = (): string => workspace.wiringSha256(place.settings, place.plane, place.root);
  const before = digestNow();
  const edit = (from: string, to: string): void => {
    fs.writeFileSync(place.settings, fs.readFileSync(place.settings, "utf8").replace(from, to));
  };
  edit('"timeout": 900', '"timeout": 5');
  assert.notEqual(
    digestNow(),
    before,
    "a hand-edited Stop timeout must change the digest, or a paired cell could not catch it",
  );
  edit('"timeout": 5', '"timeout": 900');
  assert.equal(digestNow(), before);
  edit('"allowUnsandboxedCommands": false', '"allowUnsandboxedCommands": true');
  assert.notEqual(
    digestNow(),
    before,
    "a sandbox rule is a real difference between two arms, so it must change the digest",
  );
  clear();
});

/**
 * What normalizes away, and what may not.
 *
 * Two arms run in two workspaces and two planes, and both paths carry the trial id. Nothing else
 * about the file may drop out of the digest.
 */
test("the session's own paths normalize away and the rules do not", () => {
  const one = laid("lockfile", "control", "selftest-norm-one", true);
  const two = laid("lockfile", "control", "selftest-norm-two", true);
  assert.notEqual(one.place.root, two.place.root);
  assert.notEqual(one.place.plane, two.place.plane);
  assert.equal(
    workspace.wiringSha256(one.place.settings, one.place.plane, one.place.root),
    workspace.wiringSha256(two.place.settings, two.place.plane, two.place.root),
    "only the trial's own paths differ between these two",
  );
  const before = workspace.wiringSha256(one.place.settings, one.place.plane, one.place.root);
  fs.writeFileSync(
    one.place.settings,
    fs
      .readFileSync(one.place.settings, "utf8")
      .replace('"blockReadsOutsideWorkingDirectories": true', '"blockReadsOutsideWorkingDirectories": false'),
  );
  assert.notEqual(
    workspace.wiringSha256(one.place.settings, one.place.plane, one.place.root),
    before,
    "a tool permission is a real difference and may not normalize away",
  );
  one.clear();
  two.clear();
});

/**
 * The subject is confined by the operating system, not by the layout.
 *
 * `cwd` is no sandbox: the plane, the other trials' workspaces and klin's own repository are all
 * absolute paths a shell can name. These rules are what refuse them, and `allowUnsandboxedCommands`
 * is what stops the host from retrying a refused command outside the sandbox.
 */
test("the settings confine the subject to its own repository", () => {
  const { place, clear } = laid("lockfile", "risk", "selftest-sandbox", false);
  const settings = JSON.parse(fs.readFileSync(place.settings, "utf8")) as {
    sandbox: {
      enabled: boolean;
      allowUnsandboxedCommands: boolean;
      filesystem: { denyRead: string[]; allowRead: string[]; denyWrite: string[] };
    };
    permissions: { blockReadsOutsideWorkingDirectories: boolean };
  };
  assert.equal(settings.sandbox.enabled, true);
  assert.equal(settings.sandbox.allowUnsandboxedCommands, false);
  assert.equal(settings.permissions.blockReadsOutsideWorkingDirectories, true);
  for (const denied of [place.plane, paths.workRoot(), paths.REPO]) {
    for (const form of [denied, fs.realpathSync(denied)]) {
      assert.ok(settings.sandbox.filesystem.denyRead.includes(form), form + " is readable");
      assert.ok(settings.sandbox.filesystem.denyWrite.includes(form), form + " is writable");
    }
  }
  assert.deepEqual(
    settings.sandbox.filesystem.allowRead,
    [...new Set([place.repo, fs.realpathSync(place.repo)])],
    "the repository sits inside a denied root, so only it is re-opened, in both its forms",
  );
  clear();
});

test("a trial that shares the operator's host configuration says so and is not refused", () => {
  const { place, clear } = laid("stubs", "risk", "selftest-shared-config");
  const judged = integrity.freshness(place.repo, place.state, "", place.commits);
  assert.equal(judged.verified, true);
  assert.match(named(judged, "fresh-host-configuration").detail, /shares the operator's/);
  clear();
});

test("a trial given its own host configuration is refused when that is not fresh", () => {
  const where = room();
  const { place, clear } = laid("stubs", "risk", "selftest-own-config");
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
  clear();
});
