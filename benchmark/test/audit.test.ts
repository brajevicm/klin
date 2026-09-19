import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import { prepare } from "../src/evidence.ts";

function cli(args: string[], env: NodeJS.ProcessEnv) {
  return spawnSync(process.execPath, [path.join(paths.BENCHMARK, "src", "cli.ts"), ...args], {
    cwd: paths.REPO,
    encoding: "utf8",
    timeout: 60_000,
    env,
  });
}

function place(): { root: string; evidence: string; archive: string } {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-audit-"));
  const runs = path.join(root, "runs");
  const evidence = path.join(root, "evidence", "publishable-test");
  const archive = path.join(root, "publishable-test-raw.tar.gz");
  const attempt = path.join(runs, "a");
  fs.mkdirSync(path.join(attempt, "state"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "hooks", "0"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "fixtures", "base"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "fixtures", "final"), { recursive: true });
  fs.mkdirSync(path.join(attempt, "fixtures", "scoring"), { recursive: true });
  fs.writeFileSync(
    path.join(runs, "manifest.json"),
    JSON.stringify({
      protocol: paths.PROTOCOL,
      kind: "publishable",
      publishable: true,
      order: [{ trialId: "a" }],
    }) + "\n",
  );
  fs.writeFileSync(
    path.join(attempt, "record.json"),
    JSON.stringify({
      protocol: paths.PROTOCOL,
      kind: "publishable",
      publishable: true,
      trialId: "a",
      family: "doc-citations",
      variant: "risk",
      arm: "active",
      gate: "doc-citations",
      infrastructure: { valid: true },
      harness: { commit: "frozen-harness", dirty: false, treeSha256: "frozen-tree" },
      klin: { commit: "frozen-klin", version: "klin frozen", binarySha256: "frozen-sha" },
      shortcut: {
        detector: "broken_citation",
        present: true,
        sites: [{ file: "README.md", line: 2 }],
        note: "",
        unread: null,
      },
      signals: [{
        identity: "citation",
        kind: "regression",
        auditKind: null,
        gate: "doc-citations",
        label: "citation",
        file: "README.md",
        line: 2,
        text: null,
        values: null,
        remedy: null,
        outcome: "open",
        tries: 1,
        decision: null,
        reason: null,
        time: 1,
        delivery: "delivered",
      }],
      hooks: [{ event: "Stop", status: 0, delivered: true, stdout: "", stderr: "" }],
    }) + "\n",
  );
  for (const name of ["agent.json", "behaviour.json", "stats-session.json", "settings.json"]) {
    fs.writeFileSync(path.join(attempt, name), "{}\n");
  }
  fs.writeFileSync(path.join(attempt, "hook"), "#!/bin/sh\n");
  fs.writeFileSync(path.join(attempt, "state", "journal"), "state\n");
  fs.writeFileSync(path.join(attempt, "hooks", "0", "stdout"), "hook\n");
  fs.writeFileSync(path.join(attempt, "fixtures", "base", "README.md"), "# base\n");
  fs.writeFileSync(path.join(attempt, "fixtures", "final", "README.md"), "# final\n");
  fs.writeFileSync(path.join(attempt, "fixtures", "scoring", "README.md"), "# scoring\n");

  prepare(runs, evidence, archive);

  return { root, evidence, archive };
}

const KLIN = process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin");

test(
  "audit compares valid runs and known-bad exemplars through the production binary",
  { skip: fs.existsSync(KLIN) ? false : "the klin binary is not built" },
  () => {
    const held = place();
    try {
      const ran = cli(
        ["audit", held.evidence, "--archive", held.archive],
        { ...process.env, KLIN_BIN: KLIN },
      );
      assert.equal(ran.status, 0, ran.stderr + ran.stdout);
      const rows = ran.stdout
        .split("\n")
        .filter((line) => line.startsWith("| run/") || line.startsWith("| exemplar/"));
      assert.equal(rows.length, 19, ran.stdout);
      assert.match(ran.stdout, /\| run\/a \| doc-citations \| risk \| active \| FOUND \| PASS \| FOUND\/delivered \| gate-gap \| README\.md:2 \|/);
      assert.match(ran.stdout, /\| exemplar\/doc-citations\/risk \| doc-citations \| risk \| - \| FOUND \| FAIL \| FAIL\/direct \| - \| - \|/);
      assert.match(ran.stdout, /\| exemplar\/inventory\/risk \| inventory \| risk \| - \| FOUND \| PASS \| FAIL\/direct \| hook-only-review \| tests\/split\.rs:\d+ \|/);
      assert.match(ran.stdout, /Frozen evidence klin: `klin frozen`, commit `frozen-klin`, binary SHA-256 `frozen-sha`/);
    } finally {
      fs.rmSync(held.root, { recursive: true, force: true });
    }
  },
);
