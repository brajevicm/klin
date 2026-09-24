import { table } from "../format/table-format.ts";
import type { Store } from "../store.ts";

export function listCommand(store: Store): string {
  const notes = [...store.values()];
  if (notes.length === 0) {
    return "no notes";
  }
  return table(notes.map((note) => [note.name, note.tags.map((tag) => "#" + tag).join(" ")]));
}
