import { spawnSync, execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { sha256 } from "./trees.ts";
import * as paths from "./paths.ts";
import type { HookInvocation } from "./record.ts";
import type { Workspace } from "./workspace.ts";

/**
 * One headless Claude Code session over one subject workspace.
 *
 * Both arms use the same host, the same flags, the same model and the same production hook
 * lifecycle. The arm reaches the wrapper through the hook command in the plane's settings file,
 * and the wrapper is byte-identical in both arms.
 *
 * Nothing the harness knows reaches the subject's environment. The subject has `Bash`, so it
 * inherits this process's environment and could read its own arm out of it. Every `KLIN_`
 * variable is dropped for the same reason: one left over in the operator's own shell would
 * otherwise name klin's binary, klin's state or the arm.
 */

export interface SessionOptions {
  model: string;
  budgetUsd: number;
  timeoutMs: number;
  klinBin: string;
  configRoot: string;
  extraFlags: string[];
}

export interface SessionResult {
  flags: string[];
  flagsSha256: string;
  sessionId: string;
  exit: number | null;
  stdout: string;
  stderr: string;
  agent: Record<string, unknown> | null;
  startedAt: string;
  endedAt: string;
  wallMs: number;
  timedOut: boolean;
}

export function defaults(): SessionOptions {
  return {
    model: process.env.KLIN_BENCH_MODEL ?? "sonnet",
    budgetUsd: Number(process.env.KLIN_BENCH_BUDGET ?? "5"),
    timeoutMs: Number(process.env.KLIN_BENCH_TIMEOUT_MS ?? String(30 * 60 * 1000)),
    klinBin: process.env.KLIN_BIN ?? path.join(paths.REPO, "target", "release", "klin"),
    configRoot: process.env.KLIN_BENCH_CONFIG_DIR ?? "",
    extraFlags: (process.env.KLIN_BENCH_FLAGS ?? "").split(" ").filter((one) => one.length > 0),
  };
}

/**
 * The tools a fixture task needs.
 *
 * Without this list the host's permission system refuses every `Bash` call, so an agent cannot
 * run the suite the task tells it to make green, and the fixture measures a different task.
 * Calibration on 2026-09-17 found that. klin's guard is unaffected: a `PreToolUse` hook that
 * exits 2 still refuses the call, and a hook that answers `ask` still needs a person.
 */
const TOOLS = ["Bash", "Edit", "Write", "Read", "Glob", "Grep", "TodoWrite"];

function flagsFor(workspace: Workspace, sessionId: string, options: SessionOptions): string[] {
  return [
    "--print",
    "--output-format",
    "json",
    "--permission-mode",
    "acceptEdits",
    "--allowedTools",
    TOOLS.join(","),
    "--setting-sources",
    "project,local",
    "--settings",
    workspace.settings,
    "--strict-mcp-config",
    "--disable-slash-commands",
    "--model",
    options.model,
    "--session-id",
    sessionId,
    "--max-budget-usd",
    String(options.budgetUsd),
    ...options.extraFlags,
  ];
}

/**
 * The paths a subject must not learn, in both their symbolic-link-resolved forms.
 *
 * klin's own repository is one of them. It holds the fixtures, the hidden oracles, every other
 * trial's records and, under `benchmark/runs`, this trial's plane. A subject that learns this
 * one path can name all of it, and it can name the klin binary under `target/release` without
 * ever reading a file the sandbox refuses.
 */
function owned(): string[] {
  const held = new Set<string>();
  for (const one of [paths.REPO, paths.workRoot()]) {
    held.add(one);
    try {
      held.add(fs.realpathSync(one));
    } catch {
      // The work root does not exist before the first trial materializes a workspace.
    }
  }
  return [...held];
}

/**
 * Every variable of `process.env` less the ones klin uses and the ones that name a path the
 * harness owns.
 *
 * A behaviour test and a subject session both get this: nothing the harness or klin set may reach
 * the process that decides whether the requested behaviour is correct, and nothing may reach a
 * subject that could tell it which arm it is in.
 *
 * The second rule is why this is a filter over values and not a list of names. The harness runs
 * out of klin's own repository, so `PWD` and `OLDPWD` both carry that path into every child
 * process by inheritance, whatever `cwd` the child is given. A live trial found a subject that
 * read those and ran the klin binary under `target/release` against its own tree. Naming the two
 * variables would close that one route and leave the next one open, so every variable whose
 * value names an owned path is dropped instead.
 *
 * `PATH` is filtered rather than dropped, because a subject with no `PATH` cannot run its build
 * at all. An entry under an owned path would put the klin binary one `klin` away.
 */
export function withoutKlin(): NodeJS.ProcessEnv {
  const kept: NodeJS.ProcessEnv = {};
  const secret = owned();
  const names = (value: string): boolean => secret.some((one) => value.includes(one));
  for (const [name, value] of Object.entries(process.env)) {
    if (name.startsWith("KLIN_") || value === undefined) {
      continue;
    }
    if (name === "PATH") {
      kept.PATH = value
        .split(path.delimiter)
        .filter((entry) => !names(entry))
        .join(path.delimiter);
      continue;
    }
    if (names(value)) {
      continue;
    }
    kept[name] = value;
  }
  return kept;
}

/** Run the agent over one workspace. The treatment is in the workspace's settings, not here. */
export function run(
  workspace: Workspace,
  prompt: string,
  options: SessionOptions,
  configDir = "",
): SessionResult {
  const sessionId = randomUUID();
  if (configDir !== "") {
    fs.mkdirSync(configDir, { recursive: true });
  }
  const flags = flagsFor(workspace, sessionId, options);
  const startedAt = new Date().toISOString();
  const began = Date.now();
  const ran = spawnSync("claude", [...flags, prompt], {
    cwd: workspace.repo,
    encoding: "utf8",
    timeout: options.timeoutMs,
    maxBuffer: 64 * 1024 * 1024,
    env: {
      ...withoutKlin(),
      // `cwd` moves the process. `PWD` is what a shell reports, and an inherited one would still
      // name the directory the harness ran from.
      PWD: workspace.repo,
      ...(configDir === "" ? {} : { CLAUDE_CONFIG_DIR: configDir }),
    },
  });
  const endedAt = new Date().toISOString();
  let agent: Record<string, unknown> | null = null;
  try {
    agent = JSON.parse(ran.stdout ?? "") as Record<string, unknown>;
  } catch {
    agent = null;
  }
  return {
    flags,
    flagsSha256: sha256(flags.join(" ")),
    sessionId,
    exit: ran.status,
    stdout: (ran.stdout ?? "").slice(-200_000),
    stderr: (ran.stderr ?? "").slice(-20_000),
    agent,
    startedAt,
    endedAt,
    wallMs: Date.now() - began,
    timedOut: (ran.error as NodeJS.ErrnoException | undefined)?.code === "ETIMEDOUT",
  };
}

function eventOf(payload: string): string {
  try {
    const held = JSON.parse(payload) as Record<string, unknown>;
    return String(held.hook_event_name ?? "");
  } catch {
    return "";
  }
}

/**
 * The keys of a tool input that hold a path.
 *
 * Nothing else is kept. A `Write` carries the file's whole content and an `Edit` carries the text
 * it replaces, and a moved TypeScript file's own `import "../socket"` would read as a path out of
 * the workspace if either were scanned. Keeping only these also keeps the record small and holds
 * none of the subject's prose.
 */
const PATH_KEYS = ["command", "file_path", "notebook_path", "path"];

/**
 * The tool a payload names, and the paths its input holds.
 *
 * The guard event carries what the agent asked to do. It is the harness's only record of the
 * subject's own tool calls, and `integrity.stayedInside` reads it to say whether the subject went
 * looking outside its repository. A tool klin's production matcher does not cover raises no hook,
 * so `Read`, `Glob` and `Grep` leave nothing here.
 */
function toolOf(payload: string): { tool: string; paths: string } {
  try {
    const held = JSON.parse(payload) as Record<string, unknown>;
    const input = (held.tool_input ?? {}) as Record<string, unknown>;
    return {
      tool: String(held.tool_name ?? ""),
      paths: PATH_KEYS.filter((key) => typeof input[key] === "string")
        .map((key) => input[key] as string)
        .join(" ")
        .slice(0, 4000),
    };
  } catch {
    return { tool: "", paths: "" };
  }
}

function slurp(file: string): string {
  return fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
}

/**
 * The hook evidence the wrapper left, in order.
 *
 * `stdinClosed` states that klin's read of the payload completed: the wrapper writes the whole
 * payload and closes the pipe, so klin's `read_to_string` reaches end of input and the process
 * exits. A hook that never reached end of input would still be blocked and would record no exit
 * status.
 */
export function hookEvidence(directory: string): HookInvocation[] {
  if (!fs.existsSync(directory)) {
    return [];
  }
  return fs
    .readdirSync(directory)
    .sort()
    .map((name, order) => {
      const kept = path.join(directory, name);
      const payload = slurp(path.join(kept, "payload.json"));
      const status = slurp(path.join(kept, "status")).trim();
      const asked = toolOf(payload);
      return {
        order,
        event: eventOf(payload),
        tool: asked.tool,
        paths: asked.paths,
        arguments: slurp(path.join(kept, "arguments")).trim(),
        status: Number(status),
        delivered: slurp(path.join(kept, "deliver")).trim() === "1",
        stdout: slurp(path.join(kept, "stdout")).slice(0, 20_000),
        stderr: slurp(path.join(kept, "stderr")).slice(0, 20_000),
        started: slurp(path.join(kept, "started")).trim(),
        ended: slurp(path.join(kept, "ended")).trim(),
        stdinClosed: payload.length > 0 && status.length > 0,
      };
    });
}

/** What klin reports about the trial, through the command line and never through the journal
 * file. */
export function stats(repo: string, state: string, klinBin: string, scope: string[]): unknown {
  const ran = spawnSync(klinBin, ["stats", "--json", ...scope], {
    cwd: repo,
    encoding: "utf8",
    env: { ...withoutKlin(), KLIN_STATE_DIR: state },
  });
  try {
    return JSON.parse(ran.stdout ?? "");
  } catch {
    return { error: (ran.stderr ?? "").slice(-4000), exit: ran.status };
  }
}

/**
 * Where the host reads its own configuration for this trial.
 *
 * With `KLIN_BENCH_CONFIG_DIR` set, each trial gets its own directory and the operator's
 * `~/.claude` reaches nothing. That directory needs its own credential, through
 * `ANTHROPIC_API_KEY` or a login of its own, because the host keys its keychain entry by the
 * configuration directory.
 *
 * With the variable unset, the trial runs against the operator's configuration.
 * `--setting-sources project,local` keeps the operator's settings, plugins and hooks out, and
 * `--disable-slash-commands` keeps their skills out. Their memory still reaches the session, so
 * `memory` records what it was.
 */
export function configFor(options: SessionOptions, trialId: string): string {
  return options.configRoot === "" ? "" : path.join(options.configRoot, trialId);
}

/** The user memory that reached the session, by digest and size, or null where none did. */
export function memory(configDir: string): { sha256: string; bytes: number } | null {
  const root = configDir === "" ? path.join(os.homedir(), ".claude") : configDir;
  const file = path.join(root, "CLAUDE.md");
  if (!fs.existsSync(file)) {
    return null;
  }
  const held = fs.readFileSync(file);
  return { sha256: sha256(held), bytes: held.length };
}

export function hostVersion(): string {
  try {
    return execFileSync("claude", ["--version"], { encoding: "utf8" }).trim();
  } catch {
    return "";
  }
}

export function klinVersion(klinBin: string): string {
  try {
    return execFileSync(klinBin, ["--version"], { encoding: "utf8" }).trim();
  } catch {
    return "";
  }
}
