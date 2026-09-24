import type { Store } from "../store.ts";

export function addCommand(store: Store, args: string[]): string {
  const [name, ...words] = args;
  if (!name || words.length === 0) {
    return "add needs a name and a text";
  }
  const tags = words.filter((word) => word.startsWith("#")).map((word) => word.slice(1));
  store.set(name, { name, text: words.join(" "), tags });
  return `added ${name}`;
}
