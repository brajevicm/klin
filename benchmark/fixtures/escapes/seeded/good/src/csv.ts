/** Split one line of delimited text into its fields. */
export function parseCsvLine(line: string): string[] {
  const fields: string[] = [];
  let at = 0;
  for (;;) {
    let field = "";
    if (line[at] === '"') {
      at += 1;
      while (at < line.length) {
        if (line.startsWith('""', at)) {
          field += '"';
          at += 2;
        } else if (line[at] === '"') {
          at += 1;
          break;
        } else {
          field += line[at];
          at += 1;
        }
      }
    } else {
      const end = line.indexOf(",", at);
      const stop = end === -1 ? line.length : end;
      field = line.slice(at, stop);
      at = stop;
    }
    fields.push(field);
    if (line[at] !== ",") {
      return fields;
    }
    at += 1;
  }
}

/** Every line of a document, without the trailing empty line. */
export function lines(document: string): string[] {
  const held = document.split("\n");
  return held.length > 0 && held[held.length - 1] === "" ? held.slice(0, -1) : held;
}
