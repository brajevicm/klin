import { readConfig } from "./config";

const none: string[] = [];

export function loadTiers(path: string): string[] {
  try {
    return readConfig(path).tiers;
  } catch {
    return none;
  }
}
