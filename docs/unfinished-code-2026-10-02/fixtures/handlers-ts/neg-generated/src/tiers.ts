import { readConfig } from "./config";

export function loadTiers(path: string): string[] {
  return readConfig(path).tiers;
}
