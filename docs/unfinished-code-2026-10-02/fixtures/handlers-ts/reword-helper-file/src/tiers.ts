import { readConfig } from "./config";
import { safely } from "./safely";

export function loadTiers(path: string): string[] {
  return safely(() => readConfig(path).tiers, []);
}
