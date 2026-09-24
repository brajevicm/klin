function fields(line: string): string[] {
  const found: string[] = [];
  let field = "";
  let quoted = false;
  for (let at = 0; at < line.length; at += 1) {
    const character = line[at];
    if (quoted && character === '"' && line[at + 1] === '"') {
      field += '"';
      at += 1;
    } else if (character === '"') {
      quoted = !quoted;
    } else if (character === "," && !quoted) {
      found.push(field);
      field = "";
    } else {
      field += character;
    }
  }
  found.push(field);
  return found;
}

/** The rows of a comma-separated text, each a list of its fields. */
export function parseCsv(text: string): string[][] {
  return text
    .split(/\r?\n/)
    .filter((line) => line !== "")
    .map(fields);
}
