import { spawnSync } from "node:child_process";

const agents = new Map<string, boolean>();

/** Whether this binary answers every host event through `klin __agent event`, which #498 put in
 * place of `klin radius`, `klin guard` and `klin gate --hook`. It has `klin status` from the same
 * release. */
function knowsAgent(klinBin: string): boolean {
  let known = agents.get(klinBin);
  if (known === undefined) {
    known = spawnSync(klinBin, ["status", "--help"], { stdio: "ignore" }).status === 0;
    agents.set(klinBin, known);
  }
  return known;
}

export type HookKind = "session" | "prompt" | "pre_tool" | "stop";

const BEFORE_AGENT: Record<HookKind, string[]> = {
  session: ["radius"],
  prompt: ["radius"],
  pre_tool: ["guard"],
  stop: ["gate", "--hook", "--changed"],
};

/** The arguments a host hook runs a klin binary with for one kind of event: the agent ingress, or
 * the command that served that event before #498. */
export function hookArguments(klinBin: string, kind: HookKind): string[] {
  return knowsAgent(klinBin) ? ["__agent", "event"] : BEFORE_AGENT[kind];
}
