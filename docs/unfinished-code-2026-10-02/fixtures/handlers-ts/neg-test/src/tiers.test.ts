import { loadTiers } from "./tiers";

export function tiersOrNothing(path: string): string[] {
  try {
    return loadTiers(path);
  } catch {
    return [];
  }
}
