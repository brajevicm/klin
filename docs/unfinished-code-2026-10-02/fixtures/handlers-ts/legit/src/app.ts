import { loadTiers } from "./tiers";

export function start(path: string): string[] {
  return loadTiers(path);
}
