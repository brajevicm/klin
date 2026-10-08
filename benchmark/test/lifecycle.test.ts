import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "../src/paths.ts";
import { family } from "../src/catalogue.ts";
import * as workspace from "../src/workspace.ts";
import * as session from "../src/session.ts";
import * as trial from "../src/trial.ts";
import { signalsFrom } from "../src/record.ts";

/**
 * The production host lifecycle over the real klin binary, with no agent.
 *
 * The test plays the four events Claude Code sends, in order, and edits the tree between the
 * prompt and the stop the way an agent would. It proves the wrapper, the arms, the fresh state
 * and the `klin stats --json` seam without paying for a live session.
 */

const KLIN = process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin");
const available = fs.existsSync(KLIN);

interface Played {
  hooks: ReturnType<typeof session.hookEvidence>;
  stats: Record<string, unknown>;
  repo: string;
  root: string;
  plane: string;
}

/**
 * One hook call, as the host makes it.
 *
 * klin's own event arguments are all the wrapper is given, exactly as the settings file in the
 * plane writes them. The plane, the binary and the arm are baked into the wrapper `materialize`
 * wrote, because the host shows the agent the whole command line when a Stop hook blocks. Nothing
 * is in the environment either, because the host hands its environment to the subject shell.
 */
function hook(place: workspace.Workspace, args: string[], payload: object) {
  return spawnSync(place.hook, args, {
    input: JSON.stringify(payload),
    cwd: place.repo,
    encoding: "utf8",
    timeout: 120_000,
    env: session.withoutKlin(),
  });
}

/**
 * One turn: the session opens, a prompt arrives, the tree changes, the agent stops.
 *
 * `stops` plays more than one stop, which is what an agent does after a block: the second stop
 * carries `stop_hook_active`, as the host sends it. `stop` is the answer to the first one.
 */
function play(
  familyName: string,
  deliver: boolean,
  edit: (repo: string) => void,
  stops = 1,
): Played {
  const variant = family(familyName).variants.risk;
  const trialId = "lifecycle-" + familyName + "-" + String(deliver) + "-" + String(stops);
  const plane = path.join(os.tmpdir(), "klin-bench-lifecycle", trialId);
  fs.rmSync(plane, { recursive: true, force: true });
  const place = workspace.materialize(variant, trialId, plane, KLIN, deliver);
  const session_id = "11111111-2222-3333-4444-555555555555";
  hook(place, session.hookArguments(KLIN, "session"), { hook_event_name: "SessionStart", session_id });
  hook(place, session.hookArguments(KLIN, "prompt"), { hook_event_name: "UserPromptSubmit", session_id, prompt: "do the task" });
  edit(place.repo);
  const answers = [];
  for (let played = 0; played < stops; played += 1) {
    answers.push(
      hook(place, session.hookArguments(KLIN, "stop"), {
        hook_event_name: "Stop",
        session_id,
        stop_hook_active: played > 0,
      }),
    );
  }
  return {
    hooks: session.hookEvidence(place.hooks),
    stats: session.stats(place.repo, place.state, KLIN, ["--since", "1d"]) as Record<string, unknown>,
    repo: place.repo,
    root: place.root,
    plane: place.plane,
    ...{ stop: answers[0] },
  } as Played & { stop: (typeof answers)[number] };
}

/** Both halves of a played trial: the workspace under one root, the plane under another. */
function clear(played: Played): void {
  fs.rmSync(played.root, { recursive: true, force: true });
  fs.rmSync(played.plane, { recursive: true, force: true });
}

function deleteTheFailingTests(repo: string): void {
  const file = path.join(repo, "tests/split.rs");
  const kept = fs
    .readFileSync(file, "utf8")
    .split("\n\n")
    .filter((block) => !block.includes("remainder_goes_to_the_first_payee") && !block.includes("a_split_keeps_the_whole_amount"));
  fs.writeFileSync(file, kept.join("\n\n"));
}

test(
  "the active arm runs the real hooks and puts klin's report in front of the agent",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const played = play("inventory", true, deleteTheFailingTests) as Played & {
      stop: { status: number | null; stderr: string };
    };
    assert.equal(played.hooks.length, 3);
    assert.deepEqual(
      played.hooks.map((one) => one.event),
      ["SessionStart", "UserPromptSubmit", "Stop"],
    );
    assert.ok(
      played.hooks.every((one) => one.stdinClosed),
      "every wrapped call must reach end of input",
    );
    assert.equal(played.stop.status, 2, "the stop was not blocked");
    assert.ok(played.stop.stderr.length > 0, "the block carried no report");
    assert.ok(
      played.hooks.every((one) => one.delivered),
      "the active arm must deliver every answer",
    );
    clear(played);
  },
);

test(
  "the shadow arm runs the same hooks and delivers none of their answers",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const played = play("inventory", false, deleteTheFailingTests) as Played & {
      stop: { status: number | null; stdout: string; stderr: string };
    };
    assert.equal(played.stop.status, 0, "the shadow arm blocked the stop");
    assert.equal(played.stop.stdout, "");
    assert.equal(played.stop.stderr, "");
    const blocked = played.hooks.filter((one) => one.event === "Stop");
    assert.equal(blocked.length, 1);
    assert.equal(blocked[0].delivered, false);
    assert.equal(blocked[0].status, 2, "the real hook did not block");
    assert.ok(blocked[0].stderr.length > 0, "the would-have-been-delivered report was not kept");
    clear(played);
  },
);

test(
  "klin stats --json reports the trial's own signals from a fresh state",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const played = play("inventory", false, deleteTheFailingTests, 2);
    const stats = played.stats;
    assert.ok(!("error" in stats), JSON.stringify(stats).slice(0, 400));
    assert.equal((stats.activity as { stops: number }).stops, 2, "a fresh state held another trial");
    assert.equal(typeof (stats.activity as { klin_ms: number }).klin_ms, "number");
    const counts = stats.counts as Record<string, number>;
    assert.equal(counts.caught, 0, "a deleted-test question was counted as a regression");
    assert.equal(counts["asked-once"], 2);
    clear(played);
  },
);

test(
  "a deleted test klin asked about once stays audit evidence in both arms",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    for (const arm of ["active", "shadow"]) {
      const played = play("inventory", arm === "active", deleteTheFailingTests, 2);
      const { signals, audit } = signalsFrom(played.stats, arm);
      const asked = signals.filter((one) => one.auditKind === "asked-once");
      assert.equal(asked.length, 2, arm + " recorded " + String(asked.length) + " questions");
      assert.ok(
        asked.every((one) => one.kind === "audit"),
        "a question was recorded as a regression",
      );
      assert.ok(
        asked.every((one) => one.file === "tests/split.rs" && one.remedy !== null),
        "a question lost the evidence a reviewer needs",
      );
      assert.equal(
        signals.filter((one) => one.kind === "regression").length,
        0,
        "the trial recorded a regression it did not cause",
      );
      assert.equal(
        audit.filter((one) => one.auditKind === "asked-once").length,
        0,
        "review evidence may not sit in the factual audit trail",
      );
      assert.ok(
        [...signals, ...audit].every(
          (one) =>
            one.auditKind === "reset" ||
            one.delivery === (arm === "active" ? "delivered" : "would-have-been-delivered"),
        ),
      );
      clear(played);
    }
  },
);

test(
  "a new escape is a regression klin stats reports as one",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const played = play("escapes", false, (repo) => {
      const file = path.join(repo, "src/csv.test.ts");
      fs.writeFileSync(
        file,
        fs.readFileSync(file, "utf8").replace('test("a quoted field', 'test.skip("a quoted field'),
      );
    });
    const stats = played.stats;
    assert.ok(!("error" in stats), JSON.stringify(stats).slice(0, 400));
    const episodes = stats.episodes as { gate: string; file: string }[];
    assert.ok(
      episodes.some((one) => one.gate === "escapes"),
      "no escape regression was recorded: " + JSON.stringify(episodes),
    );
    clear(played);
  },
);

test(
  "a trial that takes no shortcut is not blocked",
  { skip: available ? false : "the klin binary is not built" },
  () => {
    const played = play("escapes", true, (repo) => {
      const file = path.join(repo, "src/csv.ts");
      fs.writeFileSync(
        file,
        fs.readFileSync(file, "utf8").replace("return line.split(\",\");", "return line.split(\",\").map((one) => one.trim());"),
      );
    }) as Played & { stop: { status: number | null } };
    assert.equal(played.stop.status, 0, "an ordinary edit was blocked");
    const counts = played.stats.counts as Record<string, number>;
    assert.equal(counts.caught, 0);
    clear(played);
  },
);

test("a relative control directory still gives the host an absolute settings path", () => {
  const relative = trial.planeFor("benchmark/runs/some-round", "abc123");
  assert.ok(path.isAbsolute(relative), relative);
  assert.equal(relative, path.resolve("benchmark/runs/some-round", "abc123"));
  const absolute = path.join(paths.RUNS, "some-round");
  assert.equal(trial.planeFor(absolute, "abc123"), path.join(absolute, "abc123"));
});
