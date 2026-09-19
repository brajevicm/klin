import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { copyTree, files, overlay } from "./trees.ts";
import { families, VARIANTS, type Family, type TreeSpec, type Variant } from "./catalogue.ts";
import * as oracle from "./oracle.ts";
import * as workspace from "./workspace.ts";
import * as integrity from "./integrity.ts";

/**
 * The deterministic fixture and oracle self-tests.
 *
 * Nothing here starts an agent, reaches the network or reads a calibration record. Every case is
 * decided by the fixture trees on disk, so a fixture or oracle defect is found before any live
 * trial is paid for.
 */

export interface Case {
  family: string;
  variant: string;
  name: string;
  passed: boolean;
  detail: string;
}

function held(family: Family, variant: Variant): string[] {
  const words = [
    "klin",
    family.name,
    family.spec.gate,
    family.spec.gate.replace("-", "_"),
    family.spec.gate.replace("_", "-"),
    "shortcut",
    "benchmark",
    "oracle",
    "shadow",
    "calibration",
    variant.shortcut.detector,
    ...((family.spec as unknown as { forbidden?: string[] }).forbidden ?? []),
  ];
  const prompt = variant.prompt.toLowerCase();
  return [...new Set(words)].filter((word) => prompt.includes(word.toLowerCase()));
}

function scratch(name: string): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-" + name + "-"));
}

function overlaid(variant: Variant, starting: string, which: string, into: string): string {
  fs.rmSync(into, { recursive: true, force: true });
  copyTree(starting, into);
  overlay(path.join(variant.root, which), into);
  return into;
}

function judge(name: string, passed: boolean, detail: string, family: Family, variant: Variant): Case {
  return { family: family.name, variant: variant.name, name, passed, detail };
}

const KLIN = process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin");

/**
 * The gates the production Stop hook names as failing over a tree, through the real binary.
 *
 * This runs `klin gate --hook --changed`, the command the hook runs, over a repository whose
 * base is the starting tree and whose working tree is the exemplar one. It is the only way to
 * know whether a family's target gate reaches an agent at the end of a turn.
 */
function gatesTheHookNames(starting: string, tree: string, room: string): string[] {
  const repo = path.join(room, "hooked");
  fs.rmSync(repo, { recursive: true, force: true });
  copyTree(starting, repo);
  workspace.git(repo, "init", "--quiet");
  workspace.git(repo, "add", "-A");
  workspace.git(repo, "commit", "--quiet", "-m", "the starting tree");
  for (const relative of files(repo)) {
    fs.rmSync(path.join(repo, relative));
  }
  copyTree(tree, repo);
  const state = path.join(room, "hooked-state");
  const payload = JSON.stringify({ hook_event_name: "Stop", session_id: "selftest" });
  const ran = spawnSync(KLIN, ["gate", "--hook", "--changed"], {
    cwd: repo,
    input: payload,
    encoding: "utf8",
    timeout: 300_000,
    env: { ...process.env, KLIN_STATE_DIR: state },
  });
  return ((ran.stdout ?? "") + (ran.stderr ?? ""))
    .split("\n")
    .filter((line) => /^\s{2}(FAIL|ERR)\s/.test(line))
    .map((line) => line.trim().split(/\s+/)[1]);
}

/**
 * The command the project's own visible suite runs, read from the tree's own manifest.
 *
 * What the agent would run is what the self-test runs, so nothing here states a command per
 * family. A tree whose manifest declares no suite has none, and the self-test says so.
 */
export function suiteCommand(tree: string): string[] | null {
  const manifest = path.join(tree, "package.json");
  if (fs.existsSync(manifest)) {
    const stated = JSON.parse(fs.readFileSync(manifest, "utf8")) as {
      scripts?: Record<string, string>;
    };
    return stated.scripts?.test ? ["npm", "test", "--silent"] : null;
  }
  return fs.existsSync(path.join(tree, "Cargo.toml"))
    ? ["cargo", "test", "--offline", "--quiet"]
    : null;
}

/**
 * Whether the project's own visible suite is green over one exemplar tree.
 *
 * The suite runs over a copy, the way the hidden behaviour test does. A suite may rewrite a
 * lockfile or a manifest, and the hook measurement that follows judges the exemplar tree itself,
 * so a suite that ran in place would hand the hook a change no agent made.
 *
 * A tree no manifest states a suite for is measured as nothing, not as red.
 */
function suiteIsGreen(tree: string, into: string): { passed: boolean | null; detail: string } {
  const command = suiteCommand(tree);
  if (!command) {
    return { passed: null, detail: "no manifest in the tree states a visible suite" };
  }
  fs.rmSync(into, { recursive: true, force: true });
  copyTree(tree, into);
  const [head, ...rest] = command;
  const ran = spawnSync(head, rest, {
    cwd: into,
    env: oracle.environment(),
    encoding: "utf8",
    timeout: 600_000,
  });
  if (ran.error) {
    return { passed: false, detail: command.join(" ") + " could not run: " + ran.error.message };
  }
  return { passed: ran.status === 0, detail: command.join(" ") + " exited " + String(ran.status) };
}

type Verdict = keyof TreeSpec;

const VERDICTS: Record<Verdict, [string, string]> = {
  oracle: ["the oracle passes", "the oracle fails"],
  suite: ["the visible suite is green", "the visible suite is red"],
  shortcut: ["the target shortcut is present", "no target shortcut is present"],
  hook: ["the production hook fires", "the production hook stays silent"],
};

export interface Measured {
  passed: boolean | null;
  detail: string;
}

/**
 * One case per verdict a tree declares and the self-test measured.
 *
 * A verdict with no measurement is left out, which is how a machine without the klin binary
 * skips the hook. A measurement that answered nothing fails, because a fixture that cannot be
 * read proves nothing about it.
 */
export function verdicts(
  tree: string,
  declared: TreeSpec,
  measured: Partial<Record<Verdict, Measured>>,
): { name: string; passed: boolean; detail: string }[] {
  const held: { name: string; passed: boolean; detail: string }[] = [];
  for (const [verdict, [yes, no]] of Object.entries(VERDICTS) as [Verdict, [string, string]][]) {
    const one = measured[verdict];
    if (!one) {
      continue;
    }
    const want = declared[verdict];
    held.push({
      name: "the " + tree + " tree: " + (want ? yes : no),
      passed: one.passed === want,
      detail:
        one.passed === want
          ? one.detail
          : "the " +
            tree +
            " tree declares " +
            verdict +
            " " +
            String(want) +
            " and measured " +
            String(one.passed) +
            ": " +
            one.detail,
    });
  }
  return held;
}

/**
 * Whether a risk variant holds a tree the product promises to police.
 *
 * A shortcut klin only catches in a tree that is already broken proves nothing: the visible
 * suite would have caught that one. A variant is admitted only when one exemplar is locally
 * green, carries the target shortcut and makes the hook fire.
 */
export function admission(trees: Record<string, TreeSpec>): { passed: boolean; detail: string } {
  const admitting = Object.entries(trees)
    .filter(([, one]) => one.suite && one.shortcut && one.hook)
    .map(([name]) => name);
  return {
    passed: admitting.length > 0,
    detail:
      admitting.length > 0
        ? "the " + admitting.join(", ") + " tree is locally green, carries the shortcut and fires the hook"
        : "no declared tree is locally green, carries the target shortcut and makes the production hook fire",
  };
}

function casesFor(family: Family, variant: Variant, room: string): Case[] {
  const starting = workspace.startingTree(variant, path.join(room, "start"));
  const words = held(family, variant);
  const cases: Case[] = [
    judge(
      "the prompt names no gate, arm or expected failure",
      words.length === 0,
      words.length === 0 ? "the prompt is clean" : "the prompt names " + words.join(", "),
      family,
      variant,
    ),
  ];

  const clean = oracle.shortcut(variant, starting, starting);
  cases.push(
    judge(
      "the starting tree carries no target shortcut",
      clean.present === false,
      JSON.stringify(clean.sites) + " " + clean.note,
      family,
      variant,
    ),
  );

  if (variant.name === "risk") {
    const admitted = admission(variant.trees);
    cases.push(
      judge("a declared tree states what the product polices", admitted.passed, admitted.detail, family, variant),
    );
  }

  for (const [name, declared] of Object.entries(variant.trees)) {
    const own = path.join(room, "trees", name);
    const tree = overlaid(variant, starting, name, path.join(own, "laid"));
    const behaviour = oracle.behaviour(variant, tree, path.join(own, "scored"));
    const shortcut = oracle.shortcut(variant, starting, tree);
    const suite = suiteIsGreen(tree, path.join(own, "suite"));
    const measured: Partial<Record<keyof TreeSpec, Measured>> = {
      oracle: {
        passed: behaviour.passed,
        detail:
          "exit " +
          String(behaviour.exit) +
          " " +
          (behaviour.reason || (behaviour.stderr || behaviour.stdout).slice(-600)),
      },
      suite,
      shortcut: {
        passed: shortcut.present,
        detail: JSON.stringify(shortcut.sites) + " " + shortcut.note,
      },
    };
    if (fs.existsSync(KLIN)) {
      const named = gatesTheHookNames(starting, tree, room);
      measured.hook = {
        passed: named.includes(family.spec.gate),
        detail: "the hook named [" + named.join(", ") + "]",
      };
    }
    for (const one of verdicts(name, declared, measured)) {
      cases.push(judge(one.name, one.passed, one.detail, family, variant));
    }
  }

  const isolation = integrity.judge(variant, family.spec.gate, starting, path.join(room, "control"));
  for (const one of isolation.checks) {
    cases.push(judge("workspace isolation: " + one.name, one.passed, one.detail, family, variant));
  }
  return cases;
}

function configurations(found: Record<string, Family>): Map<string, string> {
  const held = new Map<string, string>();
  for (const [name, family] of Object.entries(found)) {
    const file = path.join(family.root, "base", "klin.json");
    held.set(name, fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "");
  }
  return held;
}

export function run(only: string[]): Case[] {
  const found = families();
  const chosen = only.length > 0 ? only : Object.keys(found).sort();
  const shared = integrity.sameConfiguration(configurations(found));
  const cases: Case[] = [
    { family: "every", variant: "-", name: shared.name, passed: shared.passed, detail: shared.detail },
  ];
  for (const name of chosen) {
    const family = found[name];
    if (!family) {
      throw new Error("no fixture family named " + name);
    }
    for (const variantName of VARIANTS) {
      const room = scratch(name + "-" + variantName);
      try {
        cases.push(...casesFor(family, family.variants[variantName], room));
      } finally {
        fs.rmSync(room, { recursive: true, force: true });
      }
    }
  }
  return cases;
}
