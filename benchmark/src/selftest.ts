import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { copyTree, files, overlay } from "./trees.ts";
import {
  families,
  variantNames,
  variantIn,
  type Family,
  type FamilySpec,
  type TreeSpec,
  type Variant,
} from "./catalogue.ts";
import * as oracle from "./oracle.ts";
import * as workspace from "./workspace.ts";
import * as integrity from "./integrity.ts";
import * as toolchain from "./toolchain.ts";

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

/** The binary under test, read on every use so a test can stand a stub in its place. */
function klinBinary(): string {
  return process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin");
}

/**
 * The gates the production Stop hook names as failing over a tree, through the real binary.
 *
 * This runs `klin gate --hook --changed`, the command the hook runs, over a repository whose
 * base is the starting tree and whose working tree is the exemplar one. It is the only way to
 * know whether a family's target gate reaches an agent at the end of a turn.
 *
 * `room` is the exemplar tree's own room, so the repository and klin's state directory are fresh
 * for every tree. A stop writes a stamp and the findings it reported into that state, and the
 * next stop reads them, so two trees measured through one state would answer in the order they
 * ran rather than on their own evidence.
 *
 * A run that could not be read answers nothing rather than an empty list. klin prints nothing and
 * exits 0 when every gate passed, so an empty list is only a fact when the exit code says so.
 */
export function gatesTheHookNames(
  starting: string,
  tree: string,
  room: string,
  gate: string,
): Measured {
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
  fs.rmSync(state, { recursive: true, force: true });
  const payload = JSON.stringify({ hook_event_name: "Stop", session_id: "selftest" });
  const ran = spawnSync(klinBinary(), ["gate", "--hook", "--changed"], {
    cwd: repo,
    input: payload,
    encoding: "utf8",
    timeout: 300_000,
    env: { ...process.env, KLIN_STATE_DIR: state },
  });
  return hookVerdict(ran, gate);
}

/**
 * What one hook run says about one gate, with every way it could not answer held apart from a
 * stop it let through.
 *
 * A Claude Code stop block is exit 2 with the report on stderr (9.1), so exit 0 is a stop klin
 * allowed and the gate did not reach the agent. Exit 1 is a host event klin could not read, and
 * every other exit code, a spawn error and a signal are the same kind of answer: none. A blocked
 * stop is read from the gate's own row, so an `ERR` on the gate this family measures is
 * indeterminate and a block another gate raised is not this gate firing.
 */
export function hookVerdict(
  ran: {
    error?: Error;
    signal?: NodeJS.Signals | null;
    status: number | null;
    stdout?: string;
    stderr?: string;
  },
  gate: string,
): Measured {
  if (ran.error) {
    return { passed: null, detail: "the hook could not run: " + ran.error.message };
  }
  if (ran.signal) {
    return { passed: null, detail: "the hook took the signal " + ran.signal };
  }
  const output = (ran.stdout ?? "") + (ran.stderr ?? "");
  if (ran.status === 0) {
    return { passed: false, detail: "the hook let the stop through" };
  }
  const row = new RegExp(
    "^\\s{2}(ok|FAIL|ERR)\\s+" + gate.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + "\\s*$",
    "m",
  ).exec(output);
  if (ran.status === 2 && row) {
    return {
      passed: row[1] === "ERR" ? null : row[1] === "FAIL",
      detail: "the hook blocked the stop and the " + gate + " row reads " + row[1],
    };
  }
  return {
    passed: null,
    detail:
      "the hook exited " +
      String(ran.status) +
      " and its report holds no " +
      gate +
      " row: " +
      output.slice(-600),
  };
}

/**
 * The command the project's own visible suite runs, from the language the family declares.
 *
 * What the agent would run is what the self-test runs, so the command is the fixture's own: the
 * npm `test` script, or `cargo test`. The language decides which manifest is read, so a stray
 * `package.json` beside a Cargo manifest cannot move a Rust family onto npm.
 */
export function suiteCommand(language: FamilySpec["language"], tree: string): string[] | null {
  if (language === "rust") {
    return fs.existsSync(path.join(tree, "Cargo.toml"))
      ? ["cargo", "test", "--offline", "--quiet"]
      : null;
  }
  const manifest = path.join(tree, "package.json");
  if (!fs.existsSync(manifest)) {
    return null;
  }
  const stated = JSON.parse(fs.readFileSync(manifest, "utf8")) as {
    scripts?: Record<string, string>;
  };
  return stated.scripts?.test ? ["npm", "test", "--silent"] : null;
}

/**
 * Whether the project's own visible suite is green over one exemplar tree.
 *
 * The suite runs over a copy, the way the hidden behaviour test does. A suite may rewrite a
 * lockfile or a manifest, and the hook measurement that follows judges the exemplar tree itself,
 * so a suite that ran in place would hand the hook a change no agent made.
 *
 * A tree whose manifest states no suite is measured as nothing, not as red.
 */
function suiteIsGreen(language: FamilySpec["language"], tree: string, into: string): Measured {
  const command = suiteCommand(language, tree);
  if (!command) {
    return { passed: null, detail: "no " + language + " manifest in the tree states a visible suite" };
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
    return { passed: null, detail: command.join(" ") + " could not run: " + ran.error.message };
  }
  if (ran.signal) {
    return { passed: null, detail: command.join(" ") + " took the signal " + ran.signal };
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
 * One case per verdict, for every one of the four a tree declares.
 *
 * Every verdict is asserted on every tree. A measurement that answered nothing fails rather than
 * passing quietly, because a verdict this could not measure proves nothing about the tree.
 */
export function verdicts(
  tree: string,
  declared: TreeSpec,
  measured: Record<Verdict, Measured>,
): { name: string; passed: boolean; detail: string }[] {
  const held: { name: string; passed: boolean; detail: string }[] = [];
  for (const [verdict, [yes, no]] of Object.entries(VERDICTS) as [Verdict, [string, string]][]) {
    const one = measured[verdict];
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
 *
 * The three answers are the measured ones, never the declared ones. A declaration that says a
 * tree polices what the product promises, over a measurement that answered nothing, admits
 * nothing.
 */
export function admission(
  measured: Record<string, Record<Verdict, Measured>>,
): { passed: boolean; detail: string } {
  const admitting = Object.entries(measured)
    .filter(([, one]) => one.suite.passed === true && one.shortcut.passed === true && one.hook.passed === true)
    .map(([name]) => name);
  return {
    passed: admitting.length > 0,
    detail:
      admitting.length > 0
        ? "the " + admitting.join(", ") + " tree measured locally green, carrying the shortcut and firing the hook"
        : "no measured tree is locally green, carries the target shortcut and makes the production hook fire",
  };
}

/**
 * The cases a seeded variant owes beyond every other variant's.
 *
 * A seed is both the overlay the harness leaves uncommitted and an exemplar tree the self-test
 * measures on all four verdicts, so it has to be one of the declared trees: what the harness
 * plants is then exactly what the four verdicts were taken over. A seed that removed a file would
 * stand in the working tree as a deletion the declared path list does not name, and `seed-as-
 * declared` would fail every live trial, so it is refused here instead.
 */
function seedCases(family: Family, variant: Variant): Case[] {
  if (variant.seed === "") {
    return [];
  }
  const declared = Object.keys(variant.trees);
  const removals = path.join(variant.root, variant.seed, "REMOVE");
  return [
    judge(
      "the seed overlay is one of the variant's declared trees",
      declared.includes(variant.seed),
      declared.includes(variant.seed)
        ? "the seed " + variant.seed + " is measured as an exemplar tree"
        : "the variant seeds " + variant.seed + " and declares " + declared.join(", "),
      family,
      variant,
    ),
    judge(
      "the seed overlay states no removal",
      !fs.existsSync(removals),
      fs.existsSync(removals)
        ? "the seed removes files, which no declared seed path can name"
        : "the seed only writes files",
      family,
      variant,
    ),
  ];
}

function casesFor(family: Family, variant: Variant, room: string): Case[] {
  const starting = workspace.startingTree(variant, path.join(room, "start"));
  const subject = workspace.subjectStartingTree(variant, path.join(room, "subject"));
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

  // The committed clean base against the tree the subject is given. They are one tree wherever
  // the variant declares no seed, so this reads as the old clean-start invariant there, and a
  // seeded variant declares the target shortcut present instead.
  const clean = oracle.shortcut(variant, starting, subject);
  cases.push(
    judge(
      variant.start.shortcut
        ? "the subject's starting tree carries the declared target shortcut"
        : "the subject's starting tree carries no target shortcut",
      clean.present === variant.start.shortcut,
      clean.present === variant.start.shortcut
        ? JSON.stringify(clean.sites) + " " + clean.note
        : "the variant declares the starting shortcut " +
          String(variant.start.shortcut) +
          " and the detector measured " +
          String(clean.present) +
          ": " +
          JSON.stringify(clean.sites) +
          " " +
          clean.note,
      family,
      variant,
    ),
    ...seedCases(family, variant),
  );

  const measured: Record<string, Record<keyof TreeSpec, Measured>> = {};
  for (const [name, declared] of Object.entries(variant.trees)) {
    const own = path.join(room, "trees", name);
    const tree = overlaid(variant, starting, name, path.join(own, "laid"));
    const behaviour = oracle.behaviour(variant, tree, path.join(own, "scored"));
    const shortcut = oracle.shortcut(variant, starting, tree);
    measured[name] = {
      oracle: {
        passed: behaviour.passed,
        detail:
          "exit " +
          String(behaviour.exit) +
          " " +
          (behaviour.reason || (behaviour.stderr || behaviour.stdout).slice(-600)),
      },
      suite: suiteIsGreen(family.spec.language, tree, path.join(own, "suite")),
      shortcut: {
        passed: shortcut.present,
        detail: JSON.stringify(shortcut.sites) + " " + shortcut.note,
      },
      hook: fs.existsSync(klinBinary())
        ? gatesTheHookNames(starting, tree, own, family.spec.gate)
        : { passed: null, detail: "no klin binary stands at " + klinBinary() },
    };
    for (const one of verdicts(name, declared, measured[name])) {
      cases.push(judge(one.name, one.passed, one.detail, family, variant));
    }
  }

  // Control is matched ordinary work where the target regression should not appear, so it owes
  // no policed tree. Every other variant does: a risk variant has to make the shortcut available
  // and a seeded one has to plant it.
  if (variant.name !== "control") {
    const admitted = admission(measured);
    cases.push(
      judge("a measured tree states what the product polices", admitted.passed, admitted.detail, family, variant),
    );
  }

  // The isolation scan reads the tree the agent is given, which for a seeded variant is the
  // committed base under its seed.
  const isolation = integrity.judge(variant, family.spec.gate, subject, path.join(room, "control"));
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

export function run(only: string[], population: "all" | "seeded" = "all"): Case[] {
  const found = families();
  const chosen = only.length > 0 ? only : Object.keys(found).sort();
  const shared = integrity.sameConfiguration(configurations(found));
  const compiler = toolchain.current();
  const cases: Case[] = [
    { family: "every", variant: "-", name: shared.name, passed: shared.passed, detail: shared.detail },
    {
      family: "every",
      variant: "-",
      name: "the benchmark TypeScript compiler is prepared",
      passed: compiler !== null,
      detail: compiler?.path ?? toolchain.requirement(),
    },
    {
      family: "every",
      variant: "-",
      name: "the klin binary under test is on this machine",
      passed: fs.existsSync(klinBinary()),
      detail: fs.existsSync(klinBinary())
        ? klinBinary()
        : "no binary stands at " +
          klinBinary() +
          ", so no hook verdict can be measured. Build it first.",
    },
  ];
  for (const name of chosen) {
    const family = found[name];
    if (!family) {
      throw new Error("no fixture family named " + name);
    }
    for (const variantName of variantNames(family).filter((one) => population === "all" || one === "seeded")) {
      const room = scratch(name + "-" + variantName);
      try {
        cases.push(...casesFor(family, variantIn(family, variantName), room));
      } finally {
        fs.rmSync(room, { recursive: true, force: true });
      }
    }
  }
  return cases;
}
