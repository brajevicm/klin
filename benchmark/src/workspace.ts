import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as paths from "./paths.ts";
import { copyTree, overlay, digest } from "./trees.ts";
import type { Variant } from "./catalogue.ts";

/**
 * The subject workspace and the control plane.
 *
 * Every trial gets a fresh standalone Git repository, a fresh klin state directory and a fresh
 * host session. The workspace holds the fixture and klin's configuration, and nothing else. The
 * arm, the scoring metadata, the expected shortcut and the external oracle stay in the control
 * plane, which sits under a different root.
 */

export interface Workspace {
  root: string;
  repo: string;
  hook: string;
  settings: string;
  state: string;
  hooks: string;
  startCommit: string;
  treeSha256: string;
  commits: number;
}

const GIT = [
  "-c",
  "user.name=klin benchmark",
  "-c",
  "user.email=benchmark@example.invalid",
  "-c",
  "commit.gpgsign=false",
  "-c",
  "init.defaultBranch=main",
];

export function git(repo: string, ...args: string[]): string {
  const ran = spawnSync("git", [...GIT, ...args], { cwd: repo, encoding: "utf8" });
  if (ran.status !== 0) {
    throw new Error("git " + args.join(" ") + " failed: " + (ran.stderr ?? ""));
  }
  return (ran.stdout ?? "").trim();
}

function settingsFor(hook: string): string {
  const command = (args: string): string => JSON.stringify(hook) + " " + args;
  return JSON.stringify(
    {
      hooks: {
        SessionStart: [{ hooks: [{ type: "command", command: command("radius"), timeout: 60 }] }],
        UserPromptSubmit: [
          { hooks: [{ type: "command", command: command("radius"), timeout: 60 }] },
        ],
        PreToolUse: [
          {
            matcher: "Write|Edit|MultiEdit|NotebookEdit|Bash|apply_patch|mcp__.*",
            hooks: [{ type: "command", command: command("guard"), timeout: 60 }],
          },
        ],
        Stop: [
          {
            hooks: [
              { type: "command", command: command("gate --hook --changed"), timeout: 900 },
            ],
          },
        ],
      },
    },
    null,
    2,
  );
}

/**
 * Lay one variant's starting tree into `into`.
 *
 * The subject workspace and the tree a detector compares against are laid by this one function,
 * so the baseline a shortcut is measured against is always the tree the agent was given.
 */
function layStartingTree(variant: Variant, into: string): string {
  copyTree(path.join(variant.familyRoot, "base"), into);
  overlay(path.join(variant.root, "overlay"), into);
  if (!fs.existsSync(path.join(into, "klin.json"))) {
    fs.writeFileSync(path.join(into, "klin.json"), "{}\n");
  }
  return into;
}

/** Materialize one trial's subject workspace and its host wiring. */
export function materialize(variant: Variant, trialId: string): Workspace {
  const root = path.join(paths.workRoot(), trialId);
  fs.rmSync(root, { recursive: true, force: true });
  const repo = path.join(root, "repo");
  const hook = path.join(root, "hook");
  const settings = path.join(root, "settings.json");
  const state = path.join(root, "state");
  const hooks = path.join(root, "h");
  fs.mkdirSync(repo, { recursive: true });

  layStartingTree(variant, repo);

  fs.copyFileSync(paths.HOOK, hook);
  fs.chmodSync(hook, 0o755);
  fs.writeFileSync(settings, settingsFor(hook) + "\n");

  const treeSha256 = digest(repo);
  git(repo, "init", "--quiet");
  git(repo, "add", "-A");
  git(repo, "commit", "--quiet", "-m", "The starting tree");
  const startCommit = git(repo, "rev-parse", "HEAD");
  const commits = git(repo, "rev-list", "--count", "HEAD");

  return {
    root,
    repo,
    hook,
    settings,
    state,
    hooks,
    startCommit,
    treeSha256,
    commits: Number(commits),
  };
}

/** The starting tree again, on its own, so a detector can compare against it. */
export function startingTree(variant: Variant, into: string): string {
  fs.rmSync(into, { recursive: true, force: true });
  fs.mkdirSync(into, { recursive: true });
  return layStartingTree(variant, into);
}
