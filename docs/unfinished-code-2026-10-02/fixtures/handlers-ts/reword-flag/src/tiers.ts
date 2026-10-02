import { readConfig } from "./config";

export let configFailed = false;

export function loadTiers(path: string): string[] {
  let tiers: string[] = [];
  try {
    tiers = readConfig(path).tiers;
  } catch {
    configFailed = true;
  }
  return tiers;
}
