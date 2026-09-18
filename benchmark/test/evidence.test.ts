import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";

function command(...args: string[]) {
  return spawnSync(process.execPath, [path.join(paths.BENCHMARK, "src", "cli.ts"), ...args], {
    cwd: paths.REPO,
    encoding: "utf8",
    timeout: 60_000,
  });
}

function fixture(): { root: string; runs: string; evidence: string; archive: string } {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-evidence-"));
  const runs = path.join(root, "runs");
  const evidence = path.join(root, "evidence", "calibration-2026-09-17");
  const archive = path.join(root, "calibration-2026-09-17-raw.tar.gz");
  fs.mkdirSync(path.join(runs, "attempt-a", "state"), { recursive: true });
  fs.mkdirSync(path.join(runs, "attempt-a", "hooks", "0"), { recursive: true });
  fs.mkdirSync(path.join(runs, "attempt-a", "fixtures", "base"), { recursive: true });
  fs.mkdirSync(path.join(runs, "attempt-a", "fixtures", "final"), { recursive: true });
  fs.mkdirSync(path.join(runs, "attempt-a", "fixtures", "scoring"), { recursive: true });
  fs.writeFileSync(
    path.join(runs, "manifest.json"),
    JSON.stringify(
      {
        protocol: paths.PROTOCOL,
        kind: "calibration",
        publishable: false,
        selectedFamilies: ["inventory"],
        order: [{ family: "inventory", variant: "risk", arm: "active", order: 0, trialId: "a" }],
      },
      null,
      2,
    ) + "\n",
  );
  fs.writeFileSync(
    path.join(runs, "attempt-a", "record.json"),
    JSON.stringify({
      protocol: paths.PROTOCOL,
      kind: "calibration",
      publishable: false,
      trialId: "a",
      infrastructure: { valid: true },
    }) + "\n",
  );
  for (const name of ["agent.json", "behaviour.json", "stats-session.json", "settings.json"]) {
    fs.writeFileSync(path.join(runs, "attempt-a", name), name + "\n");
  }
  fs.writeFileSync(path.join(runs, "attempt-a", "hook"), "#!/bin/sh\n");
  fs.writeFileSync(path.join(runs, "attempt-a", "state", "journal"), "state\n");
  fs.writeFileSync(path.join(runs, "attempt-a", "hooks", "0", "stdout"), "hook\n");
  fs.writeFileSync(path.join(runs, "attempt-a", "fixtures", "base", "README.md"), "base\n");
  fs.writeFileSync(path.join(runs, "attempt-a", "fixtures", "final", "README.md"), "final\n");
  fs.writeFileSync(path.join(runs, "attempt-a", "fixtures", "scoring", "README.md"), "scoring\n");
  fs.writeFileSync(path.join(runs, "raw-only.txt"), "raw\n");
  return { root, runs, evidence, archive };
}

function prepare(place: ReturnType<typeof fixture>): void {
  const result = command(
    "evidence-prepare",
    place.runs,
    "--into",
    place.evidence,
    "--archive",
    place.archive,
  );
  assert.equal(result.status, 0, result.stderr + result.stdout);
}

test("evidence-prepare keeps the slim files in Git and the forensic files in the archive", () => {
  const place = fixture();
  try {
    prepare(place);
    assert.ok(fs.existsSync(path.join(place.evidence, "manifest.json")));
    assert.ok(fs.existsSync(path.join(place.evidence, "files.sha256")));
    assert.ok(fs.existsSync(path.join(place.evidence, "evidence.json")));
    for (const name of ["record.json", "agent.json", "behaviour.json", "stats-session.json", "settings.json", "hook"]) {
      assert.ok(fs.existsSync(path.join(place.evidence, "attempts", "attempt-a", name)), name);
    }
    for (const name of ["state", "hooks", "fixtures"]) {
      assert.equal(fs.existsSync(path.join(place.evidence, "attempts", "attempt-a", name)), false, name);
    }
    const listed = spawnSync("tar", ["-tzf", place.archive], { encoding: "utf8" });
    assert.equal(listed.status, 0, listed.stderr);
    for (const name of [
      "attempt-a/state/journal",
      "attempt-a/hooks/0/stdout",
      "attempt-a/fixtures/base/README.md",
      "attempt-a/fixtures/final/README.md",
      "attempt-a/fixtures/scoring/README.md",
      "raw-only.txt",
    ]) {
      assert.match(listed.stdout, new RegExp("(?:^|/)" + name.replaceAll("/", "\\/") + "\\n"));
    }
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-verify binds slim bytes to the raw archive", () => {
  const place = fixture();
  try {
    prepare(place);
    const intact = command("evidence-verify", place.evidence, "--archive", place.archive);
    assert.equal(intact.status, 0, intact.stdout + intact.stderr);

    fs.appendFileSync(path.join(place.evidence, "attempts", "attempt-a", "record.json"), "changed\n");
    const rejected = command("evidence-verify", place.evidence, "--archive", place.archive);
    assert.equal(rejected.status, 1, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout + rejected.stderr, /slim attempt-a\/record\.json differs from the raw archive/);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-verify checks every slim artifact against the raw archive", () => {
  for (const name of ["agent.json", "behaviour.json", "stats-session.json", "settings.json", "hook"]) {
    const place = fixture();
    try {
      prepare(place);
      fs.appendFileSync(path.join(place.evidence, "attempts", "attempt-a", name), "changed\n");
      const rejected = command("evidence-verify", place.evidence, "--archive", place.archive);
      assert.equal(rejected.status, 1, name + ": " + rejected.stdout + rejected.stderr);
      assert.match(rejected.stdout, new RegExp("slim attempt-a/" + name + " differs from the raw archive"));
    } finally {
      fs.rmSync(place.root, { recursive: true, force: true });
    }
  }
});

test("evidence-verify rejects an unlisted slim artifact", () => {
  const place = fixture();
  try {
    prepare(place);
    fs.writeFileSync(path.join(place.evidence, "attempts", "attempt-a", "extra.txt"), "extra\n");
    const rejected = command("evidence-verify", place.evidence, "--archive", place.archive);
    assert.equal(rejected.status, 1, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /slim evidence has an unlisted file attempt-a\/extra\.txt/);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-verify rejects an archive that omits a raw file", () => {
  const place = fixture();
  try {
    prepare(place);
    const altered = path.join(place.root, "altered-runs");
    fs.cpSync(place.runs, altered, { recursive: true });
    fs.rmSync(path.join(altered, "raw-only.txt"));
    const rebuilt = spawnSync("tar", ["-czf", place.archive, "-C", altered, "."], { encoding: "utf8" });
    assert.equal(rebuilt.status, 0, rebuilt.stderr);
    const descriptor = JSON.parse(fs.readFileSync(path.join(place.evidence, "evidence.json"), "utf8")) as Record<string, unknown>;
    const archiveBytes = fs.readFileSync(place.archive);
    descriptor.archiveSha256 = createHash("sha256").update(archiveBytes).digest("hex");
    descriptor.archiveBytes = archiveBytes.length;
    fs.writeFileSync(path.join(place.evidence, "evidence.json"), JSON.stringify(descriptor) + "\n");

    const rejected = command("evidence-verify", place.evidence, "--archive", place.archive);
    assert.equal(rejected.status, 1, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /raw evidence is missing raw-only\.txt/);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-verify rejects changed archive bytes", () => {
  const place = fixture();
  try {
    prepare(place);
    fs.appendFileSync(place.archive, "changed\n");
    const rejected = command("evidence-verify", place.evidence, "--archive", place.archive);
    assert.equal(rejected.status, 1, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /raw archive has the wrong SHA-256/);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-verify derives counts from the attempts rather than trusting metadata", () => {
  const place = fixture();
  try {
    prepare(place);
    const descriptor = JSON.parse(fs.readFileSync(path.join(place.evidence, "evidence.json"), "utf8")) as Record<string, unknown>;
    descriptor.attempts = 99;
    fs.writeFileSync(path.join(place.evidence, "evidence.json"), JSON.stringify(descriptor) + "\n");
    const rejected = command("evidence-verify", place.evidence, "--archive", place.archive);
    assert.equal(rejected.status, 1, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /evidence\.json attempts is 99, source has 1/);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-prepare refuses calibration evidence marked publishable", () => {
  const place = fixture();
  try {
    const manifest = JSON.parse(fs.readFileSync(path.join(place.runs, "manifest.json"), "utf8")) as Record<string, unknown>;
    manifest.publishable = true;
    fs.writeFileSync(path.join(place.runs, "manifest.json"), JSON.stringify(manifest) + "\n");
    const rejected = command(
      "evidence-prepare",
      place.runs,
      "--into",
      place.evidence,
      "--archive",
      place.archive,
    );
    assert.equal(rejected.status, 2, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /calibration evidence must state publishable false/);
    assert.equal(fs.existsSync(path.join(place.evidence, "evidence.json")), false);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-verify refuses a descriptor that changes calibration into publishable evidence", () => {
  const place = fixture();
  try {
    prepare(place);
    const descriptor = JSON.parse(fs.readFileSync(path.join(place.evidence, "evidence.json"), "utf8")) as Record<string, unknown>;
    descriptor.kind = "publishable";
    fs.writeFileSync(path.join(place.evidence, "evidence.json"), JSON.stringify(descriptor) + "\n");
    const rejected = command("evidence-verify", place.evidence);
    assert.equal(rejected.status, 1, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /different kinds|confuses calibration and publishable/);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-prepare keeps invalid attempts and replacement metadata unchanged", () => {
  const place = fixture();
  try {
    const source = path.join(place.runs, "attempt-a");
    const replacement = path.join(place.runs, "attempt-replacement");
    fs.cpSync(source, replacement, { recursive: true });
    fs.writeFileSync(
      path.join(replacement, "record.json"),
      JSON.stringify({
        protocol: paths.PROTOCOL,
        kind: "calibration",
        publishable: false,
        trialId: "replacement",
        replacementFor: "original",
        infrastructure: { valid: false, reason: "host failed" },
      }) + "\n",
    );
    prepare(place);
    assert.equal(JSON.parse(fs.readFileSync(path.join(place.evidence, "attempts", "attempt-replacement", "record.json"), "utf8")).replacementFor, "original");
    const descriptor = JSON.parse(fs.readFileSync(path.join(place.evidence, "evidence.json"), "utf8")) as Record<string, unknown>;
    assert.equal(descriptor.attempts, 2);
    assert.equal(descriptor.validRuns, 1);
    const listed = spawnSync("tar", ["-tzf", place.archive], { encoding: "utf8" });
    assert.match(listed.stdout, /attempt-replacement\/state\/journal/);
    assert.equal(command("evidence-verify", place.evidence, "--archive", place.archive).status, 0);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-prepare refuses a schedule whose trial has no record", () => {
  const place = fixture();
  try {
    const manifest = JSON.parse(fs.readFileSync(path.join(place.runs, "manifest.json"), "utf8")) as {
      order: Record<string, unknown>[];
    };
    manifest.order.push({ trialId: "missing" });
    fs.writeFileSync(path.join(place.runs, "manifest.json"), JSON.stringify(manifest) + "\n");
    const rejected = command(
      "evidence-prepare",
      place.runs,
      "--into",
      place.evidence,
      "--archive",
      place.archive,
    );
    assert.equal(rejected.status, 2, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /scheduled trial missing left no record/);
    assert.equal(fs.existsSync(path.join(place.evidence, "evidence.json")), false);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-prepare refuses an attempt with missing forensic directories", () => {
  const place = fixture();
  try {
    fs.rmSync(path.join(place.runs, "attempt-a", "state"), { recursive: true });
    const rejected = command(
      "evidence-prepare",
      place.runs,
      "--into",
      place.evidence,
      "--archive",
      place.archive,
    );
    assert.equal(rejected.status, 2, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /attempt-a is missing state/);
    assert.equal(fs.existsSync(path.join(place.evidence, "evidence.json")), false);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});

test("evidence-prepare refuses an unrecorded attempt directory", () => {
  const place = fixture();
  try {
    fs.mkdirSync(path.join(place.runs, "attempt-abandoned"));
    const rejected = command(
      "evidence-prepare",
      place.runs,
      "--into",
      place.evidence,
      "--archive",
      place.archive,
    );
    assert.equal(rejected.status, 2, rejected.stdout + rejected.stderr);
    assert.match(rejected.stdout, /attempt-abandoned has no record\.json/);
    assert.equal(fs.existsSync(path.join(place.evidence, "evidence.json")), false);
  } finally {
    fs.rmSync(place.root, { recursive: true, force: true });
  }
});
