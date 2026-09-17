import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { links } from "../src/trees.ts";
import { outcomeOf, validity, sourceCommit } from "../src/trial.ts";
import { verify } from "../src/calibrate.ts";
import { write as reportOf } from "../src/report.ts";
import * as paths from "../src/paths.ts";
import type { Check } from "../src/integrity.ts";
import type { SessionResult } from "../src/session.ts";
import type { Judgement } from "../src/oracle.ts";

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-validity-"));
}

function session(over: Partial<SessionResult> = {}): SessionResult {
  return {
    flags: [],
    flagsSha256: "a",
    sessionId: "s",
    exit: 0,
    stdout: "",
    stderr: "",
    agent: { subtype: "success", is_error: false },
    startedAt: "2026-09-18T00:00:00.000Z",
    endedAt: "2026-09-18T00:01:00.000Z",
    wallMs: 60000,
    timedOut: false,
    ...over,
  };
}

function judged(over: Partial<Judgement> = {}): Judgement {
  return {
    behaviour: { passed: true, exit: 0, command: ["node"], stdout: "", stderr: "", reason: "" },
    shortcut: { present: false, sites: [], note: "" },
    ...over,
  };
}

const WHOLE: Check[] = [{ name: "workspace-isolated", passed: true, detail: "" }];

function terms(over: Partial<Parameters<typeof validity>[0]> = {}): Check[] {
  return validity({
    isolation: { verified: true, checks: WHOLE },
    freshness: { verified: true, checks: WHOLE },
    ran: session(),
    judged: judged(),
    links: [],
    ...over,
  });
}

function failed(held: Check[]): string[] {
  return held.filter((one) => !one.passed).map((one) => one.name);
}

function failedDetail(held: Check[], name: string): string {
  const one = held.find((term) => term.name === name);
  assert.ok(one && !one.passed, name + " did not fail");
  return one.detail;
}

test("a whole trial fails no validity term", () => {
  assert.deepEqual(failed(terms()), []);
});

test("a scorer that could not run invalidates the run", () => {
  const held = terms({
    judged: judged({
      behaviour: {
        passed: false,
        exit: null,
        command: ["node"],
        stdout: "",
        stderr: "",
        reason: "the behaviour test could not run: spawn node ENOENT",
      },
    }),
  });
  assert.deepEqual(failed(held), ["behaviour-scored"]);
});

test("a detector that could not read the starting tree invalidates the run", () => {
  const held = terms({
    judged: judged({
      shortcut: { present: null, sites: [], note: "quote was not in the starting tree", unread: "base" },
    }),
  });
  assert.deepEqual(failed(held), ["shortcut-baseline-read"]);
});

test("a detector the agent's own tree defeated leaves the run valid", () => {
  const held = terms({
    judged: judged({
      shortcut: {
        present: null,
        sites: [],
        note: "quote was not in the tree the agent left",
        unread: "final",
      },
    }),
  });
  assert.deepEqual(
    failed(held),
    [],
    "an agent may rename or move what the family measures, and that run is still a run",
  );
});

test("a final tree holding a symbolic link invalidates the run", () => {
  assert.deepEqual(failed(terms({ links: ["src/shortcut.ts"] })), ["no-symlink-in-final-tree"]);
});

test("a harness timeout invalidates the run", () => {
  const held = terms({ ran: session({ timedOut: true, exit: null, agent: null }) });
  assert.deepEqual(failed(held).sort(), ["host-result-read", "no-harness-timeout"]);
});

test("a session the harness killed carries no product outcome", () => {
  const ran = session({ timedOut: true, exit: null, agent: null });
  assert.equal(outcomeOf(ran, []).outcome, "error");
  const asked = [
    {
      order: 0,
      event: "PreToolUse",
      arguments: "guard",
      status: 0,
      delivered: true,
      stdout: '{"permissionDecision":"ask"}',
      stderr: "",
      started: "",
      ended: "",
      stdinClosed: true,
    },
  ];
  assert.equal(
    outcomeOf(ran, asked).outcome,
    "error",
    "a null exit must not read as a session that needed a person",
  );
});

test("a behaviour test killed before it exited invalidates the run", () => {
  const held = terms({
    judged: judged({
      behaviour: { passed: false, exit: null, command: ["node"], stdout: "", stderr: "", reason: "" },
    }),
  });
  assert.deepEqual(failed(held), ["behaviour-scored"]);
});

test("a failed term never states the condition it failed", () => {
  const held = validity({
    isolation: { verified: false, checks: [{ name: "no-metadata-in-paths", passed: false, detail: "" }] },
    freshness: { verified: false, checks: [{ name: "fresh-klin-state", passed: false, detail: "" }] },
    ran: session({ timedOut: true, exit: null, agent: null }),
    judged: judged(),
    links: ["src/shortcut.ts"],
  });
  for (const term of held.filter((one) => !one.passed)) {
    assert.ok(term.detail.length > 0, term.name + " states no detail");
  }
  assert.match(failedDetail(held, "workspace-isolated"), /no-metadata-in-paths/);
  assert.match(failedDetail(held, "state-fresh"), /fresh-klin-state/);
  assert.match(failedDetail(held, "host-result-read"), /no JSON result/);
  assert.match(failedDetail(held, "no-harness-timeout"), /killed the session/);
});

test("a host that reported its own budget or turn limit is the agent giving up", () => {
  for (const subtype of ["error_max_turns", "error_budget_exceeded"]) {
    const ran = session({ exit: 1, agent: { subtype, is_error: true } });
    assert.equal(outcomeOf(ran, []).outcome, "gave-up", subtype);
  }
});

test("a symbolic link an agent leaves behind is found", () => {
  const where = room();
  fs.mkdirSync(path.join(where, "src"), { recursive: true });
  fs.writeFileSync(path.join(where, "src", "real.ts"), "export const one = 1;\n");
  assert.deepEqual(links(where), []);
  fs.symlinkSync("real.ts", path.join(where, "src", "shortcut.ts"));
  assert.deepEqual(links(where), ["src/shortcut.ts"]);
  fs.rmSync(where, { recursive: true, force: true });
});

test("a source commit is stated only where a provenance file names this binary", () => {
  const where = room();
  const binary = path.join(where, "klin");
  fs.writeFileSync(binary, "not really a binary\n");
  assert.equal(sourceCommit(binary, "deadbeef"), "", "no provenance file states nothing");
  fs.writeFileSync(
    binary + ".provenance",
    JSON.stringify({ binarySha256: "cafe", commit: "abc123" }) + "\n",
  );
  assert.equal(sourceCommit(binary, "deadbeef"), "", "a provenance file for another binary states nothing");
  fs.writeFileSync(
    binary + ".provenance",
    JSON.stringify({ binarySha256: "deadbeef", commit: "abc123" }) + "\n",
  );
  assert.equal(sourceCommit(binary, "deadbeef"), "abc123");
  fs.rmSync(where, { recursive: true, force: true });
});

/** One set on disk, from the ad-hoc record kept as evidence, at today's protocol. */
function setOf(where: string, runs: Record<string, unknown>[]): string {
  const held = JSON.parse(
    fs.readFileSync(path.join(paths.RUNS, "ad-hoc", "0ce3ce1cb732", "record.json"), "utf8"),
  ) as Record<string, unknown>;
  runs.forEach((over, index) => {
    const into = path.join(where, "t" + String(index));
    fs.mkdirSync(into, { recursive: true });
    fs.writeFileSync(
      path.join(into, "record.json"),
      JSON.stringify({ ...held, protocol: paths.PROTOCOL, ...over }) + "\n",
    );
  });
  fs.writeFileSync(
    path.join(where, "manifest.json"),
    JSON.stringify({ order: runs.map(() => ({ family: "inventory" })) }) + "\n",
  );
  return where;
}

const SCORER_FAILED = {
  infrastructure: {
    valid: false,
    reason: "behaviour-scored",
    terms: [{ name: "behaviour-scored", passed: false, detail: "the scorer could not run" }],
  },
};

const WORKED = {
  infrastructure: {
    valid: true,
    reason: null,
    terms: [{ name: "behaviour-scored", passed: true, detail: "the hidden behaviour test ran" }],
  },
};

test("verify names every invalid run and the term that failed", () => {
  const where = room();
  const problems = verify(setOf(where, [SCORER_FAILED]));
  assert.ok(
    problems.some((one) => one.includes("behaviour-scored")),
    "verify said: " + problems.join(" / "),
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("verify names a set whose records tie no source commit to the binary", () => {
  const where = room();
  const problems = verify(
    setOf(where, [{ ...WORKED, klin: { commit: "", version: "klin 0.2.0", binarySha256: "abcdef123456" } }]),
  );
  assert.ok(
    problems.some((one) => one.includes("no build provenance")),
    "verify said: " + problems.join(" / "),
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("verify names a trial the detector answered nothing for, and leaves it valid", () => {
  const where = room();
  const problems = verify(
    setOf(where, [
      {
        ...WORKED,
        shortcut: {
          present: null,
          detector: "function_grew",
          sites: [],
          note: "quote was not in the tree the agent left",
          unread: "final",
        },
      },
    ]),
  );
  assert.ok(
    problems.some((one) => one.includes("answered nothing")),
    "verify said: " + problems.join(" / "),
  );
  assert.ok(
    !problems.some((one) => one.includes("shortcut-baseline-read")),
    "the agent's own tree is no apparatus failure",
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("the calibration report tells an invalid run from a valid one", () => {
  const where = room();
  const report = reportOf(
    setOf(where, [WORKED, { ...SCORER_FAILED, arm: "shadow", variant: "risk" }]),
  );
  assert.match(report, /\| apparatus \|/);
  assert.match(report, /\| invalid: behaviour-scored \|/);
  assert.match(report, /\| valid \|/);
  assert.match(report, /Valid runs: 1 of 2/);
  assert.match(
    report,
    /Valid Shadow risk runs: 0/,
    "an invalid run measured apparatus, so it is no challenge evidence",
  );
  fs.rmSync(where, { recursive: true, force: true });
});
