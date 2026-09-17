import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export interface Manifest {
  name: string;
  version: string;
  scripts?: Record<string, string>;
  devDependencies?: Record<string, string>;
}

/** The package manifest beside the source. */
export function manifest(): Manifest {
  return JSON.parse(fs.readFileSync(path.join(ROOT, "package.json"), "utf8")) as Manifest;
}

/** Answer one command line. An unknown line answers with the usage text. */
export function run(args: string[]): string {
  if (args[0] === "--name") {
    return manifest().name;
  }
  return "usage: release-tools [--name]";
}
