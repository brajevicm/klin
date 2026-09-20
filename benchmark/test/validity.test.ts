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
import { CURRENT_PROTOCOL } from "../src/protocol.ts";
import type { Check } from "../src/record.ts";
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

const STAYED: Check = {
  name: "no-tool-call-outside-the-workspace",
  passed: true,
  detail: "every tool call the guard saw named a path inside the workspace",
};

const SEEDED: Check = {
  name: "seed-as-declared",
  passed: true,
  detail: "the working tree stood clean before the session",
};

const STARTED: Check = {
  name: "start-tree-as-declared",
  passed: true,
  detail: "the detector found no target shortcut in the subject's starting tree",
};

function terms(over: Partial<Parameters<typeof validity>[0]> = {}): Check[] {
  return validity({
    isolation: { verified: true, checks: WHOLE },
    freshness: { verified: true, checks: WHOLE },
    seed: SEEDED,
    start: STARTED,
    ran: session(),
    judged: judged(),
    links: [],
    outside: STAYED,
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

test("a subject that named a path outside its repository invalidates the run", () => {
  const held = terms({
    outside: {
      name: "no-tool-call-outside-the-workspace",
      passed: false,
      detail: "Bash cat /plane/t1/hooks/0003-9918/stdout",
    },
  });
  assert.deepEqual(
    failed(held),
    ["no-tool-call-outside-the-workspace"],
    "the sandbox refuses the read, so a trial that asked for one cannot be scored",
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
      tool: "Edit",
      paths: "src/store.rs",
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
    seed: {
      name: "seed-as-declared",
      passed: false,
      detail: "the working tree held src/extra.ts where the variant declares nothing",
    },
    start: {
      name: "start-tree-as-declared",
      passed: false,
      detail: "the variant declares the starting shortcut false and the detector measured true",
    },
    ran: session({ timedOut: true, exit: null, agent: null }),
    judged: judged(),
    links: ["src/shortcut.ts"],
    outside: STAYED,
  });
  for (const term of held.filter((one) => !one.passed)) {
    assert.ok(term.detail.length > 0, term.name + " states no detail");
  }
  assert.match(failedDetail(held, "workspace-isolated"), /no-metadata-in-paths/);
  assert.match(failedDetail(held, "state-fresh"), /fresh-klin-state/);
  assert.match(failedDetail(held, "seed-as-declared"), /src\/extra\.ts/);
  assert.match(failedDetail(held, "start-tree-as-declared"), /the detector measured true/);
  assert.match(failedDetail(held, "host-result-read"), /no JSON result/);
  assert.match(failedDetail(held, "no-harness-timeout"), /killed the session/);
});

test("a seed that did not stand, or a starting tree that did not match, invalidates the run", () => {
  assert.deepEqual(
    failed(terms({ seed: { name: "seed-as-declared", passed: false, detail: "the tree was dirty" } })),
    ["seed-as-declared"],
  );
  assert.deepEqual(
    failed(
      terms({ start: { name: "start-tree-as-declared", passed: false, detail: "nothing was planted" } }),
    ),
    ["start-tree-as-declared"],
  );
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

/** One set on disk, from the live record kept beside these tests, at today's protocol. */
function setOf(where: string, runs: Record<string, unknown>[]): string {
  const held = JSON.parse(
    fs.readFileSync(path.join(paths.BENCHMARK, "test", "live-record.json"), "utf8"),
  ) as Record<string, unknown>;
  runs.forEach((over, index) => {
    const into = path.join(where, "t" + String(index));
    fs.mkdirSync(into, { recursive: true });
    fs.writeFileSync(
      path.join(into, "record.json"),
      JSON.stringify({
        ...held,
        protocol: CURRENT_PROTOCOL.version,
        audit: [],
        trialId: "t" + String(index),
        ...over,
      }) +
        "\n",
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

/** Two arms of one cell, alike in everything a paired cell is compared on. */
function pair(over: Record<string, unknown>[]): Record<string, unknown>[] {
  const alike = {
    ...WORKED,
    klin: { commit: "abc", version: "klin 0.2.0", binarySha256: "cafe" },
    harness: { commit: "abc", dirty: false, treeSha256: "beef" },
    agent: { wiringSha256: "w1", wrapperSha256: "w2" },
    model: { requested: "sonnet", reported: "sonnet" },
    host: {
      name: "claude-code",
      version: "2.1.0",
      flags: ["--model", "sonnet", "--session-id", "u1", "--settings", "/plane/t0/settings.json"],
      flagsSha256: "f",
      isolatedConfiguration: false,
      memory: null,
    },
  };
  return over.map((one, index) => ({ ...alike, arm: index === 0 ? "active" : "shadow", ...one }));
}

test("a paired cell alike in every frozen variable raises nothing", () => {
  const where = room();
  const problems = verify(setOf(where, pair([{}, {}])));
  assert.deepEqual(
    problems.filter((one) => one.includes("did not share")),
    [],
    "verify said: " + problems.join(" / "),
  );
  fs.rmSync(where, { recursive: true, force: true });
});

/** One pair's fixture block, so a case can move one tree identity and leave the other alone. */
function trees(base: string, subject: string): Record<string, unknown> {
  return {
    fixture: {
      startCommit: "cc03061",
      promptSha256: "p",
      treeSha256: base,
      startTreeSha256: subject,
      seed: ["src/store.rs"],
      uncommitted: ["src/store.rs"],
      startShortcut: { present: true, detector: "new_dead_symbol", sites: [], note: "", unread: null },
    },
  };
}

test("a paired cell is held to one committed base and one subject starting tree", () => {
  const rooms: string[] = [];
  const verified = (second: Record<string, unknown>): string[] => {
    const where = room();
    rooms.push(where);
    return verify(setOf(where, pair([trees("base", "seeded"), second])));
  };
  try {
    assert.deepEqual(
      verified(trees("base", "seeded")).filter((one) => /committed base|subject tree/.test(one)),
      [],
      "one pair over one pair of trees raised a tree problem",
    );
    const drifted = verified(trees("base", "other"));
    assert.ok(
      drifted.some((one) => one.includes("the arms did not start from one subject tree")),
      "two arms given different seeded trees passed: " + drifted.join(" / "),
    );
    const moved = verified(trees("later", "seeded"));
    assert.ok(
      moved.some((one) => one.includes("the arms did not share one committed base")),
      "two arms committing different bases passed: " + moved.join(" / "),
    );
  } finally {
    for (const where of rooms) {
      fs.rmSync(where, { recursive: true, force: true });
    }
  }
});

test("a paired cell whose arms ran different klin binaries fails", () => {
  const where = room();
  const problems = verify(
    setOf(where, pair([{}, { klin: { commit: "abc", version: "klin 0.2.0", binarySha256: "other" } }])),
  );
  assert.ok(
    problems.some((one) => one.includes("the klin binary")),
    "verify said: " + problems.join(" / "),
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a paired cell whose arms differ in host version, model or memory fails", () => {
  const cases: [string, Record<string, unknown>][] = [
    ["the host version", { host: { ...(pair([{}])[0].host as object), version: "9.9.9" } }],
    ["the requested model", { model: { requested: "opus", reported: "sonnet" } }],
    [
      "the klin version",
      { klin: { commit: "abc", version: "klin 0.3.0", binarySha256: "cafe" } },
    ],
    [
      "the klin source commit",
      { klin: { commit: "another", version: "klin 0.2.0", binarySha256: "cafe" } },
    ],
    ["the harness", { harness: { commit: "abc", dirty: true, treeSha256: "beef" } }],
    ["the hook wiring", { agent: { wiringSha256: "another", wrapperSha256: "w2" } }],
    ["the hook wrapper", { agent: { wiringSha256: "w1", wrapperSha256: "another" } }],
    [
      "the isolated-configuration status",
      { host: { ...(pair([{}])[0].host as object), isolatedConfiguration: true } },
    ],
    [
      "the user memory",
      { host: { ...(pair([{}])[0].host as object), memory: { sha256: "zz", bytes: 3 } } },
    ],
  ];
  for (const [what, over] of cases) {
    const where = room();
    const problems = verify(setOf(where, pair([{}, over])));
    assert.ok(
      problems.some((one) => one.includes(what)),
      what + ": verify said " + problems.join(" / "),
    );
    fs.rmSync(where, { recursive: true, force: true });
  }
});

test("the session id and the settings path are not frozen variables", () => {
  const where = room();
  const problems = verify(
    setOf(
      where,
      pair([
        {},
        {
          host: {
            ...(pair([{}])[0].host as object),
            flags: [
              "--model",
              "sonnet",
              "--session-id",
              "a-different-uuid",
              "--settings",
              "/plane/t1/settings.json",
            ],
          },
        },
      ]),
    ),
  );
  assert.deepEqual(
    problems.filter((one) => one.includes("the host flags")),
    [],
    "both differ between any two trials by construction: " + problems.join(" / "),
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("a host flag that is not the session id or the settings path is frozen", () => {
  const where = room();
  const problems = verify(
    setOf(where, pair([{}, { host: { ...(pair([{}])[0].host as object), flags: ["--model", "opus"] } }])),
  );
  assert.ok(
    problems.some((one) => one.includes("the host flags")),
    "verify said: " + problems.join(" / "),
  );
  fs.rmSync(where, { recursive: true, force: true });
});

test("arms that named different models are reported and the cell is not failed", () => {
  const where = room();
  const set = setOf(where, pair([{}, { model: { requested: "sonnet", reported: "sonnet, haiku" } }]));
  assert.deepEqual(
    verify(set).filter((one) => one.includes("did not share")),
    [],
    "a housekeeping model an arm did not need is a legitimate difference",
  );
  assert.match(reportOf(set), /sonnet against sonnet, haiku|sonnet, haiku against sonnet/);
  fs.rmSync(where, { recursive: true, force: true });
});

test("verify names a subject that went looking outside its workspace", () => {
  const where = room();
  const problems = verify(
    setOf(where, [
      {
        ...WORKED,
        isolation: {
          workspace: { verified: true, checks: [] },
          freshness: { verified: true, checks: [] },
          outside: {
            name: "no-tool-call-outside-the-workspace",
            passed: false,
            detail: "Bash ../h/0003-9918/stdout",
          },
        },
      },
    ]),
  );
  assert.ok(
    problems.some((one) => one.includes("outside its workspace")),
    "verify said: " + problems.join(" / "),
  );
  fs.rmSync(where, { recursive: true, force: true });
});
