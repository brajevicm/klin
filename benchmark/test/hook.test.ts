import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import { hookEvidence } from "../src/session.ts";

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

function runHook(into: string, exit: number, deliver: string) {
  const hooks = path.join(into, "h");
  const ran = spawnSync(paths.HOOK, ["gate", "--hook", "--changed"], {
    input: PAYLOAD,
    encoding: "utf8",
    timeout: 20_000,
    env: {
      ...process.env,
      KLIN_BIN: stub(into, exit),
      KLIN_BENCH_HOOKS: hooks,
      KLIN_BENCH_DELIVER: deliver,
    },
  });
  return { ran, evidence: hookEvidence(hooks) };
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
  const hooks = path.join(into, "h");
  const ran = spawnSync(paths.HOOK, ["guard"], {
    input: JSON.stringify({ hook_event_name: "PreToolUse", tool_name: "Edit" }),
    encoding: "utf8",
    timeout: 20_000,
    env: {
      ...process.env,
      KLIN_BIN: stub(into, 0),
      KLIN_BENCH_HOOKS: hooks,
      KLIN_BENCH_DELIVER: "0",
    },
  });
  assert.equal(ran.status, 0);
  assert.equal(ran.stdout, "");
  assert.equal(hookEvidence(hooks)[0].event, "PreToolUse");
  fs.rmSync(into, { recursive: true, force: true });
});

test("every invocation is kept, in the order the host made them", () => {
  const into = room();
  const hooks = path.join(into, "h");
  for (const args of [["radius"], ["guard"], ["gate", "--hook", "--changed"]]) {
    spawnSync(paths.HOOK, args, {
      input: PAYLOAD,
      encoding: "utf8",
      timeout: 20_000,
      env: {
        ...process.env,
        KLIN_BIN: stub(into, 0),
        KLIN_BENCH_HOOKS: hooks,
        KLIN_BENCH_DELIVER: "1",
      },
    });
  }
  const evidence = hookEvidence(hooks);
  assert.deepEqual(
    evidence.map((one) => one.arguments),
    ["radius", "guard", "gate --hook --changed"],
  );
  fs.rmSync(into, { recursive: true, force: true });
});
