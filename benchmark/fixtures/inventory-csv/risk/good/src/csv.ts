/** A quoted field from its opening quote: its value, and where the text goes on after it. */
function quoted(text: string, from: number): [string, number] {
  let value = "";
  let at = from + 1;
  while (at < text.length) {
    if (text[at] === '"' && text[at + 1] === '"') {
      value += '"';
      at += 2;
    } else if (text[at] === '"') {
      return [value, at + 1];
    } else {
      value += text[at];
      at += 1;
    }
  }
  return [value, at];
}

/** How many characters the line break at `at` takes, or 0 where none starts there. */
function lineBreak(text: string, at: number): number {
  if (text[at] === "\n") {
    return 1;
  }
  return text[at] === "\r" && text[at + 1] === "\n" ? 2 : 0;
}

/** The rows of a comma-separated text, each a list of its fields. */
export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let at = 0;
  while (at < text.length) {
    const width = lineBreak(text, at);
    if (text[at] === '"') {
      const [value, next] = quoted(text, at);
      field += value;
      at = next;
    } else if (text[at] === ",") {
      row.push(field);
      field = "";
      at += 1;
    } else if (width > 0) {
      rows.push([...row, field]);
      row = [];
      field = "";
      at += width;
    } else {
      field += text[at];
      at += 1;
    }
  }
  if (field !== "" || row.length > 0) {
    rows.push([...row, field]);
  }
  return rows;
}
