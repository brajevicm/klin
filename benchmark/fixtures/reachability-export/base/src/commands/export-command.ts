import { csv } from "../format/csv-format.ts";
import type { Store } from "../store.ts";

export function exportCommand(store: Store): string {
  const rows = [...store.values()].map((note) => [note.name, note.text, note.tags.join(" ")]);
  return csv([["name", "text", "tags"], ...rows]);
}
