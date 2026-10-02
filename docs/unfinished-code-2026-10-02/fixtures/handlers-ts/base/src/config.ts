import { readFileSync } from "node:fs";

export interface Config {
  tiers: string[];
}

export function readConfig(path: string): Config {
  return JSON.parse(readFileSync(path, "utf8")) as Config;
}
