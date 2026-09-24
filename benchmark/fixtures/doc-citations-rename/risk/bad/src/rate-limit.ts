export type Attempts = Map<string, number[]>;

export const LIMIT = 5;
export const WINDOW_MS = 60_000;

export function allow(attempts: Attempts, key: string, now: number): boolean {
  const recent = (attempts.get(key) ?? []).filter((at) => now - at < WINDOW_MS);
  recent.push(now);
  attempts.set(key, recent);
  return recent.length <= LIMIT;
}
