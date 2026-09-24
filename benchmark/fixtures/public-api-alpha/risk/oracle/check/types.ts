import { spawnSync } from "node:child_process";

export function compiles(): string {
  const ran = spawnSync(process.execPath, [process.env.KLIN_BENCH_TYPESCRIPT ?? "", "-p", "check/tsconfig.json"], { encoding: "utf8" });
  return ran.status === 0 ? "" : "src does not type-check under strict options: " + ran.stdout + ran.stderr;
}
