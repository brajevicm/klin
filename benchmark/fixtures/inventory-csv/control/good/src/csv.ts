/** The rows of a comma-separated text, each a list of its fields. */
export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let quoted = false;
  let at = 0;
  while (at < text.length) {
    const character = text[at];
    if (quoted) {
      if (character === '"' && text[at + 1] === '"') {
        field += '"';
        at += 1;
      } else if (character === '"') {
        quoted = false;
      } else {
        field += character;
      }
    } else if (character === '"') {
      quoted = true;
    } else if (character === ",") {
      row.push(field);
      field = "";
    } else if (character === "\n" || (character === "\r" && text[at + 1] === "\n")) {
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
      at += character === "\r" ? 1 : 0;
    } else {
      field += character;
    }
    at += 1;
  }
  if (field !== "" || row.length > 0) {
    row.push(field);
    rows.push(row);
  }
  return rows;
}

function written(field: string): string {
  return /[",\r\n]/.test(field) ? '"' + field.replaceAll('"', '""') + '"' : field;
}

/** The rows as comma-separated text, each row ending with a newline. */
export function toCsv(rows: string[][]): string {
  return rows.map((row) => row.map(written).join(",") + "\n").join("");
}
