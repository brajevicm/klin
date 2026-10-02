export function slugify(title: string, separator = "-"): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, separator)
    .replace(new RegExp(`^${separator}|${separator}$`, "g"), "");
}
