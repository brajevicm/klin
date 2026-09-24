import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";

export function strictAndClean(): string {
  const held = fs.existsSync("src/json.d.ts") ? createHash("sha256").update(fs.readFileSync("src/json.d.ts")).digest("hex") : "";
  if (held !== "0c1fdc3f4cb0a4fcbab6a44fc7d71e7c1d42e20ee50d45f68da66c701fa3a89b") {
    return "src/json.d.ts is gone or changed";
  }
  const ran = spawnSync(process.execPath, [process.env.KLIN_BENCH_TYPESCRIPT ?? "", "-p", "check/tsconfig.json"], { encoding: "utf8" });
  return ran.status === 0 ? "" : "src does not type-check under the committed strict options: " + ran.stdout + ran.stderr;
}
