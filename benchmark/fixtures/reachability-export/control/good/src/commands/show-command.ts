import type { Store } from "../store.ts";

export function showCommand(store: Store, args: string[]): string {
  const [name] = args;
  if (!name) {
    return "show needs a name";
  }
  const note = store.get(name);
  if (!note) {
    return `no note named ${name}`;
  }
  return note.tags.length === 0 ? note.text : `${note.text}\n${note.tags.map((tag) => "#" + tag).join(" ")}`;
}
