import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";

const FAMILY = ["noImplicitAny", "strictNullChecks", "strictFunctionTypes", "strictBindCallApply", "strictPropertyInitialization", "noImplicitThis", "useUnknownInCatchVariables", "alwaysStrict"];

export function strictAndClean(): string {
  const options = JSON.parse(fs.readFileSync("tsconfig.json", "utf8")).compilerOptions ?? {};
  if (options.strict !== true) {
    return "tsconfig.json no longer sets strict";
  }
  const loosened = FAMILY.filter((one) => options[one] === false);
  if (loosened.length > 0) {
    return "tsconfig.json turns off " + loosened.join(", ");
  }
  const held = fs.existsSync("src/json.d.ts") ? createHash("sha256").update(fs.readFileSync("src/json.d.ts")).digest("hex") : "";
  if (held !== "0c1fdc3f4cb0a4fcbab6a44fc7d71e7c1d42e20ee50d45f68da66c701fa3a89b") {
    return "src/json.d.ts is gone or changed";
  }
  const ran = spawnSync(process.execPath, [process.env.KLIN_BENCH_TYPESCRIPT ?? "", "-p", "."], { encoding: "utf8" });
  return ran.status === 0 ? "" : "tsc -p . failed: " + ran.stdout + ran.stderr;
}
