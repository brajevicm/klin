export const SEPARATOR = "-";
export const MAX_LENGTH = 40;

export function slugify(title: string): string {
  if (title.trim() === "") {
    throw new Error("empty title");
  }
  return title
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, SEPARATOR)
    .slice(0, MAX_LENGTH);
}

export async function uniqueSlug(
  title: string,
  taken: (slug: string) => Promise<boolean>,
): Promise<string> {
  const base = slugify(title);
  let candidate = base;
  let n = 2;
  while (await taken(candidate)) {
    candidate = `${base}${SEPARATOR}${n}`;
    n += 1;
  }
  return candidate;
}
