import { readConfig } from "./config";

export function start(path: string): string[] {
  return readConfig(path).tiers;
}
