import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import * as paths from "./paths.ts";
import { CURRENT_PROTOCOL } from "./protocol.ts";
import { PLANTED, VARIANTS, families, type NaturalVariantName } from "./catalogue.ts";
import { digest, sha256 } from "./trees.ts";
import * as session from "./session.ts";
import * as trial from "./trial.ts";
import * as toolchain from "./toolchain.ts";
import * as workspace from "./workspace.ts";
import { normalizedFlags } from "./calibrate.ts";

/**
 * The apparatus a round freezes, and the reading of whether it moved.
 *
 * This is its own module because two things read it and one of them is read by the other: the
 * probe records the apparatus it ran under, and the round holds a probe to the apparatus it is
 * about to freeze. A round that owned this would depend on the probe and the probe on the round.
 */

export interface Frozen {
  protocol: number;
  schemaSha256: string;
  harness: { commit: string; dirty: boolean; treeSha256: string; hookSha256: string };
  /** The sandbox and permission rules a trial runs under, including the work root they name. */
  confinement: string;
  /**
   * The process a subject actually runs in: the sanitized environment it is handed, the wall
   * clock and budget it runs under, and which configuration root it reads.
   *
   * The confinement digest holds the rules and this holds the rest. `CARGO_TARGET_DIR` sends a
   * Rust build outside the repository the sandbox allows, `PATH` and the toolchain homes decide
   * which compiler runs at all, and a shorter timeout ends a session the probe's own timeout let
   * finish. A probe run under one of these cannot authorize a round run under another.
   */
  execution: string;
  klin: { commit: string; version: string; binarySha256: string };
  toolchain: toolchain.Provenance;
  host: { name: string; version: string };
  model: string;
  flags: string[];
  isolatedConfiguration: boolean;
  memory: { sha256: string; bytes: number } | null;
  machine: { platform: string; release: string; arch: string; node: string };
  fixtures: Record<
    string,
    {
      gate: string;
      fixtureSha256: string;
      variants: Record<NaturalVariantName, { taskId: string; promptSha256: string; treeSha256: string }>;
    }
  >;
}

export function fixtures(): Frozen["fixtures"] {
  const held: Frozen["fixtures"] = {};
  const room = fs.mkdtempSync(path.join(os.tmpdir(), "klin-bench-plan-"));
  try {
    for (const [name, family] of Object.entries(families())) {
      const variants = {} as Frozen["fixtures"][string]["variants"];
      for (const variant of VARIANTS) {
        const laid = workspace.startingTree(family.variants[variant], path.join(room, name, variant));
        variants[variant] = {
          taskId: family.variants[variant].taskId,
          promptSha256: family.variants[variant].promptSha256,
          treeSha256: digest(laid),
        };
      }
      // The digest leaves out every planted variant's directory, so a seeded fixture added
      // beside a frozen round moves no identity that round was planned against. A family that
      // ships none digests exactly as it did before planting existed.
      held[name] = {
        gate: family.spec.gate,
        fixtureSha256: digest(family.root, new Set(PLANTED)),
        variants,
      };
    }
  } finally {
    fs.rmSync(room, { recursive: true, force: true });
  }
  return held;
}

/** Every round-wide frozen value the harness can read before the first session. */
export function frozen(options: session.SessionOptions): Frozen {
  const binary = fs.existsSync(options.klinBin) ? sha256(fs.readFileSync(options.klinBin)) : "";
  return {
    protocol: CURRENT_PROTOCOL.version,
    schemaSha256: sha256(fs.readFileSync(paths.SCHEMA)),
    harness: {
      commit: workspace.git(paths.REPO, "rev-parse", "HEAD"),
      dirty: workspace.git(paths.REPO, "status", "--porcelain") !== "",
      treeSha256: digest(path.join(paths.BENCHMARK, "src")),
      // The wrapper template is the hook every trial runs and it sits outside `src`, so without
      // this a changed `host/hook` moved nothing a round or a probe could see.
      hookSha256: sha256(fs.readFileSync(paths.HOOK)),
    },
    klin: {
      commit: trial.sourceCommit(options.klinBin, binary),
      version: session.klinVersion(options.klinBin),
      binarySha256: binary,
    },
    confinement: workspace.confinementSha256(),
    execution: sha256(
      JSON.stringify([
        Object.entries(session.withoutKlin()).sort((a, b) => a[0].localeCompare(b[0])),
        options.timeoutMs,
        options.budgetUsd,
        options.configRoot === "" ? "" : sha256(options.configRoot),
      ]),
    ),
    toolchain: toolchain.frozen(),
    host: { name: "claude-code", version: session.hostVersion() },
    model: options.model,
    flags: normalizedFlags(session.flagsFor({ settings: "" } as workspace.Workspace, "", options)),
    isolatedConfiguration: options.configRoot !== "",
    memory: options.configRoot === "" ? session.memory("") : null,
    machine: { platform: os.platform(), release: os.release(), arch: os.arch(), node: process.version },
    fixtures: fixtures(),
  };
}

/** The values two frozen readings disagree on, as sentences. */
export function drift(planned: Frozen, now: Frozen): string[] {
  const flat = (held: Frozen): [string, string][] => [
    ["the klin binary", held.klin.binarySha256],
    ["the klin version", held.klin.version],
    ["the klin source commit", held.klin.commit],
    ["the TypeScript compiler", JSON.stringify(held.toolchain ?? null)],
    ["the harness commit", held.harness.commit],
    ["the harness tree", held.harness.treeSha256],
    ["the hook wrapper", held.harness.hookSha256],
    ["the confinement", held.confinement],
    ["the execution environment", held.execution],
    ["the harness clean state", String(held.harness.dirty)],
    ["the host version", held.host.version],
    ["the requested model", held.model],
    ["the host flags", held.flags.join(" ")],
    ["the isolated-configuration status", String(held.isolatedConfiguration)],
    ["the user memory", held.memory?.sha256 ?? "none"],
    ["the record schema", held.schemaSha256],
    ["the protocol", String(held.protocol)],
    ["the fixtures", JSON.stringify(held.fixtures)],
    ["the machine", JSON.stringify(held.machine)],
  ];
  const was = new Map(flat(planned));
  return flat(now)
    .filter(([what, value]) => was.get(what) !== value)
    .map(([what, value]) => what + " moved from " + String(was.get(what)) + " to " + value);
}

