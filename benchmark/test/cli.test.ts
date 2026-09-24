import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import { preflight } from "../src/calibrate.ts";
import { positionals, wrongArguments } from "../src/cli.ts";

/** The command line refuses a mistake before it can spend a live session. */

function cli(...args: string[]) {
  return spawnSync("node", [path.join(paths.BENCHMARK, "src", "cli.ts"), ...args], {
    encoding: "utf8",
    cwd: paths.REPO,
    timeout: 60_000,
  });
}

test("a misspelled arm is refused rather than run as the other one", () => {
  const ran = cli("run", "inventory", "risk", "actve");
  assert.equal(ran.status, 2);
  assert.match(ran.stdout, /no arm named actve/);
});

test("a misspelled family or variant is refused", () => {
  assert.match(cli("run", "inventry", "risk", "active").stdout, /no family named inventry/);
  assert.match(cli("run", "inventory", "risky", "active").stdout, /no variant named risky/);
});

test("a whole command line passes the argument check", () => {
  assert.deepEqual(wrongArguments("inventory", "risk", "active"), []);
  assert.deepEqual(wrongArguments("public-api", "control", "shadow"), []);
});

test("every wrong part of a command line is named at once", () => {
  assert.deepEqual(wrongArguments("nope", "nope", "nope"), [
    "no family named nope",
    "no variant named nope",
    "no arm named nope",
  ]);
});

test("a missing klin binary stops a run before it is paid for", () => {
  const where = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-preflight-"));
  assert.match(preflight(path.join(where, "klin")), /no klin binary at/);
  fs.writeFileSync(path.join(where, "klin"), "#!/bin/sh\nexit 1\n");
  fs.chmodSync(path.join(where, "klin"), 0o755);
  assert.match(preflight(path.join(where, "klin")), /did not answer --version/);
  fs.rmSync(where, { recursive: true, force: true });
});

test("the list and usage commands answer without touching the network", () => {
  assert.equal(cli("list").status, 0);
  assert.match(cli("list").stdout, /36 calibration cells/);
  assert.match(cli("--help").stdout, /Shadow\/Active benchmark/);
  assert.equal(cli("nonsense").status, 2);
});

/**
 * A flag's value is not a family name.
 *
 * `probe [family] [--into DIR]` takes its family after the command, and the directory follows a
 * flag. Filtering on the leading dash alone kept the directory and read it as the family, so
 * `probe --into DIR` refused every directory as an unknown family.
 */
test("a flag's value is never read as a positional argument", () => {
  assert.deepEqual(positionals(["--into", "/tmp/set"]), []);
  assert.deepEqual(positionals(["inventory", "--into", "/tmp/set"]), ["inventory"]);
  assert.deepEqual(positionals(["--seed", "3", "--only", "stubs"]), []);
  assert.deepEqual(positionals([]), []);
});

test("an admission set starts only over candidates, and its report stays sealed", () => {
  const none = cli("calibrate", "--population", "admission");
  assert.equal(none.status, 2);
  assert.match(none.stdout, /the catalogue holds no candidate task/);
  assert.match(cli("calibrate", "--population", "admision").stdout, /no population named admision/);
  assert.match(cli("calibrate", "--population", "admission", "--only", "stubs").stdout, /no candidate task named stubs/);
  const where = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-admission-cli-"));
  fs.writeFileSync(path.join(where, "manifest.json"), JSON.stringify({ kind: "admission", population: "admission" }) + "\n");
  const report = cli("report", where);
  assert.equal(report.status, 2);
  assert.match(report.stdout, /signals stay sealed/);
  assert.equal(cli("scorecard", where).status, 2);
  fs.rmSync(where, { recursive: true, force: true });
});
