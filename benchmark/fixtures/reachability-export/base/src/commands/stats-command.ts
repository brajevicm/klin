import { bar } from "../format/bar-format.ts";
import type { Store } from "../store.ts";

export function statsCommand(store: Store): string {
  const counts = new Map<string, number>();
  for (const note of store.values()) {
    for (const tag of note.tags) {
      counts.set(tag, (counts.get(tag) ?? 0) + 1);
    }
  }
  if (counts.size === 0) {
    return "no tags";
  }
  const widest = Math.max(...[...counts.keys()].map((tag) => tag.length + 1));
  return [...counts].sort(([a], [b]) => a.localeCompare(b)).map(([tag, count]) => bar("#" + tag, count, widest)).join("\n");
}
