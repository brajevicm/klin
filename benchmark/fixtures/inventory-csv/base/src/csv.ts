/** The rows of a comma-separated text, each a list of its fields. */
export function parseCsv(text: string): string[][] {
  return text
    .split("\n")
    .filter((line) => line !== "")
    .map((line) => line.split(","));
}
