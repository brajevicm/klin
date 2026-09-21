import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { hookEvidence } from "../src/session.ts";
import { wrapper } from "../src/workspace.ts";

/**
 * The hook wrapper.
 *
 * A stub stands in for the klin binary. It reads its whole input before it writes anything, so a
 * wrapper that left the pipe open would never see the stub's answer and this suite would hang
 * rather than pass.
 */

const PAYLOAD = JSON.stringify({ hook_event_name: "Stop", session_id: "s1" });

function room(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-hook-"));
}

/** One trial's wrapper on disk, executable, with the three values baked in. */
function laid(into: string, plane: string, klin: string, deliver: boolean): string {
  const hook = path.join(into, "hook");
  fs.writeFileSync(hook, wrapper(plane, klin, deliver));
  fs.chmodSync(hook, 0o755);
  return hook;
}

function stub(into: string, exit: number): string {
  const file = path.join(into, "stub");
  fs.writeFileSync(
    file,
    ["#!/bin/sh", 'payload=$(cat)', 'printf "saw:%s" "$payload"', 'printf "why:%s" "$*" >&2', "exit " + String(exit)].join(
      "\n",
    ) + "\n",
  );
  fs.chmodSync(file, 0o755);
  return file;
}

function reportStub(into: string): string {
  const file = path.join(into, "report-stub");
  const calls = path.join(into, "report-calls");
  fs.writeFileSync(
    file,
    [
      "#!/bin/sh",
      'if [ -n "${KLIN_HOOK_REPORT-}" ]; then',
      "  printf '%s' '{\"status\":\"PASS\",\"summary\":\"hook\",\"derived\":[],\"gates\":[],\"findings\":[],\"notes\":[],\"exit\":0}' >\"$KLIN_HOOK_REPORT\"",
      "fi",
      "printf '%s\\n' \"$*\" >> " + JSON.stringify(calls),
      "printf '%s' 'hook'",
    ].join("\n") + "\n",
  );
  fs.chmodSync(file, 0o755);
  return file;
}

function noisyStub(into: string): string {
  const file = path.join(into, "noisy-stub");
  const output = "x".repeat(20_001) + "target";
  fs.writeFileSync(
    file,
    ["#!/bin/sh", "cat >/dev/null", "printf '" + output + "'"].join("\n") + "\n",
  );
  fs.chmodSync(file, 0o755);
  return file;
}

/**
 * One trial's wrapper, written the way `materialize` writes it.
 *
 * The plane, the binary and the arm are baked into the file, so the suite substitutes them the
 * same way rather than passing them. A wrapper that took them as arguments would put them on a
 * command line the host shows the agent when a Stop hook blocks.
 */
function runHook(into: string, exit: number, deliver: string, args = ["gate", "--hook", "--changed"]) {
  const plane = path.join(into, "plane");
  const ran = spawnSync(laid(into, plane, stub(into, exit), deliver === "1"), args, {
    input: PAYLOAD,
    encoding: "utf8",
    timeout: 20_000,
    env: process.env,
  });
  return { ran, plane, evidence: hookEvidence(path.join(plane, "hooks")) };
}

test("the wrapper closes the pipe, so klin reads its payload to the end", () => {
  const into = room();
  const { ran, evidence } = runHook(into, 0, "1");
  assert.equal(ran.status, 0);
  assert.equal(ran.stdout, "saw:" + PAYLOAD);
  assert.equal(evidence.length, 1);
  assert.equal(evidence[0].stdinClosed, true);
  fs.rmSync(into, { recursive: true, force: true });
});

test("the active arm passes klin's answer and exit status through", () => {
  const into = room();
  const { ran, evidence } = runHook(into, 2, "1");
  assert.equal(ran.status, 2);
  assert.match(ran.stderr, /why:gate --hook --changed/);
  assert.equal(evidence[0].delivered, true);
  assert.equal(evidence[0].status, 2);
  fs.rmSync(into, { recursive: true, force: true });
});

test("the wrapper keeps a structured production report for each gate hook", () => {
  const into = room();
  const plane = path.join(into, "plane");
  spawnSync(laid(into, plane, reportStub(into), true), ["gate", "--hook", "--changed"], {
    input: PAYLOAD,
    encoding: "utf8",
    timeout: 20_000,
    env: process.env,
  });
  assert.deepEqual(hookEvidence(path.join(plane, "hooks"))[0].report, {
    status: "PASS",
    summary: "hook",
    derived: [],
    gates: [],
    findings: [],
    notes: [],
    exit: 0,
  });
  assert.equal(fs.readFileSync(path.join(into, "report-calls"), "utf8"), "gate --hook --changed\n");
  fs.rmSync(into, { recursive: true, force: true });
});

test("hook evidence keeps output after the old 20KB prefix", () => {
  const into = room();
  const plane = path.join(into, "plane");
  const ran = spawnSync(laid(into, plane, noisyStub(into), true), ["gate", "--hook", "--changed"], {
    input: PAYLOAD,
    encoding: "utf8",
    timeout: 20_000,
    env: process.env,
  });
  const output = "x".repeat(20_001) + "target";
  assert.equal(ran.status, 0);
  assert.equal(hookEvidence(path.join(plane, "hooks"))[0].stdout, output);
  fs.rmSync(into, { recursive: true, force: true });
});

test("the shadow arm delivers nothing, on any stream, whatever klin decided", () => {
  const into = room();
  const { ran } = runHook(into, 2, "0");
  assert.equal(ran.status, 0);
  assert.equal(ran.stdout, "");
  assert.equal(ran.stderr, "");
  fs.rmSync(into, { recursive: true, force: true });
});

test("the shadow arm still records what the real hook would have delivered", () => {
  const into = room();
  const { evidence } = runHook(into, 2, "0");
  assert.equal(evidence.length, 1);
  assert.equal(evidence[0].delivered, false);
  assert.equal(evidence[0].status, 2);
  assert.equal(evidence[0].stdout, "saw:" + PAYLOAD);
  assert.match(evidence[0].stderr, /why:gate --hook --changed/);
  assert.equal(evidence[0].stdinClosed, true);
  assert.equal(evidence[0].event, "Stop");
  fs.rmSync(into, { recursive: true, force: true });
});

test("a guard question is suppressed in the shadow arm too", () => {
  const into = room();
  const plane = path.join(into, "plane");
  const ran = spawnSync(laid(into, plane, stub(into, 0), false), ["guard"], {
    input: JSON.stringify({ hook_event_name: "PreToolUse", tool_name: "Edit" }),
    encoding: "utf8",
    timeout: 20_000,
    env: process.env,
  });
  assert.equal(ran.status, 0);
  assert.equal(ran.stdout, "");
  assert.equal(hookEvidence(path.join(plane, "hooks"))[0].event, "PreToolUse");
  fs.rmSync(into, { recursive: true, force: true });
});

test("every invocation is kept, in the order the host made them", () => {
  const into = room();
  const plane = path.join(into, "plane");
  for (const args of [["radius"], ["guard"], ["gate", "--hook", "--changed"]]) {
    spawnSync(laid(into, plane, stub(into, 0), true), args, {
      input: PAYLOAD,
      encoding: "utf8",
      timeout: 20_000,
      env: process.env,
    });
  }
  const evidence = hookEvidence(path.join(plane, "hooks"));
  assert.deepEqual(
    evidence.map((one) => one.arguments),
    ["radius", "guard", "gate --hook --changed"],
    "the arguments recorded are klin's own, which are all the wrapper is ever given",
  );
  fs.rmSync(into, { recursive: true, force: true });
});

test("the wrapper reads its arm from its own bytes and needs no variable", () => {
  const into = room();
  const plane = path.join(into, "plane");
  const seen = path.join(into, "seen");
  fs.writeFileSync(
    seen,
    ["#!/bin/sh", "cat > /dev/null", 'env | grep -c "^KLIN_BENCH" || true'].join("\n") + "\n",
  );
  fs.chmodSync(seen, 0o755);
  const ran = spawnSync(laid(into, plane, seen, false), ["guard"], {
    input: PAYLOAD,
    encoding: "utf8",
    timeout: 20_000,
    env: process.env,
  });
  assert.equal(ran.status, 0);
  assert.equal(
    hookEvidence(path.join(plane, "hooks"))[0].stdout.trim(),
    "0",
    "no KLIN_BENCH variable reached the process the wrapper ran",
  );
  fs.rmSync(into, { recursive: true, force: true });
});

test("the wrapper tells klin where its state is, so the host never carries it", () => {
  const into = room();
  const plane = path.join(into, "plane");
  const seen = path.join(into, "seen");
  fs.writeFileSync(
    seen,
    ["#!/bin/sh", "cat > /dev/null", 'printf "%s" "$KLIN_STATE_DIR"'].join("\n") + "\n",
  );
  fs.chmodSync(seen, 0o755);
  spawnSync(laid(into, plane, seen, true), ["radius"], {
    input: PAYLOAD,
    encoding: "utf8",
    timeout: 20_000,
    env: process.env,
  });
  assert.equal(
    hookEvidence(path.join(plane, "hooks"))[0].stdout,
    path.join(plane, "state"),
  );
  fs.rmSync(into, { recursive: true, force: true });
});

test("the wrapper refuses a plane that is not an absolute path", () => {
  const into = room();
  const ran = spawnSync(laid(into, "plane", stub(into, 0), true), ["gate", "--hook", "--changed"], {
    input: PAYLOAD,
    cwd: into,
    encoding: "utf8",
    timeout: 20_000,
    env: process.env,
  });
  assert.equal(ran.status, 64);
  assert.match(ran.stderr, /must be an absolute path/);
  assert.deepEqual(
    fs.readdirSync(into).sort(),
    ["hook", "stub"],
    "a relative plane must scatter no directory where the host happened to be",
  );
  fs.rmSync(into, { recursive: true, force: true });
});
