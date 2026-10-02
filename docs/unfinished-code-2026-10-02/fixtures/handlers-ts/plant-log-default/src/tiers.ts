import { readConfig } from "./config";

export function loadTiers(path: string): string[] {
  try {
    return readConfig(path).tiers;
  } catch (error) {
    console.warn("could not read the config", error);
    return [];
  }
}
