export function detail(text: string, tags: string[]): string {
  return [text, ...tags.map((tag) => "#" + tag)].join("\n");
}
