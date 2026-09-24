import { spawnSync } from "node:child_process";
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
  const ran = spawnSync(process.execPath, [process.env.KLIN_BENCH_TYPESCRIPT ?? "", "-p", "."], { encoding: "utf8" });
  return ran.status === 0 ? "" : "tsc -p . failed: " + ran.stdout + ran.stderr;
}
